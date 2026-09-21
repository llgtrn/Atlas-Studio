//! Canonical R4 semantic ontology.
//!
//! These types separate what a fact means from how it is known, what evidence supports it, and
//! how source artifacts were dispositioned. The legacy SemanticFact triple remains a bootstrap
//! transport envelope; function-level semantics should converge on these typed records.

use crate::{
    census::ArtifactDisposition,
    identity::{EvidenceId, SymbolId},
    provenance::Provenance,
    schema::{EpistemicStatus, SemanticFactKind},
    temporal::RevisionRef,
};
use serde::{Deserialize, Serialize};

pub type FactKind = SemanticFactKind;
pub type Disposition = ArtifactDisposition;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKind {
    SourceCode,
    BuildMetadata,
    CompilerMetadata,
    Test,
    RuntimeTrace,
    Specification,
    Paper,
    Documentation,
    Declaration,
    ModelOutput,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticObligation {
    Symbol,
    Type,
    FunctionIdentity,
    FunctionSignature,
    Call,
    ControlFlow,
    DataFlow,
    StateAccess,
    Effect,
    Ownership,
    Concurrency,
    Persistence,
    EvidenceLink,
}

impl SemanticObligation {
    pub const ALL: [Self; 13] = [
        Self::Symbol,
        Self::Type,
        Self::FunctionIdentity,
        Self::FunctionSignature,
        Self::Call,
        Self::ControlFlow,
        Self::DataFlow,
        Self::StateAccess,
        Self::Effect,
        Self::Ownership,
        Self::Concurrency,
        Self::Persistence,
        Self::EvidenceLink,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Symbol => "SYMBOL",
            Self::Type => "TYPE",
            Self::FunctionIdentity => "FUNCTION_IDENTITY",
            Self::FunctionSignature => "FUNCTION_SIGNATURE",
            Self::Call => "CALL",
            Self::ControlFlow => "CONTROL_FLOW",
            Self::DataFlow => "DATA_FLOW",
            Self::StateAccess => "STATE_ACCESS",
            Self::Effect => "EFFECT",
            Self::Ownership => "OWNERSHIP",
            Self::Concurrency => "CONCURRENCY",
            Self::Persistence => "PERSISTENCE",
            Self::EvidenceLink => "EVIDENCE_LINK",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionIdentity {
    pub symbol: SymbolId,
    pub language: String,
    pub qualified_name: String,
    pub container: Option<SymbolId>,
    pub revision: Option<RevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionParameter {
    pub name: String,
    pub type_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionSignature {
    pub function: SymbolId,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: Option<String>,
    pub generics: Vec<String>,
    pub visibility: Option<String>,
    pub qualifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolFact {
    pub symbol: SymbolId,
    pub name: String,
    pub symbol_kind: String,
    pub container: Option<SymbolId>,
    pub declared_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeFact {
    pub subject: SymbolId,
    pub type_ref: String,
    pub resolved_symbol: Option<SymbolId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallDispatch {
    Static,
    Virtual,
    Dynamic,
    Indirect,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallFact {
    pub caller: SymbolId,
    pub callee: Option<SymbolId>,
    pub target_text: String,
    pub dispatch: CallDispatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlFlowFact {
    pub function: SymbolId,
    pub from_block: String,
    pub to_block: String,
    pub edge_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataFlowFact {
    pub function: SymbolId,
    pub from: String,
    pub to: String,
    pub flow_kind: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateAccessMode {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateAccessFact {
    pub function: SymbolId,
    pub state: String,
    pub access: StateAccessMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectFact {
    pub function: SymbolId,
    pub effect_kind: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnershipFact {
    pub function: SymbolId,
    pub subject: String,
    pub ownership_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConcurrencyFact {
    pub function: SymbolId,
    pub primitive: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistenceFact {
    pub function: SymbolId,
    pub operation: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceLink {
    pub fact_id: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "record_kind", content = "record", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticRecord {
    FunctionIdentity(FunctionIdentity),
    FunctionSignature(FunctionSignature),
    Symbol(SymbolFact),
    Type(TypeFact),
    Call(CallFact),
    ControlFlow(ControlFlowFact),
    DataFlow(DataFlowFact),
    StateAccess(StateAccessFact),
    Effect(EffectFact),
    Ownership(OwnershipFact),
    Concurrency(ConcurrencyFact),
    Persistence(PersistenceFact),
    EvidenceLink(EvidenceLink),
}

impl SemanticRecord {
    pub const fn kind(&self) -> SemanticFactKind {
        match self {
            Self::FunctionIdentity(_) => SemanticFactKind::FunctionIdentity,
            Self::FunctionSignature(_) => SemanticFactKind::FunctionSignature,
            Self::Symbol(_) => SemanticFactKind::Symbol,
            Self::Type(_) => SemanticFactKind::Type,
            Self::Call(_) => SemanticFactKind::Call,
            Self::ControlFlow(_) => SemanticFactKind::ControlFlow,
            Self::DataFlow(_) => SemanticFactKind::DataFlow,
            Self::StateAccess(_) => SemanticFactKind::StateAccess,
            Self::Effect(_) => SemanticFactKind::Effect,
            Self::Ownership(_) => SemanticFactKind::Ownership,
            Self::Concurrency(_) => SemanticFactKind::Concurrency,
            Self::Persistence(_) => SemanticFactKind::Persistence,
            Self::EvidenceLink(_) => SemanticFactKind::EvidenceLink,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypedSemanticFact {
    pub id: String,
    pub evidence_kind: EvidenceKind,
    pub status: EpistemicStatus,
    pub record: SemanticRecord,
    pub provenance: Provenance,
}

impl TypedSemanticFact {
    pub const fn kind(&self) -> SemanticFactKind {
        self.record.kind()
    }

    pub const fn is_epistemically_valid(&self) -> bool {
        !matches!(
            (self.evidence_kind, self.status),
            (EvidenceKind::ModelOutput, EpistemicStatus::Observed)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_record_reports_fact_kind_without_triple_encoding() {
        let fact = TypedSemanticFact {
            id: "fact:function".into(),
            evidence_kind: EvidenceKind::SourceCode,
            status: EpistemicStatus::Observed,
            record: SemanticRecord::FunctionIdentity(FunctionIdentity {
                symbol: SymbolId::new("symbol:f"),
                language: "rust".into(),
                qualified_name: "crate::f".into(),
                container: None,
                revision: None,
            }),
            provenance: Provenance {
                source_path: "src/lib.rs".into(),
                source_revision: None,
                extractor: "test".into(),
                content_hash: None,
                span: Some("1:1".into()),
            },
        };

        assert_eq!(fact.kind(), SemanticFactKind::FunctionIdentity);
        assert!(fact.is_epistemically_valid());
    }

    #[test]
    fn model_output_cannot_claim_observed_status() {
        let fact = TypedSemanticFact {
            id: "fact:model".into(),
            evidence_kind: EvidenceKind::ModelOutput,
            status: EpistemicStatus::Observed,
            record: SemanticRecord::Effect(EffectFact {
                function: SymbolId::new("symbol:f"),
                effect_kind: "candidate".into(),
                target: None,
            }),
            provenance: Provenance {
                source_path: "model://candidate".into(),
                source_revision: None,
                extractor: "model:test".into(),
                content_hash: None,
                span: None,
            },
        };

        assert!(!fact.is_epistemically_valid());
    }
}
