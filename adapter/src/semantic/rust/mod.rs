//! Real Rust semantic extractor (R4.3-R4.11).
//!
//! Scope is all twelve dimensions: SYMBOL, TYPE, FUNCTION_IDENTITY, FUNCTION_SIGNATURE, CALL,
//! CONTROL_FLOW, DATA_FLOW, STATE, EFFECT, OWNERSHIP, CONCURRENCY, PERSISTENCE.
//!
//! Parses `input.source_text` with `syn` (a real Rust parser, not regex/ad-hoc text scanning).
//! Parsing untrusted source text never authorizes executing it: this extractor never runs
//! build.rs, proc macros, `cargo build`/`test`, repository binaries, or shell/install scripts, and
//! never makes network calls (`.atlas/contracts/SEMANTIC-EXTRACTION.md`).
//!
//! Epistemic discipline: only what the parser can literally observe in source syntax is recorded,
//! and always as `EpistemicStatus::Observed` evidence, never fabricated. Compiler-resolved
//! semantics — canonical type identity, macro expansion, trait/impl equivalence, name resolution
//! across modules — are not provable from text alone and are never claimed:
//! `TypeIdentity.canonical` stays `None` for every observation this extractor produces, and every
//! `CALL` observation (R4.5) stays `CallDispatchKind::Unresolved` with an empty `callees` list --
//! this extractor has no `use`-import tracking or type inference, so it can soundly observe WHERE
//! a call syntactically occurs and WHO makes it, never WHOM it calls (whole-workspace name
//! resolution of path calls is a separate engine with its own identity, `resolve.rs`, G75).
//! CONTROL_FLOW (R4.6) and
//! DATA_FLOW (R4.7) are different in kind: a function's control-flow structure and its local
//! def-use bindings are both fully determined by Rust's own syntax and scoping rules, not by
//! name/type resolution, so real successor edges (see `cfg.rs`) and real local def-use resolution
//! (see `dataflow.rs`) ARE computed -- `ControlFlowEdgeKind::Unresolved`/`DataFlowResolution::
//! Unresolved` are used only for genuinely ambiguous/out-of-local-scope cases, never as a blanket
//! default. STATE (R4.8, see `state.rs`) is narrowly scoped to single-level `self.<field>`
//! read/write, including the read+write semantics of compound assignment. EFFECT (R4.8, see
//! `effect.rs`) recognizes panic-like macro spellings as INFERRED candidates rather than
//! OBSERVED panic effects because textual macro names can be shadowed and this extractor does not
//! perform macro/name resolution. Every other `EffectCategory` likewise requires deeper
//! resolution. STATE/EFFECT preserve useful observations while their per-dimension obligation
//! remains UNKNOWN until the declared R4.8 profile has real closure; zero observations are never
//! misreported as verified absence. OWNERSHIP (R4.9, see `ownership.rs`) splits the same way CALL
//! does: `&`/`&mut` borrow sites are fully syntax-determined (real `BorrowShared`/`BorrowMut`), but
//! whether a bare identifier used by value is actually moved or merely copied depends on its
//! type's `Copy`-ness, which this extractor cannot resolve -- `OwnershipKind::MoveOrCopy` names
//! that gap explicitly rather than guessing. CONCURRENCY (R4.10, see `concurrency.rs`) only emits
//! `Await` (dedicated `.await` syntax, fully determined) and `Spawn` (callee spelling ending in
//! `spawn`, the same name-based risk class EFFECT already accepts for panic macros);
//! `Lock`/`Unlock`/channel/atomic operations would require resolving a method call to a specific
//! known API and are never emitted this wave. PERSISTENCE (R4.11, see `persistence.rs`) has no
//! dedicated syntax at all and no resolved-API adapter, so every candidate is a textual
//! callee-spelling guess (`commit`/`flush`/`sync`/`sync_all`/`sync_data`/`checkpoint`/`snapshot`),
//! always `Inferred` with an unresolved `PlaceRef` -- see `core::semantic::persistence`'s module
//! doc comment for why a durable-state target is never derived from spelling. A malformed file
//! never silently disappears: parse failure yields a `ParseFailure` diagnostic plus
//! explicit `UNKNOWN` for all supported dimensions, with the artifact still represented.

mod cfg;
mod concurrency;
mod dataflow;
mod effect;
mod macros;
mod ownership;
mod persistence;
pub mod resolve;
mod spelling;
mod state;

use std::collections::{BTreeMap, BTreeSet};

use atlas_core::{
    CallDispatchKind, CallSiteIdentity, DataFlowResolution, EpistemicStatus, Evidence, EvidenceId,
    FunctionDeclarationKind, FunctionIdentity, FunctionOwner, FunctionParameter, FunctionSignature,
    PlaceRef, Provenance, SemanticDimension, SemanticObservation, SemanticRecordHeader,
    SemanticRecordId, SemanticScope, SymbolIdentity, SymbolRole, TypeIdentity, ValueIdentity,
    ValueRole, stable_id,
};
use syn::spanned::Spanned;

use super::batch::{ExtractionBatch, ObligationResult};
use super::extractor::{DiagnosticCode, ExtractionDiagnostic, ExtractionInput, SemanticExtractor};

pub const RUST_SEMANTIC_EXTRACTOR_ID: &str = "atlas.rust.source-semantic.v1";
pub const RUST_SEMANTIC_EXTRACTOR_VERSION: &str = "0.1.0";

/// Exactly the twelve dimensions this wave observes from parser-visible syntax -- every
/// `SemanticDimension` variant that exists.
pub const SUPPORTED_DIMENSIONS: &[SemanticDimension] = &[
    SemanticDimension::Symbol,
    SemanticDimension::Type,
    SemanticDimension::FunctionIdentity,
    SemanticDimension::FunctionSignature,
    SemanticDimension::Call,
    SemanticDimension::ControlFlow,
    SemanticDimension::DataFlow,
    SemanticDimension::State,
    SemanticDimension::Effect,
    SemanticDimension::Ownership,
    SemanticDimension::Concurrency,
    SemanticDimension::Persistence,
];

/// Whether this extractor's declared profile for a dimension has been checked to visit every
/// syntactic form that could produce an observation, or whether real, named gaps remain.
///
/// This is the one place that answers "can this extractor legitimately prove negative absence for
/// this dimension" -- a wave label or test count must never substitute for this. See
/// `dimension_coverage` for the per-dimension reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DimensionCoverage {
    /// A zero-observation result for this dimension is real evidence of absence: this extractor's
    /// declared profile has been checked to walk every syntactic form that could produce one.
    Exhaustive,
    /// A named, real syntactic form exists that this extractor does not walk into. A
    /// zero-observation result never proves absence; the dimension's obligation stays UNKNOWN
    /// even when real observations exist.
    Partial,
}

/// Coverage classification for every dimension this extractor supports. An exhaustive match (not
/// a lookup table with a permissive default) so adding a new `SemanticDimension` variant forces an
/// explicit decision here rather than silently defaulting either way.
fn dimension_coverage(dimension: SemanticDimension) -> DimensionCoverage {
    use DimensionCoverage::{Exhaustive, Partial};
    match dimension {
        // Declaration-level: every `syn::Item` this file's top-level/nested-`mod` walk visits is
        // checked for a Symbol/Type/FunctionIdentity/FunctionSignature-shaped declaration. Real,
        // documented, permanent exclusions exist (compiler-generated functions, item-level
        // function-like macro-expanded declarations, monomorphized instances, resolved DefIds --
        // see R4.4's own verification record) -- but those are OUT_OF_PROFILE exclusions this
        // extractor never silently claims to cover, not unvisited reachable syntax within the
        // declared profile itself, so a zero-observation result over that declared profile is
        // real evidence.
        SemanticDimension::Symbol
        | SemanticDimension::Type
        | SemanticDimension::FunctionIdentity
        | SemanticDimension::FunctionSignature => Exhaustive,
        // Every full-expression-tree walker (CALL/CONTROL_FLOW/DATA_FLOW/STATE/EFFECT/OWNERSHIP/
        // CONCURRENCY/PERSISTENCE) shares one real, permanent gap: a bare `syn::Expr::Macro`
        // invocation's arguments are an opaque `TokenStream`, never re-parsed as expressions
        // without macro expansion (which this extractor never performs -- `.atlas/contracts/
        // SEMANTIC-EXTRACTION.md`). A call, move, state access, effect, borrow, concurrency or
        // persistence site written only inside a macro invocation's arguments (e.g.
        // `my_macro!(hidden_call())`) is therefore structurally invisible to all eight, regardless
        // of how complete each walker's own `syn::Expr` variant coverage otherwise is. CONTROL_FLOW
        // additionally never splits below statement level (its own module doc comment already
        // states this). PERSISTENCE additionally has no dedicated syntax at all (unlike
        // CONCURRENCY's `.await`) and no resolved-API adapter, so every candidate it emits is a
        // textual spelling guess -- see `persistence.rs`. None of the eight may claim a
        // zero-observation result as verified absence -- except CALL since G119 (ADR 0041): its
        // walker recovers standard macro arguments (`macros::recovered_arguments`), so a file with
        // no opaque macro or body-rewriting attribute (`macros::opaque_sites` == 0) is exhaustively
        // walked and `finish_success` claims its CALL coverage per file.
        SemanticDimension::Call
        | SemanticDimension::ControlFlow
        | SemanticDimension::DataFlow
        | SemanticDimension::State
        | SemanticDimension::Effect
        | SemanticDimension::Ownership
        | SemanticDimension::Concurrency
        | SemanticDimension::Persistence => Partial,
    }
}

/// The token a call site is anchored at (its `CallSiteIdentity.span`): the callee's name -- a
/// method call's method identifier, a path call's last path segment -- or, for a callee that is
/// not a path (`(self.f)(x)`, `make()()`), the argument list's opening parenthesis. Each of these
/// tokens belongs to exactly one call expression, so two call sites of one function can never
/// share an identity. The expression's start cannot serve: every call of a chain
/// (`a.b().c()`, `Foo::new().bar()`) starts at the same token, and anchoring there collapsed a
/// chain into one CALL record, silently erasing the others (measured G74: 3,888 of 18,035
/// syntactic call sites of this repository). The name token is also where a compiler-grade index
/// (SCIP, LSIF) anchors the reference to the callee, so an independent engine observing the same
/// call names the same claim.
fn call_anchor(call_like: &syn::Expr) -> proc_macro2::Span {
    match call_like {
        syn::Expr::MethodCall(method_call) => method_call.method.span(),
        syn::Expr::Call(call) => match &*call.func {
            syn::Expr::Path(path) => path.path.segments.last().map_or_else(
                || call.paren_token.span.open(),
                |segment| segment.ident.span(),
            ),
            _ => call.paren_token.span.open(),
        },
        other => unreachable!(
            "call_anchor called with a non-call expression: {:?}",
            std::mem::discriminant(other)
        ),
    }
}

#[derive(Debug, Default)]
pub struct RustSemanticExtractor;

impl SemanticExtractor for RustSemanticExtractor {
    fn id(&self) -> &'static str {
        RUST_SEMANTIC_EXTRACTOR_ID
    }

    fn version(&self) -> &'static str {
        RUST_SEMANTIC_EXTRACTOR_VERSION
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &["rust"]
    }

    fn supported_dimensions(&self) -> &'static [SemanticDimension] {
        SUPPORTED_DIMENSIONS
    }

    fn extract(&self, input: &ExtractionInput) -> ExtractionBatch {
        // Both `syn::parse_file` and this extractor's own `walk_item`/`walk_expr` recursion below
        // are recursive-descent traversals of the SAME adversarial-depth tree, so both are run on
        // a dedicated, generously large stack (see `EXTRACTION_STACK_SIZE`'s own doc comment for
        // why this closes a residual class of risk `max_structural_recursion_risk` alone cannot).
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(EXTRACTION_STACK_SIZE)
                .spawn_scoped(scope, || {
                    let mut ctx = ExtractionContext::new(input, self.identity());
                    let risk = max_structural_recursion_risk(&input.source_text);
                    if risk > MAX_STRUCTURAL_RECURSION_RISK {
                        return ctx.finish_resource_limit(risk);
                    }
                    match syn::parse_file(&input.source_text) {
                        Ok(file) => {
                            ctx.shadowed_macros = macros::local_macro_names(&file);
                            ctx.opaque_macro_sites =
                                Some(macros::opaque_sites(&file, &ctx.shadowed_macros));
                            let root_scope = SemanticScope::new(Vec::<String>::new());
                            for item in &file.items {
                                ctx.walk_item(item, &root_scope);
                            }
                            ctx.finish_success()
                        }
                        Err(error) => ctx.finish_parse_failure(&error),
                    }
                })
                .expect("spawning the extraction worker thread must not fail")
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        })
    }
}

/// Deliberately large: empirically confirmed (isolated real-process reproduction, not merely
/// theorized -- a throwaway `cargo run --example` harness spawning `syn::parse_file` on adversarial
/// bracket-nesting depths at varying `stack_size`s, deleted after this finding was recorded here,
/// exactly like every prior vector in this fix's history was confirmed out-of-process rather than
/// risking a real crash inside the shared test binary) that a 2MiB stack (matching the "reduced
/// test-thread stack" this module already cites) crashes at depth 50,000 -- reproducing the
/// documented baseline -- while 256MiB survives past depth 20,000 and fails by depth 25,000 in an
/// unoptimized debug build, roughly two orders of magnitude beyond every one of the four confirmed
/// crash vectors `max_structural_recursion_risk` documents (300/2,000/3,000/2,000 levels on a
/// default-sized stack). This does NOT make stack overflow impossible -- a sufficiently large stack is still
/// finite, and a stack overflow (in this thread or any other) still aborts the whole process, same
/// as today, since Rust's stack-overflow guard-page handler cannot be caught by `catch_unwind`
/// regardless of which thread it fires on. What this closes is the SPECIFIC residual risk
/// `max_structural_recursion_risk`'s own doc comment names: "a fifth, sixth, ... construct that
/// drives the same class of parser/AST recursion through some other keyword or syntax shape may
/// exist and would not necessarily be caught by this heuristic." Any such not-yet-discovered
/// vector must now ALSO reach roughly two orders of magnitude deeper than every vector found so
/// far before it can matter -- a categorically different, generic defense (more stack) layered
/// behind the existing specific one (fewer risky bytes admitted), not a replacement for it: the
/// pre-parse heuristic still rejects obviously-adversarial input cheaply, before ever paying the
/// cost of spawning this worker thread. On Linux/macOS `stack_size` reserves virtual address space
/// (lazily paged in), not committed physical memory, so this is not a meaningful per-call cost for
/// the overwhelming majority of real, non-adversarial source files this extractor actually parses.
const EXTRACTION_STACK_SIZE: usize = 256 * 1024 * 1024;

/// Adversarial guard against a stack-overflow denial-of-service: `syn` is a recursive-descent
/// parser with no built-in recursion-depth protection (verified against its 3.0.6 source: no
/// `stacker`-style stack-growth integration exists), and `MAX_SEMANTIC_BYTES` (`adapter::source`)
/// gates only on file *size*, never on structural recursion depth, so a small, well-under-the-
/// byte-limit file that drives deep parser recursion reaches this extractor unfiltered.
///
/// `stacker::maybe_grow` was tried and does NOT help here: it can only grow the stack at the
/// moment it is called, and calling it once around the outer `syn::parse_file`/walk boundary makes
/// no difference, because at that outer call there is always still plenty of headroom -- the
/// exhaustion happens many frames deeper, inside `syn`'s own recursive descent, which never calls
/// back out to request more stack. Only a pre-parse structural bound, checked before `syn` ever
/// runs, actually prevents the crash. Confirmed empirically (isolated real-process reproduction,
/// not merely theorized): even with an outer `stacker::maybe_grow(256KiB, 256MiB, ..)` wrapper, the
/// exact same adversarial input still aborted the process with `SIGABRT`.
///
/// Four independent constructs were confirmed (real-process, isolated reproduction) to drive this
/// recursion to a crash:
/// - explicit bracket nesting (`(((...)))`) -- 300 levels reliably overflowed a reduced (~2MB)
///   test-thread stack; 100 levels did not;
/// - a bracket-free chained binary-operator expression (`1+1+1+...`) -- 2,000 terms reliably
///   overflowed the same stack; 1,000 terms did not;
/// - an `if ... else if ... else if ... else { .. }` chain -- each arm's own braces are siblings,
///   not nested (bracket-nesting depth stays at 2 regardless of chain length), and there is no
///   operator-character run either (`if`/`else` are keywords), yet the AST is exactly as deeply
///   *recursive* as the bracket case (`Expr::If { then, else: Some(Box<Expr::If{..}>) }` nests one
///   level per arm) -- 3,000 arms reliably overflowed the same stack;
/// - a chained cast expression (`x as T as T as T ...`) -- found by direct adversarial testing
///   after noting `as` is a bare keyword with no punctuation signature at all (no bracket, no
///   tracked operator character), so it was invisible to every counter above it. `Expr::Cast`
///   wraps its base expression one level per `as` (`Expr::Cast { expr: Box<Expr>, ty: Box<Type> }`),
///   exactly the same right-nesting shape as the `if`/`else` chain -- 2,000 terms reliably
///   overflowed the same stack (isolated `cargo test` reproduction, default per-test thread stack,
///   `signal: 6, SIGABRT`); 1,000 terms did not.
///
/// `max_structural_recursion_risk` bounds all four with one combined metric rather than
/// maintaining separate ad-hoc scans, since all are fundamentally the same risk (parser recursion
/// depth proportional to admitted-input structure, regardless of which concrete syntax drives it):
/// bracket nesting increments/decrements a depth counter as before; a run of consecutive
/// expression-continuation operator bytes (arithmetic, logical, comparison, method-chain `.`, ...)
/// at the *current* bracket depth also increments a counter, reset whenever a bracket boundary is
/// crossed; a run of `else` keyword occurrences (not reset by brace boundaries, since chain arms
/// are siblings at the same bracket depth -- only by `;` or a new `fn`, approximating "next
/// function") increments a third counter; a run of word-boundary-checked `as` keyword occurrences
/// (unlike the coarse, boundary-unchecked `else`/`fn` scans, `as` is short enough that an
/// unchecked substring match would false-positive on ordinary identifiers containing it --
/// `task`, `class`, `database`, `phase`, `release`, ... -- so this one specific check verifies the
/// byte immediately before and after are not themselves identifier characters) shares the same
/// `chain_run` counter as the operator-byte run, since (like those operators, and unlike
/// `else`/`if`'s sibling braces) a real `as`-chain has no intervening bracket to reset it anyway.
/// The combined maximum is compared against one threshold.
///
/// This is a coarse, syntax-unaware, pre-parse text scan (no real keyword-boundary checking for
/// `else`/`fn`, no char-literal or byte-string exclusion) that can only ever over-count real
/// structural risk, never under-count -- so it can only be more conservative than strictly
/// necessary, never miss one of these four specific, confirmed vectors. It is explicitly NOT
/// claimed complete: a fifth, sixth, ... construct that drives the same class of parser/AST
/// recursion through some other keyword or syntax shape may exist and would not necessarily be
/// caught by this heuristic.
///
/// That specific residual risk is now substantially, though not absolutely, mitigated by a second,
/// independent, generic layer: `RustSemanticExtractor::extract` runs both `syn::parse_file` and
/// this extractor's own subsequent AST walk on a dedicated large stack (`EXTRACTION_STACK_SIZE`;
/// see its own doc comment for the empirical evidence). A not-yet-discovered fifth vector must now
/// reach roughly two orders of magnitude deeper before it can crash the process, rather than being
/// an open-ended risk the moment this text scan misses it. This raises the practical bar; it does
/// not make either this heuristic or the large stack a complete, formally-proven fix -- see
/// `EXTRACTION_STACK_SIZE`'s own doc comment for exactly what is and is not closed.
/// A fully complete fix would require either patching `syn` itself to grow its own stack during
/// recursion (impractical -- `syn` is a third-party dependency this bootstrap does not vendor or
/// fork) or replacing whole-file parsing with a from-scratch, formally depth-bounded parser (a
/// large undertaking, not attempted this wave). This repository's own entire real source corpus
/// never nests brackets deeper than 13, never chains more than a handful of operators in a row,
/// and has no `if`/`else` chain anywhere near this length.
///
/// `//` line comments and `"..."`/`r#"..."#`-style string literals ARE excluded from the scan
/// (`skip_string_literal`): this repository's own common documentation style uses long dash/equals
/// divider comments as section headers (`// --- 13. section name ----------------------------`),
/// and at least one file embeds a raw-string test fixture containing similar runs -- both were
/// found, by directly re-censusing this repository's own real source, to trip this guard despite
/// having zero relation to real parser structure. Excluding them is still safe in the same
/// direction as the rest of this heuristic (removing content can only ever LOWER a computed risk,
/// never hide a real one, since no comment or string literal content is ever fed to `syn` as code
/// either way) -- it does not weaken detection of any of the three confirmed vectors, all of which
/// occur in genuine code structure, never inside a comment or string.
const MAX_STRUCTURAL_RECURSION_RISK: usize = 64;

/// If `bytes[start..]` begins a plain `"..."` (with `\"`/`\\` escape handling) or raw
/// `r#"..."#`-style string literal, returns the index just past its closing delimiter. Returns
/// `None` when `bytes[start]` does not open a recognized string literal, in which case the caller
/// processes `bytes[start]` normally. An unterminated literal is treated as extending to end of
/// input -- safe, since no chain-inducing content past that point can matter either way.
fn skip_string_literal(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes[start] == b'"' {
        let mut index = start + 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => index = (index + 2).min(bytes.len()),
                b'"' => return Some(index + 1),
                _ => index += 1,
            }
        }
        return Some(index);
    }
    if bytes[start] == b'r' && matches!(bytes.get(start + 1), Some(b'"') | Some(b'#')) {
        let mut index = start + 1;
        let mut hashes = 0usize;
        while bytes.get(index) == Some(&b'#') {
            hashes += 1;
            index += 1;
        }
        if bytes.get(index) != Some(&b'"') {
            return None;
        }
        index += 1;
        loop {
            if index >= bytes.len() {
                return Some(index);
            }
            if bytes[index] == b'"' && bytes[index + 1..].starts_with(&b"#".repeat(hashes)[..]) {
                return Some(index + 1 + hashes);
            }
            index += 1;
        }
    }
    None
}

/// `true` for a byte that can appear inside a Rust identifier (ASCII alphanumeric or `_`). Used
/// only to word-boundary-check the `as`-keyword scan in `max_structural_recursion_risk` below --
/// deliberately ASCII-only, matching this whole scan's existing byte-oriented approach; a
/// non-ASCII identifier byte (Rust identifiers may contain Unicode) is never itself `a`/`s`, so it
/// cannot hide or fabricate an `as` match either way.
const fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn max_structural_recursion_risk(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut bracket_depth: usize = 0;
    let mut chain_run: usize = 0;
    let mut else_run: usize = 0;
    let mut max_risk: usize = 0;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if let Some(after) = skip_string_literal(bytes, index) {
            index = after;
            continue;
        }
        match byte {
            b'(' | b'{' | b'[' => {
                bracket_depth += 1;
                chain_run = 0;
                max_risk = max_risk.max(bracket_depth).max(else_run);
            }
            b')' | b'}' | b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                chain_run = 0;
            }
            b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'<' | b'>' | b'=' | b'!'
            | b'.' | b'?' => {
                chain_run += 1;
                max_risk = max_risk.max(bracket_depth + chain_run);
            }
            // `,` always separates SIBLING list items (function-call/struct-literal/tuple/generic
            // arguments) -- `syn` parses a `Punctuated<T, Comma>` list with a loop, not recursion,
            // so additional comma-separated siblings never add parser stack depth the way a
            // genuine chain (nested brackets, chained binary operators, if/else-if arms) does.
            // Counting `,` toward `chain_run` made an ordinary long argument list or struct
            // literal indistinguishable from a real recursive chain: this repository's own real
            // source (e.g. large struct-literal-heavy files) tripped exactly this false positive.
            // Resets `chain_run` only, not `else_run` -- a `,` can legitimately appear inside one
            // arm of a real if/else-if chain (e.g. a function call argument list) without that
            // arm's own commas being allowed to hide the chain's true length.
            b',' => {
                chain_run = 0;
            }
            b';' => {
                chain_run = 0;
                else_run = 0;
            }
            b'e' if bytes[index..].starts_with(b"else") => {
                else_run += 1;
                max_risk = max_risk.max(else_run);
            }
            b'f' if bytes[index..].starts_with(b"fn ") || bytes[index..].starts_with(b"fn(") => {
                else_run = 0;
            }
            // `as` (the cast keyword) is only two bytes, far more collision-prone as a bare
            // substring than `else`/`fn` -- an unchecked match would false-positive on every
            // ordinary identifier containing it (`task`, `class`, `database`, `phase`, ...), so
            // this is the one check in this scan that verifies real word boundaries on both sides.
            // Shares `chain_run` (not a dedicated counter): a genuine `x as T as T as ...` chain
            // has no intervening bracket to reset it, the same as the operator-byte run above.
            b'a' if bytes[index..].starts_with(b"as")
                && !index
                    .checked_sub(1)
                    .and_then(|i| bytes.get(i))
                    .is_some_and(|b| is_identifier_byte(*b))
                && !bytes.get(index + 2).is_some_and(|b| is_identifier_byte(*b)) =>
            {
                chain_run += 1;
                max_risk = max_risk.max(bracket_depth + chain_run);
            }
            _ => {}
        }
        // Deliberately NOT reset on newline for either chain_run or else_run: an adversary could
        // otherwise trivially evade this guard by inserting a newline between every chained
        // operator or `else` arm, which drives the exact same parser recursion depth as an
        // unbroken line.
        index += 1;
    }
    max_risk
}

fn nested_scope(scope: &SemanticScope, segment: &str) -> SemanticScope {
    let mut segments = scope.segments.clone();
    segments.push(segment.to_owned());
    SemanticScope { segments }
}

fn function_signature_identity_key(signature: &FunctionSignature) -> String {
    let params = signature
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}:{}",
                parameter.name,
                parameter.type_identity.identity_key()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let return_key = signature
        .return_type
        .as_ref()
        .map(TypeIdentity::identity_key)
        .unwrap_or_default();
    format!(
        "{}|params=[{params}]|return={return_key}|generics=[{}]|abi={}|vis={}|async={}|unsafe={}|extern={}",
        signature.function.identity_key(),
        signature.generics.join(","),
        signature.abi.as_deref().unwrap_or(""),
        signature.visibility,
        signature.is_async,
        signature.is_unsafe,
        signature.is_extern,
    )
}

/// Shared statement-level dispatch for a walker whose only per-statement work is walking nested
/// expressions: a `let` binding's initializer (and its `else` diverge arm) and a bare expression
/// statement, with item declarations and macro invocations ignored. Implement `walk_expr` and
/// this default `walk_stmt` handles the rest.
///
/// `ConcurrencyWalker`/`PersistenceWalker`/`StateWalker` all previously carried their own,
/// byte-for-byte-identical `walk_stmt` method -- three independently-maintained copies of the
/// same dispatch, found by this session's own duplicate-function-body sweep
/// (`.atlas/evidence/verification/duplicate-classification-logic-swept-clean.json` and its
/// follow-up permanent regression test in `runtime::tests`). `EffectWalker` needs a genuinely
/// different `walk_stmt` (it must also recognize a bare `Stmt::Macro` as a possible panic site,
/// per `is_panic_like_macro`) and correctly does not implement this trait -- the duplication this
/// trait removes was real, not a case where every walker secretly needed the same behavior.
trait StatementWalker {
    fn walk_expr(&mut self, expr: &syn::Expr);

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
}

/// Per-artifact accumulation of observations/evidence/obligations across one `extract()` call.
struct ExtractionContext<'a> {
    input: &'a ExtractionInput,
    extractor: atlas_core::ExtractorIdentity,
    input_fingerprint: String,
    observations: Vec<SemanticObservation>,
    evidence: Vec<Evidence>,
    diagnostics: Vec<ExtractionDiagnostic>,
    /// Per-dimension (observation_ids, evidence_refs) accumulated so far.
    dimension_records: BTreeMap<SemanticDimension, (Vec<SemanticRecordId>, Vec<EvidenceId>)>,
    /// Global dedup guard: a `SemanticRecordId` already embeds its dimension in its hash prefix,
    /// so one set suffices across all four dimensions. The same type/symbol referenced from many
    /// call sites in one file (e.g. `u64` used in ten signatures) is recorded once, not ten times.
    seen_record_ids: BTreeSet<String>,
    /// G119: the standard macro names this file shadows with its own `macro_rules!`.
    shadowed_macros: BTreeSet<String>,
    /// G119: macro invocations and body-rewriting attributes the walkers cannot see through
    /// (`macros::opaque_sites`); `None` until the file parsed.
    opaque_macro_sites: Option<usize>,
    /// G119: depth of recovered macro arguments being walked. DATA_FLOW does not walk macro
    /// arguments, so CALL never builds a `Resolved` DATA_FLOW `PlaceRef` inside them.
    macro_argument_depth: usize,
}

impl<'a> ExtractionContext<'a> {
    fn new(input: &'a ExtractionInput, extractor: atlas_core::ExtractorIdentity) -> Self {
        let input_fingerprint = input.identity_key(&extractor);
        Self {
            input,
            extractor,
            input_fingerprint,
            observations: Vec::new(),
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            dimension_records: BTreeMap::new(),
            seen_record_ids: BTreeSet::new(),
            shadowed_macros: BTreeSet::new(),
            opaque_macro_sites: None,
            macro_argument_depth: 0,
        }
    }

    fn wants(&self, dimension: SemanticDimension) -> bool {
        self.input.requested_dimensions.contains(&dimension)
    }

    fn span_of<T: Spanned>(&self, node: &T) -> atlas_core::SourceSpan {
        self.span_at(node.span())
    }

    fn span_at(&self, span: proc_macro2::Span) -> atlas_core::SourceSpan {
        let start = span.start();
        atlas_core::SourceSpan {
            path: self.input.artifact_path.clone(),
            line: start.line,
            column: start.column,
        }
    }

    fn push_evidence(&mut self, id: &EvidenceId, summary: String) {
        self.evidence.push(Evidence {
            id: id.as_str().to_owned(),
            kind: "PARSER_OUTPUT".into(),
            path: self.input.artifact_path.clone(),
            summary,
            revision: Some(self.input.revision.clone()),
        });
    }

    fn provenance_for(&self, span: Option<&atlas_core::SourceSpan>) -> Provenance {
        Provenance {
            source_path: self.input.artifact_path.clone(),
            source_revision: Some(self.input.revision.clone()),
            extractor: self.extractor.id.clone(),
            content_hash: None,
            span: span.map(|span| format!("{}:{}", span.line, span.column)),
        }
    }

    /// Registers one dimension hit if `record_id` has not already been recorded in this batch.
    /// Returns `true` iff this is the first time this exact record has been seen (i.e. the caller
    /// should push the corresponding `Evidence`/`SemanticObservation`).
    fn record_dimension_hit(
        &mut self,
        dimension: SemanticDimension,
        record_id: SemanticRecordId,
        evidence_id: EvidenceId,
    ) -> bool {
        if !self.seen_record_ids.insert(record_id.as_str().to_owned()) {
            return false;
        }
        let entry = self.dimension_records.entry(dimension).or_default();
        entry.0.push(record_id);
        entry.1.push(evidence_id);
        true
    }

    fn emit_symbol(
        &mut self,
        scope: &SemanticScope,
        name: &str,
        role: SymbolRole,
        span: atlas_core::SourceSpan,
    ) {
        let dimension = SemanticDimension::Symbol;
        if !self.wants(dimension) {
            return;
        }
        let subject = SymbolIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            scope: scope.clone(),
            name: name.to_owned(),
            role,
            path: self.input.artifact_path.clone(),
        };
        let record_id = SemanticRecordId::new(dimension, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!("{}:{}:symbol", self.input_fingerprint, record_id.as_str()),
        ));
        if !self.record_dimension_hit(dimension, record_id.clone(), evidence_id.clone()) {
            return;
        }
        self.push_evidence(
            &evidence_id,
            format!(
                "parsed {} `{name}` at {}:{}:{}",
                role.as_str(),
                span.path,
                span.line,
                span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id,
            dimension,
            status: EpistemicStatus::Observed,
            subject,
            scope: scope.clone(),
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            extractor: self.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::Symbol(header);
        assert!(observation.is_dimension_consistent());
        self.observations.push(observation);
    }

    /// `span` is the source location of the syntax that produced `name`'s spelling (e.g. the
    /// `syn::Type` node, a receiver, or a field), when the caller has one available. Dedup means
    /// only the *first* occurrence of an identical (scope, name) type in this file contributes its
    /// span as evidence -- later occurrences of e.g. `u64` reuse the same record rather than each
    /// attaching their own span, which would require tracking multiple spans per identity (a
    /// larger data-model change out of scope here).
    fn emit_type_identity(
        &mut self,
        scope: &SemanticScope,
        name: &str,
        span: Option<atlas_core::SourceSpan>,
    ) -> TypeIdentity {
        let subject = TypeIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            scope: scope.clone(),
            name: name.to_owned(),
            canonical: None,
            path: self.input.artifact_path.clone(),
        };
        let dimension = SemanticDimension::Type;
        if self.wants(dimension) {
            let record_id = SemanticRecordId::new(dimension, &subject.identity_key());
            let evidence_id = EvidenceId::new(stable_id(
                "evidence",
                &format!("{}:{}:type", self.input_fingerprint, record_id.as_str()),
            ));
            if self.record_dimension_hit(dimension, record_id.clone(), evidence_id.clone()) {
                let location = span
                    .as_ref()
                    .map(|span| format!(" at {}:{}:{}", span.path, span.line, span.column))
                    .unwrap_or_default();
                self.push_evidence(
                    &evidence_id,
                    format!("parsed type spelling `{name}`{location}"),
                );
                let header = SemanticRecordHeader {
                    record_id,
                    dimension,
                    status: EpistemicStatus::Observed,
                    subject: subject.clone(),
                    scope: scope.clone(),
                    repository: self.input.repository.clone(),
                    revision: self.input.revision.clone(),
                    extractor: self.extractor.clone(),
                    evidence_refs: vec![evidence_id],
                    provenance: self.provenance_for(span.as_ref()),
                };
                let observation = SemanticObservation::Type(header);
                assert!(observation.is_dimension_consistent());
                self.observations.push(observation);
            }
        }
        subject
    }

    #[allow(clippy::too_many_arguments)]
    fn function_identity(
        &self,
        scope: &SemanticScope,
        name: &str,
        span: atlas_core::SourceSpan,
        role: SymbolRole,
        declaration_kind: FunctionDeclarationKind,
        owner: FunctionOwner,
        generics: Vec<String>,
    ) -> FunctionIdentity {
        FunctionIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            language: "rust".into(),
            scope: scope.clone(),
            symbol: SymbolIdentity {
                repository: self.input.repository.clone(),
                revision: self.input.revision.clone(),
                scope: scope.clone(),
                name: name.to_owned(),
                role,
                path: self.input.artifact_path.clone(),
            },
            span,
            generated: false,
            declaration_kind,
            owner,
            generics,
        }
    }

    fn emit_function_identity(&mut self, identity: FunctionIdentity) {
        let dimension = SemanticDimension::FunctionIdentity;
        if !self.wants(dimension) {
            return;
        }
        let record_id = SemanticRecordId::new(dimension, &identity.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:function-identity",
                self.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.record_dimension_hit(dimension, record_id.clone(), evidence_id.clone()) {
            return;
        }
        self.push_evidence(
            &evidence_id,
            format!("parsed function identity `{}`", identity.symbol.name),
        );
        let span = identity.span.clone();
        let header = SemanticRecordHeader {
            record_id,
            dimension,
            status: EpistemicStatus::Observed,
            scope: identity.scope.clone(),
            repository: identity.repository.clone(),
            revision: identity.revision.clone(),
            extractor: self.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.provenance_for(Some(&span)),
            subject: identity,
        };
        let observation = SemanticObservation::FunctionIdentity(Box::new(header));
        assert!(observation.is_dimension_consistent());
        self.observations.push(observation);
    }

    fn emit_function_signature(&mut self, signature: FunctionSignature) {
        let dimension = SemanticDimension::FunctionSignature;
        if !self.wants(dimension) {
            return;
        }
        let identity_key = function_signature_identity_key(&signature);
        let record_id = SemanticRecordId::new(dimension, &identity_key);
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!(
                "{}:{}:function-signature",
                self.input_fingerprint,
                record_id.as_str()
            ),
        ));
        if !self.record_dimension_hit(dimension, record_id.clone(), evidence_id.clone()) {
            return;
        }
        self.push_evidence(
            &evidence_id,
            format!(
                "parsed function signature for `{}`",
                signature.function.symbol.name
            ),
        );
        let scope = signature.function.scope.clone();
        let repository = signature.function.repository.clone();
        let revision = signature.function.revision.clone();
        let span = signature.function.span.clone();
        let header = SemanticRecordHeader {
            record_id,
            dimension,
            status: EpistemicStatus::Observed,
            scope,
            repository,
            revision: revision.clone(),
            extractor: self.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.provenance_for(Some(&span)),
            subject: signature,
        };
        let observation = SemanticObservation::FunctionSignature(Box::new(header));
        assert!(observation.is_dimension_consistent());
        self.observations.push(observation);
    }

    /// A `PlaceRef` at the SAME DATA_FLOW record_id `dataflow.rs`'s own walker would independently
    /// compute for a value of role `role` named `name` at `span` inside `caller` -- reusing
    /// `ValueIdentity::identity_key()` itself (never a hand-duplicated copy of its format string)
    /// so the two walkers cannot silently drift apart. Gated on
    /// `self.wants(SemanticDimension::DataFlow)`: a `Resolved` reference is only ever constructed
    /// when DATA_FLOW is actually part of this same extraction request, so it never names a record
    /// that (per this exact request) will not exist in this batch. `resolution`/
    /// `resolved_definition`/`is_return_flow`/`is_parameter` are placeholder values below because
    /// none of them affect `identity_key()`; only `function`, `name`, `span` and `role` do.
    fn place_ref_for_value(
        &self,
        name: &str,
        span: atlas_core::SourceSpan,
        caller: &SemanticRecordId,
        role: ValueRole,
    ) -> PlaceRef {
        if !self.wants(SemanticDimension::DataFlow) || self.macro_argument_depth > 0 {
            return PlaceRef::Unresolved;
        }
        let subject = ValueIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            function: caller.clone(),
            name: name.to_owned(),
            span,
            role,
            is_parameter: false,
            is_return_flow: false,
            resolution: DataFlowResolution::Unresolved,
            resolved_definition: None,
        };
        PlaceRef::Resolved {
            dimension: SemanticDimension::DataFlow,
            record_id: SemanticRecordId::new(SemanticDimension::DataFlow, &subject.identity_key()),
        }
    }

    /// The `PlaceRef` a CALL argument expression should carry: `Resolved` when `arg`, after
    /// stripping any `(..)`/`&`/`&mut` wrapping, is a simple single-identifier expression (the
    /// same shape `dataflow.rs`'s own recursive walk ultimately reaches its `Expr::Path` arm
    /// through, via `spelling::unwrap_value_read` + `spelling::simple_path_ident`), else
    /// `Unresolved`. An argument position is always a value READ, so `&x` is safe to unwrap here
    /// -- unlike `place_ref_for_assign_result`'s place position, where it would not be.
    fn place_ref_for_argument(&self, arg: &syn::Expr, caller: &SemanticRecordId) -> PlaceRef {
        let inner = spelling::unwrap_value_read(arg);
        let Some(ident) = spelling::simple_path_ident(inner) else {
            return PlaceRef::Unresolved;
        };
        self.place_ref_for_value(
            &ident.to_string(),
            self.span_of(inner),
            caller,
            ValueRole::Use,
        )
    }

    /// Emits and walks a `Call`/`MethodCall` expression (`call_like` MUST be one of those two
    /// variants), carrying `result` as this call's own `CallSiteIdentity.result`. Shared by
    /// `walk_expr` (which always passes `PlaceRef::Unresolved` -- an arbitrary call expression
    /// buried in a larger one has no single well-defined "result" binding) and
    /// `walk_value_position_expr` (which computes a real `result` for the narrow case where this
    /// call IS the entire value a `let`/assignment target receives).
    fn walk_call_like(
        &mut self,
        call_like: &syn::Expr,
        scope: &SemanticScope,
        caller: &SemanticRecordId,
        result: PlaceRef,
    ) {
        match call_like {
            syn::Expr::Call(call) => {
                let span = self.span_at(call_anchor(call_like));
                let summary = spelling::call_callee_spelling(&call.func);
                let arguments = call
                    .args
                    .iter()
                    .map(|arg| self.place_ref_for_argument(arg, caller))
                    .collect();
                self.emit_call(scope, caller.clone(), span, &summary, arguments, result);
                self.walk_expr(&call.func, scope, caller);
                for arg in &call.args {
                    self.walk_expr(arg, scope, caller);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                let span = self.span_at(call_anchor(call_like));
                let summary = format!(".{}", method_call.method);
                let arguments = method_call
                    .args
                    .iter()
                    .map(|arg| self.place_ref_for_argument(arg, caller))
                    .collect();
                self.emit_call(scope, caller.clone(), span, &summary, arguments, result);
                self.walk_expr(&method_call.receiver, scope, caller);
                for arg in &method_call.args {
                    self.walk_expr(arg, scope, caller);
                }
            }
            other => unreachable!(
                "walk_call_like called with a non-call expression: {:?}",
                std::mem::discriminant(other)
            ),
        }
    }

    /// Walks `expr` in a position where its resulting value directly becomes `result` (a `let`
    /// binding's Definition, or a plain assignment's Store) IF `expr`, after stripping any `(..)`
    /// wrapping, is exactly a `Call`/`MethodCall` expression -- e.g. the direct initializer of
    /// `let y = helper(x);` OR `let y = (helper(x));` (parentheses are transparent here, matching
    /// `dataflow.rs`'s own `Stmt::Local` arm, which creates `y`'s Definition unconditionally from
    /// the pattern regardless of how the initializer is wrapped) -- never a nested subexpression
    /// like `helper(x) + 1` (which has no single value this call's result "becomes"). Delegates to
    /// the ordinary `walk_expr` (implying `PlaceRef::Unresolved`) for every other case.
    fn walk_value_position_expr(
        &mut self,
        expr: &syn::Expr,
        scope: &SemanticScope,
        caller: &SemanticRecordId,
        result: PlaceRef,
    ) {
        match spelling::unwrap_parens(expr) {
            call_like @ (syn::Expr::Call(_) | syn::Expr::MethodCall(_)) => {
                self.walk_call_like(call_like, scope, caller, result);
            }
            _ => self.walk_expr(expr, scope, caller),
        }
    }

    /// The `PlaceRef` a `let <simple ident> = <init>;` binding's result-position should carry, if
    /// `pat` reduces to a single simple identifier (`spelling::simple_binding_ident` -- the SAME
    /// recognizer that decided `dataflow.rs`'s own `Definition` used to bind for this exact
    /// pattern, before R4.12's destructuring fix generalized `walk_binding_pat`; a destructuring
    /// pattern has no single identifier a call's result could unambiguously "become", so it stays
    /// `Unresolved` here even though DATA_FLOW itself now binds every sub-identifier).
    fn place_ref_for_let_result(&self, pat: &syn::Pat, caller: &SemanticRecordId) -> PlaceRef {
        let Some(ident) = spelling::simple_binding_ident(pat) else {
            return PlaceRef::Unresolved;
        };
        self.place_ref_for_value(
            &ident.to_string(),
            self.span_of(ident),
            caller,
            ValueRole::Definition,
        )
    }

    /// The `PlaceRef` a plain `<simple path> = <init>;` assignment's result-position should carry.
    fn place_ref_for_assign_result(&self, lhs: &syn::Expr, caller: &SemanticRecordId) -> PlaceRef {
        let Some(ident) = spelling::simple_path_ident(lhs) else {
            return PlaceRef::Unresolved;
        };
        self.place_ref_for_value(
            &ident.to_string(),
            self.span_of(lhs),
            caller,
            ValueRole::Store,
        )
    }

    /// Emits one CALL observation for a call site syntactically inside `caller`'s body.
    ///
    /// `dispatch`/`callees` are always `Unresolved`/`[]`: see the module doc comment and
    /// `core/src/semantic/call.rs` for why this extractor never claims a resolved callee.
    fn emit_call(
        &mut self,
        scope: &SemanticScope,
        caller: SemanticRecordId,
        span: atlas_core::SourceSpan,
        callee_summary: &str,
        arguments: Vec<PlaceRef>,
        result: PlaceRef,
    ) {
        let dimension = SemanticDimension::Call;
        if !self.wants(dimension) {
            return;
        }
        let subject = CallSiteIdentity {
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            function: caller,
            span: span.clone(),
            dispatch: CallDispatchKind::Unresolved,
            callees: Vec::new(),
            arguments,
            result,
        };
        let record_id = SemanticRecordId::new(dimension, &subject.identity_key());
        let evidence_id = EvidenceId::new(stable_id(
            "evidence",
            &format!("{}:{}:call", self.input_fingerprint, record_id.as_str()),
        ));
        if !self.record_dimension_hit(dimension, record_id.clone(), evidence_id.clone()) {
            return;
        }
        self.push_evidence(
            &evidence_id,
            format!(
                "parsed call to `{callee_summary}` at {}:{}:{}",
                span.path, span.line, span.column
            ),
        );
        let header = SemanticRecordHeader {
            record_id,
            dimension,
            status: EpistemicStatus::Observed,
            subject,
            scope: scope.clone(),
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            extractor: self.extractor.clone(),
            evidence_refs: vec![evidence_id],
            provenance: self.provenance_for(Some(&span)),
        };
        let observation = SemanticObservation::Call(header);
        assert!(observation.is_dimension_consistent());
        self.observations.push(observation);
    }

    /// Recursively walks a function/method body looking for `syn::Expr::Call`/`MethodCall` nodes,
    /// attributing every one found to `caller` (the enclosing function's `FunctionIdentity`
    /// record_id). A nested item (`fn`/`struct`/... declared inside a block) is dispatched back
    /// through `walk_item`, not treated as part of the enclosing function's call set.
    fn walk_block(&mut self, block: &syn::Block, scope: &SemanticScope, caller: &SemanticRecordId) {
        for stmt in &block.stmts {
            self.walk_stmt(stmt, scope, caller);
        }
    }

    fn walk_stmt(&mut self, stmt: &syn::Stmt, scope: &SemanticScope, caller: &SemanticRecordId) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    let result = self.place_ref_for_let_result(&local.pat, caller);
                    self.walk_value_position_expr(&init.expr, scope, caller, result);
                    if let Some((_, diverge)) = &init.diverge {
                        self.walk_expr(diverge, scope, caller);
                    }
                }
            }
            syn::Stmt::Expr(expr, _) => self.walk_expr(expr, scope, caller),
            syn::Stmt::Item(item) => self.walk_item(item, scope),
            syn::Stmt::Macro(stmt_macro) => {
                self.walk_macro_arguments(&stmt_macro.mac, scope, caller)
            }
        }
    }

    /// G119: the evaluated arguments of a recovered standard macro are walked like any other
    /// expression of the caller (`macros::recovered_arguments`); any other macro stays opaque.
    fn walk_macro_arguments(
        &mut self,
        mac: &syn::Macro,
        scope: &SemanticScope,
        caller: &SemanticRecordId,
    ) {
        if let Some(arguments) = macros::recovered_arguments(mac, &self.shadowed_macros) {
            self.macro_argument_depth += 1;
            for argument in &arguments {
                self.walk_expr(argument, scope, caller);
            }
            self.macro_argument_depth -= 1;
        }
    }

    fn walk_expr(&mut self, expr: &syn::Expr, scope: &SemanticScope, caller: &SemanticRecordId) {
        match expr {
            syn::Expr::Call(_) | syn::Expr::MethodCall(_) => {
                self.walk_call_like(expr, scope, caller, PlaceRef::Unresolved);
            }
            syn::Expr::Binary(binary) => {
                self.walk_expr(&binary.left, scope, caller);
                self.walk_expr(&binary.right, scope, caller);
            }
            syn::Expr::Unary(unary) => self.walk_expr(&unary.expr, scope, caller),
            syn::Expr::If(if_expr) => {
                self.walk_expr(&if_expr.cond, scope, caller);
                self.walk_block(&if_expr.then_branch, scope, caller);
                if let Some((_, else_branch)) = &if_expr.else_branch {
                    self.walk_expr(else_branch, scope, caller);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.walk_expr(&match_expr.expr, scope, caller);
                for arm in &match_expr.arms {
                    // A guard (`Some(x) if x > 0 => ...`) is parsed into `arm.pat` as
                    // `Pat::Guard` in this `syn` version, not a separate `arm.guard` field.
                    if let syn::Pat::Guard(guard) = &arm.pat {
                        self.walk_expr(&guard.guard, scope, caller);
                    }
                    self.walk_expr(&arm.body, scope, caller);
                }
            }
            syn::Expr::Block(block_expr) => self.walk_block(&block_expr.block, scope, caller),
            syn::Expr::Loop(loop_expr) => self.walk_block(&loop_expr.body, scope, caller),
            syn::Expr::While(while_expr) => {
                self.walk_expr(&while_expr.cond, scope, caller);
                self.walk_block(&while_expr.body, scope, caller);
            }
            syn::Expr::ForLoop(for_loop) => {
                self.walk_expr(&for_loop.expr, scope, caller);
                self.walk_block(&for_loop.body, scope, caller);
            }
            syn::Expr::Paren(paren) => self.walk_expr(&paren.expr, scope, caller),
            syn::Expr::Group(group) => self.walk_expr(&group.expr, scope, caller),
            syn::Expr::Reference(reference) => self.walk_expr(&reference.expr, scope, caller),
            syn::Expr::Field(field) => self.walk_expr(&field.base, scope, caller),
            syn::Expr::Index(index) => {
                self.walk_expr(&index.expr, scope, caller);
                self.walk_expr(&index.index, scope, caller);
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.walk_expr(value, scope, caller);
                }
            }
            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.walk_expr(value, scope, caller);
                }
            }
            syn::Expr::Assign(assign) => {
                self.walk_expr(&assign.left, scope, caller);
                let result = self.place_ref_for_assign_result(&assign.left, caller);
                self.walk_value_position_expr(&assign.right, scope, caller, result);
            }
            syn::Expr::Try(try_expr) => self.walk_expr(&try_expr.expr, scope, caller),
            syn::Expr::Await(await_expr) => self.walk_expr(&await_expr.base, scope, caller),
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.walk_expr(&field.expr, scope, caller);
                }
                if let Some(rest) = &struct_expr.rest {
                    self.walk_expr(rest, scope, caller);
                }
            }
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.walk_expr(elem, scope, caller);
                }
            }
            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.walk_expr(elem, scope, caller);
                }
            }
            // A closure body is a separate executable region that runs later (possibly never, or
            // from a completely different caller) than the enclosing function -- attributing its
            // call sites to `caller` would misattribute them exactly as CFG/DATA_FLOW/STATE/
            // EFFECT/OWNERSHIP/CONCURRENCY already correctly refuse to do (none of them recurse
            // into a closure body either; see each file's own "Closures get no ... attribution"
            // comment). This extractor has no closure/executable-region identity of its own yet
            // (`ExecutableRegionIdentity`-shaped work is future scope), so a closure's calls are
            // left explicitly outside this dimension's current profile rather than misattributed
            // to the outer function. Corrects a prior version of this walker that recursed here.
            syn::Expr::Closure(_) => {}
            syn::Expr::Cast(cast) => self.walk_expr(&cast.expr, scope, caller),
            syn::Expr::Range(range) => {
                if let Some(start) = &range.start {
                    self.walk_expr(start, scope, caller);
                }
                if let Some(end) = &range.end {
                    self.walk_expr(end, scope, caller);
                }
            }
            syn::Expr::Let(let_expr) => self.walk_expr(&let_expr.expr, scope, caller),
            // `unsafe { .. }`/`const { .. }`/`try { .. }` execute immediately as part of the same
            // executable region (unlike Closure/Async, they are not deferred) -- a call inside one
            // is a real call site of the enclosing function, so these recurse rather than falling
            // to the catch-all below.
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.walk_stmt(stmt, scope, caller);
                }
            }
            syn::Expr::Const(const_expr) => {
                for stmt in &const_expr.block.stmts {
                    self.walk_stmt(stmt, scope, caller);
                }
            }
            syn::Expr::TryBlock(try_block) => {
                for stmt in &try_block.block.stmts {
                    self.walk_stmt(stmt, scope, caller);
                }
            }
            syn::Expr::Repeat(repeat) => {
                self.walk_expr(&repeat.expr, scope, caller);
                self.walk_expr(&repeat.len, scope, caller);
            }
            syn::Expr::RawAddr(raw_addr) => self.walk_expr(&raw_addr.expr, scope, caller),
            // `async { .. }`/`async move { .. }` is a separate deferred executable region (a
            // Future body polled later, possibly never, exactly like a Closure) -- excluded for
            // the same misattribution reason as `Expr::Closure` above, not merely unhandled.
            syn::Expr::Async(_) => {}
            // `yield value` (unstable generator/coroutine syntax) evaluates `value` immediately as
            // part of the SAME executable region -- syn parses it wherever it lexically appears,
            // not only inside a generator body, so a bare fn/method containing `yield f()` is real,
            // parseable input this walker must not silently skip. Not a deferred region like
            // Closure/Async: matches CALL's own Return/Break precedent, and EFFECT/STATE's already
            // -correct treatment of the same variant.
            syn::Expr::Yield(yield_expr) => {
                if let Some(value) = &yield_expr.expr {
                    self.walk_expr(value, scope, caller);
                }
            }
            // G119: a standard macro whose documented input is a list of expressions is re-parsed
            // and its arguments walked (`macros::recovered_arguments`); any other macro invocation
            // stays opaque token-stream input this extractor never expands
            // (`.atlas/contracts/SEMANTIC-EXTRACTION.md`), counted by `macros::opaque_sites` so the
            // file cannot claim CALL coverage.
            syn::Expr::Macro(expr_macro) => {
                self.walk_macro_arguments(&expr_macro.mac, scope, caller);
            }
            // Literals, bare paths, `continue`, and any other/future `syn::Expr` shape structurally
            // cannot contain a nested call.
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_function(
        &mut self,
        name: &str,
        visibility: String,
        sig: &syn::Signature,
        body: Option<&syn::Block>,
        scope: &SemanticScope,
        span: atlas_core::SourceSpan,
        role: SymbolRole,
        declaration_kind: FunctionDeclarationKind,
        owner: FunctionOwner,
    ) {
        self.emit_symbol(scope, name, role, span.clone());

        let mut generics: Vec<String> = sig
            .generics
            .params
            .iter()
            .map(spelling::generic_param_spelling)
            .collect();
        if let Some(where_clause) = &sig.generics.where_clause {
            generics.extend(spelling::where_predicate_spelling(where_clause));
        }

        let identity = self.function_identity(
            scope,
            name,
            span,
            role,
            declaration_kind,
            owner,
            generics.clone(),
        );
        // Computed independently of whether FUNCTION_IDENTITY itself is requested: a caller
        // attribution for CALL must not silently disappear just because the FunctionIdentity
        // observation was suppressed by the caller's requested-dimension set.
        let caller_record_id = SemanticRecordId::new(
            SemanticDimension::FunctionIdentity,
            &identity.identity_key(),
        );
        self.emit_function_identity(identity.clone());

        if let Some(body) = body {
            self.walk_block(body, scope, &caller_record_id);
            self.build_control_flow(body, scope, &caller_record_id);
            self.build_data_flow(sig, body, scope, &caller_record_id);
            self.build_state(body, scope, &caller_record_id);
            self.build_effects(body, scope, &caller_record_id);
            self.build_ownership(body, scope, &caller_record_id);
            self.build_concurrency(body, scope, &caller_record_id);
            self.build_persistence(body, scope, &caller_record_id);
        }

        let mut parameters = Vec::new();
        for argument in &sig.inputs {
            match argument {
                syn::FnArg::Receiver(receiver) => {
                    let type_name = spelling::receiver_type_spelling(receiver);
                    let span = self.span_of(receiver);
                    let type_identity = self.emit_type_identity(scope, &type_name, Some(span));
                    let label = spelling::receiver_label(receiver);
                    parameters.push(FunctionParameter {
                        name: label,
                        type_identity,
                    });
                }
                syn::FnArg::Typed(pat_type) => {
                    let param_name = spelling::pattern_spelling(&pat_type.pat);
                    let type_name = spelling::type_spelling(&pat_type.ty);
                    let span = self.span_of(&pat_type.ty);
                    let type_identity = self.emit_type_identity(scope, &type_name, Some(span));
                    parameters.push(FunctionParameter {
                        name: param_name,
                        type_identity,
                    });
                }
            }
        }

        let return_type = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => {
                let type_name = spelling::type_spelling(ty);
                let span = self.span_of(ty.as_ref());
                Some(self.emit_type_identity(scope, &type_name, Some(span)))
            }
        };

        let abi = sig.abi.as_ref().map(spelling::abi_spelling);
        let is_extern = sig.abi.is_some();

        let signature = FunctionSignature {
            function: identity,
            parameters,
            return_type,
            generics,
            abi,
            visibility,
            is_async: sig.asyncness.is_some(),
            is_unsafe: matches!(sig.safety, syn::Safety::Unsafe(_)),
            is_extern,
            body_fingerprint: body.map(body_fingerprint),
        };
        self.emit_function_signature(signature);
    }

    fn walk_item(&mut self, item: &syn::Item, scope: &SemanticScope) {
        match item {
            syn::Item::Fn(item_fn) => {
                let span = self.span_of(item_fn);
                let name = item_fn.sig.ident.to_string();
                let visibility = spelling::visibility_spelling(&item_fn.vis);
                self.handle_function(
                    &name,
                    visibility,
                    &item_fn.sig,
                    Some(&item_fn.block),
                    scope,
                    span,
                    SymbolRole::Definition,
                    FunctionDeclarationKind::FreeFunction,
                    FunctionOwner::none(),
                );
            }
            syn::Item::Struct(item_struct) => self.handle_struct(item_struct, scope),
            syn::Item::Enum(item_enum) => self.handle_enum(item_enum, scope),
            syn::Item::Trait(item_trait) => self.handle_trait(item_trait, scope),
            syn::Item::Impl(item_impl) => self.handle_impl(item_impl, scope),
            syn::Item::Type(item_type) => {
                let span = self.span_of(item_type);
                self.emit_symbol(
                    scope,
                    &item_type.ident.to_string(),
                    SymbolRole::Definition,
                    span,
                );
                let type_name = spelling::type_spelling(&item_type.ty);
                let type_span = self.span_of(item_type.ty.as_ref());
                self.emit_type_identity(scope, &type_name, Some(type_span));
            }
            syn::Item::Const(item_const) => {
                let span = self.span_of(item_const);
                self.emit_symbol(
                    scope,
                    &item_const.ident.to_string(),
                    SymbolRole::Definition,
                    span,
                );
                let type_name = spelling::type_spelling(&item_const.ty);
                let type_span = self.span_of(item_const.ty.as_ref());
                self.emit_type_identity(scope, &type_name, Some(type_span));
            }
            syn::Item::Static(item_static) => {
                let span = self.span_of(item_static);
                self.emit_symbol(
                    scope,
                    &item_static.ident.to_string(),
                    SymbolRole::Definition,
                    span,
                );
                let type_name = spelling::type_spelling(&item_static.ty);
                let type_span = self.span_of(item_static.ty.as_ref());
                self.emit_type_identity(scope, &type_name, Some(type_span));
            }
            syn::Item::Mod(item_mod) => self.handle_mod(item_mod, scope),
            // Everything else (`use`, `extern crate`, macro invocations at item position, foreign
            // modules, trait aliases, ...) is out of scope for R4.3's minimum symbol/type/function
            // set; it is neither claimed nor fabricated.
            _ => {}
        }
    }

    fn handle_struct(&mut self, item: &syn::ItemStruct, scope: &SemanticScope) {
        let span = self.span_of(item);
        let name = item.ident.to_string();
        self.emit_symbol(scope, &name, SymbolRole::Definition, span);
        let nested = nested_scope(scope, &name);
        for (index, field) in item.fields.iter().enumerate() {
            let field_name = field
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| index.to_string());
            let field_span = self.span_of(field);
            self.emit_symbol(&nested, &field_name, SymbolRole::Definition, field_span);
            let type_name = spelling::type_spelling(&field.ty);
            let type_span = self.span_of(&field.ty);
            self.emit_type_identity(&nested, &type_name, Some(type_span));
        }
    }

    fn handle_enum(&mut self, item: &syn::ItemEnum, scope: &SemanticScope) {
        let span = self.span_of(item);
        let name = item.ident.to_string();
        self.emit_symbol(scope, &name, SymbolRole::Definition, span);
        let nested = nested_scope(scope, &name);
        for variant in &item.variants {
            let variant_span = self.span_of(variant);
            let variant_name = variant.ident.to_string();
            self.emit_symbol(&nested, &variant_name, SymbolRole::Definition, variant_span);
            // Fields are scoped under the variant, not the enum itself: unlike a struct (one
            // field namespace per declaration), two variants of the same enum can each declare a
            // field with the same name (e.g. `Active { id: u64 }` / `Inactive { id: u64 }`), and
            // enum-level scoping would silently collide them onto one Symbol identity.
            let variant_scope = nested_scope(&nested, &variant_name);
            for (index, field) in variant.fields.iter().enumerate() {
                let field_name = field
                    .ident
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| index.to_string());
                let field_span = self.span_of(field);
                self.emit_symbol(
                    &variant_scope,
                    &field_name,
                    SymbolRole::Definition,
                    field_span,
                );
                let type_name = spelling::type_spelling(&field.ty);
                let type_span = self.span_of(&field.ty);
                self.emit_type_identity(&nested, &type_name, Some(type_span));
            }
        }
    }

    fn handle_trait(&mut self, item: &syn::ItemTrait, scope: &SemanticScope) {
        let span = self.span_of(item);
        let name = item.ident.to_string();
        self.emit_symbol(scope, &name, SymbolRole::Definition, span);
        let nested = nested_scope(scope, &format!("trait:{name}"));
        for trait_item in &item.items {
            match trait_item {
                syn::TraitItem::Fn(method) => {
                    let (role, declaration_kind) = if method.default.is_some() {
                        (
                            SymbolRole::Definition,
                            FunctionDeclarationKind::TraitDefaultMethod,
                        )
                    } else {
                        (
                            SymbolRole::Declaration,
                            FunctionDeclarationKind::TraitMethodDeclaration,
                        )
                    };
                    let method_name = method.sig.ident.to_string();
                    let method_span = self.span_of(method);
                    let owner = FunctionOwner {
                        target: None,
                        trait_path: Some(name.clone()),
                    };
                    self.handle_function(
                        &method_name,
                        "inherited".to_owned(),
                        &method.sig,
                        method.default.as_ref(),
                        &nested,
                        method_span,
                        role,
                        declaration_kind,
                        owner,
                    );
                }
                syn::TraitItem::Const(assoc_const) => {
                    // A trait associated const always names a concrete declared type (`const
                    // NAME: Type`), even without a default value, unlike an associated type.
                    let role = if assoc_const.default.is_some() {
                        SymbolRole::Definition
                    } else {
                        SymbolRole::Declaration
                    };
                    let const_span = self.span_of(assoc_const);
                    self.emit_symbol(&nested, &assoc_const.ident.to_string(), role, const_span);
                    let type_name = spelling::type_spelling(&assoc_const.ty);
                    let type_span = self.span_of(&assoc_const.ty);
                    self.emit_type_identity(&nested, &type_name, Some(type_span));
                }
                syn::TraitItem::Type(assoc_type) => {
                    let type_span = self.span_of(assoc_type);
                    match &assoc_type.default {
                        Some((_, default_ty)) => {
                            self.emit_symbol(
                                &nested,
                                &assoc_type.ident.to_string(),
                                SymbolRole::Definition,
                                type_span,
                            );
                            let type_name = spelling::type_spelling(default_ty);
                            let default_span = self.span_of(default_ty);
                            self.emit_type_identity(&nested, &type_name, Some(default_span));
                        }
                        // No default: only bounds are declared, never a concrete `Type` node --
                        // emitting a TypeIdentity here would fabricate a type that isn't spelled
                        // out anywhere in this declaration.
                        None => {
                            self.emit_symbol(
                                &nested,
                                &assoc_type.ident.to_string(),
                                SymbolRole::Declaration,
                                type_span,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_impl(&mut self, item: &syn::ItemImpl, scope: &SemanticScope) {
        let self_type = spelling::type_spelling(&item.self_ty);
        let trait_path = item
            .trait_
            .as_ref()
            .map(|(path, _)| spelling::path_spelling(path));
        let segment = match &trait_path {
            Some(trait_path) => format!("impl:{trait_path} for {self_type}"),
            None => format!("impl:{self_type}"),
        };
        let nested = nested_scope(scope, &segment);

        // Owner target: the impl's self type, observed at the OUTER scope (it is referenced here,
        // not defined here) -- same TypeIdentity a parameter/return type of this shape would get,
        // so it participates in TYPE coverage like any other observed type spelling.
        let target_span = self.span_of(item.self_ty.as_ref());
        let target = self.emit_type_identity(scope, &self_type, Some(target_span));

        for impl_item in &item.items {
            match impl_item {
                syn::ImplItem::Fn(method) => {
                    let method_name = method.sig.ident.to_string();
                    let visibility = spelling::visibility_spelling(&method.vis);
                    let method_span = self.span_of(method);
                    let has_receiver =
                        matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
                    let declaration_kind = match (&trait_path, has_receiver) {
                        (Some(_), _) => FunctionDeclarationKind::TraitImplementationMethod,
                        (None, true) => FunctionDeclarationKind::InherentMethod,
                        (None, false) => FunctionDeclarationKind::AssociatedFunction,
                    };
                    let owner = FunctionOwner {
                        target: Some(target.clone()),
                        trait_path: trait_path.clone(),
                    };
                    self.handle_function(
                        &method_name,
                        visibility,
                        &method.sig,
                        Some(&method.block),
                        &nested,
                        method_span,
                        SymbolRole::Definition,
                        declaration_kind,
                        owner,
                    );
                }
                syn::ImplItem::Const(assoc_const) => {
                    // An impl always supplies a concrete value, so this is always a Definition,
                    // unlike a trait's own (possibly value-less) associated const declaration.
                    let const_span = self.span_of(assoc_const);
                    self.emit_symbol(
                        &nested,
                        &assoc_const.ident.to_string(),
                        SymbolRole::Definition,
                        const_span,
                    );
                    let type_name = spelling::type_spelling(&assoc_const.ty);
                    let type_span = self.span_of(&assoc_const.ty);
                    self.emit_type_identity(&nested, &type_name, Some(type_span));
                }
                syn::ImplItem::Type(assoc_type) => {
                    // An impl's associated type always assigns a concrete `Type` (`type Foo =
                    // Bar;`), unlike a trait's own (possibly default-less) associated type.
                    let type_span = self.span_of(assoc_type);
                    self.emit_symbol(
                        &nested,
                        &assoc_type.ident.to_string(),
                        SymbolRole::Definition,
                        type_span,
                    );
                    let type_name = spelling::type_spelling(&assoc_type.ty);
                    let concrete_span = self.span_of(&assoc_type.ty);
                    self.emit_type_identity(&nested, &type_name, Some(concrete_span));
                }
                _ => {}
            }
        }
    }

    fn handle_mod(&mut self, item: &syn::ItemMod, scope: &SemanticScope) {
        let span = self.span_of(item);
        let name = item.ident.to_string();
        match &item.content {
            Some((_, items)) => {
                self.emit_symbol(scope, &name, SymbolRole::Definition, span);
                let nested = nested_scope(scope, &name);
                for nested_item in items {
                    self.walk_item(nested_item, &nested);
                }
            }
            None => {
                // `mod foo;` -- declared here, defined in another file this extractor does not
                // (yet) follow. Declaration, not Definition: the body was never observed.
                self.emit_symbol(scope, &name, SymbolRole::Declaration, span);
            }
        }
    }

    fn unsupported_obligation(&mut self, dimension: SemanticDimension) -> ObligationResult {
        let diagnostic = ExtractionDiagnostic::new(
            DiagnosticCode::UnsupportedSemanticDimension,
            Some(dimension),
            format!(
                "RustSemanticExtractor ({}) does not extract {} yet -- deferred to R4.4+",
                RUST_SEMANTIC_EXTRACTOR_ID,
                dimension.as_str()
            ),
        );
        let obligation = ObligationResult::unsupported(dimension, diagnostic.id.clone());
        self.diagnostics.push(diagnostic);
        obligation
    }

    fn finish_success(mut self) -> ExtractionBatch {
        for (_, (ids, refs)) in self.dimension_records.iter_mut() {
            ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            refs.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        }

        let mut obligations = Vec::with_capacity(self.input.requested_dimensions.len());
        for &dimension in &self.input.requested_dimensions {
            if SUPPORTED_DIMENSIONS.contains(&dimension) {
                let records = self.dimension_records.remove(&dimension);
                // G119: CALL's only in-profile gap is opaque macro input; a file whose every macro
                // invocation and attribute is recovered or built-in is exhaustively walked.
                let call_exhaustive_here =
                    dimension == SemanticDimension::Call && self.opaque_macro_sites == Some(0);
                if dimension_coverage(dimension) == DimensionCoverage::Partial
                    && !call_exhaustive_here
                {
                    let diagnostic = ExtractionDiagnostic::new(
                        DiagnosticCode::IncompleteAnalysis,
                        Some(dimension),
                        format!(
                            "{} currently has partial {} coverage for {}; emitted observations are valid, but absence of unmodeled forms is not proven",
                            RUST_SEMANTIC_EXTRACTOR_ID,
                            dimension.as_str(),
                            self.input.artifact_path
                        ),
                    );
                    let diagnostic_id = diagnostic.id.clone();
                    self.diagnostics.push(diagnostic);
                    match records {
                        Some((ids, refs)) => {
                            obligations.push(ObligationResult::unknown_with_observations(
                                dimension,
                                ids,
                                refs,
                                diagnostic_id,
                            ))
                        }
                        None => {
                            obligations.push(ObligationResult::unknown(dimension, diagnostic_id))
                        }
                    }
                    continue;
                }

                match records {
                    Some((ids, refs)) => {
                        obligations.push(ObligationResult::observed(dimension, ids, refs))
                    }
                    None => {
                        let evidence_id = EvidenceId::new(stable_id(
                            "evidence",
                            &format!(
                                "{}:{}:verified-absence",
                                self.input_fingerprint,
                                dimension.as_str()
                            ),
                        ));
                        self.push_evidence(
                            &evidence_id,
                            format!(
                                "exhaustive parse of {} found no {} declarations",
                                self.input.artifact_path,
                                dimension.as_str()
                            ),
                        );
                        obligations.push(ObligationResult::observed(
                            dimension,
                            Vec::new(),
                            vec![evidence_id],
                        ));
                    }
                }
            } else {
                obligations.push(self.unsupported_obligation(dimension));
            }
        }
        obligations.sort_by_key(|obligation| obligation.dimension.as_str());
        self.observations
            .sort_by(|a, b| a.record_id().as_str().cmp(b.record_id().as_str()));
        self.evidence.sort_by(|a, b| a.id.cmp(&b.id));

        ExtractionBatch {
            extractor: self.extractor.clone(),
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            artifact: self.input.artifact.clone(),
            input_fingerprint: self.input_fingerprint.clone(),
            observations: self.observations,
            evidence: self.evidence,
            obligations,
            diagnostics: self.diagnostics,
        }
    }

    /// See `max_structural_recursion_risk`'s doc comment. Mirrors `finish_parse_failure`'s shape
    /// exactly (every supported dimension `UNKNOWN`, every unsupported one `UNSUPPORTED`) since
    /// the epistemic meaning is the same: evidence was not obtained, for a documented reason, and
    /// the artifact is never removed from accounting.
    fn finish_resource_limit(mut self, observed_risk: usize) -> ExtractionBatch {
        let diagnostic = ExtractionDiagnostic::new(
            DiagnosticCode::ResourceLimit,
            None,
            format!(
                "{} structural recursion risk {observed_risk} exceeds \
                 MAX_STRUCTURAL_RECURSION_RISK ({MAX_STRUCTURAL_RECURSION_RISK}); refusing to \
                 parse to avoid a stack-overflow denial-of-service in the recursive-descent parser",
                self.input.artifact_path
            ),
        );
        let diagnostic_id = diagnostic.id.clone();
        self.diagnostics.push(diagnostic);

        let mut obligations = Vec::with_capacity(self.input.requested_dimensions.len());
        for &dimension in &self.input.requested_dimensions {
            if SUPPORTED_DIMENSIONS.contains(&dimension) {
                obligations.push(ObligationResult::unknown(dimension, diagnostic_id.clone()));
            } else {
                obligations.push(self.unsupported_obligation(dimension));
            }
        }
        obligations.sort_by_key(|obligation| obligation.dimension.as_str());

        ExtractionBatch {
            extractor: self.extractor.clone(),
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            artifact: self.input.artifact.clone(),
            input_fingerprint: self.input_fingerprint.clone(),
            observations: self.observations,
            evidence: self.evidence,
            obligations,
            diagnostics: self.diagnostics,
        }
    }

    fn finish_parse_failure(mut self, error: &syn::Error) -> ExtractionBatch {
        let parse_diagnostic = ExtractionDiagnostic::new(
            DiagnosticCode::ParseFailure,
            None,
            format!(
                "failed to parse {} as Rust source: {error}",
                self.input.artifact_path
            ),
        );
        let parse_diagnostic_id = parse_diagnostic.id.clone();
        self.diagnostics.push(parse_diagnostic);

        let mut obligations = Vec::with_capacity(self.input.requested_dimensions.len());
        for &dimension in &self.input.requested_dimensions {
            if SUPPORTED_DIMENSIONS.contains(&dimension) {
                obligations.push(ObligationResult::unknown(
                    dimension,
                    parse_diagnostic_id.clone(),
                ));
            } else {
                obligations.push(self.unsupported_obligation(dimension));
            }
        }
        obligations.sort_by_key(|obligation| obligation.dimension.as_str());

        ExtractionBatch {
            extractor: self.extractor.clone(),
            repository: self.input.repository.clone(),
            revision: self.input.revision.clone(),
            artifact: self.input.artifact.clone(),
            input_fingerprint: self.input_fingerprint.clone(),
            observations: self.observations,
            evidence: self.evidence,
            obligations,
            diagnostics: self.diagnostics,
        }
    }
}

/// The G66 body fingerprint: BLAKE3 over the block's token stream as `proc_macro2` renders it.
/// Rendering is canonical (source whitespace and comments are not tokens, and no span is printed),
/// so the fingerprint survives moves and reformatting but changes with any token, literals
/// included.
fn body_fingerprint(body: &syn::Block) -> String {
    use quote::ToTokens;
    atlas_core::IntegrityDigest::of_bytes(body.to_token_stream().to_string().as_bytes())
        .as_str()
        .to_owned()
}

#[cfg(test)]
mod tests;
