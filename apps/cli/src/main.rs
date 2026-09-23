use std::{env, fs, path::PathBuf, process::ExitCode};

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|x| x == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [cmd, rest @ ..] if cmd == "contract" => {
            if value(rest, "--format").as_deref() != Some("json") {
                return Err("contract requires --format json".into());
            }
            println!("{}", json(&runtime::contract())?);
        }
        [cmd, sub, rest @ ..] if cmd == "docs" && sub == "audit" => {
            let root = value(rest, "--root").ok_or("docs audit requires --root")?;
            let report = runtime::docs_audit(&root).map_err(|e| format!("{root}: {e}"))?;
            println!("{}", json(&report)?);
            if !report.gate_ready {
                return Err("DOCS_GATE_NOT_READY".into());
            }
        }
        [cmd, sub, rest @ ..] if cmd == "code" && sub == "analyze" => {
            let root = value(rest, "--root").ok_or("code analyze requires --root")?;
            let report = runtime::code_analyze(&root).map_err(|e| format!("{root}: {e}"))?;
            println!("{}", json(&report)?);
        }
        [cmd, rest @ ..] if cmd == "parse" => {
            let root = value(rest, "--root").ok_or("parse requires --root")?;
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
            let root = value(rest, "--root").ok_or("check requires --root")?;
            let report = runtime::check(&root).map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            if let Some(out) = value(rest, "--out") {
                let out = PathBuf::from(out);
                if let Some(parent) = out.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::write(&out, &text).map_err(|e| e.to_string())?;
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
            let root = value(rest, "--root").ok_or("graph requires --root")?;
            let report = runtime::graph(&root).map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            if let Some(out) = value(rest, "--out") {
                let out = PathBuf::from(out);
                if let Some(parent) = out.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::write(&out, &text).map_err(|e| e.to_string())?;
            }
            print!("{text}");
        }
        [cmd, rest @ ..] if cmd == "systemize" => {
            let root = value(rest, "--root").ok_or("systemize requires --root")?;
            let out = value(rest, "--out").ok_or("systemize requires --out")?;
            let report = runtime::systemize(&root).map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            let out = PathBuf::from(out);
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&out, &text).map_err(|e| e.to_string())?;
            print!("{text}");
        }
        [cmd, sub, rest @ ..] if cmd == "work" && sub == "prepare" => {
            let root = value(rest, "--root").ok_or("work prepare requires --root")?;
            let goal = value(rest, "--goal").ok_or("work prepare requires --goal")?;
            let expected_base_sha = value(rest, "--base-sha");
            let report = runtime::prepare_work(&root, goal, expected_base_sha)
                .map_err(|e| format!("{root}: {e}"))?;
            let text = json(&report)? + "\n";
            if let Some(out) = value(rest, "--out") {
                let out = PathBuf::from(out);
                if let Some(parent) = out.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::write(&out, &text).map_err(|e| e.to_string())?;
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
}
