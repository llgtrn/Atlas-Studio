//! R4.8: conservative panic-like and filesystem effect-site candidates for one function/method
//! body.
//!
//! A textual macro name panic!/unreachable!/todo!/unimplemented! is useful evidence but is not
//! sufficient to prove the invoked macro resolves to Rust's standard panic behavior: macro
//! bindings may be shadowed and this extractor performs no macro/name resolution. Such sites are
//! therefore emitted as EffectCategory::Panic with EpistemicStatus::Inferred, never OBSERVED.
//!
//! A free/path function call whose callee spelling has an `fs` module qualifier (`std::fs::read`,
//! `fs::write`, `tokio::fs::read_dir`, ...) or is exactly `File::open`/`File::create` is, by the
//! identical textual-spelling-candidate discipline `persistence.rs` already established for R4.11,
//! emitted as an INFERRED FilesystemRead/FilesystemWrite candidate -- see
//! `filesystem_kind_for_free_call_spelling`'s own doc comment for the exact closed set and why
//! method calls (`file.read_to_string(..)`, `socket.write_all(..)`) are deliberately excluded this
//! wave: a bare method name carries no qualifying module/type the way a free/path call's spelling
//! does, making it far more likely to collide with an unrelated type's own identically-named
//! method (exactly the ambiguity `persistence.rs`'s own module doc comment already names for its
//! own method-call spellings) -- an explicit, named scope boundary, not a silent omission.
//!
//! NetworkSend/NetworkReceive/ProcessSpawn/FfiCall/Persist/EmitEvent/AuthCheck/Alloc/Free/
//! ExternalIo remain unmaterialized until deeper API/type resolution exists or a similarly
//! conservative spelling-candidate scheme is designed for each. Unsafe/try/repeat/raw-address/
//! yield expression containers are traversed; closures, async blocks, const blocks and macro token
//! bodies remain explicit closure gaps, so the EFFECT obligation stays UNKNOWN even when useful
//! effect observations are present.

use atlas_core::{
    EffectCategory, EffectIdentity, EpistemicStatus, EvidenceId, SemanticDimension,
    SemanticObservation, SemanticRecordHeader, SemanticRecordId, SemanticScope, stable_id,
};

use super::ExtractionContext;
use super::spelling::{call_callee_spelling, is_panic_like_macro};

/// If a free/path call's callee spelling has an `fs` module qualifier, or is exactly
/// `File::open`/`File::create`/`File::create_new`, returns the effect category that specific,
/// well-known spelling most directly implies. Checks the segment immediately before the final one
/// for an EXACT match against `fs`/`File` (bounded by the `::` path separator on both sides, or at
/// the start of the spelling) -- never a substring match, so a module merely containing the letters
/// "fs" (`prefs`, `overlayfs`, `myfs`, ...) can never collide, the same word-boundary discipline
/// `max_structural_recursion_risk`'s own `as`-keyword scan already established for an identical
/// collision risk. Every match is equally uncertain (no type/name resolution proves the qualifier
/// actually resolves to `std::fs`/`std::fs::File` rather than a same-named local module or type),
/// so all map to `Inferred` alike, exactly like `persistence.rs`'s own closed spelling set.
fn filesystem_kind_for_free_call_spelling(spelling: &str) -> Option<EffectCategory> {
    let segments: Vec<&str> = spelling.split("::").collect();
    let last = *segments.last()?;
    let qualifier = (segments.len() >= 2).then(|| segments[segments.len() - 2]);
    match qualifier {
        Some("fs") => match last {
            "read" | "read_to_string" | "read_to_end" | "read_dir" | "read_link" | "metadata"
            | "symlink_metadata" | "canonicalize" => Some(EffectCategory::FilesystemRead),
            "write" | "create_dir" | "create_dir_all" | "remove_file" | "remove_dir"
            | "remove_dir_all" | "rename" | "copy" | "hard_link" | "symlink"
            | "set_permissions" => Some(EffectCategory::FilesystemWrite),
            _ => None,
        },
        Some("File") => match last {
            "open" => Some(EffectCategory::FilesystemRead),
            "create" | "create_new" => Some(EffectCategory::FilesystemWrite),
            _ => None,
        },
        _ => None,
    }
}

struct EffectWalker<'ctx, 'a> {
    ctx: &'ctx mut ExtractionContext<'a>,
    function: SemanticRecordId,
    scope: SemanticScope,
}

impl<'ctx, 'a> EffectWalker<'ctx, 'a> {
    fn emit(&mut self, span: atlas_core::SourceSpan, category: EffectCategory) {
        let subject = EffectIdentity {
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            function: self.function.clone(),
            category,
            span: span.clone(),
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Effect, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:effect",
                self.ctx.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.ctx.record_dimension_hit(
            SemanticDimension::Effect,
            record_id.clone(),
            evidence_id.clone(),
        ) {
            return;
        }
        self.ctx.push_evidence(
            &evidence_id,
            format!(
                "parsed effect candidate {} at {}:{}:{}",
                subject.category.as_str(),
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id: record_id.clone(),
            dimension: SemanticDimension::Effect,
            status: EpistemicStatus::Inferred,
            subject,
            scope: self.scope.clone(),
            repository: self.ctx.input.repository.clone(),
            revision: self.ctx.input.revision.clone(),
            extractor: self.ctx.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.ctx.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::Effect(header);
        debug_assert!(observation.is_dimension_consistent());
        self.ctx.observations.push(observation);
    }

    fn emit_panic(&mut self, mac: &syn::Macro) {
        let span = self.ctx.span_of(mac);
        self.emit(span, EffectCategory::Panic);
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
            syn::Stmt::Macro(stmt_macro) => {
                if is_panic_like_macro(&stmt_macro.mac) {
                    self.emit_panic(&stmt_macro.mac);
                }
            }
            syn::Stmt::Item(_) => {}
        }
    }

    fn walk_expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Macro(expr_macro) => {
                if is_panic_like_macro(&expr_macro.mac) {
                    self.emit_panic(&expr_macro.mac);
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
            syn::Expr::Call(call) => {
                if let Some(category) =
                    filesystem_kind_for_free_call_spelling(&call_callee_spelling(&call.func))
                {
                    let span = self.ctx.span_of(call);
                    self.emit(span, category);
                }
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
            // future non-exhaustive syn variants remain open EFFECT obligations.
            syn::Expr::Closure(_) | syn::Expr::Async(_) | syn::Expr::Const(_) => {}
            _ => {}
        }
    }
}

impl<'a> ExtractionContext<'a> {
    /// Walks one function/method body, emitting an INFERRED Panic Effect candidate for every
    /// panic-like macro spelling reachable in expression forms this wave can soundly attribute to
    /// the enclosing function. No-op when `EFFECT` was not requested.
    pub(super) fn build_effects(
        &mut self,
        body: &syn::Block,
        scope: &SemanticScope,
        function: &SemanticRecordId,
    ) {
        if !self.wants(SemanticDimension::Effect) {
            return;
        }
        let mut walker = EffectWalker {
            ctx: self,
            function: function.clone(),
            scope: scope.clone(),
        };
        walker.walk_block(body);
    }
}
