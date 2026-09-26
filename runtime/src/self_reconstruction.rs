//! SELF_RECONSTRUCTION (G153, ADR 0069): choosing a self target from what Atlas knows, lifting
//! it from a verified census container into the construction IR, building a shadow and
//! verifying it against the original.
//!
//! Construction reads records of the container only (`lift` takes no path to source). The
//! original is used by verification alone: its compiled crate is the behavioral oracle, and the
//! shadow's census is compared with the records construction started from. The shadow workspace
//! lives under `.atlas/.cache/shadow`, which is ignored and never admitted.

use atlas_core::atlas::CensusAtlas;
use atlas_core::composition::WorldModel;
use atlas_core::construction::{
    BOOTSTRAP_CONSTRUCTION_TARGET, CONSTRUCTION_IR_SCHEMA, CheckResult, ConstructionGap,
    ConstructionInput, ConstructionInputKind, ConstructionModule, Dispatch, EquivalenceCheck,
    EquivalenceKind, GapKind, IrFunction, IrParam, IrType, IrVariant, OracleKind, OracleUse,
    RECONSTRUCTION_REPORT_SCHEMA, SelfHostingLevel, SelfReconstructionReport, ShadowArtifact,
    TypeKind, decide, module_identity, report_identity, rust::RUST_BACKEND, rust::RustEmission,
};
use atlas_core::identity::IntegrityDigest;
use atlas_core::semantic::{DeclaredItem, FieldShape, FunctionDeclarationKind, SymbolRole};
use atlas_core::{SemanticObservation, SymbolIdentity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::{fs, io, process::Command};

pub const SHADOW_ROOT: &str = ".atlas/.cache/shadow";
pub const CANDIDATES_SCHEMA: &str = "atlas.self-reconstruction-candidates.v1";

/// A self target Atlas could reconstruct: a type and its methods, as the world model sees them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Candidate {
    pub target: String,
    pub path: String,
    pub owner_type: String,
    pub functions: Vec<String>,
    /// Functions outside the target whose resolved calls reach it.
    pub external_callers: usize,
    pub control_blocks: usize,
    /// Pure and closed by what Atlas knows: no effect, state, persistence or concurrency record,
    /// no unresolved call, no call leaving the target, nothing unsafe or async.
    pub constructible: bool,
    pub excluded_by: Vec<String>,
}

/// The core subsystem's types and their methods, constructible ones first, then by how many
/// functions depend on them, then smallest first.
pub fn candidates(model: &WorldModel) -> Vec<Candidate> {
    let mut groups: BTreeMap<(String, String), Vec<_>> = BTreeMap::new();
    for function in &model.functions {
        if function.test_scope || function.subsystem.as_deref() != Some("Core") {
            continue;
        }
        if let Some(owner) = &function.owner_type {
            groups
                .entry((function.path.clone(), owner.clone()))
                .or_default()
                .push(function);
        }
    }
    let mut out: Vec<Candidate> = groups
        .into_iter()
        .map(|((path, owner_type), functions)| {
            let ids: BTreeSet<&str> = functions.iter().map(|f| f.id.as_str()).collect();
            let mut excluded_by = BTreeSet::new();
            for f in &functions {
                for (why, hit) in [
                    ("EFFECT", !f.effects.is_empty()),
                    ("STATE", !f.state.is_empty()),
                    ("PERSISTENCE", !f.persistence.is_empty()),
                    ("CONCURRENCY", !f.concurrency.is_empty()),
                    (
                        "UNRESOLVED_CALLS",
                        f.unresolved_calls > 0 || f.unnamed_unresolved_calls > 0,
                    ),
                    (
                        "CALLS_LEAVE_TARGET",
                        f.calls.iter().any(|c| !ids.contains(c.as_str())),
                    ),
                    ("UNSAFE_OR_ASYNC", f.is_unsafe || f.is_async),
                ] {
                    if hit {
                        excluded_by.insert(why.to_string());
                    }
                }
            }
            let callers: BTreeSet<&str> = functions
                .iter()
                .flat_map(|f| f.callers.iter().map(String::as_str))
                .filter(|c| !ids.contains(c))
                .collect();
            Candidate {
                target: format!("{path}::{owner_type}"),
                path,
                owner_type,
                functions: functions.iter().map(|f| f.id.clone()).collect(),
                external_callers: callers.len(),
                control_blocks: functions.iter().map(|f| f.control.blocks).sum(),
                constructible: excluded_by.is_empty(),
                excluded_by: excluded_by.into_iter().collect(),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.constructible
            .cmp(&a.constructible)
            .then(b.external_callers.cmp(&a.external_callers))
            .then(a.control_blocks.cmp(&b.control_blocks))
            .then(a.target.cmp(&b.target))
    });
    out
}

fn span_order(provenance_span: &Option<String>) -> (usize, usize) {
    let parse = |s: Option<&str>| s.and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    match provenance_span {
        Some(span) => {
            let mut parts = span.split(':');
            (parse(parts.next()), parse(parts.next()))
        }
        None => (usize::MAX, usize::MAX),
    }
}

struct SymbolRecord<'a> {
    id: &'a str,
    subject: &'a SymbolIdentity,
    order: (usize, usize),
}

fn definitions<'a>(records: &'a [SemanticObservation], path: &str) -> Vec<SymbolRecord<'a>> {
    records
        .iter()
        .filter_map(|record| match record {
            SemanticObservation::Symbol(header)
                if header.subject.path == path && header.subject.role == SymbolRole::Definition =>
            {
                Some(SymbolRecord {
                    id: header.record_id.as_str(),
                    subject: &header.subject,
                    order: span_order(&header.provenance.span),
                })
            }
            _ => None,
        })
        .collect()
}

/// The type definition named `type_name` at the shallowest scope of `path` (never one inside a
/// test module).
fn type_symbol<'a>(
    symbols: &'a [SymbolRecord<'a>],
    type_name: &str,
) -> Option<&'a SymbolRecord<'a>> {
    symbols
        .iter()
        .filter(|s| {
            s.subject.name == type_name && !s.subject.scope.segments.iter().any(|g| g == "tests")
        })
        .min_by_key(|s| (s.subject.scope.segments.len(), s.order))
}

/// The design roots of a target: its type's SYMBOL record and its methods' FUNCTION_IDENTITY
/// records, as `DIMENSION:RECORD_ID`.
pub fn target_roots(records: &[SemanticObservation], path: &str, type_name: &str) -> Vec<String> {
    let symbols = definitions(records, path);
    let mut roots: Vec<String> = type_symbol(&symbols, type_name)
        .map(|s| format!("SYMBOL:{}", s.id))
        .into_iter()
        .collect();
    for (header, _) in methods(records, path, type_name) {
        roots.push(format!("FUNCTION_IDENTITY:{}", header));
    }
    roots
}

/// The inherent methods and associated functions of `type_name` in `path`: FUNCTION_IDENTITY
/// record id and signature record.
fn methods<'a>(
    records: &'a [SemanticObservation],
    path: &str,
    type_name: &str,
) -> Vec<(
    String,
    &'a atlas_core::SemanticRecordHeader<atlas_core::FunctionSignature>,
)> {
    let identities: BTreeMap<String, String> = records
        .iter()
        .filter_map(|record| match record {
            SemanticObservation::FunctionIdentity(header) => Some((
                header.subject.identity_key(),
                header.record_id.as_str().to_owned(),
            )),
            _ => None,
        })
        .collect();
    let mut out: Vec<_> = records
        .iter()
        .filter_map(|record| match record {
            SemanticObservation::FunctionSignature(header) => {
                let f = &header.subject.function;
                let owned = f.span.path == path
                    && f.owner.trait_path.is_none()
                    && f.owner.target.as_ref().is_some_and(|t| t.name == type_name)
                    && !f.scope.segments.iter().any(|g| g == "tests");
                owned.then(|| {
                    identities
                        .get(&f.identity_key())
                        .map(|id| (id.clone(), header.as_ref()))
                })?
            }
            _ => None,
        })
        .collect();
    out.sort_by_key(|(_, h)| (h.subject.function.span.line, h.subject.function.span.column));
    out
}

fn gap(kind: GapKind, subject: &str, missing: &str, debt: &str) -> ConstructionGap {
    ConstructionGap {
        kind,
        subject: subject.into(),
        missing: missing.into(),
        debt: debt.into(),
    }
}

/// The construction module of `type_name` in `path` lifted from census `records` alone, over
/// `design_id` (and `comparison_id`, if any). Whatever the census does not carry stays unobserved
/// and becomes a gap.
pub fn lift(
    records: &[SemanticObservation],
    container_root: &str,
    path: &str,
    type_name: &str,
    design_id: &str,
    comparison_id: Option<&str>,
) -> Result<ConstructionModule, String> {
    let symbols = definitions(records, path);
    let ty = type_symbol(&symbols, type_name)
        .ok_or(format!("no definition of {type_name} in {path}"))?;
    let mut inputs = vec![ConstructionInput {
        kind: ConstructionInputKind::CensusRecord,
        reference: ty.id.to_owned(),
    }];
    let mut gaps = Vec::new();
    let type_id = format!("type:{path}::{type_name}");
    let mut member_scope = ty.subject.scope.segments.clone();
    member_scope.push(type_name.to_owned());
    let mut members: Vec<&SymbolRecord> = symbols
        .iter()
        .filter(|s| s.subject.scope.segments == member_scope)
        .collect();
    members.sort_by_key(|s| s.order);
    let mut variants = Vec::new();
    for member in members {
        let subject = format!("{type_id}::{}", member.subject.name);
        let declared = member.subject.declaration.as_ref();
        let mut payload_scope = member_scope.clone();
        payload_scope.push(member.subject.name.clone());
        let has_fields = match declared.and_then(|d| d.shape) {
            Some(shape) => shape != FieldShape::Unit,
            None => symbols
                .iter()
                .any(|s| s.subject.scope.segments == payload_scope),
        };
        if has_fields {
            gaps.push(gap(
                GapKind::Unsupported,
                &subject,
                "a member with fields: IR v0 carries unit variants only",
                "DEBT-CONSTRUCTION_IR",
            ));
            continue;
        }
        inputs.push(ConstructionInput {
            kind: ConstructionInputKind::CensusRecord,
            reference: member.id.to_owned(),
        });
        if declared.is_none() {
            gaps.push(gap(
                GapKind::AttributesUnobserved,
                &subject,
                "the member's attributes are not in its SYMBOL record",
                "DEBT-TYPE",
            ));
        }
        variants.push(IrVariant {
            name: member.subject.name.clone(),
            documentation: member
                .subject
                .documentation
                .as_ref()
                .map(|d| d.summary.clone()),
            attributes: declared.map(|d| d.attributes.clone()),
            lineage: vec![member.id.to_owned()],
        });
    }
    let declared = ty.subject.declaration.as_ref();
    let kind = declared.and_then(|d| match d.item {
        DeclaredItem::Enum => Some(TypeKind::Enum),
        DeclaredItem::Struct => Some(TypeKind::Struct),
        _ => None,
    });
    if declared.is_none() {
        for (kind, missing) in [
            (
                GapKind::ItemKindUnobserved,
                "whether the definition is an enum or a struct",
            ),
            (GapKind::DerivesUnobserved, "the derived capabilities"),
            (
                GapKind::AttributesUnobserved,
                "the representation and wire attributes",
            ),
            (GapKind::VisibilityUnobserved, "the type's visibility"),
        ] {
            gaps.push(gap(kind, &type_id, missing, "DEBT-TYPE"));
        }
    } else if kind.is_none() {
        gaps.push(gap(
            GapKind::ItemKindUnobserved,
            &type_id,
            "the definition is neither an enum nor a struct",
            "DEBT-CONSTRUCTION_IR",
        ));
    }
    let types = vec![IrType {
        id: type_id,
        name: type_name.to_owned(),
        kind,
        visibility: declared.map(|d| d.visibility.clone()),
        documentation: ty.subject.documentation.as_ref().map(|d| d.summary.clone()),
        derives: declared.map(|d| d.derives.clone()),
        attributes: declared.map(|d| d.attributes.clone()),
        variants,
        lineage: vec![ty.id.to_owned()],
    }];
    let mut functions = Vec::new();
    for (identity, header) in methods(records, path, type_name) {
        let signature = &header.subject;
        let f = &signature.function;
        let dispatch = match f.declaration_kind {
            FunctionDeclarationKind::InherentMethod => Dispatch::InherentMethod,
            FunctionDeclarationKind::AssociatedFunction => Dispatch::AssociatedFunction,
            FunctionDeclarationKind::FreeFunction => Dispatch::FreeFunction,
            _ => continue,
        };
        let id = format!("fn:{path}::{type_name}::{}", f.symbol.name);
        for record in [identity.as_str(), header.record_id.as_str()] {
            inputs.push(ConstructionInput {
                kind: ConstructionInputKind::CensusRecord,
                reference: record.to_owned(),
            });
        }
        gaps.push(gap(
            GapKind::BodyUnobserved,
            &id,
            "lowerable body semantics (the census holds relational identities and a fingerprint)",
            "DEBT-CONSTRUCTION_IR",
        ));
        functions.push(IrFunction {
            id,
            name: f.symbol.name.clone(),
            owner: Some(type_name.to_owned()),
            dispatch,
            visibility: signature.visibility.clone(),
            documentation: f.symbol.documentation.as_ref().map(|d| d.summary.clone()),
            params: signature
                .parameters
                .iter()
                .map(|p| IrParam {
                    name: p.name.clone(),
                    type_spelling: p.type_identity.name.clone(),
                })
                .collect(),
            result: signature.return_type.as_ref().map(|t| t.name.clone()),
            body_fingerprint: signature.body_fingerprint.clone(),
            lineage: vec![identity.clone(), header.record_id.as_str().to_owned()],
        });
    }
    inputs.push(ConstructionInput {
        kind: ConstructionInputKind::Design,
        reference: design_id.to_owned(),
    });
    if let Some(comparison) = comparison_id {
        inputs.push(ConstructionInput {
            kind: ConstructionInputKind::Comparison,
            reference: comparison.to_owned(),
        });
    }
    inputs.sort();
    inputs.dedup();
    gaps.sort();
    let mut module = ConstructionModule {
        schema: CONSTRUCTION_IR_SCHEMA.into(),
        module_id: String::new(),
        target: format!("{path}::{type_name}"),
        construction_target: BOOTSTRAP_CONSTRUCTION_TARGET.into(),
        container_root: container_root.to_owned(),
        inputs,
        types,
        functions,
        gaps,
    };
    module.module_id = module_identity(&module);
    Ok(module)
}

/// `atlas_core::<module>::<Type>` for a type of the core crate, when its module is public.
pub fn oracle_path(path: &str, type_name: &str) -> Option<String> {
    let relative = path.strip_prefix("core/src/")?.strip_suffix(".rs")?;
    let module = relative.strip_suffix("/mod").unwrap_or(relative);
    if module == "lib" {
        return Some(format!("atlas_core::{type_name}"));
    }
    Some(format!(
        "atlas_core::{}::{type_name}",
        module.replace('/', "::")
    ))
}

fn slug(type_name: &str) -> String {
    let mut out = String::new();
    for (i, c) in type_name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// The behavioral differential of every emitted type against its oracle, one test per property
/// the shadow derives. A `match` over the oracle without a wildcard makes a variant the shadow
/// lacks a build failure.
fn differential(module: &ConstructionModule, emission: &RustEmission, crate_name: &str) -> String {
    let mut out = String::from(
        "//! Generated by Atlas: the shadow against its oracle.\n#![allow(clippy::all)]\n",
    );
    for ty in module
        .types
        .iter()
        .filter(|t| emission.emitted.contains(&t.id))
    {
        let Some(oracle) = module
            .target
            .split_once("::")
            .and_then(|(p, _)| oracle_path(p, &ty.name))
        else {
            continue;
        };
        let derives = ty.derives.clone().unwrap_or_default();
        let has = |d: &str| derives.iter().any(|x| x == d);
        let m = slug(&ty.name);
        out.push_str(&format!(
            "\nmod {m} {{\n    use {crate_name}::{} as Shadow;\n    use {oracle} as Oracle;\n\n    fn pairs() -> Vec<(Shadow, Oracle)> {{\n        vec![\n",
            ty.name
        ));
        for v in &ty.variants {
            out.push_str(&format!(
                "            (Shadow::{0}, Oracle::{0}),\n",
                v.name
            ));
        }
        out.push_str("        ]\n    }\n\n    #[test]\n    fn exhaustive() {\n        for (_, oracle) in pairs() {\n            match oracle {\n");
        for v in &ty.variants {
            out.push_str(&format!("                Oracle::{} => {{}}\n", v.name));
        }
        out.push_str("            }\n        }\n    }\n");
        if has("Debug") {
            out.push_str("\n    #[test]\n    fn debug() {\n        for (s, o) in pairs() {\n            assert_eq!(format!(\"{s:?}\"), format!(\"{o:?}\"));\n        }\n    }\n");
        }
        if has("Serialize") {
            out.push_str("\n    #[test]\n    fn wire_serialize() {\n        for (s, o) in pairs() {\n            assert_eq!(serde_json::to_string(&s).unwrap(), serde_json::to_string(&o).unwrap());\n        }\n    }\n");
        }
        if has("Deserialize") && has("Debug") {
            out.push_str("\n    #[test]\n    fn wire_deserialize() {\n        for (s, o) in pairs() {\n            let back: Shadow = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();\n            assert_eq!(format!(\"{back:?}\"), format!(\"{s:?}\"));\n        }\n    }\n");
        }
        if has("PartialEq") {
            out.push_str("\n    #[test]\n    fn equality() {\n        for (s1, o1) in pairs() {\n            for (s2, o2) in pairs() {\n                assert_eq!(s1 == s2, o1 == o2);\n            }\n        }\n    }\n");
        }
        if has("PartialOrd") {
            out.push_str("\n    #[test]\n    fn order() {\n        for (s1, o1) in pairs() {\n            for (s2, o2) in pairs() {\n                assert_eq!(s1.partial_cmp(&s2), o1.partial_cmp(&o2));\n            }\n        }\n    }\n");
        }
        if has("Clone") && has("Debug") {
            out.push_str("\n    #[test]\n    fn clone() {\n        for (s, o) in pairs() {\n            assert_eq!(format!(\"{:?}\", s.clone()), format!(\"{:?}\", o.clone()));\n        }\n    }\n");
        }
        if has("Default") && has("Debug") {
            out.push_str("\n    #[test]\n    fn default() {\n        assert_eq!(format!(\"{:?}\", Shadow::default()), format!(\"{:?}\", Oracle::default()));\n    }\n");
        }
        out.push_str("}\n");
    }
    out
}

/// Where a shadow of `module` is built.
pub fn shadow_dir(root: &Path, module: &ConstructionModule) -> PathBuf {
    let name = module.target.rsplit("::").next().unwrap_or("target");
    root.join(SHADOW_ROOT).join(slug(name))
}

/// Writes the shadow crate (the emitted source and its differential) and returns its source path.
pub fn write_shadow(
    root: &Path,
    module: &ConstructionModule,
    emission: &RustEmission,
) -> io::Result<PathBuf> {
    let dir = shadow_dir(root, module);
    let name = module.target.rsplit("::").next().unwrap_or("target");
    let crate_name = format!("atlas_shadow_{}", slug(name));
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("tests"))?;
    let core = root.join("core").canonicalize()?;
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            "# Generated by Atlas (SELF_RECONSTRUCTION): a shadow, never admitted.\n[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\n\n[dependencies]\nserde = {{ version = \"1\", features = [\"derive\"] }}\n\n[dev-dependencies]\natlas_core = {{ package = \"core\", path = \"{}\" }}\nserde_json = \"1\"\n",
            crate_name.replace('_', "-"),
            core.display()
        ),
    )?;
    // The workspace's lockfile pins the same dependency versions offline.
    fs::copy(root.join("Cargo.lock"), dir.join("Cargo.lock"))?;
    let source = dir.join("src/lib.rs");
    fs::write(&source, &emission.source)?;
    fs::write(
        dir.join("tests/differential.rs"),
        differential(module, emission, &crate_name),
    )?;
    Ok(source)
}

/// `rustc -V`, the toolchain a shadow is built with.
pub fn toolchain() -> String {
    Command::new("rustc")
        .arg("-V")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "UNKNOWN".into())
}

/// Builds and tests the shadow offline; each differential test becomes a behavioral check. A
/// build failure is one mismatch naming the compiler's complaint.
pub fn run_differential(
    root: &Path,
    module: &ConstructionModule,
) -> io::Result<Vec<EquivalenceCheck>> {
    let dir = shadow_dir(root, module);
    let output = Command::new("cargo")
        .args(["test", "--offline", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .args(["--", "--test-threads=1", "--format=pretty"])
        .env("CARGO_TARGET_DIR", root.join(SHADOW_ROOT).join("target"))
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut checks = Vec::new();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        let Some((name, result)) = rest.rsplit_once(" ... ") else {
            continue;
        };
        let (subject, property) = name.split_once("::").unwrap_or(("", name));
        checks.push(EquivalenceCheck {
            kind: EquivalenceKind::Behavioral,
            subject: subject.to_owned(),
            property: property.to_owned(),
            result: if result.trim() == "ok" {
                CheckResult::Equivalent
            } else {
                CheckResult::Mismatch
            },
            detail: "shadow against the original crate, every variant pair".into(),
        });
    }
    if !output.status.success() && checks.iter().all(|c| c.result == CheckResult::Equivalent) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(12).collect();
        checks.push(EquivalenceCheck {
            kind: EquivalenceKind::Behavioral,
            subject: module.target.clone(),
            property: "build".into(),
            result: CheckResult::Mismatch,
            detail: tail.into_iter().rev().collect::<Vec<_>>().join("\n"),
        });
    }
    checks.sort();
    Ok(checks)
}

/// Atlas's census of the shadow against the records construction started from: the type's
/// definition, its members in order with their documentation.
pub fn semantic_checks(
    module: &ConstructionModule,
    emission: &RustEmission,
    shadow_source: &str,
) -> Vec<EquivalenceCheck> {
    use adapter::semantic::SemanticExtractor;
    let input = adapter::semantic::ExtractionInput {
        repository: atlas_core::RepositoryId::new("atlas-shadow"),
        revision: atlas_core::RevisionRef {
            kind: "WORKTREE".into(),
            value: module.module_id.clone(),
        },
        artifact: atlas_core::ArtifactId::new("shadow:src/lib.rs"),
        artifact_path: "src/lib.rs".into(),
        source_text: shadow_source.to_owned(),
        content_fingerprint: None,
        source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
        language: "rust".into(),
        build_profile: None,
        scope_policy: None,
        requested_dimensions: vec![atlas_core::SemanticDimension::Symbol],
    };
    let batch = adapter::semantic::rust::RustSemanticExtractor.extract(&input);
    let mut symbols: Vec<(&SymbolIdentity, (usize, usize))> = batch
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Symbol(h) if h.subject.role == SymbolRole::Definition => {
                Some((&h.subject, span_order(&h.provenance.span)))
            }
            _ => None,
        })
        .collect();
    symbols.sort_by_key(|(_, order)| *order);
    let mut checks = Vec::new();
    let mut check = |subject: &str, property: &str, equal: bool, detail: String| {
        checks.push(EquivalenceCheck {
            kind: EquivalenceKind::Semantic,
            subject: subject.to_owned(),
            property: property.to_owned(),
            result: if equal {
                CheckResult::Equivalent
            } else {
                CheckResult::Mismatch
            },
            detail,
        });
    };
    for ty in module
        .types
        .iter()
        .filter(|t| emission.emitted.contains(&t.id))
    {
        let found = symbols
            .iter()
            .find(|(s, _)| s.name == ty.name && s.scope.segments.is_empty());
        let documentation = found
            .and_then(|(s, _)| s.documentation.as_ref())
            .map(|d| d.summary.clone());
        check(
            &ty.id,
            "definition",
            found.is_some(),
            "the shadow defines the type at its root".into(),
        );
        check(
            &ty.id,
            "documentation",
            documentation == ty.documentation,
            format!("{documentation:?} against {:?}", ty.documentation),
        );
        let declared = found.and_then(|(s, _)| s.declaration.as_ref());
        let expected_kind = match ty.kind {
            Some(TypeKind::Enum) => Some(DeclaredItem::Enum),
            Some(TypeKind::Struct) => Some(DeclaredItem::Struct),
            None => None,
        };
        check(
            &ty.id,
            "declaration",
            declared.map(|d| {
                (
                    Some(d.item),
                    Some(d.visibility.clone()),
                    Some(d.derives.clone()),
                    Some(d.attributes.clone()),
                )
            }) == Some((
                expected_kind,
                ty.visibility.clone(),
                ty.derives.clone(),
                ty.attributes.clone(),
            )),
            format!("kind, visibility, derives and attributes: {declared:?}"),
        );
        type Member = (
            String,
            Option<String>,
            Option<Vec<String>>,
            Option<FieldShape>,
        );
        let members: Vec<Member> = symbols
            .iter()
            .filter(|(s, _)| s.scope.segments == [ty.name.clone()])
            .map(|(s, _)| {
                (
                    s.name.clone(),
                    s.documentation.as_ref().map(|d| d.summary.clone()),
                    s.declaration.as_ref().map(|d| d.attributes.clone()),
                    s.declaration.as_ref().and_then(|d| d.shape),
                )
            })
            .collect();
        let expected: Vec<Member> = ty
            .variants
            .iter()
            .map(|v| {
                (
                    v.name.clone(),
                    v.documentation.clone(),
                    v.attributes.clone(),
                    Some(FieldShape::Unit),
                )
            })
            .collect();
        check(
            &ty.id,
            "members_in_order",
            members == expected,
            format!(
                "{} members (name, documentation, attributes, shape) against {}",
                members.len(),
                expected.len()
            ),
        );
    }
    checks.sort();
    checks
}

/// Everything one attempt needs besides the container.
pub struct Attempt<'a> {
    pub root: &'a Path,
    pub container: &'a CensusAtlas,
    pub container_root: &'a str,
    pub path: &'a str,
    pub type_name: &'a str,
    pub design_id: &'a str,
    pub design_state: atlas_core::design::DesignState,
    pub comparison_id: Option<&'a str>,
}

/// One SH1 attempt: lift, emit, build the shadow when anything was emitted, verify it, report.
pub fn attempt(
    a: &Attempt,
) -> Result<(ConstructionModule, RustEmission, SelfReconstructionReport), String> {
    let module = lift(
        &a.container.typed_records,
        a.container_root,
        a.path,
        a.type_name,
        a.design_id,
        a.comparison_id,
    )?;
    let emission = atlas_core::construction::rust::emit(&module);
    let (mut shadow, mut checks, mut oracle_uses) = (None, Vec::new(), Vec::new());
    if !emission.emitted.is_empty() {
        let source = write_shadow(a.root, &module, &emission).map_err(|e| e.to_string())?;
        let relative = source
            .strip_prefix(a.root)
            .unwrap_or(&source)
            .to_string_lossy()
            .into_owned();
        shadow = Some(ShadowArtifact {
            path: relative,
            content_hash: IntegrityDigest::of_bytes(emission.source.as_bytes())
                .as_str()
                .to_owned(),
            backend: RUST_BACKEND.into(),
            toolchain: toolchain(),
            emitted: emission.emitted.clone(),
            omitted: emission.omitted.clone(),
        });
        if let Some(oracle) = oracle_path(a.path, a.type_name) {
            oracle_uses.push(OracleUse {
                kind: OracleKind::OracleExecution,
                reference: format!("crate:{oracle}"),
                purpose: "behavioral differential: the compiled original, every variant pair"
                    .into(),
            });
        }
        checks = run_differential(a.root, &module).map_err(|e| e.to_string())?;
        checks.extend(semantic_checks(&module, &emission, &emission.source));
        checks.sort();
    }
    let mut report = SelfReconstructionReport {
        schema: RECONSTRUCTION_REPORT_SCHEMA.into(),
        report_id: String::new(),
        level: SelfHostingLevel::Sh1,
        target: module.target.clone(),
        construction_target: module.construction_target.clone(),
        module_id: module.module_id.clone(),
        design_ref: a.design_id.to_owned(),
        design_state: a.design_state,
        comparison_ref: a.comparison_id.map(str::to_owned),
        construction_inputs: module.inputs.clone(),
        oracle_uses,
        shadow,
        checks,
        gaps: module.gaps.clone(),
        declared_variations: Vec::new(),
        verdict: atlas_core::construction::ReconstructionVerdict::Unsupported,
    };
    report.verdict = decide(&report);
    report.report_id = report_identity(&report);
    Ok((module, emission, report))
}

/// Every way `module` and `report` break the construction input boundary or overclaim, against
/// the container the module says it came from.
pub fn validate(
    container: &CensusAtlas,
    module: &ConstructionModule,
    report: &SelfReconstructionReport,
) -> Vec<atlas_core::construction::Violation> {
    let ids: BTreeSet<String> = container
        .typed_records
        .iter()
        .map(|r| r.record_id().as_str().to_owned())
        .collect();
    let mut violations = atlas_core::construction::validate_module(module, &ids);
    violations.extend(atlas_core::construction::validate_report(report, module));
    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter::semantic::SemanticExtractor;

    const SOURCE: &str = r#"
/// How far away.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Mode {
    /// Far away.
    Remote,
    #[default]
    Local,
}

impl Mode {
    pub fn is_local(self) -> bool {
        self != Self::Remote
    }
}

pub enum Shape {
    Unit,
    Pair(u8, u8),
}
"#;

    fn census(path: &str, source: &str) -> Vec<SemanticObservation> {
        let input = adapter::semantic::ExtractionInput {
            repository: atlas_core::RepositoryId::new("fixture"),
            revision: atlas_core::RevisionRef {
                kind: "GIT_COMMIT".into(),
                value: "abc".into(),
            },
            artifact: atlas_core::ArtifactId::new(path),
            artifact_path: path.into(),
            source_text: source.into(),
            content_fingerprint: None,
            source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
            language: "rust".into(),
            build_profile: None,
            scope_policy: None,
            requested_dimensions: vec![
                atlas_core::SemanticDimension::Symbol,
                atlas_core::SemanticDimension::FunctionIdentity,
                atlas_core::SemanticDimension::FunctionSignature,
            ],
        };
        adapter::semantic::rust::RustSemanticExtractor
            .extract(&input)
            .observations
    }

    fn ids(records: &[SemanticObservation]) -> BTreeSet<String> {
        records
            .iter()
            .map(|r| r.record_id().as_str().to_owned())
            .collect()
    }

    const PATH: &str = "core/src/mode.rs";

    #[test]
    fn a_declared_enum_lifts_whole_and_only_its_body_is_a_gap() {
        let records = census(PATH, SOURCE);
        let module = lift(&records, "root", PATH, "Mode", "design:1", None).unwrap();
        assert_eq!(
            atlas_core::construction::validate_module(&module, &ids(&records)),
            vec![]
        );
        let ty = &module.types[0];
        assert_eq!(ty.kind, Some(TypeKind::Enum));
        assert_eq!(ty.visibility.as_deref(), Some("pub"));
        assert_eq!(ty.documentation.as_deref(), Some("How far away."));
        assert_eq!(
            ty.derives.as_deref().unwrap().join(" "),
            "Debug Clone Copy Serialize Deserialize PartialEq Eq PartialOrd Ord Default"
        );
        assert_eq!(
            ty.attributes.as_deref().unwrap(),
            ["serde(rename_all = \"SCREAMING_SNAKE_CASE\")"]
        );
        let names: Vec<&str> = ty.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["Remote", "Local"]);
        assert_eq!(ty.variants[1].attributes.as_deref().unwrap(), ["default"]);
        assert_eq!(ty.variants[0].documentation.as_deref(), Some("Far away."));
        let kinds: Vec<GapKind> = module.gaps.iter().map(|g| g.kind).collect();
        assert_eq!(kinds, [GapKind::BodyUnobserved]);
        let f = &module.functions[0];
        assert_eq!(
            (f.name.as_str(), f.result.as_deref()),
            ("is_local", Some("bool"))
        );
        // The roots a design selects are the same records.
        let roots = target_roots(&records, PATH, "Mode");
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn without_declarations_nothing_is_guessed_and_nothing_is_emitted() {
        let mut records = census(PATH, SOURCE);
        for record in &mut records {
            if let SemanticObservation::Symbol(h) = record {
                h.subject.declaration = None;
            }
        }
        let module = lift(&records, "root", PATH, "Mode", "design:1", None).unwrap();
        assert_eq!(
            atlas_core::construction::validate_module(&module, &ids(&records)),
            vec![]
        );
        let kinds: BTreeSet<GapKind> = module.gaps.iter().map(|g| g.kind).collect();
        for kind in [
            GapKind::ItemKindUnobserved,
            GapKind::DerivesUnobserved,
            GapKind::AttributesUnobserved,
            GapKind::VisibilityUnobserved,
            GapKind::BodyUnobserved,
        ] {
            assert!(kinds.contains(&kind), "{kind:?}");
        }
        assert!(
            atlas_core::construction::rust::emit(&module)
                .emitted
                .is_empty()
        );
    }

    #[test]
    fn a_variant_with_fields_is_unsupported_never_flattened() {
        let records = census(PATH, SOURCE);
        let module = lift(&records, "root", PATH, "Shape", "design:1", None).unwrap();
        let names: Vec<&str> = module.types[0]
            .variants
            .iter()
            .map(|v| v.name.as_str())
            .collect();
        assert_eq!(names, ["Unit"]);
        assert!(
            module
                .gaps
                .iter()
                .any(|g| g.kind == GapKind::Unsupported && g.subject.ends_with("::Pair"))
        );
    }

    #[test]
    fn the_shadow_census_round_trips_and_catches_a_reordered_or_underived_emission() {
        let records = census(PATH, SOURCE);
        let module = lift(&records, "root", PATH, "Mode", "design:1", None).unwrap();
        let emission = atlas_core::construction::rust::emit(&module);
        let checks = semantic_checks(&module, &emission, &emission.source);
        assert_eq!(checks.len(), 4, "{checks:?}");
        assert!(
            checks.iter().all(|c| c.result == CheckResult::Equivalent),
            "{checks:?}"
        );
        // Falsifiers: the census of a wrong shadow disagrees with the module.
        let swapped = emission
            .source
            .replace("    Remote,", "    @@,")
            .replace("    Local,", "    Remote,")
            .replace("    @@,", "    Local,");
        let found = semantic_checks(&module, &emission, &swapped);
        assert!(
            found
                .iter()
                .any(|c| c.property == "members_in_order" && c.result == CheckResult::Mismatch)
        );
        let underived = emission.source.replace(", Default)]", ")]");
        let found = semantic_checks(&module, &emission, &underived);
        assert!(
            found
                .iter()
                .any(|c| c.property == "declaration" && c.result == CheckResult::Mismatch)
        );
    }

    #[test]
    fn the_differential_compares_every_derived_property_and_is_exhaustive() {
        let records = census(PATH, SOURCE);
        let module = lift(&records, "root", PATH, "Mode", "design:1", None).unwrap();
        let emission = atlas_core::construction::rust::emit(&module);
        let harness = differential(&module, &emission, "atlas_shadow_mode");
        for test in [
            "exhaustive",
            "debug",
            "wire_serialize",
            "wire_deserialize",
            "equality",
            "order",
            "clone",
            "default",
        ] {
            assert!(harness.contains(&format!("fn {test}()")), "{test}");
        }
        assert!(harness.contains("use atlas_core::mode::Mode as Oracle;"));
        assert!(
            !harness.contains("_ =>"),
            "a wildcard would hide a missing variant"
        );
    }

    #[test]
    fn oracle_paths_follow_the_core_module_tree() {
        assert_eq!(
            oracle_path("core/src/donor/mod.rs", "MaterializationMode").as_deref(),
            Some("atlas_core::donor::MaterializationMode")
        );
        assert_eq!(
            oracle_path("core/src/census/dependency.rs", "ReachOrigin").as_deref(),
            Some("atlas_core::census::dependency::ReachOrigin")
        );
        assert_eq!(oracle_path("runtime/src/lib.rs", "X"), None);
        assert_eq!(slug("MaterializationMode"), "materialization_mode");
    }
}
