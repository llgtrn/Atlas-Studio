use atlas_model::Contract;
use std::{env, fs, path::PathBuf, process::ExitCode};

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|x| x == name).and_then(|i| args.get(i + 1)).cloned()
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
            println!("{}", json(&atlas_docs::audit(root).map_err(|e| e.to_string())?)?);
        }
        [cmd, sub, rest @ ..] if cmd == "code" && sub == "analyze" => {
            let root=value(rest,"--root").ok_or("code analyze requires --root")?;
            let source=atlas_source::analyze(&root).map_err(|e| e.to_string())?;
            let graph=atlas_graph::summarize(&source);
            println!("{}", json(&serde_json::json!({"schema":"atlas.systemizer.code-analysis.v1","source":source,"graph":graph,"source_of_truth":"derived development analysis; never canonical Chronica runtime truth"}))?);
        }
        [cmd, rest @ ..] if cmd == "systemize" => {
            let root=value(rest,"--root").ok_or("systemize requires --root")?;
            let config=value(rest,"--config").ok_or("systemize requires --config")?;
            let out=value(rest,"--out").ok_or("systemize requires --out")?;
            let report=atlas_core::systemize(&root,&config).map_err(|e| e.to_string())?;
            let text=json(&report)?+"\n";
            let out=PathBuf::from(out);
            if let Some(parent)=out.parent(){fs::create_dir_all(parent).map_err(|e| e.to_string())?;}
            fs::write(&out,&text).map_err(|e| e.to_string())?;
            print!("{text}");
        }
        _ => return Err("usage: atlas-systemizer <contract|systemize|docs audit|code analyze> ...".into()),
    }
    Ok(())
}
fn main() -> ExitCode {
    let args:Vec<String>=env::args().skip(1).collect();
    match run(&args){Ok(())=>ExitCode::SUCCESS,Err(m)=>{eprintln!("atlas-systemizer: {m}");ExitCode::from(2)}}
}
