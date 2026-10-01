//! A repository's Ynventa knowledge (ADR 0104): the typed facts that replaced its Markdown
//! documents, read from `.ynventa/knowledge/*.ynv` (protocol v1). Read-only and dependency-free.
//!
//! Record layout (`.ynventa/src/compact/{codec,facts}.rs`): the magic `YNV`, format byte 1, record
//! tag 4, a LEB128 fact count, then per fact a kind rank byte, subject, key and value
//! (length-prefixed UTF-8), a provenance list, a LEB128 seq and a superseded-values list. Batches
//! fold by identity (kind, subject, key): the highest seq wins and provenances unite.

use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, io};

/// Where a repository keeps its knowledge batches.
pub const KNOWLEDGE_DIR: &str = ".ynventa/knowledge";

const MAGIC: &[u8; 3] = b"YNV";
const FORMAT: u8 = 1;
const KNOWLEDGE_TAG: u8 = 4;

/// The fact vocabulary of Ynventa protocol v1, in rank order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FactKind {
    Statement,
    Definition,
    Decision,
    LegacyClaim,
    LegacyRecord,
    Document,
    Milestone,
}

impl FactKind {
    const ALL: [FactKind; 7] = [
        FactKind::Statement,
        FactKind::Definition,
        FactKind::Decision,
        FactKind::LegacyClaim,
        FactKind::LegacyRecord,
        FactKind::Document,
        FactKind::Milestone,
    ];
}

/// One current fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub kind: FactKind,
    pub subject: String,
    pub key: String,
    pub value: String,
    pub provenance: Vec<String>,
    pub seq: u64,
}

/// The folded knowledge of a repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Knowledge {
    pub facts: Vec<Fact>,
}

impl Knowledge {
    /// The repository's knowledge; an absent knowledge directory is empty knowledge, an unreadable
    /// batch is an error (never silently skipped).
    pub fn read(root: impl AsRef<Path>) -> io::Result<Knowledge> {
        let dir = root.as_ref().join(KNOWLEDGE_DIR);
        let Ok(entries) = fs::read_dir(&dir) else {
            return Ok(Knowledge::default());
        };
        let mut names: Vec<_> = entries
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "ynv"))
            .collect();
        names.sort();
        let mut folded: BTreeMap<(FactKind, String, String), Fact> = BTreeMap::new();
        for path in names {
            let bytes = fs::read(&path)?;
            let facts = decode(&bytes).map_err(|why| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: {why}", path.display()),
                )
            })?;
            for fact in facts {
                let id = (fact.kind, fact.subject.clone(), fact.key.clone());
                match folded.get_mut(&id) {
                    Some(current) => {
                        let mut provenance = std::mem::take(&mut current.provenance);
                        provenance.extend(fact.provenance.iter().cloned());
                        if (fact.seq, &fact.value) > (current.seq, &current.value) {
                            *current = fact;
                        }
                        provenance.sort();
                        provenance.dedup();
                        current.provenance = provenance;
                    }
                    None => {
                        folded.insert(id, fact);
                    }
                }
            }
        }
        Ok(Knowledge {
            facts: folded.into_values().collect(),
        })
    }

    /// Current facts of one kind.
    pub fn of(&self, kind: FactKind) -> impl Iterator<Item = &Fact> {
        self.facts.iter().filter(move |f| f.kind == kind)
    }

    /// The documents whose knowledge was extracted: path -> content digest.
    pub fn documents(&self) -> BTreeMap<&str, &str> {
        self.of(FactKind::Document)
            .filter(|f| f.key == "digest")
            .map(|f| (f.subject.as_str(), f.value.as_str()))
            .collect()
    }

    /// Whether `path` is a document whose knowledge was extracted.
    pub fn has_document(&self, path: &str) -> bool {
        self.of(FactKind::Document)
            .any(|f| f.key == "digest" && f.subject == path)
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Decoder<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let b = *self.bytes.get(self.at).ok_or("truncated byte")?;
        self.at += 1;
        Ok(b)
    }
    fn u64(&mut self) -> Result<u64, String> {
        let (mut v, mut shift) = (0u64, 0u32);
        loop {
            let b = self.u8()?;
            if shift >= 64 {
                return Err("integer overflow".into());
            }
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            shift += 7;
        }
    }
    fn len(&mut self) -> Result<usize, String> {
        let n = usize::try_from(self.u64()?).map_err(|_| "length overflow")?;
        if n > self.bytes.len().saturating_sub(self.at) {
            return Err("length exceeds record".into());
        }
        Ok(n)
    }
    fn str(&mut self) -> Result<String, String> {
        let n = self.len()?;
        let s = std::str::from_utf8(&self.bytes[self.at..self.at + n])
            .map_err(|_| "string is not UTF-8")?
            .to_owned();
        self.at += n;
        Ok(s)
    }
    fn strs(&mut self) -> Result<Vec<String>, String> {
        let n = self.u64()?;
        (0..n).map(|_| self.str()).collect()
    }
}

/// Decodes one knowledge batch.
pub fn decode(bytes: &[u8]) -> Result<Vec<Fact>, String> {
    if bytes.len() < 5 || &bytes[..3] != MAGIC {
        return Err("not a Ynventa record".into());
    }
    if bytes[3] != FORMAT {
        return Err(format!("unsupported record format {}", bytes[3]));
    }
    if bytes[4] != KNOWLEDGE_TAG {
        return Err(format!("record tag {} is not knowledge", bytes[4]));
    }
    let mut d = Decoder { bytes, at: 5 };
    let count = d.u64()?;
    let mut facts = Vec::new();
    for _ in 0..count {
        let rank = d.u8()?;
        let kind = *FactKind::ALL
            .get(usize::from(rank))
            .ok_or_else(|| format!("unknown fact kind rank {rank}"))?;
        let subject = d.str()?;
        let key = d.str()?;
        let value = d.str()?;
        let provenance = d.strs()?;
        let seq = d.u64()?;
        let _superseded = d.strs()?;
        facts.push(Fact {
            kind,
            subject,
            key,
            value,
            provenance,
            seq,
        });
    }
    if d.at != bytes.len() {
        return Err("trailing bytes after the last fact".into());
    }
    Ok(facts)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn leb(out: &mut Vec<u8>, mut v: u64) {
        loop {
            let byte = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }

    fn string(out: &mut Vec<u8>, s: &str) {
        leb(out, s.len() as u64);
        out.extend_from_slice(s.as_bytes());
    }

    /// One encoded fact: (kind rank, subject, key, value, provenance, seq).
    pub(crate) type Row<'a> = (u8, &'a str, &'a str, &'a str, &'a [&'a str], u64);

    /// Encodes facts exactly as Ynventa protocol v1 writes a knowledge batch
    /// (`.ynventa/src/compact/facts.rs`).
    pub(crate) fn encode(facts: &[Row<'_>]) -> Vec<u8> {
        let mut out = b"YNV".to_vec();
        out.extend([FORMAT, KNOWLEDGE_TAG]);
        leb(&mut out, facts.len() as u64);
        for (rank, subject, key, value, provenance, seq) in facts {
            out.push(*rank);
            string(&mut out, subject);
            string(&mut out, key);
            string(&mut out, value);
            leb(&mut out, provenance.len() as u64);
            for p in *provenance {
                string(&mut out, p);
            }
            leb(&mut out, *seq);
            leb(&mut out, 0);
        }
        out
    }

    #[test]
    fn a_batch_decodes_fact_by_fact_and_malformed_bytes_are_refused() {
        let bytes = encode(&[
            (
                5,
                ".atlas/README.md",
                "digest",
                "sha256:aa",
                &["doc:.atlas/README.md"],
                3,
            ),
            (
                1,
                ".atlas/README.md",
                "id",
                "atlas.readme",
                &["doc:.atlas/README.md#L2"],
                3,
            ),
        ]);
        let facts = decode(&bytes).unwrap();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].kind, FactKind::Document);
        assert_eq!(facts[0].value, "sha256:aa");
        assert_eq!(facts[1].kind, FactKind::Definition);
        assert_eq!(
            facts[1].provenance,
            vec!["doc:.atlas/README.md#L2".to_owned()]
        );
        assert!(decode(b"YNV").is_err(), "truncated header");
        assert!(decode(&bytes[..bytes.len() - 1]).is_err(), "truncated fact");
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode(&trailing).is_err(), "trailing bytes");
        let mut wrong_tag = bytes.clone();
        wrong_tag[4] = 3;
        assert!(decode(&wrong_tag).is_err(), "not a knowledge record");
        assert!(
            decode(&encode(&[(9, "s", "k", "v", &[], 1)])).is_err(),
            "unknown kind rank"
        );
        assert_eq!(decode(&encode(&[])).unwrap(), Vec::new());
    }

    #[test]
    fn batches_fold_to_the_newest_value_with_every_provenance() {
        let dir = std::env::temp_dir().join(format!(
            "atlas-knowledge-fold-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let knowledge = dir.join(KNOWLEDGE_DIR);
        fs::create_dir_all(&knowledge).unwrap();
        fs::write(
            knowledge.join("a.ynv"),
            encode(&[(1, "s", "k", "old", &["p1"], 1)]),
        )
        .unwrap();
        fs::write(
            knowledge.join("b.ynv"),
            encode(&[
                (1, "s", "k", "new", &["p2"], 2),
                (5, "d.md", "digest", "x", &[], 2),
            ]),
        )
        .unwrap();
        let read = Knowledge::read(&dir).unwrap();
        let fact = read.of(FactKind::Definition).next().unwrap();
        assert_eq!(fact.value, "new");
        assert_eq!(fact.provenance, vec!["p1".to_owned(), "p2".to_owned()]);
        assert!(read.has_document("d.md") && !read.has_document("s"));
        fs::write(knowledge.join("c.ynv"), b"not knowledge").unwrap();
        assert!(
            Knowledge::read(&dir).is_err(),
            "an unreadable batch is never skipped"
        );
        assert_eq!(
            Knowledge::read(dir.join("absent")).unwrap(),
            Knowledge::default()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The repository's own knowledge is readable, and the Atlas control documents it records are
    /// the documents `.atlas` held when they were extracted (ADR 0104).
    #[test]
    fn this_repository_knowledge_is_readable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let knowledge = Knowledge::read(&root).unwrap();
        assert!(!knowledge.facts.is_empty());
        assert!(knowledge.has_document(".atlas/README.md"));
    }
}
