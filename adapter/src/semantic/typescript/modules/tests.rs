use super::*;
use atlas_core::{ArtifactId, RepositoryId, RevisionRef, SemanticDimension};

fn input(path: &str, source: &str) -> ExtractionInput {
    ExtractionInput {
        repository: RepositoryId::new("atlas-studio"),
        revision: RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        },
        artifact: ArtifactId::new(format!("artifact:{path}")),
        artifact_path: path.to_owned(),
        source_text: source.to_owned(),
        content_fingerprint: None,
        source_frontend_id: "atlas.source.typescript.bootstrap.v1".into(),
        language: "typescript".into(),
        build_profile: None,
        scope_policy: None,
        requested_dimensions: vec![SemanticDimension::FunctionIdentity, SemanticDimension::Call],
    }
}

fn facts(files: &[(&str, &str)]) -> BTreeMap<String, ModuleFacts> {
    files
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_owned(),
                module_facts(&input(path, source)).unwrap(),
            )
        })
        .collect()
}

#[test]
fn imports_exports_and_re_exports_are_read_as_declared() {
    let f = module_facts(&input(
        "src/a.ts",
        "import d, { x, y as z } from './b';\nimport type { T } from './t';\nimport * as ns from './n';\nimport { type U, v } from './u';\nexport function f() {}\nexport const g = () => 1;\nfunction h() {}\nexport { h as k, h };\nexport { p as q } from './p';\nexport * from './star';\nexport * as grouped from './g';\nexport default h;\nexport type { W } from './w';\n",
    ))
    .unwrap();
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        f.imports,
        BTreeMap::from([
            ("d".to_owned(), pair("./b", "default")),
            ("x".to_owned(), pair("./b", "x")),
            ("z".to_owned(), pair("./b", "y")),
            ("v".to_owned(), pair("./u", "v")),
        ]),
        "type-only and namespace imports bind no value"
    );
    assert_eq!(
        f.exports,
        BTreeMap::from([
            pair("f", "f"),
            pair("g", "g"),
            pair("k", "h"),
            pair("h", "h"),
            pair("default", "h"),
        ])
    );
    assert_eq!(
        f.re_exports,
        BTreeMap::from([("q".to_owned(), pair("./p", "p"))])
    );
    assert_eq!(f.stars, ["./star"], "`export * as ns` is not a star export");
    assert_eq!(
        f.functions.keys().collect::<Vec<_>>(),
        ["f", "g", "h"],
        "module-level functions bound once"
    );
}

#[test]
fn specifiers_name_files_in_a_fixed_order_and_packages_by_declared_source() {
    let files: BTreeSet<String> = [
        "packages/system/src/index.ts",
        "packages/system/src/utils/index.ts",
        "packages/system/src/utils/edge.ts",
        "packages/react/src/a.tsx",
        "packages/react/src/b.ts",
        "packages/react/src/b.js",
        "packages/react/src/style.css",
        "a/i.ts",
        "b/i.ts",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let packages = workspace_packages(&[
        (
            "packages/system/package.json".into(),
            "@x/system".into(),
            "src/index.ts".into(),
        ),
        // Two manifests declaring one name name nothing.
        ("a/package.json".into(), "@x/twice".into(), "i.ts".into()),
        ("b/package.json".into(), "@x/twice".into(), "i.ts".into()),
    ]);
    let r = |from: &str, s: &str| resolve_specifier(from, s, &files, &packages);
    assert_eq!(
        r("packages/system/src/index.ts", "./utils").as_deref(),
        Some("packages/system/src/utils/index.ts")
    );
    assert_eq!(
        r("packages/system/src/utils/index.ts", "./edge.js").as_deref(),
        Some("packages/system/src/utils/edge.ts"),
        "a .js specifier names its .ts source"
    );
    assert_eq!(
        r("packages/react/src/a.tsx", "./b").as_deref(),
        Some("packages/react/src/b.ts"),
        ".ts comes before .js"
    );
    assert_eq!(
        r("packages/react/src/a.tsx", "@x/system").as_deref(),
        Some("packages/system/src/index.ts")
    );
    for (specifier, why) in [
        ("./style.css", "not a script"),
        ("@x/system/utils", "a package subpath"),
        ("@x/twice", "a name declared twice"),
        ("react", "a registry package"),
        ("../../../../../x", "above the root"),
    ] {
        assert_eq!(r("packages/react/src/a.tsx", specifier), None, "{why}");
    }
}

#[test]
fn imports_bind_through_exports_re_exports_and_unambiguous_stars() {
    let files = facts(&[
        (
            "packages/system/src/index.ts",
            "export * from './utils';\nexport * from './other';\nexport { renamed as alias } from './utils/edge';\n",
        ),
        (
            "packages/system/src/utils/index.ts",
            "export * from './edge';\n",
        ),
        (
            "packages/system/src/utils/edge.ts",
            "export function getPath() {}\nexport function renamed() {}\nexport function clash() {}\nfunction local() {}\nexport default local;\n",
        ),
        (
            "packages/system/src/other.ts",
            "export function clash() {}\nlet mutable = () => 1;\nmutable = () => 2;\nexport { mutable };\n",
        ),
        (
            "packages/react/src/edge.tsx",
            "import { getPath, alias, clash, mutable, missing } from '@x/system';\nimport byDefault from '../../system/src/utils/edge';\nimport { getPath as twice } from '@x/system';\n",
        ),
        (
            "packages/react/src/cycle.ts",
            "export { a } from './cycle2';\nimport { a as b } from './cycle';\n",
        ),
        (
            "packages/react/src/cycle2.ts",
            "export { a } from './cycle';\n",
        ),
    ]);
    let packages = workspace_packages(&[(
        "packages/system/package.json".into(),
        "@x/system".into(),
        "src/index.ts".into(),
    )]);
    let bindings = resolve_imports(&files, &packages);
    let edge = &files["packages/system/src/utils/edge.ts"].functions;
    let got: BTreeMap<(String, String), SemanticRecordId> = bindings
        .iter()
        .map(|b| ((b.path.clone(), b.local.clone()), b.function.clone()))
        .collect();
    let at = |local: &str| got.get(&("packages/react/src/edge.tsx".to_owned(), local.to_owned()));
    assert_eq!(
        at("getPath"),
        Some(&edge["getPath"]),
        "through two star exports"
    );
    assert_eq!(
        at("alias"),
        Some(&edge["renamed"]),
        "through a renaming re-export"
    );
    assert_eq!(
        at("byDefault"),
        Some(&edge["local"]),
        "a default export by name"
    );
    assert_eq!(at("twice"), Some(&edge["getPath"]), "an aliased import");
    assert_eq!(at("clash"), None, "two star exports provide it: ambiguous");
    assert_eq!(at("mutable"), None, "an assigned binding fixes nothing");
    assert_eq!(at("missing"), None, "no module provides it");
    assert!(
        !bindings.iter().any(|b| b.path.contains("cycle")),
        "a cycle resolves to nothing"
    );
    let chain = &bindings
        .iter()
        .find(|b| b.local == "getPath")
        .unwrap()
        .chain;
    assert_eq!(
        chain,
        &[
            "packages/system/src/index.ts",
            "packages/system/src/utils/index.ts",
            "packages/system/src/utils/edge.ts"
        ]
    );
}
