//! Build identity of the semantic extractors (ADR 0008).
//!
//! `RUST_SEMANTIC_EXTRACTOR_VERSION` is a hand-maintained label and has not changed across dozens
//! of behaviour-changing commits, so a derivation cache keyed on it would serve stale results. The
//! digest emitted here covers every source file the extractors are built from (`adapter/src`,
//! `core/src`) plus the workspace `Cargo.lock` (pinned parser versions): any change to any of them
//! changes `ATLAS_EXTRACTOR_BUILD_DIGEST`, and with it every extraction cache key.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect(&path, out),
            Ok(kind) if kind.is_file() => out.push(path),
            _ => {}
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest_dir.parent().unwrap().to_path_buf();
    let roots = [manifest_dir.join("src"), workspace.join("core/src")];
    let mut files = Vec::new();
    for root in &roots {
        println!("cargo:rerun-if-changed={}", root.display());
        collect(root, &mut files);
    }
    let lock = workspace.join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    files.push(lock);
    files.sort();

    let mut hasher = atlas_core::blake3::Hasher::new();
    for file in &files {
        let relative = file.strip_prefix(&workspace).unwrap_or(file);
        let bytes = fs::read(file).unwrap_or_default();
        // Length-prefixed path and content, so no two different file sets hash alike.
        for field in [relative.to_string_lossy().as_bytes(), &bytes[..]] {
            hasher.update(&(field.len() as u64).to_le_bytes());
            hasher.update(field);
        }
    }
    let digest = atlas_core::IntegrityDigest::blake3_256(&hasher.finalize());
    println!("cargo:rustc-env=ATLAS_EXTRACTOR_BUILD_DIGEST={digest}");
}
