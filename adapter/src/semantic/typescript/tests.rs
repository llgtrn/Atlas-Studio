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

/// (scope, name, role) of every SYMBOL record.
fn symbols(batch: &ExtractionBatch) -> BTreeSet<(String, String, String)> {
    batch
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
        .collect()
}

fn definitions(batch: &ExtractionBatch) -> BTreeSet<(String, String)> {
    symbols(batch)
        .into_iter()
        .filter(|(_, _, role)| role == "DEFINITION")
        .map(|(scope, name, _)| (scope, name))
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> BTreeSet<(String, String)> {
    expected
        .iter()
        .map(|(scope, name)| ((*scope).to_owned(), (*name).to_owned()))
        .collect()
}

/// Every identifier a declaration binds outside every function is a module-level variable:
/// destructuring patterns, loop heads, catch parameters and blocks included; function-local
/// bindings are not (G188, ADR 0101: the replay found `for (const path in localeFiles)` missing
/// while SYMBOL claimed OBSERVED).
const MODULE_BINDINGS: &str = r#"const o = {}, arr = [];
const { a, b: c, ...r } = o;
const [d, , ...e] = arr;
export let { n } = o;
const { x = 1, y: { z }, [key]: w } = o;
var [p = 1, [q]] = arr;
for (const k in o) { const inner = 1; }
for (let [kk, vv] of Object.entries(o)) {}
for (var vi of arr) {}
for (let i = 0; i < 1; i++) {}
for (assigned in o) {}
if (o) { var hoisted = 1; let blockLet = 2; }
try {} catch ({ message }) {}
try {} catch (err) {}
function local(pa, { pb }) {
  const { fa, fb } = o;
  for (const fk in o) {}
  try {} catch (fe) {}
  var fv;
}
const arrow = (qa) => { const inArrow = 1; };
arr.forEach(function (cb) { const inCallback = 1; });
class K { static { const staticLocal = 1; } method() { const inMethod = 1; } }
"#;

#[test]
fn every_module_level_binding_of_a_javascript_file_is_a_symbol_definition() {
    let batch = extract("src/bindings.js", MODULE_BINDINGS);
    let expected = pairs(&[
        ("", "o"),
        ("", "arr"),
        ("", "a"),
        ("", "c"),
        ("", "r"),
        ("", "d"),
        ("", "e"),
        ("", "n"),
        ("", "x"),
        ("", "z"),
        ("", "w"),
        ("", "p"),
        ("", "q"),
        ("", "k"),
        ("", "inner"),
        ("", "kk"),
        ("", "vv"),
        ("", "vi"),
        ("", "i"),
        ("", "hoisted"),
        ("", "blockLet"),
        ("", "message"),
        ("", "err"),
        ("", "local"),
        ("", "arrow"),
        ("", "K"),
        ("K", "staticLocal"),
        ("K", "method"),
    ]);
    assert_eq!(definitions(&batch), expected);
    assert_eq!(
        status(&batch, SemanticDimension::Symbol),
        EpistemicStatus::Observed
    );
}

#[test]
fn every_module_level_binding_of_a_typescript_file_is_a_symbol_definition() {
    let source = r#"import fs = require("fs");
import Alias = Space.Inner;
declare const ambient: number;
export const { ta, tb: [tc] }: { ta: number; tb: number[] } = make();
for (const [tk, tv] of Object.entries(fs)) {}
try {} catch (te: unknown) {}
export default class {}
enum E { A }
function f(this: Window, fp: number, { fq }: { fq: string }) {
  const [fl] = [fp];
  import local = Space.Inner;
}
"#;
    let batch = extract("src/bindings.ts", source);
    let expected = pairs(&[
        ("", "Alias"),
        ("", "ambient"),
        ("", "ta"),
        ("", "tc"),
        ("", "tk"),
        ("", "tv"),
        ("", "te"),
        ("", "{class@7:15}"),
        ("", "E"),
        ("", "f"),
    ]);
    assert_eq!(definitions(&batch), expected);
    assert!(
        symbols(&batch).contains(&(
            "import fs".to_owned(),
            "fs".to_owned(),
            "DECLARATION".to_owned()
        )),
        "{:?}",
        symbols(&batch)
    );
    assert_eq!(
        status(&batch, SemanticDimension::Symbol),
        EpistemicStatus::Observed
    );
}

/// A namespace or ambient module body declares members of another scope, which the profile does
/// not enumerate: the file keeps its records, its members are scoped under it, SYMBOL is
/// UNKNOWN, FUNCTION_IDENTITY stays exhaustive. So does a module-level `using` declaration.
#[test]
fn a_namespace_ambient_module_or_using_makes_symbol_unknown_but_keeps_the_records() {
    for (source, scope, member) in [
        (
            "export const kept = 1;\nnamespace Space { export const member = 1; }\n",
            "namespace Space",
            "member",
        ),
        (
            "export const kept = 1;\nexport module Space.Inner { function member() {} }\n",
            "namespace Space.Inner",
            "member",
        ),
        (
            "export const kept = 1;\ndeclare module \"m\" { export const member: number; }\n",
            "module \"m\"",
            "member",
        ),
        (
            "export const kept = 1;\ndeclare global { interface Window { member: number } }\n",
            "global",
            "Window",
        ),
        (
            "export const kept = 1;\nusing r2 = null;\nawait using ares = null;\n",
            "",
            "kept",
        ),
    ] {
        let batch = extract("src/space.ts", source);
        let defined = definitions(&batch);
        assert!(
            defined.contains(&(String::new(), "kept".to_owned())),
            "{source}"
        );
        assert!(
            defined.contains(&(scope.to_owned(), member.to_owned())),
            "{source}: {defined:?}"
        );
        if !scope.is_empty() {
            assert!(
                !defined.contains(&(String::new(), member.to_owned())),
                "{source}: a member is not module-level"
            );
        }
        assert_eq!(
            status(&batch, SemanticDimension::Symbol),
            EpistemicStatus::Unknown,
            "{source}"
        );
        assert_eq!(
            status(&batch, SemanticDimension::FunctionIdentity),
            EpistemicStatus::Observed,
            "{source}"
        );
    }
}

/// A bodiless ambient module declares nothing, and a `using` inside a function is not
/// module-level: SYMBOL stays exhaustive.
#[test]
fn a_bodiless_ambient_module_or_a_local_using_keeps_symbol_exhaustive() {
    for source in [
        "export const kept = 1;\ndeclare module \"foo\";\ndeclare module \"*.css\";\n",
        "export function h() { using f = res(); }\n",
    ] {
        let batch = extract("src/ok.ts", source);
        assert_eq!(
            status(&batch, SemanticDimension::Symbol),
            EpistemicStatus::Observed,
            "{source}"
        );
    }
}

/// A class method whose name is computed or a string is still a function region: its locals
/// are not module-level definitions, and its calls are not module-level calls.
#[test]
fn a_computed_method_body_is_a_region_not_module_level_code() {
    let source = "class K {\n  ['x']() { const inside = 1; try {} catch (ce) {} for (const lh of []) {} go(); }\n  'y'() {}\n}\n";
    let batch = extract("src/computed.js", source);
    assert_eq!(definitions(&batch), pairs(&[("", "K")]));
    assert!(
        functions(&batch).contains(&(
            "fn {module}".to_owned(),
            "{closure@2:2}".to_owned(),
            "CLOSURE".to_owned()
        )),
        "{:?}",
        functions(&batch)
    );
    assert_eq!(
        calls(&batch)
            .into_iter()
            .map(|(caller, spelling, _, _)| (caller, spelling))
            .collect::<Vec<_>>(),
        [("{closure@2:2}".to_owned(), "go".to_owned())]
    );
    assert_eq!(
        status(&batch, SemanticDimension::FunctionIdentity),
        EpistemicStatus::Unknown,
        "a computed-name method keeps the file partial"
    );
}

/// A computed method key is evaluated where the method is defined: its calls belong to the
/// enclosing region, in a class and in an object literal alike.
#[test]
fn a_computed_method_key_is_walked_in_the_enclosing_scope() {
    let source = "function f() {}\nclass A { [f()]() { f(); } }\nclass B { [\"s\" + f()]() {} }\nconst o = { [f()]() {} };\n";
    let batch = extract("src/keys.js", source);
    let sites: BTreeSet<(usize, usize, String, Vec<String>)> = batch
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Call(h) => Some((
                h.subject.span.line,
                h.subject.span.column,
                name_of(&batch, &h.subject.function),
                h.subject
                    .callees
                    .iter()
                    .map(|c| name_of(&batch, c))
                    .collect(),
            )),
            _ => None,
        })
        .collect();
    let site = |line, column, caller: &str| (line, column, caller.to_owned(), vec!["f".to_owned()]);
    assert_eq!(
        sites,
        BTreeSet::from([
            site(2, 11, "{module}"),
            site(2, 20, "{closure@2:10}"),
            site(3, 17, "{module}"),
            site(4, 13, "{module}"),
        ])
    );
}

/// An identifier declarator keeps the record id and anchor it had before G188: the identity is
/// (scope, name, role, path) and the declarator starts at its identifier.
#[test]
fn a_plain_module_variable_keeps_its_identity_and_anchor() {
    let batch = extract(
        "src/plain.js",
        "export const plain = 1, other = 2;\nconst { da, db: [dc] } = {};\nfor (const twice of []) {}\nconst twice = 1;\n",
    );
    let identity = |name: &str| SymbolIdentity {
        repository: RepositoryId::new("atlas-studio"),
        revision: RevisionRef {
            kind: "git".into(),
            value: "abc123".into(),
        },
        scope: SemanticScope::new(Vec::<String>::new()),
        name: name.to_owned(),
        role: SymbolRole::Definition,
        path: "src/plain.js".into(),
        documentation: None,
        declaration: None,
    };
    let anchors: BTreeMap<String, (SemanticRecordId, Option<String>)> = batch
        .observations
        .iter()
        .filter_map(|o| match o {
            SemanticObservation::Symbol(h) => Some((
                h.subject.name.clone(),
                (h.record_id.clone(), h.provenance.span.clone()),
            )),
            _ => None,
        })
        .collect();
    // A destructured binding is anchored at its own identifier.
    for (name, span) in [
        ("plain", "1:13"),
        ("other", "1:24"),
        ("da", "2:8"),
        ("dc", "2:17"),
        // Bound twice in one scope: one record, anchored at the first binding in source order.
        ("twice", "3:11"),
    ] {
        assert_eq!(
            anchors[name],
            (
                SemanticRecordId::new(SemanticDimension::Symbol, &identity(name).identity_key()),
                Some(span.to_owned())
            ),
            "{name}"
        );
    }
}

/// The binding count and the assignment check of the CALL resolver walk the same patterns: a
/// loop head binds, a destructuring or loop-head assignment writes, a computed property key in a
/// pattern is a reference, not a binding.
#[test]
fn loop_heads_and_destructuring_assignments_count_for_call_resolution() {
    let source = "function f() {}\nfunction g() {}\nfunction h() {}\nfunction k() {}\nfunction m() {}\nfunction o() {}\nfunction s() {}\nfor (const f of []) f();\n[g] = [h];\ng();\nfor (m in {}) {}\nm();\nconst { [k]: v } = {};\nk();\nh();\n[o.p] = [1];\nfor (o.q of []) {}\no();\n[s[0]] = [1];\ns();\n";
    let batch = extract("src/calls.js", source);
    let resolved: BTreeMap<String, Vec<String>> = calls(&batch)
        .into_iter()
        .map(|(_, spelling, callees, _)| (spelling, callees))
        .collect();
    assert_eq!(resolved["f"], Vec::<String>::new(), "a loop head shadows f");
    assert_eq!(
        resolved["g"],
        Vec::<String>::new(),
        "g is assigned by destructuring"
    );
    assert_eq!(
        resolved["m"],
        Vec::<String>::new(),
        "m is assigned by a loop head"
    );
    assert_eq!(resolved["k"], ["k"], "a computed key is not a binding");
    assert_eq!(resolved["h"], ["h"], "h is only read");
    assert_eq!(resolved["o"], ["o"], "a member target writes no binding");
    assert_eq!(resolved["s"], ["s"], "a subscript target writes no binding");
}

/// Every other way a name escapes "bound once in the file, never assigned" defeats same-file
/// resolution, and a name both a declarator and its own function or class expression bind stays
/// resolved (G188 review fixtures `cx`).
#[test]
fn a_name_fixes_a_callee_only_where_its_one_binding_is_visible() {
    let unresolved = |path: &str, source: &str, spelling: &str| {
        let batch = extract(path, source);
        let found: Vec<_> = calls(&batch)
            .into_iter()
            .filter(|(_, s, _, _)| s == spelling)
            .collect();
        assert!(!found.is_empty(), "{source}");
        for (_, _, callees, status) in found {
            assert!(callees.is_empty(), "{source}: {callees:?}");
            assert_eq!(status, EpistemicStatus::Observed, "{source}");
        }
    };
    for (path, source, spelling) in [
        (
            "src/c01.js",
            "function f() {}\nconst g = function f() { return f(); };\n",
            "f",
        ),
        (
            "src/c02.js",
            "function f() {}\nconst X = class f { m() { return f(); } };\n",
            "f",
        ),
        ("src/c03.js", "function f() {}\nf++;\nf();\n", "f"),
        ("src/c04.mjs", "{ function f() {} }\nf();\n", "f"),
        ("src/c04.js", "if (x) function f() {}\nf();\n", "f"),
        (
            "src/c05.ts",
            "namespace N { export function g() {} }\ng();\n",
            "g",
        ),
        (
            "src/c06.js",
            "function f() {}\nwith (obj) { f(); }\nf();\n",
            "f",
        ),
        (
            "src/c07.ts",
            "export {};\nfunction f() {}\nnamespace M { import f = N.g; f(); }\n",
            "f",
        ),
        (
            "src/c07b.ts",
            "function f() {}\ndeclare module \"m\" { import f = require(\"x\"); }\nf();\n",
            "f",
        ),
        (
            "src/c08.js",
            "class K { constructor() {} }\nconst Q = class K { m() { return new K(); } };\n",
            "K::constructor",
        ),
        (
            "src/c09.js",
            "{ class K { constructor() {} } }\nnew K();\n",
            "K::constructor",
        ),
        (
            "src/c10.js",
            "const Q = class K { constructor() {} };\nnew K();\n",
            "K::constructor",
        ),
        (
            "src/c10b.js",
            "const Q = class K { constructor() {} };\nfunction K() {}\nnew K();\n",
            "K::constructor",
        ),
        (
            "src/c05b.ts",
            "namespace N { var v = () => 1; }\nv();\n",
            "v",
        ),
        // In a script another file's `namespace N { export function f() {} }` may merge into N.
        ("src/c11.ts", "function f() {}\nnamespace N { f(); }\n", "f"),
        (
            "src/c12.ts",
            "export {};\nfunction f() {}\nnamespace N { declare function f(): void; f(); }\n",
            "f",
        ),
        (
            "src/c13.js",
            "function f() {}\nfunction h() { eval(\"var f = 2\"); return f(); }\n",
            "f",
        ),
        // Still direct: parenthesized, and TypeScript's type-only wrappers compile away.
        (
            "src/c13b.js",
            "function f() {}\nfunction h() { ((eval))(\"var f = 2\"); return f(); }\n",
            "f",
        ),
        (
            "src/c13c.ts",
            "function f() {}\nfunction h() { (eval as any)(\"f = 2\"); return f(); }\n",
            "f",
        ),
        (
            "src/c13d.ts",
            "function f() {}\nfunction h() { eval!(\"f = 2\"); return f(); }\n",
            "f",
        ),
        (
            "src/c13e.ts",
            "function f() {}\nfunction h() { (<any>eval)(\"f = 2\"); return f(); }\n",
            "f",
        ),
        (
            "src/c13f.ts",
            "function f() {}\nfunction h() { (eval satisfies any)(\"f = 2\"); return f(); }\n",
            "f",
        ),
        // Module-level code before the declaration: temporal dead zone, or `undefined`.
        (
            "src/c14.js",
            "new K();\nclass K { constructor() {} }\n",
            "K::constructor",
        ),
        ("src/c15.js", "f();\nconst f = () => 1;\n", "f"),
        ("src/c16.js", "g();\nvar g = function () {};\n", "g"),
    ] {
        unresolved(path, source, spelling);
    }
    // Where the one binding is visible, the callee is fixed: a declarator naming its own
    // expression, an export, a block `const` called in its block, a `var` in a top-level `if`
    // (module-scoped), and a module function outside a same-named function expression (inside
    // it, the expression's own name).
    let batch = extract(
        "src/same.js",
        "const f = function f() { return f(); };\nf();\nconst K = class K { constructor() {} };\nnew K();\nexport function e() {}\ne();\n{ const inBlock = () => 1; inBlock(); }\nif (x) { var hoisted = function () {}; }\nhoisted();\nconst other = function h() { return h(); };\nfunction h() {}\nh();\nfunction run() { return later(); }\nconst later = () => 1;\nswitch (x) { case 1: function inCase() {} break; case 2: inCase(); }\nswitch (y) { case 1: inDefault(); break; default: function inDefault() {} }\nnamespace();\nfunction namespace() {}\n",
    );
    let got: Vec<(String, String, Vec<String>)> = calls(&batch)
        .into_iter()
        .map(|(caller, spelling, callees, _)| (caller, spelling, callees))
        .collect();
    let row = |caller: &str, spelling: &str, callees: &[&str]| {
        (
            caller.to_owned(),
            spelling.to_owned(),
            callees.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>(),
        )
    };
    assert_eq!(
        got,
        [
            row("f", "f", &["f"]),
            row("other", "h", &[]),
            row("run", "later", &["later"]),
            row("{module}", "K::constructor", &["constructor"]),
            row("{module}", "e", &["e"]),
            row("{module}", "f", &["f"]),
            row("{module}", "h", &["h"]),
            row("{module}", "hoisted", &["hoisted"]),
            row("{module}", "inBlock", &["inBlock"]),
            row("{module}", "inCase", &["inCase"]),
            row("{module}", "inDefault", &["inDefault"]),
            row("{module}", "namespace", &["namespace"]),
        ]
    );
}

/// In a module a namespace cannot be extended from another file, and an overload signature
/// does not rebind its implementation's name.
#[test]
fn a_module_namespace_and_an_overloaded_function_keep_their_callee() {
    for (path, source) in [
        (
            "src/m.ts",
            "export {};\nfunction f() {}\nnamespace N { f(); }\n",
        ),
        (
            "src/o.ts",
            "function f(a: string): void;\nfunction f(a: any) {}\nf(\"x\");\n",
        ),
    ] {
        let batch = extract(path, source);
        let resolved: Vec<Vec<String>> = calls(&batch).into_iter().map(|c| c.2).collect();
        assert_eq!(resolved, [vec!["f".to_owned()]], "{source}");
    }
}
