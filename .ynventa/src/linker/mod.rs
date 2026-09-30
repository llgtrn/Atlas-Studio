//! The Ynventa system linker: repository capsules → `chronica.system.ynv`.
//!
//! Like a compiler's linker, but for architecture and technology. Shards are physical; the
//! linked graph is the one logical Chronica. Invalid architecture is a link error, not a note:
//! unresolved references, required capabilities nobody provides, two shards owning one node,
//! code dependencies across shards (only REQUIRES and REUSES may cross), duplicate capability
//! providers or technologies without a declared relation, cycles, protocol mismatch.

use crate::capsule::Capsule;
use crate::compact::codec::{DecodeError, Decoder, Encoder};
use crate::formats::json::Json;
use crate::graph::{Graph, NodeId, SYSTEM};
use crate::metrics::Counts;
use crate::schema::{EdgeKind, NodeKind, Scope, TechnologyLifecycle};
use crate::Severity;
use std::collections::{BTreeMap, BTreeSet};

pub const SYSTEM_TAG: u8 = 7;
pub const SYSTEM_FILE: &str = "chronica.system.ynv";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkIssue {
    pub severity: Severity,
    pub code: String,
    pub subject: String,
    pub detail: String,
}

impl LinkIssue {
    fn new(severity: Severity, code: &str, subject: &str, detail: impl Into<String>) -> LinkIssue {
        LinkIssue {
            severity,
            code: code.into(),
            subject: subject.into(),
            detail: detail.into(),
        }
    }
}

/// (fingerprint, signature, [(shard, semantic key)]): one operation or type shape found in
/// several shards.
pub type SymbolGroup = (String, String, Vec<(String, String)>);

/// One technology of the linked system.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemTechnology {
    pub key: String,
    /// The shard where its canonical source lives (birthplace; not its owner — Chronica is).
    pub birthplace: String,
    pub effective: TechnologyLifecycle,
    pub implements: Vec<String>,
    pub source_digest: String,
    /// Shards (other than the birthplace) that reuse it natively.
    pub consumers: Vec<String>,
}

/// One capability of the linked system.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemCapability {
    pub key: String,
    /// (shard, node key)
    pub providers: Vec<(String, String)>,
    pub requirers: Vec<(String, String)>,
    /// Technology keys implementing it.
    pub technologies: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SystemImage {
    pub schema: String,
    /// (shard, origin, head, capsule digest)
    pub shards: Vec<(String, String, String, String)>,
    pub graph: Graph,
    pub technologies: Vec<SystemTechnology>,
    pub capabilities: Vec<SystemCapability>,
    pub issues: Vec<LinkIssue>,
    pub counts: Vec<(String, u64)>,
    /// Groups of same-shaped public operations found in several shards: (fingerprint,
    /// signature, [(shard, semantic key)]). Candidates for one canonical technology.
    pub duplicate_symbols: Vec<SymbolGroup>,
}

impl SystemImage {
    pub fn pass(&self) -> bool {
        !self.issues.iter().any(|i| i.severity == Severity::Error)
    }
    pub fn counts(&self) -> Counts {
        Counts::from_raw(&self.counts)
    }
}

const COMMON_NAMES: &[&str] = &[
    "new", "default", "from", "into", "fmt", "main", "clone", "len", "is_empty", "get", "set",
    "run", "build", "parse", "open", "close", "read", "write", "with", "name", "value", "key",
    "id", "apply", "check", "render", "load", "store", "encode", "decode", "hash", "eq", "cmp",
];

/// Links capsules into the Chronica system image.
pub fn link(capsules: &[Capsule]) -> SystemImage {
    let mut issues = Vec::new();
    let schema = crate::protocol::schema_identity();
    let mut shards = Vec::new();
    let mut seen_shards = BTreeSet::new();
    for c in capsules {
        if c.schema != schema || c.protocol != crate::YNVENTA_PROTOCOL_VERSION {
            issues.push(LinkIssue::new(
                Severity::Error,
                "PROTOCOL_MISMATCH",
                &c.shard,
                format!(
                    "capsule schema {} v{}; linker {} v{}",
                    c.schema,
                    c.protocol,
                    schema,
                    crate::YNVENTA_PROTOCOL_VERSION
                ),
            ));
        }
        if c.system != SYSTEM {
            issues.push(LinkIssue::new(
                Severity::Error,
                "FOREIGN_SYSTEM",
                &c.shard,
                format!("belongs to `{}`, not `{SYSTEM}`", c.system),
            ));
        }
        if crate::protocol::shard(&c.shard).is_none() {
            issues.push(LinkIssue::new(
                Severity::Error,
                "UNKNOWN_SHARD",
                &c.shard,
                "not one of the canonical shards of Chronica",
            ));
        }
        if !seen_shards.insert(c.shard.clone()) {
            issues.push(LinkIssue::new(
                Severity::Error,
                "DUPLICATE_SHARD",
                &c.shard,
                "linked twice",
            ));
        }
        let digest = crate::digest::content_digest(&c.encode());
        shards.push((c.shard.clone(), c.origin.clone(), c.head.clone(), digest));
    }
    shards.sort();
    let missing: Vec<&str> = crate::protocol::SHARDS
        .iter()
        .map(|s| s.id)
        .filter(|s| *s != "ynventa" && !seen_shards.contains(*s))
        .collect();
    if !missing.is_empty() {
        issues.push(LinkIssue::new(
            Severity::Warning,
            "INCOMPLETE_SYSTEM",
            "chronica",
            format!("shards not linked: {}", missing.join(", ")),
        ));
    }

    // Ownership: a Chronica node lives in exactly one shard.
    let mut owner: BTreeMap<NodeId, String> = BTreeMap::new();
    for c in capsules {
        for n in c.graph.nodes.values() {
            let owned = n.namespace == SYSTEM
                && n.repository == c.shard
                && (crate::schema::is_physical(n.kind) || n.kind == NodeKind::Technology);
            if !owned || n.kind == NodeKind::Repository {
                continue;
            }
            if let Some(prev) = owner.insert(n.id, c.shard.clone()) {
                if prev != c.shard {
                    issues.push(LinkIssue::new(Severity::Error, "OWNERSHIP_COLLISION", &n.semantic_key, format!("owned by both `{prev}` and `{}`; one semantic identity has one physical owner", c.shard)));
                }
            }
        }
    }

    let mut graph = Graph::default();
    for c in capsules {
        for i in graph.merge(&c.graph) {
            issues.push(LinkIssue::new(
                Severity::Error,
                i.code,
                &i.subject,
                i.detail,
            ));
        }
    }
    let label = |g: &Graph, id: &NodeId| {
        g.nodes
            .get(id)
            .map(|n| n.semantic_key.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let shard_of = |g: &Graph, id: &NodeId| {
        g.nodes
            .get(id)
            .map(|n| n.repository.clone())
            .unwrap_or_default()
    };

    // References.
    let unresolved: Vec<&crate::graph::GEdge> = graph
        .edges
        .iter()
        .filter(|e| !graph.nodes.contains_key(&e.from) || !graph.nodes.contains_key(&e.to))
        .collect();
    for e in &unresolved {
        let (known, missing) = if graph.nodes.contains_key(&e.from) {
            (e.from, e.to)
        } else {
            (e.to, e.from)
        };
        issues.push(LinkIssue::new(
            Severity::Error,
            "UNRESOLVED_REFERENCE",
            &label(&graph, &known),
            format!(
                "{} {} references {missing}, which no linked shard declares",
                shard_of(&graph, &known),
                e.kind
            ),
        ));
    }
    for i in graph.validate(&BTreeSet::new()) {
        if i.code != "DANGLING_EDGE" {
            issues.push(LinkIssue::new(
                Severity::Error,
                i.code,
                &i.subject,
                i.detail,
            ));
        }
    }

    // Only REQUIRES (run-time capability) and REUSES (materialized technology) may cross shards.
    for e in &graph.edges {
        if e.kind == EdgeKind::DependsOn && !matches!(e.scope, Scope::Architectural) {
            let (a, b) = (shard_of(&graph, &e.from), shard_of(&graph, &e.to));
            let both_chronica = graph
                .nodes
                .get(&e.from)
                .is_some_and(|n| n.namespace == SYSTEM)
                && graph
                    .nodes
                    .get(&e.to)
                    .is_some_and(|n| n.namespace == SYSTEM);
            if both_chronica && !a.is_empty() && !b.is_empty() && a != b {
                issues.push(LinkIssue::new(
                    Severity::Error,
                    "CROSS_SHARD_CODE_DEPENDENCY",
                    &label(&graph, &e.from),
                    format!("{a} depends ({}) on {} in {b}; shards couple only through REQUIRES or REUSES", e.scope, label(&graph, &e.to)),
                ));
            }
        }
    }

    // Capabilities.
    let related = |g: &Graph, x: NodeId, y: NodeId| {
        g.edges.iter().any(|e| {
            ((e.from == x && e.to == y) || (e.from == y && e.to == x))
                && matches!(
                    e.kind,
                    EdgeKind::Specializes
                        | EdgeKind::AlternativeFor
                        | EdgeKind::Evolves
                        | EdgeKind::Generalizes
                        | EdgeKind::Supersedes
                        | EdgeKind::Replaces
                        | EdgeKind::ForkedFrom
                        | EdgeKind::Merges
                )
        })
    };
    let mut caps: BTreeMap<String, SystemCapability> = BTreeMap::new();
    for n in graph
        .nodes
        .values()
        .filter(|n| n.kind == NodeKind::Capability && n.namespace == SYSTEM)
    {
        let Some(key) = n.semantic_key.strip_prefix("capability/") else {
            continue;
        };
        let mut sc = SystemCapability {
            key: key.to_string(),
            providers: vec![],
            requirers: vec![],
            technologies: vec![],
        };
        for e in graph.backlinks(n.id) {
            let Some(src) = graph.nodes.get(&e.from) else {
                continue;
            };
            match (e.kind, src.kind) {
                (EdgeKind::Provides, _) => sc
                    .providers
                    .push((src.repository.clone(), src.semantic_key.clone())),
                (EdgeKind::Requires, _) => sc
                    .requirers
                    .push((src.repository.clone(), src.semantic_key.clone())),
                (EdgeKind::Implements, NodeKind::Technology) => sc.technologies.push(
                    src.semantic_key
                        .trim_start_matches("technology/")
                        .to_string(),
                ),
                _ => {}
            }
        }
        sc.providers.sort();
        sc.requirers.sort();
        sc.technologies.sort();
        if !sc.requirers.is_empty() && sc.providers.is_empty() {
            issues.push(LinkIssue::new(
                Severity::Error,
                "UNPROVIDED_CAPABILITY",
                key,
                format!(
                    "required by {} but provided by no shard",
                    sc.requirers
                        .iter()
                        .map(|(s, n)| format!("{n} ({s})"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        let provider_shards: BTreeSet<&String> = sc.providers.iter().map(|(s, _)| s).collect();
        if provider_shards.len() > 1 {
            let ids: Vec<NodeId> = sc
                .providers
                .iter()
                .map(|(_, k)| NodeId::of(SYSTEM, k))
                .collect();
            let unrelated = ids
                .iter()
                .enumerate()
                .any(|(i, a)| ids.iter().skip(i + 1).any(|b| !related(&graph, *a, *b)));
            if unrelated {
                issues.push(LinkIssue::new(
                    Severity::Error,
                    "DUPLICATE_CAPABILITY",
                    key,
                    format!("provided independently by {}; declare SPECIALIZES / ALTERNATIVE_FOR or converge on one provider", sc.providers.iter().map(|(s, n)| format!("{n} ({s})")).collect::<Vec<_>>().join(", ")),
                ));
            }
        }
        caps.insert(key.to_string(), sc);
    }

    // Technologies: birthplace, adoption, duplication.
    let mut techs: Vec<SystemTechnology> = Vec::new();
    for c in capsules {
        for t in &c.technologies {
            let tid = NodeId::of(SYSTEM, &crate::graph::technology_key(&t.key));
            let mut consumers: Vec<String> = graph
                .backlinks(tid)
                .iter()
                .filter(|e| e.kind == EdgeKind::Reuses)
                .map(|e| shard_of(&graph, &e.from))
                .filter(|s| !s.is_empty() && *s != c.shard)
                .collect();
            consumers.sort();
            consumers.dedup();
            let mut effective = t.effective;
            if !consumers.is_empty() && effective == TechnologyLifecycle::Canonical {
                effective = TechnologyLifecycle::Adopted;
            }
            techs.push(SystemTechnology {
                key: t.key.clone(),
                birthplace: c.shard.clone(),
                effective,
                implements: t.implements.clone(),
                source_digest: t.source_digest.clone(),
                consumers,
            });
        }
    }
    techs.sort_by(|a, b| a.key.cmp(&b.key));
    for e in graph.edges.iter().filter(|e| e.kind == EdgeKind::Reuses) {
        let Some(t) = graph.nodes.get(&e.to) else {
            continue;
        };
        let key = t.semantic_key.trim_start_matches("technology/");
        if let Some(st) = techs.iter().find(|x| x.key == key) {
            if st.effective < TechnologyLifecycle::Canonical {
                issues.push(LinkIssue::new(
                    Severity::Warning,
                    "REUSE_OF_NON_CANONICAL",
                    key,
                    format!(
                        "{} reuses it while it is {}",
                        label(&graph, &e.from),
                        st.effective
                    ),
                ));
            }
        }
    }
    for (i, a) in techs.iter().enumerate() {
        for b in techs.iter().skip(i + 1) {
            if a.birthplace != b.birthplace && a.implements.iter().any(|x| b.implements.contains(x))
            {
                let ida = NodeId::of(SYSTEM, &crate::graph::technology_key(&a.key));
                let idb = NodeId::of(SYSTEM, &crate::graph::technology_key(&b.key));
                if !related(&graph, ida, idb) {
                    issues.push(LinkIssue::new(
                        Severity::Error,
                        "DUPLICATE_TECHNOLOGY",
                        &a.key,
                        format!("`{}` ({}) and `{}` ({}) implement the same capability with no declared relation", a.key, a.birthplace, b.key, b.birthplace),
                    ));
                }
            }
        }
    }

    // YIR: same-shaped public operations in several shards are technology candidates.
    let mut groups: BTreeMap<String, (String, BTreeSet<(String, String)>)> = BTreeMap::new();
    for c in capsules {
        for s in &c.symbols {
            let meaningful = match s.kind {
                crate::ir::SymbolKind::Operation => {
                    !s.inputs.is_empty()
                        && s.name.len() >= 5
                        && !COMMON_NAMES.contains(&s.name.as_str())
                }
                crate::ir::SymbolKind::Type | crate::ir::SymbolKind::Interface => s.name.len() >= 6,
                _ => false,
            };
            if meaningful {
                let g = groups
                    .entry(s.fingerprint())
                    .or_insert_with(|| (s.signature(), BTreeSet::new()));
                g.1.insert((c.shard.clone(), s.semantic_key()));
            }
        }
    }
    let mut duplicate_symbols: Vec<SymbolGroup> = groups
        .into_iter()
        .filter(|(_, (_, m))| m.iter().map(|(s, _)| s).collect::<BTreeSet<_>>().len() > 1)
        .map(|(f, (sig, m))| (f, sig, m.into_iter().collect()))
        .collect();
    duplicate_symbols.sort_by(|a, b| b.2.len().cmp(&a.2.len()).then(a.0.cmp(&b.0)));

    let mut total = Counts::default();
    for c in capsules {
        total.add(&c.counts());
    }
    total.technologies_adopted = techs
        .iter()
        .filter(|t| t.effective == TechnologyLifecycle::Adopted)
        .count() as u64;
    issues.sort();
    issues.dedup();
    SystemImage {
        schema,
        shards,
        graph,
        technologies: techs,
        capabilities: caps.into_values().collect(),
        issues,
        counts: total
            .raw()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        duplicate_symbols,
    }
}

impl SystemImage {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(SYSTEM_TAG);
        e.str(&self.schema);
        e.u64(self.shards.len() as u64);
        for (a, b, c, d) in &self.shards {
            e.str(a).str(b).str(c).str(d);
        }
        e.bytes(&self.graph.encode());
        e.u64(self.technologies.len() as u64);
        for t in &self.technologies {
            e.str(&t.key)
                .str(&t.birthplace)
                .u8(t.effective.rank())
                .strs(&t.implements)
                .str(&t.source_digest)
                .strs(&t.consumers);
        }
        e.u64(self.capabilities.len() as u64);
        let pairs = |e: &mut Encoder, v: &[(String, String)]| {
            e.u64(v.len() as u64);
            for (a, b) in v {
                e.str(a).str(b);
            }
        };
        for c in &self.capabilities {
            e.str(&c.key);
            pairs(&mut e, &c.providers);
            pairs(&mut e, &c.requirers);
            e.strs(&c.technologies);
        }
        e.u64(self.issues.len() as u64);
        for i in &self.issues {
            e.u8(i.severity as u8)
                .str(&i.code)
                .str(&i.subject)
                .str(&i.detail);
        }
        e.u64(self.counts.len() as u64);
        for (k, v) in &self.counts {
            e.str(k).u64(*v);
        }
        e.u64(self.duplicate_symbols.len() as u64);
        for (f, sig, m) in &self.duplicate_symbols {
            e.str(f).str(sig);
            pairs(&mut e, m);
        }
        e.finish()
    }

    pub fn decode(b: &[u8]) -> Result<SystemImage, DecodeError> {
        let mut d = Decoder::open(b, SYSTEM_TAG)?;
        let schema = d.str()?;
        let mut shards = Vec::new();
        for _ in 0..d.u64()? {
            shards.push((d.str()?, d.str()?, d.str()?, d.str()?));
        }
        let graph = Graph::decode(d.bytes()?)?;
        let mut technologies = Vec::new();
        for _ in 0..d.u64()? {
            technologies.push(SystemTechnology {
                key: d.str()?,
                birthplace: d.str()?,
                effective: d.word(TechnologyLifecycle::ALL)?,
                implements: d.strs()?,
                source_digest: d.str()?,
                consumers: d.strs()?,
            });
        }
        fn pairs(d: &mut Decoder) -> Result<Vec<(String, String)>, DecodeError> {
            let n = d.u64()?;
            (0..n).map(|_| Ok((d.str()?, d.str()?))).collect()
        }
        let mut capabilities = Vec::new();
        for _ in 0..d.u64()? {
            capabilities.push(SystemCapability {
                key: d.str()?,
                providers: pairs(&mut d)?,
                requirers: pairs(&mut d)?,
                technologies: d.strs()?,
            });
        }
        let mut issues = Vec::new();
        for _ in 0..d.u64()? {
            let sev = match d.u8()? {
                0 => Severity::Error,
                1 => Severity::Warning,
                _ => Severity::Info,
            };
            issues.push(LinkIssue {
                severity: sev,
                code: d.str()?,
                subject: d.str()?,
                detail: d.str()?,
            });
        }
        let mut counts = Vec::new();
        for _ in 0..d.u64()? {
            counts.push((d.str()?, d.u64()?));
        }
        let mut duplicate_symbols = Vec::new();
        for _ in 0..d.u64()? {
            duplicate_symbols.push((d.str()?, d.str()?, pairs(&mut d)?));
        }
        d.end()?;
        Ok(SystemImage {
            schema,
            shards,
            graph,
            technologies,
            capabilities,
            issues,
            counts,
            duplicate_symbols,
        })
    }

    /// The ecosystem report.
    pub fn render_text(&self) -> String {
        let c = self.counts();
        let v = c.values();
        let get = |k: &str| {
            v.iter()
                .find(|(n, _)| n == k)
                .map(|(_, x)| x.clone())
                .unwrap_or_default()
        };
        let errors = self
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let mut s = format!(
            "Chronica global system: linked shards {}/{} ({})\nsystem link: {}\nglobal nodes: {}\nglobal edges: {}\nglobal canonical technologies: {}\ncross-shard technology consumers: {}\nduplicate capabilities: {}\nduplicate operation/type candidates: {}\nunresolved graph references: {}\ntotal donors: {}\nextinct donors: {}\nglobal extinction ratio: {}\nexternal technology edges: {}\nlink errors: {errors}, warnings: {}\n",
            self.shards.len(),
            crate::protocol::SHARDS.len(),
            self.shards.iter().map(|x| x.0.as_str()).collect::<Vec<_>>().join(" "),
            if self.pass() { "PASS" } else { "FAIL" },
            self.graph.nodes.len(),
            self.graph.edges.len(),
            self.technologies.iter().filter(|t| t.effective >= TechnologyLifecycle::Canonical && t.effective != TechnologyLifecycle::Superseded).count(),
            self.technologies.iter().map(|t| t.consumers.len()).sum::<usize>(),
            self.issues.iter().filter(|i| i.code == "DUPLICATE_CAPABILITY").count(),
            self.duplicate_symbols.len(),
            self.issues.iter().filter(|i| i.code == "UNRESOLVED_REFERENCE").count(),
            get("donors_registered"),
            get("donors_extinct"),
            get("extinction_ratio"),
            get("external_technology_edges"),
            self.issues.iter().filter(|i| i.severity == Severity::Warning).count(),
        );
        let mut by_code: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for i in &self.issues {
            *by_code
                .entry((i.severity.wire(), i.code.as_str()))
                .or_default() += 1;
        }
        for ((sev, code), n) in by_code {
            s.push_str(&format!("  {sev:<7} {code:<28} {n}\n"));
            for i in self.issues.iter().filter(|i| i.code == code).take(3) {
                s.push_str(&format!("      {}: {}\n", i.subject, i.detail));
            }
        }
        s
    }

    pub fn to_json(&self) -> Json {
        Json::obj()
            .with("schema", &self.schema)
            .with("pass", self.pass())
            .with(
                "shards",
                Json::Array(
                    self.shards
                        .iter()
                        .map(|(a, b, c, d)| {
                            Json::obj()
                                .with("shard", a)
                                .with("origin", b)
                                .with("head", c)
                                .with("capsule", d)
                        })
                        .collect(),
                ),
            )
            .with("metrics", self.counts().to_json())
            .with(
                "technologies",
                Json::Array(
                    self.technologies
                        .iter()
                        .map(|t| {
                            Json::obj()
                                .with("key", &t.key)
                                .with("birthplace", &t.birthplace)
                                .with("effective", t.effective.wire())
                                .with("implements", t.implements.clone())
                                .with("consumers", t.consumers.clone())
                        })
                        .collect(),
                ),
            )
            .with(
                "capabilities",
                Json::Array(
                    self.capabilities
                        .iter()
                        .map(|c| {
                            Json::obj()
                                .with("key", &c.key)
                                .with(
                                    "providers",
                                    c.providers
                                        .iter()
                                        .map(|(s, n)| format!("{n}@{s}"))
                                        .collect::<Vec<_>>(),
                                )
                                .with(
                                    "requirers",
                                    c.requirers
                                        .iter()
                                        .map(|(s, n)| format!("{n}@{s}"))
                                        .collect::<Vec<_>>(),
                                )
                                .with("technologies", c.technologies.clone())
                        })
                        .collect(),
                ),
            )
            .with(
                "issues",
                Json::Array(
                    self.issues
                        .iter()
                        .map(|i| {
                            Json::obj()
                                .with("severity", i.severity.wire())
                                .with("code", &i.code)
                                .with("subject", &i.subject)
                                .with("detail", &i.detail)
                        })
                        .collect(),
                ),
            )
            .with(
                "duplicate_symbols",
                Json::Array(
                    self.duplicate_symbols
                        .iter()
                        .map(|(f, sig, m)| {
                            Json::obj()
                                .with("fingerprint", f)
                                .with("signature", sig)
                                .with(
                                    "members",
                                    m.iter()
                                        .map(|(s, k)| format!("{k}@{s}"))
                                        .collect::<Vec<_>>(),
                                )
                        })
                        .collect(),
                ),
            )
    }
}
