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

fn declared(manifest: &str, name: &str, source: &str) -> PackageDeclaration {
    PackageDeclaration {
        manifest: manifest.into(),
        name: name.into(),
        source: Some(source.into()),
        ..PackageDeclaration::default()
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
    let packages = workspace_packages(
        &[
            declared("packages/system/package.json", "@x/system", "src/index.ts"),
            // Two manifests declaring one name name nothing.
            declared("a/package.json", "@x/twice", "i.ts"),
            declared("b/package.json", "@x/twice", "i.ts"),
        ],
        &files,
    );
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
    let packages = workspace_packages(
        &[declared(
            "packages/system/package.json",
            "@x/system",
            "src/index.ts",
        )],
        &files.keys().cloned().collect(),
    );
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

#[test]
fn compiled_entries_map_back_to_sources_only_through_declared_directories() {
    // G160: a package declaring only compiled entries is entered at the one source its sibling
    // tsconfig's rootDir/outDir says they are emitted from; never without both directories, never
    // when the entries disagree or a source is missing.
    let files: BTreeSet<String> = [
        "shared/src/index.ts",
        "shared/src/other.ts",
        "web/src/main.tsx",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let compiled = |outputs: &[&str], root: Option<&str>, out: Option<&str>| PackageDeclaration {
        manifest: "shared/package.json".into(),
        name: "shared".into(),
        source: None,
        outputs: outputs.iter().map(|o| (*o).to_owned()).collect(),
        root_dir: root.map(str::to_owned),
        out_dir: out.map(str::to_owned),
    };
    let entry = |d: PackageDeclaration| package_entry(&d, &files);
    assert_eq!(
        entry(compiled(
            &["dist/index.d.ts", "./dist/index.js"],
            Some("src"),
            Some("./dist/")
        ))
        .as_deref(),
        Some("shared/src/index.ts")
    );
    assert_eq!(
        entry(compiled(&["dist/index.js"], None, Some("dist"))),
        None,
        "no declared rootDir"
    );
    assert_eq!(
        entry(compiled(&["dist/index.js"], Some("src"), None)),
        None,
        "no declared outDir"
    );
    assert_eq!(
        entry(compiled(
            &["dist/index.js", "dist/other.js"],
            Some("src"),
            Some("dist")
        )),
        None,
        "entries that disagree"
    );
    assert_eq!(
        entry(compiled(
            &["dist/index.js", "dist/missing.d.ts"],
            Some("src"),
            Some("dist")
        )),
        None,
        "a source that is not in the inventory"
    );
    assert_eq!(
        entry(compiled(&["lib/index.js"], Some("src"), Some("dist"))),
        None,
        "an entry outside outDir"
    );
    let web = PackageDeclaration {
        manifest: "web/package.json".into(),
        name: "web".into(),
        source: None,
        outputs: vec!["build/main.js".into()],
        root_dir: Some("src".into()),
        out_dir: Some("build".into()),
    };
    assert_eq!(
        entry(web).as_deref(),
        Some("web/src/main.tsx"),
        "a .js entry is emitted from .tsx when no .ts exists"
    );
}
