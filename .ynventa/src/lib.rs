//! Ynventa protocol v1 — the canonical subsystem every Ynventa repository carries at `.ynventa/`.
//!
//! One assessment of a repository root drives every command:
//!
//! ```text
//! .ynventa/declared/*.rs ─┐                        ┌─ graph (stable NodeIds, typed edges)
//! committed files ────────┼─► census ─► analysis ──┼─ donor lifecycle + extinction gates
//! .ynventa/evidence ──────┤                        ├─ shape + document budget
//! .ynventa/history ───────┤                        ├─ migration waves + shims
//! .ynventa/knowledge ─────┘                        └─ metrics (one schema, one formula set)
//! ```
//!
//! Zero dependencies; no network; no dependency on any other repository.

pub mod audit;
pub mod capsule;
pub mod census;
pub mod cli;
pub mod compact;
pub mod conformance;
pub mod context;
pub mod declare;
pub mod digest;
pub mod donors;
pub mod evidence;
pub mod extinction;
pub mod formats;
pub mod graph;
pub mod ir;
pub mod linker;
pub mod metrics;
pub mod migration;
pub mod protocol;
pub mod repository;
pub mod schema;
pub mod technology;

pub use protocol::YNVENTA_PROTOCOL_VERSION;

use crate::formats::json::Json;
use crate::graph::{Graph, NodeId};
use crate::schema::{DonorState, EdgeKind, ExceptionKind, NodeKind, Scope};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn wire(self) -> &'static str {
        match self {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
            Severity::Info => "INFO",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub severity: Severity,
    pub code: String,
    pub subject: String,
    pub detail: String,
}

impl Finding {
    pub fn new(severity: Severity, code: &str, subject: &str, detail: &str) -> Finding {
        Finding {
            severity,
            code: code.to_string(),
            subject: subject.to_string(),
            detail: detail.to_string(),
        }
    }
    pub fn to_json(&self) -> Json {
        Json::obj()
            .with("severity", self.severity.wire())
            .with("code", &self.code)
            .with("subject", &self.subject)
            .with("detail", &self.detail)
    }
}

/// Everything known about one repository, computed once.
pub struct Assessment {
    pub root: PathBuf,
    pub files: repository::files::Files,
    pub declaration: declare::Declaration,
    pub census: census::Census,
    pub evidence: evidence::Store,
    pub history: compact::history::History,
    pub knowledge: compact::facts::Knowledge,
    pub graph: Graph,
    pub analysis: donors::Analysis,
    pub technologies: Vec<technology::TechnologyAssessment>,
    pub docs: audit::DocAudit,
    pub shape: repository::shape::ShapeReport,
    pub migration: migration::Status,
    pub findings: Vec<Finding>,
    pub counts: metrics::Counts,
}

impl Assessment {
    pub fn errors(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
    }
    pub fn metric(&self, name: &str) -> String {
        self.counts
            .values()
            .into_iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
            .unwrap_or_default()
    }
    pub fn donor(&self, key: &str) -> Option<&donors::DonorAssessment> {
        self.analysis.donors.iter().find(|d| d.key == key)
    }
}

/// Assesses the repository at `root`.
pub fn assess(root: &Path) -> Result<Assessment, String> {
    let declaration = declare::load(root).map_err(|e| {
        format!(
            "{e}\n(not a Ynventa repository? run `ynventa migrate scaffold --root {}`)",
            root.display()
        )
    })?;
    let files = repository::files::Files::scan(root)
        .map_err(|e| format!("cannot scan {}: {e}", root.display()))?;
    Ok(assess_with(root, files, declaration))
}

/// Assesses with protocol conformance counted.
pub fn assess_with(
    root: &Path,
    files: repository::files::Files,
    declaration: declare::Declaration,
) -> Assessment {
    let mut a = assess_core(root, files, declaration);
    let checks = conformance::protocol_checks(&a, None);
    a.counts.protocol_checks = checks.len() as u64;
    a.counts.protocol_checks_passed = checks.iter().filter(|c| c.pass).count() as u64;
    a
}

/// Everything except the protocol checks (which themselves re-run this to prove determinism).
pub fn assess_core(
    root: &Path,
    files: repository::files::Files,
    declaration: declare::Declaration,
) -> Assessment {
    let census = census::run(&files, &declaration);
    let evidence = evidence::Store::load(root);
    let history = compact::history::History::load(root);
    let knowledge = compact::facts::Knowledge::load(root);
    let (mut graph, graph_issues) = Graph::from_declaration(&declaration);
    let analysis = donors::analyze(&declaration, &files, &census, &evidence, &history);
    let mut technology_findings = Vec::new();
    let technologies = technology::assess(
        &declaration,
        &files,
        &evidence,
        &analysis.node_status,
        &mut technology_findings,
    );
    let docs = audit::audit(&files, &declaration, &knowledge);
    let shape = repository::shape::check(&files, &declaration, &census);
    let migration = migration::status(&files, &declaration, &analysis, &knowledge, &docs);
    overlay(&mut graph, &declaration, &census, &analysis);

    let mut findings = Vec::new();
    for i in graph_issues {
        findings.push(Finding::new(Severity::Error, i.code, &i.subject, &i.detail));
    }
    findings.extend(analysis.findings.iter().cloned());
    findings.extend(technology_findings);
    let mut unreadable_locks = Vec::new();
    let locks = technology::Materialization::load_all(root, &mut unreadable_locks);
    technology::check_materialized(&files, &locks, &mut findings);
    for u in unreadable_locks {
        findings.push(Finding::new(
            Severity::Error,
            "CORRUPT_STATE",
            &u,
            "a materialization lock is unreadable",
        ));
    }
    findings.extend(shape.findings.iter().cloned());
    findings.extend(migration.findings.iter().cloned());
    for u in census.unreadable.iter() {
        findings.push(Finding::new(
            Severity::Warning,
            "UNREADABLE_MANIFEST",
            u,
            "could not be read or parsed",
        ));
    }
    for u in evidence
        .unreadable
        .iter()
        .chain(&history.unreadable)
        .chain(&knowledge.unreadable)
    {
        findings.push(Finding::new(
            Severity::Error,
            "CORRUPT_STATE",
            u,
            "a Ynventa state file is unreadable or not content-addressed",
        ));
    }
    for doc in docs.over_budget() {
        let issues: Vec<String> = doc.issues.iter().map(|i| i.wire()).collect();
        findings.push(Finding::new(
            Severity::Warning,
            "DOCUMENT_OVER_BUDGET",
            &doc.path,
            &format!(
                "{}{}",
                issues.join("; "),
                if doc.extracted {
                    "; knowledge extracted — safe to prune"
                } else {
                    ""
                }
            ),
        ));
    }
    findings.sort();
    findings.dedup();

    let mut a = Assessment {
        root: root.to_path_buf(),
        files,
        declaration,
        census,
        evidence,
        history,
        knowledge,
        graph,
        analysis,
        technologies,
        docs,
        shape,
        migration,
        findings,
        counts: metrics::Counts::default(),
    };
    a.counts = count(&a);
    a
}

/// The counts of an assessment, excluding protocol checks.
pub fn count_public(a: &Assessment) -> metrics::Counts {
    count(a)
}

/// Adds census observations and computed nativeness to the declared graph.
fn overlay(g: &mut Graph, d: &declare::Declaration, c: &census::Census, a: &donors::Analysis) {
    let ns = schema::SYSTEM.to_string();
    let shard = d.repository.shard.clone();
    let index = donors::NodeIndex::new(d);
    for (key, status) in &a.node_status {
        if let Some(n) = g.nodes.get_mut(&NodeId::of(&ns, key)) {
            n.native_status = *status;
        }
    }
    let owner_id = |file: &str| -> NodeId {
        match index.owner(file) {
            Some(k) => NodeId::of(&ns, k),
            None => NodeId::of(&ns, &format!("{}/{shard}", graph::REPOSITORY_KEY)),
        }
    };
    for da in &a.donors {
        let dn = d.donor(&da.key).expect("assessed donors are declared");
        let (dns, dkey) = Graph::donor_id(&shard, &dn.key, &dn.origin);
        let did = NodeId::of(&dns, &dkey);
        for o in da
            .facts
            .runtime
            .iter()
            .chain(&da.facts.build)
            .chain(&da.facts.linked)
            .chain(&da.facts.test)
        {
            let kind = if o.via == census::Via::Process {
                EdgeKind::Calls
            } else {
                EdgeKind::DependsOn
            };
            let scope = if o.scope == Scope::Semantic {
                Scope::Runtime
            } else {
                o.scope
            };
            g.add_edge(owner_id(&o.file), did, kind, scope);
        }
    }
    for ((eco, name), obs) in &a.unregistered {
        let mut n = graph::GNode::new(
            "external",
            &format!("{}:{name}", eco.wire().to_ascii_lowercase()),
            NodeKind::External,
            name,
        );
        n.repository = format!("{eco}");
        let id = n.id;
        let _ = g.add_node(n);
        for o in obs {
            let kind = if o.via == census::Via::Process {
                EdgeKind::Calls
            } else {
                EdgeKind::DependsOn
            };
            g.add_edge(owner_id(&o.file), id, kind, o.scope);
        }
    }
    for (from, to, scope) in &c.internal {
        let f = owner_id(&format!("{from}/Cargo.toml"));
        let t = owner_id(&format!("{to}/Cargo.toml"));
        if f != t {
            g.add_edge(f, t, EdgeKind::DependsOn, *scope);
        }
    }
}

fn count(a: &Assessment) -> metrics::Counts {
    let mut c = metrics::Counts::default();
    let declared: BTreeSet<&str> = a
        .declaration
        .donors
        .iter()
        .map(|d| d.key.as_str())
        .collect();
    let mut registered: BTreeSet<String> = a.history.registered.clone();
    for d in &a.analysis.donors {
        let is_registered =
            d.effective >= DonorState::Registered || a.history.registered.contains(&d.key);
        if is_registered {
            registered.insert(d.key.clone());
        }
        match &d.exception {
            Some((ExceptionKind::Rejected, _)) if is_registered => c.donors_rejected += 1,
            Some((ExceptionKind::Superseded, _)) if is_registered => c.donors_superseded += 1,
            Some((ExceptionKind::Blocked, _)) => c.donors_blocked += 1,
            _ => {}
        }
        if !is_registered || d.resolved() {
            continue;
        }
        let at = |s: DonorState| (d.effective >= s) as u64;
        c.donors_censused += at(DonorState::Censused);
        c.donors_specified += at(DonorState::Specified);
        c.donors_native_shadow += at(DonorState::NativeShadow);
        c.donors_parity_proven += at(DonorState::ParityProven);
        c.donors_cutover += at(DonorState::Cutover);
        c.donors_extinct += at(DonorState::Extinct);
        for cap in d.capabilities.iter().filter(|c| c.required) {
            c.capabilities_total += 1;
            c.capabilities_native += cap.native as u64;
            c.capabilities_proven += cap.proven() as u64;
        }
    }
    // Discovered donors that participate are as unregistered as unknown externals.
    let discovered_active = a
        .analysis
        .donors
        .iter()
        .filter(|d| {
            d.effective < DonorState::Registered
                && !a.history.registered.contains(&d.key)
                && d.facts.active()
        })
        .count() as u64;
    c.donors_registered = registered.len() as u64;
    c.unregistered_externals = a.analysis.unregistered.len() as u64 + discovered_active;
    c.donors_discovered = declared.len() as u64
        + a.analysis.unregistered.len() as u64
        + a.analysis.vanished.len() as u64;

    let mut seen = BTreeSet::new();
    for o in &a.census.observations {
        if o.via == census::Via::Process && donors::TOOLCHAIN_PROGRAMS.contains(&o.name.as_str()) {
            continue;
        }
        if o.via == census::Via::Link && donors::PLATFORM_LIBRARIES.contains(&o.name.as_str()) {
            continue;
        }
        if !seen.insert((o.file.clone(), o.ident.clone(), o.scope)) {
            continue;
        }
        match o.scope {
            Scope::Runtime | Scope::Semantic | Scope::Architectural => {
                c.runtime_external_edges += 1
            }
            Scope::Build => c.build_external_edges += 1,
            Scope::Linked => c.linked_external_edges += 1,
            Scope::Test => c.test_external_edges += 1,
        }
    }
    c.external_closure_packages = a.census.closure.len() as u64;
    // External technology dependence: every (owner, foreign technology) pair, whatever the
    // coupling — including source held in the tree, which has no package edge at all.
    let index = donors::NodeIndex::new(&a.declaration);
    let owner = |f: &str| index.owner(f).unwrap_or("").to_string();
    let mut tech_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for d in a.analysis.donors.iter().filter(|d| !d.resolved()) {
        let f = &d.facts;
        for o in f
            .runtime
            .iter()
            .chain(&f.build)
            .chain(&f.linked)
            .chain(&f.test)
        {
            tech_edges.insert((owner(&o.file), d.key.clone()));
        }
        for (file, _) in &f.imports {
            tech_edges.insert((owner(file), d.key.clone()));
        }
        for file in &f.resident {
            tech_edges.insert((owner(file), d.key.clone()));
        }
    }
    for ((eco, name), obs) in &a.analysis.unregistered {
        for o in obs {
            tech_edges.insert((owner(&o.file), format!("{eco}:{name}")));
        }
    }
    c.external_technology_edges = tech_edges.len() as u64;
    for t in &a.technologies {
        use schema::TechnologyLifecycle as L;
        let live = t.effective != L::Superseded;
        c.technologies_total += 1;
        c.technologies_native += (live && t.effective >= L::Native) as u64;
        c.technologies_proven += (live && t.effective >= L::Proven) as u64;
        c.technologies_canonical += (live && t.effective >= L::Canonical) as u64;
    }
    c.unmapped_nodes = a
        .shape
        .findings
        .iter()
        .filter(|f| f.code == "UNOWNED_CODE")
        .count() as u64;
    c.canonical_nodes_total = a.shape.nodes_total;
    c.canonical_nodes_conformant = a.shape.nodes_conformant;
    c.documents_total = a.docs.docs.len() as u64;
    c.documents_over_budget = a.docs.over_budget().count() as u64;
    c.shape_units = a.shape.units;
    c.shape_units_conformant = a.shape.conformant;
    c
}

/// The default repository root: the parent of the subsystem this binary was built from.
pub fn default_root() -> PathBuf {
    protocol::own_subsystem_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
