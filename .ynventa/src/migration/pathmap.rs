//! `legacy path → canonical node → canonical path`, planned in waves and applied one atomic wave
//! at a time. Applying a wave moves directories, rewrites Cargo workspace members and every
//! path dependency (relative paths recomputed from each manifest's new location), updates the
//! declarations and marks the wave APPLIED. Node identities are untouched: identity is never a
//! path.

use crate::census::cargo::{join, members};
use crate::declare::{self, Declaration, Wave};
use crate::repository::files::Files;
use crate::repository::{role_of_path, ROLES};
use crate::schema::{is_physical, NodeKind, NodeLifecycle, WaveStatus};
use std::collections::BTreeMap;
use std::path::Path;

/// One row of the path map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub legacy_path: String,
    pub node: String,
    pub node_id: String,
    pub canonical_path: String,
}

pub fn path_map(d: &Declaration) -> Vec<Mapping> {
    let mut v: Vec<Mapping> = d
        .repository
        .nodes
        .iter()
        .filter(|n| {
            is_physical(n.kind)
                && n.kind != NodeKind::Repository
                && n.lifecycle == NodeLifecycle::Active
        })
        .map(|n| Mapping {
            legacy_path: n.path.clone(),
            node: n.key.clone(),
            node_id: crate::graph::NodeId::of(crate::schema::SYSTEM, &n.key).to_string(),
            canonical_path: n.canonical_path.clone(),
        })
        .collect();
    v.sort_by(|a, b| a.legacy_path.cmp(&b.legacy_path));
    v
}

/// Plans waves for every legacy-placed node not yet in a wave: one wave per current root,
/// ordered by the plane of their canonical roles (kernel first).
pub fn plan(d: &Declaration) -> Vec<Wave> {
    let planned: Vec<&str> = d
        .migration
        .waves
        .iter()
        .flat_map(|w| w.nodes.iter().map(String::as_str))
        .collect();
    let mut groups: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for m in path_map(d) {
        if m.legacy_path == m.canonical_path || planned.contains(&m.node.as_str()) {
            continue;
        }
        let root = m.legacy_path.split('/').next().unwrap_or("").to_string();
        let rank = role_of_path(&m.canonical_path)
            .and_then(|r| ROLES.iter().position(|s| s.role == r))
            .unwrap_or(ROLES.len());
        let e = groups.entry(root).or_insert((rank, Vec::new()));
        e.0 = e.0.min(rank);
        e.1.push(m.node);
    }
    let mut ordered: Vec<(usize, String, Vec<String>)> = groups
        .into_iter()
        .map(|(root, (rank, nodes))| (rank, root, nodes))
        .collect();
    ordered.sort();
    let base = d.migration.waves.len();
    ordered
        .into_iter()
        .enumerate()
        .map(|(i, (_, root, mut nodes))| {
            nodes.sort();
            Wave {
                key: format!("w{:02}-{}", base + i + 1, root.trim_start_matches('.')),
                status: WaveStatus::Planned,
                nodes,
            }
        })
        .collect()
}

/// The relative path from directory `from` to `to` (both repository-relative).
pub fn relative(from: &str, to: &str) -> String {
    let f: Vec<&str> = from
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    let t: Vec<&str> = to
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    let common = f.iter().zip(&t).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_string(), f.len() - common).collect();
    parts.extend(t[common..].iter().map(|s| s.to_string()));
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

/// Maps a path through a list of directory moves (applied in order).
pub fn map_path(moves: &[(String, String)], p: &str) -> String {
    let mut p = p.to_string();
    for (from, to) in moves {
        if p == *from {
            p = to.clone();
        } else if let Some(rest) = p.strip_prefix(&format!("{from}/")) {
            p = format!("{to}/{rest}");
        }
    }
    p
}

/// Rewrites every `path = "…"` value of a manifest that lived in `old_dir` and now lives in
/// `new_dir`, resolving each target through `moves`.
pub fn rewrite_paths(
    text: &str,
    old_dir: &str,
    new_dir: &str,
    moves: &[(String, String)],
) -> String {
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut last = 0;
    while i + 4 <= b.len() {
        let key_start = i == 0 || matches!(b[i - 1], b' ' | b'\t' | b'\n' | b'{' | b',');
        if key_start && &b[i..i + 4] == b"path" {
            let mut j = i + 4;
            while j < b.len() && matches!(b[j], b' ' | b'\t') {
                j += 1;
            }
            if j < b.len() && b[j] == b'=' {
                j += 1;
                while j < b.len() && matches!(b[j], b' ' | b'\t') {
                    j += 1;
                }
                if j < b.len() && b[j] == b'"' {
                    let vs = j + 1;
                    if let Some(len) = text[vs..].find('"') {
                        let value = &text[vs..vs + len];
                        let target = map_path(moves, &join(old_dir, value));
                        let new_value = relative(new_dir, &target);
                        out.push_str(&text[last..vs]);
                        out.push_str(&new_value);
                        last = vs + len;
                        i = vs + len + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out.push_str(&text[last..]);
    out
}

/// Rewrites the string entries of `members`, `default-members` and `exclude` arrays.
pub fn rewrite_members(text: &str, moves: &[(String, String)]) -> String {
    let mut out = String::new();
    let mut in_array = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        let opens = ["members", "default-members", "exclude"].iter().any(|k| {
            t.strip_prefix(k)
                .is_some_and(|r| r.trim_start().starts_with('='))
        });
        if opens {
            in_array = true;
        }
        if in_array {
            let mut l = String::new();
            let mut parts = line.split('"');
            l.push_str(parts.next().unwrap_or(""));
            for (k, part) in parts.enumerate() {
                l.push('"');
                if k % 2 == 0 {
                    l.push_str(&map_path(moves, part));
                } else {
                    l.push_str(part);
                }
            }
            out.push_str(&l);
            if line.contains(']') {
                in_array = false;
            }
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Workspace references that do not resolve on disk: member directories without a manifest
/// and path dependencies whose target has none. Read from the working tree, not the index,
/// because it judges an in-flight wave.
pub fn unresolved_paths(root: &Path) -> Vec<String> {
    use crate::formats::{toml, Value};
    let mut out = Vec::new();
    let Some(ws) = std::fs::read_to_string(root.join("Cargo.toml"))
        .ok()
        .and_then(|t| toml::parse(&t).ok())
    else {
        return out;
    };
    let mut manifests: Vec<(String, Value)> = vec![(String::new(), ws.clone())];
    for m in ws
        .at("workspace")
        .map(|w| w.strings("members"))
        .unwrap_or_default()
    {
        let dirs: Vec<String> = match m.split_once('*') {
            Some((prefix, _)) => std::fs::read_dir(root.join(prefix.trim_end_matches('/')))
                .map(|rd| {
                    rd.filter_map(Result::ok)
                        .filter(|e| e.path().join("Cargo.toml").exists())
                        .map(|e| {
                            format!(
                                "{}/{}",
                                prefix.trim_end_matches('/'),
                                e.file_name().to_string_lossy()
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            None => vec![m.clone()],
        };
        for dir in dirs {
            match std::fs::read_to_string(root.join(&dir).join("Cargo.toml")) {
                Ok(t) => match toml::parse(&t) {
                    Ok(v) => manifests.push((dir, v)),
                    Err(e) => out.push(format!("{dir}/Cargo.toml: {e}")),
                },
                Err(_) => out.push(format!("member `{dir}` has no Cargo.toml")),
            }
        }
    }
    for (dir, v) in &manifests {
        let mut tables: Vec<&Value> = ["dependencies", "build-dependencies", "dev-dependencies"]
            .iter()
            .filter_map(|s| v.get(s))
            .collect();
        if let Some(Value::Table(t)) = v.get("target") {
            for x in t.values() {
                for s in ["dependencies", "build-dependencies", "dev-dependencies"] {
                    if let Some(d) = x.get(s) {
                        tables.push(d);
                    }
                }
            }
        }
        if let Some(d) = v.at("workspace.dependencies") {
            tables.push(d);
        }
        for t in tables {
            for (name, spec) in t.table().into_iter().flatten() {
                if let Some(p) = spec.str("path") {
                    let target = join(dir, p);
                    if !root.join(&target).join("Cargo.toml").exists() {
                        let at = if dir.is_empty() {
                            "Cargo.toml".to_string()
                        } else {
                            format!("{dir}/Cargo.toml")
                        };
                        out.push(format!("{at}: `{name}` -> `{target}` does not exist"));
                    }
                }
            }
        }
    }
    out
}

#[derive(Debug, Default)]
pub struct Applied {
    pub moves: Vec<(String, String)>,
    pub manifests: Vec<String>,
    /// Tracked files that still mention a moved path (not rewritten automatically).
    pub stale_references: Vec<(String, String)>,
}

/// Applies one planned wave to the repository at `root`.
pub fn apply(
    root: &Path,
    files: &Files,
    d: &mut Declaration,
    wave_key: &str,
) -> Result<Applied, String> {
    let before = unresolved_paths(root);
    if !before.is_empty() {
        return Err(format!(
            "the workspace does not resolve before the wave: {}",
            before.join("; ")
        ));
    }
    let original = d.clone();
    let wi = d
        .migration
        .waves
        .iter()
        .position(|w| w.key == wave_key)
        .ok_or_else(|| format!("no wave `{wave_key}`"))?;
    if d.migration.waves[wi].status == WaveStatus::Applied {
        return Err(format!("wave `{wave_key}` is already applied"));
    }
    let mut moves: Vec<(String, String)> = Vec::new();
    for k in &d.migration.waves[wi].nodes {
        let n = d
            .node(k)
            .ok_or_else(|| format!("wave names undeclared node `{k}`"))?;
        if n.path != n.canonical_path {
            moves.push((n.path.clone(), n.canonical_path.clone()));
        }
    }
    // Deepest first, so that nested nodes leave their parents before the parents move.
    moves.sort_by(|a, b| {
        b.0.matches('/')
            .count()
            .cmp(&a.0.matches('/').count())
            .then(a.0.cmp(&b.0))
    });
    for (from, to) in &moves {
        if !root.join(from).is_dir() {
            return Err(format!("`{from}` is not a directory"));
        }
        if root.join(to).exists() {
            return Err(format!("`{to}` already exists; refusing to overwrite"));
        }
        if to.starts_with(&format!("{from}/")) {
            return Err(format!("cannot move `{from}` into itself (`{to}`)"));
        }
    }
    // Moves are expressed on the original tree; later moves see earlier ones' results.
    let mut sequenced: Vec<(String, String)> = Vec::new();
    for (from, to) in &moves {
        sequenced.push((map_path(&sequenced, from), to.clone()));
    }

    let manifests_before: Vec<String> = {
        let mut v: Vec<String> = members(files)
            .into_iter()
            .filter(|m| m.dir != ".ynventa")
            .map(|m| m.dir)
            .collect();
        v.push(String::new());
        v.sort();
        v.dedup();
        v
    };
    let mut texts: BTreeMap<String, String> = BTreeMap::new();
    for dir in &manifests_before {
        let rel = if dir.is_empty() {
            "Cargo.toml".to_string()
        } else {
            format!("{dir}/Cargo.toml")
        };
        if let Ok(t) = std::fs::read_to_string(root.join(&rel)) {
            texts.insert(dir.clone(), t);
        }
    }

    for (from, to) in &sequenced {
        let dest = root.join(to);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        std::fs::rename(root.join(from), &dest).map_err(|e| format!("move {from} -> {to}: {e}"))?;
        // Remove emptied legacy parents.
        let mut parent = Path::new(from).parent().map(Path::to_path_buf);
        while let Some(p) = parent {
            if p.as_os_str().is_empty() || std::fs::remove_dir(root.join(&p)).is_err() {
                break;
            }
            parent = p.parent().map(Path::to_path_buf);
        }
    }

    let mut applied = Applied {
        moves: moves.clone(),
        ..Applied::default()
    };
    for (old_dir, text) in &texts {
        let new_dir = map_path(&moves, old_dir);
        let mut t = rewrite_paths(text, old_dir, &new_dir, &moves);
        if old_dir.is_empty() {
            t = rewrite_members(&t, &moves);
        }
        if t != *text || new_dir != *old_dir {
            let rel = if new_dir.is_empty() {
                "Cargo.toml".to_string()
            } else {
                format!("{new_dir}/Cargo.toml")
            };
            std::fs::write(root.join(&rel), &t).map_err(|e| format!("write {rel}: {e}"))?;
            applied.manifests.push(rel);
        }
    }

    for n in d.repository.nodes.iter_mut() {
        if is_physical(n.kind) && !n.path.is_empty() {
            n.path = map_path(&moves, &n.path);
        }
    }
    d.migration.waves[wi].status = WaveStatus::Applied;
    declare::store(root, d).map_err(|e| format!("write declarations: {e}"))?;

    // Every wave leaves the workspace resolvable, or it is undone.
    let after = unresolved_paths(root);
    if !after.is_empty() {
        for (from, to) in sequenced.iter().rev() {
            let _ = std::fs::create_dir_all(root.join(from).parent().unwrap_or(root));
            let _ = std::fs::rename(root.join(to), root.join(from));
        }
        for (old_dir, text) in &texts {
            let rel = if old_dir.is_empty() {
                "Cargo.toml".to_string()
            } else {
                format!("{old_dir}/Cargo.toml")
            };
            let _ = std::fs::write(root.join(rel), text);
        }
        *d = original;
        let _ = declare::store(root, d);
        return Err(format!(
            "wave `{wave_key}` rolled back: the workspace would not resolve: {}",
            after.join("; ")
        ));
    }

    for f in &files.paths {
        if !(f.ends_with(".rs")
            || f.ends_with(".sh")
            || f.ends_with(".yml")
            || f.ends_with(".yaml")
            || f.ends_with(".toml"))
        {
            continue;
        }
        if f.starts_with(".ynventa/") || f.starts_with(".atlas/") || f.ends_with("Cargo.toml") {
            continue;
        }
        let current = map_path(&moves, f);
        if let Ok(t) = std::fs::read_to_string(root.join(&current)) {
            for (from, _) in &moves {
                if t.contains(&format!("{from}/")) || t.contains(&format!("\"{from}\"")) {
                    applied
                        .stale_references
                        .push((current.clone(), from.clone()));
                }
            }
        }
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wave_that_breaks_resolution_is_rolled_back() {
        use crate::declare::*;
        use crate::schema::*;
        let root = std::env::temp_dir().join(format!("ynventa-rollback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let w = |p: &str, t: &str| {
            std::fs::create_dir_all(root.join(p).parent().unwrap()).unwrap();
            std::fs::write(root.join(p), t).unwrap();
        };
        w(
            "Cargo.toml",
            "[workspace]\nmembers = [\"core\", \"storage\", \"tools/*\"]\n",
        );
        w("core/Cargo.toml", "[package]\nname = \"core\"\n");
        w(
            "storage/Cargo.toml",
            "[package]\nname = \"storage\"\n[dependencies]\ncore = { path = \"../core\" }\n",
        );
        // A crate a concurrent branch just created under a glob member: on disk, not yet indexed.
        w(
            "tools/fresh/Cargo.toml",
            "[package]\nname = \"fresh\"\n[dependencies]\nstorage = { path = \"../../storage\" }\n",
        );
        let node = |k: &str, kind, p: &str, c: &str| Node::new(k, kind, p, c);
        let mut d = Declaration {
            technologies: vec![],
            repository: Repository {
                system: "chronica".into(),
                shard: "chronica".into(),
                name: "t".into(),
                origin: "t".into(),
                nodes: vec![
                    node("core", NodeKind::Kernel, "core", "core"),
                    node(
                        "storage",
                        NodeKind::Substrate,
                        "storage",
                        "substrate/storage",
                    ),
                ],
                edges: vec![],
            },
            donors: vec![],
            migration: Migration {
                waves: vec![Wave {
                    key: "w1".into(),
                    status: WaveStatus::Planned,
                    nodes: vec!["storage".into()],
                }],
                shims: vec![],
            },
        };
        store(&root, &d).unwrap();
        let indexed = Files::from_paths(
            &root,
            ["Cargo.toml", "core/Cargo.toml", "storage/Cargo.toml"].map(String::from),
        );
        assert!(unresolved_paths(&root).is_empty());
        let err = apply(&root, &indexed, &mut d, "w1").unwrap_err();
        assert!(
            err.contains("rolled back") && err.contains("tools/fresh"),
            "{err}"
        );
        assert!(
            root.join("storage/Cargo.toml").exists()
                && !root.join("substrate").join("storage").exists()
        );
        assert_eq!(
            load(&root).unwrap().migration.waves[0].status,
            WaveStatus::Planned
        );
        assert!(
            unresolved_paths(&root).is_empty(),
            "the tree is exactly as before"
        );
        // With the new crate indexed, the same wave applies and resolves.
        let all = Files::scan(&root).unwrap();
        let mut d = load(&root).unwrap();
        apply(&root, &all, &mut d, "w1").unwrap();
        assert!(unresolved_paths(&root).is_empty());
        assert!(std::fs::read_to_string(root.join("tools/fresh/Cargo.toml"))
            .unwrap()
            .contains("\"../../substrate/storage\""));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn relative_paths() {
        assert_eq!(relative("storage", "core"), "../core");
        assert_eq!(relative("substrate/storage", "core"), "../../core");
        assert_eq!(relative("", "substrate/storage"), "substrate/storage");
        assert_eq!(relative("apps/cli", "apps/cli/sub"), "sub");
    }

    #[test]
    fn manifests_are_rewritten_through_moves() {
        let moves = vec![
            ("storage".to_string(), "substrate/storage".to_string()),
            ("world".to_string(), "substrate/world".to_string()),
        ];
        let m = "[dependencies]\ncore = { path = \"../core\" }\nworld = {path=\"../world\", version = \"0.1\"}\n";
        let out = rewrite_paths(m, "storage", "substrate/storage", &moves);
        assert!(out.contains("path = \"../../core\""), "{out}");
        assert!(out.contains("path=\"../world\""), "{out}");
        let root = "[workspace]\nmembers = [\n  \"core\",\n  \"storage\",\n  \"world\", # c\n]\n[workspace.dependencies]\nstorage = { path = \"storage\" }\n";
        let r = rewrite_members(&rewrite_paths(root, "", "", &moves), &moves);
        assert!(
            r.contains("\"substrate/storage\",\n  \"substrate/world\", # c"),
            "{r}"
        );
        assert!(
            r.contains("storage = { path = \"substrate/storage\" }"),
            "{r}"
        );
        assert!(r.contains("\"core\""));
    }
}
