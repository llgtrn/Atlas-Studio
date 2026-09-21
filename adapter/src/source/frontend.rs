//! Typed structural source boundary.
//!
//! A source frontend recognizes source families and exposes stable adapter identity. Deeper syntax
//! engines (tree-sitter, language servers, compiler metadata, SCIP, etc.) are implementation
//! mechanisms behind this contract; they do not own canonical engineering truth.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceFrontendMatch {
    pub frontend_id: &'static str,
    pub language: &'static str,
}

pub trait SourceFrontend: Sync {
    fn id(&self) -> &'static str;
    fn language(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];

    fn supports(&self, path: &Path) -> bool {
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            return false;
        };
        self.extensions()
            .iter()
            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    }

    fn resolve(&self, path: &Path) -> Option<SourceFrontendMatch> {
        self.supports(path).then_some(SourceFrontendMatch {
            frontend_id: self.id(),
            language: self.language(),
        })
    }
}

#[derive(Debug)]
struct ExtensionSourceFrontend {
    id: &'static str,
    language: &'static str,
    extensions: &'static [&'static str],
}

impl SourceFrontend for ExtensionSourceFrontend {
    fn id(&self) -> &'static str {
        self.id
    }

    fn language(&self) -> &'static str {
        self.language
    }

    fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }
}

static RUST: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.rust.bootstrap.v1",
    language: "rust",
    extensions: &["rs"],
};
static TYPESCRIPT: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.typescript.bootstrap.v1",
    language: "typescript",
    extensions: &["ts", "tsx"],
};
static JAVASCRIPT: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.javascript.bootstrap.v1",
    language: "javascript",
    extensions: &["js", "jsx", "mjs", "cjs"],
};
static MARKDOWN: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.markdown.bootstrap.v1",
    language: "markdown",
    extensions: &["md"],
};
static TOML: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.toml.bootstrap.v1",
    language: "toml",
    extensions: &["toml"],
};
static JSON: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.json.bootstrap.v1",
    language: "json",
    extensions: &["json"],
};
static YAML: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.yaml.bootstrap.v1",
    language: "yaml",
    extensions: &["yaml", "yml"],
};

static BUILTIN_FRONTENDS: [&'static dyn SourceFrontend; 7] = [
    &RUST,
    &TYPESCRIPT,
    &JAVASCRIPT,
    &MARKDOWN,
    &TOML,
    &JSON,
    &YAML,
];

pub fn source_frontends() -> &'static [&'static dyn SourceFrontend] {
    &BUILTIN_FRONTENDS
}

pub fn resolve_source_frontend(path: &Path) -> Option<SourceFrontendMatch> {
    source_frontends()
        .iter()
        .find_map(|frontend| frontend.resolve(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn registry_resolves_supported_source_families() {
        let rust = resolve_source_frontend(Path::new("core/src/lib.RS")).unwrap();
        assert_eq!(rust.frontend_id, "atlas.source.rust.bootstrap.v1");
        assert_eq!(rust.language, "rust");

        let tsx = resolve_source_frontend(Path::new("apps/studio/src/App.tsx")).unwrap();
        assert_eq!(tsx.language, "typescript");

        assert!(resolve_source_frontend(Path::new("artifact.unknown")).is_none());
    }

    #[test]
    fn builtin_frontend_ids_are_unique() {
        let ids = source_frontends()
            .iter()
            .map(|frontend| frontend.id())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), source_frontends().len());
    }
}
