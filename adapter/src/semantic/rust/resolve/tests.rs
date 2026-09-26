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
                PathCallOutcome::Dynamic(t) => format!("dynamic:{}:{}:{}", t.path, t.line, t.name),
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
fn method_calls_and_initializers_are_outside_the_pass() {
    // G133: a closure body is inside the pass (its own executable region); a closure parameter
    // shadowing a function name stays a local; G159: an `async` block is inside too; method
    // calls on untyped receivers and initializers stay outside.
    let lib = "fn f() {}\nconst C: () = f();\nfn t(x: X) {\n    x.method();\n    let c = || f();\n    let d = |f: fn()| f();\n    let e = async { f() };\n    f();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("f".into(), "src/lib.rs:1:f".into()),
            ("f".into(), "unresolved:local-binding".into()),
            ("f".into(), "src/lib.rs:1:f".into()),
            ("f".into(), "src/lib.rs:1:f".into()),
        ]
    );
    let positions: Vec<(usize, usize)> = results.iter().map(|r| (r.line, r.column)).collect();
    assert_eq!(positions, [(5, 15), (6, 22), (7, 20), (8, 4)]);
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
    // generic impl, a trait-only method, a typed receiver or two candidates are never claimed
    // (a trait default body's `self` is G140's, below).
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
            // G140: `self` in a trait default body is `Self: Tr`: the trait's own method.
            (
                "self.from_trait".into(),
                "dynamic:src/lib.rs:28:from_trait".into()
            ),
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
            // `untyped` has no declared type -- none claimed.
            ("early.by_ref".into(), "src/lib.rs:3:by_ref".into()),
            // G140: `t: &T` with `T: Tr` calls the trait's method through its declaration.
            (
                "t.from_trait".into(),
                "dynamic:src/lib.rs:11:from_trait".into()
            ),
        ]
    );
}

#[test]
fn bounded_receivers_call_the_trait_method_through_its_declaration() {
    // G140: a receiver known only by workspace-trait bounds -- `&dyn Tr`, `impl Tr`, a function
    // generic bounded inline or in a where clause, method-less markers allowed -- calls the one
    // bound method of that name with the same receiver form, dynamically. A foreign bound, a
    // supertrait's method, a form mismatch, a missing method, an impl-level generic and a
    // `Box<dyn Tr>` are never claimed.
    let lib = "pub trait Tr {\n    fn look(&self);\n    fn poke(&mut self);\n}\npub trait Sub: Tr {\n    fn own(&self);\n}\npub trait Other {\n    fn look(&self);\n}\nfn object(d: &dyn Tr, m: &mut dyn Tr, b: Box<dyn Tr>) {\n    d.look();\n    m.poke();\n    d.poke();\n    d.absent();\n    b.look();\n}\nfn opaque(o: &impl Tr) {\n    o.look();\n}\nfn inline<T: Tr + Send>(t: &T) {\n    t.look();\n}\nfn clause<T>(t: &T)\nwhere\n    T: Tr + ?Sized,\n{\n    t.look();\n}\nfn foreign<T: Tr + std::fmt::Debug>(t: &T) {\n    t.look();\n}\nfn supertrait<T: Sub>(s: &T) {\n    s.own();\n    s.look();\n}\nfn two<T: Tr + Other>(t: &T) {\n    t.look();\n}\npub struct W<U>(U);\nimpl<U: Tr> W<U> {\n    fn field(&self, u: &U) {\n        u.look();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("d.look".into(), "dynamic:src/lib.rs:2:look".into()),
            ("m.poke".into(), "dynamic:src/lib.rs:3:poke".into()),
            ("d.poke".into(), "unresolved:receiver-form-differs".into()),
            ("d.absent".into(), "unresolved:method-not-in-bounds".into()),
            ("o.look".into(), "dynamic:src/lib.rs:2:look".into()),
            ("t.look".into(), "dynamic:src/lib.rs:2:look".into()),
            ("t.look".into(), "dynamic:src/lib.rs:2:look".into()),
            ("s.own".into(), "dynamic:src/lib.rs:6:own".into()),
            ("s.look".into(), "unresolved:method-not-in-bounds".into()),
            ("t.look".into(), "unresolved:ambiguous-associated".into()),
        ]
    );
    // An inherent impl on a trait object competes with the trait's methods: nothing through
    // `dyn` is claimed then.
    let with_inherent = format!("{lib}impl dyn Tr {{\n    fn extra(&self) {{}}\n}}\n");
    let results = resolve(&[("src/lib.rs", &with_inherent)], "src/lib.rs");
    assert!(
        !outcomes(&results, "src/lib.rs")
            .iter()
            .any(|(callee, _)| callee.starts_with("d.") || callee.starts_with("m.")),
    );
}

#[test]
fn signature_typed_lets_and_guarded_autoref_resolve_methods() {
    // G142: a `let` bound once to a call whose callee declares a plain workspace output (`Self`
    // of a plain impl included), a struct literal, a tuple-struct or variant constructor is a
    // value of that type; the probe's autoref step (a value taking `&self`/`&mut self`, a
    // `&mut T` taking `&self`) is claimed only when no by-value method of the name can come
    // first: a by-value workspace trait method, `into`/`try_into`/`into_iter`, a non-std import
    // in scope, or an Iterator-like impl on the type each withhold it.
    let lib = "pub struct S;\nimpl S {\n    pub fn new() -> Self { S }\n    fn by_ref(&self) {}\n    fn by_mut(&mut self) {}\n    fn into(&self) {}\n    fn consume(&self) {}\n}\npub trait Eat {\n    fn consume(self);\n}\nfn make() -> S { S }\nfn any<T>() -> T { unimplemented!() }\npub struct P { x: u8 }\nimpl P {\n    fn look(&self) {}\n}\npub struct Tup(u8);\nimpl Tup {\n    fn look(&self) {}\n}\npub enum E { A(u8) }\nimpl E {\n    fn look(&self) {}\n}\npub struct It;\nimpl It {\n    fn peek_ref(&self) {}\n}\nimpl Iterator for It {\n    type Item = u8;\n    fn next(&mut self) -> Option<u8> { None }\n}\nfn values(m: &mut S) {\n    let a = S::new();\n    a.by_ref();\n    let mut b = make();\n    b.by_mut();\n    let p = P { x: 1 };\n    p.look();\n    let t = Tup(1);\n    t.look();\n    let e = E::A(1);\n    e.look();\n    m.by_ref();\n    a.into();\n    a.consume();\n    let it = It;\n    let i = make_it();\n    i.peek_ref();\n    let g: S = any();\n    let h = any::<S>();\n    h.by_ref();\n}\nfn make_it() -> It { It }\nfn rebound() {\n    let z = S::new();\n    {\n        let z = Tup(1);\n        z.by_ref();\n    }\n}\nmod ext {\n    use outside::Thing;\n    fn f() {\n        let s = super::S::new();\n        s.by_ref();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    let got: Vec<(String, String)> = outcomes(&results, "src/lib.rs")
        .into_iter()
        .filter(|(callee, _)| callee.contains('.'))
        .collect();
    let expect: Vec<(String, String)> = [
        ("a.by_ref", "src/lib.rs:4:by_ref"),
        ("b.by_mut", "src/lib.rs:5:by_mut"),
        ("p.look", "src/lib.rs:16:look"),
        ("t.look", "src/lib.rs:20:look"),
        ("e.look", "src/lib.rs:24:look"),
        ("m.by_ref", "src/lib.rs:4:by_ref"),
        // `into` could be `Into::into` by value; `consume` could be `Eat::consume`.
        ("a.into", "unresolved:receiver-form-differs"),
        ("a.consume", "unresolved:receiver-form-differs"),
        // `It` implements Iterator: a by-value adaptor could come first.
        ("i.peek_ref", "unresolved:receiver-form-differs"),
        // `g: S` is typed by its declaration (G139); `h` from `any::<S>()` has generic arguments.
        // `rebound`: `z` is bound twice, so neither binding types it (the inner one is a Tup).
        // In `ext`, a non-std import could bring a trait: withheld.
        ("s.by_ref", "unresolved:receiver-form-differs"),
    ]
    .iter()
    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
    .collect();
    assert_eq!(got, expect);
}

#[test]
fn field_and_call_result_receivers_are_typed_forward() {
    // G143: a receiver's type follows its expression -- a field a non-generic struct declares
    // (`T`, `&T`, `&mut T`), the result of a resolved call whose callee declares a plain output,
    // a `let` bound once to either, or any of them in parentheses. Generic arguments, reference
    // expressions, untyped bases and generic structs are never typed.
    let lib = "pub struct Leaf;\nimpl Leaf {\n    fn touch(&self) {}\n    fn poke(&mut self) {}\n}\npub struct Wrap<T>(T);\nimpl<T> Wrap<T> {\n    fn touch(&self) {}\n}\npub struct Node {\n    leaf: Leaf,\n    shared: &'static Leaf,\n    many: Vec<Leaf>,\n    wrapped: Wrap<Leaf>,\n}\nimpl Node {\n    fn leaf(&self) -> Leaf {\n        Leaf\n    }\n    fn walk(&mut self) {\n        self.leaf.touch();\n        self.shared.touch();\n        self.leaf.poke();\n        self.leaf().touch();\n        self.many.len();\n        self.wrapped.touch();\n        let l = self.leaf();\n        l.touch();\n        let f = &self.leaf;\n        f.touch();\n        (self.leaf).touch();\n    }\n}\nfn make_node() -> Node {\n    unimplemented!()\n}\nfn free(other: Vec<Leaf>) {\n    make_node().walk();\n    let mut n = make_node();\n    n.leaf.poke();\n    other.first();\n    let n2 = make_node();\n    let l2 = n2.leaf();\n    l2.touch();\n}\npub struct Item;\nimpl Item {\n    fn touch(&self) {}\n}\npub struct G<Item> {\n    inner: Item,\n}\nimpl G<Leaf> {\n    fn look(&self) {\n        self.inner.touch();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    let got: Vec<(String, String)> = outcomes(&results, "src/lib.rs")
        .into_iter()
        .filter(|(callee, _)| callee.contains('.'))
        .collect();
    let expect: Vec<(String, String)> = [
        ("self.leaf.touch", "src/lib.rs:3:touch"),
        // A `&Leaf` field is a `&self` receiver at the probe's first step.
        ("self.shared.touch", "src/lib.rs:3:touch"),
        ("self.leaf.poke", "src/lib.rs:4:poke"),
        ("self.leaf", "src/lib.rs:17:leaf"),
        ("self.leaf().touch", "src/lib.rs:3:touch"),
        // `self.many` (`Vec<Leaf>`) and `self.wrapped` (`Wrap<Leaf>`) have generic arguments:
        // never typed.
        ("self.leaf", "src/lib.rs:17:leaf"),
        ("l.touch", "src/lib.rs:3:touch"),
        // `f` is bound to a reference expression: untyped.
        ("self.leaf.touch", "src/lib.rs:3:touch"),
        ("make_node().walk", "src/lib.rs:20:walk"),
        ("n.leaf.poke", "src/lib.rs:4:poke"),
        // A `let` typed from an earlier one.
        ("n2.leaf", "src/lib.rs:17:leaf"),
        ("l2.touch", "src/lib.rs:3:touch"),
        // `G<Item>` is generic (its parameter shadows the type `Item`): its fields are never
        // typed, so `self.inner.touch()` is never claimed as `Item::touch`.
    ]
    .iter()
    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
    .collect();
    assert_eq!(got, expect);
}

#[test]
fn std_receivers_reach_the_declared_std_method_paths() {
    // G144: a receiver whose std type is declared (a parameter, a `let` in any block) or comes
    // from a declared std constructor (`?` unwrapping a fallible one) calls a declared inherent
    // std method as its std path; an undeclared method, a `Result` never unwrapped, and a guarded
    // autoref (a non-std import in scope) are never claimed.
    let lib = "use std::fs::File;\nuse std::sync::Mutex;\nuse std::sync::mpsc::{Receiver, Sender};\nfn write(path: &str) -> std::io::Result<()> {\n    {\n        let mut file = File::create(path)?;\n        file.sync_all()?;\n    }\n    File::open(path)?.sync_all()?;\n    let m = Mutex::new(0);\n    m.lock();\n    let bare = File::open(path);\n    bare.sync_all();\n    Ok(())\n}\nfn channels(tx: &Sender<u8>, rx: Receiver<u8>, f: &File) {\n    tx.send(1);\n    rx.recv();\n    f.metadata();\n}\npub struct Gate;\nimpl Gate {\n    fn lock(&self) {}\n}\nstatic GATE: Gate = Gate;\n#[allow(non_snake_case)]\nfn scoped() {\n    {\n        let GATE = Mutex::new(1);\n    }\n    GATE.lock();\n}\nmod ext {\n    use outside::Thing;\n    fn by_ref(m: &std::sync::Mutex<u8>) {\n        m.lock();\n    }\n    fn by_value(m: std::sync::Mutex<u8>) {\n        m.lock();\n    }\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    let got: Vec<(String, String)> = outcomes(&results, "src/lib.rs")
        .into_iter()
        .filter(|(callee, _)| callee.contains('.'))
        .collect();
    let expect: Vec<(String, String)> = [
        ("file.sync_all", "external:std::fs::File::sync_all"),
        (
            "std::fs::File::open()?.sync_all",
            "external:std::fs::File::sync_all",
        ),
        ("m.lock", "external:std::sync::Mutex::lock"),
        // `bare` holds a `Result`: never typed.
        ("tx.send", "external:std::sync::mpsc::Sender::send"),
        ("rx.recv", "external:std::sync::mpsc::Receiver::recv"),
        ("f.metadata", "unresolved:std-method-undeclared"),
        // `GATE.lock()` after the block is the static `Gate`, not the block's `Mutex`: unclaimed.
        // `&Mutex` takes `&self` lock at the first step: no guard needed.
        ("m.lock", "external:std::sync::Mutex::lock"),
        // By value, the autoref step is withheld beside a non-std import.
        ("m.lock", "unresolved:receiver-form-differs"),
    ]
    .iter()
    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
    .collect();
    assert_eq!(got, expect);
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

/// G145: the 1-based line of the first line of `text` containing `needle`.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} not in the fixture"))
        + 1
}

/// G145: the canonical identity of each occurrence of `spelling` on `line` of `path`.
fn canonicals_at(
    resolution: &WorkspaceResolution,
    path: &str,
    line: usize,
    spelling: &str,
) -> Vec<Option<String>> {
    let found: Vec<Option<String>> = resolution
        .types
        .iter()
        .filter(|t| t.path == path && t.line == line && t.spelling == spelling)
        .map(|t| t.canonical.clone())
        .collect();
    assert!(
        !found.is_empty(),
        "{spelling} on {line}: {:#?}",
        resolution.types
    );
    found
}

const VOCAB: &str = "#[macro_export]\nmacro_rules! vocab {\n    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {\n        $(#[$meta])*\n        #[derive(Debug, Clone, Copy, serde::Serialize)]\n        $vis enum $name { $($variant),+ }\n        impl $name {\n            pub const fn as_str(&self) -> &'static str { match self { $(Self::$variant => $text),+ } }\n        }\n    };\n}\nmacro_rules! id {\n    ($name:ident) => { pub struct $name(String); };\n}\nmacro_rules! imports {\n    () => { use std::collections::*; };\n}\nmacro_rules! nested {\n    () => { other!(); };\n}\n";

#[test]
fn a_workspace_macro_rules_bounds_the_names_its_invocation_may_define() {
    // The invocation may define `Verdict` (a metavariable in a name position, bound from its
    // tokens) and what the transcriber spells after an item keyword (`as_str`); `String`, `str`
    // (after the lifetime `'static`, which is not the keyword `static`) and a glob-imported
    // `Status` are certain; the glob's `Verdict` is not, since the macro's would shadow it.
    let lib = format!(
        "{VOCAB}mod model {{\n    pub struct Status;\n    pub struct Verdict;\n}}\nmod bounded {{\n    crate::vocab! {{\n        /// doc\n        pub enum Verdict {{ Held => \"HELD\" }}\n    }}\n    crate::vocab! {{ pub enum Mode {{ On => \"ON\" }} }}\n    pub use super::model::*;\n    pub fn helper() {{}}\n    pub fn f(s: String, t: &'static str, v: Verdict, w: Status) {{\n        helper();\n        Verdict::held();\n        Mode::on();\n    }}\n}}\nid!(Named);\npub fn g(n: Named, s: String) {{}}\n"
    );
    let resolution = resolve_workspace(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", &lib)]));
    let f = line_of(&lib, "pub fn f(");
    assert_eq!(
        canonicals_at(&resolution, "src/lib.rs", f, "String"),
        [Some("std::string::String".into())]
    );
    assert_eq!(
        canonicals_at(&resolution, "src/lib.rs", f, "&'static str"),
        [Some("&str".into())]
    );
    assert_eq!(
        canonicals_at(&resolution, "src/lib.rs", f, "Verdict"),
        [None],
        "the macro defines it"
    );
    assert_eq!(
        canonicals_at(&resolution, "src/lib.rs", f, "Status"),
        [Some(". model/Status#".into())]
    );
    // Textual scope: `id!` is defined above its invocation in the same module.
    let g = line_of(&lib, "pub fn g(");
    assert_eq!(canonicals_at(&resolution, "src/lib.rs", g, "Named"), [None]);
    assert_eq!(
        canonicals_at(&resolution, "src/lib.rs", g, "String"),
        [Some("std::string::String".into())]
    );
    let results = resolve_path_calls(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", &lib)]));
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            (
                "helper".into(),
                format!("src/lib.rs:{}:helper", line_of(&lib, "pub fn helper"))
            ),
            // A name the macro may define is uncertain, not an external crate.
            ("Verdict::held".into(), "unresolved:open-scope".into()),
            ("Mode::on".into(), "unresolved:open-scope".into()),
        ]
    );
}

#[test]
fn an_unbounded_item_macro_still_opens_its_module() {
    // A transcriber that imports, one that invokes a macro outside the expression allowlist, an
    // invocation carrying an attribute outside the allowlist, an invocation before the
    // definition (no textual scope) and a macro no workspace definition names: each may define
    // any name, so even `String` stays unresolved there. The control case, a bounded invocation
    // in the same position, resolves it.
    let cases = [
        ("imports!();", None),
        ("nested!();", None),
        (
            "crate::vocab! { #[my_attr] pub enum E { A => \"A\" } }",
            None,
        ),
        ("late!(X);", None),
        ("external::make!();", None),
        ("early!(X);", Some("std::string::String".to_owned())),
    ];
    for (case, expected) in cases {
        let lib = format!(
            "{VOCAB}mod m {{\n    macro_rules! imports {{\n        () => {{ use std::collections::*; }};\n    }}\n    macro_rules! nested {{\n        () => {{ other!(); }};\n    }}\n    macro_rules! early {{\n        ($name:ident) => {{ pub struct $name; }};\n    }}\n    {case}\n    pub fn f(s: String) {{}}\n    macro_rules! late {{\n        ($name:ident) => {{ pub struct $name; }};\n    }}\n}}\n"
        );
        let resolution =
            resolve_workspace(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", &lib)]));
        let f = line_of(&lib, "pub fn f(");
        assert_eq!(
            canonicals_at(&resolution, "src/lib.rs", f, "String"),
            [expected],
            "{case}"
        );
    }
}

#[test]
fn a_glob_import_carries_the_names_a_macro_may_define() {
    // `Kind` may exist in `gen` (the macro defines it, shadowing the glob's), so through
    // `gen::*` it is uncertain in `user` -- never taken for an external crate or for
    // `model::Kind` -- while `Other` is still a plain miss.
    let lib = format!(
        "{VOCAB}mod model {{\n    pub struct Kind;\n    impl Kind {{\n        pub fn from_str() {{}}\n    }}\n}}\nmod gen {{\n    crate::vocab! {{ pub enum Kind {{ A => \"A\" }} }}\n    pub use super::model::*;\n}}\nmod user {{\n    use super::gen::*;\n    pub fn t() {{\n        Kind::from_str();\n        Other::f();\n        super::gen::Kind::from_str();\n        super::model::Kind::from_str();\n    }}\n}}\n"
    );
    let results = resolve_path_calls(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", &lib)]));
    assert_eq!(
        outcomes(&results, "src/lib.rs"),
        [
            ("Kind::from_str".into(), "unresolved:open-scope".into()),
            ("Other::f".into(), "unresolved:external".into()),
            // Through a path, too: `gen`'s glob `Kind` may be shadowed by the macro's.
            (
                "super::gen::Kind::from_str".into(),
                "unresolved:unresolved-prefix".into()
            ),
            (
                "super::model::Kind::from_str".into(),
                format!("src/lib.rs:{}:from_str", line_of(&lib, "pub fn from_str")),
            ),
        ]
    );
}

#[test]
fn transcriber_names_skip_lifetimes_and_metavariables() {
    let body: proc_macro2::TokenStream =
        "($name:ident) => { const fn a() -> &'static str { \"\" } static mut B: u8 = 0; fn $name() {} struct C<'static_ish>; };"
            .parse()
            .expect("tokens");
    let names = macro_rules_names(body);
    assert!(!names.unbounded);
    assert!(names.from_invocation);
    for name in ["a", "B", "C"] {
        assert!(names.literal.contains(name), "{name}: {names:?}");
    }
    assert!(!names.literal.contains("str"), "{names:?}");
    assert!(!names.literal.contains("u8"), "{names:?}");
}

#[test]
fn lifetime_parameters_never_change_which_method_a_receiver_reaches() {
    // G151 (replay of GitNexus): lifetimes are erased from impl shapes and declared types, so a
    // `Self`-returning constructor of `impl<'a> Walker<'a>` types its `let`, and a parameter of
    // type `&Walker<'_>` is a typed receiver; a type parameter still leaves the impl generic.
    let lib = "pub struct Walker<'a> {\n    s: &'a str,\n}\nimpl<'a> Walker<'a> {\n    pub fn new(s: &'a str) -> Self {\n        Walker { s }\n    }\n    fn step(&mut self) {}\n    fn look(&self) {}\n}\npub struct Gen<'a, T> {\n    s: &'a T,\n}\nimpl<'a, T> Gen<'a, T> {\n    fn peek(&self) {}\n}\nfn run(text: &str, w: &Walker<'_>, g: &Gen<'_, u8>) {\n    let mut walker = Walker::new(text);\n    walker.step();\n    walker.look();\n    w.look();\n    g.peek();\n}\n";
    let results = resolve(&[("src/lib.rs", lib)], "src/lib.rs");
    let expect: Vec<(String, String)> = [
        ("Walker::new", "src/lib.rs:5:new"),
        ("walker.step", "src/lib.rs:8:step"),
        ("walker.look", "src/lib.rs:9:look"),
        ("w.look", "src/lib.rs:9:look"),
        // `g: &Gen<'_, u8>` keeps a type argument: never typed, so no outcome.
    ]
    .iter()
    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
    .collect();
    assert_eq!(outcomes(&results, "src/lib.rs"), expect);
}

/// G154 (replay R3, tree-sitter): a function declared in an `extern "C"` block is a call target,
/// and an item-position `include!("bindings.rs")` splices that file's items into the including
/// module -- as the tree-sitter Rust binding reaches `ffi::ts_parser_parse_with_options`. A
/// computed `include!(concat!(..))` is not followed.
#[test]
fn foreign_functions_resolve_through_a_literal_include() {
    let lib = "pub mod ffi;\npub fn parse() {\n    unsafe { ffi::ts_parse(); }\n    ffi::local();\n    unsafe { ffi::hidden(); }\n}\n";
    let ffi = "include!(\"./bindings.rs\");\ninclude!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\nextern \"C\" {\n    pub fn local();\n}\n";
    let bindings = "extern \"C\" {\n    /// Parses.\n    pub fn ts_parse();\n    pub static VERSION: u32;\n}\n";
    let generated = "extern \"C\" {\n    pub fn hidden();\n}\n";
    let results = resolve(
        &[
            ("src/lib.rs", lib),
            ("src/ffi.rs", ffi),
            ("src/bindings.rs", bindings),
            ("src/generated.rs", generated),
        ],
        "src/lib.rs",
    );
    let found = outcomes(&results, "src/lib.rs");
    assert_eq!(
        found[0],
        ("ffi::ts_parse".into(), "src/bindings.rs:2:ts_parse".into())
    );
    assert_eq!(found[1], ("ffi::local".into(), "src/ffi.rs:4:local".into()));
    assert_eq!(found[2].0, "ffi::hidden");
    assert!(
        found[2].1.starts_with("unresolved"),
        "a computed include is never followed: {found:?}"
    );
}

/// G156 (replay R4, salsa): an impl's shape is the set of types it applies to, not its spelling.
/// `impl<C: Config> Ingredient<C>` and `impl<T> Ingredient<T> where T: Config` apply to the same
/// types, so a `self` call from one reaches a method of the other (salsa splits
/// `IngredientImpl<C>` this way across files); impls with different bound sets stay apart.
#[test]
fn inline_and_where_bounds_with_renamed_parameters_share_an_impl_shape() {
    let lib = "pub trait Config {}\npub trait Other {}\npub struct Ingredient<C> { c: C }\nmod memo;\nimpl<T> Ingredient<T> where T: Config {\n    pub fn top(&self) {\n        self.from_bound();\n        self.from_split_bounds();\n        self.from_other_bound();\n    }\n}\nimpl<C: Config + Other> Ingredient<C> {\n    fn from_split_bounds_twin(&self) {}\n}\n";
    let memo = "use super::*;\nimpl<C: Config> Ingredient<C> {\n    pub(super) fn from_bound(&self) {}\n}\nimpl<X> Ingredient<X> where X: Config, {\n    pub(super) fn from_split_bounds(&self) {}\n}\nimpl<C: Other> Ingredient<C> {\n    pub(super) fn from_other_bound(&self) {}\n}\n";
    let results = resolve(&[("src/lib.rs", lib), ("src/memo.rs", memo)], "src/lib.rs");
    let found = outcomes(&results, "src/lib.rs");
    assert_eq!(
        found[0],
        ("self.from_bound".into(), "src/memo.rs:3:from_bound".into()),
        "{found:?}"
    );
    assert_eq!(found[1].1, "src/memo.rs:6:from_split_bounds", "{found:?}");
    assert!(
        found[2].1.starts_with("unresolved"),
        "a different bound set is a different impl: {found:?}"
    );
}

/// G157: `holder@line -> (release, line)` for every release the resolver claims in `path`.
fn releases(resolution: &WorkspaceResolution, path: &str) -> Vec<(String, String, usize)> {
    resolution
        .releases
        .iter()
        .filter(|r| r.path == path)
        .map(|r| {
            (
                format!("{}@{}", r.holder, r.acquired.0),
                format!("{} {}", r.kind.as_str(), r.release.as_str()),
                r.line,
            )
        })
        .collect()
}

#[test]
fn let_bound_resources_are_released_where_their_holder_gives_them_back() {
    // G157: a holder bound once by a `let` to a resolved acquisition (through `?`, `unwrap`,
    // `expect`) and never moved is released at its block's end; a resolved `drop` or `join`
    // statement of that block releases it there; a thread is never released by a drop; every
    // move -- passed, returned, captured by a `move` closure, named in a non-formatting macro,
    // consumed by a by-value method (`take`, `into_*`), released inside a branch -- leaves the
    // release unclaimed.
    let lib = "\
use std::fs::File;
use std::sync::Mutex;
use std::thread;
fn consume(_f: File) {}
fn scope_end(p: &str) -> std::io::Result<()> {
    let file = File::open(p)?;
    file.metadata()?;
    let _r = &file;
    println!(\"{:?}\", file);
    Ok(())
}
fn explicit(m: &Mutex<Vec<u8>>) {
    let guard = m.lock().unwrap();
    let _n = guard.len();
    drop(guard);
    m.lock().unwrap().push(1);
}
fn joined() {
    let handle = thread::spawn(|| {});
    handle.join().expect(\"joined\");
    let detached = std::thread::spawn(|| {});
}
fn moved(p: &str, m: &Mutex<u8>, c: bool) -> std::io::Result<File> {
    let passed = File::open(p)?;
    consume(passed);
    let branch = m.lock().unwrap();
    if c {
        drop(branch);
    }
    let captured = m.lock().expect(\"lock\");
    let _k = move || *captured;
    let hidden = File::open(p)?;
    my_macro!(hidden);
    let raw = File::open(p)?;
    raw.into_inner();
    let stored = File::open(p)?;
    let _pair = (stored, 1);
    let limited = File::open(p)?;
    let _reader = limited.take(4);
    let returned = File::create(p)?;
    Ok(returned)
}
";
    let resolution = resolve_workspace(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", lib)]));
    let line = |needle: &str| line_of(lib, needle);
    let end_of = |needle: &str| {
        // The closing brace of the function containing `needle`.
        lib.lines()
            .enumerate()
            .skip(line(needle))
            .find(|(_, l)| *l == "}")
            .map(|(i, _)| i + 1)
            .unwrap()
    };
    assert_eq!(
        releases(&resolution, "src/lib.rs"),
        [
            (
                format!("file@{}", line("let file")),
                "FILE SCOPE_END".to_owned(),
                end_of("let file"),
            ),
            (
                format!("guard@{}", line("let guard")),
                "LOCK_GUARD EXPLICIT_DROP".to_owned(),
                line("drop(guard)"),
            ),
            (
                format!("handle@{}", line("let handle")),
                "THREAD JOIN".to_owned(),
                line("handle.join()"),
            ),
        ]
    );
    // The acquisitions themselves resolve to the std paths the resource table names.
    let got = outcomes(&resolution.calls, "src/lib.rs");
    for (callee, outcome) in [
        ("File::open", "external:std::fs::File::open"),
        ("m.lock", "external:std::sync::Mutex::lock"),
        ("thread::spawn", "external:std::thread::spawn"),
        ("drop", "external:std::mem::drop"),
        ("handle.join", "external:std::thread::JoinHandle::join"),
    ] {
        assert!(
            got.contains(&(callee.to_owned(), outcome.to_owned())),
            "{callee} -> {outcome} in {got:?}"
        );
    }
}

/// G162 (replay R7, zed's `language` crate): a module re-exports `crate::Registry` by name while
/// the crate root glob-imports that module and names `Registry` from its defining module. The
/// placeholder an early round binds for the not-yet-resolved `pub use crate::Registry` then
/// travels around the cycle in the value namespace, flipping every round; before G162 the import
/// fixed point failed after 64 rounds and every module of the workspace was opened.
#[test]
fn a_re_export_cycle_converges_instead_of_opening_the_workspace() {
    let lib = "mod registry;\nmod buffer;\npub use registry::Registry;\npub use buffer::*;\npub fn entry() {\n    helper();\n    std::fs::read(\"x\").ok();\n    Registry::new();\n}\n";
    let buffer = "pub use crate::Registry;\npub fn helper() {}\n";
    let registry = "pub struct Registry {\n    pub size: usize,\n}\nimpl Registry {\n    pub fn new() -> Self {\n        Registry { size: 0 }\n    }\n}\n";
    let workspace = resolve_workspace(
        &one_crate("src/lib.rs"),
        &sources(&[
            ("src/lib.rs", lib),
            ("src/buffer.rs", buffer),
            ("src/registry.rs", registry),
        ]),
    );
    assert_eq!(
        outcomes(&workspace.calls, "src/lib.rs"),
        [
            ("helper".into(), "src/buffer.rs:2:helper".into()),
            ("std::fs::read".into(), "external:std::fs::read".into()),
            ("Registry::new".into(), "src/registry.rs:5:new".into()),
        ]
    );
    assert_eq!(workspace.withheld, []);
}

/// G162: every path call withheld because of an open scope says which scope and why -- an item
/// macro this pass cannot bound, an unresolvable glob, a glob of an open module, a missing
/// module file -- instead of leaving the pilot to find the cause by hand.
#[test]
fn every_withheld_path_names_why_its_scope_is_open() {
    let lib = "mod missing;\nmod macros {\n    dependency::make_items! { pub struct Made; }\n    other::more_items! { pub struct More; }\n    fn t() {\n        std::fs::read(\"x\").ok();\n    }\n}\nmod importer {\n    use super::macros::*;\n    fn t() {\n        generated();\n    }\n}\nmod external {\n    use outside::*;\n    fn t() {\n        anything();\n    }\n}\nmod missing_user {\n    use super::missing::*;\n    fn t() {\n        lost();\n    }\n}\n";
    let workspace = resolve_workspace(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", lib)]));
    let causes: Vec<(String, String)> = workspace
        .withheld
        .iter()
        .map(|w| (w.callee.clone(), w.cause.clone()))
        .collect();
    assert_eq!(
        causes,
        [
            (
                "std::fs::read".into(),
                "item macro `dependency::make_items!` at src/lib.rs:3 may define any name: no workspace `macro_rules!` definition this pass follows names it (a dependency's macro, or one imported from another crate)".into()
            ),
            (
                "generated".into(),
                "glob import `super::macros::*` reads an open module (item macro `dependency::make_items!` at src/lib.rs:3 may define any name: no workspace `macro_rules!` definition this pass follows names it (a dependency's macro, or one imported from another crate))".into()
            ),
            (
                "anything".into(),
                "glob import `outside::*` reads a crate or item outside the workspace (its names are unknown)".into()
            ),
            (
                "lost".into(),
                "glob import `super::missing::*` reads an open module (`mod missing` names no module file this census holds)".into()
            ),
        ]
    );
    // The withheld calls are exactly the ones reported `open-scope`.
    let open: Vec<&str> = workspace
        .calls
        .iter()
        .filter(|c| c.outcome == PathCallOutcome::Unresolved("open-scope"))
        .map(|c| c.callee.as_str())
        .collect();
    assert_eq!(open, ["std::fs::read", "generated", "anything", "lost"]);
}

/// G162: the merge of two alternating scopes keeps what both phases agree on and binds anything
/// else to `Unknown` -- a conflicting definition and a name only one phase binds alike.
#[test]
fn alternating_scopes_merge_to_their_agreement_and_unknown() {
    let entry = |def: Def| Entry {
        def,
        vis: Vis::Public,
        origin: Origin::Named,
    };
    let mut a = Scope::default();
    let mut b = Scope::default();
    a.insert(Ns::Types, "Same", entry(Def::Module(1)));
    b.insert(Ns::Types, "Same", entry(Def::Module(1)));
    a.insert(Ns::Types, "Differs", entry(Def::Module(1)));
    b.insert(Ns::Types, "Differs", entry(Def::Module(2)));
    a.insert(Ns::Values, "OnlyA", entry(Def::Module(3)));
    b.insert(Ns::Values, "OnlyB", entry(Def::Module(4)));
    let merged = merge_alternating(&a, &b);
    assert_eq!(merged.types["Same"].def, Def::Module(1));
    assert_eq!(merged.types["Differs"].def, Def::Unknown);
    assert_eq!(merged.values["OnlyA"].def, Def::Unknown);
    assert_eq!(merged.values["OnlyB"].def, Def::Unknown);
    assert_eq!(
        merged,
        merge_alternating(&b, &a),
        "the merge is symmetric in its defs"
    );
}

/// G162: a name a bounded workspace macro may define is withheld with that reason, and a module
/// opened twice keeps the first cause found (item macros are resolved before imports).
#[test]
fn a_bounded_macro_name_and_the_first_open_cause_are_reported() {
    let lib = "mod bounded {\n    macro_rules! make {\n        ($name:ident) => {\n            pub struct $name;\n        };\n    }\n    make!(Made);\n    fn t() {\n        Made::build();\n    }\n}\n";
    let workspace = resolve_workspace(&one_crate("src/lib.rs"), &sources(&[("src/lib.rs", lib)]));
    let causes: Vec<(String, String)> = workspace
        .withheld
        .iter()
        .map(|w| (w.callee.clone(), w.cause.clone()))
        .collect();
    assert_eq!(
        causes,
        [(
            "Made::build".into(),
            "an item-position macro whose definition names `Made` may define it here".into()
        )]
    );
}
