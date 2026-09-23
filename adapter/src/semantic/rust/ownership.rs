//! R4.9: real borrow-site detection and honestly-ambiguous move-or-copy-site detection for one
//! function/method body.
//!
//! See the module doc comment on `core::semantic::ownership` for the exact scope and rationale.
//! `&expr`/`&mut expr` is fully syntax-determined (unlike a call's target, borrow syntax never
//! requires type resolution), so those sites are recorded as real `BorrowShared`/`BorrowMut`
//! observations. A bare identifier used by value -- a call argument, a `let` initializer, an
//! assignment RHS, an explicit `return` value, or the function's own implicit tail-expression
//! return -- is recorded as `MoveOrCopy`: syntax alone confirms a by-value use occurred, but
//! whether it moves or copies depends on whether the identifier's type implements `Copy`, which
//! this extractor cannot resolve, so it never guesses either way.
//!
//! Scope this wave: only a bare, single-segment identifier (`syn::Expr::Path`) is tracked as a
//! move-or-copy source -- a field-projected place (`self.field`) is not modeled as a move source,
//! matching R4.8 STATE's identical "single-level self.field only" precedent (a documented gap, not
//! a silent one). A method call's receiver is never treated as a move-or-copy site (whether a
//! method takes `self`/`&self`/`&mut self` cannot be told from the call site alone). Return-flow
//! context is threaded through nested blocks exactly as R4.7's `dataflow.rs` does (after its own
//! R4.7 fix): only a genuine chain of tail positions starting at the function body counts as a
//! move-or-copy-eligible return site, never any block's structurally-last statement regardless of
//! position.

use atlas_core::{
    EpistemicStatus, EvidenceId, OwnershipIdentity, OwnershipKind, SemanticDimension,
    SemanticObservation, SemanticRecordHeader, SemanticRecordId, SemanticScope, stable_id,
};

use super::ExtractionContext;
use super::spelling::call_callee_spelling;

fn is_bare_path(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Path(path) if path.path.leading_colon.is_none() && path.path.segments.len() == 1)
}

struct OwnershipWalker<'ctx, 'a> {
    ctx: &'ctx mut ExtractionContext<'a>,
    function: SemanticRecordId,
    scope: SemanticScope,
}

impl<'ctx, 'a> OwnershipWalker<'ctx, 'a> {
    fn emit(&mut self, name: &str, span: atlas_core::SourceSpan, kind: OwnershipKind) {
        let subject = OwnershipIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            name: name.to_owned(),
            span: span.clone(),
            kind,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Ownership, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:ownership",
                self.ctx.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.ctx.record_dimension_hit(
            SemanticDimension::Ownership,
            record_id.clone(),
            evidence_id.clone(),
        ) {
            return;
        }
        self.ctx.push_evidence(
            &evidence_id,
            format!(
                "parsed ownership {} `{}` at {}:{}:{}",
                subject.kind.as_str(),
                subject.name,
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Ownership,
            status: EpistemicStatus::Observed,
            subject,
            scope: self.scope.clone(),
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            extractor: self.ctx.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.ctx.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::Ownership(header);
        debug_assert!(observation.is_dimension_consistent());
        self.ctx.observations.push(observation);
    }

    /// `is_value_position`: whether `expr` sits at a site that would move/copy a bare identifier
    /// in real Rust (a call argument, a `let`/assignment RHS, a `return` value, or a chain of
    /// genuine tail positions back to the function body). See the module doc comment for why this
    /// must be threaded rather than recomputed structurally at each nesting level.
    fn walk_block(&mut self, block: &syn::Block, is_value_position: bool) {
        let len = block.stmts.len();
        for (index, stmt) in block.stmts.iter().enumerate() {
            let is_tail = index + 1 == len && matches!(stmt, syn::Stmt::Expr(_, None));
            self.walk_stmt(stmt, is_tail && is_value_position);
        }
    }

    fn walk_stmt(&mut self, stmt: &syn::Stmt, is_tail_value_position: bool) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.walk_expr(&init.expr, true);
                    if let Some((_, diverge)) = &init.diverge {
                        self.walk_expr(diverge, false);
                    }
                }
            }
            syn::Stmt::Expr(expr, _) => self.walk_expr(expr, is_tail_value_position),
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    fn walk_expr(&mut self, expr: &syn::Expr, is_value_position: bool) {
        match expr {
            syn::Expr::Reference(reference) => {
                let kind = if reference.mutability.is_some() {
                    OwnershipKind::BorrowMut
                } else {
                    OwnershipKind::BorrowShared
                };
                let name = call_callee_spelling(&reference.expr);
                let span = self.ctx.span_of(reference);
                self.emit(&name, span, kind);
                // The referent is borrowed here, not itself moved/copied -- but it may still
                // contain nested ownership-relevant subexpressions (e.g. `&f(x)`).
                self.walk_expr(&reference.expr, false);
            }
            syn::Expr::Path(_) if is_value_position && is_bare_path(expr) => {
                let name = call_callee_spelling(expr);
                let span = self.ctx.span_of(expr);
                self.emit(&name, span, OwnershipKind::MoveOrCopy);
            }
            syn::Expr::Assign(assign) => {
                self.walk_expr(&assign.left, false);
                self.walk_expr(&assign.right, true);
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.walk_expr(value, true);
                }
            }
            syn::Expr::Block(block_expr) => self.walk_block(&block_expr.block, is_value_position),
            syn::Expr::If(if_expr) => {
                self.walk_expr(&if_expr.cond, false);
                self.walk_block(&if_expr.then_branch, is_value_position);
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.walk_expr(else_expr, is_value_position);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.walk_expr(&match_expr.expr, false);
                for arm in &match_expr.arms {
                    if let syn::Pat::Guard(guard) = &arm.pat {
                        self.walk_expr(&guard.guard, false);
                    }
                    self.walk_expr(&arm.body, is_value_position);
                }
            }
            // Loop/while/for bodies are never value positions: their own last statement is a
            // statement-position value, never the enclosing function's return value.
            syn::Expr::Loop(loop_expr) => self.walk_block(&loop_expr.body, false),
            syn::Expr::While(while_expr) => {
                self.walk_expr(&while_expr.cond, false);
                self.walk_block(&while_expr.body, false);
            }
            syn::Expr::ForLoop(for_loop) => {
                self.walk_expr(&for_loop.expr, false);
                self.walk_block(&for_loop.body, false);
            }
            syn::Expr::Binary(binary) => {
                self.walk_expr(&binary.left, false);
                self.walk_expr(&binary.right, false);
            }
            syn::Expr::Unary(unary) => self.walk_expr(&unary.expr, false),
            syn::Expr::Paren(paren) => self.walk_expr(&paren.expr, is_value_position),
            syn::Expr::Group(group) => self.walk_expr(&group.expr, is_value_position),
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
                    self.walk_expr(arg, true);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                // The receiver's self/&self/&mut self shape cannot be told from the call site
                // alone -- never treated as a move-or-copy site this wave.
                self.walk_expr(&method_call.receiver, false);
                for arg in &method_call.args {
                    self.walk_expr(arg, true);
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
            // Closures get no ownership attribution of their own this wave (consistent with
            // R4.6/R4.7/R4.8); macro/literal/other forms carry no nested ownership-relevant
            // expressions this walker tracks.
            _ => {}
        }
    }
}

impl<'a> ExtractionContext<'a> {
    /// Walks one function/method body, emitting BorrowShared/BorrowMut for every `&`/`&mut` site
    /// and MoveOrCopy for every bare-identifier by-value use site reachable in it. No-op when
    /// `OWNERSHIP` was not requested.
    pub(super) fn build_ownership(
        &mut self,
        body: &syn::Block,
        scope: &SemanticScope,
        function: &SemanticRecordId,
    ) {
        if !self.wants(SemanticDimension::Ownership) {
            return;
        }
        let mut walker = OwnershipWalker {
            ctx: self,
            function: function.clone(),
            scope: scope.clone(),
        };
        walker.walk_block(body, true);
    }
}
