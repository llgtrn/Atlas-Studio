//! R4.11: conservative durable-state/recovery-site detection for one function/method body.
//!
//! See the module doc comment on `core::semantic::persistence` for the exact scope, the
//! `PlaceRef` bridge rationale, and why this extractor never invents a spelling-keyed target
//! identity. This wave has no dedicated Rust syntax for persistence (unlike R4.10's `.await`) and
//! no resolved-API adapter, so every `PersistenceKind` this walker emits is a textual
//! callee-spelling candidate ONLY -- exactly R4.8's `is_panic_like_macro`/R4.10's `is_spawn_call`
//! risk class -- and is always `EpistemicStatus::Inferred` with `PersistenceResolution::Unresolved`
//! and `PlaceRef::Unresolved`. `game.commit()`, `ui.flush()`, `cache.sync()` and
//! `builder.snapshot()` are exactly as "persistence-shaped" by spelling as a real durable API call;
//! this extractor cannot tell them apart without resolving the receiver's type, so it never claims
//! `Observed`.

use atlas_core::{
    EpistemicStatus, EvidenceId, PersistenceIdentity, PersistenceKind, PersistenceResolution,
    PlaceRef, SemanticDimension, SemanticObservation, SemanticRecordHeader, SemanticRecordId,
    SemanticScope, stable_id,
};

use super::ExtractionContext;
use super::spelling::call_callee_spelling;

/// The single closed set of persistence-shaped spellings this extractor recognizes, shared by
/// both call-site shapes below (a free/path call's last segment, and a method call's bare method
/// name) so the two can never silently drift apart -- the exact defect class R4.12's CALL/
/// DATA_FLOW convergence fix already closed for a different pair of walkers (see
/// `call_argument_place_ref_converges_on_the_data_flow_uses_own_record_id`'s own doc comment:
/// "the SAME recognizer ... factored out to guarantee the two walkers cannot silently drift
/// apart"). Every match is equally uncertain (see the module doc comment) -- there is no "more
/// trustworthy" spelling among these, so all five map to `Inferred` alike.
fn persistence_kind_for_spelling(name: &str) -> Option<PersistenceKind> {
    match name {
        "commit" => Some(PersistenceKind::Commit),
        "flush" => Some(PersistenceKind::Flush),
        "sync" | "sync_all" | "sync_data" => Some(PersistenceKind::Sync),
        "checkpoint" => Some(PersistenceKind::Checkpoint),
        "snapshot" => Some(PersistenceKind::Snapshot),
        _ => None,
    }
}

/// A free/path call's callee spelling, reduced to its last `::`-separated segment and classified
/// via `persistence_kind_for_spelling`.
fn persistence_candidate_kind(callee: &syn::Expr) -> Option<PersistenceKind> {
    let spelling = call_callee_spelling(callee);
    let last_segment = spelling.rsplit("::").next().unwrap_or(&spelling);
    persistence_kind_for_spelling(last_segment)
}

struct PersistenceWalker<'ctx, 'a> {
    ctx: &'ctx mut ExtractionContext<'a>,
    function: SemanticRecordId,
    scope: SemanticScope,
}

impl<'ctx, 'a> PersistenceWalker<'ctx, 'a> {
    fn emit_candidate(&mut self, span: atlas_core::SourceSpan, kind: PersistenceKind) {
        let subject = PersistenceIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            kind,
            span: span.clone(),
            // This wave's only mode: a spelling candidate names no resolvable place, and never
            // fabricates one -- see the module doc comment and the R4.9 OwnershipTarget lesson.
            place: PlaceRef::Unresolved,
            resolution: PersistenceResolution::Unresolved,
        };
        let record_id =
            SemanticRecordId::new(SemanticDimension::Persistence, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:persistence",
                self.ctx.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.ctx.record_dimension_hit(
            SemanticDimension::Persistence,
            record_id.clone(),
            evidence_id.clone(),
        ) {
            return;
        }
        self.ctx.push_evidence(
            &evidence_id,
            format!(
                "parsed persistence candidate {} at {}:{}:{}",
                subject.kind.as_str(),
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Persistence,
            status: EpistemicStatus::Inferred,
            subject,
            scope: self.scope.clone(),
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            extractor: self.ctx.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.ctx.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::Persistence(header);
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
            syn::Expr::Call(call) => {
                if let Some(kind) = persistence_candidate_kind(&call.func) {
                    let span = self.ctx.span_of(call);
                    self.emit_candidate(span, kind);
                }
                self.walk_expr(&call.func);
                for arg in &call.args {
                    self.walk_expr(arg);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                let kind = persistence_kind_for_spelling(&method_call.method.to_string());
                if let Some(kind) = kind {
                    let span = self.ctx.span_of(method_call);
                    self.emit_candidate(span, kind);
                }
                self.walk_expr(&method_call.receiver);
                for arg in &method_call.args {
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
            syn::Expr::Await(await_expr) => self.walk_expr(&await_expr.base),
            syn::Expr::Cast(cast) => self.walk_expr(&cast.expr),
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
            // executable region -- a persistence-shaped call inside one is a real site of the
            // enclosing function.
            syn::Expr::Unsafe(unsafe_expr) => self.walk_block(&unsafe_expr.block),
            syn::Expr::Const(const_expr) => self.walk_block(&const_expr.block),
            syn::Expr::TryBlock(try_block) => self.walk_block(&try_block.block),
            syn::Expr::Repeat(repeat) => {
                self.walk_expr(&repeat.expr);
                self.walk_expr(&repeat.len);
            }
            syn::Expr::RawAddr(raw_addr) => self.walk_expr(&raw_addr.expr),
            // `yield value` evaluates `value` immediately in the SAME executable region.
            syn::Expr::Yield(yield_expr) => {
                if let Some(value) = &yield_expr.expr {
                    self.walk_expr(value);
                }
            }
            // Closures and `async { .. }` blocks get no persistence attribution of their own this
            // wave (consistent with R4.6-R4.10 -- both are separate deferred executable regions;
            // see `ExecutableRegionIdentity`'s TARGET status in `dimension_coverage`); a bare macro
            // invocation's arguments are opaque token streams this extractor never re-parses (a
            // real, permanent gap -- see `dimension_coverage`); path/literal/other forms carry no
            // nested expressions this walker tracks.
            _ => {}
        }
    }
}

impl<'a> ExtractionContext<'a> {
    /// Walks one function/method body, emitting an `Inferred` `PersistenceKind` candidate for
    /// every call/method-call whose callee spelling matches `commit`/`flush`/`sync`/`sync_all`/
    /// `sync_data`/`checkpoint`/`snapshot`, reachable anywhere in it. No-op when `PERSISTENCE` was
    /// not requested.
    pub(super) fn build_persistence(
        &mut self,
        body: &syn::Block,
        scope: &SemanticScope,
        function: &SemanticRecordId,
    ) {
        if !self.wants(SemanticDimension::Persistence) {
            return;
        }
        let mut walker = PersistenceWalker {
            ctx: self,
            function: function.clone(),
            scope: scope.clone(),
        };
        walker.walk_block(body);
    }
}
