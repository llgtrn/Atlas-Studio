//! Revision-stable function identity for self-recensus (G66, ADR 0028).
//!
//! Absorbed from SCIP (first-50 #7): a symbol is identified by a position-free, revision-free
//! descriptor chain -- package, namespaces, owners, name, suffix -- while source positions belong
//! to occurrences, never to identity (`scip.proto`, `Symbol` grammar and `Occurrence`).
//! `FunctionIdentity::identity_key` embeds the revision and the span (path:line:column), so an
//! inserted line gave every later function in the file a new identity, and a commit gave every
//! function one; descriptors do neither.
//!
//! SCIP has no cross-revision correspondence (code modification is a non-goal of its design), so
//! the correspondence below is Atlas-native: deterministic structural evidence only -- equal
//! descriptors, or equal (signature, body) token fingerprints -- and anything that evidence does
//! not decide uniquely is AMBIGUOUS, never forced.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One function or method at one revision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityState {
    /// `<package> <namespace>/…/<name>().` -- see `descriptor`.
    pub descriptor: String,
    /// The file that declares it. Informational: never part of the descriptor.
    pub path: String,
    /// Fingerprint of the signature without the function's name: parameter types, return type,
    /// generics, ABI, async/unsafe/extern, declaration kind and owner.
    pub signature: String,
    /// `FunctionSignature::body_fingerprint`, or `-` for a declaration without a body.
    pub body: String,
    pub visibility: String,
}

/// The signature without the function's name, position or revision -- what a rename or a move
/// keeps: parameter types, return type, generics, ABI, async/unsafe/extern, declaration kind and
/// owner (target type and trait). Parameter names are body-level and excluded.
pub fn signature_fingerprint(sig: &crate::FunctionSignature) -> String {
    let function = &sig.function;
    let parameter_types: Vec<&str> = sig
        .parameters
        .iter()
        .map(|p| p.type_identity.name.as_str())
        .collect();
    let text = format!(
        "params=[{}]|ret={}|generics=[{}]|abi={}|async={}|unsafe={}|extern={}|kind={}|owner={}|trait={}",
        parameter_types.join(","),
        sig.return_type.as_ref().map_or("", |t| t.name.as_str()),
        sig.generics.join(","),
        sig.abi.as_deref().unwrap_or(""),
        sig.is_async,
        sig.is_unsafe,
        sig.is_extern,
        function.declaration_kind.as_str(),
        function
            .owner
            .target
            .as_ref()
            .map_or("", |t| t.name.as_str()),
        function.owner.trait_path.as_deref().unwrap_or(""),
    );
    crate::identity::IntegrityDigest::of_bytes(text.as_bytes())
        .as_str()
        .to_owned()
}

/// Escapes one descriptor name the way SCIP does: a name made only of identifier characters is
/// written as is; anything else is backtick-quoted with backticks doubled.
fn name(segment: &str) -> String {
    if !segment.is_empty()
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-' | '$'))
    {
        segment.to_owned()
    } else {
        format!("`{}`", segment.replace('`', "``"))
    }
}

/// The module namespaces of a Rust source file, inferred from Cargo's layout: `package_dir` is the
/// nearest ancestor directory holding a `Cargo.toml`. `src/lib.rs`, `src/main.rs` and `mod.rs`
/// name their directory's module; any other `src/**/x.rs` names module `x`. A file outside `src/`
/// keeps its whole relative path as one quoted namespace, so it never collides with a module.
/// Limit: a `#[path]` attribute moves a module without this inference seeing it.
pub fn module_namespaces(package_dir: &str, path: &str) -> Vec<String> {
    let prefix = if package_dir.is_empty() {
        "src/".to_owned()
    } else {
        format!("{package_dir}/src/")
    };
    let Some(rest) = path.strip_prefix(&prefix) else {
        let relative = if package_dir.is_empty() {
            path
        } else {
            path.strip_prefix(&format!("{package_dir}/"))
                .unwrap_or(path)
        };
        return vec![name(relative)];
    };
    let mut parts: Vec<&str> = rest.split('/').collect();
    let file = parts.pop().unwrap_or_default();
    let mut namespaces: Vec<String> = parts.into_iter().map(name).collect();
    if !matches!(file, "lib.rs" | "main.rs" | "mod.rs") {
        namespaces.push(name(file.strip_suffix(".rs").unwrap_or(file)));
    }
    namespaces
}

/// `<package> <namespace>/…/<scope segment>/…/<name>().`: the package is its directory (`.` for
/// the root), then module namespaces, then the extractor's lexical scope segments (inline modules,
/// `impl:Type`, traits), then the method descriptor.
pub fn descriptor(package_dir: &str, path: &str, scope: &[String], function: &str) -> String {
    let package = if package_dir.is_empty() {
        ".".to_owned()
    } else {
        name(package_dir)
    };
    let mut out = format!("{package} ");
    for namespace in module_namespaces(package_dir, path) {
        out.push_str(&namespace);
        out.push('/');
    }
    for segment in scope {
        out.push_str(&name(segment));
        out.push('/');
    }
    out.push_str(&name(function));
    out.push_str("().");
    out
}

/// The nearest ancestor directory of `path` that holds a `Cargo.toml`, among `manifests`.
pub fn package_dir<'a>(path: &str, manifests: &'a BTreeSet<String>) -> Option<&'a str> {
    let mut dir = path;
    while let Some((parent, _)) = dir.rsplit_once('/') {
        if let Some(found) = manifests.get(parent) {
            return Some(found.as_str());
        }
        dir = parent;
    }
    manifests.get("").map(String::as_str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Correspondence {
    /// Same descriptor; signature, body or visibility differ.
    Changed,
    /// Same namespace, new name; identical signature and body.
    Renamed,
    /// Same name, new namespace; identical signature and body.
    Moved,
    MovedRenamed,
    Deleted,
    Created,
    /// Evidence does not decide one correspondence: a duplicated descriptor, or several
    /// candidates with identical fingerprints.
    Ambiguous,
}

impl Correspondence {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Changed => "CHANGED",
            Self::Renamed => "RENAMED",
            Self::Moved => "MOVED",
            Self::MovedRenamed => "MOVED_RENAMED",
            Self::Deleted => "DELETED",
            Self::Created => "CREATED",
            Self::Ambiguous => "AMBIGUOUS",
        }
    }
}

/// One non-SAME correspondence between two revisions' entities.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityChange {
    pub kind: Correspondence,
    pub before: Vec<String>,
    pub after: Vec<String>,
    /// Which evidence decided it (`signature,body,visibility` for CHANGED; the shared fingerprint
    /// for a match; the candidates for AMBIGUOUS).
    pub evidence: String,
}

impl EntityChange {
    /// The observed-change item a recensus intent must declare: `KIND before -> after`.
    pub fn item(&self) -> String {
        match (self.before.as_slice(), self.after.as_slice()) {
            ([], after) => format!("{} {}", self.kind.as_str(), after.join(" | ")),
            (before, []) => format!("{} {}", self.kind.as_str(), before.join(" | ")),
            (before, after) if before == after => {
                format!("{} {}", self.kind.as_str(), before.join(" | "))
            }
            (before, after) => format!(
                "{} {} -> {}",
                self.kind.as_str(),
                before.join(" | "),
                after.join(" | ")
            ),
        }
    }
}

fn namespace(descriptor: &str) -> &str {
    descriptor.rsplit_once('/').map_or("", |(ns, _)| ns)
}

fn leaf(descriptor: &str) -> &str {
    descriptor
        .rsplit_once('/')
        .map_or(descriptor, |(_, leaf)| leaf)
}

fn group(states: &[EntityState]) -> BTreeMap<&str, Vec<&EntityState>> {
    let mut map: BTreeMap<&str, Vec<&EntityState>> = BTreeMap::new();
    for state in states {
        map.entry(state.descriptor.as_str())
            .or_default()
            .push(state);
    }
    map
}

/// Classifies every entity of `before` against `after`. SAME entities produce nothing; the result
/// is sorted, so it is a pure function of the two sets.
pub fn correspond(before: &[EntityState], after: &[EntityState]) -> Vec<EntityChange> {
    let (old, new) = (group(before), group(after));
    let mut changes = Vec::new();
    let mut unmatched_old = Vec::new();
    let mut unmatched_new = Vec::new();

    let descriptors: BTreeSet<&str> = old.keys().chain(new.keys()).copied().collect();
    for d in descriptors {
        match (old.get(d), new.get(d)) {
            (Some(a), Some(b)) if a.len() == 1 && b.len() == 1 => {
                let (a, b) = (a[0], b[0]);
                let differs: Vec<&str> = [
                    ("signature", a.signature != b.signature),
                    ("body", a.body != b.body),
                    ("visibility", a.visibility != b.visibility),
                ]
                .into_iter()
                .filter_map(|(what, differs)| differs.then_some(what))
                .collect();
                if !differs.is_empty() {
                    changes.push(EntityChange {
                        kind: Correspondence::Changed,
                        before: vec![d.to_owned()],
                        after: vec![d.to_owned()],
                        evidence: differs.join(","),
                    });
                }
            }
            (Some(a), Some(b)) => {
                // A duplicated descriptor: identity cannot tell its members apart.
                let (mut a, mut b) = (a.clone(), b.clone());
                a.sort();
                b.sort();
                if a != b {
                    changes.push(EntityChange {
                        kind: Correspondence::Ambiguous,
                        before: vec![d.to_owned()],
                        after: vec![d.to_owned()],
                        evidence: format!("descriptor declared {} -> {} times", a.len(), b.len()),
                    });
                }
            }
            (Some(a), None) => unmatched_old.extend(a.iter().copied()),
            (None, Some(b)) => unmatched_new.extend(b.iter().copied()),
            (None, None) => unreachable!("descriptor from one of the two maps"),
        }
    }

    // Unmatched entities correspond only through identical signature AND body fingerprints; a
    // bodiless declaration carries too little evidence to be matched at all.
    let fingerprint =
        |e: &EntityState| (e.body != "-").then(|| (e.signature.clone(), e.body.clone()));
    let mut old_by: BTreeMap<(String, String), Vec<&EntityState>> = BTreeMap::new();
    for e in &unmatched_old {
        if let Some(f) = fingerprint(e) {
            old_by.entry(f).or_default().push(e);
        }
    }
    let mut new_by: BTreeMap<(String, String), Vec<&EntityState>> = BTreeMap::new();
    for e in &unmatched_new {
        if let Some(f) = fingerprint(e) {
            new_by.entry(f).or_default().push(e);
        }
    }
    let mut decided: BTreeSet<(&str, bool)> = BTreeSet::new();
    for (f, olds) in &old_by {
        let Some(news) = new_by.get(f) else { continue };
        let descriptors = |states: &[&EntityState]| -> Vec<String> {
            let mut out: Vec<String> = states.iter().map(|e| e.descriptor.clone()).collect();
            out.sort();
            out
        };
        for e in olds {
            decided.insert((e.descriptor.as_str(), false));
        }
        for e in news {
            decided.insert((e.descriptor.as_str(), true));
        }
        if olds.len() == 1 && news.len() == 1 {
            let (a, b) = (olds[0], news[0]);
            let kind = match (
                namespace(&a.descriptor) == namespace(&b.descriptor),
                leaf(&a.descriptor) == leaf(&b.descriptor),
            ) {
                (true, _) => Correspondence::Renamed,
                (false, true) => Correspondence::Moved,
                (false, false) => Correspondence::MovedRenamed,
            };
            let visibility = if a.visibility == b.visibility {
                String::new()
            } else {
                format!("; visibility {} -> {}", a.visibility, b.visibility)
            };
            changes.push(EntityChange {
                kind,
                before: vec![a.descriptor.clone()],
                after: vec![b.descriptor.clone()],
                evidence: format!("identical signature and body{visibility}"),
            });
        } else {
            changes.push(EntityChange {
                kind: Correspondence::Ambiguous,
                before: descriptors(olds),
                after: descriptors(news),
                evidence: "several entities share one signature and body fingerprint".into(),
            });
        }
    }
    for e in unmatched_old {
        if !decided.contains(&(e.descriptor.as_str(), false)) {
            changes.push(EntityChange {
                kind: Correspondence::Deleted,
                before: vec![e.descriptor.clone()],
                after: Vec::new(),
                evidence: String::new(),
            });
        }
    }
    for e in unmatched_new {
        if !decided.contains(&(e.descriptor.as_str(), true)) {
            changes.push(EntityChange {
                kind: Correspondence::Created,
                before: Vec::new(),
                after: vec![e.descriptor.clone()],
                evidence: String::new(),
            });
        }
    }
    changes.sort();
    changes
}

#[cfg(test)]
mod tests;
