//! Repository capsules: `<shard>.ynv`. A capsule is one shard compiled into what the rest of
//! Chronica may know about it — graph, descriptor (owns / provides / requires / reuses), YIR
//! symbols, technologies with their source identities, donor states and raw metric counts —
//! deterministic, versioned and round-trippable. Linking capsules builds `chronica.system.ynv`.

use crate::compact::codec::{DecodeError, Decoder, Encoder};
use crate::graph::{Graph, SYSTEM};
use crate::ir::{Symbol, SymbolKind};
use crate::metrics::Counts;
use crate::schema::{DonorState, EdgeKind, NodeKind, TechnologyLifecycle};
use crate::Assessment;

pub const CAPSULE_TAG: u8 = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct TechEntry {
    pub key: String,
    pub node: String,
    pub effective: TechnologyLifecycle,
    pub implements: Vec<String>,
    pub sources: Vec<String>,
    pub source_digest: String,
    pub relations: Vec<(EdgeKind, String)>,
    pub lineage: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Capsule {
    pub protocol: u32,
    pub schema: String,
    pub system: String,
    pub shard: String,
    pub origin: String,
    /// The commit the capsule was compiled from (empty outside Git).
    pub head: String,
    pub graph: Graph,
    pub symbols: Vec<Symbol>,
    pub technologies: Vec<TechEntry>,
    /// (donor key, donor node id, effective state)
    pub donors: Vec<(String, String, DonorState)>,
    pub counts: Vec<(String, u64)>,
}

impl Capsule {
    pub fn compile(a: &Assessment) -> Capsule {
        let d = &a.declaration;
        let technologies = d
            .technologies
            .iter()
            .map(|t| {
                let x = a.technologies.iter().find(|x| x.key == t.key);
                TechEntry {
                    key: t.key.clone(),
                    node: t.node.clone(),
                    effective: x.map(|x| x.effective).unwrap_or(TechnologyLifecycle::Idea),
                    implements: t.implements.clone(),
                    sources: t.sources.clone(),
                    source_digest: x.map(|x| x.source_digest.clone()).unwrap_or_default(),
                    relations: t
                        .relations
                        .iter()
                        .map(|r| (r.kind, r.target.clone()))
                        .collect(),
                    lineage: t.lineage.clone(),
                }
            })
            .collect();
        let donors = a
            .analysis
            .donors
            .iter()
            .map(|x| {
                let dn = d.donor(&x.key).expect("assessed donors are declared");
                let (ns, key) = Graph::donor_id(&d.repository.shard, &dn.key, &dn.origin);
                (
                    x.key.clone(),
                    crate::graph::NodeId::of(&ns, &key).to_string(),
                    x.effective,
                )
            })
            .collect();
        Capsule {
            protocol: crate::YNVENTA_PROTOCOL_VERSION,
            schema: crate::protocol::schema_identity(),
            system: d.repository.system.clone(),
            shard: d.repository.shard.clone(),
            origin: d.repository.origin.clone(),
            head: crate::repository::files::head_commit(&a.root).unwrap_or_default(),
            graph: a.graph.clone(),
            symbols: crate::ir::compile(&a.files, d),
            technologies,
            donors,
            counts: a
                .counts
                .raw()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    pub fn counts(&self) -> Counts {
        Counts::from_raw(&self.counts)
    }

    pub fn file_name(&self) -> String {
        format!("{}.ynv", self.shard)
    }

    /// Capability keys this shard provides (through nodes it owns).
    pub fn provides(&self) -> Vec<String> {
        self.capability_edges(EdgeKind::Provides)
    }

    /// Capability keys this shard requires and does not provide itself.
    pub fn requires(&self) -> Vec<String> {
        let p = self.provides();
        self.capability_edges(EdgeKind::Requires)
            .into_iter()
            .filter(|c| !p.contains(c))
            .collect()
    }

    fn capability_edges(&self, kind: EdgeKind) -> Vec<String> {
        let mut v: Vec<String> = self
            .graph
            .edges
            .iter()
            .filter(|e| e.kind == kind)
            .filter(|e| {
                self.graph
                    .nodes
                    .get(&e.from)
                    .is_some_and(|n| n.repository == self.shard)
            })
            .filter_map(|e| self.graph.nodes.get(&e.to))
            .filter(|n| n.kind == NodeKind::Capability && n.namespace == SYSTEM)
            .filter_map(|n| {
                n.semantic_key
                    .strip_prefix("capability/")
                    .map(str::to_string)
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Keys of the Chronica nodes this shard owns (physically holds).
    pub fn owns(&self) -> Vec<String> {
        self.graph
            .nodes
            .values()
            .filter(|n| {
                n.repository == self.shard
                    && n.namespace == SYSTEM
                    && (crate::schema::is_physical(n.kind) || n.kind == NodeKind::Technology)
            })
            .map(|n| n.semantic_key.clone())
            .collect()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(CAPSULE_TAG);
        e.u64(self.protocol as u64)
            .str(&self.schema)
            .str(&self.system)
            .str(&self.shard)
            .str(&self.origin)
            .str(&self.head)
            .bytes(&self.graph.encode());
        e.u64(self.symbols.len() as u64);
        for s in &self.symbols {
            e.str(&s.node)
                .u8(s.kind.rank())
                .str(&s.name)
                .strs(&s.inputs)
                .str(&s.output)
                .str(&s.file);
        }
        e.u64(self.technologies.len() as u64);
        for t in &self.technologies {
            e.str(&t.key)
                .str(&t.node)
                .u8(t.effective.rank())
                .strs(&t.implements)
                .strs(&t.sources)
                .str(&t.source_digest);
            e.u64(t.relations.len() as u64);
            for (k, target) in &t.relations {
                e.u8(k.rank()).str(target);
            }
            e.strs(&t.lineage);
        }
        e.u64(self.donors.len() as u64);
        for (k, id, s) in &self.donors {
            e.str(k).str(id).u8(s.rank());
        }
        e.u64(self.counts.len() as u64);
        for (k, v) in &self.counts {
            e.str(k).u64(*v);
        }
        e.finish()
    }

    pub fn decode(b: &[u8]) -> Result<Capsule, DecodeError> {
        let mut d = Decoder::open(b, CAPSULE_TAG)?;
        let protocol = d.u64()? as u32;
        let schema = d.str()?;
        let system = d.str()?;
        let shard = d.str()?;
        let origin = d.str()?;
        let head = d.str()?;
        let graph = Graph::decode(d.bytes()?)?;
        let mut symbols = Vec::new();
        for _ in 0..d.u64()? {
            symbols.push(Symbol {
                node: d.str()?,
                kind: d.word(SymbolKind::ALL)?,
                name: d.str()?,
                inputs: d.strs()?,
                output: d.str()?,
                file: d.str()?,
            });
        }
        let mut technologies = Vec::new();
        for _ in 0..d.u64()? {
            let key = d.str()?;
            let node = d.str()?;
            let effective = d.word(TechnologyLifecycle::ALL)?;
            let implements = d.strs()?;
            let sources = d.strs()?;
            let source_digest = d.str()?;
            let mut relations = Vec::new();
            for _ in 0..d.u64()? {
                relations.push((d.word(EdgeKind::ALL)?, d.str()?));
            }
            let lineage = d.strs()?;
            technologies.push(TechEntry {
                key,
                node,
                effective,
                implements,
                sources,
                source_digest,
                relations,
                lineage,
            });
        }
        let mut donors = Vec::new();
        for _ in 0..d.u64()? {
            donors.push((d.str()?, d.str()?, d.word(DonorState::ALL)?));
        }
        let mut counts = Vec::new();
        for _ in 0..d.u64()? {
            counts.push((d.str()?, d.u64()?));
        }
        d.end()?;
        Ok(Capsule {
            protocol,
            schema,
            system,
            shard,
            origin,
            head,
            graph,
            symbols,
            technologies,
            donors,
            counts,
        })
    }
}
