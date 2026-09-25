//! Verification producers for the self scope (G127, NA-VERIFICATION-EVIDENCE, ADR 0048).
//!
//! The self scope is a library package: its plan names one SEMANTIC, one UNIT and one
//! COMPATIBILITY obligation. Evidence binds to the candidate's census digest -- the content
//! identity of exactly the tree under test -- so evidence produced for any other tree is never
//! admissible. Producers are the generation's own self-recensus report and command runs; a command
//! run records that it ran with ambient authority (not in the sandbox, `DEBT-SANDBOXED_EXECUTION`).

use atlas_core::IntegrityDigest;
use atlas_core::verification::{
    EvidenceResult, Producer, VerificationClass, VerificationEvidence, VerificationObligation,
    VerificationPlan, VerificationPolicy, VerificationReport, evaluate,
};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

pub const OBLIGATION_SEMANTIC: &str = "OBL-SELF-SEMANTIC-RECENSUS";
pub const OBLIGATION_UNIT: &str = "OBL-SELF-UNIT-WORKSPACE";
pub const OBLIGATION_COMPATIBILITY: &str = "OBL-SELF-COMPATIBILITY-SCHEMAS";

fn digest(bytes: &[u8]) -> String {
    IntegrityDigest::of_bytes(bytes).as_str().to_owned()
}

/// The self scope's plan for one candidate (a census digest).
pub fn self_scope_plan(candidate: &str) -> VerificationPlan {
    let obligation = |id: &str, class, statement: &str| VerificationObligation {
        id: id.into(),
        class,
        subject: "self scope".into(),
        statement: statement.into(),
        required: true,
    };
    VerificationPlan {
        policy: VerificationPolicy::library_package(),
        candidate: candidate.into(),
        obligations: vec![
            obligation(
                OBLIGATION_SEMANTIC,
                VerificationClass::Semantic,
                "the candidate's self-recensus is PROVEN against its predecessor",
            ),
            obligation(
                OBLIGATION_UNIT,
                VerificationClass::Unit,
                "every workspace unit test passes on the candidate",
            ),
            obligation(
                OBLIGATION_COMPATIBILITY,
                VerificationClass::Compatibility,
                "every recorded .atlas schema generation conforms and an older container is read \
                 with its history",
            ),
        ],
    }
}

/// SEMANTIC evidence from a self-recensus report: SATISFIED when PROVEN, VIOLATED with its
/// regressions otherwise, bound to the post-change census digest the report proved.
pub fn recensus_evidence(report_path: &Path) -> io::Result<VerificationEvidence> {
    let text = std::fs::read_to_string(report_path)?;
    let report: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let verdict = report["verdict"].as_str().unwrap_or_default();
    let candidate = report["after_census_digest"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let proven = verdict == "PROVEN";
    Ok(VerificationEvidence {
        id: format!(
            "evidence:recensus:{}",
            report["generation"].as_str().unwrap_or("?")
        ),
        obligations: vec![OBLIGATION_SEMANTIC.into()],
        producer: Producer {
            tool: "atlas-systemizer recensus prove".into(),
            version: report["schema"].as_str().unwrap_or_default().into(),
        },
        run_id: report["generation"].as_str().unwrap_or_default().into(),
        candidate,
        environment: format!("self-recensus report {}", report_path.display()),
        inputs: vec![
            report["before_census_digest"]
                .as_str()
                .unwrap_or_default()
                .into(),
        ],
        result: if proven {
            EvidenceResult::Satisfied
        } else {
            EvidenceResult::Violated
        },
        content_hash: digest(text.as_bytes()),
        counterexample: (!proven).then(|| {
            format!(
                "verdict {verdict}; regressions {}; unexpected {}",
                report["regressions"], report["unexpected_changes"]
            )
        }),
        status: atlas_core::EpistemicStatus::Observed,
    })
}

/// Evidence from running `program args` in `root` for `obligation` on `candidate`: SATISFIED on a
/// zero exit, VIOLATED otherwise with the tail of its output as the counterexample.
pub fn command_evidence(
    obligation: &str,
    candidate: &str,
    root: &Path,
    program: &str,
    args: &[&str],
) -> io::Result<VerificationEvidence> {
    let version = Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()?;
    let mut transcript = output.stdout.clone();
    transcript.extend_from_slice(&output.stderr);
    let run = format!("{program} {}", args.join(" "));
    let tail: String = String::from_utf8_lossy(&transcript)
        .lines()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Ok(VerificationEvidence {
        id: format!("evidence:{obligation}:{}", &digest(run.as_bytes())[11..27]),
        obligations: vec![obligation.into()],
        producer: Producer {
            tool: program.into(),
            version,
        },
        run_id: run,
        candidate: candidate.into(),
        environment: format!(
            "ambient host process in {} (not sandboxed: DEBT-SANDBOXED_EXECUTION)",
            root.display()
        ),
        inputs: vec![candidate.into()],
        result: if output.status.success() {
            EvidenceResult::Satisfied
        } else {
            EvidenceResult::Violated
        },
        content_hash: digest(&transcript),
        counterexample: (!output.status.success()).then_some(tail),
        status: atlas_core::EpistemicStatus::Observed,
    })
}

/// Evaluate the self scope: the candidate is the census digest of `root` as it is now.
pub fn verify_self(
    root: &Path,
    recensus_report: &Path,
) -> io::Result<(VerificationReport, Vec<VerificationEvidence>)> {
    let candidate = crate::recensus::snapshot(root)?.census_digest;
    let evidence = vec![
        recensus_evidence(recensus_report)?,
        command_evidence(
            OBLIGATION_UNIT,
            &candidate,
            root,
            "cargo",
            &["test", "-q", "--workspace", "--offline"],
        )?,
        command_evidence(
            OBLIGATION_COMPATIBILITY,
            &candidate,
            root,
            "cargo",
            &["test", "-q", "-p", "core", "--offline", "atlas::"],
        )?,
    ];
    Ok((evaluate(&self_scope_plan(&candidate), &evidence), evidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::verification::ObligationState;

    fn report(dir: &Path, verdict: &str, after: &str) -> std::path::PathBuf {
        let path = dir.join(format!("recensus-{verdict}-{}.json", &after[..4]));
        std::fs::write(
            &path,
            serde_json::json!({
                "schema": "atlas.self-recensus-report.v1",
                "generation": "G999",
                "verdict": verdict,
                "before_census_digest": "blake3-256:before",
                "after_census_digest": after,
                "regressions": ["a regression"],
                "unexpected_changes": [],
            })
            .to_string(),
        )
        .unwrap();
        path
    }

    #[test]
    fn semantic_evidence_counts_only_for_the_tree_it_proved() {
        let dir = std::env::temp_dir().join(format!("atlas-verification-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plan = self_scope_plan("blake3-256:cand");
        let state =
            |evidence: Vec<VerificationEvidence>| evaluate(&plan, &evidence).outcomes[0].state;
        let proven = recensus_evidence(&report(&dir, "PROVEN", "blake3-256:cand")).unwrap();
        assert_eq!(state(vec![proven]), ObligationState::Satisfied);
        let elsewhere = recensus_evidence(&report(&dir, "PROVEN", "blake3-256:other")).unwrap();
        assert_eq!(
            state(vec![elsewhere]),
            ObligationState::Unverified,
            "another tree"
        );
        let failed =
            recensus_evidence(&report(&dir, "GENERATION_NOT_PROVEN", "blake3-256:cand")).unwrap();
        let report = evaluate(&plan, &[failed]);
        assert_eq!(report.outcomes[0].state, ObligationState::Failed);
        assert!(
            report.failures[0]
                .counterexample
                .as_deref()
                .unwrap()
                .contains("a regression")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn command_evidence_records_its_result_and_that_it_was_not_sandboxed() {
        let root = std::env::temp_dir();
        let ok = command_evidence(OBLIGATION_UNIT, "c", &root, "sh", &["-c", "exit 0"]).unwrap();
        assert_eq!(ok.result, EvidenceResult::Satisfied);
        assert!(ok.environment.contains("not sandboxed"));
        let bad = command_evidence(
            OBLIGATION_UNIT,
            "c",
            &root,
            "sh",
            &["-c", "echo boom; exit 3"],
        )
        .unwrap();
        assert_eq!(bad.result, EvidenceResult::Violated);
        assert_eq!(bad.counterexample.as_deref(), Some("boom"));
        assert_ne!(ok.content_hash, bad.content_hash);
    }
}
