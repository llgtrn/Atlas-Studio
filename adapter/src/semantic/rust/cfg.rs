//! R4.6: real structured-CFG lowering for one function/method body.
//!
//! Unlike R4.5's CALL walker (which only ever *finds* call sites without resolving them), this
//! builder computes real successor edges: which branch of an `if`/`match` is taken, where a loop
//! repeats to, where a `return`/`break`/panic actually leaves to. That is sound here specifically
//! because CFG STRUCTURE is fully determined by Rust's own syntax and language semantics -- unlike
//! a call's target, it never requires name/type resolution this extractor doesn't have. See the
//! module doc comment on `core::semantic::control_flow` for the exact scope (statement-level
//! constructs only; closures get no CFG of their own) and rationale.

use atlas_core::{
    ControlFlowBlockIdentity, ControlFlowBlockKind, ControlFlowEdge, ControlFlowEdgeKind,
    EpistemicStatus, EvidenceId, SemanticDimension, SemanticObservation, SemanticRecordHeader,
    SemanticRecordId, SemanticScope, stable_id,
};

use super::ExtractionContext;
use super::spelling::is_panic_like_macro;

/// Where control goes when a straight-line statement sequence completes without an early exit.
#[derive(Clone)]
enum Continuation {
    /// Jump to a specific, already-identified block.
    FallthroughTo(SemanticRecordId),
    /// Leave the function normally (function-body top-level completion).
    Return,
    /// Jump back to a loop body's own entry (used when a loop is the last thing in an enclosing
    /// sequence with nothing following it -- that sequence's own "fell off the end" behavior IS
    /// the loop repeating, composed from the outer continuation).
    LoopRepeat(SemanticRecordId),
}

fn continuation_target(cont: &Continuation) -> Option<SemanticRecordId> {
    match cont {
        Continuation::FallthroughTo(id) | Continuation::LoopRepeat(id) => Some(id.clone()),
        Continuation::Return => None,
    }
}

fn continuation_to_edge(cont: &Continuation) -> ControlFlowEdge {
    match cont {
        Continuation::FallthroughTo(id) => ControlFlowEdge {
            kind: ControlFlowEdgeKind::Fallthrough,
            target: Some(id.clone()),
        },
        Continuation::LoopRepeat(id) => ControlFlowEdge {
            kind: ControlFlowEdgeKind::LoopRepeat,
            target: Some(id.clone()),
        },
        Continuation::Return => ControlFlowEdge {
            kind: ControlFlowEdgeKind::Return,
            target: None,
        },
    }
}

/// One loop (or labeled loop) currently enclosing the statement being lowered, used to resolve
/// `break`/`continue` -- including labeled ones -- to a concrete edge.
struct LoopFrame {
    label: Option<String>,
    /// Where a `continue` (or the loop body's own natural completion) goes: the loop body's entry.
    repeat_target: SemanticRecordId,
    /// Where a `break` goes: whatever this loop's own enclosing continuation was.
    after_loop: Continuation,
}

fn stmt_expr(stmt: &syn::Stmt) -> Option<&syn::Expr> {
    match stmt {
        syn::Stmt::Expr(expr, _) => Some(expr),
        _ => None,
    }
}

fn label_name(label: Option<&syn::Label>) -> Option<String> {
    label.map(|label| label.name.ident.to_string())
}

fn lifetime_name(lifetime: Option<&syn::Lifetime>) -> Option<String> {
    lifetime.map(|lifetime| lifetime.ident.to_string())
}

struct CfgBuilder<'ctx, 'a> {
    ctx: &'ctx mut ExtractionContext<'a>,
    function: SemanticRecordId,
    scope: SemanticScope,
    loop_stack: Vec<LoopFrame>,
    next_index: usize,
}

impl<'ctx, 'a> CfgBuilder<'ctx, 'a> {
    fn reserve_index(&mut self) -> usize {
        let index = self.next_index;
        self.next_index += 1;
        index
    }

    /// The record_id for `index` within this function. Pure function of `index` alone (kind/
    /// is_entry/successors never participate in `identity_key()`), so this can be called to get a
    /// forward reference before that block's own content is finalized.
    fn block_id_for(&self, index: usize) -> SemanticRecordId {
        let placeholder = ControlFlowBlockIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            block_index: index,
            kind: ControlFlowBlockKind::Continuation,
            is_entry: false,
            successors: Vec::new(),
        };
        SemanticRecordId::new(SemanticDimension::ControlFlow, &placeholder.identity_key())
    }

    fn resolve_break(&self, label: Option<&syn::Lifetime>) -> ControlFlowEdge {
        let name = lifetime_name(label);
        let frame = match &name {
            Some(name) => self
                .loop_stack
                .iter()
                .rev()
                .find(|frame| frame.label.as_deref() == Some(name.as_str())),
            None => self.loop_stack.last(),
        };
        match frame {
            Some(frame) => ControlFlowEdge {
                kind: ControlFlowEdgeKind::Break,
                target: continuation_target(&frame.after_loop),
            },
            None => ControlFlowEdge {
                kind: ControlFlowEdgeKind::Unresolved,
                target: None,
            },
        }
    }

    fn resolve_continue(&self, label: Option<&syn::Lifetime>) -> ControlFlowEdge {
        let name = lifetime_name(label);
        let frame = match &name {
            Some(name) => self
                .loop_stack
                .iter()
                .rev()
                .find(|frame| frame.label.as_deref() == Some(name.as_str())),
            None => self.loop_stack.last(),
        };
        match frame {
            Some(frame) => ControlFlowEdge {
                kind: ControlFlowEdgeKind::LoopRepeat,
                target: Some(frame.repeat_target.clone()),
            },
            None => ControlFlowEdge {
                kind: ControlFlowEdgeKind::Unresolved,
                target: None,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_block(
        &mut self,
        record_id: SemanticRecordId,
        block_index: usize,
        kind: ControlFlowBlockKind,
        is_entry: bool,
        successors: Vec<ControlFlowEdge>,
        span: atlas_core::SourceSpan,
        status: EpistemicStatus,
    ) {
        let subject = ControlFlowBlockIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            block_index,
            kind,
            is_entry,
            successors,
        };
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:control-flow",
                self.ctx.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.ctx.record_dimension_hit(
            SemanticDimension::ControlFlow,
            record_id.clone(),
            evidence_id.clone(),
        ) {
            return;
        }
        self.ctx.push_evidence(
            &evidence_id,
            format!(
                "parsed control-flow block {} (block_index={block_index}) at {}:{}:{}",
                subject.kind.as_str(),
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::ControlFlow,
            status,
            subject,
            scope: self.scope.clone(),
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            extractor: self.ctx.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.ctx.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::ControlFlow(header);
        debug_assert!(observation.is_dimension_consistent());
        self.ctx.observations.push(observation);
    }

    /// Lowers `stmts` into one or more CFG blocks, starting at the already-reserved
    /// `(entry_id, entry_index)`. `cont` is where control goes if this statement list completes
    /// without hitting an early exit or a divergent tail. Emits blocks as a side effect; the
    /// entry block itself is always emitted exactly once by this call (at one of its `return`
    /// points), whichever shape its termination turns out to have.
    #[allow(clippy::too_many_arguments)]
    fn lower_stmts(
        &mut self,
        stmts: &[syn::Stmt],
        cont: Continuation,
        entry_id: SemanticRecordId,
        entry_index: usize,
        entry_kind: ControlFlowBlockKind,
        is_entry: bool,
        span: atlas_core::SourceSpan,
    ) {
        for (i, stmt) in stmts.iter().enumerate() {
            if let syn::Stmt::Macro(stmt_macro) = stmt {
                if is_panic_like_macro(&stmt_macro.mac) {
                    self.emit_block(
                        entry_id,
                        entry_index,
                        entry_kind,
                        is_entry,
                        vec![ControlFlowEdge {
                            kind: ControlFlowEdgeKind::Panic,
                            target: None,
                        }],
                        span,
                        // A textual panic-like macro name can be shadowed by a local
                        // `macro_rules!` redefinition that may not actually diverge; this
                        // extractor has no macro/name resolution to rule that out (matching
                        // EFFECT's identical treatment of the same evidence -- see
                        // `is_panic_like_macro`'s doc comment). The whole block's shape genuinely
                        // depends on this classification: if the macro doesn't diverge, this
                        // block's real successor is whatever follows, not `Panic`. So the block's
                        // status is `Inferred`, not just the edge's kind.
                        EpistemicStatus::Inferred,
                    );
                    return;
                }
                continue;
            }
            let Some(expr) = stmt_expr(stmt) else {
                continue;
            };
            match expr {
                syn::Expr::Return(_) => {
                    self.emit_block(
                        entry_id,
                        entry_index,
                        entry_kind,
                        is_entry,
                        vec![ControlFlowEdge {
                            kind: ControlFlowEdgeKind::Return,
                            target: None,
                        }],
                        span,
                        EpistemicStatus::Observed,
                    );
                    return;
                }
                syn::Expr::Break(brk) => {
                    let edge = self.resolve_break(brk.label.as_ref());
                    self.emit_block(
                        entry_id,
                        entry_index,
                        entry_kind,
                        is_entry,
                        vec![edge],
                        span,
                        EpistemicStatus::Observed,
                    );
                    return;
                }
                syn::Expr::Continue(cont_expr) => {
                    let edge = self.resolve_continue(cont_expr.label.as_ref());
                    self.emit_block(
                        entry_id,
                        entry_index,
                        entry_kind,
                        is_entry,
                        vec![edge],
                        span,
                        EpistemicStatus::Observed,
                    );
                    return;
                }
                syn::Expr::Macro(expr_macro) if is_panic_like_macro(&expr_macro.mac) => {
                    self.emit_block(
                        entry_id,
                        entry_index,
                        entry_kind,
                        is_entry,
                        vec![ControlFlowEdge {
                            kind: ControlFlowEdgeKind::Panic,
                            target: None,
                        }],
                        span,
                        // See the identical rationale on the `syn::Stmt::Macro` panic arm above.
                        EpistemicStatus::Inferred,
                    );
                    return;
                }
                syn::Expr::If(_)
                | syn::Expr::Match(_)
                | syn::Expr::Loop(_)
                | syn::Expr::While(_)
                | syn::Expr::ForLoop(_)
                | syn::Expr::Block(_) => {
                    let remaining = &stmts[i + 1..];
                    let join_cont = if remaining.is_empty() {
                        cont
                    } else {
                        let join_index = self.reserve_index();
                        let join_id = self.block_id_for(join_index);
                        let join_span = self.ctx.span_of(&remaining[0]);
                        self.lower_stmts(
                            remaining,
                            cont,
                            join_id.clone(),
                            join_index,
                            ControlFlowBlockKind::Continuation,
                            false,
                            join_span,
                        );
                        Continuation::FallthroughTo(join_id)
                    };
                    let successors = self.lower_divergent(expr, join_cont);
                    self.emit_block(
                        entry_id,
                        entry_index,
                        entry_kind,
                        is_entry,
                        successors,
                        span,
                        EpistemicStatus::Observed,
                    );
                    return;
                }
                _ => continue,
            }
        }
        self.emit_block(
            entry_id,
            entry_index,
            entry_kind,
            is_entry,
            vec![continuation_to_edge(&cont)],
            span,
            EpistemicStatus::Observed,
        );
    }

    fn lower_divergent(
        &mut self,
        expr: &syn::Expr,
        join_cont: Continuation,
    ) -> Vec<ControlFlowEdge> {
        match expr {
            syn::Expr::If(if_expr) => self.lower_if(if_expr, join_cont),
            syn::Expr::Match(match_expr) => self.lower_match(match_expr, join_cont),
            syn::Expr::Loop(loop_expr) => self.lower_loop(
                &loop_expr.body,
                loop_expr.label.as_ref(),
                join_cont,
                ControlFlowBlockKind::LoopBody,
                false,
            ),
            syn::Expr::While(while_expr) => self.lower_loop(
                &while_expr.body,
                while_expr.label.as_ref(),
                join_cont,
                ControlFlowBlockKind::WhileBody,
                true,
            ),
            syn::Expr::ForLoop(for_loop) => self.lower_loop(
                &for_loop.body,
                for_loop.label.as_ref(),
                join_cont,
                ControlFlowBlockKind::ForLoopBody,
                true,
            ),
            syn::Expr::Block(block_expr) => {
                let index = self.reserve_index();
                let id = self.block_id_for(index);
                let span = self.ctx.span_of(&block_expr.block);
                self.lower_stmts(
                    &block_expr.block.stmts,
                    join_cont,
                    id.clone(),
                    index,
                    ControlFlowBlockKind::NestedBlockExpr,
                    false,
                    span,
                );
                vec![ControlFlowEdge {
                    kind: ControlFlowEdgeKind::Branch,
                    target: Some(id),
                }]
            }
            _ => {
                unreachable!("lower_divergent is only called for If/Match/Loop/While/ForLoop/Block")
            }
        }
    }

    fn lower_if(&mut self, if_expr: &syn::ExprIf, join_cont: Continuation) -> Vec<ControlFlowEdge> {
        let mut edges = Vec::new();

        let then_index = self.reserve_index();
        let then_id = self.block_id_for(then_index);
        let then_span = self.ctx.span_of(&if_expr.then_branch);
        self.lower_stmts(
            &if_expr.then_branch.stmts,
            join_cont.clone(),
            then_id.clone(),
            then_index,
            ControlFlowBlockKind::IfThen,
            false,
            then_span,
        );
        edges.push(ControlFlowEdge {
            kind: ControlFlowEdgeKind::Branch,
            target: Some(then_id),
        });

        match &if_expr.else_branch {
            Some((_, else_expr)) => match else_expr.as_ref() {
                syn::Expr::Block(block_expr) => {
                    let else_index = self.reserve_index();
                    let else_id = self.block_id_for(else_index);
                    let else_span = self.ctx.span_of(&block_expr.block);
                    self.lower_stmts(
                        &block_expr.block.stmts,
                        join_cont,
                        else_id.clone(),
                        else_index,
                        ControlFlowBlockKind::IfElse,
                        false,
                        else_span,
                    );
                    edges.push(ControlFlowEdge {
                        kind: ControlFlowEdgeKind::Branch,
                        target: Some(else_id),
                    });
                }
                syn::Expr::If(nested_if) => {
                    // else-if chain: the nested condition's own true/false destinations are
                    // real successors of THIS decision point too (no code sits between this
                    // `if` and the nested one), so they are flattened into the same edge list
                    // rather than given their own intermediate node -- consistent with how the
                    // condition expression itself is never modeled as a node either.
                    edges.extend(self.lower_if(nested_if, join_cont));
                }
                // `syn::ExprIf.else_branch`'s grammar only ever produces Block or If; anything
                // else cannot occur from valid `syn` parsing, but untrusted input is never
                // trusted to panic on, so this stays an explicit Unresolved rather than a
                // fabricated guess.
                _ => edges.push(ControlFlowEdge {
                    kind: ControlFlowEdgeKind::Unresolved,
                    target: None,
                }),
            },
            None => edges.push(continuation_to_edge(&join_cont)),
        }

        edges
    }

    fn lower_match(
        &mut self,
        match_expr: &syn::ExprMatch,
        join_cont: Continuation,
    ) -> Vec<ControlFlowEdge> {
        let mut edges = Vec::with_capacity(match_expr.arms.len());
        for arm in &match_expr.arms {
            let index = self.reserve_index();
            let id = self.block_id_for(index);
            let span = self.ctx.span_of(&arm.body);
            match arm.body.as_ref() {
                syn::Expr::Block(block_expr) => {
                    self.lower_stmts(
                        &block_expr.block.stmts,
                        join_cont.clone(),
                        id.clone(),
                        index,
                        ControlFlowBlockKind::MatchArm,
                        false,
                        span,
                    );
                }
                other => {
                    let synthetic = [syn::Stmt::Expr(other.clone(), None)];
                    self.lower_stmts(
                        &synthetic,
                        join_cont.clone(),
                        id.clone(),
                        index,
                        ControlFlowBlockKind::MatchArm,
                        false,
                        span,
                    );
                }
            }
            edges.push(ControlFlowEdge {
                kind: ControlFlowEdgeKind::Branch,
                target: Some(id),
            });
        }
        edges
    }

    /// `may_not_enter`: `true` for `while`/`for` (the condition/iterator may be false/empty on
    /// the very first check, so control may reach `after_loop` without ever entering the body);
    /// `false` for a bare `loop { .. }`, which always executes its body at least once.
    fn lower_loop(
        &mut self,
        body: &syn::Block,
        label: Option<&syn::Label>,
        after_loop: Continuation,
        kind: ControlFlowBlockKind,
        may_not_enter: bool,
    ) -> Vec<ControlFlowEdge> {
        let body_index = self.reserve_index();
        let body_id = self.block_id_for(body_index);
        self.loop_stack.push(LoopFrame {
            label: label_name(label),
            repeat_target: body_id.clone(),
            after_loop: after_loop.clone(),
        });
        let span = self.ctx.span_of(body);
        self.lower_stmts(
            &body.stmts,
            Continuation::LoopRepeat(body_id.clone()),
            body_id.clone(),
            body_index,
            kind,
            false,
            span,
        );
        self.loop_stack.pop();

        let mut edges = vec![ControlFlowEdge {
            kind: ControlFlowEdgeKind::Branch,
            target: Some(body_id),
        }];
        if may_not_enter {
            edges.push(continuation_to_edge(&after_loop));
        }
        edges
    }
}

impl<'a> ExtractionContext<'a> {
    /// Builds and emits the full CFG for one function/method body, attributed to `function` (the
    /// enclosing function's `FunctionIdentity` record_id). No-op when `CONTROL_FLOW` was not
    /// requested.
    pub(super) fn build_control_flow(
        &mut self,
        body: &syn::Block,
        scope: &SemanticScope,
        function: &SemanticRecordId,
    ) {
        if !self.wants(SemanticDimension::ControlFlow) {
            return;
        }
        let mut builder = CfgBuilder {
            ctx: self,
            function: function.clone(),
            scope: scope.clone(),
            loop_stack: Vec::new(),
            next_index: 0,
        };
        let entry_index = builder.reserve_index();
        let entry_id = builder.block_id_for(entry_index);
        let span = builder.ctx.span_of(body);
        builder.lower_stmts(
            &body.stmts,
            Continuation::Return,
            entry_id,
            entry_index,
            ControlFlowBlockKind::FunctionEntry,
            true,
            span,
        );
    }
}
