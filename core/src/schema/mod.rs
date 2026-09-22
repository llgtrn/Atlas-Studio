//! Stable repository-facing schemas and reports.
//!
//! These records are data contracts. Mechanics that populate them belong in adapter/runtime.

use crate::{
    census::InventoryReport,
    constraint::CodingAdmission,
    evidence::Evidence,
    language::adl::AdlCompileReport,
    provenance::Provenance,
    semantic::{ExtractionDiagnostic, SemanticObligationRecord, SemanticObservation},
    state::RepositorySnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoManifest {
    pub schema: String,
    pub repo: String,
    pub system_kind: String,
    pub backend_language: String,
    pub frontend_language: String,
    pub coding_requires_docs_gate: bool,
    pub graph_before_code_required: bool,
    pub exact_base_sha_required: bool,
    pub single_repository_target_required: bool,
    pub knowledge_root: String,
    pub temporary_root: String,
    pub provenance_root: String,
    pub license_root: String,
    pub source_roots: Vec<String>,
    pub backend_roots: Vec<String>,
    pub frontend_roots: Vec<String>,
    pub test_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileFact {
    pub path: String,
    pub language: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceReport {
    pub schema: String,
    pub root: String,
    pub files_total: usize,
    pub languages: BTreeMap<String, usize>,
    pub files: Vec<FileFact>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticFactKind {
    SourceArtifact,
    DeclaredNode,
    DeclaredEdge,
    Binding,
    Constraint,
    Invariant,
    ConstraintResult,
    Diagnostic,
    Transform,
    Materialization,
    /// A `SemanticExtractor`-observed SYMBOL, projected from a real `SemanticObservation::Symbol`
    /// kernel record into the bootstrap census/graph triple envelope
    /// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-acceptance-matrix`: "SemanticFact remains a
    /// compatibility envelope, not the only semantic type system" -- the typed kernel record
    /// remains the source of truth this is projected *from*, never invented independently here).
    Symbol,
    /// A `SemanticExtractor`-observed TYPE (source spelling only; never a compiler-resolved
    /// canonical identity). See `Symbol` above for why this projection exists.
    Type,
    FunctionIdentity,
    FunctionSignature,
    /// One requested `SemanticDimension`'s obligation status (`OBSERVED`/`UNKNOWN`/`UNSUPPORTED`/
    /// ...) for one (artifact, extractor) pair, independent of whether that dimension produced any
    /// individual `Symbol`/`Type`/`FunctionIdentity`/`FunctionSignature` fact. This is what makes a
    /// dimension's status visible *per artifact* in the canonical census, not only as a single
    /// repository-wide summary (see `CensusReport.coverage`'s own doc comment).
    SemanticObligation,
}

impl SemanticFactKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SourceArtifact => "SOURCE_ARTIFACT",
            Self::DeclaredNode => "DECLARED_NODE",
            Self::DeclaredEdge => "DECLARED_EDGE",
            Self::Binding => "BINDING",
            Self::Constraint => "CONSTRAINT",
            Self::Invariant => "INVARIANT",
            Self::ConstraintResult => "CONSTRAINT_RESULT",
            Self::Diagnostic => "DIAGNOSTIC",
            Self::Transform => "TRANSFORM",
            Self::Materialization => "MATERIALIZATION",
            Self::Symbol => "SYMBOL",
            Self::Type => "TYPE",
            Self::FunctionIdentity => "FUNCTION_IDENTITY",
            Self::FunctionSignature => "FUNCTION_SIGNATURE",
            Self::SemanticObligation => "SEMANTIC_OBLIGATION",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpistemicStatus {
    Observed,
    Declared,
    Derived,
    Inferred,
    Hypothesis,
    Conflict,
    Unknown,
    Unsupported,
    Ignored,
}

impl EpistemicStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "OBSERVED",
            Self::Declared => "DECLARED",
            Self::Derived => "DERIVED",
            Self::Inferred => "INFERRED",
            Self::Hypothesis => "HYPOTHESIS",
            Self::Conflict => "CONFLICT",
            Self::Unknown => "UNKNOWN",
            Self::Unsupported => "UNSUPPORTED",
            Self::Ignored => "IGNORED",
        }
    }
}

/// Bootstrap R4 interchange envelope.
///
/// Deep R4 semantics converge on typed records defined by the semantic-facts contract; this
/// subject/predicate/object carrier must not become Atlas's permanent universal semantic model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticFact {
    pub id: String,
    pub kind: SemanticFactKind,
    pub status: EpistemicStatus,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub provenance: Provenance,
}

/// Machine-enforced count accounting for the typed semantic world, R4.3.3.
///
/// `facts_total`/`input_facts_total`/`normalized_facts_total` (below) count the compatibility
/// `SemanticFact` projection only; they cannot detect a bug that silently drops a typed record
/// while leaving `facts` untouched, because `facts` is a *lossy derived projection* and never
/// carried the dropped information in the first place. `TypedClosureAccounting` is the same
/// stored-total-vs-live-length pattern `facts_total` already uses (a count captured at
/// construction time, compared against the live `Vec::len()` later), applied to the canonical
/// typed carriers instead: a caller that mutates `typed_semantic_records`/`typed_obligations`
/// after construction (or a bug that drops one before it) is caught by `is_closed()` returning
/// `false`, independent of whatever `facts`/`facts_total` still say
/// (`.atlas/contracts/CENSUS-COMPLETENESS.md`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypedClosureAccounting {
    pub typed_observations_total: usize,
    pub typed_obligations_total: usize,
}

/// `true` iff every `evidence_refs`/`diagnostic_ids`/`observation_ids` reference on every typed
/// observation/obligation resolves against the given `evidence`/`diagnostics`/`typed_semantic_records`
/// collections. An obligation's `observation_ids` may legitimately be empty (`UNKNOWN`/
/// `UNSUPPORTED`/`IGNORED`, or a verified-absence `OBSERVED` result backed only by evidence), so an
/// empty list is never itself a dangling reference.
fn typed_references_resolve(
    typed_semantic_records: &[SemanticObservation],
    typed_obligations: &[SemanticObligationRecord],
    evidence: &[Evidence],
    diagnostics: &[ExtractionDiagnostic],
) -> bool {
    let evidence_ids: BTreeSet<&str> = evidence.iter().map(|item| item.id.as_str()).collect();
    let diagnostic_ids: BTreeSet<&str> = diagnostics.iter().map(|item| item.id.as_str()).collect();
    let observation_ids: BTreeSet<&str> = typed_semantic_records
        .iter()
        .map(|observation| observation.record_id().as_str())
        .collect();

    let observations_ok = typed_semantic_records.iter().all(|observation| {
        observation
            .evidence_refs()
            .iter()
            .all(|id| evidence_ids.contains(id.as_str()))
    });

    let obligations_ok = typed_obligations.iter().all(|obligation| {
        obligation
            .evidence_refs
            .iter()
            .all(|id| evidence_ids.contains(id.as_str()))
            && obligation
                .diagnostic_ids
                .iter()
                .all(|id| diagnostic_ids.contains(id.as_str()))
            && obligation
                .observation_ids
                .iter()
                .all(|id| observation_ids.contains(id.as_str()))
    });

    observations_ok && obligations_ok
}

/// The canonical census representation, R4.3.2/R4.3.3.
///
/// `typed_semantic_records`/`evidence`/`diagnostics`/`typed_obligations` are the lossless
/// carriers: every `SemanticObservation` a `SemanticExtractor` produced, every `Evidence` record
/// backing one, every `ExtractionDiagnostic` explaining an `UNKNOWN`/`UNSUPPORTED` obligation, and
/// every obligation-lineage record connecting an (artifact, extractor, dimension) to its status,
/// observations, evidence and diagnostics -- retained verbatim (only deterministically ordered and
/// de-duplicated where the same RAW record legitimately repeats, per
/// `SemanticObservation::raw_observation_id`) -- no field of a
/// `FunctionSignature`/`TypeIdentity`/`SymbolIdentity`/`FunctionIdentity` is lost between
/// extraction and this report, and independent extractors reporting the same semantic claim never
/// silently overwrite each other (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`).
///
/// `facts` remains the bootstrap `SemanticFact { subject, predicate, object }` envelope, but it is
/// now explicitly a *derived compatibility projection* of `typed_semantic_records` (plus
/// artifact/ADL facts that have no richer typed kernel record yet), never an independent source of
/// truth (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-acceptance-matrix`: "SemanticFact remains a
/// compatibility envelope, not the only semantic type system"). Removing `facts` entirely would
/// lose only display convenience, never semantic content: everything it carries for a
/// Symbol/Type/FunctionIdentity/FunctionSignature fact is already present, in full structural
/// fidelity, in `typed_semantic_records`. Accordingly, `facts_total` is compatibility/report
/// accounting only -- a count of the lossy projection, never a measure of total semantic
/// knowledge; `typed_closure` is the canonical accounting for that.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CensusReport {
    pub schema: String,
    pub artifacts_total: usize,
    pub artifacts_accounted_total: usize,
    /// Count of the compatibility `facts` projection only -- see the struct-level doc comment.
    /// Never the canonical measure of semantic completeness; use `typed_closure` for that.
    pub facts_total: usize,
    /// Bootstrap / derived coverage projection: at most one `EpistemicStatus` per dimension key,
    /// aggregated across every artifact and extractor that addressed it (most-informative status
    /// wins: OBSERVED > CONFLICT > UNKNOWN > IGNORED > UNSUPPORTED). This is a noncanonical summary
    /// shape kept for existing callers, never the canonical multi-extractor accounting. Independent
    /// extractors that disagree on a dimension, and each artifact's own per-dimension status, are
    /// preserved without collapsing in `facts` (`SemanticFactKind::SemanticObligation`), in
    /// `typed_obligations`, and in `runtime::census::CensusExtractionAccounting`, not only in this
    /// single summary map (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`).
    pub coverage: BTreeMap<String, EpistemicStatus>,
    /// The lossless typed semantic world: every RAW `SemanticObservation` a `SemanticExtractor`
    /// produced this census run, unmodified (record identity, dimension, epistemic status,
    /// subject -- with every field of its family, e.g. `FunctionSignature`'s parameters/generics/
    /// abi/visibility/async/unsafe/extern/return type -- scope, repository, revision, extractor
    /// id+version, evidence refs, provenance/span), sorted by `record_id` for determinism and
    /// de-duplicated only by `raw_observation_id` (an EXACT raw-content match) -- never by
    /// `record_id` alone, which would silently erase one extractor's observation whenever another
    /// extractor legitimately reports the same semantic claim.
    pub typed_semantic_records: Vec<SemanticObservation>,
    /// Every `Evidence` record backing a `typed_semantic_records` entry (resolve an
    /// observation's `evidence_refs` against this list), de-duplicated by id, sorted by id.
    pub evidence: Vec<Evidence>,
    /// Every `ExtractionDiagnostic` an extractor reported this run (resolve an `ObligationResult`'s
    /// `diagnostics` ids -- see `runtime::census::CensusExtractionAccounting` -- against this list
    /// to distinguish, for example, a read failure `InvalidInput` from a parser `ParseFailure`),
    /// de-duplicated by id, sorted by id.
    pub diagnostics: Vec<ExtractionDiagnostic>,
    /// The canonical obligation ledger: one `SemanticObligationRecord` per (artifact, extractor,
    /// dimension) this run addressed, connecting its status to the observations/evidence/
    /// diagnostics that justify it. Answers "which extractor evaluated this dimension, for which
    /// artifact, at which revision, with what result, and why" from this report alone -- without
    /// needing the original transient `ExtractionBatch`es (`.atlas/contracts/CENSUS-COMPLETENESS.md`).
    pub typed_obligations: Vec<SemanticObligationRecord>,
    /// This run's own typed-record/obligation counts, captured at construction. See
    /// `TypedClosureAccounting`'s doc comment for why this exists alongside `facts_total`.
    pub typed_closure: TypedClosureAccounting,
    pub facts: Vec<SemanticFact>,
}

impl CensusReport {
    /// `true` iff every typed observation/obligation this report claims to carry is still present
    /// (count matches what was recorded at construction) and every cross-reference among them
    /// (`evidence_refs`, `diagnostic_ids`, `observation_ids`) resolves within this same report --
    /// independent of whatever the compatibility `facts` projection says.
    pub fn typed_semantics_closed(&self) -> bool {
        self.typed_closure.typed_observations_total == self.typed_semantic_records.len()
            && self.typed_closure.typed_obligations_total == self.typed_obligations.len()
            && typed_references_resolve(
                &self.typed_semantic_records,
                &self.typed_obligations,
                &self.evidence,
                &self.diagnostics,
            )
    }

    pub fn is_closed(&self) -> bool {
        self.artifacts_total == self.artifacts_accounted_total
            && self.facts_total == self.facts.len()
            && self.typed_semantics_closed()
    }
}

/// R4.3.2/R4.3.3: normalization is now typed-semantic aware. `typed_semantic_records`/`evidence`/
/// `diagnostics`/`typed_obligations` flow through from `CensusReport` under a minimal N0
/// normalization (deterministic ordering only -- no restructuring, no dedup beyond exact
/// raw-identity collapse, no status/provenance/evidence mutation): `normalize(record) == record`,
/// up to canonical ordering. This is `normalize(typed record) -> equivalent typed record`, never
/// `typed record -> display string -> normalized display string`
/// (`.atlas/contracts/NORMALIZATION.md`).
///
/// `input_facts_total`/`normalized_facts_total` count the compatibility `facts` projection only --
/// see `CensusReport.facts_total`'s doc comment for why this is not the canonical closure measure.
/// `input_typed_closure`/`normalized_typed_closure` are the typed-world equivalent, letting
/// `is_closed()` catch a dropped typed record/obligation independent of `facts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizationReport {
    pub schema: String,
    pub input_facts_total: usize,
    pub normalized_facts_total: usize,
    pub kinds: BTreeMap<String, usize>,
    /// The typed semantic world, deterministically ordered by `record_id`. Identity, status,
    /// subject (full structure), scope, repository, revision, extractor and provenance are
    /// preserved verbatim from `CensusReport.typed_semantic_records` -- normalization here means
    /// canonical ordering, not restructuring.
    pub typed_semantic_records: Vec<SemanticObservation>,
    pub evidence: Vec<Evidence>,
    pub diagnostics: Vec<ExtractionDiagnostic>,
    /// The typed obligation ledger, carried through from `CensusReport.typed_obligations` under
    /// the same N0 rules: deterministic ordering by `obligation_id`, no reconciliation, no winner
    /// selection, no epistemic promotion, no evidence/diagnostic loss.
    pub typed_obligations: Vec<SemanticObligationRecord>,
    /// `CensusReport.typed_closure` as it stood going INTO this normalization run.
    pub input_typed_closure: TypedClosureAccounting,
    /// This report's own typed-record/obligation counts, captured AFTER normalization.
    pub normalized_typed_closure: TypedClosureAccounting,
    pub facts: Vec<SemanticFact>,
}

impl NormalizationReport {
    /// `true` iff every typed observation/obligation normalization received is still present
    /// (`input_typed_closure == normalized_typed_closure`, and both match the live `Vec::len()`s),
    /// and every cross-reference among them still resolves within this report -- independent of
    /// whatever the compatibility `facts` projection says.
    pub fn typed_semantics_closed(&self) -> bool {
        self.input_typed_closure.typed_observations_total
            == self.normalized_typed_closure.typed_observations_total
            && self.normalized_typed_closure.typed_observations_total
                == self.typed_semantic_records.len()
            && self.input_typed_closure.typed_obligations_total
                == self.normalized_typed_closure.typed_obligations_total
            && self.normalized_typed_closure.typed_obligations_total == self.typed_obligations.len()
            && typed_references_resolve(
                &self.typed_semantic_records,
                &self.typed_obligations,
                &self.evidence,
                &self.diagnostics,
            )
    }

    pub fn is_closed(&self) -> bool {
        self.input_facts_total == self.normalized_facts_total
            && self.normalized_facts_total == self.facts.len()
            && self.typed_semantics_closed()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocsReport {
    pub schema: String,
    pub standard: String,
    pub root: String,
    pub gate_ready: bool,
    pub hard_violations_total: usize,
    pub documents_total: usize,
    pub canonical_frontmatter_total: usize,
    pub required_control_docs_missing: Vec<String>,
    pub missing_frontmatter: Vec<String>,
    pub documents: Vec<DocumentFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentFact {
    pub path: String,
    pub id: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub canonical: bool,
    pub title: Option<String>,
    pub headings: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoAudit {
    pub schema: String,
    pub archetype: String,
    pub manifest: Option<RepoManifest>,
    pub manifest_ready: bool,
    pub policy_violations: Vec<String>,
    pub missing_required_roles: Vec<String>,
    pub missing_mapped_paths: Vec<String>,
    pub forbidden_roots_present: Vec<String>,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSummary {
    pub schema: String,
    pub semantic_grade: String,
    pub nodes_total: usize,
    pub edges_total: usize,
    pub bindings_total: usize,
    pub facts_total: usize,
    pub language_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemizeReport {
    pub schema: String,
    pub cli_api: String,
    pub root: String,
    pub snapshot: RepositorySnapshot,
    pub repository: RepoAudit,
    pub docs: DocsReport,
    pub adl: AdlCompileReport,
    pub coding_admission: CodingAdmission,
    pub inventory: InventoryReport,
    pub source: SourceReport,
    pub census: CensusReport,
    pub normalization: NormalizationReport,
    pub graph: GraphSummary,
    pub invariants: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{ArtifactId, EvidenceId, RepositoryId};
    use crate::provenance::provenance;
    use crate::semantic::{
        ExtractorIdentity, SemanticDimension, SemanticObligationRecord, SemanticObservation,
        SemanticRecordHeader, SemanticRecordId, SemanticScope, SymbolIdentity, SymbolRole,
    };
    use crate::temporal::RevisionRef;

    fn repository() -> RepositoryId {
        RepositoryId::new("atlas-studio")
    }

    fn revision() -> RevisionRef {
        RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        }
    }

    fn extractor() -> ExtractorIdentity {
        ExtractorIdentity {
            id: "atlas.test".into(),
            version: "0.1.0".into(),
        }
    }

    fn symbol_observation(evidence_id: &str) -> SemanticObservation {
        let symbol = SymbolIdentity {
            repository: repository(),
            revision: revision(),
            scope: SemanticScope::new(Vec::<String>::new()),
            name: "known".into(),
            role: SymbolRole::Definition,
        };
        let record_id = SemanticRecordId::new(SemanticDimension::Symbol, &symbol.identity_key());
        SemanticObservation::Symbol(SemanticRecordHeader {
            record_id,
            dimension: SemanticDimension::Symbol,
            status: EpistemicStatus::Observed,
            subject: symbol,
            scope: SemanticScope::new(Vec::<String>::new()),
            repository: repository(),
            revision: revision(),
            extractor: extractor(),
            evidence_refs: vec![EvidenceId::new(evidence_id.to_owned())],
            provenance: provenance("src/lib.rs", "atlas.test"),
        })
    }

    fn evidence_record(id: &str) -> Evidence {
        Evidence {
            id: id.into(),
            kind: "PARSER_OUTPUT".into(),
            path: "src/lib.rs".into(),
            summary: "test".into(),
            revision: None,
        }
    }

    fn obligation_record(
        observation_ids: Vec<SemanticRecordId>,
        evidence_refs: Vec<EvidenceId>,
        diagnostic_ids: Vec<String>,
    ) -> SemanticObligationRecord {
        SemanticObligationRecord::new(
            ArtifactId::new("artifact:src/lib.rs"),
            repository(),
            revision(),
            extractor(),
            SemanticDimension::Symbol,
            EpistemicStatus::Observed,
            observation_ids,
            evidence_refs,
            diagnostic_ids,
        )
    }

    fn valid_census() -> CensusReport {
        let observation = symbol_observation("evidence:known");
        let record_id = observation.record_id().clone();
        let typed_semantic_records = vec![observation];
        let typed_obligations = vec![obligation_record(
            vec![record_id],
            vec![EvidenceId::new("evidence:known")],
            Vec::new(),
        )];
        CensusReport {
            schema: "test".into(),
            artifacts_total: 1,
            artifacts_accounted_total: 1,
            facts_total: 0,
            coverage: BTreeMap::new(),
            typed_closure: TypedClosureAccounting {
                typed_observations_total: typed_semantic_records.len(),
                typed_obligations_total: typed_obligations.len(),
            },
            typed_semantic_records,
            evidence: vec![evidence_record("evidence:known")],
            diagnostics: Vec::new(),
            typed_obligations,
            facts: Vec::new(),
        }
    }

    fn valid_normalization(census: &CensusReport) -> NormalizationReport {
        NormalizationReport {
            schema: "test".into(),
            input_facts_total: 0,
            normalized_facts_total: 0,
            kinds: BTreeMap::new(),
            typed_semantic_records: census.typed_semantic_records.clone(),
            evidence: census.evidence.clone(),
            diagnostics: census.diagnostics.clone(),
            typed_obligations: census.typed_obligations.clone(),
            input_typed_closure: census.typed_closure,
            normalized_typed_closure: census.typed_closure,
            facts: Vec::new(),
        }
    }

    // --- Required test 10: typed Census closure fails if an observation disappears -----------

    #[test]
    fn valid_census_report_is_typed_closed() {
        let census = valid_census();
        assert!(census.typed_semantics_closed());
        assert!(census.is_closed());
    }

    #[test]
    fn census_closure_fails_if_a_typed_observation_disappears() {
        let mut census = valid_census();
        census.typed_semantic_records.clear();
        assert!(!census.typed_semantics_closed());
        assert!(!census.is_closed());
    }

    #[test]
    fn census_closure_fails_if_a_typed_obligation_disappears() {
        let mut census = valid_census();
        census.typed_obligations.clear();
        assert!(!census.typed_semantics_closed());
    }

    // --- Required test 12/13: closure fails on a dangling evidence/diagnostic reference -------

    #[test]
    fn census_closure_fails_on_dangling_evidence_reference() {
        let mut census = valid_census();
        census.evidence.clear();
        assert!(
            !census.typed_semantics_closed(),
            "the surviving observation's evidence_refs now points nowhere"
        );
    }

    #[test]
    fn census_closure_fails_on_dangling_diagnostic_reference() {
        let mut census = valid_census();
        census.typed_obligations[0]
            .diagnostic_ids
            .push("missing-diagnostic".into());
        assert!(!census.typed_semantics_closed());
    }

    #[test]
    fn census_closure_fails_on_dangling_obligation_observation_reference() {
        let mut census = valid_census();
        census.typed_semantic_records.clear();
        census.typed_closure.typed_observations_total = 0;
        // The obligation still claims to be satisfied by an observation that no longer exists.
        assert!(!census.typed_semantics_closed());
    }

    // --- Required test 11: typed normalization closure fails if a record disappears -----------

    #[test]
    fn valid_normalization_report_is_typed_closed() {
        let census = valid_census();
        let normalization = valid_normalization(&census);
        assert!(normalization.typed_semantics_closed());
        assert!(normalization.is_closed());
    }

    #[test]
    fn normalization_closure_fails_if_a_typed_record_disappears() {
        let census = valid_census();
        let mut normalization = valid_normalization(&census);
        normalization.typed_semantic_records.clear();
        assert!(!normalization.typed_semantics_closed());
        assert!(!normalization.is_closed());
    }

    #[test]
    fn normalization_closure_fails_on_dangling_evidence_reference() {
        let census = valid_census();
        let mut normalization = valid_normalization(&census);
        normalization.evidence.clear();
        assert!(!normalization.typed_semantics_closed());
    }

    #[test]
    fn normalization_closure_fails_on_dangling_diagnostic_reference() {
        let census = valid_census();
        let mut normalization = valid_normalization(&census);
        normalization.typed_obligations[0]
            .diagnostic_ids
            .push("missing-diagnostic".into());
        assert!(!normalization.typed_semantics_closed());
    }
}
