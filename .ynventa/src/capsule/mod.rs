//! Repository capsules: `<shard>.ynv`. A capsule is one shard compiled into what the rest of
//! Chronica may know about it — graph, descriptor (owns / provides / requires / reuses), YIR
//! symbols, technologies with their source identities, donor states and raw metric counts —
//! deterministic, versioned and round-trippable. Linking capsules builds `chronica.system.ynv`.

use crate::compact::codec::{DecodeError, Decoder, Encoder};
use crate::declare::{Backend, NorlRelevance, Organism, OrganismCapability};
use crate::graph::{Graph, SYSTEM};
use crate::ir::{Symbol, SymbolKind};
use crate::metrics::Counts;
use crate::schema::{
    BackendKind, DonorState, EdgeKind, GrowthState, NodeKind, TechnologyKind, TechnologyLifecycle,
};
use crate::Assessment;

pub const CAPSULE_TAG: u8 = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct TechEntry {
    pub key: String,
    pub name: String,
    pub kind: TechnologyKind,
    pub purpose: String,
    pub node: String,
    pub effective: TechnologyLifecycle,
    pub implements: Vec<String>,
    pub sources: Vec<String>,
    pub source_digest: String,
    pub relations: Vec<(EdgeKind, String)>,
    pub lineage: Vec<String>,
    pub norl: NorlRelevance,
}

/// One donor of a shard as the rest of Chronica may know it.
#[derive(Clone, Debug, PartialEq)]
pub struct DonorEntry {
    pub key: String,
    /// Its node id (an OSS identity when the origin is known).
    pub id: String,
    pub claimed: DonorState,
    pub effective: DonorState,
    /// (capability key, maps_to) of each capability that declares a mapping: resolved by the
    /// system linker against the canonical graph.
    pub maps: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Capsule {
    pub protocol: u32,
    pub schema: String,
    /// Digest of the shard's installed subsystem (`.ynventa/`).
    pub subsystem: String,
    pub system: String,
    pub shard: String,
    pub origin: String,
    /// The commit the capsule was compiled from (empty outside Git).
    pub head: String,
    pub graph: Graph,
    pub symbols: Vec<Symbol>,
    pub technologies: Vec<TechEntry>,
    pub donors: Vec<DonorEntry>,
    pub counts: Vec<(String, u64)>,
    /// The organism declaration (empty outside the norl shard).
    pub organism: Organism,
    /// Structural violations of the shard's root (see `conformance::structure_violations`).
    pub structure: Vec<String>,
    /// Verdicts of the shard's evaluation materials: (material key, [(locator, verdict)]).
    pub evaluations: EvaluationRows,
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
                    name: t.name.clone(),
                    kind: t.kind,
                    purpose: t.purpose.clone(),
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
                    norl: t.norl.clone(),
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
                DonorEntry {
                    key: x.key.clone(),
                    id: crate::graph::NodeId::of(&ns, &key).to_string(),
                    claimed: x.claimed,
                    effective: x.effective,
                    maps: dn
                        .capabilities
                        .iter()
                        .filter_map(|c| Some((c.key.clone(), c.maps_to.clone()?)))
                        .collect(),
                }
            })
            .collect();
        Capsule {
            protocol: crate::YNVENTA_PROTOCOL_VERSION,
            schema: crate::protocol::schema_identity(),
            subsystem: crate::protocol::subsystem_digest(&a.root.join(".ynventa")),
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
            organism: d.organism.clone(),
            structure: crate::conformance::structure_violations(a),
            evaluations: a
                .evaluations
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
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
            .str(&self.subsystem)
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
                .str(&t.name)
                .u8(t.kind.rank())
                .str(&t.purpose)
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
            encode_norl(&mut e, &t.norl);
        }
        e.u64(self.donors.len() as u64);
        for x in &self.donors {
            e.str(&x.key)
                .str(&x.id)
                .str(x.claimed.wire())
                .str(x.effective.wire());
            e.u64(x.maps.len() as u64);
            for (c, m) in &x.maps {
                e.str(c).str(m);
            }
        }
        e.u64(self.counts.len() as u64);
        for (k, v) in &self.counts {
            e.str(k).u64(*v);
        }
        encode_organism(&mut e, &self.organism);
        e.strs(&self.structure);
        encode_evaluations(&mut e, &self.evaluations);
        e.finish()
    }

    pub fn decode(b: &[u8]) -> Result<Capsule, DecodeError> {
        let mut d = Decoder::open(b, CAPSULE_TAG)?;
        let protocol = d.u64()? as u32;
        let schema = d.str()?;
        let subsystem = d.str()?;
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
            let name = d.str()?;
            let kind = d.word(TechnologyKind::ALL)?;
            let purpose = d.str()?;
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
            let norl = decode_norl(&mut d)?;
            technologies.push(TechEntry {
                key,
                name,
                kind,
                purpose,
                node,
                effective,
                implements,
                sources,
                source_digest,
                relations,
                lineage,
                norl,
            });
        }
        let mut donors = Vec::new();
        for _ in 0..d.u64()? {
            let key = d.str()?;
            let id = d.str()?;
            let claimed = donor_state(&mut d)?;
            let effective = donor_state(&mut d)?;
            let mut maps = Vec::new();
            for _ in 0..d.u64()? {
                maps.push((d.str()?, d.str()?));
            }
            donors.push(DonorEntry {
                key,
                id,
                claimed,
                effective,
                maps,
            });
        }
        let mut counts = Vec::new();
        for _ in 0..d.u64()? {
            counts.push((d.str()?, d.u64()?));
        }
        let organism = decode_organism(&mut d)?;
        let structure = d.strs()?;
        let evaluations = decode_evaluations(&mut d)?;
        d.end()?;
        Ok(Capsule {
            protocol,
            schema,
            subsystem,
            system,
            shard,
            origin,
            head,
            graph,
            symbols,
            technologies,
            donors,
            counts,
            organism,
            structure,
            evaluations,
        })
    }
}

/// Evaluation verdicts in capsules and system images.
pub fn encode_evaluations(
    e: &mut Encoder,
    v: &[(String, Vec<(String, crate::evidence::Verdict)>)],
) {
    e.u64(v.len() as u64);
    for (k, rows) in v {
        e.str(k).u64(rows.len() as u64);
        for (loc, verdict) in rows {
            e.str(loc).str(verdict.wire());
        }
    }
}

pub type EvaluationRows = Vec<(String, Vec<(String, crate::evidence::Verdict)>)>;

pub fn decode_evaluations(d: &mut Decoder) -> Result<EvaluationRows, DecodeError> {
    let mut out = Vec::new();
    for _ in 0..d.u64()? {
        let k = d.str()?;
        let mut rows = Vec::new();
        for _ in 0..d.u64()? {
            let loc = d.str()?;
            let w = d.str()?;
            let v = crate::evidence::Verdict::ALL
                .iter()
                .copied()
                .find(|v| v.wire() == w)
                .ok_or_else(|| DecodeError(format!("`{w}` is not a verdict")))?;
            rows.push((loc, v));
        }
        out.push((k, rows));
    }
    Ok(out)
}

fn donor_state(d: &mut Decoder) -> Result<DonorState, DecodeError> {
    let w = d.str()?;
    DonorState::from_wire(&w).ok_or_else(|| DecodeError(format!("`{w}` is not a donor state")))
}

pub fn encode_norl(e: &mut Encoder, n: &NorlRelevance) {
    match n {
        NorlRelevance::Unresolved => e.u8(0),
        NorlRelevance::Feeds(k) => e.u8(1).str(k),
        NorlRelevance::NotRelevant(r) => e.u8(2).str(r),
    };
}

pub fn decode_norl(d: &mut Decoder) -> Result<NorlRelevance, DecodeError> {
    Ok(match d.u8()? {
        0 => NorlRelevance::Unresolved,
        1 => NorlRelevance::Feeds(d.str()?),
        2 => NorlRelevance::NotRelevant(d.str()?),
        t => return Err(DecodeError(format!("norl relevance tag {t}"))),
    })
}

fn opt(e: &mut Encoder, v: &Option<String>) {
    match v {
        None => e.bool(false),
        Some(s) => e.bool(true).str(s),
    };
}

fn decode_opt(d: &mut Decoder) -> Result<Option<String>, DecodeError> {
    Ok(if d.bool()? { Some(d.str()?) } else { None })
}

/// The organism declaration in capsules and system images.
pub fn encode_organism(e: &mut Encoder, o: &Organism) {
    e.u64(o.capabilities.len() as u64);
    for c in &o.capabilities {
        e.str(&c.key).str(&c.organ).str(c.claimed.wire());
        opt(e, &c.backend);
        e.strs(&c.evaluations);
    }
    e.u64(o.backends.len() as u64);
    for b in &o.backends {
        e.str(&b.key).str(b.kind.wire()).str(&b.node);
        opt(e, &b.donor);
        e.str(&b.weight);
    }
}

pub fn decode_organism(d: &mut Decoder) -> Result<Organism, DecodeError> {
    let word = |d: &mut Decoder| d.str();
    let mut o = Organism::default();
    for _ in 0..d.u64()? {
        let key = d.str()?;
        let organ = d.str()?;
        let w = word(d)?;
        let claimed = GrowthState::from_wire(&w)
            .ok_or_else(|| DecodeError(format!("`{w}` is not a growth state")))?;
        o.capabilities.push(OrganismCapability {
            key,
            organ,
            claimed,
            backend: decode_opt(d)?,
            evaluations: d.strs()?,
        });
    }
    for _ in 0..d.u64()? {
        let key = d.str()?;
        let w = word(d)?;
        let kind = BackendKind::from_wire(&w)
            .ok_or_else(|| DecodeError(format!("`{w}` is not a backend kind")))?;
        o.backends.push(Backend {
            key,
            kind,
            node: d.str()?,
            donor: decode_opt(d)?,
            weight: d.str()?,
        });
    }
    Ok(o)
}
