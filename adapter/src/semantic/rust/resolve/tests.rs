use super::*;

fn sources(files: &[(&str, &str)]) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

fn one_crate(root: &str) -> Vec<CrateInput> {
    vec![CrateInput {
        root: root.into(),
        externs: BTreeMap::new(),
    }]
}

/// `callee spelling -> outcome` for every path call in `path`, where a resolved outcome prints
/// as `file:line:name` of the target.
fn outcomes(results: &[PathCallResolution], path: &str) -> Vec<(String, String)> {
    results
        .iter()
        .filter(|r| r.path == path)
        .map(|r| {
            let outcome = match &r.outcome {
                PathCallOutcome::Resolved(t) => format!("{}:{}:{}", t.path, t.line, t.name),
                PathCallOutcome::Unresolved(reason) => format!("unresolved:{reason}"),
            };
            (r.callee.clone(), outcome)
        })
        .collect()
}

fn resolve(files: &[(&str, &str)], root: &str) -> Vec<PathCallResolution> {
    resolve_path_calls(&one_crate(root), &sources(files))
}

#[test]
fn items_imports_renames_groups_and_relative_paths() {
    let lib = "mod a;\nmod b {\n    pub fn g() {}\n}\nuse a::{f, h as renamed};\npub fn top() {\n    f();\n    renamed();\n    b::g();\n    crate::a::f();\n    self::b::g();\n    local();\n}\nfn local() {}\n";
    let a = "pub fn f() {}\npub fn h() {\n    super::local();\n}\n";
    let results = resolve(&[("src/lib.rs", lib), ("src/a.rs", a)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("f".into(), "src/a.rs:1:f".into()),
            ("renamed".into(), "src/a.rs:2:h".into()),
            ("b::g".into(), "src/lib.rs:3:g".into()),
            ("crate::a::f".into(), "src/a.rs:1:f".into()),
            ("self::b::g".into(), "src/lib.rs:3:g".into()),
            ("local".into(), "src/lib.rs:14:local".into()),
        ]
    );
    assert_eq!(
        outcomes(&results, "src/a.rs"),
        [("super::local".into(), "src/lib.rs:14:local".into())]
    );
}

#[test]
fn module_files_follow_mod_rs_and_non_mod_rs_rules() {
    // `x.rs` declares children in `x/`; `y/mod.rs` next to itself; `#[path]` is relative to the
    // declaring file's directory.
    let lib = "mod x;\nmod y;\n#[path = \"other/z_impl.rs\"]\nmod z;\npub fn top() {\n    x::inner::f();\n    y::leaf::g();\n    z::h();\n    x::q::from_p();\n}\n";
    let results = resolve(
        &[
            ("src/lib.rs", lib),
            (
                "src/x.rs",
                "pub mod inner;\n#[path = \"p.rs\"]\npub mod q;\n",
            ),
            ("src/p.rs", "pub fn from_p() {}\n"),
            ("src/x/p.rs", "pub fn from_x_p() {}\n"),
            ("src/x/inner.rs", "pub fn f() {}\n"),
            ("src/y/mod.rs", "pub mod leaf;\n"),
            ("src/y/leaf.rs", "pub fn g() {}\n"),
            ("src/other/z_impl.rs", "pub fn h() {}\n"),
        ],
        "src/lib.rs",
    );
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("x::inner::f".into(), "src/x/inner.rs:1:f".into()),
            ("y::leaf::g".into(), "src/y/leaf.rs:1:g".into()),
            ("z::h".into(), "src/other/z_impl.rs:1:h".into()),
            // In the non-mod-rs `x.rs`, `#[path]` is relative to `src/`, not `src/x/`.
            ("x::q::from_p".into(), "src/p.rs:1:from_p".into()),
        ]
    );
}

#[test]
fn a_test_module_glob_sees_the_parents_private_items_and_imports() {
    let lib = "mod helpers;\nuse helpers::build;\nfn private() {}\n#[cfg(test)]\nmod tests {\n    use super::*;\n    fn t() {\n        private();\n        build();\n    }\n}\n";
    let results = resolve(
        &[
            ("src/lib.rs", lib),
            ("src/helpers.rs", "pub fn build() {}\n"),
        ],
        "src/lib.rs",
    );
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("private".into(), "src/lib.rs:3:private".into()),
            ("build".into(), "src/helpers.rs:1:build".into()),
        ]
    );
}

#[test]
fn a_sibling_glob_never_imports_private_items() {
    let lib = "mod a {\n    fn hidden() {}\n    pub fn shown() {}\n}\nmod b {\n    use super::a::*;\n    fn t() {\n        shown();\n        hidden();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("shown".into(), "src/lib.rs:3:shown".into()),
            ("hidden".into(), "unresolved:external".into()),
        ]
    );
}

#[test]
fn re_exports_resolve_to_a_fixed_point_regardless_of_order() {
    // `c` re-exports from `b`, which re-exports from `a`, declared in reverse order.
    let lib = "mod c {\n    pub use super::b::f;\n}\nmod b {\n    pub use super::a::*;\n}\nmod a {\n    pub fn f() {}\n}\nfn t() {\n    c::f();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [("c::f".into(), "src/lib.rs:8:f".into())]
    );
}

#[test]
fn an_item_shadows_a_glob_and_conflicting_globs_are_ambiguous() {
    let lib = "mod a {\n    pub fn f() {}\n    pub fn g() {}\n}\nmod b {\n    pub fn g() {}\n}\nmod user {\n    use super::a::*;\n    use super::b::*;\n    fn f() {}\n    fn t() {\n        f();\n        g();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("f".into(), "src/lib.rs:11:f".into()),
            ("g".into(), "unresolved:ambiguous".into()),
        ]
    );
}

#[test]
fn block_items_shadow_module_items_and_locals_shadow_both() {
    let lib = "fn helper() {}\nfn t(param: fn()) {\n    fn helper() {}\n    helper();\n    let local = helper;\n    local();\n    param();\n}\nfn u() {\n    helper();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("helper".into(), "src/lib.rs:3:helper".into()),
            ("local".into(), "unresolved:local-binding".into()),
            ("param".into(), "unresolved:local-binding".into()),
            ("helper".into(), "src/lib.rs:1:helper".into()),
        ]
    );
}

#[test]
fn associated_functions_resolve_through_inherent_then_trait_impls() {
    let lib = "pub struct S;\nimpl S {\n    pub fn new() -> S {\n        Self::make()\n    }\n    fn make() -> S {\n        S\n    }\n}\nimpl Default for S {\n    fn default() -> S {\n        S::new()\n    }\n}\npub trait Make {\n    fn new() -> S;\n}\nimpl Make for S {\n    fn new() -> S {\n        S\n    }\n}\ntype Alias = S;\nfn t<T: Default>() {\n    S::new();\n    S::default();\n    Alias::new();\n    T::default();\n    S::missing();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("Self::make".into(), "src/lib.rs:6:make".into()),
            ("S::new".into(), "src/lib.rs:3:new".into()),
            ("S::new".into(), "src/lib.rs:3:new".into()),
            ("S::default".into(), "src/lib.rs:11:default".into()),
            ("Alias::new".into(), "src/lib.rs:3:new".into()),
            ("T::default".into(), "unresolved:generic-parameter".into()),
            (
                "S::missing".into(),
                "unresolved:associated-not-found".into()
            ),
        ]
    );
}

#[test]
fn constructors_traits_and_cfg_alternates_are_never_claimed_as_functions() {
    let lib = "pub struct Wrap(u8);\npub enum E {\n    V(u8),\n}\npub trait Tr {\n    fn call();\n    fn provided() {\n        Self::call();\n    }\n}\n#[cfg(unix)]\nfn alt() {}\n#[cfg(not(unix))]\nfn alt() {}\nfn t() {\n    Wrap(1);\n    E::V(1);\n    Tr::call();\n    alt();\n    Some(1);\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("Self::call".into(), "unresolved:trait-method".into()),
            ("Wrap".into(), "unresolved:constructor".into()),
            ("E::V".into(), "unresolved:constructor".into()),
            ("Tr::call".into(), "unresolved:trait-method".into()),
            ("alt".into(), "unresolved:ambiguous".into()),
            ("Some".into(), "unresolved:external".into()),
        ]
    );
}

#[test]
fn an_open_scope_never_falls_through_and_never_trusts_its_globs() {
    // `std::collections::*` could provide any name, so a miss here is not a miss, and a name a
    // workspace glob provides might be shadowed by the external one.
    let lib = "mod a {\n    pub fn f() {}\n}\nmod user {\n    use std::collections::*;\n    use super::a::*;\n    fn own() {}\n    fn t() {\n        own();\n        f();\n        unknown();\n    }\n}\nmod macros {\n    make_items!();\n    fn t() {\n        anything();\n    }\n}\nmod importer {\n    use super::macros::*;\n    fn t() {\n        generated();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("own".into(), "src/lib.rs:7:own".into()),
            ("f".into(), "unresolved:open-scope".into()),
            ("unknown".into(), "unresolved:open-scope".into()),
            ("anything".into(), "unresolved:open-scope".into()),
            // A glob of an open module may import names this pass cannot see.
            ("generated".into(), "unresolved:open-scope".into()),
        ]
    );
}

#[test]
fn workspace_crates_resolve_through_the_extern_prelude_and_only_see_pub_items() {
    let crates = vec![
        CrateInput {
            root: "core/src/lib.rs".into(),
            externs: BTreeMap::new(),
        },
        CrateInput {
            root: "app/src/main.rs".into(),
            externs: BTreeMap::from([("atlas_core".to_owned(), 0)]),
        },
    ];
    let core = "pub mod m {\n    pub fn shared() {}\n    pub(crate) fn internal() {}\n}\npub use m::shared as reexported;\n";
    let app = "use atlas_core::m;\nfn main() {\n    m::shared();\n    atlas_core::reexported();\n    m::internal();\n    serde_json::from_str();\n}\n";
    let results = resolve_path_calls(
        &crates,
        &sources(&[("core/src/lib.rs", core), ("app/src/main.rs", app)]),
    );
    assert_eq!(
        outcomes(&results, "app/src/main.rs"),
        [
            ("m::shared".into(), "core/src/lib.rs:2:shared".into()),
            (
                "atlas_core::reexported".into(),
                "core/src/lib.rs:2:shared".into()
            ),
            ("m::internal".into(), "unresolved:not-found".into()),
            ("serde_json::from_str".into(), "unresolved:external".into()),
        ]
    );
}

#[test]
fn method_calls_closures_and_initializers_are_outside_the_pass() {
    let lib = "fn f() {}\nconst C: () = f();\nfn t(x: X) {\n    x.method();\n    let c = || f();\n    f();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [("f".into(), "src/lib.rs:1:f".into())]
    );
    assert_eq!((results[0].line, results[0].column), (6, 4));
}

#[test]
fn a_missing_module_file_is_open_not_empty() {
    let lib = "mod gone;\nuse gone::f;\nfn t() {\n    gone::g();\n    f();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("gone::g".into(), "unresolved:open-scope".into()),
            ("f".into(), "unresolved:unresolved-import".into()),
        ]
    );
}

#[test]
fn an_unresolvable_import_binds_its_name_and_never_falls_through() {
    // A block-level import naming something this pass cannot find still binds `element`: the
    // outer glob's `element` must not be picked in its place. `super::super` climbs twice.
    let lib = "mod a {\n    pub fn element() {}\n}\nmod outer {\n    pub fn element() {}\n    pub mod inner {\n        use crate::a::*;\n        fn t() {\n            use super::super::outer::element;\n            element();\n        }\n        fn u() {\n            use crate::nowhere::element;\n            element();\n        }\n        fn v() {\n            element();\n        }\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("element".into(), "src/lib.rs:5:element".into()),
            ("element".into(), "unresolved:unresolved-import".into()),
            ("element".into(), "src/lib.rs:2:element".into()),
        ]
    );
}

#[test]
fn a_file_the_recursion_pre_scan_refuses_is_open_and_never_parsed() {
    // 300 levels of bracket nesting overflowed a reduced stack in the syntactic extractor's own
    // history; the resolver refuses it exactly as the extractor does.
    let deep = format!(
        "pub fn g() {{ let _ = {}0{}; }}\n",
        "(".repeat(300),
        ")".repeat(300)
    );
    let lib = "mod deep;\nfn t() {\n    deep::g();\n}\n";
    let results = resolve(&[("src/lib.rs", lib), ("src/deep.rs", &deep)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [("deep::g".into(), "unresolved:open-scope".into())]
    );
    assert!(outcomes(&results, "src/deep.rs").is_empty());
}
