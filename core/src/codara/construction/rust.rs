//! The Rust backend of the construction IR (G153; bodies G181): IR in, Rust source text out,
//! deterministic.
//!
//! The emitted text is a projection, never truth (`COMPILER-IR-PIPELINE.md`, delegated phase-1
//! backend). The backend reads nothing but the module. It emits an element only when everything
//! the element needs was observed. Otherwise it omits the element and says why; it never guesses
//! a missing kind, derive, attribute or body.

use super::{
    ConstructionModule, Dispatch, HirBody, HirIntrinsic, HirNodeKind, HirPattern, IrFunction,
    IrType, TypeKind,
};

pub const RUST_BACKEND: &str = "atlas.construction.rust-backend.v1";

/// Derive macros the backend knows how to bring into scope: the prelude's, and serde's. Any other
/// derive leaves its type unemitted. This table is a declared backend assumption, recorded.
const PRELUDE_DERIVES: &[&str] = &[
    "Debug",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Default",
];
const SERDE_DERIVES: &[&str] = &["Serialize", "Deserialize"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustEmission {
    pub source: String,
    pub emitted: Vec<String>,
    /// `<element id>: <why>` for every element the backend could not emit.
    pub omitted: Vec<String>,
    pub assumptions: Vec<String>,
}

/// Why `ty` cannot be emitted, if it cannot. Anything unobserved is a refusal on its own, not only
/// through `missing`: no path emits a guess.
fn refusal(ty: &IrType) -> Option<String> {
    let (Some(kind), Some(derives), Some(_), Some(_)) = (
        ty.kind.as_ref(),
        ty.derives.as_ref(),
        ty.attributes.as_ref(),
        ty.visibility.as_ref(),
    ) else {
        return Some(format!("unobserved {}", missing(ty).join(", ")));
    };
    if *kind != TypeKind::Enum {
        return Some(format!("{} types are not emitted by v0", kind.as_str()));
    }
    if let Some(unknown) = derives
        .iter()
        .find(|d| !PRELUDE_DERIVES.contains(&d.as_str()) && !SERDE_DERIVES.contains(&d.as_str()))
    {
        return Some(format!("derive {unknown} is not in the backend's table"));
    }
    if ty.variants.is_empty() {
        return Some("an enum without observed variants".into());
    }
    if let Some(variant) = ty.variants.iter().find(|v| v.attributes.is_none()) {
        return Some(format!(
            "variant {} has unobserved attributes",
            variant.name
        ));
    }
    None
}

fn missing(ty: &IrType) -> Vec<&'static str> {
    [
        (ty.kind.is_none(), "kind"),
        (ty.derives.is_none(), "derives"),
        (ty.attributes.is_none(), "attributes"),
        (ty.visibility.is_none(), "visibility"),
    ]
    .into_iter()
    .filter_map(|(absent, what)| absent.then_some(what))
    .collect()
}

fn doc(out: &mut String, indent: &str, documentation: &Option<String>) {
    if let Some(text) = documentation {
        out.push_str(&format!("{indent}/// {text}\n"));
    }
}

fn visibility_prefix(visibility: &str) -> String {
    if visibility == "inherited" {
        String::new()
    } else {
        format!("{visibility} ")
    }
}

/// G181: why `function` cannot be emitted, if it cannot. It needs its body, its owner type
/// emitted, inherent dispatch and parameters it can spell: none, or `self` by value.
fn function_refusal(function: &IrFunction, emitted_types: &[&IrType]) -> Option<String> {
    let Some(body) = &function.body else {
        return Some("no lowerable body".into());
    };
    if body.nodes.get(body.root).is_none() {
        return Some("a body without its root".into());
    }
    if !matches!(
        function.dispatch,
        Dispatch::InherentMethod | Dispatch::AssociatedFunction
    ) {
        return Some(format!(
            "{} functions are not emitted by v1",
            function.dispatch.as_str()
        ));
    }
    if !function
        .owner
        .as_deref()
        .is_some_and(|o| emitted_types.iter().any(|t| t.name == o))
    {
        return Some("its owner type is not emitted".into());
    }
    if function.result.is_none() {
        return Some("no observed result type".into());
    }
    match function.params.as_slice() {
        [] => None,
        [p] if p.name == "self" && p.type_spelling == "Self" => None,
        _ => Some("parameters other than `self` by value".into()),
    }
}

/// The Rust expression of node `id` of `body` inside the impl of `owner` (type id).
fn expression(body: &HirBody, id: usize, owner: &str, indent: &str) -> String {
    let Some(node) = body.nodes.get(id) else {
        return String::new();
    };
    let variant = |reference: &str| match reference.rsplit_once("::") {
        Some((ty, variant)) if ty == owner => format!("Self::{variant}"),
        Some((ty, variant)) => format!("{}::{variant}", ty.rsplit("::").next().unwrap_or(ty)),
        None => reference.to_owned(),
    };
    // A compound operand is parenthesized: comparisons do not chain in Rust.
    let operand = |i: usize| {
        let text = expression(body, i, owner, indent);
        match body.nodes.get(i).map(|n| n.kind) {
            Some(HirNodeKind::Intrinsic | HirNodeKind::Match) => format!("({text})"),
            _ => text,
        }
    };
    match node.kind {
        HirNodeKind::Const => {
            let value = node.value.as_deref().unwrap_or_default();
            if value == "true" || value == "false" {
                value.to_owned()
            } else {
                variant(value)
            }
        }
        HirNodeKind::Copy => node.value.clone().unwrap_or_default(),
        HirNodeKind::Intrinsic => {
            let op = match node.intrinsic {
                Some(HirIntrinsic::Eq) => "==",
                _ => "!=",
            };
            let (left, right) = (node.operands[0], node.operands[1]);
            format!("{} {op} {}", operand(left), operand(right))
        }
        HirNodeKind::Match => {
            let inner = format!("{indent}    ");
            let mut out = format!("match {} {{\n", operand(node.operands[0]));
            for arm in &node.arms {
                let patterns: Vec<String> = arm
                    .patterns
                    .iter()
                    .map(|p| match p {
                        HirPattern::Variant(reference) => variant(reference),
                        HirPattern::Wildcard => "_".into(),
                    })
                    .collect();
                out.push_str(&format!(
                    "{inner}{} => {},\n",
                    patterns.join(" | "),
                    expression(body, arm.value, owner, &inner)
                ));
            }
            out.push_str(&format!("{indent}}}"));
            out
        }
    }
}

fn emit_function(out: &mut String, function: &IrFunction, owner: &IrType) {
    let Some(body) = &function.body else {
        return;
    };
    doc(out, "    ", &function.documentation);
    let params: Vec<&str> = function.params.iter().map(|_| "self").collect();
    out.push_str(&format!(
        "    {}fn {}({}) -> {} {{\n        {}\n    }}\n",
        visibility_prefix(&function.visibility),
        function.name,
        params.join(", "),
        function.result.as_deref().unwrap_or_default(),
        expression(body, body.root, &owner.id, "        ")
    ));
}

/// Emits every emittable type of `module` and, in an inherent impl after it, every method whose
/// body the IR carries.
pub fn emit(module: &ConstructionModule) -> RustEmission {
    let mut body = String::new();
    let mut emitted = Vec::new();
    let mut omitted = Vec::new();
    let mut serde: Vec<&str> = Vec::new();
    for ty in &module.types {
        if let Some(why) = refusal(ty) {
            omitted.push(format!("{}: {why}", ty.id));
            continue;
        }
        let (derives, attributes, visibility) = (
            ty.derives.as_deref().unwrap_or_default(),
            ty.attributes.as_deref().unwrap_or_default(),
            ty.visibility.as_deref().unwrap_or_default(),
        );
        for derive in derives {
            if SERDE_DERIVES.contains(&derive.as_str()) && !serde.contains(&derive.as_str()) {
                serde.push(derive);
            }
        }
        body.push('\n');
        doc(&mut body, "", &ty.documentation);
        if !derives.is_empty() {
            body.push_str(&format!("#[derive({})]\n", derives.join(", ")));
        }
        for attribute in attributes {
            body.push_str(&format!("#[{attribute}]\n"));
        }
        body.push_str(&format!(
            "{}enum {} {{\n",
            visibility_prefix(visibility),
            ty.name
        ));
        for variant in &ty.variants {
            doc(&mut body, "    ", &variant.documentation);
            for attribute in variant.attributes.as_deref().unwrap_or_default() {
                body.push_str(&format!("    #[{attribute}]\n"));
            }
            body.push_str(&format!("    {},\n", variant.name));
        }
        body.push_str("}\n");
        emitted.push(ty.id.clone());
    }
    let emitted_types: Vec<&IrType> = module
        .types
        .iter()
        .filter(|t| emitted.contains(&t.id))
        .collect();
    for owner in &emitted_types {
        let mut methods = String::new();
        for function in module
            .functions
            .iter()
            .filter(|f| f.owner.as_deref() == Some(owner.name.as_str()))
        {
            if function_refusal(function, &emitted_types).is_none() {
                emit_function(&mut methods, function, owner);
                emitted.push(function.id.clone());
            }
        }
        if !methods.is_empty() {
            body.push_str(&format!("\nimpl {} {{\n{methods}}}\n", owner.name));
        }
    }
    for function in &module.functions {
        if let Some(why) = function_refusal(function, &emitted_types) {
            omitted.push(format!("{}: {why}", function.id));
        }
    }
    let mut source = format!(
        "//! Shadow reconstruction of `{}` by Atlas from construction module `{}`.\n//! Generated by {RUST_BACKEND}; never admitted.\n",
        module.target, module.module_id
    );
    if !serde.is_empty() {
        serde.sort();
        source.push_str(&format!("\nuse serde::{{{}}};\n", serde.join(", ")));
    }
    source.push_str(&body);
    RustEmission {
        source,
        emitted,
        omitted,
        assumptions: vec![format!(
            "derive paths: {} from the prelude, {} from serde",
            PRELUDE_DERIVES.join(" "),
            SERDE_DERIVES.join(" ")
        )],
    }
}
