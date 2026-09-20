use atlas_model::Contract;
use std::{env, fs, path::PathBuf, process::ExitCode};

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|x| x == name).and_then(|i| args.get(i + 1)).cloned()
}
fn values(args: &[String], name: &str) -> Vec<String> {
    args.iter().enumerate().filter_map(|(i,v)| if v == name { args.get(i+1).cloned() } else { None }).collect()
}
fn json<T: serde::Serialize>(value: &T) -> Result<String,String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}
fn run(args: &[String]) -> Result<(),String> {
    match args {
        [cmd, rest @ ..] if cmd == "contract" => {
            if value(rest, "--format").as_deref() != Some("json") { return Err("contract requires --format json".into()); }
            println!("{}", json(&Contract::default())?);
        }
        [cmd, sub, rest @ ..] if cmd == "docs" && sub == "audit" => {
            let root=value(rest,"--root").ok_or("docs audit requires --root")?;
            let report=atlas_docs::audit(root).map_err(|e| e.to_string())?;
            println!("{}", json(&report)?);
            if !report.gate_ready { return Err("DOCS_GATE_NOT_READY".into()); }
        }
        [cmd, sub, rest @ ..] if cmd == "code" && sub == "analyze" => {
            let root=value(rest,"--root").ok_or("code analyze requires --root")?;
            let source=atlas_source::analyze(&root).map_err(|e| e.to_string())?;
            let graph=atlas_graph::summarize(&source);
            println!("{}", json(&serde_json::json!({"schema":"atlas.systemizer.code-analysis.v1","source":source,"graph":graph,"source_of_truth":"derived engineering analysis; target repositories remain sovereign"}))?);
        }
        [cmd, sub, rest @ ..] if cmd == "fleet" && sub == "connect" => {
            let manifest=value(rest,"--manifest").unwrap_or_else(|| ".atlas/fleet/repos.yaml".into());
            let report=atlas_fleet::connect(&manifest).map_err(|e| e.to_string())?;
            println!("{}", json(&report)?);
        }
        [cmd, sub, rest @ ..] if cmd == "work" && sub == "prepare" => {
            let repo=value(rest,"--repo").ok_or("work prepare requires --repo")?;
            let root=value(rest,"--root").ok_or("work prepare requires --root")?;
            let base_sha=value(rest,"--base-sha").ok_or("work prepare requires --base-sha")?;
            let scope=values(rest,"--scope");
            if scope.is_empty() { return Err("work prepare requires at least one --scope".into()); }
            let repository=atlas_repo::audit(&root).map_err(|e| e.to_string())?;
            let docs=atlas_docs::audit(PathBuf::from(&root).join(".atlas")).map_err(|e| e.to_string())?;
            let request=atlas_fleet::WorkRequest {
                id: format!("work:{}", repo.replace('/','-')),
                repo,
                base_sha,
                scope,
                repo_gate_ready: repository.ready,
                docs_gate_ready: docs.gate_ready,
                docs_standard: docs.standard.clone(),
            };
            let plan=atlas_fleet::plan(vec![request])?;
            println!("{}", json(&plan)?);
        }
        [cmd, rest @ ..] if cmd == "systemize" => {
            let root=value(rest,"--root").ok_or("systemize requires --root")?;
            let out=value(rest,"--out").ok_or("systemize requires --out")?;
            let report=atlas_core::systemize(&root).map_err(|e| e.to_string())?;
            let text=json(&report)?+"\n";
            let out=PathBuf::from(out);
            if let Some(parent)=out.parent(){fs::create_dir_all(parent).map_err(|e| e.to_string())?;}
            fs::write(&out,&text).map_err(|e| e.to_string())?;
            print!("{text}");
        }
        _ => return Err("usage: atlas-systemizer <contract|systemize|docs audit|code analyze|fleet connect|work prepare> ...".into()),
    }
    Ok(())
}
fn main() -> ExitCode {
    let args:Vec<String>=env::args().skip(1).collect();
    match run(&args){Ok(())=>ExitCode::SUCCESS,Err(m)=>{eprintln!("atlas-systemizer: {m}");ExitCode::from(2)}}
}
