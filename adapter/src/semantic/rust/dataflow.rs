//! R4.7: local def-use data-flow tracking for one function/method body.
//!
//! Like R4.6's CFG builder (and unlike R4.5's CALL walker), *which lexical binding a bare
//! identifier use refers to* within one function is fully determined by Rust's own scoping/
//! shadowing rules -- syntax alone, no type inference needed -- so this walker resolves it for
//! real via a scope stack. What it never attempts is anything requiring actual alias/borrow
//! analysis (whether `&x`/`&mut y` overlap, whether a compound place like `self.field`/`arr[i]`
//! aliases another value, points-to reasoning) -- those read as `DataFlowResolution::Unresolved`
//! or are simply not modeled as `Store`/`Definition` events this wave. See the module doc comment
//! on `core::semantic::data_flow` for the full scope/rationale.
//!
//! `walk_binding_pat` emits a `Definition` for every simple identifier binding within a `let`/
//! match-arm/`for`/parameter pattern, including tuple/tuple-struct/struct/slice destructuring,
//! `&`/`&mut`/parenthesized wrapping and `ident @ sub_pattern` bindings (R4.12: this previously
//! bound nothing for any destructuring pattern, a documented, not fabricated, gap -- now closed for
//! the pattern shapes Rust source actually uses). A plain `x = ..;` assignment
//! (`syn::Expr::Assign`) with a simple identifier LHS is modeled as a `Store`. Compound-assignment
//! operators (`x += 1`) are represented by this `syn` version as `syn::Expr::Binary` with a
//! compound `BinOp` (`AddAssign`, ...), not a distinct assignment form -- but that `BinOp` variant
//! is itself syntax-distinct from plain arithmetic (`AddAssign` vs `Add`, ...), so a compound
//! assignment to a simple identifier is modeled as BOTH a `Use` (the old value) and a `Store` (the
//! new value), matching R4.8's `state.rs` treatment of `self.field += 1` (see
//! `spelling::is_compound_assign_op`), correcting an earlier reading of this representation that
//! concluded it could only ever produce a plain `Use`.

use atlas_core::{
    DataFlowResolution, EpistemicStatus, EvidenceId, SemanticDimension, SemanticObservation,
    SemanticRecordHeader, SemanticRecordId, SemanticScope, ValueIdentity, ValueRole, stable_id,
};
use std::collections::BTreeMap;

use super::ExtractionContext;
use super::spelling;

struct DataFlowWalker<'ctx, 'a> {
    ctx: &'ctx mut ExtractionContext<'a>,
    function: SemanticRecordId,
    scope: SemanticScope,
    /// Innermost-last stack of (name -> most recent Definition record_id) maps, one per lexical
    /// block/pattern scope currently open.
    scope_stack: Vec<BTreeMap<String, SemanticRecordId>>,
}

impl<'ctx, 'a> DataFlowWalker<'ctx, 'a> {
    fn push_scope(&mut self) {
        self.scope_stack.push(BTreeMap::new());
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn lookup(&self, name: &str) -> Option<SemanticRecordId> {
        self.scope_stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn bind(&mut self, name: &str, record_id: SemanticRecordId) {
        if let Some(innermost) = self.scope_stack.last_mut() {
            innermost.insert(name.to_owned(), record_id);
        }
    }

    fn emit(&mut self, subject: ValueIdentity, span: atlas_core::SourceSpan) -> SemanticRecordId {
        let record_id = SemanticRecordId::new(SemanticDimension::DataFlow, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:data-flow",
                self.ctx.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.ctx.record_dimension_hit(
            SemanticDimension::DataFlow,
            record_id.clone(),
            evidence_id.clone(),
        ) {
            return record_id;
        }
        self.ctx.push_evidence(
            &evidence_id,
            format!(
                "parsed data-flow {} `{}` at {}:{}:{}",
                subject.role.as_str(),
                subject.name,
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::DataFlow,
            status: EpistemicStatus::Observed,
            subject,
            scope: self.scope.clone(),
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            extractor: self.ctx.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.ctx.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::DataFlow(header);
        assert!(observation.is_dimension_consistent());
        self.ctx.observations.push(observation);
        record_id
    }

    fn emit_definition(&mut self, name: &str, span: atlas_core::SourceSpan, is_parameter: bool) {
        let subject = ValueIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            name: name.to_owned(),
            span: span.clone(),
            role: ValueRole::Definition,
            is_parameter,
            is_return_flow: false,
            resolution: DataFlowResolution::Resolved,
            resolved_definition: None,
        };
        let record_id = self.emit(subject, span);
        self.bind(name, record_id);
    }

    fn emit_use_or_store(
        &mut self,
        name: &str,
        span: atlas_core::SourceSpan,
        role: ValueRole,
        is_return_flow: bool,
    ) {
        let resolved = self.lookup(name);
        let (resolution, resolved_definition) = match resolved {
            Some(id) => (DataFlowResolution::Resolved, Some(id)),
            None => (DataFlowResolution::Unresolved, None),
        };
        let subject = ValueIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            name: name.to_owned(),
            span: span.clone(),
            role,
            is_parameter: false,
            is_return_flow,
            resolution,
            resolved_definition,
        };
        self.emit(subject, span);
    }

    /// Emits a `Definition` for every simple identifier binding within `pat`, recursing through the
    /// full pattern surface a `let`/`match`-arm/`for` binding position can use: tuple/tuple-struct/
    /// struct/slice destructuring, `&`/`&mut` and parenthesized wrapping, type ascription, and an
    /// `ident @ sub_pattern` binding (which binds BOTH `ident` and whatever `sub_pattern` itself
    /// binds). `Pat::Or` (`Some(x) | None`) walks every alternative -- each alternative that binds a
    /// name produces its own `Definition` at that alternative's own span, since only one alternative
    /// matches at runtime and this walker does not attempt to prove which. `Pat::Rest` (`..`),
    /// `Pat::Wild` (`_`), literal/range/path/const patterns and `Pat::Const` bind nothing and are
    /// walked (for nested cases) or ignored (for leaves) without emitting anything.
    ///
    /// Replaces the earlier `simple_pat_ident`-only treatment of a `let`/match-arm/`for` pattern,
    /// which silently bound nothing for any destructuring pattern (a documented, not fabricated,
    /// gap -- see this module's doc comment history) -- closing that gap for the actually-common
    /// destructuring shapes Rust source uses, not merely the simple-identifier case.
    fn walk_binding_pat(&mut self, pat: &syn::Pat, is_parameter: bool) {
        match pat {
            syn::Pat::Ident(pat_ident) => {
                let span = self.ctx.span_of(&pat_ident.ident);
                self.emit_definition(&pat_ident.ident.to_string(), span, is_parameter);
                if let Some((_, sub_pat)) = &pat_ident.subpat {
                    self.walk_binding_pat(sub_pat, is_parameter);
                }
            }
            syn::Pat::Type(pat_type) => self.walk_binding_pat(&pat_type.pat, is_parameter),
            syn::Pat::Reference(pat_ref) => self.walk_binding_pat(&pat_ref.pat, is_parameter),
            syn::Pat::Paren(pat_paren) => self.walk_binding_pat(&pat_paren.pat, is_parameter),
            syn::Pat::Tuple(pat_tuple) => {
                for elem in &pat_tuple.elems {
                    self.walk_binding_pat(elem, is_parameter);
                }
            }
            syn::Pat::TupleStruct(pat_tuple_struct) => {
                for elem in &pat_tuple_struct.elems {
                    self.walk_binding_pat(elem, is_parameter);
                }
            }
            syn::Pat::Struct(pat_struct) => {
                for field in &pat_struct.fields {
                    self.walk_binding_pat(&field.pat, is_parameter);
                }
            }
            syn::Pat::Slice(pat_slice) => {
                for elem in &pat_slice.elems {
                    self.walk_binding_pat(elem, is_parameter);
                }
            }
            syn::Pat::Or(pat_or) => {
                for case in &pat_or.cases {
                    self.walk_binding_pat(case, is_parameter);
                }
            }
            syn::Pat::Wild(_)
            | syn::Pat::Rest(_)
            | syn::Pat::Lit(_)
            | syn::Pat::Range(_)
            | syn::Pat::Path(_)
            | syn::Pat::Const(_)
            | syn::Pat::Verbatim(_) => {}
            // Non-exhaustive enum: any pattern shape this `syn` version adds later binds nothing
            // until this walker is updated for it, rather than failing to compile.
            _ => {}
        }
    }

    /// `is_return_flow`: whether THIS block's own tail expression (if it has one) is itself in a
    /// position that flows to the enclosing function's return value -- true only for the function
    /// body itself and for a chain of directly-nested tail positions (an `if`/`match` arm/`{ }`
    /// block expression that is itself in return-flow position). A block reached as a `let`
    /// initializer, a loop body, or any other non-tail position must pass `false`, since its own
    /// last statement never flows to the function's return regardless of that statement's own
    /// syntactic shape.
    fn walk_block(&mut self, block: &syn::Block, is_return_flow: bool) {
        self.push_scope();
        let len = block.stmts.len();
        for (index, stmt) in block.stmts.iter().enumerate() {
            let is_tail = index + 1 == len && matches!(stmt, syn::Stmt::Expr(_, None));
            self.walk_stmt(stmt, is_tail && is_return_flow);
        }
        self.pop_scope();
    }

    fn walk_stmt(&mut self, stmt: &syn::Stmt, is_tail: bool) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.walk_expr(&init.expr, false);
                    if let Some((_, diverge)) = &init.diverge {
                        self.walk_expr(diverge, false);
                    }
                }
                self.walk_binding_pat(&local.pat, false);
            }
            syn::Stmt::Expr(expr, _) => self.walk_expr(expr, is_tail),
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    fn walk_expr(&mut self, expr: &syn::Expr, is_return_flow: bool) {
        match expr {
            syn::Expr::Path(_) => {
                if let Some(ident) = spelling::simple_path_ident(expr) {
                    let name = ident.to_string();
                    let span = self.ctx.span_of(expr);
                    self.emit_use_or_store(&name, span, ValueRole::Use, is_return_flow);
                }
            }
            syn::Expr::Assign(assign) => {
                match spelling::simple_path_ident(assign.left.as_ref()) {
                    Some(ident) => {
                        let name = ident.to_string();
                        let span = self.ctx.span_of(assign.left.as_ref());
                        self.emit_use_or_store(&name, span, ValueRole::Store, false);
                    }
                    None => self.walk_expr(assign.left.as_ref(), false),
                }
                self.walk_expr(&assign.right, false);
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.walk_expr(value, true);
                }
            }
            syn::Expr::Block(block_expr) => self.walk_block(&block_expr.block, is_return_flow),
            // `if let PAT = EXPR { .. }`: the scrutinee is walked in the OUTER scope (so a
            // same-named referent on the right, e.g. `if let Some(x) = x`, still resolves to the
            // pre-existing binding), then PAT's own names are pushed into a fresh scope covering
            // only the then-branch -- mirrors ForLoop's own push_scope/walk_binding_pat/pop_scope
            // shape below. A plain (non-`let`) condition is never a value/binding position, same
            // as before this fix.
            syn::Expr::If(if_expr) => {
                match if_expr.cond.as_ref() {
                    syn::Expr::Let(let_expr) => {
                        self.walk_expr(&let_expr.expr, false);
                        self.push_scope();
                        self.walk_binding_pat(&let_expr.pat, false);
                        self.walk_block(&if_expr.then_branch, is_return_flow);
                        self.pop_scope();
                    }
                    cond => {
                        self.walk_expr(cond, false);
                        self.walk_block(&if_expr.then_branch, is_return_flow);
                    }
                }
                // The else branch never sees an `if let` pattern's bindings -- reaching it means
                // the pattern did NOT match -- so it is always walked after the then-branch's
                // scope (if any) has already been popped above.
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.walk_expr(else_expr, is_return_flow);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.walk_expr(&match_expr.expr, false);
                for arm in &match_expr.arms {
                    self.push_scope();
                    // Match guards are parsed into `arm.pat` as `Pat::Guard` in this `syn`
                    // version (see adapter/src/semantic/rust/mod.rs's walk_expr::Expr::Match
                    // arm for the same discovery).
                    let (bound_pat, guard) = match &arm.pat {
                        syn::Pat::Guard(guard) => (guard.pat.as_ref(), Some(&guard.guard)),
                        other => (other, None),
                    };
                    // The pattern's own bindings must be in scope BEFORE the guard is walked --
                    // real Rust evaluates a guard with its arm's own pattern already bound
                    // (`Some(x) if x > 0 => ..`); walking the guard first would read `x` before
                    // it exists in this fresh scope.
                    self.walk_binding_pat(bound_pat, false);
                    if let Some(guard) = guard {
                        self.walk_expr(guard, false);
                    }
                    self.walk_expr(&arm.body, is_return_flow);
                    self.pop_scope();
                }
            }
            // Loop/while/for bodies are never return-flow: their own last statement is a
            // statement-position value (unit, absent a `break value`, which this wave does not
            // thread as return-flow either), never the enclosing function's return value.
            syn::Expr::Loop(loop_expr) => self.walk_block(&loop_expr.body, false),
            // `while let PAT = EXPR { .. }`: same reasoning as `if let` above, except the fresh
            // scope covers the loop body instead of a then-branch.
            syn::Expr::While(while_expr) => match while_expr.cond.as_ref() {
                syn::Expr::Let(let_expr) => {
                    self.walk_expr(&let_expr.expr, false);
                    self.push_scope();
                    self.walk_binding_pat(&let_expr.pat, false);
                    self.walk_block(&while_expr.body, false);
                    self.pop_scope();
                }
                cond => {
                    self.walk_expr(cond, false);
                    self.walk_block(&while_expr.body, false);
                }
            },
            syn::Expr::ForLoop(for_loop) => {
                self.walk_expr(&for_loop.expr, false);
                self.push_scope();
                self.walk_binding_pat(&for_loop.pat, false);
                self.walk_block(&for_loop.body, false);
                self.pop_scope();
            }
            syn::Expr::Binary(binary) if spelling::is_compound_assign_op(&binary.op) => {
                // `x += 1` etc.: provably a read-modify-write of the left operand, not merely a
                // read -- see `spelling::is_compound_assign_op`.
                match spelling::simple_path_ident(binary.left.as_ref()) {
                    Some(ident) => {
                        let name = ident.to_string();
                        let span = self.ctx.span_of(binary.left.as_ref());
                        self.emit_use_or_store(&name, span.clone(), ValueRole::Use, false);
                        self.emit_use_or_store(&name, span, ValueRole::Store, false);
                    }
                    None => self.walk_expr(binary.left.as_ref(), false),
                }
                self.walk_expr(&binary.right, false);
            }
            syn::Expr::Binary(binary) => {
                self.walk_expr(&binary.left, false);
                self.walk_expr(&binary.right, false);
            }
            syn::Expr::Unary(unary) => self.walk_expr(&unary.expr, false),
            syn::Expr::Paren(paren) => self.walk_expr(&paren.expr, is_return_flow),
            syn::Expr::Group(group) => self.walk_expr(&group.expr, is_return_flow),
            syn::Expr::Reference(reference) => self.walk_expr(&reference.expr, false),
            syn::Expr::Field(field) => self.walk_expr(&field.base, false),
            syn::Expr::Index(index) => {
                self.walk_expr(&index.expr, false);
                self.walk_expr(&index.index, false);
            }
            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.walk_expr(value, false);
                }
            }
            syn::Expr::Try(try_expr) => self.walk_expr(&try_expr.expr, false),
            syn::Expr::Await(await_expr) => self.walk_expr(&await_expr.base, false),
            syn::Expr::Cast(cast) => self.walk_expr(&cast.expr, false),
            syn::Expr::Call(call) => {
                self.walk_expr(&call.func, false);
                for arg in &call.args {
                    self.walk_expr(arg, false);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                self.walk_expr(&method_call.receiver, false);
                for arg in &method_call.args {
                    self.walk_expr(arg, false);
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.walk_expr(&field.expr, false);
                }
                if let Some(rest) = &struct_expr.rest {
                    self.walk_expr(rest, false);
                }
            }
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.walk_expr(elem, false);
                }
            }
            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.walk_expr(elem, false);
                }
            }
            syn::Expr::Range(range) => {
                if let Some(start) = &range.start {
                    self.walk_expr(start, false);
                }
                if let Some(end) = &range.end {
                    self.walk_expr(end, false);
                }
            }
            // Reached only when a `let PAT = EXPR` is NOT the direct condition of an `if`/`while`
            // (the common cases, handled above with their pattern bound into a fresh scope) --
            // e.g. one operand of a let-chain (`if let A = a && let B = b { .. }`). This walker
            // does not thread let-chain bindings into scope; a name PAT binds in that position
            // falls back to Unresolved or an outer definition, a known, narrower-scope gap this
            // fix does not close (a genuinely rarer construct than the plain if-let/while-let
            // case this fix targets).
            syn::Expr::Let(let_expr) => self.walk_expr(&let_expr.expr, false),
            // `unsafe { .. }`/`try { .. }` execute immediately as part of the same executable
            // region and may themselves be in return-flow (tail) position, exactly like a plain
            // `{ .. }` block above. `const { .. }` is a distinct compile-time-evaluated context,
            // never itself the function's runtime return value, so it is walked but never as
            // return-flow.
            syn::Expr::Unsafe(unsafe_expr) => self.walk_block(&unsafe_expr.block, is_return_flow),
            syn::Expr::TryBlock(try_block) => self.walk_block(&try_block.block, is_return_flow),
            syn::Expr::Const(const_expr) => self.walk_block(&const_expr.block, false),
            syn::Expr::Repeat(repeat) => {
                self.walk_expr(&repeat.expr, false);
                self.walk_expr(&repeat.len, false);
            }
            syn::Expr::RawAddr(raw_addr) => self.walk_expr(&raw_addr.expr, false),
            // `yield value` evaluates `value` immediately in the SAME executable region -- syn
            // parses it wherever it lexically appears, not only inside a real generator body.
            syn::Expr::Yield(yield_expr) => {
                if let Some(value) = &yield_expr.expr {
                    self.walk_expr(value, false);
                }
            }
            // Closures and `async { .. }` blocks get no data-flow scoping of their own this wave
            // (consistent with R4.6's CFG, which gives neither a CFG either -- both are separate
            // deferred executable regions with no FunctionIdentity to attribute records to); a bare
            // macro invocation's arguments are opaque token streams this extractor never re-parses
            // (a real, permanent gap, not silently omitted -- see `dimension_coverage`).
            // Continue/literals/other forms carry no nested expressions this walker tracks. Never
            // claimed, never fabricated.
            _ => {}
        }
    }
}

impl<'a> ExtractionContext<'a> {
    /// Walks one function/method body, emitting Definition/Use/Store data-flow events attributed
    /// to `function` (the enclosing function's `FunctionIdentity` record_id), including parameter
    /// definitions. No-op when `DATA_FLOW` was not requested.
    pub(super) fn build_data_flow(
        &mut self,
        sig: &syn::Signature,
        body: &syn::Block,
        scope: &SemanticScope,
        function: &SemanticRecordId,
    ) {
        if !self.wants(SemanticDimension::DataFlow) {
            return;
        }
        let mut walker = DataFlowWalker {
            ctx: self,
            function: function.clone(),
            scope: scope.clone(),
            scope_stack: Vec::new(),
        };
        walker.push_scope();
        for input in &sig.inputs {
            match input {
                syn::FnArg::Receiver(receiver) => {
                    let span = walker.ctx.span_of(receiver);
                    walker.emit_definition("self", span, true);
                }
                syn::FnArg::Typed(pat_type) => {
                    walker.walk_binding_pat(&pat_type.pat, true);
                }
            }
        }
        walker.walk_block(body, true);
        walker.pop_scope();
    }
}
