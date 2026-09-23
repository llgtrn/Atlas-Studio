//! Atlas adapters for external repository mechanics.

use atlas_core::{AdlSource, DocsReport, DocumentFact, RepoAudit, RepoManifest, validate_manifest};
use std::{collections::BTreeMap, fs, io, path::Path};

pub mod dependency;
pub mod semantic;
pub mod source;
pub mod vcs;

pub use dependency::census_cargo_workspace;
pub use semantic::{
    DiagnosticCode, ExtractionBatch, ExtractionDiagnostic, ExtractionInput, ObligationResult,
    SemanticExtractor, StaticUnsupportedExtractor, extractors_for_language, semantic_extractors,
};
pub use source::{
    SourceFrontend, SourceFrontendMatch, inventory_declared_source, inventory_source,
    resolve_source_frontend, scan_declared_source, scan_source, source_frontends,
    source_report_from_inventory,
};
pub use vcs::snapshot_git;

pub fn read_adl_sources(root: impl AsRef<Path>) -> io::Result<Vec<AdlSource>> {
    let root = root.as_ref().canonicalize()?;
    let declared = root.join(".atlas").join("declared");
    let mut sources = Vec::new();
    if declared.is_dir() {
        visit_adl_sources(&root, &declared, &mut sources)?;
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(sources)
}

fn visit_adl_sources(root: &Path, dir: &Path, out: &mut Vec<AdlSource>) -> io::Result<()> {
    // Same discipline as `adapter::source::visit_inventory`'s own `read_dir` hardening: a
    // subdirectory this process cannot list (permission denial being the common real case) must
    // not abort reading every other `.adl` file in the declared tree. There is no per-entry
    // accounting channel for a *directory* here (unlike `visit_inventory`'s `ArtifactRecord`
    // ledger), so this is a best-effort skip rather than a recorded fact -- still a strict
    // improvement over aborting the whole read, and consistent with never silently corrupting
    // (or fabricating) the `.adl` sources that *were* successfully read.
    let Ok(read_dir) = fs::read_dir(dir) else {
        return Ok(());
    };
    let Ok(mut entries) = read_dir.collect::<Result<Vec<_>, _>>() else {
        return Ok(());
    };
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            visit_adl_sources(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("adl") {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // `runtime::census::extraction::extract_semantics` already establishes the right
        // discipline for a per-artifact read/decode failure: account for it, never abort every
        // other artifact's processing because of it (its own doc comment: "one artifact's read
        // failure must not abort extraction of every artifact that comes after it"). A `.adl`
        // file's bytes are not guaranteed to be valid UTF-8 (a hostile or merely corrupted
        // encoding), and `AdlSource.text` requires a real `String` -- `from_utf8_lossy` is the
        // same lossy-but-never-crashing decode `Path::to_string_lossy` already uses elsewhere in
        // this codebase: the file remains present in `sources` (never silently dropped) and its
        // valid portions stay byte-for-byte intact; only genuinely undecodable byte sequences
        // become U+FFFD, which then naturally surfaces through `parse_adl_source`'s normal
        // diagnostics (e.g. `ATLAS-E000`/`ATLAS-E010`) rather than aborting the whole read.
        let text = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
        out.push(AdlSource {
            path: relative,
            text,
        });
    }
    Ok(())
}

fn value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|raw| {
        let line = raw.split('#').next().unwrap_or_default().trim();
        let (left, right) = line.split_once('=')?;
        if left.trim() != key {
            return None;
        }
        right
            .trim()
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .map(ToOwned::to_owned)
    })
}

fn bool_value(text: &str, key: &str) -> Option<bool> {
    text.lines().find_map(|raw| {
        let line = raw.split('#').next().unwrap_or_default().trim();
        let (left, right) = line.split_once('=')?;
        if left.trim() != key {
            return None;
        }
        match right.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    })
}

fn array_value(text: &str, section: &str, key: &str) -> Option<Vec<String>> {
    let mut active = "";
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') && line.ends_with(']') {
            active = line.trim_start_matches('[').trim_end_matches(']').trim();
            continue;
        }
        if active != section {
            continue;
        }
        let (left, right) = line.split_once('=')?;
        if left.trim() != key {
            continue;
        }
        let inner = right.trim().strip_prefix('[')?.strip_suffix(']')?;
        return Some(
            inner
                .split(',')
                .filter_map(|item| {
                    let item = item.trim();
                    item.strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .map(ToOwned::to_owned)
                })
                .collect(),
        );
    }
    None
}

pub fn parse_repo_manifest(text: &str) -> Result<RepoManifest, Vec<String>> {
    let mut errors = Vec::new();
    macro_rules! string {
        ($key:literal) => {
            value(text, $key).unwrap_or_else(|| {
                errors.push(format!("missing_or_invalid_string:{}", $key));
                String::new()
            })
        };
    }
    macro_rules! boolean {
        ($key:literal) => {
            bool_value(text, $key).unwrap_or_else(|| {
                errors.push(format!("missing_or_invalid_bool:{}", $key));
                false
            })
        };
    }
    macro_rules! array {
        ($section:literal, $key:literal) => {
            array_value(text, $section, $key).unwrap_or_else(|| {
                errors.push(format!("missing_or_invalid_array:{}.{}", $section, $key));
                Vec::new()
            })
        };
    }
    let manifest = RepoManifest {
        schema: string!("schema"),
        repo: string!("repo"),
        system_kind: string!("system_kind"),
        backend_language: string!("backend_language"),
        frontend_language: string!("frontend_language"),
        coding_requires_docs_gate: boolean!("coding_requires_docs_gate"),
        graph_before_code_required: boolean!("graph_before_code_required"),
        exact_base_sha_required: boolean!("exact_base_sha_required"),
        single_repository_target_required: boolean!("single_repository_target_required"),
        knowledge_root: string!("knowledge_root"),
        temporary_root: string!("temporary_root"),
        provenance_root: string!("provenance_root"),
        license_root: string!("license_root"),
        source_roots: array!("code", "source_roots"),
        backend_roots: array!("code", "backend_roots"),
        frontend_roots: array!("code", "frontend_roots"),
        test_roots: array!("code", "test_roots"),
    };
    if errors.is_empty() {
        Ok(manifest)
    } else {
        Err(errors)
    }
}

pub fn audit_repository(root: impl AsRef<Path>) -> io::Result<RepoAudit> {
    let root = root.as_ref();
    let atlas_root = root.join(".atlas");
    let repo_manifest = atlas_root.join("repo.toml");
    let mut missing_required_roles = Vec::new();
    let mut missing_mapped_paths = Vec::new();
    if !atlas_root.is_dir() {
        missing_required_roles.push("canonical_knowledge_root".into());
        missing_mapped_paths.push(".atlas".into());
    }
    if !repo_manifest.is_file() {
        missing_required_roles.push("repository_manifest".into());
        missing_mapped_paths.push(".atlas/repo.toml".into());
    }

    let mut archetype = "UNKNOWN".to_owned();
    let mut manifest = None;
    let mut policy_violations = Vec::new();
    if repo_manifest.is_file() {
        // Same discipline as `visit_adl_sources`/`visit_docs`: a manifest whose bytes are not
        // valid UTF-8 must not abort the whole repository audit. Lossy-decoded, never dropped.
        let text = String::from_utf8_lossy(&fs::read(&repo_manifest)?).into_owned();
        match parse_repo_manifest(&text) {
            Ok(parsed) => {
                archetype = parsed.system_kind.clone();
                policy_violations = validate_manifest(&parsed);
                manifest = Some(parsed);
            }
            Err(errors) => missing_required_roles.extend(errors),
        }
    }

    let forbidden_roots_present = ["docs", "migration", "oss", "donors", "references"]
        .iter()
        .filter(|path| root.join(path).exists())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    let manifest_ready = manifest.is_some() && policy_violations.is_empty();
    let ready = missing_required_roles.is_empty()
        && missing_mapped_paths.is_empty()
        && forbidden_roots_present.is_empty()
        && manifest_ready;
    Ok(RepoAudit {
        schema: "atlas.systemizer.repo-audit.v6".into(),
        archetype,
        manifest,
        manifest_ready,
        policy_violations,
        missing_required_roles,
        missing_mapped_paths,
        forbidden_roots_present,
        ready,
    })
}

/// The control-document paths (`.atlas`-relative) `audit_docs` treats as required for
/// `gate_ready`. Shared between the real walk below and the "no `.atlas` directory at all" early
/// report so the two paths can never silently disagree about what "required" means.
const REQUIRED_CONTROL_DOCS: [&str; 20] = [
    "README.md",
    "INDEX.md",
    "TEMPLATE.md",
    "architecture/README.md",
    "architecture/SYSTEM.md",
    "architecture/constitution/NORTH-STAR.md",
    "blueprints/SYSTEM-BLUEPRINT.md",
    "blueprints/PHYSICAL-REFOUNDATION.md",
    "contracts/SYSTEM-CONTRACT.md",
    "contracts/SEMANTIC-FACTS.md",
    "contracts/SEMANTIC-EXTRACTION.md",
    "contracts/NORMALIZATION.md",
    "contracts/CENSUS-COMPLETENESS.md",
    "contracts/CENSUS-CERTIFICATE.md",
    "decisions/README.md",
    "decisions/0001-one-normalized-semantic-path.md",
    "decisions/0002-epistemic-status-model.md",
    "roadmap/ROADMAP.md",
    "guides/DEVELOPMENT.md",
    "references/README.md",
];

pub fn audit_docs(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    let root = root.as_ref();
    // A target repository this bootstrap has never been pointed at before (or any repository
    // never admitted into Atlas at all) has no `.atlas` directory whatsoever -- not an empty one,
    // not an incomplete one, simply absent. `Path::canonicalize()` and `fs::read_dir` both require
    // the path to exist, so calling either on a genuinely missing root would raise a raw,
    // contextless `io::ErrorKind::NotFound` that propagates uncaught to the CLI's top level. Every
    // other "declared but not present" fact in this codebase is reported explicitly rather than
    // treated as fatal (a missing Cargo.lock is `DependencyClosureState::NotApplicable`; a missing
    // individual required doc is a `required_control_docs_missing` entry) -- an absent `.atlas`
    // directory is the same class of fact, reported the same way: every required doc missing, zero
    // documents observed, gate not ready.
    if !root.is_dir() {
        let required_control_docs_missing = REQUIRED_CONTROL_DOCS
            .iter()
            .map(|path| (*path).to_owned())
            .collect::<Vec<_>>();
        let hard_violations_total = required_control_docs_missing.len();
        return Ok(DocsReport {
            schema: "atlas.systemizer.docs-report.v2".into(),
            standard: "atlas.docs.v1".into(),
            root: root.to_string_lossy().into_owned(),
            gate_ready: false,
            hard_violations_total,
            documents_total: 0,
            canonical_frontmatter_total: 0,
            required_control_docs_missing,
            missing_frontmatter: Vec::new(),
            documents: Vec::new(),
        });
    }
    let root = root.canonicalize()?;
    let required = REQUIRED_CONTROL_DOCS;
    let required_control_docs_missing = required
        .iter()
        .filter(|path| !root.join(path).is_file())
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    let mut documents_total = 0;
    let mut canonical_frontmatter_total = 0;
    let mut missing_frontmatter = Vec::new();
    let mut documents = Vec::new();
    visit_docs(
        &root,
        &root,
        &mut documents_total,
        &mut canonical_frontmatter_total,
        &mut missing_frontmatter,
        &mut documents,
    )?;
    let hard_violations_total = required_control_docs_missing.len() + missing_frontmatter.len();
    Ok(DocsReport {
        schema: "atlas.systemizer.docs-report.v2".into(),
        standard: "atlas.docs.v1".into(),
        root: root.to_string_lossy().into_owned(),
        gate_ready: hard_violations_total == 0,
        hard_violations_total,
        documents_total,
        canonical_frontmatter_total,
        required_control_docs_missing,
        missing_frontmatter,
        documents,
    })
}

fn has_frontmatter(text: &str) -> bool {
    text.starts_with("---\n") || text.starts_with("---\r\n")
}

fn frontmatter(text: &str) -> BTreeMap<String, String> {
    if !has_frontmatter(text) {
        return BTreeMap::new();
    }
    let body = if let Some(rest) = text.strip_prefix("---\r\n") {
        rest
    } else {
        text.strip_prefix("---\n").unwrap_or(text)
    };
    let Some(end) = body.find("\n---") else {
        return BTreeMap::new();
    };
    let mut fields = BTreeMap::new();
    for line in body[..end].lines() {
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            );
        }
    }
    fields
}

fn markdown_title(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("# ").map(|title| title.trim().to_owned()))
}

fn markdown_headings(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('#') {
                return None;
            }
            let heading = trimmed.trim_start_matches('#').trim();
            if heading.is_empty() {
                None
            } else {
                Some(heading.to_owned())
            }
        })
        .collect()
}

fn path_references(text: &str) -> Vec<String> {
    let mut references = Vec::new();
    for root in ["core/", "runtime/", "adapter/", "apps/studio/"] {
        let mut cursor = 0;
        while let Some(offset) = text[cursor..].find(root) {
            let start = cursor + offset;
            let tail = &text[start..];
            let end = tail
                .find(|ch: char| {
                    !(ch.is_ascii_alphanumeric() || matches!(ch, '/' | '\\' | '.' | '_' | '-'))
                })
                .unwrap_or(tail.len());
            let candidate = tail[..end].replace('\\', "/");
            if candidate.contains('.') && !references.contains(&candidate) {
                references.push(candidate);
            }
            cursor = start + end.max(root.len());
        }
    }
    references
}

fn visit_docs(
    root: &Path,
    dir: &Path,
    documents_total: &mut usize,
    canonical_frontmatter_total: &mut usize,
    missing_frontmatter: &mut Vec<String>,
    documents: &mut Vec<DocumentFact>,
) -> io::Result<()> {
    // Same discipline as `visit_adl_sources`/`adapter::source::visit_inventory`: a subdirectory
    // this process cannot list must not abort auditing every other doc in the tree. Best-effort
    // skip (no per-directory accounting channel exists here), never a crash.
    let Ok(read_dir) = fs::read_dir(dir) else {
        return Ok(());
    };
    let Ok(mut entries) = read_dir.collect::<Result<Vec<_>, _>>() else {
        return Ok(());
    };
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if matches!(name.as_str(), "temporary" | "provenance" | "licenses") {
                continue;
            }
            visit_docs(
                root,
                &path,
                documents_total,
                canonical_frontmatter_total,
                missing_frontmatter,
                documents,
            )?;
            continue;
        }
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        *documents_total += 1;
        // Same discipline as `visit_adl_sources`/`audit_repository`: a doc whose bytes are not
        // valid UTF-8 must not abort auditing every other doc in the tree. Lossy-decoded, never
        // dropped -- it still counts toward `documents_total` and gets a `DocumentFact` below.
        let text = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let meta = frontmatter(&text);
        if has_frontmatter(&text) {
            *canonical_frontmatter_total += 1;
        } else {
            missing_frontmatter.push(relative.clone());
        }
        documents.push(DocumentFact {
            path: format!(".atlas/{relative}"),
            id: meta.get("id").cloned(),
            kind: meta.get("type").cloned(),
            status: meta.get("status").cloned(),
            canonical: meta.get("canonical").map(String::as_str) == Some("true"),
            title: markdown_title(&text),
            headings: markdown_headings(&text),
            references: path_references(&text),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_root() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("atlas-audit-docs-{}-{nonce}", std::process::id()))
    }

    // Falsification: a target repository `atlas-systemizer` has never been pointed at before has
    // no `.atlas` directory at all -- not an empty one, not an incomplete one, simply absent. Every
    // other "declared but not actually present" case in this codebase (a missing Cargo.lock, a
    // missing declared source root, a missing individual required doc) is reported as an explicit,
    // typed, non-fatal fact; this one instead let `Path::canonicalize()` raise a raw
    // `io::ErrorKind::NotFound`, propagated by `?` all the way to `main`, which prints only the raw
    // OS message ("No such file or directory (os error 2)") with no path and no explanation, and
    // exits nonzero -- the single most common real first-time input (a fresh, not-yet-admitted
    // repository) crashed the whole CLI instead of producing the same kind of report an
    // existing-but-incomplete `.atlas` directory already receives.
    #[test]
    fn a_completely_missing_atlas_directory_is_reported_not_fatal() {
        let root = scratch_root();
        std::fs::create_dir_all(&root).unwrap();
        let docs_root = root.join(".atlas");
        assert!(!docs_root.exists());

        let report = audit_docs(&docs_root).expect(
            "a missing .atlas directory must be reported as a not-ready DocsReport, \
             never a raw io::Error",
        );

        assert!(!report.gate_ready);
        assert_eq!(report.documents_total, 0);
        assert!(!report.required_control_docs_missing.is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    // Falsification: `runtime::census::extraction::extract_semantics` already established the
    // right discipline for exactly this class of input -- "never `?`-propagate [a file read
    // failure]: one artifact's read failure must not abort extraction of every artifact that comes
    // after it" (see its own doc comment) -- but `read_adl_sources` did not follow it: a single
    // `.adl` file containing bytes that are not valid UTF-8 (a hostile or merely corrupted
    // encoding, the same "malformed/hostile inputs" scenario class already exercised elsewhere
    // this session) made `fs::read_to_string` return `Err`, propagated by `?` straight through
    // `read_adl_sources` -> `systemize`/`code_analyze`/`check`/`parse`, aborting the ENTIRE run
    // (every other file in the repository, ADL or otherwise) with no report at all -- confirmed
    // against the unfixed code before writing the fix.
    #[test]
    fn a_non_utf8_adl_file_does_not_abort_reading_the_rest_of_the_declared_tree() {
        let root = scratch_root();
        let declared = root.join(".atlas").join("declared");
        std::fs::create_dir_all(&declared).unwrap();
        std::fs::write(
            declared.join("valid.adl"),
            "atlas 1\ncomponent Foo {\n  path = \"x\"\n}\n",
        )
        .unwrap();
        // Not valid UTF-8: a lone continuation byte can never start a valid sequence.
        std::fs::write(declared.join("corrupt.adl"), [b'a', b'\xff', b'\xfe', b'z']).unwrap();

        let sources = read_adl_sources(&root)
            .expect("a non-UTF-8 .adl file must not abort reading the rest of the declared tree");

        assert_eq!(sources.len(), 2);
        assert!(
            sources
                .iter()
                .any(|source| source.path == ".atlas/declared/valid.adl"
                    && source.text.contains("component Foo")),
            "the well-formed sibling file must still be read intact"
        );
        assert!(
            sources
                .iter()
                .any(|source| source.path == ".atlas/declared/corrupt.adl"),
            "the corrupt file must still be accounted for, never silently dropped"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    // Same defect, one layer up the stack: `.atlas/repo.toml` is read before `read_adl_sources`
    // even runs, so a non-UTF-8 manifest previously aborted `audit_repository` -- and therefore
    // every caller of it (`systemize`, `code_analyze`) -- before any report could be built at all.
    #[test]
    fn a_non_utf8_repo_manifest_does_not_abort_the_repository_audit() {
        let root = scratch_root();
        let atlas_root = root.join(".atlas");
        std::fs::create_dir_all(&atlas_root).unwrap();
        std::fs::write(atlas_root.join("repo.toml"), [b'a', b'\xff', b'\xfe', b'z']).unwrap();

        let audit = audit_repository(&root)
            .expect("a non-UTF-8 repo.toml must not abort the repository audit");
        // The file exists (it was read, lossily, not crashed on) so it must never be reported
        // via the "repository_manifest role missing" path -- that path is reserved for the file
        // genuinely not existing at all, a distinct fact from "exists but fails to parse".
        assert!(
            !audit
                .missing_required_roles
                .iter()
                .any(|role| role == "repository_manifest"),
            "an existing-but-malformed manifest must be distinguished from a missing one, got: {:?}",
            audit.missing_required_roles
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    // Same defect, in the docs walker: a single non-UTF-8 `.md` file previously aborted
    // `audit_docs` for the whole `.atlas` tree, hiding every other document's real status.
    #[test]
    fn a_non_utf8_doc_does_not_abort_auditing_the_rest_of_the_tree() {
        let root = scratch_root();
        let docs_root = root.join(".atlas");
        std::fs::create_dir_all(&docs_root).unwrap();
        std::fs::write(docs_root.join("README.md"), "# ok\n").unwrap();
        std::fs::write(docs_root.join("corrupt.md"), [b'a', b'\xff', b'\xfe', b'z']).unwrap();

        let report = audit_docs(&docs_root)
            .expect("a non-UTF-8 doc must not abort auditing the rest of the tree");
        assert_eq!(report.documents_total, 2);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parses_atlas_manifest_policy_fields() {
        let manifest = parse_repo_manifest(
            r#"schema = "atlas.repo.v2"
repo = "org/repo"
system_kind = "SYSTEM_INVENTION_FORGE"
backend_language = "rust"
frontend_language = "typescript"
coding_requires_docs_gate = true
graph_before_code_required = true
exact_base_sha_required = true
single_repository_target_required = true
knowledge_root = ".atlas"
temporary_root = ".atlas/temporary"
provenance_root = ".atlas/provenance"
license_root = ".atlas/licenses"

[code]
source_roots = ["core"]
backend_roots = ["core"]
frontend_roots = ["apps/studio"]
test_roots = ["core/tests", "runtime/tests", "adapter/tests", "apps/studio/src"]
"#,
        )
        .unwrap();
        assert!(validate_manifest(&manifest).is_empty());
    }
}
