//! The Chronica technology graph, locally: effective technology lifecycles, improvement claims,
//! duplicate detection within a shard, and the source identity used to materialize a technology
//! into another shard without forking it.
//!
//! A technology is HOW Chronica natively does something (a capability is WHAT). Its identity is
//! Chronica's; the shard that declares it is only its birthplace.

use crate::declare::{Declaration, Technology};
use crate::digest::{hex, Sha256};
use crate::evidence::{judge, Store, Verdict};
use crate::repository::files::Files;
use crate::schema::{EdgeKind, NativeStatus, TechnologyLifecycle};
use crate::{Finding, Severity};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct TechnologyAssessment {
    pub key: String,
    pub claimed: TechnologyLifecycle,
    /// Effective lifecycle as far as one shard can judge (ADOPTED is decided by the linker).
    pub effective: TechnologyLifecycle,
    pub stopped_by: String,
    pub proofs: Vec<(String, Verdict)>,
    /// (dimension, statement, proven)
    pub claims: Vec<(String, String, bool)>,
    /// Digest over the canonical sources: the identity materialized copies are checked against.
    pub source_digest: String,
}

/// Digest of a technology's canonical sources (paths and bytes).
pub fn source_digest(files: &Files, t: &Technology) -> String {
    let mut h = Sha256::new();
    let mut sources = t.sources.clone();
    sources.sort();
    for s in &sources {
        h.field(s.as_bytes());
        h.field(&files.read_bytes(s).unwrap_or_default());
    }
    format!("sha256:{}", hex(&h.finish()))
}

pub fn assess(
    d: &Declaration,
    files: &Files,
    store: &Store,
    node_status: &BTreeMap<String, NativeStatus>,
    findings: &mut Vec<Finding>,
) -> Vec<TechnologyAssessment> {
    let superseded: Vec<&str> = d
        .technologies
        .iter()
        .flat_map(|t| t.relations.iter())
        .filter(|r| {
            matches!(
                r.kind,
                EdgeKind::Evolves | EdgeKind::Supersedes | EdgeKind::Replaces
            )
        })
        .map(|r| r.target.as_str())
        .collect();
    let mut out = Vec::new();
    for t in &d.technologies {
        let subject = format!("technology/{}", t.key);
        let proofs: Vec<(String, Verdict)> = t
            .proofs
            .iter()
            .map(|p| (p.locator.clone(), judge(store, files, d, p, Some(&subject))))
            .collect();
        let node = d.node(&t.node);
        let exists = node.is_some_and(|n| !n.path.is_empty() && files.exists(&n.path))
            && !t.sources.is_empty()
            && t.sources.iter().all(|s| files.paths.contains(s));
        let node_native = node_status.get(&t.node) == Some(&NativeStatus::Native);
        let native = node_native || self_contained(files, t);
        let proven = !proofs.is_empty() && proofs.iter().all(|(_, v)| *v == Verdict::Pass);
        let unrelated_twin = d.technologies.iter().find(|o| {
            o.key != t.key
                && o.implements.iter().any(|c| t.implements.contains(c))
                && !related(&d.technologies, t, o)
        });
        let ladder = [
            (
                TechnologyLifecycle::Experimental,
                exists,
                "its node and every canonical source must exist".to_string(),
            ),
            (
                TechnologyLifecycle::Native,
                native,
                format!(
                    "node `{}` is {} and its sources are not self-contained Rust",
                    t.node,
                    node_status
                        .get(&t.node)
                        .map(|s| s.wire())
                        .unwrap_or("undeclared")
                ),
            ),
            (
                TechnologyLifecycle::Proven,
                proven,
                format!(
                    "proofs: {}",
                    if proofs.is_empty() {
                        "none declared".to_string()
                    } else {
                        proofs
                            .iter()
                            .map(|(l, v)| format!("{l} {}", v.wire()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ),
            ),
            (
                TechnologyLifecycle::Canonical,
                t.claimed >= TechnologyLifecycle::Canonical && unrelated_twin.is_none(),
                match unrelated_twin {
                    Some(o) => format!(
                        "`{}` implements the same capability with no declared relation",
                        o.key
                    ),
                    None => "canonical status is an explicit claim".into(),
                },
            ),
        ];
        let mut effective = TechnologyLifecycle::Idea;
        let mut stopped_by = String::new();
        for (state, ok, why) in ladder {
            if ok {
                effective = state;
            } else {
                stopped_by = format!("{state}: {why}");
                break;
            }
        }
        if superseded.contains(&t.key.as_str()) {
            effective = TechnologyLifecycle::Superseded;
        }
        let claim_ceiling = t.claimed.min(TechnologyLifecycle::Canonical);
        if claim_ceiling > effective && effective != TechnologyLifecycle::Superseded {
            findings.push(Finding::new(
                Severity::Error,
                "TECHNOLOGY_CLAIM_EXCEEDS_EVIDENCE",
                &t.key,
                &format!(
                    "claims {} but evidence supports {effective}; stopped at {stopped_by}",
                    t.claimed
                ),
            ));
        }
        if let Some(o) = unrelated_twin {
            findings.push(Finding::new(
                Severity::Error,
                "DUPLICATE_TECHNOLOGY",
                &t.key,
                &format!("`{}` implements the same capability; declare EVOLVES, SPECIALIZES or ALTERNATIVE_FOR, or converge", o.key),
            ));
        }
        let claims = t
            .claims
            .iter()
            .map(|c| {
                let ok = !c.evidence.is_empty()
                    && c.evidence
                        .iter()
                        .all(|p| judge(store, files, d, p, Some(&subject)) == Verdict::Pass);
                if !ok {
                    findings.push(Finding::new(
                        Severity::Info,
                        "IMPROVEMENT_UNPROVEN",
                        &t.key,
                        &format!(
                            "{} vs {}: \"{}\" has no fresh passing evidence",
                            c.dimension, c.baseline, c.statement
                        ),
                    ));
                }
                (c.dimension.wire().to_string(), c.statement.clone(), ok)
            })
            .collect();
        out.push(TechnologyAssessment {
            key: t.key.clone(),
            claimed: t.claimed,
            effective,
            stopped_by,
            proofs,
            claims,
            source_digest: source_digest(files, t),
        });
    }
    out
}

/// Whether every canonical source is Rust that stands alone: no foreign crate roots, native
/// links or processes, and no `crate::` reach into the enclosing node. Such a technology is
/// native even inside a node that wraps something else, because nothing it compiles is foreign.
pub fn self_contained(files: &Files, t: &Technology) -> bool {
    use crate::census::sources::{crate_roots, lex, links_and_processes, Tok};
    !t.sources.is_empty()
        && t.sources.iter().all(|s| {
            let Some(text) = s.ends_with(".rs").then(|| files.read(s)).flatten() else {
                return false;
            };
            let toks = lex(&text);
            let (links, processes) = links_and_processes(&toks);
            let reaches_crate = toks.windows(3).any(|w| {
                matches!(&w[0], Tok::Ident(x) if x == "crate")
                    && matches!(w[1], Tok::Punct(':'))
                    && matches!(w[2], Tok::Punct(':'))
            });
            crate_roots(&toks).is_empty()
                && links.is_empty()
                && processes.is_empty()
                && !reaches_crate
        })
}

/// Whether two technologies are connected through any chain of declared relations (in either
/// direction, possibly through technologies declared elsewhere): one declared family.
pub fn related(all: &[Technology], a: &Technology, b: &Technology) -> bool {
    let mut seen = std::collections::BTreeSet::from([a.key.as_str()]);
    let mut frontier = vec![a.key.as_str()];
    while let Some(k) = frontier.pop() {
        let out = all
            .iter()
            .filter(|t| t.key == k)
            .flat_map(|t| t.relations.iter().map(|r| r.target.as_str()));
        let inc = all
            .iter()
            .filter(|t| t.relations.iter().any(|r| r.target == k))
            .map(|t| t.key.as_str());
        for next in out.chain(inc).collect::<Vec<_>>() {
            if next == b.key {
                return true;
            }
            if seen.insert(next) {
                frontier.push(next);
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------------------------
// Materialization: technology reuse without runtime coupling and without silent forks.

use crate::compact::codec::{DecodeError, Decoder, Encoder};
use std::path::Path;

pub const MATERIALIZED_DIR: &str = ".ynventa/materialized";
pub const MATERIALIZED_TAG: u8 = 8;

/// A lock recording that canonical technology sources were compiled into this shard.
#[derive(Clone, Debug, PartialEq)]
pub struct Materialization {
    pub technology: String,
    pub birthplace: String,
    pub source_digest: String,
    /// (destination path here, canonical source path in the birthplace, content digest)
    pub files: Vec<(String, String, String)>,
}

impl Materialization {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(MATERIALIZED_TAG);
        e.str(&self.technology)
            .str(&self.birthplace)
            .str(&self.source_digest);
        e.u64(self.files.len() as u64);
        for (a, b, c) in &self.files {
            e.str(a).str(b).str(c);
        }
        e.finish()
    }
    pub fn decode(b: &[u8]) -> Result<Materialization, DecodeError> {
        let mut d = Decoder::open(b, MATERIALIZED_TAG)?;
        let technology = d.str()?;
        let birthplace = d.str()?;
        let source_digest = d.str()?;
        let mut files = Vec::new();
        for _ in 0..d.u64()? {
            files.push((d.str()?, d.str()?, d.str()?));
        }
        d.end()?;
        Ok(Materialization {
            technology,
            birthplace,
            source_digest,
            files,
        })
    }
    pub fn load_all(root: &Path, unreadable: &mut Vec<String>) -> Vec<Materialization> {
        crate::compact::read_addressed(root, MATERIALIZED_DIR, unreadable)
            .into_iter()
            .filter_map(|(n, b)| match Materialization::decode(&b) {
                Ok(m) => Some(m),
                Err(e) => {
                    unreadable.push(format!("{MATERIALIZED_DIR}/{n}: {e}"));
                    None
                }
            })
            .collect()
    }
}

/// Copies a technology's canonical sources from its birthplace shard into `into`, records the
/// lock, and returns it. The consumer compiles the sources natively; nothing runs remotely.
pub fn materialize(
    from_root: &Path,
    from: &crate::declare::Declaration,
    key: &str,
    to_root: &Path,
    into: &str,
) -> Result<Materialization, String> {
    let t = from
        .technology(key)
        .ok_or_else(|| format!("`{key}` is not declared by {}", from.repository.shard))?;
    let files = Files::scan(from_root).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for src in &t.sources {
        let bytes = files
            .read_bytes(src)
            .ok_or_else(|| format!("{src}: unreadable in {}", from.repository.shard))?;
        let name = src.rsplit('/').next().unwrap_or(src);
        let dest = format!("{}/{name}", into.trim_end_matches('/'));
        let p = to_root.join(&dest);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, &bytes).map_err(|e| e.to_string())?;
        out.push((dest, src.clone(), crate::digest::content_digest(&bytes)));
    }
    let m = Materialization {
        technology: key.to_string(),
        birthplace: from.repository.shard.clone(),
        source_digest: source_digest(&files, t),
        files: out,
    };
    crate::compact::write_addressed(to_root, MATERIALIZED_DIR, &m.encode())
        .map_err(|e| e.to_string())?;
    Ok(m)
}

/// A materialized copy edited locally is a silent fork: an error until it is re-materialized or
/// declared as its own technology with a relation to the canonical one.
pub fn check_materialized(files: &Files, locks: &[Materialization], findings: &mut Vec<Finding>) {
    for m in locks {
        for (dest, _, digest) in &m.files {
            match files.read_bytes(dest) {
                None => findings.push(Finding::new(Severity::Error, "MATERIALIZED_MISSING", &m.technology, &format!("`{dest}` was materialized and is gone"))),
                Some(b) if crate::digest::content_digest(&b) != *digest => findings.push(Finding::new(
                    Severity::Error,
                    "MATERIALIZED_FORK",
                    &m.technology,
                    &format!("`{dest}` differs from canonical {} ({}); re-materialize, or declare a technology that EVOLVES/SPECIALIZES it", m.technology, m.birthplace),
                )),
                _ => {}
            }
        }
    }
}
