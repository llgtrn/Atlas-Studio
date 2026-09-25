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
                PathCallOutcome::External(path) if path.is_empty() => "unresolved:external".into(),
                PathCallOutcome::External(path) => format!("external:{path}"),
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
fn method_calls_async_blocks_and_initializers_are_outside_the_pass() {
    // G133: a closure body is inside the pass (its own executable region); a closure parameter
    // shadowing a function name stays a local; `async` blocks and initializers stay outside.
    let lib = "fn f() {}\nconst C: () = f();\nfn t(x: X) {\n    x.method();\n    let c = || f();\n    let d = |f: fn()| f();\n    let e = async { f() };\n    f();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("f".into(), "src/lib.rs:1:f".into()),
            ("f".into(), "unresolved:local-binding".into()),
            ("f".into(), "src/lib.rs:1:f".into()),
        ]
    );
    let positions: Vec<(usize, usize)> = results.iter().map(|r| (r.line, r.column)).collect();
    assert_eq!(positions, [(5, 15), (6, 22), (8, 4)]);
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

#[test]
fn standard_library_paths_are_spelled_canonically_through_imports() {
    // `fs::write` through `use std::fs`, a renamed type's associated function, a `self` import,
    // an absolute `::std` path and a direct import all name their std path; a workspace module
    // named `fs` shadows nothing it should not, and a registry crate's path stays unknown.
    let lib = "use std::fs;\nuse std::fs::File as Handle;\nuse std::env::{self};\nuse std::fs::read_to_string;\nmod local {\n    pub mod fs {\n        pub fn write() {}\n    }\n    fn t() {\n        fs::write();\n    }\n}\nfn t() {\n    fs::write();\n    Handle::open();\n    env::var();\n    ::std::fs::remove_dir_all();\n    read_to_string();\n    serde_json::from_str();\n    std::io::stdout();\n}\nmod globbed {\n    use std::fs::*;\n    fn t() {\n        write();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("fs::write".into(), "src/lib.rs:7:write".into()),
            ("fs::write".into(), "external:std::fs::write".into()),
            ("Handle::open".into(), "external:std::fs::File::open".into()),
            ("env::var".into(), "external:std::env::var".into()),
            (
                "std::fs::remove_dir_all".into(),
                "external:std::fs::remove_dir_all".into()
            ),
            (
                "read_to_string".into(),
                "external:std::fs::read_to_string".into()
            ),
            ("serde_json::from_str".into(), "unresolved:external".into()),
            ("std::io::stdout".into(), "external:std::io::stdout".into()),
            // A glob of an external module could provide any name: never claimed.
            ("write".into(), "unresolved:open-scope".into()),
        ]
    );
}

#[test]
fn self_method_calls_resolve_only_at_the_method_probes_first_step() {
    // `self.m()` in an impl method: the inherent method of the impl's self type with the SAME
    // receiver form wins the probe's first step. A different receiver form, a differently shaped
    // generic impl, a trait-only method, a typed receiver, two candidates, or a trait default
    // body are never claimed.
    let lib = "pub struct S;\nimpl S {\n    fn by_ref(&self) {}\n    fn by_mut(&mut self) {}\n    fn by_value(self) {}\n    fn caller(&self) {\n        self.by_ref();\n        self.by_mut();\n        self.by_value();\n        self.from_trait();\n        self.twice();\n        let other = S;\n        other.by_ref();\n    }\n    fn caller_mut(&mut self) {\n        self.by_mut();\n    }\n    fn caller_boxed(self: Box<Self>) {\n        self.by_ref();\n    }\n    fn twice(&self) {}\n}\nimpl S {\n    #[cfg(unix)]\n    fn twice(&self) {}\n}\npub trait Tr {\n    fn from_trait(&self) {}\n    fn default_body(&self) {\n        self.from_trait();\n    }\n}\nimpl Tr for S {\n    fn from_trait(&self) {\n        self.by_ref();\n    }\n}\npub struct G<T>(T);\nimpl G<u8> {\n    fn only_u8(&self) {}\n}\nimpl G<u16> {\n    fn caller(&self) {\n        self.only_u8();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("self.by_ref".into(), "src/lib.rs:3:by_ref".into()),
            (
                "self.by_mut".into(),
                "unresolved:receiver-form-differs".into()
            ),
            (
                "self.by_value".into(),
                "unresolved:receiver-form-differs".into()
            ),
            (
                "self.from_trait".into(),
                "unresolved:method-not-inherent".into()
            ),
            (
                "self.twice".into(),
                "unresolved:ambiguous-associated".into()
            ),
            ("self.by_mut".into(), "src/lib.rs:4:by_mut".into()),
            ("self.by_ref".into(), "src/lib.rs:3:by_ref".into()),
            ("self.only_u8".into(), "unresolved:generic-impl".into()),
            // `other.by_ref()`: a receiver other than `self` needs its type inferred.
        ]
    );
}

#[test]
fn typed_local_method_calls_resolve_at_the_method_probes_first_step() {
    // G139 (NA-CALL-TYPE-RESIDUAL): a parameter or top-level `let` whose declared type is a plain
    // workspace type (`T`, `&T`, `&mut T`, or `Self` of a plain impl), bound once in the whole
    // function, decides `x.m()` exactly like `self.m()` does. Untyped, rebound, generic,
    // not-yet-bound and generic-typed receivers are never claimed.
    let lib = "pub struct S;\nimpl S {\n    fn by_ref(&self) {}\n    fn by_mut(&mut self) {}\n    fn by_value(self) {}\n    fn with_other(&self, other: &Self) {\n        other.by_ref();\n    }\n}\npub trait Tr {\n    fn from_trait(&self) {}\n}\nimpl Tr for S {}\npub struct G<T>(T);\nimpl G<u8> {\n    fn only_u8(&self) {}\n}\nfn params(r: &S, m: &mut S, v: S, g: &G<u8>) {\n    r.by_ref();\n    m.by_mut();\n    r.by_mut();\n    r.from_trait();\n    g.only_u8();\n    let f = || r.by_ref();\n    v.by_value();\n}\nfn shadowed(r: &S) {\n    let r = S;\n    r.by_ref();\n}\nfn closure_shadow(r: &S) {\n    let f = |r: &S| r.by_ref();\n}\nfn lets(owned: S) {\n    early.by_ref();\n    let early: &S = &owned;\n    early.by_ref();\n    let untyped = &owned;\n    untyped.by_ref();\n}\nfn generic<T: Tr>(t: &T) {\n    t.from_trait();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("other.by_ref".into(), "src/lib.rs:3:by_ref".into()),
            ("r.by_ref".into(), "src/lib.rs:3:by_ref".into()),
            ("m.by_mut".into(), "src/lib.rs:4:by_mut".into()),
            ("r.by_mut".into(), "unresolved:receiver-form-differs".into()),
            (
                "r.from_trait".into(),
                "unresolved:method-not-inherent".into()
            ),
            // `g: &G<u8>` has generic arguments: its method is left to inference, unclaimed.
            ("r.by_ref".into(), "src/lib.rs:3:by_ref".into()),
            ("v.by_value".into(), "src/lib.rs:5:by_value".into()),
            // `shadowed`, `closure_shadow`: `r` is bound twice; `early` before its `let`;
            // `untyped` has no declared type; `t: &T` is generic -- none claimed.
            ("early.by_ref".into(), "src/lib.rs:3:by_ref".into()),
        ]
    );
}

/// `spelling -> canonical` for every type occurrence in `path` (first occurrence per spelling).
fn canonical_types(files: &[(&str, &str)], root: &str, path: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for t in resolve_workspace(&one_crate(root), &sources(files)).types {
        if t.path == path {
            out.entry(t.spelling)
                .or_insert_with(|| t.canonical.unwrap_or_else(|| "-".into()));
        }
    }
    out
}

#[test]
fn types_canonicalize_bottom_up_through_resolution() {
    // One type under several spellings shares one canonical identity; composition is
    // structural; anything generic, opaque or unresolvable has none.
    let lib = "mod model {\n    pub struct Status;\n    pub struct Wrap<T>(pub T);\n}\nmod other {\n    pub struct Status;\n}\nuse model::Status;\nuse std::fmt;\npub type Alias = model::Status;\npub type GenAlias<T> = Vec<T>;\npub fn f(\n    a: Status,\n    b: crate::model::Status,\n    c: &mut fmt::Formatter<'_>,\n    d: Vec<Status>,\n    e: Option<Alias>,\n    g: [u8; 4],\n    h: (Status,),\n    i: GenAlias<u8>,\n    j: other::Status,\n    k: &dyn fmt::Debug,\n) {\n}\npub fn g<T>(t: T, w: model::Wrap<T>, x: model::Wrap<u8>) {}\npub struct S;\nimpl S {\n    fn m(&self) -> Self {\n        S\n    }\n}\npub struct G<T>(T);\nimpl<T> G<T> {\n    fn n(&self) -> Self {\n        todo!()\n    }\n}\n";
    let types = canonical_types(&[("src/lib.rs", lib)], "src/lib.rs", "src/lib.rs");
    let expect = [
        ("Status", ". model/Status#"),
        ("crate::model::Status", ". model/Status#"),
        ("&mut fmt::Formatter<'_>", "&mut std::fmt::Formatter"),
        ("Vec<Status>", "std::vec::Vec<. model/Status#>"),
        ("Option<Alias>", "std::option::Option<. model/Status#>"),
        ("[u8; 4]", "[u8; 4]"),
        ("(Status,)", "(. model/Status#,)"),
        ("GenAlias<u8>", "-"),
        ("other::Status", ". other/Status#"),
        ("&dyn fmt :: Debug", "-"),
        ("T", "-"),
        ("model::Wrap<T>", "-"),
        ("model::Wrap<u8>", ". model/Wrap#<u8>"),
        ("S", ". S#"),
    ];
    for (spelling, canonical) in expect {
        assert_eq!(
            types.get(spelling).map(String::as_str),
            Some(canonical),
            "{spelling}: {types:#?}"
        );
    }
    // `Self` in a generic impl is not one type.
    let selves: Vec<Option<String>> =
        resolve_workspace(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", lib)]))
            .types
            .into_iter()
            .filter(|t| t.spelling == "Self")
            .map(|t| t.canonical)
            .collect();
    assert_eq!(selves, [Some(". S#".to_owned()), None]);
}

#[test]
fn type_canonicalization_never_guesses() {
    // A generic parameter shadowing a type, an alias whose arguments a bare path cannot carry,
    // a glob-provided name in an open module, and (by contrast) an explicit item in an open
    // module, which is certain.
    let lib = "mod model {\n    pub struct Status;\n    pub struct Wrap<T>(pub T);\n}\nmod open {\n    make_items!();\n    pub struct Real;\n    pub use super::model::*;\n}\nuse model::Status;\npub type Pair<T> = model::Wrap<(T, T)>;\npub fn shadow<Status>(s: Status) {}\npub fn f(p: Pair<u8>, r: open::Real, w: open::Status, x: Status) {}\n";
    let occurrences: Vec<(usize, String, Option<String>)> =
        resolve_workspace(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", lib)]))
            .types
            .into_iter()
            .map(|t| (t.line, t.spelling, t.canonical))
            .collect();
    let at = |line: usize, spelling: &str| {
        occurrences
            .iter()
            .find(|(l, s, _)| *l == line && s == spelling)
            .map(|(_, _, c)| c.clone())
            .unwrap_or_else(|| panic!("{line} {spelling}: {occurrences:#?}"))
    };
    assert_eq!(
        at(12, "Status"),
        None,
        "a generic parameter, not model::Status"
    );
    assert_eq!(at(13, "Status"), Some(". model/Status#".into()));
    assert_eq!(at(13, "Pair<u8>"), None, "Pair<u8> is Wrap<(u8, u8)>");
    assert_eq!(at(13, "open::Real"), Some(". open/Real#".into()));
    assert_eq!(
        at(13, "open::Status"),
        None,
        "a glob name in an open module"
    );
}
