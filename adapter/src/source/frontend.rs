//! Typed structural source boundary.
//!
//! A source frontend recognizes source families and exposes stable adapter identity. Deeper syntax
//! engines (tree-sitter, language servers, compiler metadata, SCIP, etc.) are implementation
//! mechanisms behind this contract; they do not own canonical engineering truth.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::{
    fs,
    path::{Path, PathBuf},
};

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

static HTML: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.html.bootstrap.v1",
    language: "html",
    extensions: &["html", "htm"],
};
static CSS: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.css.bootstrap.v1",
    language: "css",
    extensions: &["css"],
};
/// A file spliced into Rust source by `include!` (G61). The compiler itself parses such a file as
/// Rust tokens in expression or item position -- never as a module -- so it is neither
/// "unrecognized data" nor an ordinary `.rs` module. It has no extension of its own (`.in` is used
/// by autoconf and others), so it is never resolved by path: only `resolve_included_fragment`,
/// which observes the including `include!` call, may classify a file as this language.
static RUST_INCLUDE_FRAGMENT: ExtensionSourceFrontend = ExtensionSourceFrontend {
    id: "atlas.source.rust-include-fragment.bootstrap.v1",
    language: "rust-include-fragment",
    extensions: &[],
};

static BUILTIN_FRONTENDS: [&'static dyn SourceFrontend; 10] = [
    &RUST,
    &TYPESCRIPT,
    &JAVASCRIPT,
    &MARKDOWN,
    &TOML,
    &JSON,
    &YAML,
    &HTML,
    &CSS,
    &RUST_INCLUDE_FRAGMENT,
];

pub fn source_frontends() -> &'static [&'static dyn SourceFrontend] {
    &BUILTIN_FRONTENDS
}

pub fn resolve_source_frontend(path: &Path) -> Option<SourceFrontendMatch> {
    source_frontends()
        .iter()
        .find_map(|frontend| frontend.resolve(path))
}

/// The registered frontend for a language an inventory record already carries. Languages are
/// unique across the registry, so this recovers the frontend of an artifact that was classified by
/// observation (`resolve_included_fragment`) rather than by its path.
pub fn source_frontend_for_language(language: &str) -> Option<SourceFrontendMatch> {
    source_frontends()
        .iter()
        .find(|frontend| frontend.language() == language)
        .map(|frontend| SourceFrontendMatch {
            frontend_id: frontend.id(),
            language: frontend.language(),
        })
}

/// Evidence that a sibling Rust file splices `path` in with `include!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedFragment {
    pub matched: SourceFrontendMatch,
    /// The first (by file name) sibling `.rs` file whose code calls `include!("<file name>")`.
    pub includer: PathBuf,
}

/// Classifies `path` as a Rust include fragment when, and only when, a `.rs` file in the same
/// directory calls `include!("<file name of path>")`. The includer is tokenized with `proc_macro2`
/// (comments and string contents are never mistaken for code; `include_str!`, `include_bytes!` and
/// `my_include!` are different identifiers), but this is still a bootstrap check, not name
/// resolution: an `include!` behind `concat!`/`env!`, across directories, or through a macro-built
/// path is not seen, and such a file stays unrecognized (`UNKNOWN`) rather than being guessed.
/// Sibling files above `max_bytes` are not read.
pub fn resolve_included_fragment(path: &Path, max_bytes: u64) -> Option<IncludedFragment> {
    let name = path.file_name()?.to_str()?;
    if resolve_source_frontend(path).is_some() {
        return None;
    }
    let mut siblings = fs::read_dir(path.parent()?)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
                && fs::symlink_metadata(candidate)
                    .is_ok_and(|meta| meta.is_file() && meta.len() <= max_bytes)
        })
        .collect::<Vec<_>>();
    siblings.sort();
    siblings.into_iter().find_map(|includer| {
        let tokens = fs::read_to_string(&includer)
            .ok()?
            .parse::<TokenStream>()
            .ok()?;
        calls_include(tokens, name).then(|| IncludedFragment {
            matched: RUST_INCLUDE_FRAGMENT.resolve_unconditionally(),
            includer,
        })
    })
}

/// Whether `tokens` contain `include ! ( "<name>" )` at any group depth.
fn calls_include(tokens: TokenStream, name: &str) -> bool {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    tokens.iter().enumerate().any(|(at, token)| match token {
        TokenTree::Group(group) => calls_include(group.stream(), name),
        TokenTree::Ident(ident) if ident == "include" => {
            matches!(tokens.get(at + 1), Some(TokenTree::Punct(bang)) if bang.as_char() == '!')
                && matches!(tokens.get(at + 2), Some(TokenTree::Group(args))
                    if args.delimiter() == Delimiter::Parenthesis
                        && is_string_literal(args.stream(), name))
        }
        _ => false,
    })
}

fn is_string_literal(args: TokenStream, name: &str) -> bool {
    let mut args = args.into_iter();
    matches!(
        (args.next(), args.next()),
        (Some(TokenTree::Literal(literal)), None) if literal.to_string() == format!("{name:?}")
    )
}

impl ExtensionSourceFrontend {
    const fn resolve_unconditionally(&self) -> SourceFrontendMatch {
        SourceFrontendMatch {
            frontend_id: self.id,
            language: self.language,
        }
    }
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

        let html = resolve_source_frontend(Path::new("fixtures/page.HTM")).unwrap();
        assert_eq!(html.language, "html");
        let css = resolve_source_frontend(Path::new("fixtures/probe.css")).unwrap();
        assert_eq!(css.frontend_id, "atlas.source.css.bootstrap.v1");
        assert!(
            resolve_source_frontend(Path::new("core/src/vectors.in")).is_none(),
            "an include fragment is never recognized by its path alone"
        );
    }

    #[test]
    fn builtin_frontend_languages_are_unique_and_recoverable() {
        let languages = source_frontends()
            .iter()
            .map(|frontend| frontend.language())
            .collect::<BTreeSet<_>>();
        assert_eq!(languages.len(), source_frontends().len());
        for frontend in source_frontends() {
            let matched = source_frontend_for_language(frontend.language()).unwrap();
            assert_eq!(matched.frontend_id, frontend.id());
        }
        assert!(source_frontend_for_language("cobol").is_none());
    }

    fn fragment_dir(files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "atlas-fragment-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for (name, text) in files {
            fs::write(dir.join(name), text).unwrap();
        }
        dir
    }

    #[test]
    fn an_include_fragment_is_recognized_only_through_an_observed_include() {
        let dir = fragment_dir(&[
            ("vectors.in", "&[(0, \"af13\")]"),
            (
                "hash.rs",
                "// include!(\"stale.in\")\n/* include!(\"block.in\") */\nconst S: &str = \"include!(\\\"quoted.in\\\")\";\nfn v() -> &'static [(usize, &'static str)] { { include!(\"vectors.in\") } }\n",
            ),
            ("stale.in", "1"),
            ("block.in", "1"),
            ("quoted.in", "1"),
            ("page.in", "<p>"),
            ("data.in", "x"),
            (
                "other.rs",
                "const T: &str = include_str!(\"page.in\");\nmy_include!(\"data.in\");\ninclude!(\"hash.rs\");\n",
            ),
            (
                "calls.rs",
                "fn f(include: &str) -> bool { include > (\"fncall.in\") }\n",
            ),
            ("fncall.in", "1"),
            ("notes.txt", "include!(\"textonly.in\")"),
            ("textonly.in", "1"),
        ]);
        let fragment = resolve_included_fragment(&dir.join("vectors.in"), 1 << 20).unwrap();
        assert_eq!(fragment.matched.language, "rust-include-fragment");
        assert_eq!(fragment.includer, dir.join("hash.rs"));

        for (name, why) in [
            (
                "stale.in",
                "an include! inside a line comment is not a call",
            ),
            (
                "block.in",
                "an include! inside a block comment is not a call",
            ),
            (
                "quoted.in",
                "an include! inside a string literal is not a call",
            ),
            ("page.in", "include_str! splices data, not Rust tokens"),
            ("data.in", "a differently named macro is not include!"),
            ("missing.in", "no includer at all"),
            ("textonly.in", "only a .rs file can be an includer"),
            (
                "fncall.in",
                "a function named include is not the include! macro",
            ),
        ] {
            assert!(
                resolve_included_fragment(&dir.join(name), 1 << 20).is_none(),
                "{why}"
            );
        }
        assert!(
            resolve_included_fragment(&dir.join("vectors.in"), 8).is_none(),
            "an includer above the read bound is not read"
        );
        assert!(
            resolve_included_fragment(&dir.join("hash.rs"), 1 << 20).is_none(),
            "a path-recognized file is never reclassified"
        );
        fs::remove_dir_all(&dir).unwrap();
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
