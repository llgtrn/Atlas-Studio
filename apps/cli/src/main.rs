use std::{env, fs, path::PathBuf, process::ExitCode};

/// `Ok(None)` when `name` is not present at all; `Ok(Some(v))` when present with a following
/// value; `Err` when `name` is present but has no following argument (e.g. it is the last token,
/// or immediately followed by another flag) -- a real shell-scripting failure mode (an unset/empty
/// variable dropped by word-splitting: `--base-sha "$BASE_SHA"` with `$BASE_SHA` unset becomes
/// `--base-sha` with nothing after it), never conflated with the flag being absent on purpose.
/// Before this fix, a genuinely optional flag like `--base-sha` silently fell back to `None` in
/// either case, defeating `EXACT_BASE_SHA_REQUIRED` on malformed input instead of failing loud.
fn value(args: &[String], name: &str) -> Result<Option<String>, String> {
    match args.iter().position(|x| x == name) {
        None => Ok(None),
        Some(i) => match args.get(i + 1) {
            Some(v) => Ok(Some(v.clone())),
            None => Err(format!("{name} requires a value")),
        },
    }
}

fn json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

/// Writes `text` to `out`, creating its parent directory first. Shared by every subcommand that
/// takes `--out` so the path-context fix below applies uniformly instead of needing to be
/// remembered at each of the four call sites separately -- the exact failure mode this closes
/// (`io::Error::to_string()` naming no path) is the same class already fixed for `--root` in
/// every subcommand's `runtime::*` call, just for the output path instead of the input one.
fn write_report_to_out(out: &str, text: &str) -> Result<(), String> {
    let out_path = PathBuf::from(out);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{out}: {e}"))?;
    }
    fs::write(&out_path, text).map_err(|e| format!("{out}: {e}"))
}

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [cmd, rest @ ..] if cmd == "contract" => {
            if value(rest, "--format")?.as_deref() != Some("json") {
                return Err("contract requires --format json".into());
            }
            println!("{}", json(&runtime::contract())?);
        }
        [cmd, sub, rest @ ..] if cmd == "docs" && sub == "audit" => {
            let root = value(rest, "--root")?.ok_or("docs audit requires --root")?;
            let report = runtime::docs_audit(&root).map_err(|e| format!("{root}: {e}"))?;
            println!("{}", json(&report)?);
            if !report.gate_ready {
                return Err("DOCS_GATE_NOT_READY".into());
            }
        }
        [cmd, sub, rest @ ..] if cmd == "code" && sub == "analyze" => {
            let root = value(rest, "--root")?.ok_or("code analyze requires --root")?;
            let report = runtime::code_analyze(&root).map_err(|e| format!("{root}: {e}"))?;
            println!("{}", json(&report)?);
        }
        [cmd, rest @ ..] if cmd == "parse" => {
            let root = value(rest, "--root")?.ok_or("parse requires --root")?;
            let programs = runtime::parse(&root).map_err(|e| format!("{root}: {e}"))?;
            println!(
                "{}",
                json(&serde_json::json!({
                    "schema": "atlas.adl.parse-report.v1",
                    "sources_total": programs.len(),
                    "programs": programs
                }))?
            );
        }
        [cmd, rest @ ..] if cmd == "check" => {
            let root = value(rest, "--root")?.ok_or("check requires --root")?;
            let report = runtime::check(&root).map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            if let Some(out) = value(rest, "--out")? {
                write_report_to_out(&out, &text)?;
            }
            print!("{text}");
            if !report.diagnostics.is_empty()
                || report
                    .constraint_results
                    .iter()
                    .any(|result| !result.passed)
            {
                return Err("ADL_CHECK_NOT_READY".into());
            }
        }
        [cmd, rest @ ..] if cmd == "graph" => {
            let root = value(rest, "--root")?.ok_or("graph requires --root")?;
            let report = runtime::graph(&root).map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            if let Some(out) = value(rest, "--out")? {
                write_report_to_out(&out, &text)?;
            }
            print!("{text}");
        }
        [cmd, rest @ ..] if cmd == "systemize" => {
            let root = value(rest, "--root")?.ok_or("systemize requires --root")?;
            let out = value(rest, "--out")?.ok_or("systemize requires --out")?;
            // `--previous <earlier systemize --out report>`: diff this run's inventory against
            // that report's `inventory` (ADR 0006) and emit `inventory_delta`.
            let previous = value(rest, "--previous")?
                .map(|path| {
                    runtime::read_previous_inventory(&path).map_err(|e| format!("{path}: {e}"))
                })
                .transpose()?;
            // `--cache <dir>`: reuse per-artifact semantic extraction across runs (ADR 0008).
            let mut cache = value(rest, "--cache")?
                .map(|dir| {
                    runtime::census::extraction::ExtractionCache::open(&dir)
                        .map_err(|e| format!("{dir}: {e}"))
                })
                .transpose()?;
            let report = runtime::systemize_with(&root, previous.as_ref(), cache.as_mut())
                .map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            write_report_to_out(&out, &text)?;
            print!("{text}");
            // Every sibling subcommand whose report carries a readiness gate enforces it here
            // (`docs audit` -> DOCS_GATE_NOT_READY, `check` -> ADL_CHECK_NOT_READY, `work prepare`
            // -> WORK_PREPARE_NOT_ALLOWED) so a blocked repository makes the process exit non-zero,
            // not just print a report a caller must remember to re-parse. `systemize` carries the
            // single most consequential gate of all -- `coding_admission.allowed`, derived from
            // every blocker this pipeline can raise (REPO_GATE_NOT_READY, DOCS_GATE_NOT_READY,
            // ADL_DIAGNOSTICS_PRESENT, ADL_CONSTRAINT_VIOLATED, and every *_ACCOUNTING_NOT_CLOSED/
            // DEPENDENCY_CLOSURE_NOT_CLOSED coverage gap) -- yet this was the one handler that
            // silently dropped the check its three siblings all have: `.atlas/repo.toml`'s own
            // `compile` command invokes exactly this subcommand, so any orchestration step gating
            // on this process's exit code (rather than re-parsing the JSON body itself) previously
            // treated a blocked repository as a successful compile.
            if !report.coding_admission.allowed {
                return Err("CODING_ADMISSION_NOT_ALLOWED".into());
            }
        }
        [cmd, sub, rest @ ..] if cmd == "work" && sub == "prepare" => {
            let root = value(rest, "--root")?.ok_or("work prepare requires --root")?;
            let goal = value(rest, "--goal")?.ok_or("work prepare requires --goal")?;
            let expected_base_sha = value(rest, "--base-sha")?;
            let report = runtime::prepare_work(&root, goal, expected_base_sha)
                .map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            if let Some(out) = value(rest, "--out")? {
                write_report_to_out(&out, &text)?;
            }
            print!("{text}");
            if !report.allowed {
                return Err("WORK_PREPARE_NOT_ALLOWED".into());
            }
        }
        _ => {
            return Err(
                "usage: atlas-systemizer <contract|systemize|docs audit|code analyze|parse|check|graph|work prepare> ..."
                    .into(),
            );
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("atlas-systemizer: {message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Falsification: `io::Error::to_string()` never includes the path that failed -- it is purely
    // the OS message ("No such file or directory (os error 2)"). Every subcommand handler passed
    // that bare string straight through via `.map_err(|e| e.to_string())`, so a mistyped or
    // nonexistent `--root` (arguably the single most common CLI usage mistake) produced a message
    // with no indication of which path was the problem, indistinguishable from any other io
    // failure anywhere in the pipeline. Confirmed against the unfixed code before writing the fix.
    #[test]
    fn a_nonexistent_root_error_names_the_offending_path() {
        let root = "/tmp/atlas-cli-tests-definitely-does-not-exist-xyz789";
        let out = std::env::temp_dir()
            .join("atlas-cli-tests-out.json")
            .to_string_lossy()
            .into_owned();
        let err = run(&[
            "systemize".to_owned(),
            "--root".to_owned(),
            root.to_owned(),
            "--out".to_owned(),
            out,
        ])
        .expect_err("a nonexistent root must not succeed");
        assert!(
            err.contains(root),
            "error message must name the offending root path, got: {err}"
        );
    }

    // Falsification: the same defect class as the `--root` fix above, in the output path instead
    // of the input one. `write_report_to_out`'s two `fs::create_dir_all`/`fs::write` calls used
    // `.map_err(|e| e.to_string())` (no path context) until this fix -- confirmed against the
    // unfixed code with a real repository root (this repo itself, via `check`, the cheapest
    // subcommand to run) and an `--out` path whose parent component is a real, existing FILE
    // (`fs::create_dir_all` cannot create a directory through a file: `ENOTDIR`), producing "Not a
    // directory (os error 20)" with no indication of which path was the problem.
    #[test]
    fn an_unwritable_out_path_error_names_the_offending_path() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let blocking_file = std::env::temp_dir().join(format!("atlas-cli-tests-blocker-{nonce}"));
        std::fs::write(&blocking_file, b"not a directory").unwrap();
        let out = blocking_file
            .join("nested")
            .join("out.json")
            .to_string_lossy()
            .into_owned();

        let err = run(&[
            "check".to_owned(),
            "--root".to_owned(),
            ".".to_owned(),
            "--out".to_owned(),
            out.clone(),
        ])
        .expect_err("an --out path blocked by an existing file must not succeed");
        assert!(
            err.contains(&out),
            "error message must name the offending --out path, got: {err}"
        );

        std::fs::remove_file(&blocking_file).unwrap();
    }

    // Falsification: `docs audit`, `check`, and `work prepare` each enforce their own report's
    // readiness field with a distinct error (`DOCS_GATE_NOT_READY`/`ADL_CHECK_NOT_READY`/
    // `WORK_PREPARE_NOT_ALLOWED`) so a blocked repository makes the process exit non-zero.
    // `systemize` -- the one subcommand `.atlas/repo.toml`'s own `compile` command invokes --
    // silently dropped this check: no branch in its handler could ever return `Err`, so a
    // repository with `coding_admission.allowed == false` (e.g. no admitted `.atlas/repo.toml` at
    // all, `REPO_GATE_NOT_READY`) still exited 0. Confirmed against the unfixed code before
    // writing this fix.
    #[test]
    fn a_repository_with_no_admitted_manifest_makes_systemize_exit_non_zero() {
        let dir = std::env::temp_dir().join(format!(
            "atlas-cli-tests-no-manifest-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // `runtime::systemize` requires a real git repository (it pins identity via
        // `git rev-parse HEAD`) -- this scratch repo has a commit but deliberately no
        // `.atlas/repo.toml` at all, the exact `REPO_GATE_NOT_READY` shape.
        let run_git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&dir)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?} failed");
        };
        run_git(&["init", "--quiet"]);
        run_git(&["config", "user.email", "test@example.com"]);
        run_git(&["config", "user.name", "test"]);
        std::fs::write(dir.join("README.md"), "no atlas manifest here\n").unwrap();
        run_git(&["add", "."]);
        run_git(&["commit", "--quiet", "-m", "init"]);
        let out_path = dir.join("out.json");
        let out = out_path.to_string_lossy().into_owned();

        let err = run(&[
            "systemize".to_owned(),
            "--root".to_owned(),
            dir.to_string_lossy().into_owned(),
            "--out".to_owned(),
            out,
        ])
        .expect_err("a repository with no admitted manifest must not exit successfully");
        assert_eq!(err, "CODING_ADMISSION_NOT_ALLOWED");

        // `runtime::systemize` computes 8 independent blocker conditions
        // (REPO_GATE_NOT_READY, DOCS_GATE_NOT_READY, ADL_DIAGNOSTICS_PRESENT,
        // ADL_CONSTRAINT_VIOLATED, INVENTORY/CENSUS/NORMALIZATION/SEMANTIC_EXTRACTION_
        // ACCOUNTING_NOT_CLOSED, DEPENDENCY_CLOSURE_NOT_CLOSED) and has zero direct unit test
        // coverage anywhere in the runtime crate -- this was the only test exercising it at all,
        // and its assertion above proves only that SOME blocker fired, which is trivially true
        // here regardless of whether the other 7 conditions are computed correctly (this bare
        // repository's missing `.atlas/repo.toml` alone guarantees `blockers` is non-empty). A
        // bug swapping two blocker labels, or wrongly raising/suppressing an unrelated condition,
        // would leave this test passing unchanged. Reading the report back and asserting the
        // EXACT blocker list this real, minimal fixture produces closes that gap.
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out_path).unwrap()).unwrap();
        assert_eq!(
            report["coding_admission"]["blockers"],
            serde_json::json!(["REPO_GATE_NOT_READY", "DOCS_GATE_NOT_READY"]),
            "exactly these two conditions -- no fewer, no more, no others -- must fire for a bare \
             git repository with no .atlas/ directory at all: {report:#}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn systemize_previous_reports_exactly_the_changed_artifacts_and_refuses_a_foreign_baseline() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("atlas-cli-tests-delta-{nonce}"));
        let dir = base.join("repo");
        std::fs::create_dir_all(&dir).unwrap();
        let run_git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&dir)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?} failed");
        };
        run_git(&["init", "--quiet"]);
        run_git(&["config", "user.email", "test@example.com"]);
        run_git(&["config", "user.name", "test"]);
        std::fs::write(dir.join("README.md"), "first\n").unwrap();
        std::fs::write(dir.join("stable.txt"), "unchanged\n").unwrap();
        std::fs::write(dir.join("doomed.txt"), "to be deleted\n").unwrap();
        run_git(&["add", "."]);
        run_git(&["commit", "--quiet", "-m", "init"]);
        let root = dir.to_string_lossy().into_owned();
        // Reports live outside the scanned root so they never appear in its inventory.
        let first = base.join("first.json").to_string_lossy().into_owned();
        let second = base.join("second.json").to_string_lossy().into_owned();
        let systemize = |extra: &[&str], out: &str| {
            let mut args = vec!["systemize", "--root", &root, "--out", out];
            args.extend_from_slice(extra);
            run(&args.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>())
        };
        // Not admitted (no .atlas/repo.toml), but the report is still written.
        let _ = systemize(&[], &first);
        let baseline: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&first).unwrap()).unwrap();
        assert!(
            baseline.get("inventory_delta").is_none(),
            "a first run has no baseline"
        );

        std::fs::write(dir.join("README.md"), "second\n").unwrap();
        std::fs::write(dir.join("stable.txt"), "unchanged\n").unwrap(); // rewritten, same bytes
        std::fs::remove_file(dir.join("doomed.txt")).unwrap();
        std::fs::write(dir.join("new.txt"), "hello\n").unwrap();
        let _ = systemize(&["--previous", &first], &second);
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&second).unwrap()).unwrap();
        let delta = &report["inventory_delta"];
        let changes: Vec<(String, String, String)> = delta["changes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["path"].as_str().unwrap().to_owned(),
                    c["change"].as_str().unwrap().to_owned(),
                    c["cause"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let expected = [
            ("README.md", "MODIFIED", "CONTENT_DIGEST_DIFFERS"),
            ("doomed.txt", "DELETED", "DISAPPEARED"),
            ("new.txt", "CREATED", "APPEARED"),
        ]
        .map(|(p, k, c)| (p.to_owned(), k.to_owned(), c.to_owned()));
        assert_eq!(changes, expected, "{delta:#}");

        let mut foreign = baseline.clone();
        foreign["inventory"]["root"] = serde_json::json!("/somewhere/else");
        let foreign_path = base.join("foreign.json").to_string_lossy().into_owned();
        std::fs::write(&foreign_path, foreign.to_string()).unwrap();
        let err = systemize(&["--previous", &foreign_path], &second)
            .expect_err("a baseline from another root must be refused");
        assert!(err.contains("inventory roots differ"), "{err}");

        std::fs::remove_dir_all(&base).unwrap();
    }

    // Falsification: `--base-sha` is genuinely optional (`Option<String>`), so before this fix
    // `value()` returning `None` for "flag present but its value is missing" was silently
    // indistinguishable from "flag never passed at all" -- exactly the shape a common
    // shell-scripting mistake produces (`--base-sha "$BASE_SHA"` with `$BASE_SHA` unset/empty,
    // dropped entirely by word-splitting). That would have silently skipped
    // `EXACT_BASE_SHA_REQUIRED`'s drift check instead of failing loud on malformed invocation.
    // Confirmed against the unfixed code before writing this fix.
    #[test]
    fn a_base_sha_flag_with_no_following_value_is_a_usage_error_not_a_silent_none() {
        let err = run(&[
            "work".to_owned(),
            "prepare".to_owned(),
            "--root".to_owned(),
            ".".to_owned(),
            "--goal".to_owned(),
            "fix bug".to_owned(),
            "--base-sha".to_owned(),
        ])
        .expect_err("a --base-sha flag with no following value must never succeed silently");
        assert_eq!(err, "--base-sha requires a value");
    }

    // Same defect class, the required-flag side: `--root` present but with no following value
    // must be reported as a malformed flag, not conflated with "--root was never passed".
    #[test]
    fn a_root_flag_with_no_following_value_is_a_usage_error() {
        let err = run(&["check".to_owned(), "--root".to_owned()])
            .expect_err("a --root flag with no following value must never succeed silently");
        assert_eq!(err, "--root requires a value");
    }
}
