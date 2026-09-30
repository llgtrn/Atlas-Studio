use super::*;
use atlas_core::{ArtifactId, ContentFingerprint, RepositoryId, RevisionRef};

const ALL: [SemanticDimension; 12] = [
    SemanticDimension::Symbol,
    SemanticDimension::Type,
    SemanticDimension::FunctionIdentity,
    SemanticDimension::FunctionSignature,
    SemanticDimension::Call,
    SemanticDimension::ControlFlow,
    SemanticDimension::DataFlow,
    SemanticDimension::State,
    SemanticDimension::Effect,
    SemanticDimension::Ownership,
    SemanticDimension::Concurrency,
    SemanticDimension::Persistence,
];

fn extract(path: &str, source: &str) -> ExtractionBatch {
    let language = if path.ends_with(".ts") || path.ends_with(".tsx") {
        "typescript"
    } else {
        "javascript"
    };
    TypeScriptSemanticExtractor.extract(&ExtractionInput {
        repository: RepositoryId::new("atlas-studio"),
        revision: RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        },
        artifact: ArtifactId::new(format!("artifact:{path}")),
        artifact_path: path.to_owned(),
        source_text: source.to_owned(),
        content_fingerprint: Some(ContentFingerprint(format!("sha256:{path}"))),
        source_frontend_id: format!("atlas.source.{language}.bootstrap.v1"),
        language: language.into(),
        build_profile: None,
        scope_policy: None,
        requested_dimensions: ALL.to_vec(),
    })
}

fn functions(batch: &ExtractionBatch) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = batch
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) => Some((
                h.subject.scope.join(),
                h.subject.symbol.name.clone(),
                h.subject.declaration_kind.as_str().to_owned(),
            )),
            _ => None,
        })
        .collect();
    out.sort();
    out
}

fn name_of(batch: &ExtractionBatch, id: &SemanticRecordId) -> String {
    batch
        .observations
        .iter()
        .find_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) if h.record_id == *id => {
                Some(h.subject.symbol.name.clone())
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// (caller, callee spelling, resolved callee names, status)
fn calls(batch: &ExtractionBatch) -> Vec<(String, String, Vec<String>, EpistemicStatus)> {
    let mut out: Vec<_> = batch
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Call(h) => Some((
                name_of(batch, &h.subject.function),
                h.subject.callee_spelling.clone().unwrap_or_default(),
                h.subject
                    .callees
                    .iter()
                    .map(|c| name_of(batch, c))
                    .collect(),
                h.status,
            )),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    out
}

fn status(batch: &ExtractionBatch, dimension: SemanticDimension) -> EpistemicStatus {
    batch.obligation_for(dimension).unwrap().status
}

const MODULE: &str = r#"import { helper as h, other } from "./util";
import def from "./def";
import * as ns from "./ns";

export function run(input: string): number {
  const parsed = parse(input);
  h(parsed);
  ns.go();
  return compute(parsed, (x: number) => twice(x));
}

function parse(text: string): string[] {
  return text.split(",");
}

export const compute = async (items: string[], f: (x: number) => number) => {
  return items.map(f);
};

const twice = function (n: number) {
  return n * 2;
};

function shadowed() {}
function user() {
  const shadowed = 1;
  shadowed();
}

function reassigned() {}
reassigned = () => {};
reassigned();

class Engine {
  private count = 0;
  constructor(seed: number) {
    this.count = seed;
  }
  step(): void {
    this.bump();
    parse("a");
  }
  static make(): Engine {
    return new Engine(1);
  }
  private bump() {
    this.count += 1;
  }
}

interface Shape { area(): number }
type Id = string;
enum Color { Red }

run("x");
"#;

#[test]
fn functions_symbols_and_signatures_of_a_typescript_module_are_typed_records() {
    let batch = extract("src/main.ts", MODULE);
    assert!(batch.is_closed(&ALL));
    let fns = functions(&batch);
    let has = |scope: &str, name: &str, kind: &str| {
        fns.contains(&(scope.to_owned(), name.to_owned(), kind.to_owned()))
    };
    assert!(has("", "run", "FREE_FUNCTION"));
    assert!(has("", "parse", "FREE_FUNCTION"));
    assert!(
        has("", "compute", "FREE_FUNCTION"),
        "an arrow bound to a const"
    );
    assert!(
        has("", "twice", "FREE_FUNCTION"),
        "a function expression bound to a const"
    );
    assert!(has("Engine", "constructor", "INHERENT_METHOD"));
    assert!(has("Engine", "step", "INHERENT_METHOD"));
    assert!(
        has("Engine", "make", "ASSOCIATED_FUNCTION"),
        "a static method"
    );
    assert!(has("Engine", "bump", "INHERENT_METHOD"));
    assert!(has("fn run", "{closure@9:25}", "CLOSURE"), "{fns:?}");
    assert!(
        has("", "{module}", "FREE_FUNCTION"),
        "top-level calls have a region"
    );
    // An interface method signature has no body: not a function.
    assert!(!fns.iter().any(|(_, name, _)| name == "area"));
    let signature = batch
        .observations
        .iter()
        .find_map(|o| match o {
            SemanticObservation::FunctionSignature(h)
                if h.subject.function.symbol.name == "compute" =>
            {
                Some(h.subject.clone())
            }
            _ => None,
        })
        .unwrap();
    assert!(signature.is_async);
    assert_eq!(signature.visibility, "export");
    let parameters: Vec<(String, String)> = signature
        .parameters
        .iter()
        .map(|p| (p.name.clone(), p.type_identity.name.clone()))
        .collect();
    assert_eq!(
        parameters,
        [
            ("items".to_owned(), "string[]".to_owned()),
            ("f".to_owned(), "(x: number) => number".to_owned())
        ]
    );
    let symbols: BTreeSet<(String, String, String)> = batch
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Symbol(h) => Some((
                h.subject.scope.join(),
                h.subject.name.clone(),
                h.subject.role.as_str().to_owned(),
            )),
            _ => None,
        })
        .collect();
    for (scope, name, role) in [
        ("", "Engine", "DEFINITION"),
        ("", "Shape", "DEFINITION"),
        ("", "Id", "DEFINITION"),
        ("", "Color", "DEFINITION"),
        ("import ./util", "h", "DECLARATION"),
        ("import ./util", "other", "DECLARATION"),
        ("import ./def", "def", "DECLARATION"),
        ("import ./ns", "ns", "DECLARATION"),
    ] {
        assert!(
            symbols.contains(&(scope.to_owned(), name.to_owned(), role.to_owned())),
            "{scope} {name} {role}: {symbols:?}"
        );
    }
    // An error-free parse: declarations are exhaustive; CALL is not; the rest is UNSUPPORTED.
    assert_eq!(
        status(&batch, SemanticDimension::FunctionIdentity),
        EpistemicStatus::Observed
    );
    assert_eq!(
        status(&batch, SemanticDimension::Symbol),
        EpistemicStatus::Observed
    );
    assert_eq!(
        status(&batch, SemanticDimension::Call),
        EpistemicStatus::Unknown
    );
    for dimension in [
        SemanticDimension::Type,
        SemanticDimension::ControlFlow,
        SemanticDimension::DataFlow,
        SemanticDimension::Effect,
    ] {
        assert_eq!(
            status(&batch, dimension),
            EpistemicStatus::Unsupported,
            "{dimension:?}"
        );
    }
}

#[test]
fn a_call_resolves_only_by_an_unambiguous_lexical_binding_in_the_same_file() {
    let batch = extract("src/main.ts", MODULE);
    let got = calls(&batch);
    let find = |caller: &str, spelling: &str| {
        got.iter()
            .find(|(c, s, _, _)| c == caller && s == spelling)
            .unwrap_or_else(|| panic!("{caller} -> {spelling}: {got:#?}"))
            .clone()
    };
    // A module-level function bound once: resolved, DERIVED.
    assert_eq!(find("run", "parse").2, ["parse"]);
    assert_eq!(find("run", "parse").3, EpistemicStatus::Derived);
    assert_eq!(find("run", "compute").2, ["compute"]);
    assert_eq!(find("{closure@9:25}", "twice").2, ["twice"]);
    assert_eq!(find("step", "parse").2, ["parse"]);
    assert_eq!(find("{module}", "run").2, ["run"]);
    assert_eq!(find("make", "Engine::constructor").2, ["constructor"]);
    // Imported, member, `this` and shadowed callees stay unresolved and OBSERVED only.
    for (caller, spelling) in [
        ("run", "h"),
        ("run", ".go"),
        ("step", ".bump"),
        ("user", "shadowed"),
        ("{module}", "reassigned"),
        ("run", ".split"),
    ] {
        let call = got.iter().find(|(c, s, _, _)| c == caller && s == spelling);
        if let Some(call) = call {
            assert!(call.2.is_empty(), "{caller} -> {spelling} was resolved");
            assert_eq!(call.3, EpistemicStatus::Observed);
        } else {
            assert_eq!(
                spelling, ".split",
                "{caller} -> {spelling} missing: {got:#?}"
            );
        }
    }
    assert!(got.iter().any(|(c, s, _, _)| c == "parse" && s == ".split"));
}

#[test]
fn javascript_jsx_and_tsx_files_are_parsed_with_their_grammar() {
    let js = extract(
        "tools/run.mjs",
        "export function main() {\n  return helper(1);\n}\nfunction helper(x) { return x; }\nconst obj = { go() { main(); } };\nmain();\n",
    );
    let fns = functions(&js);
    assert!(fns.contains(&("".into(), "main".into(), "FREE_FUNCTION".into())));
    // A module-level object method is a closure of the module region.
    assert!(
        fns.contains(&(
            "fn {module}".into(),
            "{closure@5:14}".into(),
            "CLOSURE".into()
        )),
        "{fns:?}"
    );
    assert_eq!(
        calls(&js)
            .iter()
            .find(|(c, s, _, _)| c == "main" && s == "helper")
            .unwrap()
            .2,
        ["helper"]
    );
    let tsx = extract(
        "ui/App.tsx",
        "export function App(props: { name: string }) {\n  return <div onClick={() => greet(props.name)}>{props.name}</div>;\n}\nfunction greet(n: string) {}\n",
    );
    assert_eq!(
        status(&tsx, SemanticDimension::FunctionIdentity),
        EpistemicStatus::Observed
    );
    assert!(functions(&tsx).iter().any(|(_, name, _)| name == "App"));
    assert!(
        calls(&tsx)
            .iter()
            .any(|(c, s, callees, _)| c.starts_with("{closure@")
                && s == "greet"
                && callees == &["greet"])
    );
}

#[test]
fn a_syntax_error_keeps_observations_but_nothing_is_exhaustive() {
    let batch = extract(
        "src/broken.ts",
        "function ok() { go(); }\nfunction broken( {\n",
    );
    assert!(batch.is_closed(&ALL));
    assert!(functions(&batch).iter().any(|(_, name, _)| name == "ok"));
    for dimension in SUPPORTED_DIMENSIONS {
        assert_eq!(
            status(&batch, *dimension),
            EpistemicStatus::Unknown,
            "{dimension:?}"
        );
    }
    assert!(
        batch
            .diagnostics
            .iter()
            .any(|d| d.message.contains("syntax errors"))
    );
}

#[test]
fn an_empty_file_proves_absence_and_extraction_is_deterministic() {
    let empty = extract("src/empty.ts", "// nothing\n");
    let identity = empty
        .obligation_for(SemanticDimension::FunctionIdentity)
        .unwrap();
    assert_eq!(identity.status, EpistemicStatus::Observed);
    assert!(identity.observation_ids.is_empty() && !identity.evidence_refs.is_empty());
    assert_eq!(
        extract("src/main.ts", MODULE),
        extract("src/main.ts", MODULE)
    );
}

#[test]
fn every_closure_has_an_enclosing_region() {
    // Module-level callbacks, class field initializers and anonymous classes inside closures.
    let batch = extract(
        "src/regions.ts",
        "export const a = [1].map((x) => x + 1);\nclass K {\n  f = () => go();\n}\nfunction go() {\n  return () => {\n    const C = class {\n      g = () => go();\n    };\n  };\n}\n",
    );
    let fns = functions(&batch);
    let names: BTreeSet<String> = fns
        .iter()
        .filter(|(_, _, kind)| kind != "CLOSURE")
        .map(|(scope, name, _)| {
            if scope.is_empty() {
                format!("fn {name}")
            } else {
                format!("{scope}::fn {name}")
            }
        })
        .collect();
    let mut closure_scopes: BTreeSet<String> = BTreeSet::new();
    for (scope, name, kind) in &fns {
        if kind == "CLOSURE" {
            // The closure's scope ends in `fn <region>`, and that region exists.
            assert!(
                scope.rsplit("::").next().unwrap().starts_with("fn "),
                "{name}: {scope}"
            );
            closure_scopes.insert(scope.clone());
        }
    }
    for scope in &closure_scopes {
        let region_is_a_closure = scope
            .rsplit("::")
            .next()
            .unwrap()
            .starts_with("fn {closure@");
        assert!(
            names.contains(scope) || region_is_a_closure,
            "{scope}: no such region in {names:?}"
        );
    }
    assert!(closure_scopes.contains("fn {module}"));
    assert!(closure_scopes.contains("fn go"));
}

// --- G186 CALL site identity: every call of a chain is its own call site -----------------------
// A call site was anchored at its expression's first token, which every call of a chain shares
// (`inner().m()` and `inner()` both start at `inner`), so the chain collapsed into one CALL record:
// the outer call kept the identity and the inner calls were silently erased (1,018 of the donor's
// 30,118 call and `new` sites, 21 of the 210 in Atlas's own browser scripts). The anchor is now
// the callee's name token, as `CallSiteIdentity::span` (G74) and the Rust extractor define it
// (ADR 0098).

/// (caller, callee spelling, line, column, the token at the anchor), in source order.
fn call_anchors(
    batch: &ExtractionBatch,
    source: &str,
) -> Vec<(String, String, usize, usize, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<_> = batch
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Call(h) => {
                let span = &h.subject.span;
                let rest = &lines[span.line - 1][span.column..];
                let word: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$' || *c == '#')
                    .collect();
                let token = if word.is_empty() {
                    rest.chars().next().map(String::from).unwrap_or_default()
                } else {
                    word
                };
                Some((
                    name_of(batch, &h.subject.function),
                    h.subject.callee_spelling.clone().unwrap_or_default(),
                    span.line,
                    span.column,
                    token,
                ))
            }
            _ => None,
        })
        .collect();
    out.sort_by_key(|c| (c.2, c.3));
    out
}

const CHAINS: &str = r#"function inner() { return { m() { return 1; } }; }
function viaChain() { return inner().m(); }
async function hover(page, target) { await page.locator(target).first().hover(); }
function viaNew() { return new Box().open(); }
class Box { open() { return 1; } }
(async () => { await 1; })().catch(fail);
function curried(a) { return a()(); }
function others(f, k, o, ns) { (f)(1); o[k](); new ns.C(); new (f())(); new Box; o?.m(); f()`t`; new (f()); }
class Kid extends Box { #p() { return 1; } constructor() { super(); this.#p(); import("x"); } }
"#;

#[test]
fn every_call_of_a_chain_is_its_own_call_site_anchored_at_the_callee_name() {
    for path in [
        "src/chains.js",
        "src/chains.ts",
        "src/chains.tsx",
        "src/chains.mjs",
    ] {
        let batch = extract(path, CHAINS);
        let got: Vec<(String, usize, usize, String)> = call_anchors(&batch, CHAINS)
            .into_iter()
            .map(|(caller, _, line, column, token)| (caller, line, column, token))
            .collect();
        let want: Vec<(&str, usize, usize, &str)> = vec![
            ("viaChain", 2, 29, "inner"),
            ("viaChain", 2, 37, "m"),
            ("hover", 3, 48, "locator"),
            ("hover", 3, 64, "first"),
            ("hover", 3, 72, "hover"),
            ("viaNew", 4, 31, "Box"),
            ("viaNew", 4, 37, "open"),
            // The IIFE's callee is not a name: anchored at its own argument list.
            ("{module}", 6, 26, "("),
            ("{module}", 6, 29, "catch"),
            ("curried", 7, 29, "a"),
            // `a()()`: the outer call's callee is a call, anchored at its own `(`.
            ("curried", 7, 32, "("),
            ("others", 8, 34, "("),
            ("others", 8, 43, "("),
            ("others", 8, 54, "C"),
            ("others", 8, 64, "f"),
            // `new (f())()`: a constructor that is not a name, anchored at its argument list.
            ("others", 8, 68, "("),
            ("others", 8, 76, "Box"),
            ("others", 8, 84, "m"),
            ("others", 8, 89, "f"),
            // A call tagging a template: the template is its argument list.
            ("others", 8, 92, "`"),
            // `new (f())`: neither a name nor an argument list, anchored at its `new`.
            ("others", 8, 97, "new"),
            ("others", 8, 102, "f"),
            ("constructor", 9, 59, "super"),
            ("constructor", 9, 73, "#p"),
            ("constructor", 9, 79, "import"),
        ];
        let want: Vec<(String, usize, usize, String)> = want
            .into_iter()
            .map(|(c, l, col, t)| (c.to_owned(), l, col, t.to_owned()))
            .collect();
        assert_eq!(got, want, "{path}");
        // Every anchor is a distinct identity.
        let ids: BTreeSet<&str> = batch
            .observations
            .iter()
            .filter(|o| matches!(o, SemanticObservation::Call(_)))
            .map(|o| o.record_id().as_str())
            .collect();
        assert_eq!(ids.len(), want.len(), "{path}");
    }
}

/// The minimal G186 repro: `inner().m()` recorded only `.m`, so `inner` had no caller at all.
#[test]
fn the_inner_call_of_a_chain_is_resolved_and_the_outer_stays_unresolved() {
    let batch = extract(
        "src/mini.js",
        "function target() { return 1; }\nfunction inner() { return { m() { return target(); } }; }\nfunction viaChain() {\n  return inner().m();\n}\n",
    );
    let got = calls(&batch);
    let via: Vec<_> = got.iter().filter(|c| c.0 == "viaChain").collect();
    assert_eq!(via.len(), 2, "{got:#?}");
    assert_eq!(
        (via[0].1.as_str(), via[0].2.clone(), via[0].3),
        (".m", Vec::<String>::new(), EpistemicStatus::Observed)
    );
    assert_eq!(
        (via[1].1.as_str(), via[1].2.clone(), via[1].3),
        ("inner", vec!["inner".to_owned()], EpistemicStatus::Derived)
    );
}

/// A call whose callee is a bare name keeps the identity it had before G186: its name token is
/// the expression's first token.
#[test]
fn a_bare_call_keeps_its_identity() {
    let source =
        "function f() { return 1; }\nfunction g(x) {\n  f();\n  return  f(x) + f(f());\n}\n";
    let batch = extract("src/bare.js", source);
    let spans: Vec<(usize, usize)> = call_anchors(&batch, source)
        .into_iter()
        .map(|c| (c.2, c.3))
        .collect();
    // The start of each call expression, counted by hand.
    assert_eq!(spans, [(3, 2), (4, 10), (4, 17), (4, 19)]);
    let caller = batch
        .observations
        .iter()
        .find_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) if h.subject.symbol.name == "g" => {
                Some(h.record_id.clone())
            }
            _ => None,
        })
        .unwrap();
    let first = batch
        .observations
        .iter()
        .find_map(|o| match o {
            SemanticObservation::Call(h) if h.subject.span.line == 3 => Some(h),
            _ => None,
        })
        .unwrap();
    let pre_g186 = CallSiteIdentity {
        span: SourceSpan {
            path: "src/bare.js".into(),
            line: 3,
            column: 2,
        },
        function: caller,
        ..first.subject.clone()
    };
    assert_eq!(
        first.record_id,
        SemanticRecordId::new(SemanticDimension::Call, &pre_g186.identity_key())
    );
}

/// Every call and `new` expression of Atlas's own browser scripts is one CALL record, counted by
/// an enumeration of every syntax node independent of the extractor's walker.
#[test]
fn every_call_of_the_browser_scripts_is_a_call_record() {
    for (path, source) in [
        (
            "adapter/src/browser/observe.js",
            include_str!("../../browser/observe.js"),
        ),
        (
            "adapter/src/browser/measure_layout.js",
            include_str!("../../browser/measure_layout.js"),
        ),
        (
            "adapter/src/browser/webdriver.js",
            include_str!("../../browser/webdriver.js"),
        ),
    ] {
        let mut parser = Parser::new();
        parser.set_language(&grammar(path)).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut expressions = 0;
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(node.kind(), "call_expression" | "new_expression") {
                expressions += 1;
            }
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
        let batch = extract(path, source);
        let records = batch
            .observations
            .iter()
            .filter(|o| matches!(o, SemanticObservation::Call(_)))
            .count();
        assert!(expressions > 0, "{path}");
        assert_eq!(records, expressions, "{path}");
    }
}
