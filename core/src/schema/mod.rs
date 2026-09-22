//! Stable repository-facing schemas and reports.
//!
//! These records are data contracts. Mechanics that populate them belong in adapter/runtime.

use crate::{
    census::InventoryReport,
    constraint::CodingAdmission,
    evidence::Evidence,
    language::adl::AdlCompileReport,
    provenance::Provenance,
    semantic::{ExtractionDiagnostic, SemanticObservation},
    state::RepositorySnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// The canonical census representation, R4.3.2.
///
/// `typed_semantic_records`/`evidence`/`diagnostics` are the lossless carriers: every
/// `SemanticObservation` a `SemanticExtractor` produced, every `Evidence` record backing one, and
/// every `ExtractionDiagnostic` explaining an `UNKNOWN`/`UNSUPPORTED` obligation, retained verbatim
/// (only deterministically ordered and de-duplicated by id where the same record legitimately
/// repeats across batches) -- no field of a `FunctionSignature`/`TypeIdentity`/`SymbolIdentity`/
/// `FunctionIdentity` is lost between extraction and this report.
///
/// `facts` remains the bootstrap `SemanticFact { subject, predicate, object }` envelope, but it is
/// now explicitly a *derived compatibility projection* of `typed_semantic_records` (plus
/// artifact/ADL facts that have no richer typed kernel record yet), never an independent source of
/// truth (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-acceptance-matrix`: "SemanticFact remains a
/// compatibility envelope, not the only semantic type system"). Removing `facts` entirely would
/// lose only display convenience, never semantic content: everything it carries for a
/// Symbol/Type/FunctionIdentity/FunctionSignature fact is already present, in full structural
/// fidelity, in `typed_semantic_records`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CensusReport {
    pub schema: String,
    pub artifacts_total: usize,
    pub artifacts_accounted_total: usize,
    pub facts_total: usize,
    /// Bootstrap / derived coverage projection: at most one `EpistemicStatus` per dimension key,
    /// aggregated across every artifact and extractor that addressed it (most-informative status
    /// wins: OBSERVED > CONFLICT > UNKNOWN > IGNORED > UNSUPPORTED). This is a noncanonical summary
    /// shape kept for existing callers, never the canonical multi-extractor accounting. Independent
    /// extractors that disagree on a dimension, and each artifact's own per-dimension status, are
    /// preserved without collapsing in `facts` (`SemanticFactKind::SemanticObligation`) and in
    /// `runtime::census::CensusExtractionAccounting`, not only in this single summary map
    /// (`.atlas/contracts/SEMANTIC-EXTRACTION.md#multi-engine-extraction`).
    pub coverage: BTreeMap<String, EpistemicStatus>,
    /// The lossless typed semantic world: every `SemanticObservation` a `SemanticExtractor`
    /// produced this census run, unmodified (record identity, dimension, epistemic status,
    /// subject -- with every field of its family, e.g. `FunctionSignature`'s parameters/generics/
    /// abi/visibility/async/unsafe/extern/return type -- scope, repository, revision, extractor
    /// id+version, evidence refs, provenance/span), sorted by `record_id` for determinism.
    pub typed_semantic_records: Vec<SemanticObservation>,
    /// Every `Evidence` record backing a `typed_semantic_records` entry (resolve an
    /// observation's `evidence_refs` against this list), de-duplicated by id, sorted by id.
    pub evidence: Vec<Evidence>,
    /// Every `ExtractionDiagnostic` an extractor reported this run (resolve an `ObligationResult`'s
    /// `diagnostics` ids -- see `runtime::census::CensusExtractionAccounting` -- against this list
    /// to distinguish, for example, a read failure `InvalidInput` from a parser `ParseFailure`),
    /// de-duplicated by id, sorted by id.
    pub diagnostics: Vec<ExtractionDiagnostic>,
    pub facts: Vec<SemanticFact>,
}

impl CensusReport {
    pub fn is_closed(&self) -> bool {
        self.artifacts_total == self.artifacts_accounted_total
            && self.facts_total == self.facts.len()
    }
}

/// R4.3.2: normalization is now typed-semantic aware. `typed_semantic_records`/`evidence`/
/// `diagnostics` flow through from `CensusReport` under a minimal N0 normalization (deterministic
/// ordering only -- no restructuring, no dedup beyond exact-id collapse, no status/provenance/
/// evidence mutation): `normalize(record) == record`, up to canonical ordering. This is
/// `normalize(typed record) -> equivalent typed record`, never `typed record -> display string ->
/// normalized display string` (`.atlas/contracts/NORMALIZATION.md`).
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
    pub facts: Vec<SemanticFact>,
}

impl NormalizationReport {
    pub fn is_closed(&self) -> bool {
        self.input_facts_total == self.normalized_facts_total
            && self.normalized_facts_total == self.facts.len()
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
