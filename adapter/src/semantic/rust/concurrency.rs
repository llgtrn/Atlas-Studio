//! R4.10: real `Await`/`Spawn` concurrency-site detection for one function/method body.
//!
//! See the module doc comment on `core::semantic::concurrency` for the exact scope and rationale.
//! `expr.await` is dedicated `syn::Expr::Await` syntax -- fully syntax-determined, cannot be
//! shadowed or overloaded. A call whose callee spelling ends in the segment `spawn` is detected
//! purely by that textual spelling, the same risk/precision class R4.8's `is_panic_like_macro`
//! already accepts for macro names. Every other `ConcurrencyKind` (`Lock`/`Unlock`/`ChannelCreate`/
//! `ChannelSend`/`ChannelReceive`/`AtomicOp`) would require resolving a method/function call to a
//! specific known API, which this extractor cannot do without fabricating semantics, so none of
//! them are emitted this wave. Unlike R4.9's OWNERSHIP, no context threading is needed: `Await`/
//! `Spawn` sites are real wherever they occur in the expression tree, not only in specific
//! positions, so this walker mirrors R4.8 EFFECT's simpler unconditional full-tree walk.

use atlas_core::{
    ConcurrencyIdentity, ConcurrencyKind, EpistemicStatus, EvidenceId, SemanticDimension,
    SemanticObservation, SemanticRecordHeader, SemanticRecordId, SemanticScope, stable_id,
};

use super::ExtractionContext;
use super::spelling::call_callee_spelling;

fn is_spawn_call(callee: &syn::Expr) -> bool {
    call_callee_spelling(callee)
        .rsplit("::")
        .next()
        .is_some_and(|last| last == "spawn")
}

struct ConcurrencyWalker<'ctx, 'a> {
    ctx: &'ctx mut ExtractionContext<'a>,
    function: SemanticRecordId,
    scope: SemanticScope,
}

impl<'ctx, 'a> ConcurrencyWalker<'ctx, 'a> {
    fn emit(&mut self, span: atlas_core::SourceSpan, kind: ConcurrencyKind) {
        let subject = ConcurrencyIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            kind,
            span: span.clone(),
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Concurrency, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:concurrency",
                self.ctx.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.ctx.record_dimension_hit(
            SemanticDimension::Concurrency,
            record_id.clone(),
            evidence_id.clone(),
        ) {
            return;
        }
        self.ctx.push_evidence(
            &evidence_id,
            format!(
                "parsed concurrency {} at {}:{}:{}",
                subject.kind.as_str(),
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Concurrency,
            status: EpistemicStatus::Observed,
            subject,
            scope: self.scope.clone(),
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            extractor: self.ctx.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.ctx.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::Concurrency(header);
        debug_assert!(observation.is_dimension_consistent());
        self.ctx.observations.push(observation);
    }

    fn walk_block(&mut self, block: &syn::Block) {
        for stmt in &block.stmts {
            self.walk_stmt(stmt);
        }
    }

    fn walk_stmt(&mut self, stmt: &syn::Stmt) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.walk_expr(&init.expr);
                    if let Some((_, diverge)) = &init.diverge {
                        self.walk_expr(diverge);
                    }
                }
            }
            syn::Stmt::Expr(expr, _) => self.walk_expr(expr),
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    fn walk_expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Await(await_expr) => {
                let span = self.ctx.span_of(expr);
                self.emit(span, ConcurrencyKind::Await);
                self.walk_expr(&await_expr.base);
            }
            syn::Expr::Call(call) => {
                if is_spawn_call(&call.func) {
                    let span = self.ctx.span_of(call);
                    self.emit(span, ConcurrencyKind::Spawn);
                }
                self.walk_expr(&call.func);
                for arg in &call.args {
                    self.walk_expr(arg);
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
            syn::Expr::Assign(assign) => {
                self.walk_expr(&assign.left);
                self.walk_expr(&assign.right);
            }
            syn::Expr::Binary(binary) => {
                self.walk_expr(&binary.left);
                self.walk_expr(&binary.right);
            }
            syn::Expr::Unary(unary) => self.walk_expr(&unary.expr),
            syn::Expr::Paren(paren) => self.walk_expr(&paren.expr),
            syn::Expr::Group(group) => self.walk_expr(&group.expr),
            syn::Expr::Reference(reference) => self.walk_expr(&reference.expr),
            syn::Expr::Field(field) => self.walk_expr(&field.base),
            syn::Expr::Index(index) => {
                self.walk_expr(&index.expr);
                self.walk_expr(&index.index);
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.walk_expr(value);
                }
            }
            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.walk_expr(value);
                }
            }
            syn::Expr::Try(try_expr) => self.walk_expr(&try_expr.expr),
            syn::Expr::Cast(cast) => self.walk_expr(&cast.expr),
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
            // `unsafe { .. }`/`const { .. }`/`try { .. }` execute immediately as part of the same
            // executable region -- an `.await`/spawn-shaped call inside one is a real site of the
            // enclosing function.
            syn::Expr::Unsafe(unsafe_expr) => self.walk_block(&unsafe_expr.block),
            syn::Expr::Const(const_expr) => self.walk_block(&const_expr.block),
            syn::Expr::TryBlock(try_block) => self.walk_block(&try_block.block),
            syn::Expr::Repeat(repeat) => {
                self.walk_expr(&repeat.expr);
                self.walk_expr(&repeat.len);
            }
            syn::Expr::RawAddr(raw_addr) => self.walk_expr(&raw_addr.expr),
            // Closures and `async { .. }` blocks get no concurrency attribution of their own this
            // wave (consistent with R4.6/R4.7/R4.8/R4.9 -- both are separate deferred executable
            // regions, and `async { .. }`'s own `.await`/spawn sites belong to whatever polls it,
            // not to this function); path/literal/other forms carry no nested expressions this
            // walker tracks.
            _ => {}
        }
    }
}

impl<'a> ExtractionContext<'a> {
    /// Walks one function/method body, emitting Await for every `.await` and Spawn for every call
    /// whose callee spelling ends in `spawn`, reachable anywhere in it. No-op when `CONCURRENCY`
    /// was not requested.
    pub(super) fn build_concurrency(
        &mut self,
        body: &syn::Block,
        scope: &SemanticScope,
        function: &SemanticRecordId,
    ) {
        if !self.wants(SemanticDimension::Concurrency) {
            return;
        }
        let mut walker = ConcurrencyWalker {
            ctx: self,
            function: function.clone(),
            scope: scope.clone(),
        };
        walker.walk_block(body);
    }
}
