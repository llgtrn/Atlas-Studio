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

const SCHEMA_COMPATIBILITY_TESTS: &[&str] = &[
    "codara::artifact::census_container::tests::the_current_schema_is_the_latest_recorded_generation",
    "codara::artifact::census_container::tests::every_recorded_schema_conforms_to_the_current_one",
    "codara::artifact::census_container::tests::a_container_an_older_conforming_writer_produced_is_read_with_its_history",
];

fn schema_compatibility_args() -> Vec<&'static str> {
    let mut args = vec!["test", "-p", "core", "--lib", "--offline", "--"];
    args.extend_from_slice(SCHEMA_COMPATIBILITY_TESTS);
    args.extend_from_slice(&["--exact", "--format=pretty", "--color=never"]);
    args
}

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
    command_evidence_with_tests(obligation, candidate, root, program, args, &[])
}

// Filtered Cargo success can mean zero tests. For compatibility, require exactly the selected
// named tests and one completed libtest summary; unknown output formats stay unresolved.
fn completed_test_run(stdout: &[u8], stderr: &[u8], required_tests: &[&str]) -> bool {
    let (Ok(stdout), Ok(stderr)) = (std::str::from_utf8(stdout), std::str::from_utf8(stderr))
    else {
        return false;
    };
    if stderr
        .lines()
        .any(|line| line.starts_with("test ") || line.starts_with("running "))
    {
        return false;
    }
    let mut lines = stdout.lines().filter(|line| !line.is_empty());
    if lines.next() != Some(format!("running {} tests", required_tests.len()).as_str()) {
        return false;
    }
    let mut seen = vec![false; required_tests.len()];
    for _ in required_tests {
        let Some(line) = lines.next() else {
            return false;
        };
        let Some(index) = required_tests
            .iter()
            .position(|test| line == format!("test {test} ... ok"))
        else {
            return false;
        };
        if std::mem::replace(&mut seen[index], true) {
            return false;
        }
    }
    let summary_prefix = format!(
        "test result: ok. {} passed; 0 failed; 0 ignored; 0 measured; ",
        required_tests.len()
    );
    let Some(summary) = lines
        .next()
        .and_then(|line| line.strip_prefix(&summary_prefix))
    else {
        return false;
    };
    let Some((filtered, duration)) = summary.split_once(" filtered out; finished in ") else {
        return false;
    };
    filtered.parse::<usize>().is_ok()
        && duration
            .strip_suffix('s')
            .and_then(|value| value.parse::<f64>().ok())
            .is_some_and(|value| value.is_finite() && value >= 0.0)
        && lines.next().is_none()
}

fn command_evidence_with_tests(
    obligation: &str,
    candidate: &str,
    root: &Path,
    program: &str,
    args: &[&str],
    required_tests: &[&str],
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
    let (result, counterexample) = if !output.status.success() {
        (EvidenceResult::Violated, Some(tail))
    } else if !required_tests.is_empty()
        && !completed_test_run(&output.stdout, &output.stderr, required_tests)
    {
        (
            EvidenceResult::Unresolved,
            Some(format!(
                "required tests did not complete exactly once with a successful summary: {}\n{tail}",
                required_tests.join(", ")
            )),
        )
    } else {
        (EvidenceResult::Satisfied, None)
    };
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
        result,
        content_hash: digest(&transcript),
        counterexample,
        status: atlas_core::EpistemicStatus::Observed,
    })
}

/// Run only the self scope's current-schema, conformance and older-writer history checks.
/// This observed evidence binds to the supplied candidate; it does not admit or certify it.
pub fn schema_compatibility_evidence(
    root: &Path,
    candidate: &str,
) -> io::Result<VerificationEvidence> {
    command_evidence_with_tests(
        OBLIGATION_COMPATIBILITY,
        candidate,
        root,
        "cargo",
        &schema_compatibility_args(),
        SCHEMA_COMPATIBILITY_TESTS,
    )
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
        schema_compatibility_evidence(root, &candidate)?,
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

    fn successful_compatibility_output() -> String {
        format!(
            "\nrunning 3 tests\n{}\ntest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 533 filtered out; finished in 0.01s\n\n",
            SCHEMA_COMPATIBILITY_TESTS
                .iter()
                .map(|test| format!("test {test} ... ok\n"))
                .collect::<String>()
        )
    }

    #[test]
    fn compatibility_requires_an_exact_completed_run_of_all_three_schema_history_tests() {
        let passed = successful_compatibility_output();
        let first = format!("test {} ... ok\n", SCHEMA_COMPATIBILITY_TESTS[0]);
        let second = format!("test {} ... ok\n", SCHEMA_COMPATIBILITY_TESTS[1]);
        let summary = passed
            .lines()
            .find(|line| line.starts_with("test result:"))
            .unwrap();
        let cases = [
            ("empty", String::new()),
            ("zero tests", "running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 536 filtered out; finished in 0.00s\n".into()),
            ("quiet summary", format!("{summary}\n")),
            ("wrong subset", passed.replace(SCHEMA_COMPATIBILITY_TESTS[0], "other::test")),
            ("lookalike name", passed.replace("test codara::", "test other::codara::")),
            ("retired namespace", passed.replace("codara::artifact::census_container::", "atlas::")),
            ("missing named test", passed.replace(&first, "")),
            ("duplicate named test", passed.replace(&second, &first)),
            ("ignored test", passed.replacen(" ... ok", " ... ignored", 1)),
            ("failed test", passed.replacen(" ... ok", " ... FAILED", 1)),
            ("missing summary", passed.replace(summary, "")),
            ("duplicate summary", format!("{passed}{summary}\n")),
            ("contradictory status", format!("{passed}test {} ... FAILED\n", SCHEMA_COMPATIBILITY_TESTS[0])),
            ("extra passing test", format!("{passed}test other::test ... ok\n")),
            ("ignored summary", passed.replace("0 ignored", "1 ignored")),
            ("failure summary", passed.replace("0 failed", "1 failed")),
            ("wrong passed count", passed.replace("3 passed", "2 passed")),
            ("wrong running count", passed.replace("running 3 tests", "running 4 tests")),
            ("invalid duration", passed.replace("0.01s", "NaNs")),
            ("invalid filtered count", passed.replace("533 filtered", "? filtered")),
            ("list only", SCHEMA_COMPATIBILITY_TESTS.iter().map(|test| format!("{test}: test\n")).collect()),
        ];
        assert!(completed_test_run(
            passed.as_bytes(),
            b"Compiling core\n",
            SCHEMA_COMPATIBILITY_TESTS
        ));
        for (case, stdout) in cases {
            let evidence = command_evidence_with_tests(
                OBLIGATION_COMPATIBILITY,
                "candidate",
                &std::env::temp_dir(),
                "sh",
                &["-c", "printf '%s' \"$1\"", "compatibility-fixture", &stdout],
                SCHEMA_COMPATIBILITY_TESTS,
            )
            .unwrap();
            assert_eq!(evidence.result, EvidenceResult::Unresolved, "{case}");
            assert_eq!(evidence.content_hash, digest(stdout.as_bytes()), "{case}");
            assert_eq!(evidence.candidate, "candidate", "{case}");
            assert_eq!(evidence.obligations, [OBLIGATION_COMPATIBILITY], "{case}");
            assert!(
                evidence
                    .counterexample
                    .as_deref()
                    .unwrap()
                    .contains("required tests"),
                "{case}"
            );
            assert_eq!(
                evaluate(&self_scope_plan("candidate"), &[evidence]).outcomes[2].state,
                ObligationState::Unresolved,
                "{case}"
            );
        }
        assert!(!completed_test_run(
            passed.as_bytes(),
            b"test result: FAILED\n",
            SCHEMA_COMPATIBILITY_TESTS
        ));
        assert!(!completed_test_run(
            b"\xff",
            b"",
            SCHEMA_COMPATIBILITY_TESTS
        ));
        assert!(!completed_test_run(
            passed.as_bytes(),
            b"\xff",
            SCHEMA_COMPATIBILITY_TESTS
        ));
    }

    #[test]
    fn compatibility_process_result_and_transcript_stay_bound_to_the_observation() {
        let passed = successful_compatibility_output();
        for (case, stdout, stderr, exit, expected) in [
            (
                "executed",
                passed.as_str(),
                "cargo diagnostics\n",
                "0",
                EvidenceResult::Satisfied,
            ),
            (
                "stderr only",
                "",
                passed.as_str(),
                "0",
                EvidenceResult::Unresolved,
            ),
            (
                "conflicting stderr",
                passed.as_str(),
                "test other::test ... ignored\n",
                "0",
                EvidenceResult::Unresolved,
            ),
            (
                "nonzero exit",
                passed.as_str(),
                "build failed",
                "101",
                EvidenceResult::Violated,
            ),
        ] {
            let evidence = command_evidence_with_tests(
                OBLIGATION_COMPATIBILITY,
                "candidate",
                &std::env::temp_dir(),
                "sh",
                &[
                    "-c",
                    "printf '%s' \"$1\"; printf '%s' \"$2\" >&2; exit \"$3\"",
                    "compatibility-fixture",
                    stdout,
                    stderr,
                    exit,
                ],
                SCHEMA_COMPATIBILITY_TESTS,
            )
            .unwrap();
            assert_eq!(evidence.result, expected, "{case}");
            assert_eq!(
                evidence.content_hash,
                digest(format!("{stdout}{stderr}").as_bytes()),
                "{case}"
            );
            assert_eq!(
                evidence.counterexample.is_none(),
                expected == EvidenceResult::Satisfied,
                "{case}"
            );
        }
        let signalled = command_evidence_with_tests(
            OBLIGATION_COMPATIBILITY,
            "candidate",
            &std::env::temp_dir(),
            "sh",
            &["-c", "kill -TERM $$"],
            SCHEMA_COMPATIBILITY_TESTS,
        )
        .unwrap();
        assert_eq!(signalled.result, EvidenceResult::Violated);
        let not_run = command_evidence_with_tests(
            OBLIGATION_COMPATIBILITY,
            "candidate",
            &std::env::temp_dir(),
            "atlas-missing-compatibility-producer-71ed311",
            &[],
            SCHEMA_COMPATIBILITY_TESTS,
        )
        .unwrap_err();
        assert_eq!(not_run.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn compatibility_selects_only_the_three_exact_schema_history_tests_with_named_output() {
        let args = schema_compatibility_args();
        assert_eq!(
            &args[..6],
            ["test", "-p", "core", "--lib", "--offline", "--"]
        );
        assert_eq!(&args[6..9], SCHEMA_COMPATIBILITY_TESTS);
        assert_eq!(&args[9..], ["--exact", "--format=pretty", "--color=never"]);
    }
}
