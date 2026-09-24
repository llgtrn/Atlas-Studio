//! Physical Engineering orchestration (ADR 0012): compile a root's declared ADL and analyze its
//! physical entities. Digital only -- no simulation, no hardware.

use atlas_core::{
    AdlCompileReport, SourceReport, compile_adl,
    physical::{PhysicalReport, analyze_physical},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhysicalAnalysis {
    pub physical: PhysicalReport,
    /// The ADL compile the model was assembled from, including its constraint verdicts.
    pub adl: AdlCompileReport,
}

pub fn analyze_root(root: impl AsRef<Path>) -> io::Result<PhysicalAnalysis> {
    let sources = adapter::read_adl_sources(root)?;
    let observed = SourceReport {
        schema: "atlas.physical-no-source-observation".into(),
        root: String::new(),
        files_total: 0,
        languages: BTreeMap::new(),
        files: Vec::new(),
    };
    let adl = compile_adl(&sources, &observed);
    let physical = analyze_physical(&adl.ir.declared.nodes);
    Ok(PhysicalAnalysis { physical, adl })
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{ConstraintVerdict, quantity::Quantity};

    #[test]
    fn the_declared_two_link_arm_is_modelled_and_its_requirements_decided() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/physical/two-link-arm");
        let analysis = analyze_root(&root).unwrap();
        let report = &analysis.physical;
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.arms.len(), 1);
        assert_eq!(report.evidence_level, "SEMANTIC_MODEL");
        let reach = report.derived.iter().find(|d| d.name == "reach").unwrap();
        assert_eq!(reach.value, Quantity::parse("0.55 m").unwrap());
        assert!(
            report
                .requirements
                .iter()
                .all(|check| check.verdict == ConstraintVerdict::Satisfied),
            "{:?}",
            report.requirements
        );
        let links_light = analysis
            .adl
            .constraint_results
            .iter()
            .find(|result| result.name == "LinksAreLight")
            .unwrap();
        assert_eq!(links_light.verdict, ConstraintVerdict::Satisfied);
    }
}
