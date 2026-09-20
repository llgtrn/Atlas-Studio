use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TranslationKind {
    CodeLanguage,
    DesignLanguage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationPlan {
    pub schema: String,
    pub kind: TranslationKind,
    pub source_language: String,
    pub target_language: String,
    pub candidate_only: bool,
    pub graph_first: bool,
    pub proof_required: bool,
}

pub fn code(source: impl Into<String>, target: impl Into<String>) -> TranslationPlan {
    TranslationPlan {
        schema: "atlas.systemizer.translation-plan.v1".into(),
        kind: TranslationKind::CodeLanguage,
        source_language: source.into(),
        target_language: target.into(),
        candidate_only: true,
        graph_first: true,
        proof_required: true,
    }
}

pub fn design(source: impl Into<String>, target: impl Into<String>) -> TranslationPlan {
    TranslationPlan {
        schema: "atlas.systemizer.translation-plan.v1".into(),
        kind: TranslationKind::DesignLanguage,
        source_language: source.into(),
        target_language: target.into(),
        candidate_only: true,
        graph_first: true,
        proof_required: true,
    }
}
