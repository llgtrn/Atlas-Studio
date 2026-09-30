//! Census history: content-addressed batches in `.ynventa/history/`. Each census adds a batch;
//! concurrent branches add different files and never conflict; `compact` folds all batches into
//! one. The union of every batch's registered donors is the denominator floor: a donor, once
//! registered, is counted forever.

use super::codec::{DecodeError, Decoder, Encoder};
use super::write_addressed;
use crate::schema::DonorState;
use std::collections::BTreeSet;
use std::path::Path;

pub const HISTORY_DIR: &str = ".ynventa/history";
pub const HISTORY_TAG: u8 = 3;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row {
    pub seq: u64,
    pub commit: String,
    pub states: Vec<(String, DonorState)>,
    /// Metric name → canonical printed value.
    pub metrics: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batch {
    pub registered: BTreeSet<String>,
    /// Donors observed participating (any edge, import or resident source) at some census.
    pub active: BTreeSet<String>,
    pub rows: Vec<Row>,
}

impl Batch {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(HISTORY_TAG);
        e.strs(&self.registered.iter().cloned().collect::<Vec<_>>());
        e.strs(&self.active.iter().cloned().collect::<Vec<_>>());
        let mut rows = self.rows.clone();
        rows.sort();
        rows.dedup();
        e.u64(rows.len() as u64);
        for r in &rows {
            e.u64(r.seq).str(&r.commit);
            e.u64(r.states.len() as u64);
            for (k, s) in &r.states {
                e.str(k).u8(s.rank());
            }
            e.u64(r.metrics.len() as u64);
            for (k, v) in &r.metrics {
                e.str(k).str(v);
            }
        }
        e.finish()
    }

    pub fn decode(b: &[u8]) -> Result<Batch, DecodeError> {
        let mut d = Decoder::open(b, HISTORY_TAG)?;
        let registered = d.strs()?.into_iter().collect();
        let active = d.strs()?.into_iter().collect();
        let mut rows = Vec::new();
        for _ in 0..d.u64()? {
            let seq = d.u64()?;
            let commit = d.str()?;
            let mut states = Vec::new();
            for _ in 0..d.u64()? {
                states.push((d.str()?, d.word(DonorState::ALL)?));
            }
            let mut metrics = Vec::new();
            for _ in 0..d.u64()? {
                metrics.push((d.str()?, d.str()?));
            }
            rows.push(Row {
                seq,
                commit,
                states,
                metrics,
            });
        }
        d.end()?;
        Ok(Batch {
            registered,
            active,
            rows,
        })
    }
}

/// All history of a repository.
#[derive(Clone, Debug, Default)]
pub struct History {
    pub registered: BTreeSet<String>,
    pub active: BTreeSet<String>,
    pub rows: Vec<Row>,
    pub files: Vec<String>,
    pub unreadable: Vec<String>,
}

impl History {
    pub fn load(root: &Path) -> History {
        let mut h = History::default();
        for (name, bytes) in super::read_addressed(root, HISTORY_DIR, &mut h.unreadable) {
            match Batch::decode(&bytes) {
                Ok(b) => {
                    h.registered.extend(b.registered);
                    h.active.extend(b.active);
                    h.rows.extend(b.rows);
                    h.files.push(name);
                }
                Err(e) => h.unreadable.push(format!("{HISTORY_DIR}/{name}: {e}")),
            }
        }
        h.rows.sort();
        h.rows.dedup();
        h
    }

    pub fn next_seq(&self) -> u64 {
        self.rows.iter().map(|r| r.seq).max().map_or(1, |m| m + 1)
    }

    /// Appends one census as a new batch file.
    pub fn record(root: &Path, batch: &Batch) -> std::io::Result<String> {
        write_addressed(root, HISTORY_DIR, &batch.encode())
    }

    /// Folds every batch into one file; returns (files removed, file written).
    pub fn compact(root: &Path) -> std::io::Result<(usize, Option<String>)> {
        let h = History::load(root);
        if h.files.len() <= 1 {
            return Ok((0, h.files.first().map(|f| format!("{HISTORY_DIR}/{f}"))));
        }
        let folded = Batch {
            registered: h.registered.clone(),
            active: h.active.clone(),
            rows: h.rows.clone(),
        };
        let written = write_addressed(root, HISTORY_DIR, &folded.encode())?;
        let mut removed = 0;
        for f in &h.files {
            let p = format!("{HISTORY_DIR}/{f}");
            if p != written {
                std::fs::remove_file(root.join(&p))?;
                removed += 1;
            }
        }
        Ok((removed, Some(written)))
    }
}
