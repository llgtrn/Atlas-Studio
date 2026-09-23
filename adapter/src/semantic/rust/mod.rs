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
//! a call syntactically occurs and WHO makes it, never WHOM it calls. CONTROL_FLOW (R4.6) and
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
mod ownership;
mod persistence;
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
        // zero-observation result as verified absence.
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
        let mut ctx = ExtractionContext::new(input, self.identity());
        let risk = max_structural_recursion_risk(&input.source_text);
        if risk > MAX_STRUCTURAL_RECURSION_RISK {
            return ctx.finish_resource_limit(risk);
        }
        match syn::parse_file(&input.source_text) {
            Ok(file) => {
                let root_scope = SemanticScope::new(Vec::<String>::new());
                for item in &file.items {
                    ctx.walk_item(item, &root_scope);
                }
                ctx.finish_success()
            }
            Err(error) => ctx.finish_parse_failure(&error),
        }
    }
}

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
/// Three independent constructs were confirmed (real-process, isolated reproduction) to drive this
/// recursion to a crash:
/// - explicit bracket nesting (`(((...)))`) -- 300 levels reliably overflowed a reduced (~2MB)
///   test-thread stack; 100 levels did not;
/// - a bracket-free chained binary-operator expression (`1+1+1+...`) -- 2,000 terms reliably
///   overflowed the same stack; 1,000 terms did not;
/// - an `if ... else if ... else if ... else { .. }` chain -- each arm's own braces are siblings,
///   not nested (bracket-nesting depth stays at 2 regardless of chain length), and there is no
///   operator-character run either (`if`/`else` are keywords), yet the AST is exactly as deeply
///   *recursive* as the bracket case (`Expr::If { then, else: Some(Box<Expr::If{..}>) }` nests one
///   level per arm) -- 3,000 arms reliably overflowed the same stack.
///
/// `max_structural_recursion_risk` bounds all three with one combined metric rather than
/// maintaining separate ad-hoc scans, since all are fundamentally the same risk (parser recursion
/// depth proportional to admitted-input structure, regardless of which concrete syntax drives it):
/// bracket nesting increments/decrements a depth counter as before; a run of consecutive
/// expression-continuation operator bytes (arithmetic, logical, comparison, method-chain `.`, ...)
/// at the *current* bracket depth also increments a counter, reset whenever a bracket boundary is
/// crossed; a run of `else` keyword occurrences (not reset by brace boundaries, since chain arms
/// are siblings at the same bracket depth -- only by `;` or a new `fn`, approximating "next
/// function") increments a third counter. The combined maximum is compared against one threshold.
///
/// This is a coarse, syntax-unaware, pre-parse text scan (no exclusion for string/char literals or
/// comments, no real keyword-boundary checking) that can only ever over-count real structural
/// risk, never under-count -- so it can only be more conservative than strictly necessary, never
/// miss one of these three specific, confirmed vectors. It is explicitly NOT claimed complete: a
/// fourth, fifth, ... construct that drives the same class of parser/AST recursion through some
/// other keyword or syntax shape may exist and would not necessarily be caught by this heuristic.
/// A fully complete fix would require either patching `syn` itself to grow its own stack during
/// recursion (impractical -- `syn` is a third-party dependency this bootstrap does not vendor or
/// fork) or replacing whole-file parsing with a from-scratch, formally depth-bounded parser (a
/// large undertaking, not attempted this wave). This repository's own entire real source corpus
/// never nests brackets deeper than 13, never chains more than a handful of operators in a row,
/// and has no `if`/`else` chain anywhere near this length.
const MAX_STRUCTURAL_RECURSION_RISK: usize = 64;

fn max_structural_recursion_risk(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut bracket_depth: usize = 0;
    let mut chain_run: usize = 0;
    let mut else_run: usize = 0;
    let mut max_risk: usize = 0;
    for (index, &byte) in bytes.iter().enumerate() {
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
            | b'.' | b'?' | b',' => {
                chain_run += 1;
                max_risk = max_risk.max(bracket_depth + chain_run);
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
            _ => {}
        }
        // Deliberately NOT reset on newline for either chain_run or else_run: an adversary could
        // otherwise trivially evade this guard by inserting a newline between every chained
        // operator or `else` arm, which drives the exact same parser recursion depth as an
        // unbroken line.
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
        }
    }

    fn wants(&self, dimension: SemanticDimension) -> bool {
        self.input.requested_dimensions.contains(&dimension)
    }

    fn span_of<T: Spanned>(&self, node: &T) -> atlas_core::SourceSpan {
        let start = node.span().start();
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
        debug_assert!(observation.is_dimension_consistent());
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
                debug_assert!(observation.is_dimension_consistent());
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
        debug_assert!(observation.is_dimension_consistent());
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
        debug_assert!(observation.is_dimension_consistent());
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
        if !self.wants(SemanticDimension::DataFlow) {
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

    /// The `PlaceRef` a CALL argument expression should carry: `Resolved` when `arg` is a simple
    /// single-identifier expression (the same shape `dataflow.rs`'s own `Expr::Path` arm
    /// recognizes, via the shared `spelling::simple_path_ident`), else `Unresolved`.
    fn place_ref_for_argument(&self, arg: &syn::Expr, caller: &SemanticRecordId) -> PlaceRef {
        let Some(ident) = spelling::simple_path_ident(arg) else {
            return PlaceRef::Unresolved;
        };
        self.place_ref_for_value(
            &ident.to_string(),
            self.span_of(arg),
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
                let span = self.span_of(call);
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
                let span = self.span_of(method_call);
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
    /// binding's Definition, or a plain assignment's Store) IF `expr` is exactly a `Call`/
    /// `MethodCall` expression -- e.g. the direct initializer of `let y = helper(x);`, not a nested
    /// subexpression like `helper(x) + 1` (which has no single value this call's result "becomes").
    /// Delegates to the ordinary `walk_expr` (implying `PlaceRef::Unresolved`) for every other case.
    fn walk_value_position_expr(
        &mut self,
        expr: &syn::Expr,
        scope: &SemanticScope,
        caller: &SemanticRecordId,
        result: PlaceRef,
    ) {
        match expr {
            syn::Expr::Call(_) | syn::Expr::MethodCall(_) => {
                self.walk_call_like(expr, scope, caller, result);
            }
            other => self.walk_expr(other, scope, caller),
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
        debug_assert!(observation.is_dimension_consistent());
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
            syn::Stmt::Macro(_) => {}
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
            // A bare macro invocation used as an expression (`Expr::Macro`) is opaque token-stream
            // input this extractor never re-parses as expressions without macro expansion, which
            // it never performs (`.atlas/contracts/SEMANTIC-EXTRACTION.md`) -- a call written only
            // inside a macro invocation's arguments is a real, permanent, out-of-profile gap (see
            // `dimension_coverage`), not a silently-omitted one. Literals, bare paths, `continue`,
            // and any other/future `syn::Expr` shape structurally cannot contain a nested call.
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

        let generics: Vec<String> = sig
            .generics
            .params
            .iter()
            .map(spelling::generic_param_spelling)
            .collect();

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
            self.emit_symbol(
                &nested,
                &variant.ident.to_string(),
                SymbolRole::Definition,
                variant_span,
            );
            for field in &variant.fields {
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
            if let syn::TraitItem::Fn(method) = trait_item {
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
            if let syn::ImplItem::Fn(method) = impl_item {
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
                if dimension_coverage(dimension) == DimensionCoverage::Partial {
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

#[cfg(test)]
mod tests;
