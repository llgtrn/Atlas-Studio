use super::*;

fn e(descriptor: &str, signature: &str, body: &str) -> EntityState {
    EntityState {
        descriptor: descriptor.into(),
        path: "core/src/a.rs".into(),
        signature: signature.into(),
        body: body.into(),
        visibility: "pub".into(),
    }
}

fn kinds(changes: &[EntityChange]) -> Vec<String> {
    changes.iter().map(EntityChange::item).collect()
}

#[test]
fn descriptors_follow_cargo_layout_and_ignore_positions() {
    assert_eq!(
        descriptor(
            "core",
            "core/src/language/adl/census.rs",
            &[],
            "entity_name"
        ),
        "core language/adl/census/entity_name()."
    );
    assert_eq!(
        descriptor("core", "core/src/lib.rs", &["tests".into()], "root"),
        "core tests/root()."
    );
    assert_eq!(
        descriptor(
            "runtime",
            "runtime/src/census/mod.rs",
            &["impl:Stream".into()],
            "next"
        ),
        "runtime census/`impl:Stream`/next()."
    );
    assert_eq!(
        descriptor("apps/cli", "apps/cli/src/main.rs", &[], "main"),
        "`apps/cli` main()."
    );
    // A file outside src/ is its own quoted namespace, never a module.
    assert_eq!(
        descriptor("adapter", "adapter/build.rs", &[], "main"),
        "adapter `build.rs`/main()."
    );
    assert_eq!(descriptor("", "src/lib.rs", &[], "f"), ". f().");
    // Same-named functions in different modules never share a descriptor.
    assert_ne!(
        descriptor("runtime", "runtime/src/lib.rs", &["tests".into()], "root"),
        descriptor(
            "runtime",
            "runtime/src/certificate.rs",
            &["tests".into()],
            "root"
        )
    );
    let manifests: BTreeSet<String> = ["core".to_owned(), "apps/cli".to_owned()].into();
    assert_eq!(
        package_dir("apps/cli/src/main.rs", &manifests),
        Some("apps/cli")
    );
    assert_eq!(package_dir("core/src/a/b.rs", &manifests), Some("core"));
    assert_eq!(package_dir("docs/x.rs", &manifests), None);
}

#[test]
fn identical_sets_have_no_changes_and_order_does_not_matter() {
    let a = vec![e("p m/f().", "s1", "b1"), e("p m/g().", "s2", "b2")];
    let mut b = a.clone();
    b.reverse();
    assert!(correspond(&a, &b).is_empty());
}

#[test]
fn same_descriptor_reports_what_changed() {
    let before = [e("p m/f().", "s1", "b1")];
    let mut vis = e("p m/f().", "s1", "b1");
    vis.visibility = "pub(crate)".into();
    for (after, evidence) in [
        (e("p m/f().", "s2", "b1"), "signature"),
        (e("p m/f().", "s1", "b2"), "body"),
        (e("p m/f().", "s2", "b2"), "signature,body"),
        (vis, "visibility"),
    ] {
        let changes = correspond(&before, &[after]);
        assert_eq!(kinds(&changes), ["CHANGED p m/f()."]);
        assert_eq!(changes[0].evidence, evidence);
    }
}

#[test]
fn renames_and_moves_need_identical_signature_and_body() {
    let before = [e("p m/f().", "s", "b")];
    let cases = [
        ("p m/g().", "RENAMED p m/f(). -> p m/g()."),
        ("p n/f().", "MOVED p m/f(). -> p n/f()."),
        ("p n/g().", "MOVED_RENAMED p m/f(). -> p n/g()."),
        ("q m/f().", "MOVED p m/f(). -> q m/f()."),
    ];
    for (to, expected) in cases {
        assert_eq!(kinds(&correspond(&before, &[e(to, "s", "b")])), [expected]);
    }
    // Any fingerprint difference: no correspondence is claimed.
    assert_eq!(
        kinds(&correspond(&before, &[e("p m/g().", "s", "b2")])),
        ["DELETED p m/f().", "CREATED p m/g()."]
    );
    assert_eq!(
        kinds(&correspond(&before, &[e("p m/g().", "s2", "b")])),
        ["DELETED p m/f().", "CREATED p m/g()."]
    );
    // A visibility change rides along as evidence; it does not break the match.
    let mut moved = e("p n/f().", "s", "b");
    moved.visibility = "pub(crate)".into();
    let changes = correspond(&before, &[moved]);
    assert_eq!(kinds(&changes), ["MOVED p m/f(). -> p n/f()."]);
    assert!(changes[0].evidence.contains("visibility pub -> pub(crate)"));
}

#[test]
fn ties_are_ambiguous_never_forced() {
    // Two new entities share the deleted one's fingerprints.
    let changes = correspond(
        &[e("p m/f().", "s", "b")],
        &[e("p m/g().", "s", "b"), e("p m/h().", "s", "b")],
    );
    assert_eq!(
        kinds(&changes),
        ["AMBIGUOUS p m/f(). -> p m/g(). | p m/h()."]
    );
    // Two deleted entities share one new entity's fingerprints.
    let changes = correspond(
        &[e("p m/f().", "s", "b"), e("p m/g().", "s", "b")],
        &[e("p m/h().", "s", "b")],
    );
    assert_eq!(
        kinds(&changes),
        ["AMBIGUOUS p m/f(). | p m/g(). -> p m/h()."]
    );
    // A duplicated descriptor whose members change is ambiguous; unchanged it is not.
    let dup = [e("p m/f().", "s1", "b1"), e("p m/f().", "s2", "b2")];
    assert!(correspond(&dup, &dup).is_empty());
    let changes = correspond(
        &dup,
        &[e("p m/f().", "s1", "b1"), e("p m/f().", "s2", "b3")],
    );
    assert_eq!(kinds(&changes), ["AMBIGUOUS p m/f()."]);
}

#[test]
fn bodiless_declarations_are_never_matched_by_fingerprint() {
    assert_eq!(
        kinds(&correspond(
            &[e("p m/f().", "s", "-")],
            &[e("p m/g().", "s", "-")]
        )),
        ["DELETED p m/f().", "CREATED p m/g()."]
    );
}

#[test]
fn a_split_is_a_change_plus_a_creation_not_a_claimed_split() {
    let changes = correspond(
        &[e("p m/f().", "s", "b")],
        &[
            e("p m/f().", "s", "b-shorter"),
            e("p m/helper().", "s2", "b-extracted"),
        ],
    );
    assert_eq!(
        kinds(&changes),
        ["CHANGED p m/f().", "CREATED p m/helper()."]
    );
}

#[test]
fn correspondence_is_a_pure_function_of_the_two_sets() {
    let before = vec![
        e("p m/a().", "s", "b1"),
        e("p m/b().", "s", "b2"),
        e("p m/c().", "s", "b3"),
        e("p m/d().", "s", "b4"),
    ];
    let after = vec![
        e("p m/a().", "s", "b1x"),
        e("p n/b().", "s", "b2"),
        e("p m/z().", "s", "b3"),
        e("p m/new().", "s", "b9"),
    ];
    let expected = correspond(&before, &after);
    assert_eq!(
        kinds(&expected),
        [
            "CHANGED p m/a().",
            "RENAMED p m/c(). -> p m/z().",
            "MOVED p m/b(). -> p n/b().",
            "DELETED p m/d().",
            "CREATED p m/new()."
        ]
    );
    let (mut b, mut a) = (before.clone(), after.clone());
    b.reverse();
    a.rotate_left(1);
    assert_eq!(correspond(&b, &a), expected);
}

mod signature {
    use crate::SourceSpan;
    use crate::semantic::function::{
        FunctionDeclarationKind, FunctionIdentity, FunctionOwner, FunctionParameter,
        FunctionSignature,
    };
    use crate::semantic::symbol::{SymbolIdentity, SymbolRole};
    use crate::semantic::{SemanticScope, TypeIdentity};
    use crate::{RepositoryId, RevisionRef};

    fn ty(name: &str, revision: &str) -> TypeIdentity {
        TypeIdentity {
            path: String::new(),
            repository: RepositoryId::new("r"),
            revision: RevisionRef {
                kind: "git".into(),
                value: revision.into(),
            },
            scope: SemanticScope::new(Vec::<String>::new()),
            name: name.into(),
            canonical: None,
        }
    }

    fn sig(name: &str, path: &str, line: usize, revision: &str) -> FunctionSignature {
        let rev = RevisionRef {
            kind: "git".into(),
            value: revision.into(),
        };
        let scope = SemanticScope::new(["m"]);
        FunctionSignature {
            function: FunctionIdentity {
                repository: RepositoryId::new("r"),
                revision: rev.clone(),
                language: "rust".into(),
                scope: scope.clone(),
                symbol: SymbolIdentity {
                    path: String::new(),
                    repository: RepositoryId::new("r"),
                    revision: rev,
                    scope,
                    name: name.into(),
                    role: SymbolRole::Definition,
                    documentation: None,
                    declaration: None,
                },
                span: SourceSpan {
                    path: path.into(),
                    line,
                    column: 1,
                },
                generated: false,
                declaration_kind: FunctionDeclarationKind::FreeFunction,
                owner: FunctionOwner::none(),
                generics: Vec::new(),
            },
            parameters: vec![FunctionParameter {
                name: "x".into(),
                type_identity: ty("u8", revision),
            }],
            return_type: Some(ty("u8", revision)),
            generics: Vec::new(),
            abi: None,
            visibility: "pub".into(),
            is_async: false,
            is_unsafe: false,
            is_extern: false,
            body_fingerprint: None,
        }
    }

    #[test]
    fn the_signature_fingerprint_survives_rename_move_and_revision_only() {
        let base = super::signature_fingerprint(&sig("f", "core/src/a.rs", 10, "rev1"));
        for (same, why) in [
            (sig("g", "core/src/a.rs", 10, "rev1"), "renamed"),
            (sig("f", "core/src/b.rs", 99, "rev1"), "moved"),
            (sig("f", "core/src/a.rs", 10, "rev2"), "another revision"),
        ] {
            assert_eq!(super::signature_fingerprint(&same), base, "{why}");
        }
        let mut renamed_param = sig("f", "core/src/a.rs", 10, "rev1");
        renamed_param.parameters[0].name = "y".into();
        assert_eq!(super::signature_fingerprint(&renamed_param), base);
        let mut visibility = sig("f", "core/src/a.rs", 10, "rev1");
        visibility.visibility = "pub(crate)".into();
        assert_eq!(
            super::signature_fingerprint(&visibility),
            base,
            "tracked apart"
        );

        let mut param_type = sig("f", "core/src/a.rs", 10, "rev1");
        param_type.parameters[0].type_identity = ty("u16", "rev1");
        let mut ret = sig("f", "core/src/a.rs", 10, "rev1");
        ret.return_type = None;
        let mut asynchronous = sig("f", "core/src/a.rs", 10, "rev1");
        asynchronous.is_async = true;
        let mut kind = sig("f", "core/src/a.rs", 10, "rev1");
        kind.function.declaration_kind = FunctionDeclarationKind::InherentMethod;
        for (other, why) in [
            (param_type, "parameter type"),
            (ret, "return type"),
            (asynchronous, "async"),
            (kind, "declaration kind"),
        ] {
            assert_ne!(super::signature_fingerprint(&other), base, "{why}");
        }
    }
}
