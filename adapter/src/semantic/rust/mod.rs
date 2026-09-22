//! Real Rust semantic extractor (R4.3).
//!
//! Scope is strictly four dimensions: SYMBOL, TYPE, FUNCTION_IDENTITY, FUNCTION_SIGNATURE. Every
//! other requested dimension (CALL, CONTROL_FLOW, DATA_FLOW, STATE, EFFECT, OWNERSHIP,
//! CONCURRENCY, PERSISTENCE) is always explicit `UNSUPPORTED` here — R4.4+ territory, never
//! silently omitted.
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
//! `TypeIdentity.canonical` stays `None` for every observation this extractor produces. A
//! malformed file never silently disappears: parse failure yields a `ParseFailure` diagnostic plus
//! explicit `UNKNOWN` for all four supported dimensions, with the artifact still represented.

mod spelling;

use std::collections::{BTreeMap, BTreeSet};

use atlas_core::{
    EpistemicStatus, Evidence, EvidenceId, FunctionIdentity, FunctionParameter, FunctionSignature,
    Provenance, SemanticDimension, SemanticObservation, SemanticRecordHeader, SemanticRecordId,
    SemanticScope, SymbolIdentity, SymbolRole, TypeIdentity, stable_id,
};
use syn::spanned::Spanned;

use super::batch::{ExtractionBatch, ObligationResult};
use super::extractor::{DiagnosticCode, ExtractionDiagnostic, ExtractionInput, SemanticExtractor};

pub const RUST_SEMANTIC_EXTRACTOR_ID: &str = "atlas.rust.source-semantic.v1";
pub const RUST_SEMANTIC_EXTRACTOR_VERSION: &str = "0.1.0";

/// Exactly the four dimensions this wave observes from parser-visible declaration syntax. Every
/// other `SemanticDimension` is out of scope for R4.3 and always answered `UNSUPPORTED`.
pub const SUPPORTED_DIMENSIONS: &[SemanticDimension] = &[
    SemanticDimension::Symbol,
    SemanticDimension::Type,
    SemanticDimension::FunctionIdentity,
    SemanticDimension::FunctionSignature,
];

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

    fn function_identity(
        &self,
        scope: &SemanticScope,
        name: &str,
        span: atlas_core::SourceSpan,
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
                role: SymbolRole::Definition,
            },
            span,
            generated: false,
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
        let observation = SemanticObservation::FunctionIdentity(header);
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

    fn handle_function(
        &mut self,
        name: &str,
        visibility: String,
        sig: &syn::Signature,
        scope: &SemanticScope,
        span: atlas_core::SourceSpan,
        role: SymbolRole,
    ) {
        self.emit_symbol(scope, name, role, span.clone());

        let identity = self.function_identity(scope, name, span);
        self.emit_function_identity(identity.clone());

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

        let generics = sig
            .generics
            .params
            .iter()
            .map(spelling::generic_param_spelling)
            .collect();
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
                    scope,
                    span,
                    SymbolRole::Definition,
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
                let role = if method.default.is_some() {
                    SymbolRole::Definition
                } else {
                    SymbolRole::Declaration
                };
                let method_name = method.sig.ident.to_string();
                let method_span = self.span_of(method);
                self.handle_function(
                    &method_name,
                    "inherited".to_owned(),
                    &method.sig,
                    &nested,
                    method_span,
                    role,
                );
            }
        }
    }

    fn handle_impl(&mut self, item: &syn::ItemImpl, scope: &SemanticScope) {
        let self_type = spelling::type_spelling(&item.self_ty);
        let segment = match &item.trait_ {
            Some((trait_path, _)) => {
                format!(
                    "impl:{} for {self_type}",
                    spelling::path_spelling(trait_path)
                )
            }
            None => format!("impl:{self_type}"),
        };
        let nested = nested_scope(scope, &segment);
        for impl_item in &item.items {
            if let syn::ImplItem::Fn(method) = impl_item {
                let method_name = method.sig.ident.to_string();
                let visibility = spelling::visibility_spelling(&method.vis);
                let method_span = self.span_of(method);
                self.handle_function(
                    &method_name,
                    visibility,
                    &method.sig,
                    &nested,
                    method_span,
                    SymbolRole::Definition,
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
                match self.dimension_records.remove(&dimension) {
                    Some((ids, refs)) => {
                        obligations.push(ObligationResult::observed(dimension, ids, refs))
                    }
                    None => {
                        // Verified absence: the file parsed successfully (an exhaustive scan) and
                        // simply declares nothing of this kind. OBSERVED with zero observation_ids
                        // but a non-empty evidence_ref, never a bare "not found".
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
