//! Atlas adapters for external repository mechanics.

use atlas_core::{
    AdlSource, CONTROL_ROOT, DECLARED_ROOT, DocsReport, DocumentFact, REPO_MANIFEST_PATH,
    RepoAudit, RepoManifest, validate_manifest,
};
use std::{collections::BTreeMap, fs, io, path::Path};

pub mod browser;
pub mod dependency;
pub mod knowledge;
pub mod semantic;
pub mod source;
pub mod vcs;
pub mod weights;

pub use dependency::{
    DeclaredSource, ManifestTargets, WorkspaceDependencies, census_cargo_workspace,
    manifest_targets, workspace_dependencies,
};
pub use semantic::rust::RUST_SEMANTIC_EXTRACTOR_ID;
pub use semantic::typescript::TYPESCRIPT_SEMANTIC_EXTRACTOR_ID;
pub use semantic::{
    CrateInput, DiagnosticCode, ExtractionBatch, ExtractionDiagnostic, ExtractionInput, FnTarget,
    ImportBinding, LockedTraitMethods, ModuleFacts, ObligationResult, PackageDeclaration,
    PackageRead, PackageReading, PathCallOutcome, PathCallResolution, ReleaseResolution,
    SemanticExtractor, StaticUnsupportedExtractor, WithheldPath, WorkspaceResolution,
    extractors_for_language, join_path, module_facts, read_locked_trait_methods, resolve_imports,
    resolve_path_calls, resolve_specifier, resolve_workspace, resolve_workspace_with,
    semantic_extractors, workspace_packages,
};
pub use source::{
    IncludedFragment, SourceFrontend, SourceFrontendMatch, inventory_declared_source,
    inventory_source, resolve_included_fragment, resolve_source_frontend, scan_declared_source,
    scan_source, source_frontend_for_language, source_frontends, source_report_from_inventory,
};
pub use vcs::snapshot_git;

/// BLAKE3 digest of every source file the semantic extractors are built from, plus `Cargo.lock`
/// (computed by `build.rs`, ADR 0008). Any code or pinned-dependency change changes it.
pub const EXTRACTOR_BUILD_DIGEST: &str = env!("ATLAS_EXTRACTOR_BUILD_DIGEST");

pub fn read_adl_sources(root: impl AsRef<Path>) -> io::Result<Vec<AdlSource>> {
    let root = root.as_ref().canonicalize()?;
    let declared = root.join(DECLARED_ROOT);
    let mut sources = Vec::new();
    if declared.is_dir() {
        visit_adl_sources(&root, &declared, &mut sources, 0)?;
    }
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(sources)
}

fn visit_adl_sources(
    root: &Path,
    dir: &Path,
    out: &mut Vec<AdlSource>,
    depth: usize,
) -> io::Result<()> {
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
        // `entry.file_type()` itself can fail (the entry vanishing between being listed and
        // being typed, or a permission oddity) -- same "never abort every other entry" discipline
        // as the `read_dir` calls above; skip just this one entry rather than propagating.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            // Same empirically-justified guard as `adapter::source::visit_inventory`'s own
            // `MAX_DIRECTORY_NESTING_DEPTH` (see its doc comment): unbounded recursion here is the
            // identical stack-overflow-in-debug-profile risk, just on the `tools/atlas/declared` tree
            // instead of a repository's own source tree. Best-effort skip past the limit (no
            // per-directory accounting channel exists here, matching this function's own existing
            // best-effort-skip discipline for every other directory failure mode).
            if depth >= source::MAX_DIRECTORY_NESTING_DEPTH {
                continue;
            }
            visit_adl_sources(root, &path, out, depth + 1)?;
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
        // failure must not abort extraction of every artifact that comes after it"). Two distinct
        // failure modes share this same discipline here: `fs::read` itself can fail (permission
        // denial or the file vanishing -- skip this one file, same as any other entry-level
        // failure above), and a successfully-read file's bytes are not guaranteed to be valid
        // UTF-8 (a hostile or merely corrupted encoding), handled by `from_utf8_lossy` -- the same
        // lossy-but-never-crashing decode `Path::to_string_lossy` already uses elsewhere in this
        // codebase: the file remains present in `sources` (never silently dropped) and its valid
        // portions stay byte-for-byte intact; only genuinely undecodable byte sequences become
        // U+FFFD, which then naturally surfaces through `parse_adl_source`'s normal diagnostics
        // (e.g. `ATLAS-E000`/`ATLAS-E010`) rather than aborting the whole read.
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
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
    let control_root = root.join(CONTROL_ROOT);
    let repo_manifest = root.join(REPO_MANIFEST_PATH);
    let mut missing_required_roles = Vec::new();
    let mut missing_mapped_paths = Vec::new();
    if !control_root.is_dir() {
        missing_required_roles.push("atlas_control_root".into());
        missing_mapped_paths.push(CONTROL_ROOT.into());
    }
    if !repo_manifest.is_file() {
        missing_required_roles.push("repository_manifest".into());
        missing_mapped_paths.push(REPO_MANIFEST_PATH.into());
    }

    let mut archetype = "UNKNOWN".to_owned();
    let mut manifest = None;
    let mut policy_violations = Vec::new();
    if repo_manifest.is_file() {
        // Same discipline as `visit_adl_sources`/`visit_docs`: `fs::read` itself can fail
        // (permission denial being the common real case even though `is_file()` above confirmed
        // the manifest exists), and a manifest whose bytes ARE readable are not guaranteed to be
        // valid UTF-8 -- neither must abort the whole repository audit. An unreadable manifest is
        // reported the same way a manifest that fails to parse already is (via
        // `missing_required_roles`), since both are "the manifest exists but this audit could not
        // use it" -- a distinct fact from "repository_manifest" (the file genuinely absent) above.
        match fs::read(&repo_manifest) {
            Ok(bytes) => match parse_repo_manifest(&String::from_utf8_lossy(&bytes)) {
                Ok(parsed) => {
                    archetype = parsed.system_kind.clone();
                    policy_violations = validate_manifest(&parsed);
                    manifest = Some(parsed);
                }
                Err(errors) => missing_required_roles.extend(errors),
            },
            Err(error) => {
                missing_required_roles.push(format!("repository_manifest_unreadable:{error}"));
            }
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
        schema: "atlas.systemizer.repo-audit.v7".into(),
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

/// The control-document paths (relative to the control-document root, `CONTROL_DOCUMENT_ROOT`)
/// `audit_docs` and `audit_control_knowledge` treat as required for `gate_ready`. Shared between
/// the Markdown walk, the "no directory at all" early report and the knowledge audit so they can
/// never silently disagree about what "required" means.
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

/// The docs gate over a directory of Markdown control documents, for a repository that still keeps
/// them as files (`docs audit --root <dir>`). A repository whose control documents are knowledge
/// is audited by `audit_control_knowledge` (ADR 0104). Document paths are reported under
/// `CONTROL_DOCUMENT_ROOT`.
pub fn audit_docs(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    let root = root.as_ref();
    // A target repository this bootstrap has never been pointed at before (or any repository
    // never admitted into Atlas at all) has no control-document directory whatsoever -- not an empty one,
    // not an incomplete one, simply absent. `Path::canonicalize()` and `fs::read_dir` both require
    // the path to exist, so calling either on a genuinely missing root would raise a raw,
    // contextless `io::ErrorKind::NotFound` that propagates uncaught to the CLI's top level. Every
    // other "declared but not present" fact in this codebase is reported explicitly rather than
    // treated as fatal (a missing Cargo.lock is `DependencyClosureState::NotApplicable`; a missing
    // individual required doc is a `required_control_docs_missing` entry) -- an absent
    // control-document directory is the same class of fact, reported the same way: every required doc missing, zero
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
        0,
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

/// The knowledge identity of the Atlas control documents: the root they were read under when
/// `ynventa knowledge extract` turned them into facts, before `.atlas` was retired (ADR 0104).
pub const CONTROL_DOCUMENT_ROOT: &str = ".atlas";

/// The docs gate over a repository's knowledge (ADR 0104): its control documents are Ynventa
/// knowledge, not files. Every required control document must be an extracted document under
/// `CONTROL_DOCUMENT_ROOT`, and every extracted control document must carry the `id` its
/// frontmatter declared. A document's metadata is read back from its facts: `id`, `type`, `status`
/// and `canonical` from its frontmatter definitions (a decision record's status from its DECISION
/// facts), its title and headings from its `markdown_title` / `markdown_headings` definitions
/// (recorded from the Markdown when `.atlas` was retired; otherwise the decision title and the
/// sections its facts sit under), and its code references from the text of its facts. A
/// repository without knowledge is not ready.
pub fn audit_control_knowledge(root: impl AsRef<Path>) -> io::Result<DocsReport> {
    let root = root.as_ref();
    let knowledge = knowledge::Knowledge::read(root)?;
    let prefix = format!("{CONTROL_DOCUMENT_ROOT}/");
    let extracted = knowledge.documents();
    let paths: Vec<&str> = extracted
        .keys()
        .copied()
        .filter(|p| p.starts_with(&prefix))
        .collect();
    let mut definitions: BTreeMap<&str, BTreeMap<&str, &str>> = BTreeMap::new();
    let mut decisions: BTreeMap<&str, BTreeMap<&str, &str>> = BTreeMap::new();
    let mut sections: BTreeMap<&str, Vec<(usize, &str)>> = BTreeMap::new();
    let mut texts: BTreeMap<&str, Vec<(usize, &str)>> = BTreeMap::new();
    for fact in &knowledge.facts {
        match fact.kind {
            knowledge::FactKind::Definition => {
                definitions
                    .entry(fact.subject.as_str())
                    .or_default()
                    .insert(fact.key.as_str(), fact.value.as_str());
            }
            knowledge::FactKind::Decision => {
                decisions
                    .entry(fact.subject.as_str())
                    .or_default()
                    .insert(fact.key.as_str(), fact.value.as_str());
            }
            _ => {}
        }
        for provenance in &fact.provenance {
            let Some(at) = provenance.strip_prefix("doc:") else {
                continue;
            };
            let (path, lines) = at.split_once("#L").unwrap_or((at, ""));
            if !path.starts_with(&prefix) {
                continue;
            }
            let line = lines
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(usize::MAX);
            texts
                .entry(path)
                .or_default()
                .push((line, fact.value.as_str()));
            if let Some(section) = fact
                .subject
                .strip_prefix(path)
                .and_then(|s| s.strip_prefix(" / "))
            {
                sections.entry(path).or_default().push((line, section));
            }
        }
    }
    let empty = BTreeMap::new();
    let mut documents = Vec::new();
    let mut missing_frontmatter = Vec::new();
    for path in &paths {
        let relative = &path[prefix.len()..];
        let defined = definitions.get(path).unwrap_or(&empty);
        let stem = relative
            .rsplit('/')
            .next()
            .unwrap_or(relative)
            .trim_end_matches(".md");
        let decided = decisions.get(stem).filter(|_| {
            relative.split('/').any(|segment| segment == "decisions")
                && !relative.ends_with("README.md")
        });
        if !defined.contains_key("id") {
            missing_frontmatter.push(relative.to_owned());
        }
        let mut ordered = sections.get(path).cloned().unwrap_or_default();
        ordered.sort();
        let mut headings: Vec<String> = Vec::new();
        for (_, section) in &ordered {
            for heading in section.split(" / ") {
                if !headings.iter().any(|h| h == heading) {
                    headings.push(heading.to_owned());
                }
            }
        }
        let title = match defined.get("markdown_headings") {
            // The Markdown structure was recorded: its `# ` title, if it had one, and its headings.
            Some(recorded) => {
                headings = recorded.lines().map(str::to_owned).collect();
                defined.get("markdown_title").map(|t| (*t).to_owned())
            }
            None => defined
                .get("markdown_title")
                .or_else(|| decided.and_then(|d| d.get("title")))
                .map(|t| (*t).to_owned())
                .or_else(|| headings.first().cloned()),
        };
        let status = decided
            .and_then(|d| d.get("status"))
            .or_else(|| defined.get("status"))
            .map(|s| (*s).to_owned());
        // The document's text as its facts hold it, in document order: references come out as
        // `path_references` finds them in a Markdown file.
        let mut text = texts.get(path).cloned().unwrap_or_default();
        text.sort();
        let text = text.iter().map(|(_, t)| *t).collect::<Vec<_>>().join("\n");
        let references = path_references(&text);
        documents.push(DocumentFact {
            path: (*path).to_owned(),
            id: defined.get("id").map(|s| (*s).to_owned()),
            kind: defined.get("type").map(|s| (*s).to_owned()),
            status,
            canonical: defined.get("canonical") == Some(&"true"),
            title,
            headings,
            references,
        });
    }
    let required_control_docs_missing = REQUIRED_CONTROL_DOCS
        .iter()
        .filter(|relative| !extracted.contains_key(format!("{prefix}{relative}").as_str()))
        .map(|relative| (*relative).to_owned())
        .collect::<Vec<_>>();
    let hard_violations_total = required_control_docs_missing.len() + missing_frontmatter.len();
    Ok(DocsReport {
        schema: "atlas.systemizer.docs-report.v3".into(),
        standard: "ynventa.knowledge.v1".into(),
        root: root
            .join(knowledge::KNOWLEDGE_DIR)
            .to_string_lossy()
            .into_owned(),
        gate_ready: hard_violations_total == 0,
        hard_violations_total,
        documents_total: documents.len(),
        canonical_frontmatter_total: documents.len() - missing_frontmatter.len(),
        required_control_docs_missing,
        missing_frontmatter,
        documents,
    })
}

fn has_frontmatter(text: &str) -> bool {
    text.starts_with("---\n") || text.starts_with("---\r\n")
}

/// The text between a WELL-FORMED frontmatter block's opening and closing `---` delimiters, or
/// `None` if either is missing. Deliberately distinct from `has_frontmatter` (which only checks
/// the opening delimiter): a doc that opens frontmatter but never closes it must not be treated as
/// having real, parseable frontmatter just because it starts with `---`.
fn closed_frontmatter_body(text: &str) -> Option<&str> {
    if !has_frontmatter(text) {
        return None;
    }
    let body = if let Some(rest) = text.strip_prefix("---\r\n") {
        rest
    } else {
        text.strip_prefix("---\n").unwrap_or(text)
    };
    let end = body.find("\n---")?;
    Some(&body[..end])
}

/// Whether `text` contains a well-formed frontmatter block (both delimiters present), regardless
/// of how many real fields it then parses to -- an empty-but-closed block (`---\n---\n`) is still
/// well-formed markup, distinct from an unclosed one. `canonical_frontmatter_total`/
/// `missing_frontmatter` classify by this, not by `has_frontmatter` alone: before this fix, a doc
/// that opened frontmatter but never closed it silently counted toward `canonical_frontmatter_total`
/// (and was excluded from `missing_frontmatter`) even though `frontmatter()` parses zero real
/// fields from it -- silently passing `docs_audit`'s own `gate_ready` computation
/// (`hard_violations_total = required_control_docs_missing.len() + missing_frontmatter.len()`) for
/// genuinely malformed markup.
fn has_closed_frontmatter(text: &str) -> bool {
    closed_frontmatter_body(text).is_some()
}

fn frontmatter(text: &str) -> BTreeMap<String, String> {
    let Some(body) = closed_frontmatter_body(text) else {
        return BTreeMap::new();
    };
    let mut fields = BTreeMap::new();
    for line in body.lines() {
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
    for root in ["core/", "adapter/", "apps/studio/"] {
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
    depth: usize,
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
        // `entry.file_type()` itself can fail (entry vanishing mid-walk, a permission oddity) --
        // same "never abort every other entry" discipline as the `read_dir` calls above.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // G152: `.cache` is gitignored scratch (packed containers, a materialized donor), never
            // canonical documentation.
            if matches!(
                name.as_str(),
                "temporary" | "provenance" | "licenses" | ".cache"
            ) {
                continue;
            }
            // Same empirically-justified guard as `adapter::source::visit_inventory`'s own
            // `MAX_DIRECTORY_NESTING_DEPTH` (see its doc comment) -- unbounded recursion here is
            // the identical stack-overflow-in-debug-profile risk, just walking a control-document
            // directory. Best-effort skip past the limit, matching this function's own existing
            // best-effort-skip discipline for every other directory failure mode.
            if depth >= source::MAX_DIRECTORY_NESTING_DEPTH {
                continue;
            }
            visit_docs(
                root,
                &path,
                documents_total,
                canonical_frontmatter_total,
                missing_frontmatter,
                documents,
                depth + 1,
            )?;
            continue;
        }
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        // Same discipline as `visit_adl_sources`/`audit_repository`: `fs::read` itself can fail
        // (permission denial or the file vanishing) -- skip this one doc, never abort auditing
        // every other doc in the tree. Only counted toward `documents_total` once actually read,
        // so a skipped doc is never silently miscounted as an audited-but-unreported one.
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        *documents_total += 1;
        // A doc whose bytes are not valid UTF-8 must not abort auditing every other doc in the
        // tree either. Lossy-decoded, never dropped.
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let meta = frontmatter(&text);
        if has_closed_frontmatter(&text) {
            *canonical_frontmatter_total += 1;
        } else {
            missing_frontmatter.push(relative.clone());
        }
        documents.push(DocumentFact {
            path: format!("{CONTROL_DOCUMENT_ROOT}/{relative}"),
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

    // Mirrors `adapter::source::tests::running_as_root` (duplicated rather than shared, matching
    // this file's own existing `scratch_root` precedent of a small per-module test helper rather
    // than new cross-module plumbing for test-only code). Root bypasses discretionary directory
    // permission checks entirely on Linux, so a `chmod 000` file remains fully readable to it and
    // a permission-denied-read test cannot be exercised meaningfully under it.
    #[cfg(unix)]
    fn running_as_root() -> bool {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|status| {
                status
                    .lines()
                    .find_map(|line| line.strip_prefix("Uid:"))
                    .and_then(|rest| rest.split_whitespace().next())
                    .map(|uid| uid == "0")
            })
            .unwrap_or(false)
    }

    // Falsification: a target repository `atlas-systemizer` has never been pointed at before has
    // no control-document directory at all -- not an empty one, not an incomplete one, simply
    // absent. Every other "declared but not actually present" case in this codebase (a missing Cargo.lock, a
    // missing declared source root, a missing individual required doc) is reported as an explicit,
    // typed, non-fatal fact; this one instead let `Path::canonicalize()` raise a raw
    // `io::ErrorKind::NotFound`, propagated by `?` all the way to `main`, which prints only the raw
    // OS message ("No such file or directory (os error 2)") with no path and no explanation, and
    // exits nonzero -- the single most common real first-time input (a fresh, not-yet-admitted
    // repository) crashed the whole CLI instead of producing the same kind of report an
    // existing-but-incomplete control-document directory already receives.
    #[test]
    fn a_completely_missing_control_document_directory_is_reported_not_fatal() {
        let root = scratch_root();
        std::fs::create_dir_all(&root).unwrap();
        let docs_root = root.join("control-docs");
        assert!(!docs_root.exists());

        let report = audit_docs(&docs_root).expect(
            "a missing control-document directory must be reported as a not-ready DocsReport, \
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
        let declared = root.join(DECLARED_ROOT);
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
                .any(|source| source.path == "tools/atlas/declared/valid.adl"
                    && source.text.contains("component Foo")),
            "the well-formed sibling file must still be read intact"
        );
        assert!(
            sources
                .iter()
                .any(|source| source.path == "tools/atlas/declared/corrupt.adl"),
            "the corrupt file must still be accounted for, never silently dropped"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    // Same defect, one layer up the stack: `tools/atlas/repo.toml` is read before `read_adl_sources`
    // even runs, so a non-UTF-8 manifest previously aborted `audit_repository` -- and therefore
    // every caller of it (`systemize`, `code_analyze`) -- before any report could be built at all.
    #[test]
    fn a_non_utf8_repo_manifest_does_not_abort_the_repository_audit() {
        let root = scratch_root();
        let control_root = root.join(CONTROL_ROOT);
        std::fs::create_dir_all(&control_root).unwrap();
        std::fs::write(
            root.join(REPO_MANIFEST_PATH),
            [b'a', b'\xff', b'\xfe', b'z'],
        )
        .unwrap();

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
    // `audit_docs` for the whole control-document tree, hiding every other document's real status.
    #[test]
    fn a_non_utf8_doc_does_not_abort_auditing_the_rest_of_the_tree() {
        let root = scratch_root();
        let docs_root = root.join("control-docs");
        std::fs::create_dir_all(&docs_root).unwrap();
        std::fs::write(docs_root.join("README.md"), "# ok\n").unwrap();
        std::fs::write(docs_root.join("corrupt.md"), [b'a', b'\xff', b'\xfe', b'z']).unwrap();

        let report = audit_docs(&docs_root)
            .expect("a non-UTF-8 doc must not abort auditing the rest of the tree");
        assert_eq!(report.documents_total, 2);

        std::fs::remove_dir_all(&root).unwrap();
    }

    // Falsification: distinct from the three non-UTF-8 tests above, which only exercise a
    // successful `fs::read` returning garbled content. Confirmed against the code as it stood
    // immediately after those three fixes, via a real non-root process (`su ubuntu -c
    // '.../atlas-systemizer systemize --root ...'`), that `fs::read` itself FAILING (permission
    // denial on a `.adl` file) still aborted the whole `systemize` run with "Permission denied
    // (os error 13)" and zero output -- the `from_utf8_lossy` fix only ever saw bytes that were
    // successfully read. Same root-detection skip as `adapter::source::tests`; the real-process
    // verification is the authoritative evidence for this fix.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_adl_file_does_not_abort_reading_the_rest_of_the_declared_tree() {
        if running_as_root() {
            eprintln!(
                "skipping an_unreadable_adl_file_does_not_abort_reading_the_rest_of_the_declared_tree: \
                 running as root, which bypasses the permission check this test exercises"
            );
            return;
        }
        use std::os::unix::fs::PermissionsExt;

        let root = scratch_root();
        let declared = root.join(DECLARED_ROOT);
        std::fs::create_dir_all(&declared).unwrap();
        std::fs::write(declared.join("valid.adl"), "atlas 1\n").unwrap();
        let locked = declared.join("locked.adl");
        std::fs::write(&locked, "atlas 1\n").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let sources = read_adl_sources(&root)
            .expect("an unreadable .adl file must not abort reading the rest of the tree");
        assert_eq!(
            sources.len(),
            1,
            "the unreadable file is skipped (no accounting channel exists here); the readable \
             sibling must still be read: {sources:?}"
        );
        assert_eq!(sources[0].path, "tools/atlas/declared/valid.adl");

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_repo_manifest_does_not_abort_the_repository_audit() {
        if running_as_root() {
            eprintln!(
                "skipping an_unreadable_repo_manifest_does_not_abort_the_repository_audit: \
                 running as root, which bypasses the permission check this test exercises"
            );
            return;
        }
        use std::os::unix::fs::PermissionsExt;

        let root = scratch_root();
        let control_root = root.join(CONTROL_ROOT);
        std::fs::create_dir_all(&control_root).unwrap();
        let manifest = root.join(REPO_MANIFEST_PATH);
        std::fs::write(&manifest, "schema = \"atlas.repo.v3\"\n").unwrap();
        std::fs::set_permissions(&manifest, std::fs::Permissions::from_mode(0o000)).unwrap();

        let audit = audit_repository(&root)
            .expect("an unreadable repo.toml must not abort the repository audit");
        // The file exists (it was found and an unreadable-read was attempted, not crashed on) so
        // it must never be reported via the "repository_manifest role missing" path -- reserved
        // for the file genuinely not existing, a distinct fact from "exists but unreadable".
        assert!(
            !audit
                .missing_required_roles
                .iter()
                .any(|role| role == "repository_manifest"),
            "an existing-but-unreadable manifest must be distinguished from a missing one, got: {:?}",
            audit.missing_required_roles
        );

        std::fs::set_permissions(&manifest, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_doc_does_not_abort_auditing_the_rest_of_the_tree() {
        if running_as_root() {
            eprintln!(
                "skipping an_unreadable_doc_does_not_abort_auditing_the_rest_of_the_tree: \
                 running as root, which bypasses the permission check this test exercises"
            );
            return;
        }
        use std::os::unix::fs::PermissionsExt;

        let root = scratch_root();
        let docs_root = root.join("control-docs");
        std::fs::create_dir_all(&docs_root).unwrap();
        std::fs::write(docs_root.join("README.md"), "# ok\n").unwrap();
        let locked = docs_root.join("locked.md");
        std::fs::write(&locked, "# secret\n").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let report = audit_docs(&docs_root)
            .expect("an unreadable doc must not abort auditing the rest of the tree");
        assert_eq!(
            report.documents_total, 1,
            "the unreadable doc is skipped (no accounting channel exists here); the readable \
             sibling must still be counted"
        );

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    // `has_frontmatter` only checks the OPENING `---` delimiter. A doc that opens frontmatter but
    // never closes it previously counted toward `canonical_frontmatter_total` (and was excluded
    // from `missing_frontmatter`) even though `frontmatter()` itself parses zero real fields from
    // it -- silently passing `docs_audit`'s own `gate_ready` computation for malformed markup.
    #[test]
    fn a_doc_with_unclosed_frontmatter_is_not_counted_as_canonical() {
        let root = scratch_root();
        let docs_root = root.join("control-docs");
        std::fs::create_dir_all(&docs_root).unwrap();
        std::fs::write(
            docs_root.join("broken.md"),
            "---\nid: atlas.broken\nstatus: active\n\n# Broken\n\nNo closing delimiter above.\n",
        )
        .unwrap();

        let report = audit_docs(&docs_root).unwrap();
        assert_eq!(report.documents_total, 1);
        assert_eq!(
            report.canonical_frontmatter_total, 0,
            "unclosed frontmatter must never be counted as canonical -- zero real fields were \
             actually parsed from it"
        );
        assert_eq!(
            report.missing_frontmatter,
            vec!["broken.md".to_owned()],
            "unclosed frontmatter must be treated the same as no frontmatter at all: a hard \
             violation, not a silently-passing gate"
        );
        let doc = &report.documents[0];
        assert_eq!(
            doc.id, None,
            "no field can be honestly parsed from an unclosed frontmatter block"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    /// ADR 0104: the docs gate reads the control documents from the repository's knowledge.
    #[test]
    fn the_control_documents_are_read_back_from_knowledge() {
        let root = scratch_root();
        let dir = root.join(knowledge::KNOWLEDGE_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        let mut facts: Vec<(String, String, String, String, Vec<String>)> = Vec::new();
        let mut doc = |path: &str, id: Option<&str>| {
            let full = format!("{CONTROL_DOCUMENT_ROOT}/{path}");
            facts.push((
                "D".into(),
                full.clone(),
                "digest".into(),
                "sha256:00".into(),
                vec![format!("doc:{full}")],
            ));
            if let Some(id) = id {
                facts.push((
                    "F".into(),
                    full.clone(),
                    "id".into(),
                    id.into(),
                    vec![format!("doc:{full}#L2")],
                ));
            }
        };
        for path in REQUIRED_CONTROL_DOCS {
            doc(path, Some("atlas.doc"));
        }
        doc("guides/UNMARKED.md", None);
        let architecture = format!("{CONTROL_DOCUMENT_ROOT}/architecture/SYSTEM.md");
        facts.push((
            "S".into(),
            String::new(),
            "the census lives in core/src/census/mod.rs".into(),
            "The census lives in `core/src/census/mod.rs`.".into(),
            vec![format!("doc:{architecture}#L9")],
        ));
        facts.push((
            "F".into(),
            format!("{architecture} / System / Census"),
            "owner".into(),
            "adapter/runtime/src/census/mod.rs".into(),
            vec![format!("doc:{architecture}#L12")],
        ));
        let guide = format!("{CONTROL_DOCUMENT_ROOT}/guides/DEVELOPMENT.md");
        for (key, value) in [
            ("markdown_title", "Development"),
            ("markdown_headings", "Development\nSetup\nTests"),
        ] {
            facts.push((
                "F".into(),
                guide.clone(),
                key.into(),
                value.into(),
                vec![format!("doc:{guide}")],
            ));
        }
        let decision = format!("{CONTROL_DOCUMENT_ROOT}/decisions/0002-epistemic-status-model.md");
        for (key, value) in [("title", "ADR 0002"), ("status", "accepted")] {
            facts.push((
                "C".into(),
                "0002-epistemic-status-model".into(),
                key.into(),
                value.into(),
                vec![format!("doc:{decision}#L5")],
            ));
        }
        let rank = |k: &str| match k {
            "S" => 0,
            "F" => 1,
            "C" => 2,
            _ => 5,
        };
        let provenance: Vec<Vec<&str>> = facts
            .iter()
            .map(|f| f.4.iter().map(String::as_str).collect())
            .collect();
        let rows: Vec<knowledge::tests::Row<'_>> = facts
            .iter()
            .zip(&provenance)
            .map(|(f, p)| {
                (
                    rank(&f.0),
                    f.1.as_str(),
                    f.2.as_str(),
                    f.3.as_str(),
                    p.as_slice(),
                    1,
                )
            })
            .collect();
        std::fs::write(dir.join("k.ynv"), knowledge::tests::encode(&rows)).unwrap();

        let report = audit_control_knowledge(&root).unwrap();
        assert!(report.required_control_docs_missing.is_empty());
        assert_eq!(report.documents_total, REQUIRED_CONTROL_DOCS.len() + 1);
        assert_eq!(
            report.missing_frontmatter,
            vec!["guides/UNMARKED.md".to_owned()]
        );
        assert!(
            !report.gate_ready,
            "a control document without its id is a hard violation"
        );
        let system = report
            .documents
            .iter()
            .find(|d| d.path == architecture)
            .unwrap();
        assert_eq!(system.id.as_deref(), Some("atlas.doc"));
        assert_eq!(system.title.as_deref(), Some("System"));
        assert_eq!(
            system.headings,
            vec!["System".to_owned(), "Census".to_owned()]
        );
        assert_eq!(
            system.references,
            vec![
                "core/src/census/mod.rs".to_owned(),
                "adapter/runtime/src/census/mod.rs".to_owned()
            ]
        );
        let development = report.documents.iter().find(|d| d.path == guide).unwrap();
        assert_eq!(development.title.as_deref(), Some("Development"));
        assert_eq!(development.headings, vec!["Development", "Setup", "Tests"]);
        let adr = report
            .documents
            .iter()
            .find(|d| d.path == decision)
            .unwrap();
        assert_eq!(adr.title.as_deref(), Some("ADR 0002"));
        assert_eq!(adr.status.as_deref(), Some("accepted"));

        // Without the unmarked document the gate is ready; without a required one it is not.
        let ready: Vec<_> = rows
            .iter()
            .copied()
            .filter(|r| !r.1.contains("UNMARKED"))
            .collect();
        std::fs::write(dir.join("k.ynv"), knowledge::tests::encode(&ready)).unwrap();
        assert!(audit_control_knowledge(&root).unwrap().gate_ready);
        let readme = format!("{CONTROL_DOCUMENT_ROOT}/README.md");
        let missing: Vec<_> = ready.into_iter().filter(|r| r.1 != readme).collect();
        std::fs::write(dir.join("k.ynv"), knowledge::tests::encode(&missing)).unwrap();
        let report = audit_control_knowledge(&root).unwrap();
        assert_eq!(
            report.required_control_docs_missing,
            vec!["README.md".to_owned()]
        );
        assert!(!report.gate_ready);

        // A repository without knowledge has no control documents at all.
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        let report = audit_control_knowledge(&root).unwrap();
        assert_eq!(report.documents_total, 0);
        assert_eq!(
            report.required_control_docs_missing.len(),
            REQUIRED_CONTROL_DOCS.len()
        );
        assert!(!report.gate_ready);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parses_atlas_manifest_policy_fields() {
        let manifest = parse_repo_manifest(
            r#"schema = "atlas.repo.v3"
repo = "org/repo"
system_kind = "SYSTEM_INVENTION_FORGE"
backend_language = "rust"
frontend_language = "typescript"
coding_requires_docs_gate = true
graph_before_code_required = true
exact_base_sha_required = true
single_repository_target_required = true
knowledge_root = ".ynventa"
temporary_root = "target/donors"
provenance_root = "tools/atlas/provenance"
license_root = "tools/atlas/licenses"

[code]
source_roots = ["core"]
backend_roots = ["core"]
frontend_roots = ["apps/studio"]
test_roots = ["core/tests", "runtime/tests", "adapter/tests", "apps/studio/src"]
"#,
        )
        .unwrap();
        // `validate_manifest` alone is a genuinely weaker property than this test's own name
        // ("parses ... policy fields") promises: it never checks `manifest.repo` at all, and for
        // `source_roots`/`backend_roots`/`frontend_roots`/`test_roots` it only checks that each
        // declared path stays contained within the repository root -- never that the parsed value
        // actually equals what the TOML declared. A real bug (e.g. `backend_roots` and
        // `frontend_roots` accidentally swapped in `parse_repo_manifest`'s own field list -- an
        // easy copy/paste slip since the four `array!` calls are visually identical) would still
        // leave `validate_manifest` returning zero violations under this fixture, since both
        // swapped values remain ordinary non-escaping relative paths. Asserting directly on the
        // parsed struct's own fields closes that gap.
        assert!(validate_manifest(&manifest).is_empty());
        assert_eq!(manifest.repo, "org/repo");
        assert_eq!(manifest.source_roots, vec!["core".to_owned()]);
        assert_eq!(manifest.backend_roots, vec!["core".to_owned()]);
        assert_eq!(manifest.frontend_roots, vec!["apps/studio".to_owned()]);
        assert_eq!(
            manifest.test_roots,
            vec![
                "core/tests".to_owned(),
                "runtime/tests".to_owned(),
                "adapter/tests".to_owned(),
                "apps/studio/src".to_owned(),
            ]
        );
    }

    // `visit_docs` shares `visit_inventory`'s exact unbounded-mutual-recursion shape (see
    // `adapter::source::tests::deeply_nested_directory_tree_does_not_abort_the_process` for the
    // full empirical history of that sibling bug) -- independently falsified here too, not merely
    // assumed fixed by association. Confirmed real, not theoretical: with this guard temporarily
    // disabled, walking a real on-disk tree at the maximum depth this walker's own path-
    // accumulating access pattern can even reach (`fs::read_dir` rejects a longer path outright
    // once it exceeds Linux's `PATH_MAX`) reliably reproduced the same `SIGABRT`/stack-overflow
    // crash `visit_inventory` had. This test's own fixture uses a shallower, cheaper-to-build 600
    // levels (well past the 512 guard, so it still exercises the guard triggering) rather than
    // that full ~2,000-level reproduction depth.
    #[test]
    fn deeply_nested_docs_tree_does_not_abort_the_process() {
        let base = scratch_root();
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let mut cursor = base.clone();
        for _ in 0..600 {
            cursor.push("d");
            fs::create_dir(&cursor).unwrap();
        }

        let report = audit_docs(&base).expect("must not abort the process");
        // `visit_docs` has no per-directory accounting channel (unlike `visit_inventory`'s
        // `ArtifactRecord` ledger) -- the depth limit is a best-effort skip, so the only
        // observable proof available is that the call returned at all rather than aborting the
        // whole process. `report` is deliberately unused beyond that.
        let _ = report;

        let _ = fs::remove_dir_all(&base);
    }

    // Unlike `visit_docs`/`visit_inventory` above, `visit_adl_sources`'s crash was NOT
    // reproducible: with this guard temporarily disabled, walking a real on-disk tree at the
    // maximum depth reachable at all (~2,000 levels, the same `PATH_MAX` ceiling as above) did
    // NOT overflow the stack -- this function's own per-frame stack usage is evidently small
    // enough that the filesystem's own path-length limit bounds it safely first. The guard is
    // still added here as deliberate, honest defense-in-depth (consistency with its two sibling
    // walkers, and safety against a future change to this function's own stack usage or a
    // filesystem/OS without the same `PATH_MAX` ceiling), not because a crash was proven -- see
    // this generation's own evidence record for the full, honest distinction.
    #[test]
    fn deeply_nested_adl_declared_tree_does_not_abort_the_process() {
        let base = scratch_root();
        let _ = fs::remove_dir_all(&base);
        let declared = base.join(DECLARED_ROOT);
        fs::create_dir_all(&declared).unwrap();
        let mut cursor = declared.clone();
        for _ in 0..600 {
            cursor.push("d");
            fs::create_dir(&cursor).unwrap();
        }

        let sources = read_adl_sources(&base).expect("must not abort the process");
        // Same best-effort-skip discipline as `visit_docs` above -- no per-directory accounting
        // channel exists here either, so the load-bearing proof is that this call returned.
        let _ = sources;

        let _ = fs::remove_dir_all(&base);
    }
}
