use atlas_core::Contract;
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
            println!("{}", json(&Contract::default())?);
        }
        [cmd, sub, rest @ ..] if cmd == "docs" && sub == "audit" => {
            let root = value(rest, "--root").ok_or("docs audit requires --root")?;
            let report = adapter::audit_docs(root).map_err(|e| e.to_string())?;
            println!("{}", json(&report)?);
            if !report.gate_ready {
                return Err("DOCS_GATE_NOT_READY".into());
            }
        }
        [cmd, sub, rest @ ..] if cmd == "code" && sub == "analyze" => {
            let root = value(rest, "--root").ok_or("code analyze requires --root")?;
            let repository = adapter::audit_repository(&root).map_err(|e| e.to_string())?;
            let source = match repository.manifest.as_ref() {
                Some(manifest) => {
                    adapter::scan_declared_source(&root, manifest).map_err(|e| e.to_string())?
                }
                None => adapter::scan_source(&root).map_err(|e| e.to_string())?,
            };
            let graph = atlas_core::summarize_graph(&source);
            println!(
                "{}",
                json(&serde_json::json!({
                    "schema": "atlas.systemizer.code-analysis.v1",
                    "source": source,
                    "graph": graph,
                    "source_of_truth": "derived engineering analysis; target repositories remain sovereign"
                }))?
            );
        }
        [cmd, rest @ ..] if cmd == "systemize" => {
            let root = value(rest, "--root").ok_or("systemize requires --root")?;
            let out = value(rest, "--out").ok_or("systemize requires --out")?;
            let report = runtime::systemize(&root).map_err(|e| e.to_string())?;
            let text = json(&report)? + "\n";
            let out = PathBuf::from(out);
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&out, &text).map_err(|e| e.to_string())?;
            print!("{text}");
        }
        _ => {
            return Err(
                "usage: atlas-systemizer <contract|systemize|docs audit|code analyze> ...".into(),
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
