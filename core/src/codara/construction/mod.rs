//! SELF_RECONSTRUCTION (G153, ADR 0069, `contracts/SELF-RECONSTRUCTION.md`): the construction
//! IR, its input boundary, typed construction gaps and the self-reconstruction report.
//!
//! Atlas reconstructs a bounded part of itself only from what it knows: typed census records of
//! a verified container, a design over them and its comparison. The original source is an oracle
//! for verification and never an input to construction. `ConstructionInputKind` has no variant
//! that names source text, and oracle reads live in the report, never in the module.
//!
//! The IR is target-neutral and uses the HIR type vocabulary (`COMPILER-IR-SCHEMAS.md`). It is
//! pre-AtlasX: there is no `atlasx_root_id` yet (DEBT-ATLASX). Its lineage is the census records
//! it was lifted from. An element Atlas did not observe stays `None` and carries a
//! `ConstructionGap`. A backend never fills a gap by guessing.

pub mod rust;

use crate::identity::IntegrityDigest;
use crate::vocabulary_enum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CONSTRUCTION_IR_SCHEMA: &str = "atlas.construction-ir.v1";
pub const RECONSTRUCTION_REPORT_SCHEMA: &str = "atlas.self-reconstruction-report.v1";
/// The first construction target of the bootstrap (SYSTEM-CONTRACT `BACKEND_BOOTSTRAP_IS_RUST`).
pub const BOOTSTRAP_CONSTRUCTION_TARGET: &str = "RUST";

vocabulary_enum! {
    /// What construction may read. There is deliberately no variant for source text.
    pub enum ConstructionInputKind {
        /// A typed record of the verified census container, by record id.
        CensusRecord => "CENSUS_RECORD",
        /// A design over the container (`SelectedDesign`), by design id.
        Design => "DESIGN",
        /// The comparison a design was measured in, by comparison id.
        Comparison => "COMPARISON",
    }
}

vocabulary_enum! {
    /// What verification may read or run. Never a construction input.
    pub enum OracleKind {
        /// Reading the original's source or records to compare against.
        OracleRead => "ORACLE_READ",
        /// Running the original to compare behavior.
        OracleExecution => "ORACLE_EXECUTION",
    }
}

vocabulary_enum! {
    /// HIR type kinds the IR can carry so far (`COMPILER-IR-SCHEMAS.md` HirType).
    pub enum TypeKind {
        Enum => "ENUM",
        Struct => "STRUCT",
    }
}

vocabulary_enum! {
    /// HIR declaration/dispatch kinds the IR can carry so far.
    pub enum Dispatch {
        FreeFunction => "FREE_FUNCTION",
        InherentMethod => "INHERENT_METHOD",
        AssociatedFunction => "ASSOCIATED_FUNCTION",
    }
}

vocabulary_enum! {
    /// Why a part of the target could not be constructed from what Atlas knows.
    pub enum GapKind {
        /// Whether a type is an enum or a struct is not recorded.
        ItemKindUnobserved => "ITEM_KIND_UNOBSERVED",
        /// The derived capabilities of a type are not recorded.
        DerivesUnobserved => "DERIVES_UNOBSERVED",
        /// The attributes of a type or variant (representation, wire names) are not recorded.
        AttributesUnobserved => "ATTRIBUTES_UNOBSERVED",
        /// A type's visibility is not recorded.
        VisibilityUnobserved => "VISIBILITY_UNOBSERVED",
        /// A variant's payload shape is not recorded or not supported.
        VariantShapeUnobserved => "VARIANT_SHAPE_UNOBSERVED",
        /// A function body has no lowerable semantics in the census (M14).
        BodyUnobserved => "BODY_UNOBSERVED",
        /// A construct outside the IR's vocabulary.
        Unsupported => "UNSUPPORTED",
    }
}

vocabulary_enum! {
    /// The self-hosting ladder (`contracts/SELF-RECONSTRUCTION.md`).
    pub enum SelfHostingLevel {
        /// Atlas censuses itself.
        Sh0 => "SH0",
        /// A bounded shadow reconstruction of one self target, verified against the original.
        Sh1 => "SH1",
        /// A module reconstructed and equivalent.
        Sh2 => "SH2",
        /// A crate reconstructed and equivalent.
        Sh3 => "SH3",
        /// A reconstructed part replaces the original under admission.
        Sh4 => "SH4",
        /// Atlas rebuilds its own construction path from its own knowledge.
        Sh5 => "SH5",
        /// Atlas reconstructs all of itself (fixed point).
        Sh6 => "SH6",
    }
}

vocabulary_enum! {
    /// The outcome of a self-reconstruction attempt.
    pub enum ReconstructionVerdict {
        ReconstructedEquivalent => "RECONSTRUCTED_EQUIVALENT",
        ReconstructedWithDeclaredVariation => "RECONSTRUCTED_WITH_DECLARED_VARIATION",
        ConstructionGap => "CONSTRUCTION_GAP",
        VerificationFailed => "VERIFICATION_FAILED",
        SemanticMismatch => "SEMANTIC_MISMATCH",
        Unsupported => "UNSUPPORTED",
    }
}

vocabulary_enum! {
    /// Which equivalence a check establishes.
    pub enum EquivalenceKind {
        /// The shadow and the original behave alike when run.
        Behavioral => "BEHAVIORAL",
        /// Atlas's census of the shadow says what its census of the original says.
        Semantic => "SEMANTIC",
    }
}

vocabulary_enum! {
    /// The result of one equivalence check.
    pub enum CheckResult {
        Equivalent => "EQUIVALENT",
        Mismatch => "MISMATCH",
    }
}

vocabulary_enum! {
    /// G181 (M14, ADR 0094): the HIR body node kinds the IR can carry so far, a bounded subset of
    /// `COMPILER-IR-SCHEMAS.md` "Core HIR node kinds v1". Every one is pure and call-closed.
    pub enum HirNodeKind {
        /// A constant: `true`/`false`, or a unit variant of a type in the module.
        Const => "CONST",
        /// A by-value read of a parameter whose type is a `Copy` type of the module.
        Copy => "COPY",
        /// A comparison (`HirIntrinsic`) over two operands of one type deriving `PartialEq`.
        Intrinsic => "INTRINSIC",
        /// A match over a module enum: its scrutinee and its arms.
        Match => "MATCH",
    }
}

vocabulary_enum! {
    /// G181: the intrinsic operations an `INTRINSIC` node can apply.
    pub enum HirIntrinsic {
        /// Structural equality through a derived `PartialEq`.
        Eq => "EQ",
        /// Its negation.
        Ne => "NE",
    }
}

vocabulary_enum! {
    /// G181: why `validate_module` refuses an IR body. A body is whole and typed or it is not
    /// carried at all: a function without one keeps its `BODY_UNOBSERVED` gap.
    pub enum BodyDefect {
        /// A node's fields do not fit its kind (a comparison without its operator, a match without
        /// arms, a node id that is not its position).
        MalformedNode => "BODY_MALFORMED_NODE",
        /// A child reference outside the body or not before its parent (the body is a DAG in
        /// post-order, so it has no cycle).
        DanglingChild => "BODY_DANGLING_CHILD",
        /// A node without a result type, or one naming no type of the module.
        UntypedNode => "BODY_UNTYPED_NODE",
        /// A constant, parameter or pattern that does not resolve in the module.
        UnresolvedReference => "BODY_UNRESOLVED_REFERENCE",
        /// Operands, arms or the root disagree with the types they must have.
        TypeMismatch => "BODY_TYPE_MISMATCH",
        /// A capability the node needs (`Copy`, `PartialEq`) that its type was not observed to
        /// derive.
        CapabilityUnobserved => "BODY_CAPABILITY_UNOBSERVED",
        /// A match that misses a variant without a wildcard.
        NonExhaustiveMatch => "BODY_NON_EXHAUSTIVE_MATCH",
        /// A function carrying a body and a `BODY_UNOBSERVED` gap at once.
        GapContradiction => "BODY_GAP_CONTRADICTION",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstructionInput {
    pub kind: ConstructionInputKind,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IrVariant {
    pub name: String,
    pub documentation: Option<String>,
    /// Outer attributes other than documentation, as recorded; `None` when not recorded.
    pub attributes: Option<Vec<String>>,
    pub lineage: Vec<String>,
}

/// A HIR type. `None` means Atlas did not observe it, never "none".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IrType {
    pub id: String,
    pub name: String,
    pub kind: Option<TypeKind>,
    pub visibility: Option<String>,
    pub documentation: Option<String>,
    pub derives: Option<Vec<String>>,
    pub attributes: Option<Vec<String>>,
    /// In declaration order.
    pub variants: Vec<IrVariant>,
    pub lineage: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IrParam {
    pub name: String,
    pub type_spelling: String,
}

/// G181: a match arm pattern: a unit variant (`<type id>::<variant>`) or the wildcard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HirPattern {
    Variant(String),
    Wildcard,
}

/// G181: one arm of a `MATCH` node: its alternatives and the node giving its value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HirArm {
    pub patterns: Vec<HirPattern>,
    pub value: usize,
}

/// G181: one HIR body node (`COMPILER-IR-SCHEMAS.md` "HIR body node"). Its children come before
/// it, so a body is acyclic by construction and checked to be.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HirNode {
    /// Its position in the body.
    pub id: usize,
    pub kind: HirNodeKind,
    /// `bool`, or the id of a type of the module.
    pub result_type: String,
    /// CONST: `true`, `false` or `<type id>::<variant>`; COPY: the parameter's name.
    pub value: Option<String>,
    pub intrinsic: Option<HirIntrinsic>,
    /// INTRINSIC: its two operands; MATCH: its scrutinee.
    pub operands: Vec<usize>,
    pub arms: Vec<HirArm>,
    pub lineage: Vec<String>,
}

/// G181: a lowered function body: typed nodes, children first, and the node giving its value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HirBody {
    pub root: usize,
    pub nodes: Vec<HirNode>,
}

/// A HIR function: its signature and, when its census records a body inside the lowerable subset
/// (G181, M14), that body. A function without one carries a `BODY_UNOBSERVED` gap and the backend
/// emits none.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IrFunction {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    pub dispatch: Dispatch,
    pub visibility: String,
    pub documentation: Option<String>,
    pub params: Vec<IrParam>,
    pub result: Option<String>,
    /// The census's content fingerprint of the original body (G66), kept to report whether a
    /// reconstructed body is token-identical; never a construction input.
    pub body_fingerprint: Option<String>,
    /// G181: the lowered body; `None` means not observed in the lowerable subset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<HirBody>,
    pub lineage: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstructionGap {
    pub kind: GapKind,
    /// The IR element the gap belongs to.
    pub subject: String,
    pub missing: String,
    /// The essential-complexity debt that owns closing it.
    pub debt: String,
}

/// One construction unit: what Atlas can build of a target, from what, and what it cannot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstructionModule {
    pub schema: String,
    pub module_id: String,
    /// `<path>::<name>` of the target in the census.
    pub target: String,
    pub construction_target: String,
    /// The verified container the records come from (its root identity).
    pub container_root: String,
    pub inputs: Vec<ConstructionInput>,
    pub types: Vec<IrType>,
    pub functions: Vec<IrFunction>,
    pub gaps: Vec<ConstructionGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    pub code: String,
    pub detail: String,
}

fn violation(code: &str, detail: impl Into<String>) -> Violation {
    Violation {
        code: code.into(),
        detail: detail.into(),
    }
}

/// The content identity of `value` with its own id field blanked.
fn identity_of<T: Serialize>(prefix: &str, value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("an IR record always serializes");
    format!("{prefix}:{}", IntegrityDigest::of_bytes(&bytes).as_str())
}

pub fn module_identity(module: &ConstructionModule) -> String {
    let mut blank = module.clone();
    blank.module_id.clear();
    identity_of("construction-module", &blank)
}

/// Checks the input boundary and gap accounting of `module` against the record ids of the
/// container it claims to come from.
pub fn validate_module(
    module: &ConstructionModule,
    container_record_ids: &BTreeSet<String>,
) -> Vec<Violation> {
    let mut out = Vec::new();
    if module.schema != CONSTRUCTION_IR_SCHEMA {
        out.push(violation("SCHEMA", module.schema.clone()));
    }
    if module.module_id != module_identity(module) {
        out.push(violation(
            "MODULE_ID",
            "the module id is not its content identity",
        ));
    }
    if module.construction_target != BOOTSTRAP_CONSTRUCTION_TARGET {
        out.push(violation("TARGET", module.construction_target.clone()));
    }
    let records: BTreeSet<&str> = module
        .inputs
        .iter()
        .filter(|i| i.kind == ConstructionInputKind::CensusRecord)
        .map(|i| i.reference.as_str())
        .collect();
    for record in &records {
        if !container_record_ids.contains(*record) {
            out.push(violation(
                "INPUT_NOT_IN_CONTAINER",
                format!("{record} does not resolve in the container"),
            ));
        }
    }
    if !module
        .inputs
        .iter()
        .any(|i| i.kind == ConstructionInputKind::Design)
    {
        out.push(violation(
            "NO_DESIGN",
            "construction goes through a design over the container",
        ));
    }
    // Every element rests on declared inputs: lineage never reaches outside them.
    let lineages = module
        .types
        .iter()
        .flat_map(|t| {
            std::iter::once((&t.id, &t.lineage))
                .chain(t.variants.iter().map(move |v| (&t.id, &v.lineage)))
        })
        .chain(module.functions.iter().map(|f| (&f.id, &f.lineage)))
        .chain(module.functions.iter().flat_map(|f| {
            f.body
                .iter()
                .flat_map(|b| b.nodes.iter())
                .map(move |n| (&f.id, &n.lineage))
        }));
    for (element, lineage) in lineages {
        if lineage.is_empty() {
            out.push(violation("NO_LINEAGE", element.clone()));
        }
        for record in lineage {
            if !records.contains(record.as_str()) {
                out.push(violation(
                    "LINEAGE_OUTSIDE_INPUTS",
                    format!("{element}: {record}"),
                ));
            }
        }
    }
    // Nothing unobserved is silent: each `None` has its gap.
    let gapped = |subject: &str, kind: GapKind| {
        module
            .gaps
            .iter()
            .any(|g| g.subject == subject && g.kind == kind)
    };
    let mut require = |subject: &str, missing: bool, kind: GapKind| {
        if missing && !gapped(subject, kind) {
            out.push(violation(
                "SILENT_GAP",
                format!("{subject}: {} without a gap", kind.as_str()),
            ));
        }
    };
    for t in &module.types {
        require(&t.id, t.kind.is_none(), GapKind::ItemKindUnobserved);
        require(&t.id, t.derives.is_none(), GapKind::DerivesUnobserved);
        require(&t.id, t.attributes.is_none(), GapKind::AttributesUnobserved);
        require(&t.id, t.visibility.is_none(), GapKind::VisibilityUnobserved);
        for v in &t.variants {
            let subject = format!("{}::{}", t.id, v.name);
            require(
                &subject,
                v.attributes.is_none(),
                GapKind::AttributesUnobserved,
            );
        }
    }
    for f in &module.functions {
        require(&f.id, f.body.is_none(), GapKind::BodyUnobserved);
    }
    for f in &module.functions {
        if let Some(body) = &f.body {
            if gapped(&f.id, GapKind::BodyUnobserved) {
                out.push(body_violation(
                    BodyDefect::GapContradiction,
                    &f.id,
                    "a body and a BODY_UNOBSERVED gap",
                ));
            }
            out.extend(validate_body(module, f, body));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn body_violation(defect: BodyDefect, function: &str, detail: impl std::fmt::Display) -> Violation {
    violation(defect.as_str(), format!("{function}: {detail}"))
}

/// The type of the module a spelling names from inside `owner`'s impl: `Self` or its name.
pub fn resolve_type<'m>(
    module: &'m ConstructionModule,
    owner: Option<&str>,
    spelling: &str,
) -> Option<&'m IrType> {
    let name = if spelling == "Self" { owner? } else { spelling };
    module.types.iter().find(|t| t.name == name)
}

fn derives(ty: &IrType, capability: &str) -> bool {
    ty.derives
        .as_ref()
        .is_some_and(|d| d.iter().any(|x| x == capability))
}

/// Every way `body` of `f` is malformed, untyped or unresolved within `module`.
fn validate_body(module: &ConstructionModule, f: &IrFunction, body: &HirBody) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut flag = |defect: BodyDefect, detail: String| {
        out.push(body_violation(defect, &f.id, detail));
    };
    let type_of = |id: &str| module.types.iter().find(|t| t.id == id);
    let variant_of = |reference: &str| {
        let (ty, variant) = reference.rsplit_once("::")?;
        let ty = type_of(ty)?;
        ty.variants.iter().any(|v| v.name == variant).then_some(ty)
    };
    for (position, node) in body.nodes.iter().enumerate() {
        let at = format!("node {position}");
        if node.id != position {
            flag(BodyDefect::MalformedNode, format!("{at}: id {}", node.id));
        }
        let children = node
            .operands
            .iter()
            .chain(node.arms.iter().map(|a| &a.value));
        for child in children {
            if *child >= position {
                flag(
                    BodyDefect::DanglingChild,
                    format!("{at}: child {child} is not before it"),
                );
            }
        }
        let typed = node.result_type == "bool" || type_of(&node.result_type).is_some();
        if !typed {
            flag(
                BodyDefect::UntypedNode,
                format!("{at}: result type `{}`", node.result_type),
            );
        }
        let child_type = |i: &usize| {
            body.nodes
                .get(*i)
                .filter(|_| *i < position)
                .map(|n| n.result_type.as_str())
        };
        let shape = match node.kind {
            HirNodeKind::Const | HirNodeKind::Copy => {
                node.value.is_some()
                    && node.intrinsic.is_none()
                    && node.operands.is_empty()
                    && node.arms.is_empty()
            }
            HirNodeKind::Intrinsic => {
                node.value.is_none()
                    && node.intrinsic.is_some()
                    && node.operands.len() == 2
                    && node.arms.is_empty()
            }
            HirNodeKind::Match => {
                node.value.is_none()
                    && node.intrinsic.is_none()
                    && node.operands.len() == 1
                    && !node.arms.is_empty()
                    && node.arms.iter().all(|a| !a.patterns.is_empty())
            }
        };
        if !shape {
            flag(
                BodyDefect::MalformedNode,
                format!("{at}: fields do not fit {}", node.kind.as_str()),
            );
            continue;
        }
        match node.kind {
            HirNodeKind::Const => {
                let value = node.value.as_deref().unwrap_or_default();
                let resolved = match value {
                    "true" | "false" => node.result_type == "bool",
                    reference => variant_of(reference).is_some_and(|t| t.id == node.result_type),
                };
                if !resolved {
                    flag(
                        BodyDefect::UnresolvedReference,
                        format!("{at}: constant `{value}` of `{}`", node.result_type),
                    );
                }
            }
            HirNodeKind::Copy => {
                let name = node.value.as_deref().unwrap_or_default();
                let param = f.params.iter().find(|p| p.name == name);
                let ty =
                    param.and_then(|p| resolve_type(module, f.owner.as_deref(), &p.type_spelling));
                match ty {
                    Some(ty) if ty.id == node.result_type => {
                        if !derives(ty, "Copy") {
                            flag(
                                BodyDefect::CapabilityUnobserved,
                                format!("{at}: {} is not observed to derive Copy", ty.name),
                            );
                        }
                    }
                    Some(ty) => flag(
                        BodyDefect::TypeMismatch,
                        format!("{at}: `{name}` is a {}", ty.id),
                    ),
                    None => flag(
                        BodyDefect::UnresolvedReference,
                        format!("{at}: `{name}` is no parameter of a module type"),
                    ),
                }
            }
            HirNodeKind::Intrinsic => {
                let (left, right) = (child_type(&node.operands[0]), child_type(&node.operands[1]));
                if node.result_type != "bool" || left.is_none() || left != right {
                    flag(
                        BodyDefect::TypeMismatch,
                        format!("{at}: {left:?} compared with {right:?}"),
                    );
                } else if let Some(ty) = left.and_then(type_of)
                    && !derives(ty, "PartialEq")
                {
                    flag(
                        BodyDefect::CapabilityUnobserved,
                        format!("{at}: {} is not observed to derive PartialEq", ty.name),
                    );
                }
            }
            HirNodeKind::Match => {
                let Some(scrutinee) = child_type(&node.operands[0]).and_then(type_of) else {
                    flag(
                        BodyDefect::TypeMismatch,
                        format!("{at}: the scrutinee is not a module type"),
                    );
                    continue;
                };
                let mut covered = BTreeSet::new();
                let mut wildcard = false;
                for arm in &node.arms {
                    if child_type(&arm.value) != Some(node.result_type.as_str()) {
                        flag(
                            BodyDefect::TypeMismatch,
                            format!("{at}: an arm is not a `{}`", node.result_type),
                        );
                    }
                    for pattern in &arm.patterns {
                        match pattern {
                            HirPattern::Wildcard => wildcard = true,
                            HirPattern::Variant(reference) => {
                                if variant_of(reference).is_some_and(|t| t.id == scrutinee.id) {
                                    covered.insert(reference.as_str());
                                } else {
                                    flag(
                                        BodyDefect::UnresolvedReference,
                                        format!("{at}: pattern `{reference}`"),
                                    );
                                }
                            }
                        }
                    }
                }
                let missing = scrutinee
                    .variants
                    .iter()
                    .any(|v| !covered.contains(format!("{}::{}", scrutinee.id, v.name).as_str()));
                if missing && !wildcard {
                    flag(
                        BodyDefect::NonExhaustiveMatch,
                        format!("{at}: over {}", scrutinee.name),
                    );
                }
            }
        }
    }
    match body.nodes.get(body.root) {
        None => flag(
            BodyDefect::DanglingChild,
            format!("root {} is outside the body", body.root),
        ),
        Some(root) => {
            let expected = f.result.as_deref().and_then(|r| {
                if r == "bool" {
                    Some("bool".to_owned())
                } else {
                    resolve_type(module, f.owner.as_deref(), r).map(|t| t.id.clone())
                }
            });
            if expected.as_deref() != Some(root.result_type.as_str()) {
                flag(
                    BodyDefect::TypeMismatch,
                    format!(
                        "the root is a `{}` where the function returns {:?}",
                        root.result_type, f.result
                    ),
                );
            }
        }
    }
    out
}

/// A shadow artifact: generated where nothing is admitted from (`target/atlas/shadow`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShadowArtifact {
    pub path: String,
    pub content_hash: String,
    pub backend: String,
    /// The toolchain that built and tested it (`rustc -V`), recorded, never ambient.
    pub toolchain: String,
    /// IR elements the backend emitted and those it could not.
    pub emitted: Vec<String>,
    pub omitted: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OracleUse {
    pub kind: OracleKind,
    pub reference: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct EquivalenceCheck {
    pub kind: EquivalenceKind,
    pub subject: String,
    pub property: String,
    pub result: CheckResult,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfReconstructionReport {
    pub schema: String,
    pub report_id: String,
    pub level: SelfHostingLevel,
    pub target: String,
    pub construction_target: String,
    pub module_id: String,
    pub design_ref: String,
    /// The design's state: construction of a shadow needs a VALIDATED design; admission of a
    /// reconstruction (SH4) needs a SELECTED one from a declared principal.
    pub design_state: crate::design::DesignState,
    pub comparison_ref: Option<String>,
    pub construction_inputs: Vec<ConstructionInput>,
    pub oracle_uses: Vec<OracleUse>,
    pub shadow: Option<ShadowArtifact>,
    pub checks: Vec<EquivalenceCheck>,
    pub gaps: Vec<ConstructionGap>,
    /// Differences from the original the construction declares on purpose.
    pub declared_variations: Vec<String>,
    pub verdict: ReconstructionVerdict,
}

/// The verdict the evidence in `report` supports. A mismatch outranks a gap: a wrong
/// reconstruction is worse than an incomplete one.
pub fn decide(report: &SelfReconstructionReport) -> ReconstructionVerdict {
    let mismatch = |kind| {
        report
            .checks
            .iter()
            .any(|c| c.kind == kind && c.result == CheckResult::Mismatch)
    };
    let verified = |kind| {
        report
            .checks
            .iter()
            .any(|c| c.kind == kind && c.result == CheckResult::Equivalent)
    };
    if report.shadow.is_none() {
        return if report.gaps.is_empty() {
            ReconstructionVerdict::Unsupported
        } else {
            ReconstructionVerdict::ConstructionGap
        };
    }
    if mismatch(EquivalenceKind::Semantic) {
        return ReconstructionVerdict::SemanticMismatch;
    }
    if mismatch(EquivalenceKind::Behavioral)
        || !verified(EquivalenceKind::Behavioral)
        || !verified(EquivalenceKind::Semantic)
    {
        return ReconstructionVerdict::VerificationFailed;
    }
    if !report.gaps.is_empty() {
        return ReconstructionVerdict::ConstructionGap;
    }
    if report.declared_variations.is_empty() {
        ReconstructionVerdict::ReconstructedEquivalent
    } else {
        ReconstructionVerdict::ReconstructedWithDeclaredVariation
    }
}

pub fn report_identity(report: &SelfReconstructionReport) -> String {
    let mut blank = report.clone();
    blank.report_id.clear();
    identity_of("self-reconstruction", &blank)
}

/// Checks that `report` claims only what its evidence supports and kept the input boundary.
pub fn validate_report(
    report: &SelfReconstructionReport,
    module: &ConstructionModule,
) -> Vec<Violation> {
    let mut out = Vec::new();
    if report.schema != RECONSTRUCTION_REPORT_SCHEMA {
        out.push(violation("SCHEMA", report.schema.clone()));
    }
    if report.report_id != report_identity(report) {
        out.push(violation(
            "REPORT_ID",
            "the report id is not its content identity",
        ));
    }
    if report.module_id != module.module_id || report.construction_inputs != module.inputs {
        out.push(violation(
            "MODULE_MISMATCH",
            "the report cites another module or other inputs",
        ));
    }
    if report.gaps != module.gaps {
        out.push(violation(
            "GAPS_MISMATCH",
            "the report hides or adds construction gaps",
        ));
    }
    // An oracle is for verification only: nothing it read is a construction input.
    for oracle in &report.oracle_uses {
        if report
            .construction_inputs
            .iter()
            .any(|i| i.reference == oracle.reference)
        {
            out.push(violation(
                "ORACLE_AS_INPUT",
                format!(
                    "{} is both an oracle and a construction input",
                    oracle.reference
                ),
            ));
        }
    }
    if report.shadow.is_some() && report.oracle_uses.is_empty() {
        out.push(violation(
            "UNVERIFIED",
            "a shadow was built but nothing compared it to the original",
        ));
    }
    let decided = decide(report);
    if report.verdict != decided {
        out.push(violation(
            "VERDICT",
            format!(
                "{} claimed where the evidence supports {}",
                report.verdict.as_str(),
                decided.as_str()
            ),
        ));
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests;
