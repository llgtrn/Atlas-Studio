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
//! Scope this wave: only simple identifier patterns (`syn::Pat::Ident`, with or without `mut`) are
//! modeled as `Definition`s -- a `let (a, b) = ..;` tuple/struct/slice-destructuring pattern binds
//! nothing this wave (a documented gap, not a fabricated one). A plain `x = ..;` assignment
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
        debug_assert!(observation.is_dimension_consistent());
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

    /// The identifier this simple `syn::Pat` binds, if it is (or wraps, via `mut`/type ascription)
    /// a plain `syn::Pat::Ident` -- the only pattern shape modeled this wave.
    fn simple_pat_ident(pat: &syn::Pat) -> Option<&syn::Ident> {
        match pat {
            syn::Pat::Ident(pat_ident) => Some(&pat_ident.ident),
            syn::Pat::Type(pat_type) => Self::simple_pat_ident(&pat_type.pat),
            _ => None,
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
                if let Some(ident) = Self::simple_pat_ident(&local.pat) {
                    let span = self.ctx.span_of(ident);
                    self.emit_definition(&ident.to_string(), span, false);
                }
            }
            syn::Stmt::Expr(expr, _) => self.walk_expr(expr, is_tail),
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    fn walk_expr(&mut self, expr: &syn::Expr, is_return_flow: bool) {
        match expr {
            syn::Expr::Path(path)
                if path.path.leading_colon.is_none() && path.path.segments.len() == 1 =>
            {
                let name = path.path.segments[0].ident.to_string();
                let span = self.ctx.span_of(path);
                self.emit_use_or_store(&name, span, ValueRole::Use, is_return_flow);
            }
            syn::Expr::Assign(assign) => {
                match assign.left.as_ref() {
                    syn::Expr::Path(path)
                        if path.path.leading_colon.is_none() && path.path.segments.len() == 1 =>
                    {
                        let name = path.path.segments[0].ident.to_string();
                        let span = self.ctx.span_of(assign.left.as_ref());
                        self.emit_use_or_store(&name, span, ValueRole::Store, false);
                    }
                    other => self.walk_expr(other, false),
                }
                self.walk_expr(&assign.right, false);
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.walk_expr(value, true);
                }
            }
            syn::Expr::Block(block_expr) => self.walk_block(&block_expr.block, is_return_flow),
            syn::Expr::If(if_expr) => {
                self.walk_expr(&if_expr.cond, false);
                self.walk_block(&if_expr.then_branch, is_return_flow);
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
                    // arm for the same discovery). Only a bare identifier arm pattern (or the
                    // inner pattern of a guard) is modeled as a Definition this wave.
                    let bound_pat = match &arm.pat {
                        syn::Pat::Guard(guard) => {
                            self.walk_expr(&guard.guard, false);
                            guard.pat.as_ref()
                        }
                        other => other,
                    };
                    if let Some(ident) = Self::simple_pat_ident(bound_pat) {
                        let span = self.ctx.span_of(ident);
                        self.emit_definition(&ident.to_string(), span, false);
                    }
                    self.walk_expr(&arm.body, is_return_flow);
                    self.pop_scope();
                }
            }
            // Loop/while/for bodies are never return-flow: their own last statement is a
            // statement-position value (unit, absent a `break value`, which this wave does not
            // thread as return-flow either), never the enclosing function's return value.
            syn::Expr::Loop(loop_expr) => self.walk_block(&loop_expr.body, false),
            syn::Expr::While(while_expr) => {
                self.walk_expr(&while_expr.cond, false);
                self.walk_block(&while_expr.body, false);
            }
            syn::Expr::ForLoop(for_loop) => {
                self.walk_expr(&for_loop.expr, false);
                self.push_scope();
                if let Some(ident) = Self::simple_pat_ident(&for_loop.pat) {
                    let span = self.ctx.span_of(ident);
                    self.emit_definition(&ident.to_string(), span, false);
                }
                self.walk_block(&for_loop.body, false);
                self.pop_scope();
            }
            syn::Expr::Binary(binary) if spelling::is_compound_assign_op(&binary.op) => {
                // `x += 1` etc.: provably a read-modify-write of the left operand, not merely a
                // read -- see `spelling::is_compound_assign_op`.
                match binary.left.as_ref() {
                    syn::Expr::Path(path)
                        if path.path.leading_colon.is_none() && path.path.segments.len() == 1 =>
                    {
                        let name = path.path.segments[0].ident.to_string();
                        let span = self.ctx.span_of(binary.left.as_ref());
                        self.emit_use_or_store(&name, span.clone(), ValueRole::Use, false);
                        self.emit_use_or_store(&name, span, ValueRole::Store, false);
                    }
                    other => self.walk_expr(other, false),
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
                    if let Some(ident) = DataFlowWalker::simple_pat_ident(&pat_type.pat) {
                        let span = walker.ctx.span_of(ident);
                        walker.emit_definition(&ident.to_string(), span, true);
                    }
                }
            }
        }
        walker.walk_block(body, true);
        walker.pop_scope();
    }
}
