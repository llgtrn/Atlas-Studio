//! The Rust backend of the construction IR (G153): IR in, Rust source text out, deterministic.
//!
//! The emitted text is a projection, never truth (`COMPILER-IR-PIPELINE.md`, delegated phase-1
//! backend). The backend reads nothing but the module. It emits an element only when everything
//! the element needs was observed. Otherwise it omits the element and says why; it never guesses
//! a missing kind, derive, attribute or body.

use super::{ConstructionModule, IrType, TypeKind};

pub const RUST_BACKEND: &str = "atlas.construction.rust-backend.v0";

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

/// Emits every emittable type of `module`; functions are omitted until IR bodies exist.
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
    for function in &module.functions {
        omitted.push(format!("{}: no lowerable body in IR v0", function.id));
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
