//! R4.8: real `self.field` read/write detection for one function/method body.
//!
//! See the module doc comment on `core::semantic::state` for the exact scope and rationale.
//! `self.<field>` is a Read (or, as the LHS of a plain `name = ..` assignment, a Write) attributed
//! to the enclosing method and scoped to the owning `impl` (the same `scope` already threaded
//! through `ExtractionContext::handle_function` for every other dimension -- no separate owner-name
//! tracking is needed). An arbitrarily deep chain of NAMED field accesses ultimately rooted at
//! `self` (`self.a.b`, `self.a.b.c`, ...) is modeled as ONE compound state-access event against the
//! `.`-joined path (`"a.b"`, `"a.b.c"`, ...), not decomposed into one event per level -- see
//! `self_field_path`'s own doc comment for why (evaluating an intermediate place like `self.a`
//! inside `self.a.b` is a pure place computation, never a value read, regardless of whether the
//! whole chain is ultimately read from or written to; fabricating a spurious Read of the
//! intermediate field would misrepresent Rust's own place-expression semantics). This closes a real
//! defect an earlier, single-level-only version of this walker had: `self.a.b = 5;` used to report
//! a spurious Read of `a` alone (the single-level check failed on the outer `.b` projection and
//! fell back to walking `field.base`, which the general `Expr::Field` dispatch then reinterpreted
//! as a READ of `self.a`) and never recorded the real write at all -- a fabricated fact, not merely
//! a coverage gap. A tuple-index member anywhere in the chain (`self.0`, `self.a.0`) still bails
//! the whole chain, matching the prior single-level behavior for that case exactly. Module-level
//! `static` reads/writes, and any access not rooted at a bare `self`, are not modeled this wave --
//! resolving them would require a source-unit-wide declaration pre-pass this extractor does not yet
//! perform; a documented gap, not a silent one. Compound-assignment operators such as
//! `self.field += 1` are represented by syn as `Expr::Binary` with an assignment BinOp. They are
//! semantically read-modify-write, so this extractor emits both a Read and a Write at the same
//! operation site, recognized via the same `spelling::is_compound_assign_op` R4.7's `dataflow.rs`
//! already uses for its own Use+Store pair (previously a second, independently-maintained copy of
//! the identical match existed here -- unified so STATE and DATA_FLOW can never silently disagree
//! about which operators are compound-assignment). Async/closure/const bodies remain separate
//! attribution domains this syntax-only wave does not flatten into the enclosing function; the
//! dimension obligation therefore remains UNKNOWN until those and other documented gaps are closed.

use atlas_core::{
    EpistemicStatus, EvidenceId, SemanticDimension, SemanticObservation, SemanticRecordHeader,
    SemanticRecordId, SemanticScope, StateAccessIdentity, StateAccessKind, StateResolution,
    stable_id,
};

use super::ExtractionContext;
use super::StatementWalker;
use super::spelling::is_compound_assign_op;

/// The `.`-joined field path this expression accesses, if it is an arbitrarily deep chain of
/// NAMED field accesses (`self.a`, `self.a.b`, `self.a.b.c`, ...) ultimately rooted at a bare
/// `self` -- one compound state-access event for the whole chain, not one per level (see the
/// module doc comment for why). Returns `None` the moment any level's member is unnamed (a
/// tuple-index field like `self.0`/`self.a.0`) or the chain's base is anything other than another
/// named-field `Expr::Field` or a bare `self` path (a local variable, a method call, an index
/// expression, ...) -- the caller falls back to walking that base normally in every such case.
fn self_field_path(field: &syn::ExprField) -> Option<String> {
    let syn::Member::Named(ident) = &field.member else {
        return None;
    };
    match field.base.as_ref() {
        syn::Expr::Path(path)
            if path.path.leading_colon.is_none()
                && path.path.segments.len() == 1
                && path.path.segments[0].ident == "self" =>
        {
            Some(ident.to_string())
        }
        syn::Expr::Field(base_field) => {
            let base_path = self_field_path(base_field)?;
            Some(format!("{base_path}.{ident}"))
        }
        _ => None,
    }
}

struct StateWalker<'ctx, 'a> {
    ctx: &'ctx mut ExtractionContext<'a>,
    function: SemanticRecordId,
    scope: SemanticScope,
}

impl<'ctx, 'a> StateWalker<'ctx, 'a> {
    fn emit_access(&mut self, name: &str, span: atlas_core::SourceSpan, kind: StateAccessKind) {
        let subject = StateAccessIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            scope: self.scope.clone(),
            name: name.to_owned(),
            span: span.clone(),
            kind,
            resolution: StateResolution::Resolved,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::State, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:state",
                self.ctx.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.ctx.record_dimension_hit(
            SemanticDimension::State,
            record_id.clone(),
            evidence_id.clone(),
        ) {
            return;
        }
        self.ctx.push_evidence(
            &evidence_id,
            format!(
                "parsed state-access {} `self.{}` at {}:{}:{}",
                subject.kind.as_str(),
                subject.name,
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::State,
            status: EpistemicStatus::Observed,
            subject,
            scope: self.scope.clone(),
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            extractor: self.ctx.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.ctx.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::State(header);
        assert!(observation.is_dimension_consistent());
        self.ctx.observations.push(observation);
    }

    fn walk_block(&mut self, block: &syn::Block) {
        for stmt in &block.stmts {
            self.walk_stmt(stmt);
        }
    }

    /// Walks a field expression's base ONLY when it might itself hide an UNRELATED self-access
    /// (e.g. `foo(self.y).z`) -- if the base is itself another `Expr::Field`, walking it would
    /// just re-attempt (and, before this fix, wrongly partially succeed at) interpreting a
    /// sub-chain that, as a whole, `self_field_path` already determined it cannot honestly name.
    /// An intermediate place is never itself a read/write event (see the module doc comment), so
    /// re-entering field-chain interpretation one level down must never fire here.
    fn walk_unresolved_field_base(&mut self, base: &syn::Expr) {
        if !matches!(base, syn::Expr::Field(_)) {
            self.walk_expr(base);
        }
    }
}

impl<'ctx, 'a> StatementWalker for StateWalker<'ctx, 'a> {
    fn walk_expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Field(field) => match self_field_path(field) {
                Some(path) => {
                    let span = self.ctx.span_of(field);
                    self.emit_access(&path, span, StateAccessKind::Read);
                }
                None => self.walk_unresolved_field_base(&field.base),
            },
            syn::Expr::Assign(assign) => {
                match assign.left.as_ref() {
                    syn::Expr::Field(field) => match self_field_path(field) {
                        Some(path) => {
                            let span = self.ctx.span_of(field);
                            self.emit_access(&path, span, StateAccessKind::Write);
                        }
                        None => self.walk_unresolved_field_base(&field.base),
                    },
                    other => self.walk_expr(other),
                }
                self.walk_expr(&assign.right);
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.walk_expr(value);
                }
            }
            syn::Expr::Block(block_expr) => self.walk_block(&block_expr.block),
            syn::Expr::If(if_expr) => {
                self.walk_expr(&if_expr.cond);
                self.walk_block(&if_expr.then_branch);
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.walk_expr(else_expr);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.walk_expr(&match_expr.expr);
                for arm in &match_expr.arms {
                    if let syn::Pat::Guard(guard) = &arm.pat {
                        self.walk_expr(&guard.guard);
                    }
                    self.walk_expr(&arm.body);
                }
            }
            syn::Expr::Loop(loop_expr) => self.walk_block(&loop_expr.body),
            syn::Expr::While(while_expr) => {
                self.walk_expr(&while_expr.cond);
                self.walk_block(&while_expr.body);
            }
            syn::Expr::ForLoop(for_loop) => {
                self.walk_expr(&for_loop.expr);
                self.walk_block(&for_loop.body);
            }
            syn::Expr::Binary(binary) => {
                if is_compound_assign_op(&binary.op) {
                    match binary.left.as_ref() {
                        syn::Expr::Field(field) => match self_field_path(field) {
                            Some(path) => {
                                let span = self.ctx.span_of(field);
                                self.emit_access(&path, span.clone(), StateAccessKind::Read);
                                self.emit_access(&path, span, StateAccessKind::Write);
                            }
                            None => self.walk_unresolved_field_base(&field.base),
                        },
                        other => self.walk_expr(other),
                    }
                } else {
                    self.walk_expr(&binary.left);
                }
                self.walk_expr(&binary.right);
            }
            syn::Expr::Unary(unary) => self.walk_expr(&unary.expr),
            syn::Expr::Paren(paren) => self.walk_expr(&paren.expr),
            syn::Expr::Group(group) => self.walk_expr(&group.expr),
            syn::Expr::Reference(reference) => self.walk_expr(&reference.expr),
            syn::Expr::RawAddr(raw_addr) => self.walk_expr(&raw_addr.expr),
            syn::Expr::Unsafe(unsafe_expr) => self.walk_block(&unsafe_expr.block),
            syn::Expr::TryBlock(try_block) => self.walk_block(&try_block.block),
            syn::Expr::Repeat(repeat) => {
                self.walk_expr(&repeat.expr);
                self.walk_expr(&repeat.len);
            }
            syn::Expr::Yield(yield_expr) => {
                if let Some(value) = &yield_expr.expr {
                    self.walk_expr(value);
                }
            }
            syn::Expr::Index(index) => {
                self.walk_expr(&index.expr);
                self.walk_expr(&index.index);
            }
            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.walk_expr(value);
                }
            }
            syn::Expr::Try(try_expr) => self.walk_expr(&try_expr.expr),
            syn::Expr::Await(await_expr) => self.walk_expr(&await_expr.base),
            syn::Expr::Cast(cast) => self.walk_expr(&cast.expr),
            syn::Expr::Call(call) => {
                self.walk_expr(&call.func);
                for arg in &call.args {
                    self.walk_expr(arg);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                self.walk_expr(&method_call.receiver);
                for arg in &method_call.args {
                    self.walk_expr(arg);
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.walk_expr(&field.expr);
                }
                if let Some(rest) = &struct_expr.rest {
                    self.walk_expr(rest);
                }
            }
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.walk_expr(elem);
                }
            }
            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.walk_expr(elem);
                }
            }
            syn::Expr::Range(range) => {
                if let Some(start) = &range.start {
                    self.walk_expr(start);
                }
                if let Some(end) = &range.end {
                    self.walk_expr(end);
                }
            }
            syn::Expr::Let(let_expr) => self.walk_expr(&let_expr.expr),
            // Closures, async blocks and const blocks are separate attribution/execution domains
            // this wave does not flatten into the enclosing function. Macro token bodies and
            // future non-exhaustive syn variants likewise remain open STATE obligations.
            syn::Expr::Closure(_) | syn::Expr::Async(_) | syn::Expr::Const(_) => {}
            _ => {}
        }
    }
}

impl<'a> ExtractionContext<'a> {
    /// Walks one function/method body, emitting a Read/Write State observation for every
    /// single-level `self.<field>` access reachable in it. No-op when `STATE` was not requested.
    pub(super) fn build_state(
        &mut self,
        body: &syn::Block,
        scope: &SemanticScope,
        function: &SemanticRecordId,
    ) {
        if !self.wants(SemanticDimension::State) {
            return;
        }
        let mut walker = StateWalker {
            ctx: self,
            function: function.clone(),
            scope: scope.clone(),
        };
        walker.walk_block(body);
    }
}
