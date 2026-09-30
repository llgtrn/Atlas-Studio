//! Source observation: a Rust lexer that finds crate-root paths, `extern crate`, native link
//! attributes, build-script link directives and process invocations, plus a scan of executable
//! files for references to held donor source trees.

use super::{is_excluded, scope_of_file, Census, Observation, Via};
use crate::declare::Declaration;
use crate::repository::files::Files;
use crate::schema::Ecosystem;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Punct(char),
    Str(String),
}

/// Tokenizes Rust source, dropping comments, and keeping string literal contents.
pub fn lex(src: &str) -> Vec<Tok> {
    let s: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < s.len() {
        let c = s[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && s.get(i + 1) == Some(&'/') {
            while i < s.len() && s[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && s.get(i + 1) == Some(&'*') {
            let mut depth = 1;
            i += 2;
            while i < s.len() && depth > 0 {
                if s[i] == '/' && s.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if s[i] == '*' && s.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if (c == 'r' || c == 'b') && raw_start(&s, i).is_some() {
            let (hashes, body) = raw_start(&s, i).unwrap();
            let mut j = body;
            let mut text = String::new();
            while j < s.len() {
                if s[j] == '"' && (0..hashes).all(|k| s.get(j + 1 + k) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                text.push(s[j]);
                j += 1;
            }
            out.push(Tok::Str(text));
            i = j;
        } else if c == '"' || (c == 'b' && s.get(i + 1) == Some(&'"')) {
            i += if c == 'b' { 2 } else { 1 };
            let mut text = String::new();
            while i < s.len() && s[i] != '"' {
                if s[i] == '\\' && i + 1 < s.len() {
                    text.push(s[i + 1]);
                    i += 2;
                } else {
                    text.push(s[i]);
                    i += 1;
                }
            }
            i += 1;
            out.push(Tok::Str(text));
        } else if c == '\'' {
            // A char literal ('x', '\n', '\u{..}') or a lifetime ('a).
            if s.get(i + 1) == Some(&'\\') {
                i += 2;
                while i < s.len() && s[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if s.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
                while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                    i += 1;
                }
            }
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(s[start..i].iter().collect()));
        } else if c.is_ascii_digit() {
            while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_' || s[i] == '.') {
                i += 1;
            }
        } else {
            out.push(Tok::Punct(c));
            i += 1;
        }
    }
    out
}

/// If a raw string (`r"`, `r#"`, `br#"`) starts at `i`, returns (hash count, body start).
fn raw_start(s: &[char], i: usize) -> Option<(usize, usize)> {
    let mut j = i;
    if s.get(j) == Some(&'b') {
        j += 1;
    }
    if s.get(j) != Some(&'r') {
        return None;
    }
    j += 1;
    let mut hashes = 0;
    while s.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    (s.get(j) == Some(&'"')).then_some((hashes, j + 1))
}

const NOT_CRATES: &[&str] = &[
    "crate",
    "self",
    "super",
    "Self",
    "std",
    "core",
    "alloc",
    "proc_macro",
    "test",
];

/// Root identifiers of paths (`x::..`, `use x`, `extern crate x`) in one file.
pub fn crate_roots(toks: &[Tok]) -> BTreeSet<String> {
    let mut roots = BTreeSet::new();
    let mut local_mods = BTreeSet::new();
    let is = |i: usize, c: char| matches!(toks.get(i), Some(Tok::Punct(p)) if *p == c);
    for i in 0..toks.len() {
        if let Tok::Ident(x) = &toks[i] {
            if x == "mod" {
                if let Some(Tok::Ident(m)) = toks.get(i + 1) {
                    local_mods.insert(m.clone());
                }
            }
            // A path root: not preceded by `::`, or preceded by a leading (global) `::`.
            let after_colons = i >= 2 && is(i - 1, ':') && is(i - 2, ':');
            let continuation = after_colons
                && i >= 3
                && matches!(
                    toks.get(i - 3),
                    Some(Tok::Ident(_)) | Some(Tok::Punct('>')) | Some(Tok::Punct(')'))
                );
            let path_start = !after_colons || !continuation;
            if path_start && is(i + 1, ':') && is(i + 2, ':') {
                roots.insert(x.clone());
            }
            if (x == "use" || x == "crate") && i + 2 < toks.len() {
                // `use x;` / `extern crate x;` / `extern crate x as y;`
                if let (Some(Tok::Ident(n)), true) = (
                    toks.get(i + 1),
                    is(i + 2, ';') || matches!(toks.get(i + 2), Some(Tok::Ident(a)) if a == "as"),
                ) {
                    if x == "use"
                        || matches!(toks.get(i.wrapping_sub(1)), Some(Tok::Ident(e)) if e == "extern")
                    {
                        roots.insert(n.clone());
                    }
                }
            }
        }
    }
    // Names a `use crate::…` / `use self::…` / `use super::…` (or local module) statement
    // brings into scope shadow crates of the same name.
    let mut local_names = BTreeSet::new();
    let mut i = 0;
    while i < toks.len() {
        if matches!(&toks[i], Tok::Ident(u) if u == "use") {
            let end = (i..toks.len()).find(|j| is(*j, ';')).unwrap_or(toks.len());
            let root = toks[i + 1..end].iter().find_map(|t| match t {
                Tok::Ident(x) => Some(x.as_str()),
                _ => None,
            });
            if root
                .is_some_and(|r| matches!(r, "crate" | "self" | "super") || local_mods.contains(r))
            {
                for j in i + 1..end {
                    if let Tok::Ident(x) = &toks[j] {
                        let next_closes =
                            matches!(toks.get(j + 1), Some(Tok::Punct(',' | '}' | ';')));
                        let renamed =
                            matches!(toks.get(j.wrapping_sub(1)), Some(Tok::Ident(a)) if a == "as");
                        if next_closes || renamed || j + 1 == end {
                            local_names.insert(x.clone());
                        }
                    }
                }
            }
            i = end;
        }
        i += 1;
    }
    roots
        .into_iter()
        .filter(|r| {
            !NOT_CRATES.contains(&r.as_str())
                && !local_mods.contains(r)
                && !local_names.contains(r)
                && r.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        })
        .collect()
}

/// `#[link(name = "x")]` libraries and `Command::new("x")` programs.
pub fn links_and_processes(toks: &[Tok]) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut links = BTreeSet::new();
    let mut procs = BTreeSet::new();
    let ident = |i: usize, w: &str| matches!(toks.get(i), Some(Tok::Ident(x)) if x == w);
    let punct = |i: usize, c: char| matches!(toks.get(i), Some(Tok::Punct(p)) if *p == c);
    for i in 0..toks.len() {
        if ident(i, "link") && punct(i + 1, '(') && ident(i + 2, "name") && punct(i + 3, '=') {
            if let Some(Tok::Str(n)) = toks.get(i + 4) {
                links.insert(n.clone());
            }
        }
        if ident(i, "Command")
            && punct(i + 1, ':')
            && punct(i + 2, ':')
            && ident(i + 3, "new")
            && punct(i + 4, '(')
        {
            if let Some(Tok::Str(p)) = toks.get(i + 5) {
                let program = p.rsplit('/').next().unwrap_or(p).to_string();
                if !program.is_empty() {
                    procs.insert(program);
                }
            }
        }
        if let Some(Tok::Str(s)) = toks.get(i) {
            for marker in ["rustc-link-lib=", "rustc-link-lib:"] {
                if let Some(rest) = s.split(marker).nth(1) {
                    let lib = rest.rsplit('=').next().unwrap_or(rest).trim();
                    if !lib.is_empty() {
                        links.insert(lib.to_string());
                    }
                }
            }
        }
    }
    (links, procs)
}

const EXECUTABLE: &[&str] = &[
    ".rs",
    ".sh",
    ".bash",
    ".py",
    ".js",
    ".mjs",
    ".cjs",
    ".ts",
    ".tsx",
    ".c",
    ".h",
    ".cc",
    ".cpp",
    ".cmake",
    ".yml",
    ".yaml",
    ".ps1",
    "Makefile",
    "Justfile",
    "justfile",
    "Dockerfile",
];

pub fn observe(files: &Files, d: &Declaration, excluded: &[String], c: &mut Census) {
    let donor_paths: Vec<String> = d
        .donors
        .iter()
        .flat_map(|dn| dn.source_paths.iter())
        .map(|p| p.trim_end_matches('/').to_string())
        .filter(|p| !p.is_empty())
        .collect();
    for f in files.paths.iter() {
        if is_excluded(f, excluded) {
            continue;
        }
        let exec = EXECUTABLE.iter().any(|e| f.ends_with(e));
        if !exec {
            continue;
        }
        let in_ynventa = f.starts_with(".ynventa/");
        let Some(text) = files.read(f) else {
            continue;
        };
        if f.ends_with(".rs") {
            let toks = lex(&text);
            let roots = crate_roots(&toks);
            if !roots.is_empty() {
                c.imports.insert(f.clone(), roots);
            }
            if !in_ynventa {
                let (links, procs) = links_and_processes(&toks);
                let scope = scope_of_file(f);
                for l in links {
                    c.observations.insert(Observation {
                        file: f.clone(),
                        ecosystem: Ecosystem::Native,
                        name: l.clone(),
                        ident: l,
                        scope: if scope == crate::schema::Scope::Test {
                            scope
                        } else {
                            crate::schema::Scope::Linked
                        },
                        via: Via::Link,
                    });
                }
                for p in procs {
                    c.observations.insert(Observation {
                        file: f.clone(),
                        ecosystem: Ecosystem::Native,
                        name: p.clone(),
                        ident: p,
                        scope,
                        via: Via::Process,
                    });
                }
            }
        }
        if !in_ynventa {
            for p in &donor_paths {
                if text.contains(p.as_str()) {
                    c.observations.insert(Observation {
                        file: f.clone(),
                        ecosystem: Ecosystem::Native,
                        name: p.clone(),
                        ident: p.clone(),
                        scope: scope_of_file(f),
                        via: Via::SourceReference,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_crate_roots_not_comments_or_strings() {
        let src = r##"
            // use commented::Out;
            /* nested /* block */ proj::x */
            use serde::Serialize;
            use geo;
            extern crate rstar;
            mod local; use local::thing;
            use crate::formats::{json, toml as t, Value};
            fn h() { json::parse(); t::parse(); }
            fn f() { let s = "tokio::spawn"; let r = r#"hyper::x"#; let c = 'a'; }
            fn g<'a>(x: &'a str) -> std::string::String { crate::m(); NodeKind::Kernel; ::regex::Regex::new(x) }
        "##;
        let roots = crate_roots(&lex(src));
        let got: Vec<&str> = roots.iter().map(String::as_str).collect();
        assert_eq!(got, vec!["geo", "regex", "rstar", "serde"]);
    }

    #[test]
    fn finds_links_and_processes() {
        let src = r#"
            #[link(name = "proj")] extern "C" {}
            fn main() { println!("cargo:rustc-link-lib=static=modbus"); std::process::Command::new("/usr/bin/proj"); }
        "#;
        let (l, p) = links_and_processes(&lex(src));
        assert!(l.contains("proj") && l.contains("modbus"));
        assert!(p.contains("proj"));
    }
}
