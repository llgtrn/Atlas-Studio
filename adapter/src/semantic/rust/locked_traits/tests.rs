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

/// G177: the per-item reading, over fixture packages with their own manifests.
mod per_item {
    use super::*;

    /// A manifest: `edition`, each `(key, package)` normal dependency, a procedural macro crate.
    fn manifest(name: &str, edition: &str, deps: &[(&str, &str)], proc_macro: bool) -> String {
        let mut text = format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\n");
        if !edition.is_empty() {
            text.push_str(&format!("edition = \"{edition}\"\n"));
        }
        if proc_macro {
            text.push_str("\n[lib]\nproc-macro = true\n");
        }
        for (key, package) in deps {
            text.push_str(&format!(
                "\n[dependencies.{key}]\nversion = \"1\"\npackage = \"{package}\"\n"
            ));
        }
        text
    }

    fn item(table: &LockedTraitMethods, package: &str, path: &str) -> Option<Vec<String>> {
        let path: Vec<String> = path
            .split("::")
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        table
            .item_methods(package, &path)
            .map(|names| names.into_iter().collect())
    }

    fn names(list: &[&str]) -> Option<Vec<String>> {
        Some(list.iter().map(|n| n.to_string()).collect())
    }

    /// A serde-like facade: a build-script `include!` beside a local zero-parameter `crate_root!`
    /// re-exporting the core package's traits, and a derive crate's macros of the same names.
    const FACADE: &str = "#![cfg_attr(not(feature = \"std\"), no_std)]\n\
        macro_rules! crate_root {\n    () => {\n        mod lib {}\n        \
        pub use core_pkg::{ser, Serialize, Serializer};\n        \
        macro_rules! tri { ($e:expr) => { $e }; }\n        \
        include!(concat!(env!(\"OUT_DIR\"), \"/private.rs\"));\n    };\n}\n\
        crate_root!();\n\
        #[cfg(feature = \"derive_pkg\")]\npub use derive_pkg::{Deserialize, Serialize};\n\
        pub use core_pkg::*;\npub struct Plain;\npub enum Kind { A }\npub mod module {}\n\
        pub type Alias = Plain;\npub fn only_a_function() {}\n\
        pub use core_pkg::ser::Serialize as Renamed;\n";

    /// The core package: its root re-exports through a `#[macro_use] mod` defined elsewhere.
    const CORE: &[(&str, &str)] = &[
        (
            "src/lib.rs",
            "#[macro_use]\nmod crate_root;\ncrate_root!();\npub use crate::ser::*;\n",
        ),
        (
            "src/crate_root.rs",
            "macro_rules! crate_root {\n    () => {\n        pub mod ser;\n        \
             pub use crate::ser::{Serialize, Serializer};\n        \
             include!(concat!(env!(\"OUT_DIR\"), \"/private.rs\"));\n    };\n}\n",
        ),
        (
            "src/ser/mod.rs",
            "pub trait Sup { fn sup(self); }\n\
             #[cfg_attr(not(no_diagnostic_namespace), diagnostic::on_unimplemented(message = \"x\"))]\n\
             pub trait Serialize: Sup {\n    fn serialize(&self);\n    #[doc(hidden)]\n    \
             fn r#type(&self) {}\n    type Out;\n    const C: u8;\n}\n\
             pub struct Serializer;\npub trait Globbed { fn globbed(self); }\n\
             pub trait WithMacro { fn a(&self); more!(); }\n\
             pub trait WithAttribute { #[generate] fn a(&self); }\n",
        ),
    ];

    fn facade(registry: &Registry, facade: &str) -> LockedTraitMethods {
        let core_manifest = manifest("core_pkg", "2021", &[], false);
        let mut core = CORE.to_vec();
        core.push(("Cargo.toml", &core_manifest));
        let derives = manifest("derive_pkg", "2021", &[], true);
        let facade_manifest = manifest(
            "facade",
            "2021",
            &[("core_pkg", "core_pkg"), ("derive_pkg", "derive_pkg")],
            false,
        );
        registry
            .package(
                "facade",
                &[("Cargo.toml", &facade_manifest), ("src/lib.rs", facade)],
            )
            .package("core_pkg", &core)
            .package(
                "derive_pkg",
                &[("Cargo.toml", &derives), ("src/lib.rs", PROC_MACROS)],
            );
        read(
            registry,
            &[
                ("facade", &["core_pkg", "derive_pkg"]),
                ("core_pkg", &[]),
                ("derive_pkg", &[]),
            ],
        )
    }

    #[test]
    fn a_named_re_export_reaches_the_trait_whatever_an_include_adds_beside_it() {
        let registry = Registry::new();
        let table = facade(&registry, FACADE);
        // The closures hold a build-script include: UNKNOWN.
        assert!(!table.lacks("facade", "look"));
        assert!(!table.lacks("core_pkg", "look"));
        // `facade::Serialize` is `core_pkg::ser::Serialize`, whatever the include adds and though
        // the derive crate's `Serialize` is re-exported too (macros only). Its own methods only.
        assert_eq!(
            item(&table, "facade", "Serialize"),
            names(&["serialize", "type"])
        );
        assert_eq!(
            item(&table, "facade", "ser::Serialize"),
            names(&["serialize", "type"])
        );
        assert_eq!(
            item(&table, "facade", "Renamed"),
            names(&["serialize", "type"])
        );
        assert_eq!(
            item(&table, "core_pkg", "Serialize"),
            names(&["serialize", "type"])
        );
        // A supertrait answers for itself; it is not in scope through its subtrait.
        assert_eq!(item(&table, "core_pkg", "ser::Sup"), names(&["sup"]));
        assert!(table.item_lacks("facade", &["Serialize".into()], "sup"));
        assert!(table.item_lacks("facade", &["Serialize".into()], "look"));
        assert!(!table.item_lacks("facade", &["Serialize".into()], "serialize"));
        assert!(!table.item_lacks("facade", &["Serialize".into()], "r#type"));
        // Items that are no trait: none.
        for path in [
            "Serializer",
            "Plain",
            "Kind",
            "module",
            "Alias",
            "ser",
            "ser::Serializer",
            "Kind::A",
        ] {
            assert_eq!(item(&table, "facade", path), names(&[]), "{path}");
        }
        assert_eq!(item(&table, "core_pkg", ""), names(&[]));
        // Only a glob binds `Globbed` at either root; only the value namespace holds
        // `only_a_function` (an unseen `trait only_a_function` would compile beside it); a
        // trait body holding a macro or a procedural attribute; a derive crate's name alone.
        for (package, path) in [
            ("facade", "Globbed"),
            ("core_pkg", "Globbed"),
            ("facade", "only_a_function"),
            ("facade", "Deserialize"),
            ("facade", "Nowhere"),
            ("core_pkg", "ser::WithMacro"),
            ("core_pkg", "ser::WithAttribute"),
            ("facade", "lib::Anything"),
        ] {
            assert_eq!(item(&table, package, path), None, "{package}::{path}");
        }
        assert_eq!(
            item(&table, "core_pkg", "ser::Globbed"),
            names(&["globbed"])
        );
        // An UNKNOWN item falls back to the closure.
        assert!(!table.item_lacks("facade", &["Globbed".into()], "look"));
        // A package not locked, or not extracted, is UNKNOWN.
        assert_eq!(item(&table, "absent", "Serialize"), None);
        let unextracted = read_locked_trait_methods(
            &lock(&[
                ("facade", &["core_pkg", "derive_pkg"]),
                ("core_pkg", &[]),
                ("derive_pkg", &[]),
            ]),
            &registry.root.join("elsewhere"),
        );
        assert_eq!(item(&unextracted, "facade", "Serialize"), None);
        assert!(!unextracted.item_lacks("facade", &["Serialize".into()], "look"));
    }

    #[test]
    fn only_a_single_empty_rule_invoked_empty_is_expanded() {
        let registry = Registry::new();
        let table = facade(
            &registry,
            "macro_rules! named { ($n:ident) => { pub trait $n { fn a(&self); } }; }\n\
             named!(Param);\n\
             macro_rules! two { () => { pub trait Two { fn two(&self); } }; ($x:tt) => {}; }\n\
             two!();\n\
             macro_rules! any { ($($x:tt)*) => { pub trait Any { fn any(&self); } }; }\n\
             any!();\n\
             macro_rules! one { () => { pub trait One { fn one(&self); } }; }\n\
             one!();\n\
             macro_rules! nested { () => { one_more!(); }; }\n\
             macro_rules! one_more { () => { pub trait Nested { fn nested(&self); } }; }\n\
             nested!();\n",
        );
        assert_eq!(item(&table, "facade", "Param"), None);
        assert_eq!(item(&table, "facade", "Two"), None);
        assert_eq!(item(&table, "facade", "Any"), None);
        assert_eq!(item(&table, "facade", "One"), names(&["one"]));
        assert_eq!(item(&table, "facade", "Nested"), names(&["nested"]));
    }

    #[test]
    fn a_macro_use_definition_is_in_scope_only_after_its_module() {
        let registry = Registry::new();
        let dep = manifest("dep", "2021", &[], false);
        registry.package(
            "dep",
            &[
                ("Cargo.toml", &dep),
                (
                    "src/lib.rs",
                    "early!();\nmod plain;\nplain_only!();\n\
                     macro_rules! shadowed { () => { pub trait Shadowed { fn local(&self); } }; }\n\
                     #[macro_use]\nmod defs;\nshadowed!();\nlate!();\n\
                     mod child { in_child!(); }\npub use child::Child;\n",
                ),
                (
                    "src/defs.rs",
                    "macro_rules! early { () => { pub trait Early { fn e(&self); } }; }\n\
                     macro_rules! shadowed { () => { pub struct Shadowed; }; }\n\
                     macro_rules! late { () => { pub trait Late { fn late(&self); } }; }\n\
                     macro_rules! in_child { () => { pub trait Child { fn child(&self); } }; }\n",
                ),
                (
                    "src/plain.rs",
                    "macro_rules! plain_only { () => { pub trait Plain { fn p(&self); } }; }\n",
                ),
            ],
        );
        let table = read(&registry, &[("dep", &[])]);
        // Invoked before its `#[macro_use]` module: not this definition.
        assert_eq!(item(&table, "dep", "Early"), None);
        // The later definition shadows the local one.
        assert_eq!(item(&table, "dep", "Shadowed"), names(&[]));
        assert_eq!(item(&table, "dep", "Late"), names(&["late"]));
        // A child module sees the textual scope before it.
        assert_eq!(item(&table, "dep", "Child"), names(&["child"]));
        // Without `#[macro_use]` a module's definitions end with it.
        assert_eq!(item(&table, "dep", "Plain"), None);
    }

    #[test]
    fn cfg_alternatives_answer_their_union_or_unknown() {
        let registry = Registry::new();
        let table = facade(
            &registry,
            "#[cfg(docsrs)]\n#[macro_use]\n#[path = \"core/crate_root.rs\"]\nmod crate_root;\n\
             #[cfg(not(docsrs))]\nmacro_rules! crate_root { () => { pub use core_pkg::Serialize; }; }\n\
             crate_root!();\n\
             #[cfg(unix)]\npub use core_pkg::ser::Sup as OnlyUnix;\n\
             #[cfg(any(feature = \"gone\", unix))]\npub struct Both;\n\
             #[cfg(not(any(feature = \"gone\", unix)))]\npub trait Both { fn both(self); }\n",
        );
        registry.package(
            "facade",
            &[
                (
                    "src/core/crate_root.rs",
                    "macro_rules! crate_root { () => {\n\
                     #[cfg_attr(all(docsrs, if_docsrs), path = \"core/ser.rs\")]\npub mod ser;\n\
                     pub use crate::ser::Serialize;\n}; }\n",
                ),
                (
                    "src/core/ser.rs",
                    "pub trait Serialize { fn serialize(&self); fn documented(&self); }\n",
                ),
            ],
        );
        let table = {
            let _ = table;
            read(
                &registry,
                &[
                    ("facade", &["core_pkg", "derive_pkg"]),
                    ("core_pkg", &[]),
                    ("derive_pkg", &[]),
                ],
            )
        };
        // Under `docsrs` the facade's own trait (its `ser` file exists only under `if_docsrs`:
        // without it that build does not compile), else the core package's: the union.
        assert_eq!(
            item(&table, "facade", "Serialize"),
            names(&["documented", "serialize", "type"])
        );
        // Bound only under `unix`: elsewhere a glob or an unseen item may bind it.
        assert_eq!(item(&table, "facade", "OnlyUnix"), None);
        // One alternative a trait, the other not: the trait's methods.
        assert_eq!(item(&table, "facade", "Both"), names(&["both"]));
    }

    #[test]
    fn a_renamed_dependency_is_followed_by_its_key() {
        let registry = Registry::new();
        let facade_manifest = manifest("facade", "2021", &[("renamed", "core_pkg")], false);
        let core_manifest = manifest("core_pkg", "2021", &[], false);
        let mut core = CORE.to_vec();
        core.push(("Cargo.toml", &core_manifest));
        registry
            .package(
                "facade",
                &[
                    ("Cargo.toml", &facade_manifest),
                    (
                        "src/lib.rs",
                        "pub use renamed::ser::Serialize;\npub use core_pkg::ser::Sup;\n\
                         pub use ::renamed::ser::Sup as Rooted;\n",
                    ),
                ],
            )
            .package("core_pkg", &core);
        let table = read(&registry, &[("facade", &["core_pkg"]), ("core_pkg", &[])]);
        assert_eq!(
            item(&table, "facade", "Serialize"),
            names(&["serialize", "type"])
        );
        assert_eq!(item(&table, "facade", "Rooted"), names(&["sup"]));
        // `core_pkg` is not an extern name of the facade: its dependency is renamed.
        assert_eq!(item(&table, "facade", "Sup"), None);
        // A 2015 package's paths start at its crate root, where `extern crate` binds the crate.
        let registry = Registry::new();
        let old = manifest("facade", "", &[("renamed", "core_pkg")], false);
        let mut core = CORE.to_vec();
        core.push(("Cargo.toml", &core_manifest));
        registry
            .package(
                "facade",
                &[
                    ("Cargo.toml", &old),
                    (
                        "src/lib.rs",
                        "extern crate renamed as core_pkg;\npub use core_pkg::ser::Serialize;\n\
                         mod inner { pub use core_pkg::ser::Sup; pub use helpers::Local; }\n\
                         pub use inner::Sup;\npub use inner::Local as Relative;\n\
                         pub mod helpers { pub trait Local { fn local(&self); } }\n",
                    ),
                ],
            )
            .package("core_pkg", &core);
        let table = read(&registry, &[("facade", &["core_pkg"]), ("core_pkg", &[])]);
        assert_eq!(
            item(&table, "facade", "Serialize"),
            names(&["serialize", "type"])
        );
        assert_eq!(item(&table, "facade", "Sup"), names(&["sup"]));
        // A plain path in a 2015 module starts at the crate root, not the extern prelude.
        assert_eq!(item(&table, "facade", "Relative"), names(&["local"]));
    }

    /// G177 review: each hole the independent review reproduced on rustc, in isolation.
    const NAMED: &str = "pub trait T { fn name(self); }\nimpl<X> T for X { fn name(self) {} }\n";

    #[test]
    fn a_features_atoms_are_its_own_packages() {
        // `a` with `x` and `b` without is a build: `a::T` is `b`'s by-value trait.
        let registry = Registry::new();
        let features = |name: &str, deps: &str| {
            format!(
                "[package]\nname = \"{name}\"\nversion = \"1.0.0\"\nedition = \"2021\"\n\
                 [features]\nx = []\n{deps}"
            )
        };
        let a = features("a", "[dependencies.b]\nversion = \"1\"\n");
        let b = features("b", "");
        let b_lib = format!(
            "#[cfg(feature = \"x\")]\npub trait T {{}}\n#[cfg(not(feature = \"x\"))]\n{NAMED}"
        );
        registry
            .package(
                "a",
                &[
                    ("Cargo.toml", &a),
                    (
                        "src/lib.rs",
                        "#[cfg(feature = \"x\")]\npub use b::T;\n\
                         #[cfg(not(feature = \"x\"))]\npub trait T {}\n",
                    ),
                ],
            )
            .package("b", &[("Cargo.toml", &b), ("src/lib.rs", &b_lib)]);
        let table = read(&registry, &[("a", &["b"]), ("b", &[])]);
        assert_eq!(item(&table, "a", "T"), names(&["name"]));
        assert!(!table.item_lacks("a", &["T".into()], "name"));
    }

    #[test]
    fn a_macro_use_module_this_reading_does_not_walk_may_shadow_a_macro() {
        let registry = Registry::new();
        let dep = manifest("dep", "2021", &[], false);
        let shadow = format!(
            "macro_rules! m {{ () => {{ {} }}; }}\n",
            NAMED.replace('\n', " ")
        );
        registry.package(
            "dep",
            &[
                ("Cargo.toml", &dep),
                (
                    "src/lib.rs",
                    "macro_rules! m { () => { pub trait T {} }; }\n\
                     #[cfg_attr(unix, path = \"x_unix.rs\")]\n\
                     #[cfg_attr(not(unix), path = \"x_other.rs\")]\n#[macro_use]\nmod x;\nm! {}\n",
                ),
                ("src/x_unix.rs", &shadow),
                ("src/x_other.rs", &shadow),
            ],
        );
        let table = read(&registry, &[("dep", &[])]);
        assert_eq!(item(&table, "dep", "T"), None);
        // Without `#[macro_use]` the unwalked module's macros end with it.
        let registry = Registry::new();
        registry.package(
            "dep",
            &[
                ("Cargo.toml", &dep),
                (
                    "src/lib.rs",
                    "macro_rules! m { () => { pub trait T {} }; }\n\
                     #[cfg_attr(unix, path = \"x_unix.rs\")]\n\
                     #[cfg_attr(not(unix), path = \"x_other.rs\")]\nmod x;\nm! {}\n",
                ),
                ("src/x_unix.rs", &shadow),
                ("src/x_other.rs", &shadow),
            ],
        );
        let table = read(&registry, &[("dep", &[])]);
        assert_eq!(item(&table, "dep", "T"), names(&[]));
    }

    #[test]
    fn a_crate_root_extern_crate_decides_its_name_in_every_module() {
        let registry = Registry::new();
        let a = manifest("a", "2021", &[("b", "b"), ("c", "c")], false);
        registry
            .package(
                "a",
                &[
                    ("Cargo.toml", &a),
                    (
                        "src/lib.rs",
                        "extern crate c as b;\npub mod sub { pub use b::T; }\n\
                         pub mod rooted { pub use ::b::T; }\n",
                    ),
                ],
            )
            .package("b", &[("src/lib.rs", "pub trait T {}\n")])
            .package("c", &[("src/lib.rs", NAMED)]);
        let lock = [("a", &["b", "c"][..]), ("b", &[]), ("c", &[])];
        let table = read(&registry, &lock);
        assert_eq!(item(&table, "a", "sub::T"), names(&["name"]));
        assert_eq!(item(&table, "a", "rooted::T"), names(&["name"]));
        // `extern crate self as b` names the crate itself.
        registry.package(
            "a",
            &[(
                "src/lib.rs",
                &format!("extern crate self as b;\n{NAMED}pub mod sub {{ pub use b::T as U; }}\n"),
            )],
        );
        let table = read(&registry, &lock);
        assert_eq!(item(&table, "a", "sub::U"), names(&["name"]));
        // Under `cfg`, both crates may be meant.
        registry.package(
            "a",
            &[(
                "src/lib.rs",
                "#[cfg(unix)]\nextern crate c as b;\npub mod sub { pub use b::T; }\n",
            )],
        );
        let table = read(&registry, &lock);
        assert_eq!(item(&table, "a", "sub::T"), names(&["name"]));
    }

    #[test]
    fn an_unrenamed_dependency_is_named_after_its_library() {
        let registry = Registry::new();
        let p = manifest(
            "p",
            "2021",
            &[("alpha", "alpha"), ("gamma", "gamma")],
            false,
        );
        let lib = |name: &str, lib: &str| {
            format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\n[lib]\nname = \"{lib}\"\n")
        };
        registry
            .package(
                "p",
                &[
                    ("Cargo.toml", &p),
                    ("src/lib.rs", "pub use alpha::T;\npub use beta::T as B;\n"),
                ],
            )
            .package(
                "alpha",
                &[
                    ("Cargo.toml", &lib("alpha", "beta")),
                    ("src/lib.rs", "pub trait T {}\n"),
                ],
            )
            .package(
                "gamma",
                &[
                    ("Cargo.toml", &lib("gamma", "alpha")),
                    ("src/lib.rs", NAMED),
                ],
            );
        let table = read(
            &registry,
            &[("p", &["alpha", "gamma"]), ("alpha", &[]), ("gamma", &[])],
        );
        // Cargo passes `--extern alpha=<gamma>` and `--extern beta=<alpha>`: neither key names it.
        assert_eq!(item(&table, "p", "T"), None);
        assert_eq!(item(&table, "p", "B"), None);
    }

    #[test]
    fn the_digest_moves_with_the_per_item_reading() {
        let registry = Registry::new();
        let before = facade(&registry, FACADE).digest;
        let unchanged = facade(&registry, FACADE).digest;
        assert_eq!(before, unchanged);
        let after = facade(&registry, &FACADE.replace("Renamed", "Other")).digest;
        assert_ne!(before, after);
    }
}
