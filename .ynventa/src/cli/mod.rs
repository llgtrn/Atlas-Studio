//! `ynventa <command>` — identical commands with identical semantics in every repository:
//!
//! ```text
//! cargo run --manifest-path .ynventa/Cargo.toml -- <command> [--root <repo>] [options]
//! ```

use crate::compact::facts::Knowledge;
use crate::compact::history::{Batch, History, Row};
use crate::formats::json::Json;
use crate::protocol::{own_subsystem_dir, Snapshot, COMMANDS, SNAPSHOT_FILE};
use crate::schema::DonorState;
use crate::{
    assess, audit, compact, conformance, declare, evidence, migration, Assessment, Severity,
};
use std::path::{Path, PathBuf};

struct Args {
    root: PathBuf,
    json: bool,
    flags: Vec<String>,
    values: Vec<(String, String)>,
    positional: Vec<String>,
    multi: Vec<(String, Vec<String>)>,
}

impl Args {
    fn parse(raw: &[String]) -> Result<Args, String> {
        let mut a = Args {
            root: crate::default_root(),
            json: false,
            flags: vec![],
            values: vec![],
            positional: vec![],
            multi: vec![],
        };
        let valued = [
            "--root",
            "--shard",
            "--origin",
            "--binary",
            "--reference",
            "--out",
            "--system",
            "--from",
            "--node",
            "--into",
        ];
        let lists = ["--merge", "--aggregate"];
        let mut i = 0;
        while i < raw.len() {
            let x = &raw[i];
            if valued.contains(&x.as_str()) {
                let v = raw.get(i + 1).ok_or(format!("{x} needs a value"))?.clone();
                if x == "--root" {
                    a.root = PathBuf::from(&v);
                } else {
                    a.values.push((x.clone(), v));
                }
                i += 2;
            } else if lists.contains(&x.as_str()) {
                let mut vs = Vec::new();
                i += 1;
                while i < raw.len() && !raw[i].starts_with("--") {
                    vs.push(raw[i].clone());
                    i += 1;
                }
                a.multi.push((x.clone(), vs));
            } else if x == "--json" {
                a.json = true;
                i += 1;
            } else if x.starts_with("--") {
                a.flags.push(x.clone());
                i += 1;
            } else {
                a.positional.push(x.clone());
                i += 1;
            }
        }
        Ok(a)
    }
    fn flag(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x == f)
    }
    fn value(&self, k: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    }
    fn list(&self, k: &str) -> Vec<PathBuf> {
        self.multi
            .iter()
            .filter(|(n, _)| n == k)
            .flat_map(|(_, v)| v.iter().map(PathBuf::from))
            .collect()
    }
}

fn usage() -> String {
    let mut s = String::from("ynventa (protocol v1)\nusage: ynventa <command> [--root <repo>] [--json] [options]\n\ncommands:\n");
    for (c, m) in COMMANDS {
        s.push_str(&format!("  {c:<12} {m}\n"));
    }
    s
}

/// Runs a command; returns (exit status, output).
pub fn run(raw: &[String]) -> (i32, String) {
    let Some(cmd) = raw.first() else {
        return (2, usage());
    };
    let args = match Args::parse(&raw[1..]) {
        Ok(a) => a,
        Err(e) => return (2, format!("{e}\n{}", usage())),
    };
    let result = match cmd.as_str() {
        "status" => with(&args, status),
        "verify" => with(&args, verify),
        "audit" => with(&args, audit_cmd),
        "census" => with(&args, census),
        "extinction" => with(&args, extinction),
        "graph" => with(&args, graph),
        "metrics" => with(&args, metrics),
        "compact" => compact_cmd(&args),
        "conformance" => with(&args, conformance_cmd),
        "migrate" => migrate(&args),
        "protocol" => protocol(&args),
        "prove" => prove(&args),
        "capsule" => capsule(&args),
        "link" => link_cmd(&args),
        "show" => show(&args, false),
        "backlinks" => show(&args, true),
        "context" => context(&args),
        "technology" => technology(&args),
        "help" | "--help" | "-h" => Ok((0, usage())),
        other => Err(format!("unknown command `{other}`\n{}", usage())),
    };
    match result {
        Ok(r) => r,
        Err(e) => (2, format!("error: {e}\n")),
    }
}

type Out = Result<(i32, String), String>;

fn with(args: &Args, f: fn(&Args, &Assessment) -> Out) -> Out {
    let a = assess(&args.root)?;
    f(args, &a)
}

fn findings_text(a: &Assessment, limit: usize) -> String {
    let mut s = String::new();
    let (e, w, i) = a
        .findings
        .iter()
        .fold((0, 0, 0), |(e, w, i), f| match f.severity {
            Severity::Error => (e + 1, w, i),
            Severity::Warning => (e, w + 1, i),
            Severity::Info => (e, w, i + 1),
        });
    s.push_str(&format!("findings: {e} errors, {w} warnings, {i} info\n"));
    for f in a.findings.iter().take(limit) {
        s.push_str(&format!(
            "  {:<7} {:<26} {}: {}\n",
            f.severity.wire(),
            f.code,
            f.subject,
            f.detail
        ));
    }
    if a.findings.len() > limit {
        s.push_str(&format!(
            "  … {} more (ynventa audit)\n",
            a.findings.len() - limit
        ));
    }
    s
}

fn metrics_text(a: &Assessment) -> String {
    let mut s = String::new();
    for (k, v) in a.counts.values() {
        s.push_str(&format!("  {k:<30} {v}\n"));
    }
    s
}

fn gate_text(a: &Assessment) -> (bool, String) {
    let mut s = String::from("V1 gate:\n");
    let mut all = true;
    for (k, want, got, pass) in a.counts.v1_gate() {
        all &= pass;
        s.push_str(&format!(
            "  [{}] {k} = {got} (required {want})\n",
            if pass { "x" } else { " " }
        ));
    }
    (all, s)
}

fn status(args: &Args, a: &Assessment) -> Out {
    if args.json {
        return Ok((
            0,
            conformance::render_json(a, &conformance::Suite { checks: vec![] }).render(),
        ));
    }
    let d = &a.declaration;
    let (pass, gate) = gate_text(a);
    Ok((
        0,
        format!(
            "{} ({}) system={} shard={} protocol=v{}\nnodes={} donors={} waves={} shims={}\n\nmetrics:\n{}\n{}V1: {}\n\n{}",
            d.repository.name,
            d.repository.origin,
            d.repository.system,
            d.repository.shard,
            crate::YNVENTA_PROTOCOL_VERSION,
            d.repository.nodes.len(),
            d.donors.len(),
            d.migration.waves.len(),
            d.migration.shims.len(),
            metrics_text(a),
            gate,
            if pass { "PASS" } else { "NOT YET" },
            findings_text(a, 25)
        ),
    ))
}

fn verify(_: &Args, a: &Assessment) -> Out {
    let errors = a.errors().count();
    let checks = conformance::protocol_checks(a, None);
    let failed: Vec<&conformance::Check> = checks.iter().filter(|c| !c.pass).collect();
    let mut s = findings_text(a, usize::MAX);
    s.push_str(&format!(
        "protocol checks: {}/{} pass\n",
        checks.len() - failed.len(),
        checks.len()
    ));
    for c in &failed {
        s.push_str(&format!("  FAIL {}: {}\n", c.id, c.detail));
    }
    let ok = errors == 0 && failed.is_empty();
    s.push_str(if ok {
        "verify: PASS\n"
    } else {
        "verify: FAIL\n"
    });
    Ok((if ok { 0 } else { 1 }, s))
}

fn audit_cmd(args: &Args, a: &Assessment) -> Out {
    if args.json {
        let docs: Vec<Json> = a
            .docs
            .docs
            .iter()
            .map(|d| {
                Json::obj()
                    .with("path", &d.path)
                    .with("allowed", d.allowed)
                    .with("extracted", d.extracted)
                    .with(
                        "issues",
                        d.issues.iter().map(|i| i.wire()).collect::<Vec<_>>(),
                    )
            })
            .collect();
        return Ok((
            0,
            Json::obj()
                .with(
                    "findings",
                    Json::Array(a.findings.iter().map(|f| f.to_json()).collect()),
                )
                .with("documents", Json::Array(docs))
                .render(),
        ));
    }
    let mut s = findings_text(a, usize::MAX);
    s.push_str(&format!(
        "\ndocument budget: {} documents, {} over budget, {} extracted\n",
        a.docs.docs.len(),
        a.docs.over_budget().count(),
        a.docs.docs.iter().filter(|d| d.extracted).count()
    ));
    Ok((0, s))
}

fn census(args: &Args, a: &Assessment) -> Out {
    let c = &a.census;
    let mut s = format!(
        "members: {}\ninternal edges: {}\nforeign references: {}\nlocked external closure: {}\n",
        c.members.len(),
        c.internal.len(),
        c.observations.len(),
        c.closure.len()
    );
    for o in &c.observations {
        s.push_str(&format!(
            "  {:<16} {:<8} {:<7} {} ({})\n",
            o.via.wire(),
            o.ecosystem,
            o.scope,
            o.name,
            o.file
        ));
    }
    if args.flag("--record") {
        let mut registered = a.history.registered.clone();
        let mut active = a.history.active.clone();
        let mut states = Vec::new();
        for d in &a.analysis.donors {
            if d.effective >= DonorState::Registered {
                registered.insert(d.key.clone());
            }
            if d.facts.active() {
                active.insert(d.key.clone());
            }
            states.push((d.key.clone(), d.effective));
        }
        let row = Row {
            seq: a.history.next_seq(),
            commit: crate::repository::files::head_commit(&a.root).unwrap_or_default(),
            states,
            metrics: a.counts.values(),
        };
        let path = History::record(
            &a.root,
            &Batch {
                registered,
                active,
                rows: vec![row],
            },
        )
        .map_err(|e| e.to_string())?;
        s.push_str(&format!("recorded {path}\n"));
    }
    Ok((0, s))
}

fn extinction(args: &Args, a: &Assessment) -> Out {
    let only = args.positional.first();
    if args.json {
        let donors: Vec<Json> = a
            .analysis
            .donors
            .iter()
            .filter(|d| only.is_none_or(|k| *k == d.key))
            .map(|d| {
                Json::obj()
                    .with("donor", &d.key)
                    .with("claimed", d.claimed.wire())
                    .with("effective", d.effective.wire())
                    .with(
                        "exception",
                        d.exception
                            .as_ref()
                            .map(|(k, r)| format!("{k}: {r}"))
                            .unwrap_or_default(),
                    )
                    .with("stopped_by", &d.stopped_by)
                    .with(
                        "gates",
                        Json::Array(
                            d.gates
                                .iter()
                                .map(|g| {
                                    Json::obj()
                                        .with("gate", g.gate.wire())
                                        .with("pass", g.pass)
                                        .with("detail", &g.detail)
                                })
                                .collect(),
                        ),
                    )
            })
            .collect();
        return Ok((
            0,
            Json::obj()
                .with("schema", crate::protocol::schema_identity())
                .with("donors", Json::Array(donors))
                .render(),
        ));
    }
    let mut s = String::new();
    for d in a
        .analysis
        .donors
        .iter()
        .filter(|d| only.is_none_or(|k| *k == d.key))
    {
        s.push_str(&format!(
            "{}  claimed {}  effective {}{}\n",
            d.key,
            d.claimed,
            d.effective,
            d.exception
                .as_ref()
                .map(|(k, r)| format!("  [{k}: {r}]"))
                .unwrap_or_default()
        ));
        if !d.stopped_by.is_empty() {
            s.push_str(&format!("  next: {}\n", d.stopped_by));
        }
        if only.is_some() {
            for g in &d.gates {
                s.push_str(&format!(
                    "  [{}] {:<28} {}\n",
                    if g.pass { "x" } else { " " },
                    g.gate.wire(),
                    g.detail
                ));
            }
            for c in &d.capabilities {
                s.push_str(&format!("  capability {} required={} specified={} native={} ({}) parity={:?} regression={:?}\n", c.key, c.required, c.specified, c.native, c.native_detail, c.parity.iter().map(|(_, v)| v.wire()).collect::<Vec<_>>(), c.regression.iter().map(|(_, v)| v.wire()).collect::<Vec<_>>()));
            }
        }
    }
    if s.is_empty() {
        s.push_str("no donors\n");
    }
    Ok((0, s))
}

fn graph(args: &Args, a: &Assessment) -> Out {
    let mut g = a.graph.clone();
    let mut issues = Vec::new();
    for other in args.list("--merge") {
        let b = assess(&other)?;
        issues.extend(g.merge(&b.graph));
    }
    if let Some(out) = args.value("--binary") {
        std::fs::write(out, g.encode()).map_err(|e| e.to_string())?;
    }
    let mut s = if args.json {
        g.to_json().render()
    } else {
        g.render_text()
    };
    for i in issues {
        s.push_str(&format!(
            "merge issue {} {}: {}\n",
            i.code, i.subject, i.detail
        ));
    }
    Ok((0, s))
}

fn metrics(args: &Args, a: &Assessment) -> Out {
    let mut total = a.counts.clone();
    let others = args.list("--aggregate");
    let mut names = vec![a.declaration.repository.shard.clone()];
    for o in &others {
        let b = assess(o)?;
        total.add(&b.counts);
        names.push(b.declaration.repository.shard.clone());
    }
    if args.json {
        return Ok((0, total.to_json().with("repositories", names).render()));
    }
    let mut s = format!("repositories: {}\n", names.join(" "));
    for (k, v) in total.values() {
        s.push_str(&format!("{k} {v}\n"));
    }
    Ok((0, s))
}

fn conformance_cmd(args: &Args, a: &Assessment) -> Out {
    let reference = match args.value("--reference") {
        Some(p) => {
            Some(conformance::load_reference(Path::new(p)).ok_or(format!("no snapshot at {p}"))?)
        }
        None => None,
    };
    let suite = conformance::run(a, reference.as_ref());
    let pass = suite.checks.iter().all(|c| c.pass);
    if args.json {
        return Ok((
            if pass { 0 } else { 1 },
            conformance::render_json(a, &suite).render(),
        ));
    }
    let mut s = String::new();
    for c in &suite.checks {
        s.push_str(&format!(
            "[{}] {:<36} {}\n",
            if c.pass { "x" } else { " " },
            c.id,
            c.detail
        ));
    }
    let (gate_pass, gate) = gate_text(a);
    s.push_str(&format!("\n{gate}"));
    s.push_str(&format!(
        "\nconformance: {}/{} checks pass; V1 gate {}\n",
        suite.checks.iter().filter(|c| c.pass).count(),
        suite.checks.len(),
        if gate_pass { "PASS" } else { "NOT YET" }
    ));
    Ok((if pass { 0 } else { 1 }, s))
}

fn compact_cmd(args: &Args) -> Out {
    let root = &args.root;
    let mut s = String::new();
    let a = assess(root)?;
    if args.flag("--extract-docs") {
        let mut seq = a.knowledge.next_seq();
        let mut facts = Vec::new();
        let mut n = 0;
        for d in a.docs.over_budget().filter(|d| !d.extracted) {
            if let Some(text) = a.files.read(&d.path) {
                facts.extend(audit::extract(&d.path, &text, seq));
                seq += 1;
                n += 1;
            }
        }
        if !facts.is_empty() {
            let p = Knowledge::add(root, &facts).map_err(|e| e.to_string())?;
            s.push_str(&format!("extracted {n} documents into {p}\n"));
        }
    }
    if args.flag("--extract-legacy") {
        let mut seq = a.knowledge.next_seq();
        let mut facts = Vec::new();
        let (mut n, mut binary) = (0, 0);
        let docs: std::collections::BTreeSet<&str> =
            a.docs.docs.iter().map(|d| d.path.as_str()).collect();
        let sources = crate::census::excluded_roots(&a.declaration);
        let imported: std::collections::BTreeMap<&str, &str> = a
            .knowledge
            .facts
            .iter()
            .filter(|f| f.kind == crate::schema::FactKind::LegacyRecord && f.key == "imported")
            .map(|f| (f.subject.as_str(), f.value.as_str()))
            .collect();
        for sh in a
            .declaration
            .migration
            .shims
            .iter()
            .filter(|s| s.kind == crate::schema::ShimKind::LegacyInput)
        {
            for f in a.files.under(&sh.path) {
                let in_source = sources
                    .iter()
                    .any(|p| p != &sh.path && (f.starts_with(&format!("{p}/")) || f == p));
                if docs.contains(f.as_str()) || in_source {
                    continue;
                }
                let Some(bytes) = a.files.read_bytes(f) else {
                    continue;
                };
                if imported.get(f.as_str()) == Some(&crate::digest::content_digest(&bytes).as_str())
                {
                    continue;
                }
                match migration::legacy::extract_file(f, &bytes, seq) {
                    Some(x) => {
                        facts.extend(x);
                        n += 1;
                        seq += 1;
                    }
                    None => binary += 1,
                }
            }
        }
        if !facts.is_empty() {
            let p = Knowledge::add(root, &facts).map_err(|e| e.to_string())?;
            s.push_str(&format!("extracted {n} legacy files into {p}\n"));
        }
        if binary > 0 {
            s.push_str(&format!(
                "{binary} binary legacy files cannot be consumed; handle them explicitly\n"
            ));
        }
    }
    let (kr, kw) = Knowledge::compact(root).map_err(|e| e.to_string())?;
    let (hr, hw) = History::compact(root).map_err(|e| e.to_string())?;
    s.push_str(&format!(
        "knowledge: folded {kr} batches -> {}\nhistory: folded {hr} batches -> {}\n",
        kw.unwrap_or_else(|| "(empty)".into()),
        hw.unwrap_or_else(|| "(empty)".into())
    ));
    // Evidence: a stale record is dropped once a fresh record of the same proof exists.
    let mut pruned = 0;
    for dn in &a.declaration.donors {
        for c in &dn.capabilities {
            let Some(subject) = &c.replacement else {
                continue;
            };
            for p in &c.proofs {
                let (pd, sd) = evidence::current_digests(&a.files, &a.declaration, p, subject);
                let Some(records) = a.evidence.records.get(&p.locator) else {
                    continue;
                };
                let fresh = |r: &evidence::Record| r.proof_digest == pd && r.subject_digest == sd;
                if records.iter().any(fresh) {
                    for r in records.iter().filter(|r| !fresh(r)) {
                        let name = format!(
                            "{}.ynv",
                            &crate::digest::hex(&crate::digest::sha256(&r.encode()))[..32]
                        );
                        if std::fs::remove_file(root.join(evidence::EVIDENCE_DIR).join(name))
                            .is_ok()
                        {
                            pruned += 1;
                        }
                    }
                }
            }
        }
    }
    s.push_str(&format!("evidence: pruned {pruned} stale records\n"));
    if args.flag("--prune-docs") {
        // Judge extraction on the knowledge as it is now, including what was just extracted.
        let a = assess(root)?;
        let mut n = 0;
        for d in a.docs.over_budget().filter(|d| d.extracted) {
            if std::fs::remove_file(root.join(&d.path)).is_ok() {
                n += 1;
            }
        }
        s.push_str(&format!("documents: deleted {n} extracted documents\n"));
    }
    let a = assess(root)?;
    let facts = compact::facts::encode(&a.knowledge.facts);
    let capsule = compact::encode_capsule(&a.graph.encode(), &facts, &a.counts.values());
    std::fs::create_dir_all(root.join(compact::GENERATED_DIR)).map_err(|e| e.to_string())?;
    std::fs::write(root.join(compact::CAPSULE_FILE), &capsule).map_err(|e| e.to_string())?;
    std::fs::write(root.join(compact::VIEW_FILE), view(&a)).map_err(|e| e.to_string())?;
    s.push_str(&format!(
        "capsule: {} ({} bytes)\nview: {}\n",
        compact::CAPSULE_FILE,
        capsule.len(),
        compact::VIEW_FILE
    ));
    Ok((0, s))
}

/// The generated human view. Never authoritative; regenerate at will.
pub fn view(a: &Assessment) -> String {
    let d = &a.declaration;
    let mut s = format!("<!-- GENERATED by `ynventa compact`: a view, not a source of truth. Do not edit or commit. -->\n# {} — Ynventa view\n\nschema `{}`\n\n## Metrics\n\n| metric | value |\n|---|---|\n", d.repository.name, crate::protocol::schema_identity());
    for (k, v) in a.counts.values() {
        s.push_str(&format!("| {k} | {v} |\n"));
    }
    s.push_str("\n## Nodes (legacy path → node → canonical path)\n\n| node | id | kind | path | canonical | native |\n|---|---|---|---|---|---|\n");
    for m in migration::pathmap::path_map(d) {
        let n = d.node(&m.node).unwrap();
        s.push_str(&format!(
            "| {} | `{}` | {} | {} | {} | {} |\n",
            m.node,
            m.node_id,
            n.kind,
            m.legacy_path,
            m.canonical_path,
            a.analysis
                .node_status
                .get(&m.node)
                .map(|x| x.wire())
                .unwrap_or("-")
        ));
    }
    s.push_str("\n## Donors\n\n| donor | claimed | effective | next |\n|---|---|---|---|\n");
    for x in &a.analysis.donors {
        s.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            x.key,
            x.claimed,
            x.effective,
            x.stopped_by.replace('|', "/")
        ));
    }
    s.push_str("\n## Knowledge\n\n");
    for f in a
        .knowledge
        .facts
        .iter()
        .filter(|f| f.kind != crate::schema::FactKind::Document)
    {
        s.push_str(&format!(
            "- {} {} {}: {}{}\n",
            f.kind,
            f.subject,
            f.key,
            f.value.replace('\n', " "),
            if f.superseded.is_empty() {
                String::new()
            } else {
                format!(" (supersedes {})", f.superseded.len())
            }
        ));
    }
    s
}

fn migrate(args: &Args) -> Out {
    let root = &args.root;
    let sub = args.positional.first().map(String::as_str).unwrap_or("");
    match sub {
        "scaffold" => {
            let r = migration::scaffold::scaffold(
                root,
                args.value("--shard"),
                args.value("--origin"),
                args.flag("--force"),
            )?;
            Ok((
                0,
                format!(
                    "subsystem files copied: {}\ndeclarations written: {}\ndialects: {}\nnodes: {} donors: {} waves: {} legacy facts: {}\n",
                    r.copied,
                    r.declarations_written,
                    if r.dialects.is_empty() { "none".into() } else { r.dialects.join(", ") },
                    r.nodes,
                    r.donors,
                    r.waves,
                    r.facts
                ),
            ))
        }
        "import" => {
            let files = crate::repository::files::Files::scan(root).map_err(|e| e.to_string())?;
            let imp = migration::legacy::import(
                root,
                &files,
                args.value("--shard"),
                args.value("--origin"),
            );
            let exists = declare::declared_dir(root)
                .join(declare::REPOSITORY_FILE)
                .exists();
            let write = !args.flag("--dry-run") && (!exists || args.flag("--force"));
            if write {
                declare::store(root, &imp.declaration).map_err(|e| e.to_string())?;
                if !imp.facts.is_empty() {
                    Knowledge::add(root, &imp.facts).map_err(|e| e.to_string())?;
                }
            }
            let mut s = format!(
                "dialects: {}\nnodes: {}\ndonors: {}\nwaves: {}\nlegacy facts: {}\n",
                imp.dialects.join(", "),
                imp.declaration.repository.nodes.len(),
                imp.declaration.donors.len(),
                imp.declaration.migration.waves.len(),
                imp.facts.len()
            );
            s.push_str(if write {
                "declarations written\n"
            } else {
                "dry run (declarations exist; --force overwrites)\n"
            });
            if !write && args.flag("--report") {
                // Judge the imported declarations in memory; nothing is written.
                let a = crate::assess_with(root, files, imp.declaration);
                let (_, gate) = gate_text(&a);
                s.push_str(&format!(
                    "\nmetrics (in memory):\n{}\n{gate}",
                    metrics_text(&a)
                ));
                let mut codes: std::collections::BTreeMap<(&str, &str), usize> =
                    std::collections::BTreeMap::new();
                for f in &a.findings {
                    *codes
                        .entry((f.severity.wire(), f.code.as_str()))
                        .or_default() += 1;
                }
                s.push_str("findings by code:\n");
                for ((sev, code), n) in codes {
                    s.push_str(&format!("  {sev:<7} {code:<26} {n}\n"));
                    for f in a.findings.iter().filter(|f| f.code == code).take(3) {
                        s.push_str(&format!("      {}: {}\n", f.subject, f.detail));
                    }
                }
            }
            Ok((0, s))
        }
        "map" | "plan" | "apply" => {
            let mut d = declare::load(root).map_err(|e| e.to_string())?;
            match sub {
                "map" => {
                    let mut s = String::new();
                    for m in migration::pathmap::path_map(&d) {
                        s.push_str(&format!(
                            "{} -> {} {} -> {}\n",
                            m.legacy_path, m.node, m.node_id, m.canonical_path
                        ));
                    }
                    Ok((0, s))
                }
                "plan" => {
                    let waves = migration::pathmap::plan(&d);
                    let mut s = String::new();
                    for w in &waves {
                        s.push_str(&format!("{} {}: {}\n", w.key, w.status, w.nodes.join(", ")));
                    }
                    if args.flag("--write") && !waves.is_empty() {
                        d.migration.waves.extend(waves);
                        declare::store(root, &d).map_err(|e| e.to_string())?;
                        s.push_str("waves written to .ynventa/declared/migration.rs\n");
                    }
                    Ok((
                        0,
                        if s.is_empty() {
                            "nothing to plan: every node is at its canonical path\n".into()
                        } else {
                            s
                        },
                    ))
                }
                _ => {
                    let wave = args.positional.get(1).ok_or("migrate apply <wave>")?;
                    let files =
                        crate::repository::files::Files::scan(root).map_err(|e| e.to_string())?;
                    let r = migration::pathmap::apply(root, &files, &mut d, wave)?;
                    let mut s = String::new();
                    for (f, t) in &r.moves {
                        s.push_str(&format!("moved {f} -> {t}\n"));
                    }
                    for m in &r.manifests {
                        s.push_str(&format!("rewrote {m}\n"));
                    }
                    for (f, p) in &r.stale_references {
                        s.push_str(&format!(
                            "stale reference to `{p}` in {f} (fix within this wave)\n"
                        ));
                    }
                    s.push_str(&format!("wave {wave} APPLIED; node ids unchanged\n"));
                    Ok((0, s))
                }
            }
        }
        _ => Err("migrate import | map | plan [--write] | apply <wave> | scaffold".into()),
    }
}

fn protocol(args: &Args) -> Out {
    let own = Snapshot::current(own_subsystem_dir());
    if args.flag("--write") {
        std::fs::write(own_subsystem_dir().join("protocol.snapshot"), own.render())
            .map_err(|e| e.to_string())?;
        return Ok((
            0,
            format!(
                "wrote {}\n{}",
                own_subsystem_dir().join("protocol.snapshot").display(),
                own.render()
            ),
        ));
    }
    if args.flag("--schema") {
        return Ok((0, crate::protocol::schema_text()));
    }
    if args.flag("--check") {
        let at = args.root.join(SNAPSHOT_FILE);
        let recorded = Snapshot::load(&args.root);
        return Ok(match recorded {
            Some(r) if r == own => (0, format!("{} is current\n", at.display())),
            Some(r) => (
                1,
                format!(
                    "{} is stale\nrecorded:\n{}current:\n{}",
                    at.display(),
                    r.render(),
                    own.render()
                ),
            ),
            None => (1, format!("{} missing\n", at.display())),
        });
    }
    Ok((0, own.render()))
}

fn prove(args: &Args) -> Out {
    let root = &args.root;
    let d = declare::load(root).map_err(|e| e.to_string())?;
    let files = crate::repository::files::Files::scan(root).map_err(|e| e.to_string())?;
    let results = evidence::prove(
        root,
        &files,
        &d,
        args.positional.first().map(String::as_str),
        &mut evidence::CargoRunner,
    );
    let mut s = String::new();
    let mut ok = true;
    for (loc, pass, rec) in &results {
        ok &= *pass;
        s.push_str(&format!(
            "[{}] {loc} -> {rec}\n",
            if *pass { "x" } else { " " }
        ));
    }
    if results.is_empty() {
        s.push_str("no declared proofs\n");
    }
    Ok((if ok { 0 } else { 1 }, s))
}

/// Assesses a shard from its declarations, or — for a shard without `.ynventa` yet — from an
/// in-memory legacy import that writes nothing.
pub fn assess_or_import(root: &Path) -> Result<Assessment, String> {
    if declare::declared_dir(root)
        .join(declare::REPOSITORY_FILE)
        .exists()
    {
        return assess(root);
    }
    let files = crate::repository::files::Files::scan(root).map_err(|e| e.to_string())?;
    let imp = migration::legacy::import(root, &files, None, None);
    Ok(crate::assess_with(root, files, imp.declaration))
}

fn load_system(args: &Args) -> Result<Option<crate::linker::SystemImage>, String> {
    let Some(p) = args.value("--system") else {
        return Ok(None);
    };
    let b = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
    crate::linker::SystemImage::decode(&b)
        .map(Some)
        .map_err(|e| format!("{p}: {e}"))
}

fn capsule(args: &Args) -> Out {
    let a = assess_or_import(&args.root)?;
    let c = crate::capsule::Capsule::compile(&a);
    let bytes = c.encode();
    let out = args
        .value("--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| args.root.join(compact::GENERATED_DIR).join(c.file_name()));
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
    Ok((
        0,
        format!(
            "capsule {} ({} bytes, {})
shard {} head {}
nodes {} edges {} symbols {} technologies {} donors {}
provides {}
requires {}
",
            out.display(),
            bytes.len(),
            crate::digest::content_digest(&bytes),
            c.shard,
            if c.head.is_empty() { "-" } else { &c.head },
            c.graph.nodes.len(),
            c.graph.edges.len(),
            c.symbols.len(),
            c.technologies.len(),
            c.donors.len(),
            c.provides().join(", "),
            c.requires().join(", ")
        ),
    ))
}

fn link_cmd(args: &Args) -> Out {
    if args.positional.is_empty() {
        return Err("link <capsule.ynv | shard root>...".into());
    }
    let mut capsules = Vec::new();
    for p in &args.positional {
        let path = Path::new(p);
        let c = if path.is_file() {
            let b = std::fs::read(path).map_err(|e| format!("{p}: {e}"))?;
            crate::capsule::Capsule::decode(&b).map_err(|e| format!("{p}: {e}"))?
        } else {
            crate::capsule::Capsule::compile(&assess_or_import(path)?)
        };
        capsules.push(c);
    }
    let image = crate::linker::link(&capsules);
    let bytes = image.encode();
    let out = args.value("--out").map(PathBuf::from).unwrap_or_else(|| {
        args.root
            .join(compact::GENERATED_DIR)
            .join(crate::linker::SYSTEM_FILE)
    });
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
    let code = if image.pass() { 0 } else { 1 };
    if args.json {
        return Ok((code, image.to_json().render()));
    }
    Ok((
        code,
        format!(
            "{}image {} ({} bytes, {})\n",
            image.render_text(),
            out.display(),
            bytes.len(),
            crate::digest::content_digest(&bytes)
        ),
    ))
}

/// Resolves a user key to a node: a node key, `capability/<k>`, `technology/<k>` or `ynv://` URI.
fn resolve_key<'a>(g: &'a crate::graph::Graph, key: &str) -> Option<&'a crate::graph::GNode> {
    let k = key
        .strip_prefix("ynv://chronica/")
        .map(|r| r.replace('/', "."))
        .unwrap_or_else(|| key.to_string());
    let k = k.strip_prefix("chronica.").unwrap_or(&k).to_string();
    [
        k.clone(),
        format!("capability/{k}"),
        format!("technology/{k}"),
    ]
    .iter()
    .find_map(|c| g.node_by_key(crate::graph::SYSTEM, c))
    .or_else(|| g.nodes.values().find(|n| n.semantic_key == key))
}

fn show(args: &Args, backlinks_only: bool) -> Out {
    let key = args.positional.first().ok_or("<key>")?;
    let system = load_system(args)?;
    let local;
    let g = match &system {
        Some(s) => &s.graph,
        None => {
            local = assess_or_import(&args.root)?;
            &local.graph
        }
    };
    let n = resolve_key(g, key).ok_or(format!("no node `{key}`"))?;
    let label = |id: &crate::graph::NodeId| {
        g.nodes
            .get(id)
            .map(|m| {
                format!(
                    "{} [{}@{}]",
                    m.semantic_key,
                    m.kind,
                    if m.repository.is_empty() {
                        "-"
                    } else {
                        &m.repository
                    }
                )
            })
            .unwrap_or_else(|| format!("{id} (unresolved)"))
    };
    let mut s = format!(
        "{} {}
  id {}
  kind {} concept {}
  physical owner {}
  path {} canonical {}
  lifecycle {} native {}
",
        crate::graph::uri(&n.namespace, &n.semantic_key),
        n.name,
        n.id,
        n.kind,
        n.concept,
        if n.repository.is_empty() {
            "-"
        } else {
            &n.repository
        },
        if n.path.is_empty() { "-" } else { &n.path },
        if n.canonical_path.is_empty() {
            "-"
        } else {
            &n.canonical_path
        },
        n.lifecycle,
        n.native_status
    );
    if !backlinks_only {
        for e in g.outbound(n.id) {
            s.push_str(&format!("  -> {} {} {}\n", e.kind, e.scope, label(&e.to)));
        }
    }
    for e in g.backlinks(n.id) {
        s.push_str(&format!("  <- {} {} {}\n", e.kind, e.scope, label(&e.from)));
    }
    Ok((0, s))
}

fn context(args: &Args) -> Out {
    let a = assess_or_import(&args.root)?;
    let system = load_system(args)?;
    Ok((0, crate::context::render(&a, system.as_ref())))
}

fn technology(args: &Args) -> Out {
    let sub = args
        .positional
        .first()
        .map(String::as_str)
        .unwrap_or("list");
    let system = load_system(args)?;
    match sub {
        "materialize" => {
            let key = args.positional.get(1).ok_or(
                "technology materialize <key> --from <birthplace root> --node <node> --into <dir>",
            )?;
            let from = PathBuf::from(args.value("--from").ok_or("--from <birthplace root>")?);
            let node = args.value("--node").ok_or("--node <consuming node key>")?;
            let into = args.value("--into").ok_or("--into <directory>")?;
            let fd = declare::load(&from).map_err(|e| e.to_string())?;
            let mut d = declare::load(&args.root).map_err(|e| e.to_string())?;
            let n = d
                .repository
                .nodes
                .iter_mut()
                .find(|n| n.key == node)
                .ok_or(format!("no node `{node}` here"))?;
            let m = crate::technology::materialize(&from, &fd, key, &args.root, into)?;
            if !n.reuses.contains(key) {
                n.reuses.push(key.clone());
                n.reuses.sort();
            }
            declare::store(&args.root, &d).map_err(|e| e.to_string())?;
            let mut s = format!(
                "materialized {} from {} ({})\n",
                m.technology, m.birthplace, m.source_digest
            );
            for (dest, src, _) in &m.files {
                s.push_str(&format!("  {src} -> {dest}\n"));
            }
            s.push_str(&format!("node `{node}` REUSES {key}; edits to these files are forks until re-materialized\n"));
            Ok((0, s))
        }
        "search" | "list" | "consumers" => {
            let needle = args
                .positional
                .get(1)
                .map(|x| x.to_ascii_lowercase())
                .unwrap_or_default();
            let mut s = String::new();
            match &system {
                Some(sys) => {
                    for t in &sys.technologies {
                        let hay =
                            format!("{} {}", t.key, t.implements.join(" ")).to_ascii_lowercase();
                        let hit = match sub {
                            "consumers" => t.key == needle,
                            _ => needle.split_whitespace().all(|w| hay.contains(w)),
                        };
                        if hit {
                            s.push_str(&format!(
                                "{:<36} {:<12} born in {:<20} implements {}; consumers: {}\n",
                                t.key,
                                t.effective.wire(),
                                t.birthplace,
                                t.implements.join(", "),
                                if t.consumers.is_empty() {
                                    "-".into()
                                } else {
                                    t.consumers.join(", ")
                                }
                            ));
                        }
                    }
                    if sub == "search" {
                        for c in sys.capabilities.iter().filter(|c| {
                            c.key.to_ascii_lowercase().contains(&needle) && !needle.is_empty()
                        }) {
                            s.push_str(&format!(
                                "capability {:<30} providers {}\n",
                                c.key,
                                c.providers
                                    .iter()
                                    .map(|(sh, n)| format!("{n}@{sh}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                    }
                }
                None => {
                    let a = assess_or_import(&args.root)?;
                    for t in &a.technologies {
                        if t.key.contains(&needle) {
                            s.push_str(&format!(
                                "{:<36} claimed {:<12} effective {:<12} {}\n",
                                t.key,
                                t.claimed.wire(),
                                t.effective.wire(),
                                t.stopped_by
                            ));
                        }
                    }
                }
            }
            if s.is_empty() {
                s.push_str("nothing found: no canonical technology exists for this need; start the research -> native technology pipeline\n");
            }
            Ok((0, s))
        }
        _ => Err("technology list | search <text> | consumers <key> | materialize <key>".into()),
    }
}
