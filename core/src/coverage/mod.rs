//! Measured support levels (G155, ADR 0071, `contracts/UNIVERSAL-ENGINEERING-WORLD.md`).
//!
//! Atlas says how far it understands each language and each artifact class, and it says so from
//! evidence, never from a declaration. A level is the highest rung whose evidence exists, and
//! every rung below it must hold too:
//!
//! - L0 DISCOVERED: the inventory holds an artifact of the subject.
//! - L1 IDENTIFIED: a registered source frontend names its language.
//! - L2 SYNTAX: the artifact was admitted for parsing (disposition PARSED).
//! - L3 TYPED_RECORDS: typed semantic records came from it.
//! - L4 OBLIGATIONS_EVALUATED: an obligation over it is OBSERVED or DERIVED, not UNKNOWN or
//!   UNSUPPORTED.
//! - L5 COMPOSED: the world model composes functions from it.
//! - L6 AGENT_USABLE: a composed function of it has a DERIVED relation, which is what the
//!   agent's impact, trace and why lenses follow.
//!
//! A language parsed but without typed records is at L2, and L2 is not semantic support
//! (`is_semantic`). An artifact class Atlas has no adapter for (an image, a video, a CAD or
//! electronics file) is at L0: its class is read from the file extension only. A claim above
//! the measured level is refused (`validate_claims`).

use crate::census::{ArtifactDisposition, InventoryReport};
use crate::composition::WorldModel;
use crate::schema::{CensusReport, EpistemicStatus};
use crate::vocabulary_enum;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SUPPORT_REPORT_SCHEMA: &str = "atlas.support-levels.v1";

vocabulary_enum! {
    /// How far Atlas understands a subject, by the evidence it holds (lowest first).
    pub enum SupportLevel {
        L0Discovered => "L0_DISCOVERED",
        L1Identified => "L1_IDENTIFIED",
        L2Syntax => "L2_SYNTAX",
        L3TypedRecords => "L3_TYPED_RECORDS",
        L4ObligationsEvaluated => "L4_OBLIGATIONS_EVALUATED",
        L5Composed => "L5_COMPOSED",
        L6AgentUsable => "L6_AGENT_USABLE",
    }
}

impl SupportLevel {
    /// Semantic support starts at typed records: discovery and syntax are not understanding.
    pub fn is_semantic(self) -> bool {
        self >= Self::L3TypedRecords
    }
}

vocabulary_enum! {
    /// What a measured subject is.
    pub enum SubjectKind {
        /// A source language a registered frontend identifies.
        Language => "LANGUAGE",
        /// A class of artifacts no frontend identifies, named by file extension only.
        ArtifactClass => "ARTIFACT_CLASS",
    }
}

/// The evidence behind one subject's level, rung by rung.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LevelEvidence {
    pub artifacts: usize,
    pub parsed_artifacts: usize,
    pub typed_records: usize,
    /// Per dimension, the strongest obligation status over the subject's artifacts.
    pub dimensions: BTreeMap<String, EpistemicStatus>,
    pub composed_functions: usize,
    pub functions_with_derived_relations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubjectSupport {
    pub subject: String,
    pub kind: SubjectKind,
    pub level: SupportLevel,
    /// Why the level is not higher, in words; empty at L6.
    pub next_rung_missing: String,
    pub evidence: LevelEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportReport {
    pub schema: String,
    pub revision: String,
    pub subjects: Vec<SubjectSupport>,
}

impl SupportReport {
    pub fn level_of(&self, subject: &str) -> Option<SupportLevel> {
        self.subjects
            .iter()
            .find(|s| s.subject == subject)
            .map(|s| s.level)
    }
}

/// Artifact classes read from the extension alone. The list is open: an extension outside it
/// is its own class.
pub fn artifact_class(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    let extension = name
        .rsplit_once('.')
        .map_or(String::new(), |(_, e)| e.to_ascii_lowercase());
    let class = match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "heic" | "heif" | "bmp" | "ico"
        | "tif" | "tiff" => "image",
        "mp4" | "mov" | "webm" | "mkv" | "avi" => "video",
        "wav" | "mp3" | "ogg" | "flac" | "aac" | "m4a" => "audio",
        "pdf" | "docx" | "doc" | "odt" | "pptx" | "xlsx" => "document",
        "step" | "stp" | "iges" | "igs" | "stl" | "obj" | "gltf" | "glb" | "fbx" | "3mf" => {
            "cad_3d"
        }
        "kicad_pcb" | "kicad_sch" | "kicad_pro" | "brd" | "sch" | "gbr" => "electronics",
        "fig" | "sketch" | "psd" | "xd" => "ui_design",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => "cpp",
        "py" | "pyi" => "python",
        "go" => "go",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "java" => "java",
        "zig" | "zon" => "zig",
        "" => return "(no extension)".into(),
        other => return format!(".{other}"),
    };
    class.into()
}

fn rank(status: EpistemicStatus) -> u8 {
    match status {
        EpistemicStatus::Observed => 6,
        EpistemicStatus::Derived => 5,
        EpistemicStatus::Inferred => 4,
        EpistemicStatus::Declared => 3,
        EpistemicStatus::Unknown => 2,
        EpistemicStatus::Unsupported => 1,
        _ => 0,
    }
}

/// Measures every subject from the inventory, the census and the composed world model of one
/// revision.
pub fn measure(
    inventory: &InventoryReport,
    census: &CensusReport,
    model: &WorldModel,
) -> SupportReport {
    // Which subject each artifact belongs to.
    let mut subject_of: BTreeMap<&str, (String, SubjectKind)> = BTreeMap::new();
    let mut by_id: BTreeMap<&str, &str> = BTreeMap::new();
    let mut evidence: BTreeMap<(String, SubjectKind), LevelEvidence> = BTreeMap::new();
    for artifact in &inventory.artifacts {
        let key = match &artifact.language {
            Some(language) => (language.clone(), SubjectKind::Language),
            None => (artifact_class(&artifact.path), SubjectKind::ArtifactClass),
        };
        let entry = evidence.entry(key.clone()).or_default();
        entry.artifacts += 1;
        if artifact.language.is_some() && artifact.disposition == ArtifactDisposition::Parsed {
            entry.parsed_artifacts += 1;
        }
        by_id.insert(artifact.id.as_str(), artifact.path.as_str());
        subject_of.insert(artifact.path.as_str(), key);
    }
    for record in &census.typed_semantic_records {
        if let Some(key) = subject_of.get(record.provenance().source_path.as_str())
            && let Some(entry) = evidence.get_mut(key)
        {
            entry.typed_records += 1;
        }
    }
    for obligation in &census.typed_obligations {
        let Some(path) = by_id.get(obligation.artifact.as_str()) else {
            continue;
        };
        if let Some(key) = subject_of.get(path)
            && let Some(entry) = evidence.get_mut(key)
        {
            let dimension = obligation.dimension.as_str().to_owned();
            let best = entry
                .dimensions
                .entry(dimension)
                .or_insert(obligation.status);
            if rank(obligation.status) > rank(*best) {
                *best = obligation.status;
            }
        }
    }
    let derived: BTreeSet<&str> = model
        .relations
        .iter()
        .filter(|r| r.status == EpistemicStatus::Derived)
        .flat_map(|r| [r.from.as_str(), r.to.as_str()])
        .collect();
    for function in &model.functions {
        if let Some(key) = subject_of.get(function.path.as_str())
            && let Some(entry) = evidence.get_mut(key)
        {
            entry.composed_functions += 1;
            if derived.contains(function.id.as_str()) {
                entry.functions_with_derived_relations += 1;
            }
        }
    }
    let mut subjects: Vec<SubjectSupport> = evidence
        .into_iter()
        .map(|((subject, kind), evidence)| {
            let (level, next_rung_missing) = level_of(kind, &evidence);
            SubjectSupport {
                subject,
                kind,
                level,
                next_rung_missing,
                evidence,
            }
        })
        .collect();
    subjects.sort_by(|a, b| b.level.cmp(&a.level).then(a.subject.cmp(&b.subject)));
    SupportReport {
        schema: SUPPORT_REPORT_SCHEMA.into(),
        revision: model.revision.clone(),
        subjects,
    }
}

fn level_of(kind: SubjectKind, e: &LevelEvidence) -> (SupportLevel, String) {
    let evaluated = e
        .dimensions
        .values()
        .any(|s| matches!(s, EpistemicStatus::Observed | EpistemicStatus::Derived));
    let rungs: [(SupportLevel, bool, &str); 6] = [
        (
            SupportLevel::L1Identified,
            kind == SubjectKind::Language,
            "no registered source frontend identifies it (its class is read from the extension)",
        ),
        (
            SupportLevel::L2Syntax,
            e.parsed_artifacts > 0,
            "no artifact of it is admitted for parsing",
        ),
        (
            SupportLevel::L3TypedRecords,
            e.typed_records > 0,
            "no typed semantic record comes from it (syntax is not semantics)",
        ),
        (
            SupportLevel::L4ObligationsEvaluated,
            evaluated,
            "no obligation over it is OBSERVED or DERIVED",
        ),
        (
            SupportLevel::L5Composed,
            e.composed_functions > 0,
            "the world model composes no function from it",
        ),
        (
            SupportLevel::L6AgentUsable,
            e.functions_with_derived_relations > 0,
            "no composed function of it has a DERIVED relation the agent lenses can follow",
        ),
    ];
    let mut level = SupportLevel::L0Discovered;
    for (rung, holds, missing) in rungs {
        if !holds {
            return (level, missing.into());
        }
        level = rung;
    }
    (level, String::new())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClaimViolation {
    pub subject: String,
    pub claimed: SupportLevel,
    pub measured: Option<SupportLevel>,
}

/// Every claim above what `report` measured; a subject the report never measured supports
/// nothing, so any claim for it is refused.
pub fn validate_claims(
    report: &SupportReport,
    claims: &[(String, SupportLevel)],
) -> Vec<ClaimViolation> {
    let mut out: Vec<ClaimViolation> = claims
        .iter()
        .filter_map(|(subject, claimed)| {
            let measured = report.level_of(subject);
            (measured.is_none_or(|m| *claimed > m)).then(|| ClaimViolation {
                subject: subject.clone(),
                claimed: *claimed,
                measured,
            })
        })
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(
        parsed: usize,
        records: usize,
        evaluated: bool,
        composed: usize,
        derived: usize,
    ) -> LevelEvidence {
        let mut dimensions = BTreeMap::new();
        dimensions.insert(
            "SYMBOL".to_string(),
            if evaluated {
                EpistemicStatus::Observed
            } else {
                EpistemicStatus::Unsupported
            },
        );
        LevelEvidence {
            artifacts: 1,
            parsed_artifacts: parsed,
            typed_records: records,
            dimensions,
            composed_functions: composed,
            functions_with_derived_relations: derived,
        }
    }

    #[test]
    fn a_level_needs_every_rung_below_it() {
        let language = SubjectKind::Language;
        assert_eq!(
            level_of(language, &evidence(1, 1, true, 1, 1)).0,
            SupportLevel::L6AgentUsable
        );
        // Composed functions without evaluated obligations stop at L3.
        assert_eq!(
            level_of(language, &evidence(1, 5, false, 3, 3)).0,
            SupportLevel::L3TypedRecords
        );
        // Records from an artifact never admitted for parsing do not lift it past L1.
        assert_eq!(
            level_of(language, &evidence(0, 5, true, 3, 3)).0,
            SupportLevel::L1Identified
        );
        // Syntax only.
        let (level, missing) = level_of(language, &evidence(1, 0, false, 0, 0));
        assert_eq!(level, SupportLevel::L2Syntax);
        assert!(!level.is_semantic() && missing.contains("syntax is not semantics"));
        // An artifact class is never identified by a frontend, whatever else is claimed for it.
        assert_eq!(
            level_of(SubjectKind::ArtifactClass, &evidence(1, 1, true, 1, 1)).0,
            SupportLevel::L0Discovered
        );
    }

    #[test]
    fn artifact_classes_come_from_the_extension_and_stay_open() {
        assert_eq!(artifact_class("assets/hero.PNG"), "image");
        assert_eq!(artifact_class("cad/arm.step"), "cad_3d");
        assert_eq!(artifact_class("hw/board.kicad_pcb"), "electronics");
        assert_eq!(artifact_class("lib/src/parser.c"), "c");
        assert_eq!(artifact_class("Makefile"), "(no extension)");
        assert_eq!(artifact_class("data/x.parquet"), ".parquet");
        assert!(SupportLevel::L3TypedRecords.is_semantic());
        assert!(!SupportLevel::L2Syntax.is_semantic());
    }
}
