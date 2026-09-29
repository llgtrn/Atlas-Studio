use super::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// A throwaway registry source tree: `<root>/<index>/<name>-<version>/` per package, complete
/// (`.cargo-ok`) unless asked otherwise.
struct Registry {
    root: PathBuf,
}

impl Registry {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "atlas-locked-traits-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("index.crates.io-0000000000000000")).unwrap();
        Self { root }
    }

    fn package(&self, name: &str, files: &[(&str, &str)]) -> &Self {
        let dir = self
            .root
            .join("index.crates.io-0000000000000000")
            .join(format!("{name}-1.0.0"));
        for (path, text) in files {
            let file = dir.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, text).unwrap();
        }
        if !dir.join("Cargo.toml").exists() {
            fs::write(
                dir.join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\n"),
            )
            .unwrap();
        }
        fs::write(dir.join(".cargo-ok"), "{\"v\":1}").unwrap();
        self
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// A lockfile: `member` (a path package) depends on the first package; each `(name, deps)`.
fn lock(packages: &[(&str, &[&str])]) -> String {
    let mut text =
        String::from("version = 4\n\n[[package]]\nname = \"member\"\nversion = \"0.1.0\"\n");
    text.push_str(&format!("dependencies = [\"{}\"]\n", packages[0].0));
    for (name, deps) in packages {
        text.push_str(&format!(
            "\n[[package]]\nname = \"{name}\"\nversion = \"1.0.0\"\nsource = \"{REGISTRY}\"\n"
        ));
        if !deps.is_empty() {
            let quoted: Vec<String> = deps.iter().map(|d| format!("\"{d}\"")).collect();
            text.push_str(&format!("dependencies = [{}]\n", quoted.join(", ")));
        }
    }
    text
}

fn read(registry: &Registry, packages: &[(&str, &[&str])]) -> LockedTraitMethods {
    read_locked_trait_methods(&lock(packages), &registry.root)
}

fn reason(table: &LockedTraitMethods, package: &str) -> String {
    table
        .readings
        .iter()
        .find(|r| r.name == package)
        .and_then(|r| match &r.outcome {
            PackageRead::Refused(reason) => Some(reason.clone()),
            PackageRead::Read(_) => None,
        })
        .unwrap_or_default()
}

#[test]
fn a_closure_answers_the_union_of_its_trait_methods() {
    let registry = Registry::new();
    registry
        .package(
            "outer",
            &[(
                "src/lib.rs",
                "pub use inner::Consume;\npub trait Look { fn look(&self); fn r#type(&self); }\n\
                 pub struct S;\nimpl S { pub fn inherent(self) {} }\n",
            )],
        )
        .package(
            "inner",
            &[(
                "src/lib.rs",
                "pub trait Consume { fn consume(self) -> u8 { 0 } }\n",
            )],
        );
    let table = read(&registry, &[("outer", &["inner"]), ("inner", &[])]);
    // Its own traits and its dependency's, by any receiver, and a raw identifier's name.
    for name in ["look", "consume", "type", "r#type"] {
        assert!(!table.lacks("outer", name), "{name}");
    }
    // An inherent method is no trait method; an unrelated name is absent.
    assert!(table.lacks("outer", "inherent"));
    assert!(table.lacks("outer", "render"));
    // A dependency's closure does not include its dependant.
    assert!(table.lacks("inner", "look"));
    assert!(!table.lacks("inner", "consume"));
    // An unknown package or a missing one never lacks anything.
    assert!(!table.lacks("absent", "render"));
    assert!(table.direct_dependency("member", "outer"));
    assert!(!table.direct_dependency("member", "inner"));
    assert!(table.digest.is_some());
}

/// A procedural macro crate exporting an attribute, two derives and a function-like macro.
const PROC_MACROS: &str = "extern crate proc_macro;\nuse proc_macro::TokenStream;\n\
    #[proc_macro_attribute]\npub fn generate(_: TokenStream, i: TokenStream) -> TokenStream { i }\n\
    #[proc_macro_derive(Serialize, attributes(serde))]\npub fn s(i: TokenStream) -> TokenStream { i }\n\
    #[proc_macro_derive(Deserialize)]\npub fn d(i: TokenStream) -> TokenStream { i }\n\
    #[proc_macro]\npub fn define_traits(i: TokenStream) -> TokenStream { i }\n";

#[test]
fn every_unseen_declaration_makes_the_closure_unknown() {
    let cases: [(&str, &str); 21] = [
        ("attribute macro", "#[generate]\nimpl X for Y {}\n"),
        (
            "attribute macro by path",
            "#[pm::generate]\npub fn f() {}\n",
        ),
        (
            "custom derive",
            "#[derive(Clone, Serialize)]\npub struct S;\n",
        ),
        (
            "derive under cfg_attr",
            "#[cfg_attr(feature = \"x\", derive(Deserialize))]\npub struct S;\n",
        ),
        ("renamed procedural macro", "use pm::Serialize as Ser;\n"),
        (
            "trait item attribute",
            "pub trait T { #[generate] fn a(&self); }\n",
        ),
        ("trait body macro", "pub trait T { methods!(); }\n"),
        (
            "procedural item macro",
            "define_traits! { pub trait T {} }\n",
        ),
        (
            "include",
            "include!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));\n",
        ),
        ("renamed include", "use std::include as inc;\n"),
        (
            "metavariable method name",
            "macro_rules! m { ($n:ident) => { pub trait T { fn $n(self); } }; }\n",
        ),
        (
            "metavariable trait body",
            "macro_rules! m { ($($t:tt)*) => { pub trait T { $($t)* } }; }\n",
        ),
        (
            "macro in a transcribed trait",
            "macro_rules! m { () => { pub trait T { fn a(&self); other!(); } }; }\n",
        ),
        ("path out", "#[path = \"../../elsewhere.rs\"]\nmod m;\n"),
        (
            "re-exported proc_macro",
            "extern crate proc_macro;\npub use proc_macro::MultiSpan;\n",
        ),
        (
            "renamed proc_macro",
            "extern crate proc_macro as pm2;\npub use pm2::MultiSpan;\n",
        ),
        ("does not parse", "pub trait {\n"),
        (
            "derive in a transcriber",
            "macro_rules! m { () => { #[derive(Serialize)] pub struct S; }; }\nm!();\n",
        ),
        (
            "attribute in an invocation",
            "macro_rules! m { ($($t:tt)*) => { $($t)* }; }\nm! { #[generate] pub struct S; }\n",
        ),
        (
            "procedural macro in a transcriber",
            "macro_rules! m { () => { define_traits! {} }; }\n",
        ),
        (
            "include in a transcriber",
            "macro_rules! m { () => { include!(\"x.rs\"); }; }\n",
        ),
    ];
    for (label, source) in cases {
        let registry = Registry::new();
        registry
            .package("dep", &[("src/lib.rs", source)])
            .package("pm", &[("src/lib.rs", PROC_MACROS)]);
        let table = read(&registry, &[("dep", &["pm"]), ("pm", &[])]);
        assert!(!table.lacks("dep", "anything"), "{label}");
    }
    let registry = Registry::new();
    registry.package("dep", &[("src/lib.rs", "pub macro m() {}\n")]);
    let table = read(&registry, &[("dep", &[])]);
    assert!(!table.lacks("dep", "anything"), "verbatim item");
}

#[test]
fn a_name_no_procedural_macro_of_the_closure_exports_is_inert() {
    // Without the procedural macro crate locked, a feature-gated attribute, derive or macro of
    // that name never expands: it would not compile.
    let registry = Registry::new();
    registry.package(
        "dep",
        &[(
            "src/lib.rs",
            "#[cfg_attr(feature = \"serde\", derive(serde::Deserialize))]\npub struct S;\n\
             #[cfg_attr(feature = \"np\", no_panic::no_panic)]\npub fn f() {}\n\
             #[generate]\npub struct G;\npub trait T { #[inline] fn t(&self) { format_args!(\"\"); } }\n",
        )],
    );
    let table = read(&registry, &[("dep", &[])]);
    assert_eq!(reason(&table, "dep"), "");
    assert!(table.lacks("dep", "anything"));
    assert!(!table.lacks("dep", "t"));
    // With it locked, the same attribute is an unseen expansion.
    registry.package("pm", &[("src/lib.rs", PROC_MACROS)]);
    let table = read(&registry, &[("dep", &["pm"]), ("pm", &[])]);
    assert!(!table.lacks("dep", "anything"));
}

#[test]
fn a_header_a_metavariable_completes_counts_once_the_keyword_can_bind() {
    let rebind = "macro_rules! m { ($k:tt $n:ident {}) => { pub $k $n { fn hidden(self); } }; }\n\
         macro_rules! n { ($t:ty) => { impl Clone for $t { fn clone(&self) -> Self { *self } } impl $t { fn inherent(self) {} } }; }\n\
         macro_rules! Token { [trait] => { () }; [$x:tt] => { () }; }\n\
         macro_rules! Bind { [$x:tt] => { () }; }\n\
         macro_rules! wrap { ($($t:tt)*) => { $($t)* }; }\n";
    let registry = Registry::new();
    // No input passes the keyword to a macro: a metavariable never binds `trait`, and a
    // literally matched `Token![trait]` binds nothing either.
    registry.package(
        "dep",
        &[(
            "src/lib.rs",
            &format!("{rebind}wrap! {{ pub struct S(Token![trait]); }}\n"),
        )],
    );
    let table = read(&registry, &[("dep", &[])]);
    assert!(table.lacks("dep", "hidden"));
    // An input holding the keyword: `m!` may pair it with its own body.
    let registry = Registry::new();
    registry.package(
        "dep",
        &[("src/lib.rs", &format!("{rebind}m! {{ trait X {{}} }}\n"))],
    );
    let table = read(&registry, &[("dep", &[])]);
    assert!(!table.lacks("dep", "hidden"));
    // A lone keyword a metavariable rule may bind.
    let registry = Registry::new();
    registry.package(
        "dep",
        &[(
            "src/lib.rs",
            &format!("{rebind}wrap! {{ pub struct S(Bind![trait]); }}\n"),
        )],
    );
    let table = read(&registry, &[("dep", &[])]);
    assert!(!table.lacks("dep", "hidden"));
    // An impl header is never a trait, metavariables or not.
    assert!(table.lacks("dep", "clone"));
    assert!(table.lacks("dep", "inherent"));
    // A transcribed `trait` whose body is a metavariable.
    let registry = Registry::new();
    registry.package(
        "dep",
        &[(
            "src/lib.rs",
            "macro_rules! t { ($b:tt) => { pub trait T $b }; }\n",
        )],
    );
    let table = read(&registry, &[("dep", &[])]);
    assert!(!table.lacks("dep", "anything"));
}

#[test]
fn allowed_forms_keep_the_closure_known() {
    let registry = Registry::new();
    registry.package(
        "dep",
        &[
            (
                "src/lib.rs",
                "#![no_std]\n#![cfg_attr(docsrs, feature(doc_cfg))]\nextern crate alloc;\n\
                 extern crate proc_macro;\nuse proc_macro::TokenStream;\n\
                 #[derive(Clone, Copy, Debug, core::hash::Hash)]\n#[rustfmt::skip]\npub struct S;\n\
                 #[cfg_attr(test, allow(dead_code), doc = \"x\")]\n#[unsafe(no_mangle)]\npub fn f() {}\n\
                 pub use core::iter::Iterator;\n#[path = \"sub/inner.rs\"]\nmod inner;\n\
                 macro_rules! make { ($n:ident) => { pub trait $crate_like {} pub struct $n; \
                 impl $n { fn $n(&self) {} } pub trait Made { fn made(self); } }; }\n\
                 make!(Thing);\nthread_local! { static X: u8 = 0; }\n\
                 fn local() { trait Hidden { fn hidden(self); } }\n\
             include!(\"sub/included.rs\");\n\
             #[cfg(feature = \"gen\")]\ninclude!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));\n\
             #[cfg(all(unix, feature = \"gen\"))]\ninclude!(concat!(env!(\"OUT_DIR\"), \"/unix.rs\"));\n\
             #[cfg(any(feature = \"gen\", feature = \"other\"))]\npub trait Gated { fn gated(self); }\n",
            ),
            ("src/sub/included.rs", "pub trait Included { fn included(&self); }\n"),
            // Only the package's own top-level test, bench and example targets are skipped.
            ("src/tests/nested.rs", "pub trait Nested { fn nested(&self); }\n"),
            ("tests/it.rs", "pub trait Integration { fn integration(self); }\n"),
            (
                "Cargo.toml",
                "[package]\nname = \"dep\"\nversion = \"1.0.0\"\n\
                 [features]\ngen = [\"dep:bindgen\"]\n\
                 [build-dependencies.bindgen]\nversion = \"1\"\noptional = true\n",
            ),
            ("src/sub/inner.rs", "pub trait Inner { fn inner(&self); }\n"),
        ],
    );
    let table = read(&registry, &[("dep", &[])]);
    assert_eq!(reason(&table, "dep"), "");
    assert!(!table.lacks("dep", "made"));
    assert!(!table.lacks("dep", "inner"));
    assert!(!table.lacks("dep", "included"));
    assert!(!table.lacks("dep", "nested"));
    // `any` holds if any part may: `other` is no feature, but the trait is read regardless.
    assert!(!table.lacks("dep", "gated"));
    assert!(
        table.lacks("dep", "integration"),
        "{:?}",
        table.unknown("dep")
    );
    // A trait in a function body is local: it never reaches a dependant.
    assert!(table.lacks("dep", "hidden"));
    assert!(table.lacks("dep", "f"));
}

#[test]
fn an_item_macro_invocation_is_read_for_the_traits_it_spells() {
    let registry = Registry::new();
    registry
        .package(
            "user",
            &[("src/lib.rs", "helper! { pub trait T { fn t(&self); } }\n")],
        )
        .package(
            "provider",
            &[(
                "src/lib.rs",
                "#[macro_export]\nmacro_rules! helper { ($($t:tt)*) => { $($t)* }; }\n",
            )],
        )
        .package(
            "sibling",
            &[(
                "src/lib.rs",
                "#[macro_export]\nmacro_rules! helper { ($($t:tt)*) => { $($t)* }; }\n",
            )],
        )
        .package("top", &[("src/lib.rs", "")]);
    // `user` depends on `provider`: the invocation's own trait is read.
    let table = read(&registry, &[("user", &["provider"]), ("provider", &[])]);
    assert!(!table.lacks("user", "t"));
    assert!(table.lacks("user", "u"));
    // Whoever defines the macro, the invocation's own trait tokens are read.
    let table = read(
        &registry,
        &[
            ("top", &["user", "sibling"]),
            ("user", &[]),
            ("sibling", &[]),
        ],
    );
    assert!(!table.lacks("top", "t"));
    assert!(table.lacks("top", "u"));
}

#[test]
fn a_procedural_macro_of_the_closure_poisons_its_names() {
    let registry = Registry::new();
    registry
        .package(
            "user",
            &[(
                "src/lib.rs",
                "#[macro_export]\nmacro_rules! gen { () => {}; }\ngen! {}\n",
            )],
        )
        .package(
            "derives",
            &[(
                "src/lib.rs",
                "extern crate proc_macro;\n#[proc_macro]\npub fn gen(_: proc_macro::TokenStream) \
                 -> proc_macro::TokenStream { todo!() }\n",
            )],
        )
        .package(
            "shadow",
            &[(
                "src/lib.rs",
                "#[proc_macro_derive(Debug)]\npub fn d(_: proc_macro::TokenStream) \
                 -> proc_macro::TokenStream { todo!() }\n",
            )],
        );
    // A name both a `macro_rules!` and a procedural macro of the closure define.
    let table = read(&registry, &[("user", &["derives"]), ("derives", &[])]);
    assert!(!table.lacks("user", "x"));
    // A procedural derive named like a standard one poisons its dependants' closures.
    let table = read(&registry, &[("user", &["shadow"]), ("shadow", &[])]);
    assert!(!table.lacks("user", "x"));
    // The procedural macro crate alone is known: it exports no trait.
    let table = read(&registry, &[("derives", &[])]);
    assert!(table.lacks("derives", "x"));
}

#[test]
fn unextracted_or_unregistered_sources_are_unknown() {
    let registry = Registry::new();
    registry.package("present", &[("src/lib.rs", "")]);
    let dir = registry
        .root
        .join("index.crates.io-0000000000000000")
        .join("present-1.0.0");
    let table = read(&registry, &[("present", &[])]);
    assert!(table.lacks("present", "x"));
    // Another registry's package of the same name and version is not crates.io's copy.
    let other = lock(&[("present", &[])]).replace(REGISTRY, "registry+https://example.org/index");
    let table = read_locked_trait_methods(&other, &registry.root);
    assert!(!table.lacks("present", "x"));
    // A deprecated `[replace]` entry redirects packages this reading does not follow.
    let replaced = format!("{}replace = \"present 1.0.0\"\n", lock(&[("present", &[])]));
    let table = read_locked_trait_methods(&replaced, &registry.root);
    assert!(!table.lacks("present", "x"));
    // A copy under another registry's index is not the one Cargo compiles for crates.io.
    let elsewhere = registry
        .root
        .join("example.org-0000000000000000")
        .join("elsewhere-1.0.0");
    fs::create_dir_all(elsewhere.join("src")).unwrap();
    fs::write(elsewhere.join("src/lib.rs"), "").unwrap();
    fs::write(
        elsewhere.join("Cargo.toml"),
        "[package]\nname = \"elsewhere\"\n",
    )
    .unwrap();
    fs::write(elsewhere.join(".cargo-ok"), "").unwrap();
    let table = read(&registry, &[("elsewhere", &[])]);
    assert!(!table.lacks("elsewhere", "x"));
    // Without `.cargo-ok` the extraction is incomplete.
    fs::remove_file(dir.join(".cargo-ok")).unwrap();
    let table = read(&registry, &[("present", &[])]);
    assert!(!table.lacks("present", "x"));
    assert_eq!(
        reason(&table, "present"),
        "sources not extracted under the registry"
    );
    // A git source is not read.
    let git = lock(&[("present", &[])]).replace(REGISTRY, "git+https://example.org/present");
    let table = read_locked_trait_methods(&git, &registry.root);
    assert!(!table.lacks("present", "x"));
    // A dependency the closure cannot resolve.
    let table = read(&registry, &[("present", &["missing"])]);
    assert!(!table.lacks("present", "x"));
}

#[cfg(unix)]
#[test]
fn a_symlink_is_refused() {
    let registry = Registry::new();
    registry.package("linked", &[("src/lib.rs", "")]);
    let dir = registry
        .root
        .join("index.crates.io-0000000000000000")
        .join("linked-1.0.0");
    std::os::unix::fs::symlink("/etc", dir.join("src/escape")).unwrap();
    let table = read(&registry, &[("linked", &[])]);
    assert!(!table.lacks("linked", "x"));
}

#[test]
fn the_digest_moves_with_any_source() {
    let registry = Registry::new();
    registry.package("dep", &[("src/lib.rs", "pub trait T { fn t(&self); }\n")]);
    let before = read(&registry, &[("dep", &[])]).digest;
    registry.package(
        "dep",
        &[("src/lib.rs", "pub trait T { fn t(&self); }\n// edited\n")],
    );
    let after = read(&registry, &[("dep", &[])]).digest;
    assert_ne!(before, after);
}

/// G173 review: dependencies written in uncommon but legal ways that declared a trait method
/// the first reading missed, each in isolation. Each closure must now answer UNKNOWN or name the
/// method; a benign control stays known.
mod review_holes {
    use super::*;

    fn case(user: &str, prov: &str, extra: &[(&str, &str)]) -> LockedTraitMethods {
        let registry = Registry::new();
        let mut files = vec![("src/lib.rs", user)];
        files.extend_from_slice(extra);
        registry
            .package("user", &files)
            .package("prov", &[("src/lib.rs", prov)]);
        read(&registry, &[("user", &["prov"]), ("prov", &[])])
    }

    const REBIND: &str = "#[macro_export]\nmacro_rules! m2 { ($t:tt) => { pub $t ExtA { fn name_a(self) -> u8; } impl<T> ExtA for T { fn name_a(self) -> u8 { 1 } } }; }\n";

    #[test]
    fn a_renamed_keyword_macro_binds_the_keyword() {
        let t = case("use prov::m2 as n;\nn!(trait);\n", REBIND, &[]);
        assert!(!t.lacks("user", "name_a"), "{:?}", t.unknown("user"));
        let t = case("prov::m2!(trait);\n", REBIND, &[]);
        assert!(!t.lacks("user", "name_a"), "{:?}", t.unknown("user"));
        // Nothing passes the keyword: the completed header never becomes a trait.
        let t = case("", REBIND, &[]);
        assert!(t.lacks("user", "name_a"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn a_default_in_a_completed_header_is_no_expression() {
        let prov = "#[macro_export]\nmacro_rules! m { ($t:tt) => { pub $t ExtA<X = u8> { fn name_a(self) -> u8; } impl<T> ExtA for T { fn name_a(self) -> u8 { 1 } } }; }\n";
        let t = case("prov::m!(trait);\n", prov, &[]);
        assert!(!t.lacks("user", "name_a"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn a_trait_body_from_a_metavariable_is_refused() {
        let prov = "#[macro_export]\nmacro_rules! body { ($b:tt $ib:tt) => { pub trait ExtB $b pub struct Z {} impl<T> ExtB for T $ib }; }\n";
        let t = case(
            "prov::body!({ fn name_b(self) -> u8; } { fn name_b(self) -> u8 { 2 } });\n",
            prov,
            &[],
        );
        assert!(!t.lacks("user", "name_b"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn a_macro_exported_from_a_statement_macro_is_read() {
        let prov = "macro_rules! define { ($($t:tt)*) => { $($t)* } }\npub fn _hidden() {\n    define! {\n        #[macro_export]\n        macro_rules! mk { () => { pub trait ExtC { fn name_c(self) -> u8; } impl<T> ExtC for T { fn name_c(self) -> u8 { 3 } } } }\n    }\n}\n";
        let t = case("prov::mk!();\n", prov, &[]);
        assert!(!t.lacks("user", "name_c"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn an_export_under_cfg_attr_is_a_procedural_macro() {
        let pm = "extern crate proc_macro;\nuse proc_macro::TokenStream;\n#[cfg_attr(all(), proc_macro)]\npub fn gen_d(_: TokenStream) -> TokenStream { TokenStream::new() }\n";
        let t = case("prov::gen_d!();\n", pm, &[]);
        assert!(!t.lacks("user", "name_d"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn a_procedural_macro_named_through_a_metavariable_is_unseen() {
        let pm = "extern crate proc_macro;\nuse proc_macro::TokenStream;\n#[proc_macro]\npub fn gen_e(_: TokenStream) -> TokenStream { TokenStream::new() }\n#[proc_macro_attribute]\npub fn gen_f(_: TokenStream, i: TokenStream) -> TokenStream { i }\n";
        let t = case(
            "use prov::gen_e;\nmacro_rules! call { ($m:ident) => { $m!{} }; }\ncall!(gen_e);\n",
            pm,
            &[],
        );
        assert!(!t.lacks("user", "name_e"), "{:?}", t.unknown("user"));
        let t = case(
            "macro_rules! attr { ($a:path) => { #[$a] pub struct Q; }; }\nattr!(prov::gen_f);\n",
            pm,
            &[],
        );
        assert!(!t.lacks("user", "name_f"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn an_include_or_path_of_a_file_not_read_is_refused() {
        let traits = [(
            "src/traits.in",
            "pub trait ExtG { fn name_g(self) -> u8; }\n",
        )];
        let t = case("include!(\"traits.in\");\n", "", &traits);
        assert!(!t.lacks("user", "name_g"), "{:?}", t.unknown("user"));
        let t = case("#[path = \"traits.in\"] mod t;\n", "", &traits);
        assert!(!t.lacks("user", "name_g"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn a_library_at_the_package_root_reaches_its_tests_directory() {
        let registry = Registry::new();
        registry.package(
            "user",
            &[
                (
                    "Cargo.toml",
                    "[package]\nname = \"user\"\nversion = \"1.0.0\"\nautotests = false\n[lib]\npath = \"lib.rs\"\n",
                ),
                ("lib.rs", "pub mod tests { pub mod x; }\n"),
                ("tests/x.rs", "pub trait ExtH { fn name_h(self) -> u8; }\n"),
            ],
        );
        let t = read(&registry, &[("user", &[])]);
        assert!(!t.lacks("user", "name_h"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn a_block_in_a_trait_header_is_no_body() {
        let user = "pub trait Other<const N: usize> {}\nmacro_rules! mk { () => { pub trait ExtI: Other<{ 1 }> { fn name_i(self) -> u8; } } }\nmk!();\n";
        let t = case(user, "", &[]);
        assert!(!t.lacks("user", "name_i"), "{:?}", t.unknown("user"));
    }

    #[test]
    fn a_feature_on_a_target_dependency_stays_possible() {
        let registry = Registry::new();
        registry
            .package(
                "user",
                &[
                    (
                        "Cargo.toml",
                        "[package]\nname = \"user\"\nversion = \"1.0.0\"\n\n[features]\nwin = [\"dep:winapi\"]\n\n[target.\"cfg(windows)\".dependencies.winapi]\nversion = \"0.3\"\noptional = true\n",
                    ),
                    (
                        "src/lib.rs",
                        "#[cfg(feature = \"win\")]\npub trait ExtJ { fn name_j(self) -> u8; }\n",
                    ),
                ],
            )
            .package("winapi", &[("src/lib.rs", "")]);
        let t = read(&registry, &[("user", &["winapi"]), ("winapi", &[])]);
        assert!(!t.lacks("user", "name_j"), "{:?}", t.unknown("user"));
    }
}
