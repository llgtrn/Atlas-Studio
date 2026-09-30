//! G184 (ADR 0096): whether a directory with a build-output name (`build`, `dist`, `.next`,
//! `coverage`) is written Rust source, decided from which files are present -- no file's contents
//! are read, so no untrusted text is parsed for the decision.
//!
//! A directory `P/N` is walked as source when:
//! - it contains `mod.rs` (a module directory), or
//! - it contains `Cargo.toml` (a package), or
//! - `P` contains `N.rs` that is not a crate root, and `P/N` contains at least one `.rs` file (the
//!   directory of a non-`mod.rs` module's children). A crate root is `build.rs`, `main.rs` or
//!   `lib.rs` beside a `Cargo.toml`, or any file under a `src/bin`, `examples`, `tests` or
//!   `benches` directory.
//!
//! Anything else keeps the generated-or-external boundary, as before G184: a `#[path]` into such a
//! directory, an inline `mod N { mod x; }`, `cfg_attr`/`cfg_if!` modules, workspace members,
//! Cargo `path`/`build` targets and non-Rust source are not recognized (a disclosed residual).

use std::{fs, path::Path};

/// At most this many entries of an output-named directory are examined for a `.rs` file.
const MAX_EXAMINED_ENTRIES: usize = 4096;

/// Why an output-named directory stays a boundary, or `None` when it is walked as source.
pub(super) fn boundary_reason(root: &Path, directory: &Path, name: &str) -> Option<&'static str> {
    let is_file = |p: &Path| fs::symlink_metadata(p).is_ok_and(|m| m.is_file());
    if is_file(&directory.join("mod.rs")) || is_file(&directory.join("Cargo.toml")) {
        return None;
    }
    let Some(parent) = directory.parent() else {
        return Some(GENERATED);
    };
    let file_name = format!("{name}.rs");
    if !is_file(&parent.join(&file_name)) || crate_root(root, parent, &file_name) {
        return Some(GENERATED);
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return Some(GENERATED);
    };
    for (examined, entry) in entries.enumerate() {
        if examined == MAX_EXAMINED_ENTRIES {
            return Some(CAPPED);
        }
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") && is_file(&path) {
            return None;
        }
    }
    Some(GENERATED)
}

pub(super) const GENERATED: &str = "generated-or-external-directory-boundary";
pub(super) const CAPPED: &str = "generated-or-external-directory-boundary; the entry cap was \
     reached before a .rs file was found, so whether it is a module directory is not decided";

/// Whether `file_name` in `parent` is a crate root rather than a module.
fn crate_root(root: &Path, parent: &Path, file_name: &str) -> bool {
    let beside_manifest = matches!(file_name, "build.rs" | "main.rs" | "lib.rs")
        && fs::symlink_metadata(parent.join("Cargo.toml")).is_ok_and(|m| m.is_file());
    let relative = parent.strip_prefix(root).unwrap_or(parent);
    let components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let target_directory = components
        .iter()
        .any(|c| matches!(*c, "examples" | "tests" | "benches"))
        || components.windows(2).any(|w| w == ["src", "bin"]);
    beside_manifest || target_directory
}
