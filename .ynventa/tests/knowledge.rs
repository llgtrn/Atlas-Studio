//! The knowledge write path: single typed facts with mandatory provenance, supersession that
//! keeps history, milestones and decisions in `status` and `context`, read-only views, and
//! extraction of exactly the named documents.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::path::Path;
use ynventa::compact::facts::KNOWLEDGE_DIR;
use ynventa::schema::FactKind;

/// Every file under `dir` with its bytes.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn provenance_is_required() {
    let r = extinct_baseline("fact-provenance");
    let before = tree(r.path());
    let (code, out) = r.cli(&["fact", "add", "MILESTONE", "milestone/v1", "status", "DONE"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("provenance is mandatory"), "{out}");
    let (code, out) = r.cli(&[
        "fact",
        "add",
        "MILESTONE",
        "milestone/v1",
        "status",
        "DONE",
        "--provenance",
        "",
    ]);
    assert_eq!(code, 2, "{out}");
    // Unknown kinds, kinds only extraction writes, and malformed subjects are refused too.
    for args in [
        ["fact", "add", "ROADMAP", "milestone/v1", "status", "DONE"],
        ["fact", "add", "DOCUMENT", "README.md", "digest", "sha256:0"],
        ["fact", "add", "MILESTONE", "v1", "status", "DONE"],
    ] {
        let mut v = args.to_vec();
        v.extend(["--provenance", "agent:test"]);
        let (code, out) = r.cli(&v);
        assert_eq!(code, 2, "{args:?}: {out}");
    }
    assert_eq!(before, tree(r.path()), "a refused fact writes nothing");
    assert!(r.assess().knowledge.facts.is_empty());
}

#[test]
fn supersession_keeps_history() {
    let r = extinct_baseline("fact-supersede");
    let add = |verb: &str, kind: &str, subject: &str, key: &str, value: &str, prov: &[&str]| {
        let mut v = vec!["fact", verb, kind, subject, key, value];
        for p in prov {
            v.extend(["--provenance", p]);
        }
        r.cli(&v)
    };
    // `supersede` needs something to supersede.
    let (code, out) = add(
        "supersede",
        "MILESTONE",
        "milestone/v1",
        "status",
        "DONE",
        &["agent:a"],
    );
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("nothing to supersede"), "{out}");

    let (code, out) = add(
        "add",
        "MILESTONE",
        "milestone/v1",
        "status",
        "IN_PROGRESS",
        &["agent:a", "doc:ROADMAP.md#L3"],
    );
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("MILESTONE milestone/v1 status = IN_PROGRESS"),
        "{out}"
    );
    let (code, out) = add(
        "supersede",
        "milestone",
        "milestone/v1",
        "status",
        "DONE",
        &["git:abc:tests/v1.rs"],
    );
    assert_eq!(code, 0, "{out}");
    let (code, _) = add(
        "supersede",
        "MILESTONE",
        "milestone/v1",
        "status",
        "DONE",
        &["agent:b"],
    );
    assert_eq!(code, 2, "an equal value is not a supersession");

    // DECISION facts are written key by key, like any other kind.
    for (key, value) in [("title", "Use native hashing"), ("status", "Proposed")] {
        assert_eq!(
            add("add", "DECISION", "0007-hash", key, value, &["agent:a"]).0,
            0
        );
    }
    assert_eq!(
        add(
            "add",
            "DECISION",
            "0007-hash",
            "status",
            "Accepted",
            &["review:2026-10-01"]
        )
        .0,
        0
    );

    let a = r.assess();
    let k = &a.knowledge;
    let st = k
        .get(FactKind::Milestone, "milestone/v1", "status")
        .unwrap();
    assert_eq!(st.value, "DONE");
    assert_eq!(st.superseded, vec!["IN_PROGRESS".to_string()]);
    assert!(st.provenance.contains("git:abc:tests/v1.rs"));
    assert!(st.provenance.contains("doc:ROADMAP.md#L3"));
    let d = k.get(FactKind::Decision, "0007-hash", "status").unwrap();
    assert_eq!(
        (d.value.as_str(), d.superseded.clone()),
        ("Accepted", vec!["Proposed".to_string()])
    );
    assert_eq!(
        k.get(FactKind::Decision, "0007-hash", "title")
            .unwrap()
            .value,
        "Use native hashing"
    );

    // Folding the batches keeps the history.
    assert_eq!(r.cli(&["compact"]).0, 0);
    let a = r.assess();
    assert_eq!(a.knowledge.files.len(), 1);
    let st = a
        .knowledge
        .get(FactKind::Milestone, "milestone/v1", "status")
        .unwrap();
    assert_eq!(
        (st.value.as_str(), st.superseded.clone()),
        ("DONE", vec!["IN_PROGRESS".to_string()])
    );

    // `fact list` is read-only and filters by kind and subject prefix.
    let before = tree(r.path());
    let (code, out) = r.cli(&["fact", "list", "--kind", "DECISION", "--subject", "0007"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("DECISION 0007-hash status = Accepted")
            && out.contains("supersedes 1")
            && !out.contains("MILESTONE"),
        "{out}"
    );
    let (_, json) = r.cli(&["fact", "list", "--kind", "MILESTONE", "--json"]);
    assert!(
        json.contains("\"superseded\"") && json.contains("IN_PROGRESS"),
        "{json}"
    );
    assert_eq!(before, tree(r.path()));
}

#[test]
fn context_and_status_show_milestones_and_decisions() {
    let r = extinct_baseline("fact-context");
    let (_, ctx) = r.cli(&["context"]);
    assert!(ctx.contains("KNOWLEDGE (0 current facts"), "{ctx}");
    let facts = [
        ("MILESTONE", "gap/no-gpu", "status", "OPEN"),
        ("MILESTONE", "milestone/v2", "rank", "2"),
        ("MILESTONE", "milestone/v2", "status", "PLANNED"),
        ("MILESTONE", "milestone/v1", "rank", "1"),
        ("MILESTONE", "milestone/v1", "status", "DONE"),
        ("MILESTONE", "milestone/v1", "scope", "native page store"),
        ("DECISION", "0001-pages", "status", "Accepted"),
        (
            "STATEMENT",
            "",
            "the log is append-only",
            "The log is append-only.",
        ),
    ];
    for (k, s, key, v) in facts {
        let (code, out) = r.cli(&["fact", "add", k, s, key, v, "--provenance", "agent:t"]);
        assert_eq!(code, 0, "{out}");
    }
    let (code, ctx) = r.cli(&["context"]);
    assert_eq!(code, 0);
    assert!(ctx.contains("KNOWLEDGE (8 current facts"), "{ctx}");
    assert!(
        ctx.contains("STATEMENT 1 · DECISION 1 · MILESTONE 6"),
        "{ctx}"
    );
    let v1 = ctx.find("milestone/v1").expect("milestone v1 shown");
    let v2 = ctx.find("milestone/v2").expect("milestone v2 shown");
    let gap = ctx.find("gap/no-gpu").expect("gap shown");
    assert!(v1 < v2 && v2 < gap, "milestones by rank, then gaps: {ctx}");
    assert!(
        ctx.contains("status=DONE  rank=1  scope=native page store"),
        "{ctx}"
    );
    assert!(ctx.contains("0001-pages") && ctx.contains("status=Accepted"));
    assert!(
        !ctx.contains("The log is append-only."),
        "statements are counted, not listed"
    );
    // Deterministic.
    assert_eq!(ctx, r.cli(&["context"]).1);

    let (code, st) = r.cli(&["status"]);
    assert_eq!(code, 0);
    assert!(
        st.contains("MILESTONES (3)") && st.contains("milestone/v1"),
        "{st}"
    );
    assert!(
        st.contains("DECISIONS (1)") && st.contains("0001-pages"),
        "{st}"
    );
}

#[test]
fn view_writes_nothing_under_ynventa() {
    let r = extinct_baseline("knowledge-view");
    for (k, s, key, v) in [
        ("MILESTONE", "milestone/v1", "status", "DONE"),
        ("DECISION", "0001-pages", "status", "Accepted"),
    ] {
        assert_eq!(
            r.cli(&["fact", "add", k, s, key, v, "--provenance", "agent:t"])
                .0,
            0
        );
    }
    let before = tree(r.path());
    let (code, md) = r.cli(&["knowledge", "view"]);
    assert_eq!(code, 0);
    assert!(md.starts_with("<!-- GENERATED"), "{md}");
    assert!(
        md.contains("## MILESTONE") && md.contains("### milestone/v1"),
        "{md}"
    );
    assert!(md.contains("- **status**: DONE"), "{md}");
    let (_, only) = r.cli(&["knowledge", "view", "--kind", "DECISION", "--text"]);
    assert_eq!(
        only,
        "DECISION 0001-pages status = Accepted  [seq 2, 1 provenance] — agent:t\n"
    );
    assert_eq!(before, tree(r.path()), "a view to stdout writes nothing");

    // Into canonical state: refused, however the path is spelled.
    for out in [
        r.path().join(".ynventa/VIEW.md"),
        r.path().join("core/../.ynventa/knowledge/VIEW.md"),
    ] {
        let (code, msg) = r.cli(&["knowledge", "view", "--out", &out.display().to_string()]);
        assert_eq!(code, 2, "{msg}");
    }
    assert_eq!(before, tree(r.path()));

    // Elsewhere: exactly the named file, nothing under .ynventa.
    let out = r.path().join("target/ynventa/KNOWLEDGE.md");
    let (code, msg) = r.cli(&["knowledge", "view", "--out", &out.display().to_string()]);
    assert_eq!(code, 0, "{msg}");
    let after = tree(r.path());
    let added: Vec<&String> = after.keys().filter(|p| !before.contains_key(*p)).collect();
    assert_eq!(added, vec!["target/ynventa/KNOWLEDGE.md"]);
    assert_eq!(after["target/ynventa/KNOWLEDGE.md"], md.as_bytes());
    assert!(after
        .iter()
        .filter(|(p, _)| p.starts_with(".ynventa/"))
        .all(|(p, b)| before.get(p) == Some(b)));
}

#[test]
fn extract_of_one_path_touches_no_other_document() {
    let r = extinct_baseline("knowledge-extract");
    r.write(
        "docs/roadmap.md",
        "# Roadmap\n\n- The native page store replaces the donor\n  before the second milestone.\n",
    );
    r.write(
        "docs/other.md",
        "# Other\n\n- A remark that must stay unextracted.\n",
    );
    let before = tree(r.path());
    let (code, out) = r.cli(&["knowledge", "extract", "./docs/roadmap.md"]);
    assert_eq!(code, 0, "{out}");
    let after = tree(r.path());
    // Exactly one new knowledge batch; every other file is byte-identical.
    let changed: Vec<&String> = after
        .iter()
        .filter(|(p, b)| before.get(*p) != Some(b))
        .map(|(p, _)| p)
        .collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(changed[0].starts_with(&format!("{KNOWLEDGE_DIR}/")));
    assert!(
        before.keys().all(|p| after.contains_key(p)),
        "nothing removed"
    );

    let a = r.assess();
    let doc = |p: &str| a.docs.docs.iter().find(|d| d.path == p).unwrap();
    assert!(doc("docs/roadmap.md").extracted);
    assert!(!doc("docs/other.md").extracted);
    assert!(a
        .knowledge
        .facts
        .iter()
        .all(|f| f.provenance.iter().all(|p| !p.contains("docs/other.md"))));
    // The wrapped bullet is one fact with a line range.
    let st = a
        .knowledge
        .facts
        .iter()
        .find(|f| f.kind == FactKind::Statement)
        .unwrap();
    assert_eq!(
        st.value,
        "The native page store replaces the donor before the second milestone."
    );
    assert!(st.provenance.contains("doc:docs/roadmap.md#L3-L4"));
    assert_eq!(
        a.knowledge
            .facts
            .iter()
            .filter(|f| f.kind == FactKind::Statement)
            .count(),
        1
    );

    // Unchanged documents are not extracted twice; non-documents and state are refused.
    let (code, out) = r.cli(&["knowledge", "extract", "docs/roadmap.md"]);
    assert_eq!(code, 0);
    assert!(out.contains("unchanged"), "{out}");
    assert_eq!(after, tree(r.path()));
    for bad in ["core/src/lib.rs", ".ynventa/README.md", "docs/missing.md"] {
        assert_eq!(r.cli(&["knowledge", "extract", bad]).0, 2, "{bad}");
    }
    assert_eq!(after, tree(r.path()));
}

#[test]
fn extraction_provenance_names_the_checked_out_commit() {
    let r = extinct_baseline("knowledge-git");
    let sha = "89abcdef0123456789abcdef0123456789abcdef";
    r.write(".git/HEAD", "ref: refs/heads/main\n");
    r.write(".git/refs/heads/main", &format!("{sha}\n"));
    r.write("docs/plan.md", "# Plan\n\n- One step that\n  wraps.\n");
    assert_eq!(r.cli(&["knowledge", "extract", "docs/plan.md"]).0, 0);
    let a = r.assess();
    let st = a
        .knowledge
        .facts
        .iter()
        .find(|f| f.kind == FactKind::Statement)
        .unwrap();
    assert!(
        st.provenance
            .contains(&format!("git:{sha}:docs/plan.md#L3-L4")),
        "{:?}",
        st.provenance
    );
    assert!(st.provenance.contains("doc:docs/plan.md#L3-L4"));
}

#[test]
fn views_show_where_each_fact_comes_from() {
    let r = extinct_baseline("knowledge-provenance");
    let sha = "89abcdef0123456789abcdef0123456789abcdef";
    r.write(".git/HEAD", "ref: refs/heads/main\n");
    r.write(".git/refs/heads/main", &format!("{sha}\n"));
    r.write("docs/plan.md", "# Plan\n\n- One step that\n  wraps.\n");
    assert_eq!(r.cli(&["knowledge", "extract", "docs/plan.md"]).0, 0);
    let (code, out) = r.cli(&[
        "fact",
        "add",
        "MILESTONE",
        "milestone/v1",
        "status",
        "DONE",
        "--provenance",
        "agent:a",
        "--provenance",
        "agent:b",
    ]);
    assert_eq!(code, 0, "{out}");

    let (_, md) = r.cli(&["knowledge", "view"]);
    // The git provenance twin of a doc provenance is not repeated.
    assert!(
        md.contains("- One step that wraps. — doc:docs/plan.md#L3-L4\n"),
        "{md}"
    );
    assert!(
        md.contains("- **status**: DONE — agent:a (+1 more)\n"),
        "{md}"
    );
    let (_, text) = r.cli(&["knowledge", "view", "--text"]);
    assert!(
        text.contains("[seq 1, 2 provenance] — doc:docs/plan.md#L3-L4\n"),
        "{text}"
    );
    // The generated VIEW.md says it too.
    assert_eq!(r.cli(&["compact"]).0, 0);
    let view = r.read("target/ynventa/VIEW.md");
    assert!(
        view.contains("One step that wraps. — doc:docs/plan.md#L3-L4\n"),
        "{view}"
    );
    assert!(view.contains("DONE — agent:a (+1 more)\n"), "{view}");
}
