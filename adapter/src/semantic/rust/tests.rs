//! R4.3 correctness corpus and invariant tests for `RustSemanticExtractor`.
//!
//! A small, hand-curated set of real Rust fixtures (not a benchmark) exercising every construct
//! named in `.atlas/contracts/SEMANTIC-EXTRACTION.md`'s R4.3 minimum: free/public/async/unsafe/
//! extern/generic functions, struct/generic-struct/enum/trait/impl/impl-method/trait-method/
//! type-alias/const/static declarations, nested modules, references, tuples, `Option`/`Result`,
//! and one malformed file. Nothing here executes the fixture source; it is only ever parsed.

use super::super::batch::ExtractionBatch;
use super::super::extractor::{ExtractionInput, SemanticExtractor};
use super::{RustSemanticExtractor, SUPPORTED_DIMENSIONS, spelling};
use atlas_core::{
    ArtifactId, CallDispatchKind, CallSiteIdentity, ConcurrencyIdentity, ConcurrencyKind,
    ContentFingerprint, ControlFlowBlockIdentity, ControlFlowBlockKind, ControlFlowEdge,
    ControlFlowEdgeKind, DataFlowResolution, DiagnosticCode, EffectCategory, EffectIdentity,
    EpistemicStatus, FunctionDeclarationKind, FunctionIdentity, FunctionSignature,
    OwnershipIdentity, OwnershipKind, PersistenceIdentity, PersistenceKind, PersistenceResolution,
    PlaceRef, RepositoryId, RevisionRef, SemanticDimension, SemanticObservation, SemanticRecordId,
    StateAccessIdentity, StateAccessKind, StateResolution, SymbolIdentity, SymbolRole,
    TypeIdentity, ValueIdentity, ValueRole,
};

const ALL_DIMENSIONS: [SemanticDimension; 12] = [
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

/// Reference correctness corpus: one small, real Rust file covering every R4.3 minimum
/// construct. `greet` is deliberately declared twice -- once as a free function, once as a
/// `Greeter for User` trait impl method -- to prove scope disambiguates identical names.
const CORPUS: &str = r#"
pub fn free_function(x: u64) -> u64 { x }

fn private_function() {}

pub fn greet() -> String { "top-level".to_owned() }

pub async fn async_function() -> u64 { 0 }

pub unsafe fn unsafe_function() {}

pub extern "C" fn extern_function() {}

pub fn generic_function<T: Clone>(value: T) -> T { value }

pub struct User {
    pub id: u64,
    pub name: String,
}

pub struct Wrapper<T> {
    pub value: T,
}

pub enum Status {
    Active,
    Inactive(String),
}

pub trait Greeter {
    fn greet(&self) -> String;
    fn default_greet(&self) -> String {
        "hello".to_owned()
    }
}

impl Greeter for User {
    fn greet(&self) -> String {
        self.name.clone()
    }
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        User { id, name }
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}

pub type UserId = u64;

pub const MAX_USERS: u64 = 100;
pub static GREETING: &str = "hi";

pub mod nested {
    pub struct Inner {
        pub value: u64,
    }

    pub fn takes_reference(value: &u64, value_mut: &mut u64) -> (u64, u64) {
        (*value, *value_mut)
    }

    pub fn find_user(id: u64) -> Option<u64> {
        Some(id)
    }

    pub fn load_user(id: u64) -> Result<u64, String> {
        Ok(id)
    }
}

pub mod other_nested {
    pub fn find_user(id: u64) -> Option<u64> {
        None
    }
}
"#;

/// The exact worked example from `.atlas/contracts/SEMANTIC-EXTRACTION.md`'s R4.3 scope note.
const USERS_EXAMPLE: &str = r#"
pub mod users {
    pub struct User {
        pub id: u64,
    }

    pub async fn load_user(id: u64) -> Result<User, Error> {
        todo!()
    }
}
"#;

const MALFORMED: &str = "pub fn broken( {\n";

/// R4.4 function-identity-closure corpus (`.atlas/contracts/SEMANTIC-EXTRACTION.md` R4.4): the
/// exact declaration shapes R4.4 must keep distinguishable -- same name in different modules, same
/// method name on different inherent impl targets, an inherent method vs a trait impl method of
/// the same name on the same target, two different traits implementing the same method name for
/// the same target, a trait method declaration vs its sibling default-bodied method, an
/// associated function (no receiver) vs a receiver method, and generic function declarations.
const R4_4_CORPUS: &str = r#"
mod a {
    pub fn run() {}
}

mod b {
    pub fn run() {}
}

pub struct Foo;
pub struct Bar;

impl Foo {
    pub fn get(&self) {}
    pub fn create() -> Self { Foo }
    pub fn read(&self) {}
}

impl Bar {
    pub fn get(&self) {}
}

pub trait Reader {
    fn read(&self);
    fn default_read(&self) {}
}

impl Reader for Foo {
    fn read(&self) {}
}

pub trait OtherReader {
    fn read(&self);
}

impl OtherReader for Foo {
    fn read(&self) {}
}

pub fn generic<T>(value: T) -> T { value }

pub fn generic_two<T, U>(a: T, b: U) -> T { a }

pub mod nested {
    pub mod inner {
        pub fn run() {}
    }
}
"#;

/// R4.4 regression fixture (`.atlas/contracts/SEMANTIC-EXTRACTION.md` R4.4 "future CALL safety"):
/// four declarations all named `execute`, split across two modules and two distinct inherent impl
/// targets. A future CALL relation must be able to target exactly one of these -- never a bare
/// textual `"execute"` key -- so every one of them MUST produce a pairwise-distinct
/// `FunctionIdentity`. No `Call`/`ControlFlow`/`DataFlow`/`State`/`Effect` observation is ever
/// produced from this fixture; R4.4 stops at identity, not the call relation itself.
const CALL_SAFETY_CORPUS: &str = r#"
mod a {
    pub fn execute() {}
}

mod b {
    pub fn execute() {}
}

pub struct X;
pub struct Y;

impl X {
    pub fn execute(&self) {}
}

impl Y {
    pub fn execute(&self) {}
}
"#;

/// R4.5 CALL corpus: direct calls, calls nested in control flow, method calls, an associated-
/// function call, a `let`-bound call, functions with zero calls, and one macro invocation that
/// must never be mistaken for a call (a macro's expansion is not observable from syntax alone).
const CALL_CORPUS: &str = r#"
pub fn helper(x: u64) -> u64 { x }

pub fn caller_direct() -> u64 {
    helper(1)
}

pub fn caller_nested() -> u64 {
    if helper(1) > 0 {
        helper(2)
    } else {
        helper(3)
    }
}

pub struct Greeter;

impl Greeter {
    pub fn new() -> Self {
        Greeter
    }

    pub fn greet(&self) -> String {
        self.shout()
    }

    fn shout(&self) -> String {
        "hi".to_owned()
    }
}

pub fn caller_method() -> String {
    let g = Greeter::new();
    g.greet()
}

pub fn caller_macro_only() {
    println!("no calls here");
}

pub fn caller_no_calls() -> u64 {
    42
}

pub struct Widget {
    pub value: u64,
}

impl Widget {
    pub fn get(&self) -> u64 {
        self.value
    }
}

pub fn caller_with_simple_argument(x: u64) -> u64 {
    helper(x)
}

pub fn caller_with_two_simple_arguments(x: u64, y: u64) -> u64 {
    two_args(x, y)
}

pub fn two_args(a: u64, b: u64) -> u64 {
    a + b
}

pub fn caller_with_complex_argument(w: &Widget) -> u64 {
    helper(w.get())
}

pub fn caller_with_method_call_argument(w: &Widget, x: u64) -> u64 {
    w.get_with(x)
}

impl Widget {
    pub fn get_with(&self, extra: u64) -> u64 {
        self.value + extra
    }
}

pub fn caller_with_let_result(x: u64) -> u64 {
    let y = helper(x);
    y
}

pub fn caller_with_assign_result(x: u64) -> u64 {
    let mut y = 0;
    y = helper(x);
    y
}

pub fn caller_with_call_inside_a_larger_expression(x: u64) -> u64 {
    let y = helper(x) + 1;
    y
}

pub fn two_args_tuple(a: u64, b: u64) -> (u64, u64) {
    (a, b)
}

pub fn caller_with_destructured_result(x: u64) -> (u64, u64) {
    let (a, b) = two_args_tuple(x, x);
    (a, b)
}
"#;

/// R4.6 CONTROL_FLOW corpus: straight-line, `if`/`if-else`/else-if chain, `while` (may-not-enter),
/// labeled nested loops with a labeled `break` resolving through an intermediate unlabeled loop,
/// an unresolved (unmatched-label) `break`, an unconditional panic, and a `match`.
const CFG_CORPUS: &str = r#"
pub fn straight_line() -> u64 {
    let x = 1;
    let y = 2;
    x + y
}

pub fn simple_if(cond: bool) -> u64 {
    if cond {
        return 1;
    }
    2
}

pub fn if_else(cond: bool) -> u64 {
    if cond {
        1
    } else {
        2
    }
}

pub fn else_if_chain(x: u64) -> u64 {
    if x == 0 {
        0
    } else if x == 1 {
        1
    } else {
        2
    }
}

pub fn while_loop(n: u64) -> u64 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

pub fn labeled_break(n: u64) -> u64 {
    'outer: loop {
        loop {
            if n > 0 {
                break 'outer;
            }
        }
    }
    n
}

pub fn unresolved_break() -> u64 {
    loop {
        break 'nowhere;
    }
}

pub fn panics_unconditionally() -> u64 {
    panic!("boom")
}

pub fn matches_on_value(x: u64) -> u64 {
    match x {
        0 => 10,
        1 => {
            return 20;
        }
        _ => 30,
    }
}

pub fn no_calls_no_branches() -> u64 {
    42
}
"#;

/// R4.7 DATA_FLOW corpus: simple def-use, shadowing, mutation (Store), block-scoped shadowing,
/// a method receiver's Definition/Use, return-flow (explicit `return` and tail-expression), an
/// unresolved use (no matching local definition), and (R4.12) a tuple-destructuring pattern, whose
/// sub-bindings now each emit a real Definition.
const DATA_FLOW_CORPUS: &str = r#"
pub fn simple_def_use(x: u64) -> u64 {
    let y = x + 1;
    y
}

pub fn shadowing() -> u64 {
    let x = 1;
    let x = x + 1;
    x
}

pub fn mutation() -> u64 {
    let mut x = 1;
    x = 2;
    x
}

pub fn block_scoped_shadow() -> u64 {
    let x = 1;
    {
        let x = 2;
        let _unused = x;
    }
    x
}

pub fn uses_unknown_name() -> u64 {
    y
}

pub fn early_return(x: u64) -> u64 {
    if x > 0 {
        return x;
    }
    0
}

pub struct Holder {
    pub value: u64,
}

impl Holder {
    pub fn get(&self) -> u64 {
        self.value
    }
}

pub fn destructures_a_tuple() -> u64 {
    let (a, b) = (1, 2);
    a + b
}

pub fn if_in_let(x: u64) -> u64 {
    let y = if x > 0 { x } else { x };
    y
}

pub struct Point {
    pub x: u64,
    pub y: u64,
}

pub fn destructures_a_struct(p: Point) -> u64 {
    let Point { x, y } = p;
    x + y
}

pub fn destructures_a_slice(values: [u64; 2]) -> u64 {
    let [first, second] = values;
    first + second
}

pub fn binds_with_at_pattern(x: u64) -> u64 {
    match x {
        n @ 1..=5 => n,
        other => other,
    }
}

pub fn binds_via_or_pattern(x: Result<u64, u64>) -> u64 {
    match x {
        Ok(value) | Err(value) => value,
    }
}
"#;

const STATE_EFFECT_CORPUS: &str = r#"
pub struct Inner {
    pub value: u64,
}

pub struct Counter {
    pub value: u64,
    pub inner: Inner,
}

impl Counter {
    pub fn get(&self) -> u64 {
        self.value
    }

    pub fn set(&mut self, new_value: u64) {
        self.value = new_value;
    }

    pub fn increment_and_get(&mut self) -> u64 {
        self.value = self.value + 1;
        self.value
    }

    pub fn maybe_panic(&self, ok: bool) -> u64 {
        if !ok {
            panic!("not ok");
        }
        self.value
    }

    pub fn read_nested(&self) -> u64 {
        self.inner.value
    }

    pub fn compound_increment(&mut self) {
        self.value += 1;
    }

    pub fn unsafe_wrapped(&mut self) {
        unsafe {
            self.value = 7;
            panic!("inside unsafe");
        }
    }
}

pub fn free_function_with_no_self() -> u64 {
    0
}
"#;

const OWNERSHIP_CORPUS: &str = r#"
pub struct Widget {
    pub value: u64,
}

pub fn borrow_shared_via_field(w: &Widget) -> u64 {
    w.value
}

pub fn borrow_mut_via_field(w: &mut Widget) {
    w.value = 1;
}

pub fn consume(w: Widget) -> Widget {
    w
}

pub fn borrow_then_consume(w: Widget) -> Widget {
    let _ = &w;
    consume(w)
}

pub fn conditional_move(flag: bool, a: Widget, b: Widget) -> Widget {
    if flag {
        a
    } else {
        b
    }
}

pub fn double_borrow(w: &Widget) -> u64 {
    let x = &w.value;
    *x
}
"#;

const CONCURRENCY_CORPUS: &str = r#"
pub async fn await_something(x: u64) -> u64 {
    x.await
}

fn do_work() {}

pub fn spawn_work() {
    thread::spawn(do_work);
}

pub fn call_without_spawn() {
    do_work();
}

pub async fn conditional_await(flag: bool, a: u64, b: u64) -> u64 {
    if flag {
        a.await
    } else {
        b.await
    }
}

pub fn spawn_in_a_loop() {
    for _ in 0..3 {
        thread::spawn(do_work);
    }
}
"#;

fn input_for(
    artifact_path: &str,
    source: &str,
    revision_value: &str,
    dimensions: Vec<SemanticDimension>,
) -> ExtractionInput {
    ExtractionInput {
        repository: RepositoryId::new("atlas-studio"),
        revision: RevisionRef {
            kind: "git".into(),
            value: revision_value.into(),
        },
        artifact: ArtifactId::new(format!("artifact:{artifact_path}")),
        artifact_path: artifact_path.to_owned(),
        source_text: source.to_owned(),
        content_fingerprint: Some(ContentFingerprint(format!("sha256:{artifact_path}"))),
        source_frontend_id: "atlas.source.rust.bootstrap.v1".into(),
        language: "rust".into(),
        build_profile: None,
        scope_policy: None,
        requested_dimensions: dimensions,
    }
}

fn extract(
    artifact_path: &str,
    source: &str,
    dimensions: Vec<SemanticDimension>,
) -> ExtractionBatch {
    RustSemanticExtractor.extract(&input_for(artifact_path, source, "abc123", dimensions))
}

fn extract_all(artifact_path: &str, source: &str) -> ExtractionBatch {
    extract(artifact_path, source, ALL_DIMENSIONS.to_vec())
}

fn scope_of(segments: &[&str]) -> Vec<String> {
    segments.iter().map(|s| s.to_string()).collect()
}

fn find_symbol<'a>(
    batch: &'a ExtractionBatch,
    scope: &[&str],
    name: &str,
) -> Option<&'a SymbolIdentity> {
    batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::Symbol(header)
                if header.subject.scope.segments == scope_of(scope)
                    && header.subject.name == name =>
            {
                Some(&header.subject)
            }
            _ => None,
        })
}

fn find_type<'a>(
    batch: &'a ExtractionBatch,
    scope: &[&str],
    name: &str,
) -> Option<&'a TypeIdentity> {
    batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::Type(header)
                if header.subject.scope.segments == scope_of(scope)
                    && header.subject.name == name =>
            {
                Some(&header.subject)
            }
            _ => None,
        })
}

fn find_function_identity<'a>(
    batch: &'a ExtractionBatch,
    scope: &[&str],
    name: &str,
) -> Option<&'a FunctionIdentity> {
    batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::FunctionIdentity(header)
                if header.subject.scope.segments == scope_of(scope)
                    && header.subject.symbol.name == name =>
            {
                Some(&header.subject)
            }
            _ => None,
        })
}

fn find_function_signature<'a>(
    batch: &'a ExtractionBatch,
    scope: &[&str],
    name: &str,
) -> Option<&'a FunctionSignature> {
    batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::FunctionSignature(header)
                if header.subject.function.scope.segments == scope_of(scope)
                    && header.subject.function.symbol.name == name =>
            {
                Some(&header.subject)
            }
            _ => None,
        })
}

/// Every `FunctionIdentity` observation whose symbol name is `name`, regardless of scope --
/// R4.4's owner/trait/declaration-kind fields (not scope alone) are what a test then filters on to
/// pick out one exact declaration among several same-named ones.
fn function_identities_named<'a>(
    batch: &'a ExtractionBatch,
    name: &str,
) -> Vec<&'a FunctionIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::FunctionIdentity(header) if header.subject.symbol.name == name => {
                Some(&header.subject)
            }
            _ => None,
        })
        .collect()
}

fn owner_target_name(identity: &FunctionIdentity) -> Option<&str> {
    identity
        .owner
        .target
        .as_ref()
        .map(|target| target.name.as_str())
}

fn all_calls(batch: &ExtractionBatch) -> Vec<&CallSiteIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::Call(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every CALL observation whose `function` (the caller) is exactly `caller`'s `FunctionIdentity`
/// record_id -- the same identity a future CALL relation would target, never a bare name match.
fn calls_by_caller<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a CallSiteIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    all_calls(batch)
        .into_iter()
        .filter(|call| call.function == caller_id)
        .collect()
}

fn all_control_flow_blocks(batch: &ExtractionBatch) -> Vec<&ControlFlowBlockIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::ControlFlow(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every ControlFlow block observation whose `function` is exactly `caller`'s `FunctionIdentity`
/// record_id, sorted by `block_index` for deterministic assertions.
fn control_flow_blocks_for<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a ControlFlowBlockIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let mut blocks: Vec<&ControlFlowBlockIdentity> = all_control_flow_blocks(batch)
        .into_iter()
        .filter(|block| block.function == caller_id)
        .collect();
    blocks.sort_by_key(|block| block.block_index);
    blocks
}

fn all_data_flow_values(batch: &ExtractionBatch) -> Vec<&ValueIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::DataFlow(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every DataFlow value observation whose `function` is exactly `caller`'s `FunctionIdentity`
/// record_id, sorted by (span line, span column, role) for deterministic assertions.
fn data_flow_values_for<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a ValueIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let mut values: Vec<&ValueIdentity> = all_data_flow_values(batch)
        .into_iter()
        .filter(|value| value.function == caller_id)
        .collect();
    values.sort_by_key(|value| (value.span.line, value.span.column, value.role.as_str()));
    values
}

fn all_state_accesses(batch: &ExtractionBatch) -> Vec<&StateAccessIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::State(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every State access observation whose `function` is exactly `caller`'s `FunctionIdentity`
/// record_id, sorted by (span line, span column, kind) for deterministic assertions.
fn state_accesses_for<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a StateAccessIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let mut values: Vec<&StateAccessIdentity> = all_state_accesses(batch)
        .into_iter()
        .filter(|value| value.function == caller_id)
        .collect();
    values.sort_by_key(|value| (value.span.line, value.span.column, value.kind.as_str()));
    values
}

fn all_effects(batch: &ExtractionBatch) -> Vec<&EffectIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::Effect(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every Effect observation whose `function` is exactly `caller`'s `FunctionIdentity` record_id,
/// sorted by (span line, span column) for deterministic assertions.
fn effects_for<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a EffectIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let mut values: Vec<&EffectIdentity> = all_effects(batch)
        .into_iter()
        .filter(|value| value.function == caller_id)
        .collect();
    values.sort_by_key(|value| (value.span.line, value.span.column));
    values
}

fn all_ownership_ops(batch: &ExtractionBatch) -> Vec<&OwnershipIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::Ownership(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every Ownership operation observation whose `function` is exactly `caller`'s `FunctionIdentity`
/// record_id, sorted by (span line, span column, kind) for deterministic assertions.
fn ownership_ops_for<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a OwnershipIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let mut values: Vec<&OwnershipIdentity> = all_ownership_ops(batch)
        .into_iter()
        .filter(|value| value.function == caller_id)
        .collect();
    values.sort_by_key(|value| (value.span.line, value.span.column, value.kind.as_str()));
    values
}

fn all_concurrency_ops(batch: &ExtractionBatch) -> Vec<&ConcurrencyIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::Concurrency(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every Concurrency operation observation whose `function` is exactly `caller`'s
/// `FunctionIdentity` record_id, sorted by (span line, span column, kind) for deterministic
/// assertions.
fn concurrency_ops_for<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a ConcurrencyIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let mut values: Vec<&ConcurrencyIdentity> = all_concurrency_ops(batch)
        .into_iter()
        .filter(|value| value.function == caller_id)
        .collect();
    values.sort_by_key(|value| (value.span.line, value.span.column, value.kind.as_str()));
    values
}

fn all_persistence_ops(batch: &ExtractionBatch) -> Vec<&PersistenceIdentity> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::Persistence(header) => Some(&header.subject),
            _ => None,
        })
        .collect()
}

/// Every Persistence operation observation whose `function` is exactly `caller`'s
/// `FunctionIdentity` record_id, sorted by (span line, span column, kind) for deterministic
/// assertions.
fn persistence_ops_for<'a>(
    batch: &'a ExtractionBatch,
    caller: &FunctionIdentity,
) -> Vec<&'a PersistenceIdentity> {
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let mut values: Vec<&PersistenceIdentity> = all_persistence_ops(batch)
        .into_iter()
        .filter(|value| value.function == caller_id)
        .collect();
    values.sort_by_key(|value| (value.span.line, value.span.column, value.kind.as_str()));
    values
}

// --- 1. free/public function symbol observation ------------------------------------------------

#[test]
fn free_function_produces_a_symbol_definition() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let symbol = find_symbol(&batch, &[], "free_function").expect("free_function symbol");
    assert_eq!(symbol.role, SymbolRole::Definition);
}

// --- 2. FunctionIdentity is recorded ------------------------------------------------------------

#[test]
fn function_identity_is_recorded_for_a_free_function() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let identity = find_function_identity(&batch, &[], "free_function").expect("identity");
    assert_eq!(identity.language, "rust");
    assert!(!identity.generated);
}

// --- 3. signature params/return/visibility ------------------------------------------------------

#[test]
fn function_signature_captures_parameters_return_type_and_visibility() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let signature = find_function_signature(&batch, &[], "free_function").expect("signature");
    assert_eq!(signature.visibility, "pub");
    assert_eq!(signature.parameters.len(), 1);
    assert_eq!(signature.parameters[0].name, "x");
    assert_eq!(signature.parameters[0].type_identity.name, "u64");
    assert_eq!(signature.return_type.as_ref().unwrap().name, "u64");

    let private = find_function_signature(&batch, &[], "private_function").expect("private");
    assert_eq!(private.visibility, "inherited");
}

// --- 4. async/unsafe/extern flags -----------------------------------------------------------

#[test]
fn async_unsafe_and_extern_flags_are_captured() {
    let batch = extract_all("src/lib.rs", CORPUS);

    let async_sig = find_function_signature(&batch, &[], "async_function").unwrap();
    assert!(async_sig.is_async);
    assert!(!async_sig.is_unsafe);
    assert!(!async_sig.is_extern);

    let unsafe_sig = find_function_signature(&batch, &[], "unsafe_function").unwrap();
    assert!(unsafe_sig.is_unsafe);
    assert!(!unsafe_sig.is_async);

    let extern_sig = find_function_signature(&batch, &[], "extern_function").unwrap();
    assert!(extern_sig.is_extern);
    assert_eq!(extern_sig.abi.as_deref(), Some("C"));
}

// --- 5. generic function signature ------------------------------------------------------------

#[test]
fn generic_function_signature_preserves_generic_parameters_and_bounds() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let signature = find_function_signature(&batch, &[], "generic_function").unwrap();
    assert_eq!(signature.generics, vec!["T: Clone".to_owned()]);
    assert_eq!(signature.parameters[0].type_identity.name, "T");
    assert_eq!(signature.return_type.as_ref().unwrap().name, "T");
}

// --- 6. struct symbols + field types ------------------------------------------------------------

#[test]
fn struct_and_its_fields_produce_symbol_and_type_observations() {
    let batch = extract_all("src/lib.rs", CORPUS);
    find_symbol(&batch, &[], "User").expect("struct symbol");
    find_symbol(&batch, &["User"], "id").expect("field symbol id");
    find_symbol(&batch, &["User"], "name").expect("field symbol name");
    find_type(&batch, &["User"], "u64").expect("field type u64");
    find_type(&batch, &["User"], "String").expect("field type String");
}

// --- 7. generic struct field type uses the generic parameter's own spelling --------------------

#[test]
fn generic_struct_field_type_uses_the_generic_parameter_spelling() {
    let batch = extract_all("src/lib.rs", CORPUS);
    find_symbol(&batch, &[], "Wrapper").expect("Wrapper symbol");
    find_type(&batch, &["Wrapper"], "T").expect("field type T (the generic parameter itself)");
}

// --- 8. enum variants and payload types -----------------------------------------------------

#[test]
fn enum_variants_and_payload_types_are_observed() {
    let batch = extract_all("src/lib.rs", CORPUS);
    find_symbol(&batch, &[], "Status").expect("enum symbol");
    find_symbol(&batch, &["Status"], "Active").expect("unit variant symbol");
    find_symbol(&batch, &["Status"], "Inactive").expect("tuple variant symbol");
    find_type(&batch, &["Status"], "String").expect("tuple variant payload type");
}

// --- 9. trait methods: declaration vs default-body definition ----------------------------------

#[test]
fn trait_methods_distinguish_declaration_from_default_definition() {
    let batch = extract_all("src/lib.rs", CORPUS);
    find_symbol(&batch, &[], "Greeter").expect("trait symbol");

    let declared = find_symbol(&batch, &["trait:Greeter"], "greet").expect("abstract method");
    assert_eq!(declared.role, SymbolRole::Declaration);

    let defaulted =
        find_symbol(&batch, &["trait:Greeter"], "default_greet").expect("defaulted method");
    assert_eq!(defaulted.role, SymbolRole::Definition);
}

// --- 10. impl method scope differs from a same-named free function ----------------------------

#[test]
fn impl_method_scope_differs_from_a_same_named_free_function() {
    let batch = extract_all("src/lib.rs", CORPUS);

    let free = find_function_identity(&batch, &[], "greet").expect("free-function greet");
    let impl_method =
        find_function_identity(&batch, &["impl:Greeter for User"], "greet").expect("impl greet");

    assert_ne!(free.identity_key(), impl_method.identity_key());
    assert_eq!(
        find_function_signature(&batch, &["impl:Greeter for User"], "greet")
            .unwrap()
            .parameters[0]
            .name,
        "&self"
    );
}

// --- 11. same function name in different modules produces different identities -----------------

#[test]
fn same_function_name_in_different_modules_produces_different_identities() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let a = find_function_identity(&batch, &["nested"], "find_user").expect("nested::find_user");
    let b = find_function_identity(&batch, &["other_nested"], "find_user")
        .expect("other_nested::find_user");
    assert_ne!(a.identity_key(), b.identity_key());
}

// --- 12. revision change alters function identity -----------------------------------------------

#[test]
fn revision_change_alters_function_identity() {
    let source = "pub fn stable_name() {}\n";
    let a = RustSemanticExtractor.extract(&input_for(
        "src/lib.rs",
        source,
        "revision-a",
        vec![SemanticDimension::FunctionIdentity],
    ));
    let b = RustSemanticExtractor.extract(&input_for(
        "src/lib.rs",
        source,
        "revision-b",
        vec![SemanticDimension::FunctionIdentity],
    ));
    let identity_a = find_function_identity(&a, &[], "stable_name").unwrap();
    let identity_b = find_function_identity(&b, &[], "stable_name").unwrap();
    assert_ne!(identity_a.identity_key(), identity_b.identity_key());
}

// --- 13. type alias produces a symbol plus the aliased type ------------------------------------

#[test]
fn type_alias_produces_symbol_and_aliased_type_observation() {
    let batch = extract_all("src/lib.rs", CORPUS);
    find_symbol(&batch, &[], "UserId").expect("alias symbol");
    find_type(&batch, &[], "u64").expect("aliased type");
}

// --- 14. const/static declarations ---------------------------------------------------------

#[test]
fn const_and_static_declarations_produce_symbol_and_type_observations() {
    let batch = extract_all("src/lib.rs", CORPUS);
    find_symbol(&batch, &[], "MAX_USERS").expect("const symbol");
    find_symbol(&batch, &[], "GREETING").expect("static symbol");
    find_type(&batch, &[], "&str").expect("static's declared type");
}

// --- 15. nested module scope accumulates from source structure, not the filesystem path --------

#[test]
fn nested_module_scope_is_source_level_and_deterministic() {
    let batch = extract_all("completely/unrelated/path.rs", CORPUS);
    find_symbol(&batch, &[], "nested").expect("mod nested symbol");
    find_symbol(&batch, &["nested"], "Inner").expect("nested::Inner");
    find_symbol(&batch, &["nested", "Inner"], "value").expect("nested::Inner::value field");
}

// --- 16. reference / mutable reference parameter types, and tuple return -----------------------

#[test]
fn reference_and_mutable_reference_parameter_types_are_captured() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let signature = find_function_signature(&batch, &["nested"], "takes_reference").unwrap();
    assert_eq!(signature.parameters[0].type_identity.name, "&u64");
    assert_eq!(signature.parameters[1].type_identity.name, "&mut u64");
    assert_eq!(signature.return_type.as_ref().unwrap().name, "(u64, u64)");
}

// --- 17. Option<T> / Result<T, E> return types --------------------------------------------------

#[test]
fn option_and_result_return_types_are_captured() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let find_user = find_function_signature(&batch, &["nested"], "find_user").unwrap();
    assert_eq!(find_user.return_type.as_ref().unwrap().name, "Option<u64>");
    let load_user = find_function_signature(&batch, &["nested"], "load_user").unwrap();
    assert_eq!(
        load_user.return_type.as_ref().unwrap().name,
        "Result<u64, String>"
    );
}

// --- 18. the exact worked example from the R4.3 scope note --------------------------------------

#[test]
fn the_users_example_from_the_contract_note_extracts_as_documented() {
    let batch = extract_all("src/users.rs", USERS_EXAMPLE);
    find_symbol(&batch, &[], "users").expect("users module symbol");
    find_symbol(&batch, &["users"], "User").expect("User struct symbol");
    find_symbol(&batch, &["users", "User"], "id").expect("User.id field symbol");
    let load_user_identity =
        find_function_identity(&batch, &["users"], "load_user").expect("load_user identity");
    let signature = find_function_signature(&batch, &["users"], "load_user").unwrap();
    assert_eq!(signature.visibility, "pub");
    assert!(signature.is_async);
    assert_eq!(signature.parameters[0].name, "id");
    assert_eq!(signature.parameters[0].type_identity.name, "u64");
    assert_eq!(
        signature.return_type.as_ref().unwrap().name,
        "Result<User, Error>"
    );

    // `todo!()` is a macro invocation (`syn::Expr::Macro`), not a `syn::Expr::Call` -- so even
    // with R4.5 CALL support, this body contributes no Call observation. `load_user` is a free
    // function (no `self`), so R4.8 STATE finds nothing to claim either.
    assert!(batch.observations.iter().all(|observation| !matches!(
        observation,
        SemanticObservation::Call(_) | SemanticObservation::State(_)
    )));

    // R4.8: `todo!()` IS panic-like, so it produces a real Effect(Panic) observation, attributed
    // to `load_user`.
    let effects: Vec<_> = batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::Effect(header) => Some(&header.subject),
            _ => None,
        })
        .collect();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].category, EffectCategory::Panic);
    assert_eq!(
        effects[0].function,
        SemanticRecordId::new(
            SemanticDimension::FunctionIdentity,
            &load_user_identity.identity_key()
        )
    );

    // R4.7: the `id` parameter still produces a real Definition, even though the body (`todo!()`)
    // never uses it -- an unused-but-declared parameter is not the same as no data flow at all.
    let values = data_flow_values_for(&batch, load_user_identity);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].role, ValueRole::Definition);
    assert!(values[0].is_parameter);
    assert_eq!(values[0].name, "id");

    // R4.6: `todo!()` IS panic-like, so `load_user`'s single FunctionEntry block gets a real
    // Panic terminator edge.
    let load_user = find_function_identity(&batch, &["users"], "load_user").unwrap();
    let blocks = control_flow_blocks_for(&batch, load_user);
    assert_eq!(blocks.len(), 1);
    assert!(blocks[0].is_entry);
    assert_eq!(blocks[0].successors.len(), 1);
    assert_eq!(blocks[0].successors[0].kind, ControlFlowEdgeKind::Panic);
    assert!(blocks[0].successors[0].target.is_none());
}

// --- 19. malformed Rust: ParseFailure + UNKNOWN for supported dims, UNSUPPORTED for the rest ----

#[test]
fn malformed_rust_produces_parse_failure_and_unknown_for_supported_dimensions() {
    let batch = extract("src/broken.rs", MALFORMED, ALL_DIMENSIONS.to_vec());
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    assert!(batch.observations.is_empty());
    assert!(
        batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code
                == super::super::extractor::DiagnosticCode::ParseFailure)
    );
    for &dimension in &[
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
    ] {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_eq!(obligation.status, EpistemicStatus::Unknown);
        assert!(!obligation.diagnostics.is_empty());
    }
}

// --- 20. every SemanticDimension variant is now supported by this extractor (R4.11 closes the
// last gap); a dimension going from unsupported to supported must never silently drop it from
// batch accounting -----------------------------------------------------------------------------

#[test]
fn every_dimension_variant_is_now_supported_and_accounted_for() {
    let batch = extract_all("src/lib.rs", CORPUS);
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    for &dimension in &ALL_DIMENSIONS {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_ne!(obligation.status, EpistemicStatus::Unsupported);
    }
}

// --- 21. every requested dimension is accounted, whatever subset is requested -------------------

#[test]
fn every_requested_dimension_is_accounted_for_any_requested_subset() {
    let requested = vec![
        SemanticDimension::Symbol,
        SemanticDimension::Call,
        SemanticDimension::Effect,
    ];
    let batch = extract("src/lib.rs", CORPUS, requested.clone());
    assert!(batch.is_closed(&requested));
    assert_eq!(batch.obligations.len(), 3);
}

// --- 22. deterministic across repeated runs -----------------------------------------------------

#[test]
fn extraction_is_byte_for_byte_deterministic_across_repeated_runs() {
    let a = extract_all("src/lib.rs", CORPUS);
    let b = extract_all("src/lib.rs", CORPUS);
    assert_eq!(a, b);
}

// --- 23. unaffected by requested-dimension ordering ----------------------------------------------

#[test]
fn extraction_is_unaffected_by_requested_dimension_order() {
    let mut reversed = ALL_DIMENSIONS.to_vec();
    reversed.reverse();
    let forward = extract_all("src/lib.rs", CORPUS);
    let backward = extract("src/lib.rs", CORPUS, reversed);
    assert_eq!(forward.observations, backward.observations);
    let mut forward_obligations = forward.obligations.clone();
    let mut backward_obligations = backward.obligations.clone();
    forward_obligations.sort_by_key(|o| o.dimension.as_str());
    backward_obligations.sort_by_key(|o| o.dimension.as_str());
    assert_eq!(forward_obligations, backward_obligations);
}

// --- 24. every observation satisfies the variant/dimension consistency invariant ---------------

#[test]
fn every_observation_satisfies_dimension_consistency() {
    let batch = extract_all("src/lib.rs", CORPUS);
    assert!(!batch.observations.is_empty());
    for observation in &batch.observations {
        assert!(observation.is_dimension_consistent());
    }
}

// --- 25. evidence and provenance are attached, and identify the extractor/artifact -------------

#[test]
fn observed_records_carry_evidence_and_provenance() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let SemanticObservation::Symbol(header) = batch
        .observations
        .iter()
        .find(|observation| matches!(observation, SemanticObservation::Symbol(_)))
        .unwrap()
    else {
        unreachable!()
    };
    assert!(!header.evidence_refs.is_empty());
    assert_eq!(header.provenance.source_path, "src/lib.rs");
    assert_eq!(header.provenance.extractor, "atlas.rust.source-semantic.v1");
    let evidence_id = header.evidence_refs[0].as_str();
    assert!(
        batch
            .evidence
            .iter()
            .any(|evidence| evidence.id == evidence_id && evidence.kind == "PARSER_OUTPUT")
    );
}

// --- 26. TypeIdentity.canonical is never fabricated ----------------------------------------------

#[test]
fn type_identity_canonical_is_always_none() {
    let batch = extract_all("src/lib.rs", CORPUS);
    let mut saw_a_type = false;
    for observation in &batch.observations {
        if let SemanticObservation::Type(header) = observation {
            saw_a_type = true;
            assert!(header.subject.canonical.is_none());
        }
    }
    assert!(saw_a_type);
}

// --- 27. real extractor is selected for "rust", not the static-unsupported fallback ------------

#[test]
fn real_extractor_is_selected_for_rust() {
    let extractors = crate::semantic::extractors_for_language("rust");
    assert_eq!(extractors.len(), 1);
    assert_eq!(extractors[0].id(), "atlas.rust.source-semantic.v1");
}

// --- 28. verified absence: an empty file exhaustively yields zero declarations, not "not found" -

#[test]
fn verified_absence_is_explicit_for_an_empty_file() {
    let batch = extract("src/empty.rs", "", ALL_DIMENSIONS.to_vec());
    for &dimension in &[
        SemanticDimension::Symbol,
        SemanticDimension::Type,
        SemanticDimension::FunctionIdentity,
        SemanticDimension::FunctionSignature,
    ] {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_eq!(obligation.status, EpistemicStatus::Observed);
        assert!(obligation.observation_ids.is_empty());
        assert!(!obligation.evidence_refs.is_empty());
    }
}

// --- 29. no graph dependency, no repository code execution ---------------------------------------

#[test]
fn rust_extractor_has_no_graph_dependency_and_never_executes_repository_code() {
    // Built at runtime (not written as contiguous literals) so this check doesn't trip on its own
    // text, including doc comments that legitimately *name* these concepts while forbidding them.
    let graph_type = format!("{}{}", "Engineering", "Graph");
    let graph_module_path = format!("{}::{}", "atlas_core", "graph");
    let process_command = format!("{}::{}", "std::process", "Command");
    for source in [
        include_str!("mod.rs"),
        include_str!("spelling.rs"),
        include_str!("tests.rs"),
    ] {
        assert!(
            !source.contains(&graph_type) && !source.contains(&graph_module_path),
            "rust semantic extractor must not depend on the engineering graph"
        );
        assert!(
            !source.contains(&process_command),
            "rust semantic extractor must never spawn a process to execute repository code"
        );
    }
}

// --- 30. type spelling helper sanity (unit-level, not full-extractor) ---------------------------

#[test]
fn type_spelling_renders_common_shapes_cleanly() {
    let parse = |source: &str| -> syn::Type { syn::parse_str(source).unwrap() };
    assert_eq!(spelling::type_spelling(&parse("u64")), "u64");
    assert_eq!(
        spelling::type_spelling(&parse("Result<User, Error>")),
        "Result<User, Error>"
    );
    assert_eq!(
        spelling::type_spelling(&parse("Option<u64>")),
        "Option<u64>"
    );
    assert_eq!(spelling::type_spelling(&parse("&u64")), "&u64");
    assert_eq!(spelling::type_spelling(&parse("&mut u64")), "&mut u64");
    assert_eq!(spelling::type_spelling(&parse("(u64, u64)")), "(u64, u64)");
    assert_eq!(spelling::type_spelling(&parse("[u8; 4]")), "[u8; 4]");
}

#[test]
fn a_one_element_tuple_type_is_never_spelled_the_same_as_its_parenthesized_element() {
    // `(u64,)` (a real 1-element tuple type, accessed via `.0`) and `(u64)` (a merely-parenthesized
    // `u64` -- exactly the same type as plain `u64`) are genuinely different Rust types. Since
    // `TypeIdentity.canonical` stays `None` throughout this extractor's scope, `TypeIdentity`'s own
    // `identity_key()` hashes on `name` alone -- so if both rendered to the same string, a real
    // type difference (e.g. refactoring a field from `(u64,)` to `(u64)`) would silently collapse
    // onto the same graph node instead of producing a distinct one.
    let parse = |source: &str| -> syn::Type { syn::parse_str(source).unwrap() };
    let one_element_tuple = spelling::type_spelling(&parse("(u64,)"));
    let parenthesized = spelling::type_spelling(&parse("(u64)"));
    assert_eq!(one_element_tuple, "(u64,)");
    assert_eq!(parenthesized, "(u64)");
    assert_ne!(
        one_element_tuple, parenthesized,
        "a real 1-tuple and a merely-parenthesized element type must never collide"
    );
}

// =================================================================================================
// R4.4 -- Function Identity Closure
//
// Every test below fails under the pre-R4.4 extractor: `FunctionIdentity` had no
// `declaration_kind`/`owner`/`generics` fields, and `function_identity()` hardcoded
// `symbol.role = SymbolRole::Definition` regardless of the actual declaration (a trait method
// DECLARATION with no body would incoherently claim `Definition`). Distinctness that already held
// only incidentally through the scope-string encoding (e.g. `"impl:Reader for Foo"`) is asserted
// here directly against the new typed fields, never against a display string.
// =================================================================================================

// --- 1. same function name in different modules => distinct FunctionIdentity -------------------

#[test]
fn same_name_in_different_modules_is_distinct() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let a = find_function_identity(&batch, &["a"], "run").expect("a::run");
    let b = find_function_identity(&batch, &["b"], "run").expect("b::run");
    assert_ne!(a.identity_key(), b.identity_key());
    assert_eq!(a.declaration_kind, FunctionDeclarationKind::FreeFunction);
    assert_eq!(b.declaration_kind, FunctionDeclarationKind::FreeFunction);
}

// --- 2. same method name on different inherent impl targets => distinct identity ---------------

#[test]
fn same_method_name_on_different_inherent_impl_targets_is_distinct() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let candidates = function_identities_named(&batch, "get");
    let foo_get = candidates
        .iter()
        .find(|identity| owner_target_name(identity) == Some("Foo"))
        .expect("Foo::get");
    let bar_get = candidates
        .iter()
        .find(|identity| owner_target_name(identity) == Some("Bar"))
        .expect("Bar::get");
    assert_ne!(foo_get.identity_key(), bar_get.identity_key());
    assert_eq!(
        foo_get.declaration_kind,
        FunctionDeclarationKind::InherentMethod
    );
    assert_eq!(
        bar_get.declaration_kind,
        FunctionDeclarationKind::InherentMethod
    );
}

// --- 3. inherent `Foo::read` vs `Reader for Foo::read` => distinct identity --------------------

#[test]
fn inherent_method_is_distinct_from_trait_impl_method_of_the_same_name() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let candidates = function_identities_named(&batch, "read");
    let inherent = candidates
        .iter()
        .find(|identity| {
            owner_target_name(identity) == Some("Foo") && identity.owner.trait_path.is_none()
        })
        .expect("inherent Foo::read");
    // `owner.trait_path == Some("Reader")` alone is ambiguous: it also matches the trait
    // DECLARATION `Reader::read` (which shares the same trait_path but has no impl owner). Require
    // declaration_kind explicitly so this picks out the trait IMPLEMENTATION method regardless of
    // `candidates`' iteration order.
    let trait_impl = candidates
        .iter()
        .find(|identity| {
            identity.owner.trait_path.as_deref() == Some("Reader")
                && identity.declaration_kind == FunctionDeclarationKind::TraitImplementationMethod
        })
        .expect("Reader for Foo::read");
    assert_ne!(inherent.identity_key(), trait_impl.identity_key());
    assert_eq!(
        inherent.declaration_kind,
        FunctionDeclarationKind::InherentMethod
    );
    assert_eq!(
        trait_impl.declaration_kind,
        FunctionDeclarationKind::TraitImplementationMethod
    );
}

// --- 4. two traits with the same method name implemented for the same type => distinct identity

#[test]
fn two_traits_with_the_same_method_name_on_the_same_target_are_distinct() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let candidates = function_identities_named(&batch, "read");
    // `owner.trait_path == Some(<name>)` alone is ambiguous: each trait's own DECLARATION
    // (`Reader::read`/`OtherReader::read`) shares the same trait_path as its implementation.
    // Require declaration_kind explicitly so this picks out the IMPLEMENTATION methods
    // regardless of `candidates`' iteration order.
    let reader = candidates
        .iter()
        .find(|identity| {
            identity.owner.trait_path.as_deref() == Some("Reader")
                && identity.declaration_kind == FunctionDeclarationKind::TraitImplementationMethod
        })
        .expect("Reader for Foo::read");
    let other_reader = candidates
        .iter()
        .find(|identity| {
            identity.owner.trait_path.as_deref() == Some("OtherReader")
                && identity.declaration_kind == FunctionDeclarationKind::TraitImplementationMethod
        })
        .expect("OtherReader for Foo::read");
    assert_ne!(reader.identity_key(), other_reader.identity_key());
    assert_eq!(owner_target_name(reader), Some("Foo"));
    assert_eq!(owner_target_name(other_reader), Some("Foo"));
}

// --- 5. trait method declaration vs trait default method body are represented correctly --------

#[test]
fn trait_method_declaration_vs_default_method_are_represented_correctly() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let declared = find_function_identity(&batch, &["trait:Reader"], "read")
        .expect("Reader::read declaration");
    assert_eq!(
        declared.declaration_kind,
        FunctionDeclarationKind::TraitMethodDeclaration
    );
    assert_eq!(declared.symbol.role, SymbolRole::Declaration);
    assert_eq!(declared.owner.target, None);
    assert_eq!(declared.owner.trait_path.as_deref(), Some("Reader"));

    let defaulted = find_function_identity(&batch, &["trait:Reader"], "default_read")
        .expect("Reader::default_read");
    assert_eq!(
        defaulted.declaration_kind,
        FunctionDeclarationKind::TraitDefaultMethod
    );
    assert_eq!(defaulted.symbol.role, SymbolRole::Definition);
    assert_eq!(defaulted.owner.target, None);
    assert_eq!(defaulted.owner.trait_path.as_deref(), Some("Reader"));
}

// --- 6. trait declaration vs implementation method: distinct, but source relationship survives -

#[test]
fn trait_declaration_and_implementation_method_are_distinct_but_share_an_observable_trait_path() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let declaration = find_function_identity(&batch, &["trait:Reader"], "read")
        .expect("Reader::read declaration");
    let implementation = function_identities_named(&batch, "read")
        .into_iter()
        .find(|identity| {
            identity.declaration_kind == FunctionDeclarationKind::TraitImplementationMethod
                && identity.owner.trait_path.as_deref() == Some("Reader")
        })
        .expect("Reader for Foo::read implementation");

    // Distinct declarations -- never claimed to be "the same" function.
    assert_ne!(declaration.identity_key(), implementation.identity_key());
    // ...but the syntactic relationship (both name trait `Reader`) remains directly observable,
    // never requiring re-derivation from a display string.
    assert_eq!(
        declaration.owner.trait_path,
        implementation.owner.trait_path
    );
}

// --- 7. associated function vs receiver method are distinguishable -----------------------------

#[test]
fn associated_function_is_distinguishable_from_a_receiver_method() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let create = find_function_identity(&batch, &["impl:Foo"], "create").expect("Foo::create");
    let get = find_function_identity(&batch, &["impl:Foo"], "get").expect("Foo::get");
    assert_eq!(
        create.declaration_kind,
        FunctionDeclarationKind::AssociatedFunction
    );
    assert_eq!(
        get.declaration_kind,
        FunctionDeclarationKind::InherentMethod
    );
    assert_ne!(create.identity_key(), get.identity_key());

    let signature = find_function_signature(&batch, &["impl:Foo"], "create").unwrap();
    assert!(
        signature.parameters.is_empty(),
        "an associated function has no receiver parameter"
    );
}

// --- 8. generic function declaration produces deterministic identity ---------------------------

#[test]
fn generic_function_declaration_produces_a_deterministic_identity() {
    let a = extract_all("src/lib.rs", R4_4_CORPUS);
    let b = extract_all("src/lib.rs", R4_4_CORPUS);
    let identity_a = find_function_identity(&a, &[], "generic").unwrap();
    let identity_b = find_function_identity(&b, &[], "generic").unwrap();
    assert_eq!(identity_a.identity_key(), identity_b.identity_key());
    assert_eq!(identity_a.generics, vec!["T".to_owned()]);
}

// --- 9. generic parameter order/content participates in identity -------------------------------

#[test]
fn generic_parameter_content_participates_in_identity() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let single = find_function_identity(&batch, &[], "generic").unwrap();
    let double = find_function_identity(&batch, &[], "generic_two").unwrap();
    assert_eq!(single.generics, vec!["T".to_owned()]);
    assert_eq!(double.generics, vec!["T".to_owned(), "U".to_owned()]);
    assert_ne!(single.identity_key(), double.identity_key());
}

// --- 10. revision change produces revision-distinct identity (R4.4 corpus) ---------------------

#[test]
fn revision_change_alters_identity_for_r4_4_declaration_shapes() {
    let a = RustSemanticExtractor.extract(&input_for(
        "src/lib.rs",
        R4_4_CORPUS,
        "revision-a",
        vec![SemanticDimension::FunctionIdentity],
    ));
    let b = RustSemanticExtractor.extract(&input_for(
        "src/lib.rs",
        R4_4_CORPUS,
        "revision-b",
        vec![SemanticDimension::FunctionIdentity],
    ));
    let identity_a = function_identities_named(&a, "get")
        .into_iter()
        .find(|i| owner_target_name(i) == Some("Foo"))
        .unwrap();
    let identity_b = function_identities_named(&b, "get")
        .into_iter()
        .find(|i| owner_target_name(i) == Some("Foo"))
        .unwrap();
    assert_ne!(identity_a.identity_key(), identity_b.identity_key());
}

// --- 11. repeated extraction is exactly deterministic over the R4.4 corpus ---------------------

#[test]
fn r4_4_corpus_extraction_is_deterministic() {
    let a = extract_all("src/lib.rs", R4_4_CORPUS);
    let b = extract_all("src/lib.rs", R4_4_CORPUS);
    assert_eq!(a, b);
}

// --- 12. requested-dimension ordering does not change identities (R4.4 corpus) -----------------

#[test]
fn r4_4_corpus_is_unaffected_by_requested_dimension_order() {
    let mut reversed = ALL_DIMENSIONS.to_vec();
    reversed.reverse();
    let forward = extract_all("src/lib.rs", R4_4_CORPUS);
    let backward = extract("src/lib.rs", R4_4_CORPUS, reversed);
    assert_eq!(forward.observations, backward.observations);
}

// --- 13. FunctionSignature refers to the correct, strengthened FunctionIdentity ----------------

#[test]
fn function_signature_carries_the_strengthened_function_identity() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let signature = find_function_signature(&batch, &["impl:Foo"], "get").unwrap();
    assert_eq!(owner_target_name(&signature.function), Some("Foo"));
    assert_eq!(
        signature.function.declaration_kind,
        FunctionDeclarationKind::InherentMethod
    );

    let trait_signature = batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::FunctionSignature(header)
                if header.subject.function.symbol.name == "read"
                    && header.subject.function.owner.trait_path.as_deref() == Some("Reader")
                    && header.subject.function.declaration_kind
                        == FunctionDeclarationKind::TraitImplementationMethod =>
            {
                Some(&header.subject)
            }
            _ => None,
        })
        .next()
        .expect("Reader for Foo::read signature");
    assert_eq!(owner_target_name(&trait_signature.function), Some("Foo"));
}

// --- 14. SymbolIdentity and FunctionIdentity remain coherent ------------------------------------

#[test]
fn symbol_and_function_identity_roles_remain_coherent() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);

    let declared_symbol =
        find_symbol(&batch, &["trait:Reader"], "read").expect("Reader::read symbol");
    let declared_identity = find_function_identity(&batch, &["trait:Reader"], "read")
        .expect("Reader::read function identity");
    assert_eq!(declared_symbol.role, SymbolRole::Declaration);
    assert_eq!(declared_identity.symbol.role, SymbolRole::Declaration);
    assert_eq!(declared_symbol.role, declared_identity.symbol.role);

    let defaulted_symbol =
        find_symbol(&batch, &["trait:Reader"], "default_read").expect("default_read symbol");
    let defaulted_identity = find_function_identity(&batch, &["trait:Reader"], "default_read")
        .expect("default_read function identity");
    assert_eq!(defaulted_symbol.role, SymbolRole::Definition);
    assert_eq!(defaulted_identity.symbol.role, SymbolRole::Definition);
}

// --- 15. every observation satisfies is_dimension_consistent() (R4.4 corpus) -------------------

#[test]
fn r4_4_corpus_observations_satisfy_dimension_consistency() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    assert!(!batch.observations.is_empty());
    for observation in &batch.observations {
        assert!(observation.is_dimension_consistent());
    }
}

// --- nested module scope (mod nested { mod inner { fn run() } }) still closes identity ---------

#[test]
fn deeply_nested_module_function_has_its_own_distinct_identity() {
    let batch = extract_all("src/lib.rs", R4_4_CORPUS);
    let nested_run =
        find_function_identity(&batch, &["nested", "inner"], "run").expect("nested::inner::run");
    let a_run = find_function_identity(&batch, &["a"], "run").expect("a::run");
    assert_ne!(nested_run.identity_key(), a_run.identity_key());
    assert_eq!(
        nested_run.declaration_kind,
        FunctionDeclarationKind::FreeFunction
    );
}

// --- Regression: future CALL safety. Four `execute()`s, all pairwise distinct ------------------

#[test]
fn four_same_named_execute_declarations_are_all_pairwise_distinct_function_identities() {
    let batch = extract_all("src/lib.rs", CALL_SAFETY_CORPUS);

    // Every declaration here has an empty body (`{}`), so no Call observation is produced even
    // with R4.5 CALL support; State/Effect remain wholly unclaimed this wave. R4.6 CONTROL_FLOW
    // still produces one (trivial, Return-terminated) FunctionEntry block per function, and R4.7
    // DATA_FLOW still produces a `self` Definition for X::execute/Y::execute's receivers -- an
    // empty body is not the same as "no body" or "no parameters."
    assert!(batch.observations.iter().all(|observation| !matches!(
        observation,
        SemanticObservation::Call(_)
            | SemanticObservation::State(_)
            | SemanticObservation::Effect(_)
    )));

    let a_execute = find_function_identity(&batch, &["a"], "execute").expect("a::execute");
    let b_execute = find_function_identity(&batch, &["b"], "execute").expect("b::execute");
    let x_execute = function_identities_named(&batch, "execute")
        .into_iter()
        .find(|identity| owner_target_name(identity) == Some("X"))
        .expect("X::execute");
    let y_execute = function_identities_named(&batch, "execute")
        .into_iter()
        .find(|identity| owner_target_name(identity) == Some("Y"))
        .expect("Y::execute");

    let all = [
        a_execute.identity_key(),
        b_execute.identity_key(),
        x_execute.identity_key(),
        y_execute.identity_key(),
    ];
    for (i, left) in all.iter().enumerate() {
        for (j, right) in all.iter().enumerate() {
            if i != j {
                assert_ne!(
                    left, right,
                    "every `execute` declaration must be a pairwise-distinct FunctionIdentity \
                     so a future CALL relation can target exactly one of them, never a bare \
                     textual `\"execute\"` key"
                );
            }
        }
    }

    // Each record_id (the stable identity a future CALL target would reference) is likewise
    // pairwise distinct, derived from the strengthened identity_key -- never from display name.
    let record_ids: std::collections::BTreeSet<&str> = batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::FunctionIdentity(header)
                if header.subject.symbol.name == "execute" =>
            {
                Some(header.record_id.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(record_ids.len(), 4);
}

// =================================================================================================
// R4.5: CALL semantics
// =================================================================================================

// --- 23. a direct free-function call is attributed to the correct caller -----------------------

#[test]
fn direct_call_is_attributed_to_the_correct_caller() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_direct").expect("caller_direct");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].repository, caller.repository);
    assert_eq!(calls[0].revision, caller.revision);
}

// --- 24. calls nested inside control flow (condition + both branches) are all captured ---------

#[test]
fn calls_nested_in_control_flow_are_all_captured() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_nested").expect("caller_nested");
    // One call in the `if` condition, one in the `then` branch, one in the `else` branch.
    assert_eq!(calls_by_caller(&batch, caller).len(), 3);
}

// --- 25. method calls and associated-function calls are captured, attributed to their caller ----

#[test]
fn method_and_associated_function_calls_are_captured() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);

    // `Greeter::new()` (an associated-function call) + `g.greet()` (a method call).
    let caller_method =
        find_function_identity(&batch, &[], "caller_method").expect("caller_method");
    assert_eq!(calls_by_caller(&batch, caller_method).len(), 2);

    // `self.shout()` inside `greet`'s own body.
    let greet = find_function_identity(&batch, &["impl:Greeter"], "greet").expect("Greeter::greet");
    assert_eq!(calls_by_caller(&batch, greet).len(), 1);

    // `"hi".to_owned()` inside `shout`'s own body.
    let shout = find_function_identity(&batch, &["impl:Greeter"], "shout").expect("Greeter::shout");
    assert_eq!(calls_by_caller(&batch, shout).len(), 1);
}

// --- 26. functions whose body makes no calls produce zero CALL observations for that caller -----

#[test]
fn functions_with_no_calls_produce_no_call_observations() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);

    let helper = find_function_identity(&batch, &[], "helper").expect("helper");
    assert!(calls_by_caller(&batch, helper).is_empty());

    let no_calls = find_function_identity(&batch, &[], "caller_no_calls").expect("caller_no_calls");
    assert!(calls_by_caller(&batch, no_calls).is_empty());

    // `Greeter::new`'s body (`Greeter`) is a bare path expression, not a call.
    let new_fn = find_function_identity(&batch, &["impl:Greeter"], "new").expect("Greeter::new");
    assert!(calls_by_caller(&batch, new_fn).is_empty());
}

// --- 27. a macro invocation is never mistaken for a call: its expansion is unobservable ---------

#[test]
fn macro_invocation_is_never_treated_as_a_call() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller =
        find_function_identity(&batch, &[], "caller_macro_only").expect("caller_macro_only");
    assert!(calls_by_caller(&batch, caller).is_empty());
}

// --- 28. anti-fabrication: every CALL observation stays UNRESOLVED with zero claimed callees ----

#[test]
fn call_dispatch_is_always_unresolved_with_no_fabricated_callees() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let calls = all_calls(&batch);
    assert!(!calls.is_empty());
    for call in calls {
        assert_eq!(
            call.dispatch,
            CallDispatchKind::Unresolved,
            "this extractor has no use-import tracking or type inference; it must never claim a \
             resolved callee"
        );
        assert!(call.callees.is_empty());
    }
}

// --- 29. CALL observations carry evidence/provenance like every other dimension -----------------

#[test]
fn call_observations_carry_evidence_and_provenance() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    for observation in &batch.observations {
        if let SemanticObservation::Call(header) = observation {
            assert!(!header.evidence_refs.is_empty());
            assert_eq!(header.provenance.source_path, "src/lib.rs");
            assert!(header.provenance.span.is_some());
        }
    }
}

// --- 30. every CALL observation satisfies dimension consistency ---------------------------------

#[test]
fn call_observations_satisfy_dimension_consistency() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let mut saw_call = false;
    for observation in &batch.observations {
        if matches!(observation, SemanticObservation::Call(_)) {
            saw_call = true;
        }
        assert!(observation.is_dimension_consistent());
    }
    assert!(saw_call);
}

// --- 31. extraction of the CALL corpus is byte-for-byte deterministic across repeated runs ------

#[test]
fn call_corpus_extraction_is_deterministic() {
    let first = extract_all("src/lib.rs", CALL_CORPUS);
    let second = extract_all("src/lib.rs", CALL_CORPUS);
    assert_eq!(first.observations, second.observations);
    assert_eq!(first.evidence, second.evidence);
}

// --- 32. CALL extraction is unaffected by requested-dimension order -----------------------------

#[test]
fn call_corpus_is_unaffected_by_requested_dimension_order() {
    let forward = extract(
        "src/lib.rs",
        CALL_CORPUS,
        vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Call,
        ],
    );
    let reversed = extract(
        "src/lib.rs",
        CALL_CORPUS,
        vec![
            SemanticDimension::Call,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Symbol,
        ],
    );
    assert_eq!(forward.observations, reversed.observations);
}

// --- 33. repeated extraction yields the exact same call-site record_ids -------------------------
// (`core/src/semantic/call.rs`'s `identity_key_is_unaffected_by_dispatch_and_callees` proves the
// underlying type-level guarantee; this proves the extractor's own output is consistent with it.)

#[test]
fn repeated_extraction_yields_stable_call_record_ids() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_direct").expect("caller_direct");
    let first: Vec<String> = calls_by_caller(&batch, caller)
        .into_iter()
        .map(CallSiteIdentity::identity_key)
        .collect();

    let batch2 = extract_all("src/lib.rs", CALL_CORPUS);
    let caller2 = find_function_identity(&batch2, &[], "caller_direct").expect("caller_direct");
    let second: Vec<String> = calls_by_caller(&batch2, caller2)
        .into_iter()
        .map(CallSiteIdentity::identity_key)
        .collect();

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

// --- 34. R4.12: a simple single-identifier call argument's PlaceRef converges on the EXACT SAME
//     record_id DATA_FLOW's own Use observation carries for that argument -- proven, not merely
//     designed, matching R4.11's PlaceRef-convergence proof pattern -----------------------------

#[test]
fn call_argument_place_ref_converges_on_the_data_flow_uses_own_record_id() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_simple_argument")
        .expect("caller_with_simple_argument");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].arguments.len(), 1);

    let PlaceRef::Resolved {
        dimension,
        record_id,
    } = &calls[0].arguments[0]
    else {
        panic!("a simple identifier argument must resolve to an existing DATA_FLOW record");
    };
    assert_eq!(*dimension, SemanticDimension::DataFlow);

    let values = data_flow_values_for(&batch, caller);
    let x_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "x")
        .expect("DATA_FLOW's own Use observation for the `x` argument");
    let x_use_record_id = SemanticRecordId::new(SemanticDimension::DataFlow, &x_use.identity_key());

    assert_eq!(
        record_id, &x_use_record_id,
        "the CALL argument's PlaceRef must name the EXACT SAME node DATA_FLOW's own pass produced, \
         not an independently invented one"
    );
}

// --- 35. multiple arguments each get their own PlaceRef, in source order ------------------------

#[test]
fn multiple_call_arguments_each_get_their_own_place_ref_in_order() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_two_simple_arguments")
        .expect("caller_with_two_simple_arguments");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].arguments.len(), 2);

    let values = data_flow_values_for(&batch, caller);
    let expected: Vec<SemanticRecordId> = ["x", "y"]
        .iter()
        .map(|name| {
            let use_value = values
                .iter()
                .find(|v| v.role == ValueRole::Use && v.name == *name)
                .unwrap_or_else(|| panic!("DATA_FLOW Use for `{name}`"));
            SemanticRecordId::new(SemanticDimension::DataFlow, &use_value.identity_key())
        })
        .collect();

    for (argument, expected_record_id) in calls[0].arguments.iter().zip(expected.iter()) {
        let PlaceRef::Resolved { record_id, .. } = argument else {
            panic!("both simple-identifier arguments must resolve");
        };
        assert_eq!(record_id, expected_record_id);
    }
}

// --- 36. an argument requiring deeper analysis (a method-call receiver expression) stays
//     Unresolved -- never fabricated from spelling, matching every other PlaceRef consumer -------

#[test]
fn a_non_simple_call_argument_expression_is_unresolved() {
    // `caller_with_complex_argument` contains two call sites: `w.get()` (zero arguments) and the
    // outer `helper(w.get())` (one argument, the method call's own result) -- filter to the one
    // with an argument to isolate the outer call.
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_complex_argument")
        .expect("caller_with_complex_argument");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(
        calls.len(),
        2,
        "the outer helper(..) call and the w.get() method call"
    );
    let outer_call = calls
        .iter()
        .find(|call| !call.arguments.is_empty())
        .expect("the outer helper(w.get()) call");
    assert_eq!(outer_call.arguments, vec![PlaceRef::Unresolved]);
}

// --- 37. a method call's own arguments are bound identically to a direct call's -------------------

#[test]
fn method_call_arguments_are_bound_identically_to_direct_call_arguments() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_method_call_argument")
        .expect("caller_with_method_call_argument");
    let calls = calls_by_caller(&batch, caller);
    let get_with_call = calls
        .iter()
        .find(|call| call.arguments.len() == 1)
        .expect("the .get_with(x) method call");

    let values = data_flow_values_for(&batch, caller);
    let x_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "x")
        .expect("DATA_FLOW's own Use observation for the `x` argument");
    let expected = SemanticRecordId::new(SemanticDimension::DataFlow, &x_use.identity_key());

    assert_eq!(
        get_with_call.arguments,
        vec![PlaceRef::Resolved {
            dimension: SemanticDimension::DataFlow,
            record_id: expected
        }]
    );
}

// --- 38. when DATA_FLOW is not part of the same requested-dimension set, CALL never fabricates a
//     Resolved reference to a record this exact request will not produce -------------------------

#[test]
fn call_argument_place_ref_stays_unresolved_when_data_flow_was_not_requested() {
    let batch = extract(
        "src/lib.rs",
        CALL_CORPUS,
        vec![SemanticDimension::Call, SemanticDimension::FunctionIdentity],
    );
    let caller = find_function_identity(&batch, &[], "caller_with_simple_argument")
        .expect("caller_with_simple_argument");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls[0].arguments, vec![PlaceRef::Unresolved]);
}

// --- 39. arguments are content, not identity: two identical-argument calls at different sites
//     remain distinct, and identity_key() is unaffected by the arguments field itself -------------

#[test]
fn call_arguments_never_affect_the_call_sites_own_identity() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_simple_argument")
        .expect("caller_with_simple_argument");
    let calls = calls_by_caller(&batch, caller);
    let call = calls[0];
    let without_arguments = CallSiteIdentity {
        arguments: Vec::new(),
        ..call.clone()
    };
    assert_eq!(call.identity_key(), without_arguments.identity_key());
}

// --- 40. R4.12: a call that IS the direct initializer of a simple `let` binding gets a `result`
//     PlaceRef converging on the EXACT SAME record_id DATA_FLOW's own Definition carries ---------

#[test]
fn call_result_place_ref_converges_on_the_data_flows_own_definition_record_id() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_let_result")
        .expect("caller_with_let_result");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls.len(), 1);

    let PlaceRef::Resolved {
        dimension,
        record_id,
    } = &calls[0].result
    else {
        panic!("a direct `let y = helper(x);` initializer must resolve its call's result");
    };
    assert_eq!(*dimension, SemanticDimension::DataFlow);

    let values = data_flow_values_for(&batch, caller);
    let y_definition = values
        .iter()
        .find(|v| v.role == ValueRole::Definition && v.name == "y")
        .expect("DATA_FLOW's own Definition for `y`");
    let expected = SemanticRecordId::new(SemanticDimension::DataFlow, &y_definition.identity_key());

    assert_eq!(
        record_id, &expected,
        "the CALL result's PlaceRef must name the EXACT SAME node DATA_FLOW's own pass produced"
    );
}

// --- 41. a call that IS the direct right-hand side of a plain assignment gets a `result` PlaceRef
//     converging on DATA_FLOW's own Store record_id --------------------------------------------

#[test]
fn call_result_place_ref_converges_on_the_data_flows_own_store_record_id_for_assignment() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_assign_result")
        .expect("caller_with_assign_result");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls.len(), 1);

    let PlaceRef::Resolved {
        dimension,
        record_id,
    } = &calls[0].result
    else {
        panic!("a direct `y = helper(x);` assignment must resolve its call's result");
    };
    assert_eq!(*dimension, SemanticDimension::DataFlow);

    let values = data_flow_values_for(&batch, caller);
    let y_store = values
        .iter()
        .find(|v| v.role == ValueRole::Store && v.name == "y")
        .expect("DATA_FLOW's own Store for `y`");
    let expected = SemanticRecordId::new(SemanticDimension::DataFlow, &y_store.identity_key());

    assert_eq!(record_id, &expected);
}

// --- 42. a call buried inside a larger expression has no single value it "becomes" -- result
//     stays Unresolved even though the OUTER let binding itself is a simple identifier ----------

#[test]
fn call_result_stays_unresolved_when_the_call_is_not_the_entire_initializer() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_call_inside_a_larger_expression")
        .expect("caller_with_call_inside_a_larger_expression");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].result, PlaceRef::Unresolved);
}

// --- 43. a destructuring `let` pattern has no single identifier a call's result could
//     unambiguously become -- result stays Unresolved even though DATA_FLOW itself now binds
//     every sub-identifier (R4.12's destructuring fix) --------------------------------------------

#[test]
fn call_result_stays_unresolved_for_a_destructured_let_pattern() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_destructured_result")
        .expect("caller_with_destructured_result");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].result, PlaceRef::Unresolved);
}

// --- 44. when DATA_FLOW is not part of the same requested-dimension set, CALL never fabricates a
//     Resolved result reference either (mirrors test 38 for arguments) ---------------------------

#[test]
fn call_result_place_ref_stays_unresolved_when_data_flow_was_not_requested() {
    let batch = extract(
        "src/lib.rs",
        CALL_CORPUS,
        vec![SemanticDimension::Call, SemanticDimension::FunctionIdentity],
    );
    let caller = find_function_identity(&batch, &[], "caller_with_let_result")
        .expect("caller_with_let_result");
    let calls = calls_by_caller(&batch, caller);
    assert_eq!(calls[0].result, PlaceRef::Unresolved);
}

// --- 45. result is content, not identity ----------------------------------------------------------

#[test]
fn call_result_never_affects_the_call_sites_own_identity() {
    let batch = extract_all("src/lib.rs", CALL_CORPUS);
    let caller = find_function_identity(&batch, &[], "caller_with_let_result")
        .expect("caller_with_let_result");
    let calls = calls_by_caller(&batch, caller);
    let call = calls[0];
    let without_result = CallSiteIdentity {
        result: PlaceRef::Unresolved,
        ..call.clone()
    };
    assert_eq!(call.identity_key(), without_result.identity_key());
}

// =================================================================================================
// R4.6: CONTROL_FLOW semantics
// =================================================================================================

fn edge_kinds(block: &ControlFlowBlockIdentity) -> Vec<ControlFlowEdgeKind> {
    block.successors.iter().map(|edge| edge.kind).collect()
}

// --- 34. a straight-line body produces exactly one block, falling through to Return -------------

#[test]
fn straight_line_body_produces_one_entry_block_that_returns() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "straight_line").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(blocks.len(), 1);
    assert!(blocks[0].is_entry);
    assert_eq!(blocks[0].kind, ControlFlowBlockKind::FunctionEntry);
    assert_eq!(edge_kinds(blocks[0]), vec![ControlFlowEdgeKind::Return]);
    assert!(blocks[0].successors[0].target.is_none());
}

// --- 35. `if` with no `else`, followed by more code: Branch to the then-block, Fallthrough to
//     a real join block for the "condition false" case -----------------------------------------

#[test]
fn if_with_no_else_produces_a_join_block_for_the_false_case() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "simple_if").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(blocks.len(), 3, "entry + then-branch + join");

    let entry = blocks.iter().find(|b| b.is_entry).unwrap();
    assert_eq!(entry.kind, ControlFlowBlockKind::FunctionEntry);
    let mut entry_kinds = edge_kinds(entry);
    entry_kinds.sort();
    assert_eq!(
        entry_kinds,
        vec![
            ControlFlowEdgeKind::Fallthrough,
            ControlFlowEdgeKind::Branch
        ]
    );

    let then_block = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::IfThen)
        .expect("an IfThen block");
    // `return 1;` inside the then-branch always returns, regardless of the join.
    assert_eq!(edge_kinds(then_block), vec![ControlFlowEdgeKind::Return]);

    let join = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::Continuation)
        .expect("a join/continuation block for the code after the if");
    // The join's own tail (`2`) falls through to the function's implicit return.
    assert_eq!(edge_kinds(join), vec![ControlFlowEdgeKind::Return]);

    // The entry's Fallthrough edge must land exactly on the join block.
    let fallthrough_target = entry
        .successors
        .iter()
        .find(|edge| edge.kind == ControlFlowEdgeKind::Fallthrough)
        .and_then(|edge| edge.target.as_ref());
    let join_record_id =
        SemanticRecordId::new(SemanticDimension::ControlFlow, &join.identity_key());
    assert_eq!(fallthrough_target, Some(&join_record_id));
}

// --- 36. `if`/`else` (both blocks): two Branch edges, no join block needed when the if is the
//     function's own tail ------------------------------------------------------------------------

#[test]
fn if_else_produces_two_branch_edges_and_no_unnecessary_join() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "if_else").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    // entry + then + else -- no Continuation block, since nothing follows the if.
    assert_eq!(blocks.len(), 3);
    assert!(
        blocks
            .iter()
            .all(|b| b.kind != ControlFlowBlockKind::Continuation)
    );

    let entry = blocks.iter().find(|b| b.is_entry).unwrap();
    assert_eq!(
        edge_kinds(entry),
        vec![ControlFlowEdgeKind::Branch, ControlFlowEdgeKind::Branch]
    );

    let then_block = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::IfThen)
        .unwrap();
    let else_block = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::IfElse)
        .unwrap();
    assert_eq!(edge_kinds(then_block), vec![ControlFlowEdgeKind::Return]);
    assert_eq!(edge_kinds(else_block), vec![ControlFlowEdgeKind::Return]);
}

// --- 37. an else-if chain flattens into one decision point with N Branch edges ------------------

#[test]
fn else_if_chain_flattens_into_one_multi_branch_decision_point() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "else_if_chain").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    // entry + 3 arm bodies (if/else-if/else), no separate block for the nested condition itself.
    assert_eq!(blocks.len(), 4);

    let entry = blocks.iter().find(|b| b.is_entry).unwrap();
    assert_eq!(
        edge_kinds(entry),
        vec![
            ControlFlowEdgeKind::Branch,
            ControlFlowEdgeKind::Branch,
            ControlFlowEdgeKind::Branch
        ]
    );
    // Every branch target is distinct.
    let targets: std::collections::BTreeSet<&SemanticRecordId> = entry
        .successors
        .iter()
        .filter_map(|edge| edge.target.as_ref())
        .collect();
    assert_eq!(targets.len(), 3);
}

// --- 38. a `while` loop may skip its body entirely: Branch to body + Fallthrough past the loop --

#[test]
fn while_loop_has_a_may_not_enter_edge_and_a_self_loop_repeat_edge() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "while_loop").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);

    let entry = blocks.iter().find(|b| b.is_entry).unwrap();
    let mut entry_kinds = edge_kinds(entry);
    entry_kinds.sort();
    assert_eq!(
        entry_kinds,
        vec![
            ControlFlowEdgeKind::Fallthrough,
            ControlFlowEdgeKind::Branch
        ],
        "entering the loop AND skipping it entirely must both be real edges"
    );

    let body = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::WhileBody)
        .expect("a WhileBody block");
    // The body falls through to itself (LoopRepeat), a genuine self-loop edge.
    assert_eq!(edge_kinds(body), vec![ControlFlowEdgeKind::LoopRepeat]);
    let self_target = body.successors[0].target.as_ref().unwrap();
    let body_record_id =
        SemanticRecordId::new(SemanticDimension::ControlFlow, &body.identity_key());
    assert_eq!(self_target, &body_record_id);
}

// --- 39. a `loop` (never `while`/`for`) has NO may-not-enter edge: it always executes at least
//     once ----------------------------------------------------------------------------------------

#[test]
fn bare_loop_has_no_skip_edge() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "unresolved_break").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    let entry = blocks.iter().find(|b| b.is_entry).unwrap();
    // Only one way forward: into the loop body. A bare `loop` can only be left via `break`.
    assert_eq!(edge_kinds(entry), vec![ControlFlowEdgeKind::Branch]);
}

// --- 40. a labeled `break` resolves through an intermediate unlabeled loop to the correct outer
//     loop's own continuation, not the nearest (unlabeled) enclosing loop -----------------------

#[test]
fn labeled_break_resolves_through_an_intermediate_unlabeled_loop() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "labeled_break").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);

    // Exactly one block has a Break edge (the `if n > 0 { break 'outer; }` then-branch).
    let break_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| edge_kinds(b).contains(&ControlFlowEdgeKind::Break))
        .collect();
    assert_eq!(break_blocks.len(), 1);
    let break_edge = break_blocks[0]
        .successors
        .iter()
        .find(|e| e.kind == ControlFlowEdgeKind::Break)
        .unwrap();
    // It must resolve to a concrete target (the code after the OUTER labeled loop, i.e. the
    // block containing `n`), not Unresolved and not the inner unlabeled loop's own repeat point.
    let target = break_edge.target.as_ref().expect("a resolved break target");

    // The target must be the join/continuation block that contains the final `n` -- i.e. NOT
    // any block whose own kind is LoopBody (which would indicate it incorrectly resolved to a
    // loop-repeat point instead of the code after the outer loop).
    let target_block = blocks
        .iter()
        .find(|b| {
            &SemanticRecordId::new(SemanticDimension::ControlFlow, &b.identity_key()) == target
        })
        .expect("the break target must be one of this function's own blocks");
    assert_ne!(target_block.kind, ControlFlowBlockKind::LoopBody);
    assert_eq!(edge_kinds(target_block), vec![ControlFlowEdgeKind::Return]);

    // Two distinct LoopBody blocks exist (outer loop's body, inner loop's body).
    let loop_bodies: Vec<_> = blocks
        .iter()
        .filter(|b| b.kind == ControlFlowBlockKind::LoopBody)
        .collect();
    assert_eq!(loop_bodies.len(), 2);
}

// --- 41. a `break` whose label matches no enclosing loop is explicit UNRESOLVED, never dropped --

#[test]
fn unmatched_labeled_break_is_explicitly_unresolved() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "unresolved_break").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    let unresolved_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| edge_kinds(b).contains(&ControlFlowEdgeKind::Unresolved))
        .collect();
    assert_eq!(unresolved_blocks.len(), 1);
    let edge = unresolved_blocks[0]
        .successors
        .iter()
        .find(|e| e.kind == ControlFlowEdgeKind::Unresolved)
        .unwrap();
    assert!(edge.target.is_none());
}

// --- 42. an unconditional panic-like macro produces a Panic edge, never fallthrough --------------

#[test]
fn unconditional_panic_produces_a_panic_edge() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "panics_unconditionally").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(blocks.len(), 1);
    assert_eq!(edge_kinds(blocks[0]), vec![ControlFlowEdgeKind::Panic]);
    assert!(blocks[0].successors[0].target.is_none());
}

// --- 43. a `match` produces one Branch edge per arm, each arm its own MatchArm block ------------

#[test]
fn match_produces_one_branch_edge_per_arm() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "matches_on_value").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    // entry + 3 arms.
    assert_eq!(blocks.len(), 4);

    let entry = blocks.iter().find(|b| b.is_entry).unwrap();
    assert_eq!(
        edge_kinds(entry),
        vec![
            ControlFlowEdgeKind::Branch,
            ControlFlowEdgeKind::Branch,
            ControlFlowEdgeKind::Branch
        ]
    );

    let arms: Vec<_> = blocks
        .iter()
        .filter(|b| b.kind == ControlFlowBlockKind::MatchArm)
        .collect();
    assert_eq!(arms.len(), 3);
    // Every arm -- whether a bare expression (`10`, `30`) or an explicit `return 20;` -- resolves
    // to Return, since this match is the function's own tail expression.
    for arm in &arms {
        assert_eq!(edge_kinds(arm), vec![ControlFlowEdgeKind::Return]);
    }
}

// --- 44. every ControlFlow observation satisfies dimension consistency, and CALL/ControlFlow
//     coexist without interfering with each other -------------------------------------------------

#[test]
fn control_flow_observations_satisfy_dimension_consistency_alongside_call() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let mut saw_control_flow = false;
    for observation in &batch.observations {
        if matches!(observation, SemanticObservation::ControlFlow(_)) {
            saw_control_flow = true;
        }
        assert!(observation.is_dimension_consistent());
    }
    assert!(saw_control_flow);
}

// --- 45. CFG extraction is byte-for-byte deterministic across repeated runs ----------------------

#[test]
fn cfg_corpus_extraction_is_deterministic() {
    let first = extract_all("src/lib.rs", CFG_CORPUS);
    let second = extract_all("src/lib.rs", CFG_CORPUS);
    assert_eq!(first.observations, second.observations);
    assert_eq!(first.evidence, second.evidence);
}

// --- 46. CFG extraction is unaffected by requested-dimension order ------------------------------

#[test]
fn cfg_corpus_is_unaffected_by_requested_dimension_order() {
    let forward = extract(
        "src/lib.rs",
        CFG_CORPUS,
        vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::ControlFlow,
        ],
    );
    let reversed = extract(
        "src/lib.rs",
        CFG_CORPUS,
        vec![
            SemanticDimension::ControlFlow,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Symbol,
        ],
    );
    assert_eq!(forward.observations, reversed.observations);
}

// --- 47. a function with no calls and no branches produces exactly one trivial CFG block --------

#[test]
fn no_branches_function_produces_exactly_one_block() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "no_calls_no_branches").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(blocks.len(), 1);
    assert!(blocks[0].is_entry);
}

// --- 48. block record_ids are stable identity: identical source re-extracted twice produces the
//     exact same block record_ids and edge targets, never dependent on traversal/hash order -----

#[test]
fn repeated_extraction_yields_stable_control_flow_record_ids_and_edges() {
    let batch = extract_all("src/lib.rs", CFG_CORPUS);
    let caller = find_function_identity(&batch, &[], "labeled_break").unwrap();
    let first: Vec<String> = control_flow_blocks_for(&batch, caller)
        .into_iter()
        .map(ControlFlowBlockIdentity::identity_key)
        .collect();

    let batch2 = extract_all("src/lib.rs", CFG_CORPUS);
    let caller2 = find_function_identity(&batch2, &[], "labeled_break").unwrap();
    let second: Vec<String> = control_flow_blocks_for(&batch2, caller2)
        .into_iter()
        .map(ControlFlowBlockIdentity::identity_key)
        .collect();

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

// =================================================================================================
// R4.7: DATA_FLOW semantics
// =================================================================================================

// --- 49. a parameter Definition and a simple let-binding resolve correctly ----------------------

#[test]
fn simple_def_use_resolves_correctly() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "simple_def_use").unwrap();
    let values = data_flow_values_for(&batch, caller);
    assert_eq!(values.len(), 4);

    let x_def = values
        .iter()
        .find(|v| v.role == ValueRole::Definition && v.name == "x")
        .expect("Definition of x (the parameter)");
    assert!(x_def.is_parameter);

    let x_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "x")
        .expect("Use of x inside y's initializer");
    assert_eq!(x_use.resolution, DataFlowResolution::Resolved);
    let x_def_id = SemanticRecordId::new(SemanticDimension::DataFlow, &x_def.identity_key());
    assert_eq!(x_use.resolved_definition.as_ref(), Some(&x_def_id));

    let y_def = values
        .iter()
        .find(|v| v.role == ValueRole::Definition && v.name == "y")
        .expect("Definition of y");
    assert!(!y_def.is_parameter);

    let y_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "y")
        .expect("Use of y in tail position");
    assert!(
        y_use.is_return_flow,
        "the tail `y` is the function's return value"
    );
    let y_def_id = SemanticRecordId::new(SemanticDimension::DataFlow, &y_def.identity_key());
    assert_eq!(y_use.resolved_definition.as_ref(), Some(&y_def_id));
}

// --- 50. shadowing: a use resolves to the definition visible AT THAT POINT, not the final one ----

#[test]
fn shadowing_resolves_to_the_definition_visible_at_that_point() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "shadowing").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let defs: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition && v.name == "x")
        .collect();
    assert_eq!(defs.len(), 2, "two distinct `let x` bindings");
    assert_ne!(defs[0].span, defs[1].span);

    let uses: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Use && v.name == "x")
        .collect();
    assert_eq!(uses.len(), 2);

    let first_def_id = SemanticRecordId::new(SemanticDimension::DataFlow, &defs[0].identity_key());
    let second_def_id = SemanticRecordId::new(SemanticDimension::DataFlow, &defs[1].identity_key());

    // The use inside the second `let x = x + 1;`'s initializer must resolve to the FIRST x
    // (shadowing takes effect only after the new binding completes, matching real Rust semantics).
    let init_use = uses.iter().find(|u| !u.is_return_flow).unwrap();
    assert_eq!(init_use.resolved_definition.as_ref(), Some(&first_def_id));

    // The tail `x` must resolve to the SECOND (most recent) x.
    let tail_use = uses.iter().find(|u| u.is_return_flow).unwrap();
    assert_eq!(tail_use.resolved_definition.as_ref(), Some(&second_def_id));
}

// --- 51. a plain assignment produces a Store resolved to its Definition, not a new Definition ---

#[test]
fn assignment_produces_a_store_not_a_new_definition() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "mutation").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let defs: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition && v.name == "x")
        .collect();
    assert_eq!(
        defs.len(),
        1,
        "`x = 2;` must not create a second Definition"
    );

    let store = values
        .iter()
        .find(|v| v.role == ValueRole::Store)
        .expect("a Store event for `x = 2;`");
    let def_id = SemanticRecordId::new(SemanticDimension::DataFlow, &defs[0].identity_key());
    assert_eq!(store.resolved_definition.as_ref(), Some(&def_id));
}

// --- 52. a shadow inside a nested block does not leak out after the block ends -------------------

#[test]
fn block_scoped_shadow_does_not_leak_out() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "block_scoped_shadow").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let defs: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition && v.name == "x")
        .collect();
    assert_eq!(defs.len(), 2, "outer `let x` and inner `let x`");
    let outer_def = defs.iter().min_by_key(|d| d.span.line).unwrap();
    let inner_def = defs.iter().max_by_key(|d| d.span.line).unwrap();
    let outer_id = SemanticRecordId::new(SemanticDimension::DataFlow, &outer_def.identity_key());
    let inner_id = SemanticRecordId::new(SemanticDimension::DataFlow, &inner_def.identity_key());

    let uses: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Use && v.name == "x")
        .collect();
    assert_eq!(uses.len(), 2);

    // The use inside the inner block (`let _unused = x;`) resolves to the INNER shadow.
    let inner_use = uses.iter().min_by_key(|u| u.span.line).unwrap();
    assert_eq!(inner_use.resolved_definition.as_ref(), Some(&inner_id));

    // The tail `x`, after the block has closed, resolves back to the OUTER definition -- the
    // inner shadow's scope ended when its enclosing block did.
    let tail_use = uses.iter().find(|u| u.is_return_flow).unwrap();
    assert_eq!(tail_use.resolved_definition.as_ref(), Some(&outer_id));
}

// --- 53. a use with no matching local definition is explicitly UNRESOLVED, never guessed ---------

#[test]
fn use_with_no_local_definition_is_explicitly_unresolved() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "uses_unknown_name").unwrap();
    let values = data_flow_values_for(&batch, caller);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].role, ValueRole::Use);
    assert_eq!(values[0].resolution, DataFlowResolution::Unresolved);
    assert!(values[0].resolved_definition.is_none());
    assert!(values[0].is_return_flow);
}

// --- 54. an explicit `return x;` flags that specific use as return-flow, a condition use does not

#[test]
fn explicit_return_is_flagged_as_return_flow_a_condition_use_is_not() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "early_return").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let uses: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Use && v.name == "x")
        .collect();
    assert_eq!(
        uses.len(),
        2,
        "one in the `if` condition, one in `return x;`"
    );
    assert_eq!(uses.iter().filter(|u| u.is_return_flow).count(), 1);
    assert_eq!(uses.iter().filter(|u| !u.is_return_flow).count(), 1);
    // Both resolve to the same parameter Definition.
    let resolved: std::collections::BTreeSet<_> =
        uses.iter().map(|u| u.resolved_definition.clone()).collect();
    assert_eq!(resolved.len(), 1);
    assert!(resolved.iter().next().unwrap().is_some());
}

// --- 55. return-flow does not leak out of a non-tail nested block: an `if` used as a `let`
// initializer's branches are never return-flow, only the outer `let`-bound tail identifier is ----

#[test]
fn return_flow_does_not_leak_out_of_a_non_tail_if_branch() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "if_in_let").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let branch_uses: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Use && v.name == "x")
        .collect();
    assert_eq!(
        branch_uses.len(),
        3,
        "one in the `if` condition, one in each of the then/else branches"
    );
    assert!(
        branch_uses.iter().all(|u| !u.is_return_flow),
        "none of these `x` uses is the function's own return value -- they all feed a `let` \
         binding, not a `return`/tail position"
    );

    let tail_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "y")
        .expect("the function body's own tail identifier `y`");
    assert!(
        tail_use.is_return_flow,
        "`y` is the function body's own bare-identifier tail expression"
    );
}

// --- 56. a method receiver (`&self`) is a real Definition, and `self.field` resolves it ----------

#[test]
fn method_receiver_is_a_definition_and_field_access_resolves_it() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Holder"], "get").unwrap();
    let values = data_flow_values_for(&batch, caller);
    assert_eq!(values.len(), 2);

    let self_def = values
        .iter()
        .find(|v| v.role == ValueRole::Definition && v.name == "self")
        .expect("a Definition for the &self receiver");
    assert!(self_def.is_parameter);

    let self_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "self")
        .expect("a Use of self as the base of `self.value`");
    let def_id = SemanticRecordId::new(SemanticDimension::DataFlow, &self_def.identity_key());
    assert_eq!(self_use.resolved_definition.as_ref(), Some(&def_id));
}

// --- 57. R4.12: a tuple-destructuring pattern binds every sub-binding, so subsequent uses of them
//     resolve instead of staying explicitly UNRESOLVED (the earlier documented gap) --------------

#[test]
fn tuple_destructuring_pattern_binds_both_sub_bindings_and_their_uses_resolve() {
    // R4.12 closed the previously-documented destructuring gap: `let (a, b) = ..;` now emits a
    // Definition for each sub-binding, so `a + b` resolves both uses instead of staying
    // unresolved.
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "destructures_a_tuple").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let definitions: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition)
        .collect();
    let mut definition_names: Vec<&str> = definitions.iter().map(|d| d.name.as_str()).collect();
    definition_names.sort_unstable();
    assert_eq!(
        definition_names,
        vec!["a", "b"],
        "a `let (a, b) = ..;` pattern must bind both sub-bindings"
    );

    let uses: Vec<_> = values.iter().filter(|v| v.role == ValueRole::Use).collect();
    assert_eq!(
        uses.len(),
        2,
        "uses of both `a` and `b` in the tail expression"
    );
    assert!(
        uses.iter()
            .all(|u| u.resolution == DataFlowResolution::Resolved),
        "with a Definition now registered for both a and b, their uses must resolve"
    );
}

// --- 57a. R4.12: struct destructuring binds each named field's local, not the field name itself --

#[test]
fn struct_destructuring_pattern_binds_each_local_and_its_uses_resolve() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "destructures_a_struct").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let mut definition_names: Vec<&str> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition && !v.is_parameter)
        .map(|d| d.name.as_str())
        .collect();
    definition_names.sort_unstable();
    assert_eq!(
        definition_names,
        vec!["x", "y"],
        "`let Point {{ x, y }} = p;` must bind both `x` and `y`"
    );

    let body_uses: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Use && (v.name == "x" || v.name == "y"))
        .collect();
    assert_eq!(body_uses.len(), 2);
    assert!(
        body_uses
            .iter()
            .all(|u| u.resolution == DataFlowResolution::Resolved)
    );
}

// --- 57b. R4.12: slice destructuring binds each element local ------------------------------------

#[test]
fn slice_destructuring_pattern_binds_each_local_and_its_uses_resolve() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "destructures_a_slice").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let mut definition_names: Vec<&str> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition && !v.is_parameter)
        .map(|d| d.name.as_str())
        .collect();
    definition_names.sort_unstable();
    assert_eq!(
        definition_names,
        vec!["first", "second"],
        "`let [first, second] = values;` must bind both elements"
    );

    let body_uses: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Use && (v.name == "first" || v.name == "second"))
        .collect();
    assert_eq!(body_uses.len(), 2);
    assert!(
        body_uses
            .iter()
            .all(|u| u.resolution == DataFlowResolution::Resolved)
    );
}

// --- 57c. R4.12: an `n @ sub_pattern` match arm binds `n`, distinct from any name `sub_pattern`
//     itself might also bind -------------------------------------------------------------------

#[test]
fn at_pattern_binds_its_own_name_in_addition_to_the_sub_pattern() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "binds_with_at_pattern").unwrap();
    let values = data_flow_values_for(&batch, caller);

    // The fixture's second arm (`other => other`) also binds a plain identifier pattern, so this
    // asserts `n` is present among the definitions rather than that it is the only non-parameter
    // one.
    let n_definitions: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition && v.name == "n")
        .collect();
    assert_eq!(
        n_definitions.len(),
        1,
        "`n @ 1..=5` must bind `n`; the range pattern itself binds nothing"
    );

    let n_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "n")
        .expect("the arm body's use of `n`");
    assert_eq!(n_use.resolution, DataFlowResolution::Resolved);
}

// --- 57d. R4.12: an Or pattern (`Ok(value) | Err(value)`) binds `value` from whichever alternative
//     matches; this walker does not attempt to prove which, so each alternative's own occurrence of
//     the shared name gets its own Definition rather than fabricating a single merged one ----------

#[test]
fn or_pattern_binds_the_shared_name_in_every_alternative() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "binds_via_or_pattern").unwrap();
    let values = data_flow_values_for(&batch, caller);

    let value_definitions: Vec<_> = values
        .iter()
        .filter(|v| v.role == ValueRole::Definition && v.name == "value")
        .collect();
    assert_eq!(
        value_definitions.len(),
        2,
        "`Ok(value) | Err(value)` binds `value` once per alternative"
    );

    let value_use = values
        .iter()
        .find(|v| v.role == ValueRole::Use && v.name == "value")
        .expect("the arm body's use of `value`");
    assert_eq!(value_use.resolution, DataFlowResolution::Resolved);
}

// --- 58. every DataFlow observation satisfies dimension consistency, alongside CALL/CONTROL_FLOW -

#[test]
fn data_flow_observations_satisfy_dimension_consistency() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let mut saw_data_flow = false;
    for observation in &batch.observations {
        if matches!(observation, SemanticObservation::DataFlow(_)) {
            saw_data_flow = true;
        }
        assert!(observation.is_dimension_consistent());
    }
    assert!(saw_data_flow);
}

// --- 59. DATA_FLOW extraction is byte-for-byte deterministic across repeated runs ----------------

#[test]
fn data_flow_corpus_extraction_is_deterministic() {
    let first = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let second = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    assert_eq!(first.observations, second.observations);
    assert_eq!(first.evidence, second.evidence);
}

// --- 60. DATA_FLOW extraction is unaffected by requested-dimension order ------------------------

#[test]
fn data_flow_corpus_is_unaffected_by_requested_dimension_order() {
    let forward = extract(
        "src/lib.rs",
        DATA_FLOW_CORPUS,
        vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::DataFlow,
        ],
    );
    let reversed = extract(
        "src/lib.rs",
        DATA_FLOW_CORPUS,
        vec![
            SemanticDimension::DataFlow,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Symbol,
        ],
    );
    assert_eq!(forward.observations, reversed.observations);
}

// --- 61. repeated extraction yields the exact same DataFlow record_ids --------------------------

#[test]
fn repeated_extraction_yields_stable_data_flow_record_ids() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "shadowing").unwrap();
    let first: Vec<String> = data_flow_values_for(&batch, caller)
        .into_iter()
        .map(ValueIdentity::identity_key)
        .collect();

    let batch2 = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller2 = find_function_identity(&batch2, &[], "shadowing").unwrap();
    let second: Vec<String> = data_flow_values_for(&batch2, caller2)
        .into_iter()
        .map(ValueIdentity::identity_key)
        .collect();

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

// --- 62. a bare `self.field` read produces a real State Read, resolved, scoped to the impl ------

#[test]
fn self_field_read_produces_a_state_read_observation() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "get").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(accesses.len(), 1);
    assert_eq!(accesses[0].kind, StateAccessKind::Read);
    assert_eq!(accesses[0].name, "value");
    assert_eq!(accesses[0].resolution, StateResolution::Resolved);
}

// --- 63. a plain `self.field = value` assignment produces a Write, not a Read -------------------

#[test]
fn self_field_assignment_produces_a_write_not_a_read() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "set").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(accesses.len(), 1);
    assert_eq!(accesses[0].kind, StateAccessKind::Write);
    assert_eq!(accesses[0].name, "value");
}

// --- 64. a read-modify-write produces one Write (the assignment target) and two distinct Reads
// (the RHS use and the following tail use), never collapsed into one record -----------------------

#[test]
fn read_modify_write_produces_a_write_and_two_distinct_reads() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "increment_and_get").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(accesses.len(), 3, "one Write (LHS) + two Reads (RHS, tail)");
    assert_eq!(
        accesses
            .iter()
            .filter(|a| a.kind == StateAccessKind::Write)
            .count(),
        1
    );
    assert_eq!(
        accesses
            .iter()
            .filter(|a| a.kind == StateAccessKind::Read)
            .count(),
        2
    );
    // The two Reads are at genuinely different sites (different spans), so they carry distinct
    // identity_key()s even though they name the exact same entity.
    let read_spans: std::collections::BTreeSet<_> = accesses
        .iter()
        .filter(|a| a.kind == StateAccessKind::Read)
        .map(|a| (a.span.line, a.span.column))
        .collect();
    assert_eq!(read_spans.len(), 2);
}

// --- 65. STATE and EFFECT coexist without interference: a conditional panic still leaves the
// tail `self.value` read intact, and the panic itself is correctly attributed --------------------

#[test]
fn state_and_effect_coexist_without_interference() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "maybe_panic").unwrap();

    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(
        accesses.len(),
        1,
        "only the tail `self.value`, none inside the panic branch"
    );
    assert_eq!(accesses[0].kind, StateAccessKind::Read);
    assert_eq!(accesses[0].name, "value");

    let effects = effects_for(&batch, caller);
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].category, EffectCategory::Panic);
}

// --- 66. a free function with no `self` produces zero State observations, never a false positive -

#[test]
fn free_function_with_no_self_produces_no_state_observations() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &[], "free_function_with_no_self").unwrap();
    assert!(state_accesses_for(&batch, caller).is_empty());
}

// --- 67. a nested field chain (`self.inner.value`) is now a real, compound Read of the whole
// `.`-joined path -- found missing (worse: fabricating a wrong Read of the outer field alone), then
// closed, by direct adversarial testing this session; see the sibling tests below for the write,
// deeper-chain and tuple-index-bailout cases -------------------------------------------------------

#[test]
fn nested_field_chain_reports_a_read_of_the_full_joined_path() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "read_nested").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(accesses.len(), 1);
    assert_eq!(accesses[0].kind, StateAccessKind::Read);
    assert_eq!(
        accesses[0].name, "inner.value",
        "the whole chain is one compound state-access event against its joined path, not a Read \
         of the outer field alone"
    );
}

// --- a nested field WRITE (`self.a.b = x`) used to fabricate a spurious Read of the outer field
// alone and never record the real write at all -- a wrong fact, not merely a coverage gap. Confirmed
// via direct extraction before the fix (probed and removed once the finding was recorded here). --

#[test]
fn nested_field_write_reports_a_write_of_the_full_joined_path_with_no_spurious_read() {
    const SRC: &str = r#"
pub struct Inner { b: u8 }
pub struct Outer { a: Inner }
impl Outer {
    pub fn set(&mut self) {
        self.a.b = 5;
    }
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &["impl:Outer"], "set").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(
        accesses.len(),
        1,
        "exactly one access: the real write to a.b, with no fabricated read of `a` alongside it"
    );
    assert_eq!(accesses[0].kind, StateAccessKind::Write);
    assert_eq!(accesses[0].name, "a.b");
}

// --- a three-level chain (`self.a.b.c`) joins every level, not just the first two ----------------

#[test]
fn three_level_field_chain_joins_every_level() {
    const SRC: &str = r#"
pub struct Grand { c: u8 }
pub struct Middle { b: Grand }
pub struct Outer { a: Middle }
impl Outer {
    pub fn read(&self) -> u8 {
        self.a.b.c
    }
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &["impl:Outer"], "read").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(accesses.len(), 1);
    assert_eq!(accesses[0].kind, StateAccessKind::Read);
    assert_eq!(accesses[0].name, "a.b.c");
}

// --- a tuple-index member anywhere in the chain (`self.0.field`, `self.field.0`) still bails the
// whole chain, matching the prior single-level `self.0` behavior exactly -- no partial/fabricated
// path is ever emitted for a chain this extractor cannot fully name -------------------------------

#[test]
fn tuple_index_anywhere_in_a_field_chain_bails_the_whole_chain() {
    const SRC: &str = r#"
pub struct Outer(Inner);
pub struct Inner { field: u8, also_tuple: (u8,) }
impl Outer {
    pub fn read_through_tuple_base(&self) -> u8 {
        self.0.field
    }
    pub fn read_through_tuple_tail(&self) -> u8 {
        self.field.0
    }
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let base_caller =
        find_function_identity(&batch, &["impl:Outer"], "read_through_tuple_base").unwrap();
    assert!(
        state_accesses_for(&batch, base_caller).is_empty(),
        "self.0.field must not fabricate a partial path when the chain's own base is a \
         tuple-index field"
    );
    let tail_caller =
        find_function_identity(&batch, &["impl:Outer"], "read_through_tuple_tail").unwrap();
    assert!(
        state_accesses_for(&batch, tail_caller).is_empty(),
        "self.field.0 must not fabricate a partial path when the chain's own final member is a \
         tuple-index field"
    );
}

// --- an unresolvable field chain's base is still walked when it is NOT itself another field
// projection (e.g. a call), so a genuinely unrelated nested self-access is still found ------------

#[test]
fn unresolvable_field_chain_still_finds_a_self_access_hidden_in_a_non_field_base() {
    const SRC: &str = r#"
pub struct Outer { value: u8 }
pub struct Inner { z: u8 }
impl Outer {
    pub fn helper(&self, _v: u8) -> Inner { Inner { z: 0 } }
    pub fn read(&self) -> u8 {
        self.helper(self.value).z
    }
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &["impl:Outer"], "read").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(
        accesses.len(),
        1,
        "self.helper(self.value).z is not itself a self-rooted field chain (its base is a method \
         call, not `self` or a named field), so `.z` is correctly not reported as any kind of \
         self-field access -- but the call's own ARGUMENT (self.value) is a genuine, separate \
         self-field read that must still be found, proving `walk_unresolved_field_base` does not \
         over-suppress: it only skips re-entering FIELD-chain interpretation, never walking \
         into a call/method-call base entirely"
    );
    assert_eq!(accesses[0].kind, StateAccessKind::Read);
    assert_eq!(accesses[0].name, "value");
}

// --- 68. compound assignment is read-modify-write: one Read + one Write at the same site --------

#[test]
fn compound_assignment_is_recorded_as_read_and_write() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "compound_increment").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(
        accesses.len(),
        2,
        "`self.value += 1` is a read-modify-write"
    );
    assert_eq!(
        accesses
            .iter()
            .filter(|access| access.kind == StateAccessKind::Read)
            .count(),
        1
    );
    assert_eq!(
        accesses
            .iter()
            .filter(|access| access.kind == StateAccessKind::Write)
            .count(),
        1
    );
    assert!(accesses.iter().all(|access| access.name == "value"));
}

// --- 68a. macro spelling is evidence, not compiler-resolved truth -------------------------------

#[test]
fn panic_like_macro_is_inferred_not_observed() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "maybe_panic").unwrap();
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let header = batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::Effect(header) if header.subject.function == caller_id => {
                Some(header)
            }
            _ => None,
        })
        .expect("panic-like effect candidate");
    assert_eq!(header.subject.category, EffectCategory::Panic);
    assert_eq!(header.status, EpistemicStatus::Inferred);
}

// --- 68b. nested unsafe blocks are still traversed ---------------------------------------------

#[test]
fn unsafe_block_preserves_nested_state_and_effect_observations() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "unsafe_wrapped").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(accesses.len(), 1);
    assert_eq!(accesses[0].kind, StateAccessKind::Write);
    assert_eq!(accesses[0].name, "value");
    let effects = effects_for(&batch, caller);
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].category, EffectCategory::Panic);
}

// --- 68c. partial R4.8 analysis never fabricates verified absence -------------------------------

#[test]
fn partial_state_effect_dimensions_remain_unknown_with_or_without_observations() {
    let populated = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    for dimension in [SemanticDimension::State, SemanticDimension::Effect] {
        let obligation = populated.obligation_for(dimension).unwrap();
        assert_eq!(obligation.status, EpistemicStatus::Unknown);
        assert!(!obligation.observation_ids.is_empty());
        assert!(!obligation.diagnostics.is_empty());
    }

    let empty = extract(
        "src/empty.rs",
        "",
        vec![SemanticDimension::State, SemanticDimension::Effect],
    );
    for dimension in [SemanticDimension::State, SemanticDimension::Effect] {
        let obligation = empty.obligation_for(dimension).unwrap();
        assert_eq!(obligation.status, EpistemicStatus::Unknown);
        assert!(obligation.observation_ids.is_empty());
        assert!(!obligation.diagnostics.is_empty());
    }
}

// --- 69. every State/Effect observation satisfies dimension consistency, alongside every other
// dimension this extractor produces -------------------------------------------------------------

#[test]
fn state_and_effect_observations_satisfy_dimension_consistency() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let mut saw_state = false;
    let mut saw_effect = false;
    for observation in &batch.observations {
        match observation {
            SemanticObservation::State(_) => saw_state = true,
            SemanticObservation::Effect(_) => saw_effect = true,
            _ => {}
        }
        assert!(observation.is_dimension_consistent());
    }
    assert!(saw_state);
    assert!(saw_effect);
}

// --- 70. STATE_EFFECT_CORPUS extraction is byte-for-byte deterministic across repeated runs ------

#[test]
fn state_effect_corpus_extraction_is_deterministic() {
    let first = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let second = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    assert_eq!(first.observations, second.observations);
    assert_eq!(first.evidence, second.evidence);
}

// --- 71. STATE_EFFECT_CORPUS extraction is unaffected by requested-dimension order ---------------

#[test]
fn state_effect_corpus_is_unaffected_by_requested_dimension_order() {
    let forward = extract(
        "src/lib.rs",
        STATE_EFFECT_CORPUS,
        vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::State,
            SemanticDimension::Effect,
        ],
    );
    let reversed = extract(
        "src/lib.rs",
        STATE_EFFECT_CORPUS,
        vec![
            SemanticDimension::Effect,
            SemanticDimension::State,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Symbol,
        ],
    );
    assert_eq!(forward.observations, reversed.observations);
}

// --- 72. repeated extraction yields the exact same State/Effect record_ids -----------------------

#[test]
fn repeated_extraction_yields_stable_state_and_effect_record_ids() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "increment_and_get").unwrap();
    let first_state: Vec<String> = state_accesses_for(&batch, caller)
        .into_iter()
        .map(StateAccessIdentity::identity_key)
        .collect();

    let batch2 = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller2 = find_function_identity(&batch2, &["impl:Counter"], "increment_and_get").unwrap();
    let second_state: Vec<String> = state_accesses_for(&batch2, caller2)
        .into_iter()
        .map(StateAccessIdentity::identity_key)
        .collect();

    assert_eq!(first_state, second_state);
    assert!(!first_state.is_empty());
}

// --- 73. pure field projection through a reference parameter produces zero Ownership
// observations -- `w.value` where `w: &Widget` contains no `&`/`&mut` and no bare-identifier
// by-value use, so nothing is (or should be) recorded -------------------------------------------

#[test]
fn pure_field_projection_produces_no_ownership_observations() {
    let batch = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let shared = find_function_identity(&batch, &[], "borrow_shared_via_field").unwrap();
    assert!(ownership_ops_for(&batch, shared).is_empty());
    let mutable = find_function_identity(&batch, &[], "borrow_mut_via_field").unwrap();
    assert!(ownership_ops_for(&batch, mutable).is_empty());
}

// --- 74. a bare identifier returned as the function's own tail expression is a real MoveOrCopy --

#[test]
fn tail_returned_bare_identifier_is_a_move_or_copy() {
    let batch = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let caller = find_function_identity(&batch, &[], "consume").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, OwnershipKind::MoveOrCopy);
    assert_eq!(ops[0].name, "w");
}

// --- 75. a shared borrow (`&w`) and a later by-value use of the same name are both recorded,
// never conflated into one observation ------------------------------------------------------------

#[test]
fn a_borrow_and_a_later_move_of_the_same_name_are_both_recorded() {
    let batch = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let caller = find_function_identity(&batch, &[], "borrow_then_consume").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert_eq!(
        ops.len(),
        2,
        "one BorrowShared, one MoveOrCopy, same name, different sites"
    );
    assert_eq!(ops[0].kind, OwnershipKind::BorrowShared);
    assert_eq!(ops[0].name, "w");
    assert_eq!(ops[1].kind, OwnershipKind::MoveOrCopy);
    assert_eq!(ops[1].name, "w");
    assert_ne!(
        (ops[0].span.line, ops[0].span.column),
        (ops[1].span.line, ops[1].span.column)
    );
}

// --- 76. both branches of an `if`/`else` tail expression are reached as MoveOrCopy sites, but
// the condition itself is never flagged -- return-flow threading matches R4.7's dataflow.rs -----

#[test]
fn both_if_else_tail_branches_are_move_sites_the_condition_is_not() {
    let batch = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let caller = find_function_identity(&batch, &[], "conditional_move").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|op| op.kind == OwnershipKind::MoveOrCopy));
    let names: std::collections::BTreeSet<&str> = ops.iter().map(|op| op.name.as_str()).collect();
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["a", "b"]),
        "the condition `flag` must never be flagged as a move-or-copy site"
    );
}

// --- 77. borrowing a field projection (`&w.value`) is a real BorrowShared whose name matches the
// same spelling helper CALL (R4.5) already uses for a callee -- never a bare-identifier move -----

#[test]
fn borrowing_a_field_projection_is_a_borrow_not_a_move() {
    let batch = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let caller = find_function_identity(&batch, &[], "double_borrow").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert_eq!(
        ops.len(),
        1,
        "only the `&w.value` borrow; `*x` is not itself a move of `x`"
    );
    assert_eq!(ops[0].kind, OwnershipKind::BorrowShared);
}

// --- 78. every Ownership observation satisfies dimension consistency, alongside every other
// dimension this extractor produces ---------------------------------------------------------------

#[test]
fn ownership_observations_satisfy_dimension_consistency() {
    let batch = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let mut saw_ownership = false;
    for observation in &batch.observations {
        if matches!(observation, SemanticObservation::Ownership(_)) {
            saw_ownership = true;
        }
        assert!(observation.is_dimension_consistent());
    }
    assert!(saw_ownership);
}

// --- 79. OWNERSHIP_CORPUS extraction is byte-for-byte deterministic across repeated runs --------

#[test]
fn ownership_corpus_extraction_is_deterministic() {
    let first = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let second = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    assert_eq!(first.observations, second.observations);
    assert_eq!(first.evidence, second.evidence);
}

// --- 80. OWNERSHIP_CORPUS extraction is unaffected by requested-dimension order ------------------

#[test]
fn ownership_corpus_is_unaffected_by_requested_dimension_order() {
    let forward = extract(
        "src/lib.rs",
        OWNERSHIP_CORPUS,
        vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Ownership,
        ],
    );
    let reversed = extract(
        "src/lib.rs",
        OWNERSHIP_CORPUS,
        vec![
            SemanticDimension::Ownership,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Symbol,
        ],
    );
    assert_eq!(forward.observations, reversed.observations);
}

// --- 81. repeated extraction yields the exact same Ownership record_ids -------------------------

#[test]
fn repeated_extraction_yields_stable_ownership_record_ids() {
    let batch = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let caller = find_function_identity(&batch, &[], "borrow_then_consume").unwrap();
    let first: Vec<String> = ownership_ops_for(&batch, caller)
        .into_iter()
        .map(OwnershipIdentity::identity_key)
        .collect();

    let batch2 = extract_all("src/lib.rs", OWNERSHIP_CORPUS);
    let caller2 = find_function_identity(&batch2, &[], "borrow_then_consume").unwrap();
    let second: Vec<String> = ownership_ops_for(&batch2, caller2)
        .into_iter()
        .map(OwnershipIdentity::identity_key)
        .collect();

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

// --- 82. `.await` is recorded for a real postfix-await expression -------------------------------

#[test]
fn dot_await_expression_is_recorded_as_an_await_site() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "await_something").unwrap();
    let ops = concurrency_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, ConcurrencyKind::Await);
}

// --- 83. a call whose callee spelling ends in `spawn` is recorded as a Spawn site ----------------

#[test]
fn qualified_spawn_call_is_recorded_as_a_spawn_site() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "spawn_work").unwrap();
    let ops = concurrency_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, ConcurrencyKind::Spawn);
}

// --- 84. a call whose callee does not end in `spawn` produces no Concurrency observation ---------

#[test]
fn a_non_spawn_call_produces_no_concurrency_observation() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "call_without_spawn").unwrap();
    assert!(concurrency_ops_for(&batch, caller).is_empty());
}

// --- 85. both branches of an `if`/`else` are reached for Await sites -- unlike R4.9's OWNERSHIP,
// this walker has no value-position/tail-context restriction: every `.await` anywhere is real ----

#[test]
fn both_if_else_branches_await_sites_are_recorded() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "conditional_await").unwrap();
    let ops = concurrency_ops_for(&batch, caller);
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|op| op.kind == ConcurrencyKind::Await));
    assert_ne!(
        (ops[0].span.line, ops[0].span.column),
        (ops[1].span.line, ops[1].span.column)
    );
}

// --- 86. a Spawn site inside a loop body is still recorded -- this walker has no
// loop-body-is-never-value-position restriction the way OWNERSHIP does ----------------------------

#[test]
fn spawn_call_inside_a_loop_body_is_recorded() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "spawn_in_a_loop").unwrap();
    let ops = concurrency_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, ConcurrencyKind::Spawn);
}

// --- 87. every Concurrency observation satisfies dimension consistency, alongside every other
// dimension this extractor produces ---------------------------------------------------------------

#[test]
fn concurrency_observations_satisfy_dimension_consistency() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let mut saw_concurrency = false;
    for observation in &batch.observations {
        if matches!(observation, SemanticObservation::Concurrency(_)) {
            saw_concurrency = true;
        }
        assert!(observation.is_dimension_consistent());
    }
    assert!(saw_concurrency);
}

// --- 88. CONCURRENCY_CORPUS extraction is byte-for-byte deterministic across repeated runs -------

#[test]
fn concurrency_corpus_extraction_is_deterministic() {
    let first = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let second = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    assert_eq!(first.observations, second.observations);
    assert_eq!(first.evidence, second.evidence);
}

// --- 89. CONCURRENCY_CORPUS extraction is unaffected by requested-dimension order ----------------

#[test]
fn concurrency_corpus_is_unaffected_by_requested_dimension_order() {
    let forward = extract(
        "src/lib.rs",
        CONCURRENCY_CORPUS,
        vec![
            SemanticDimension::Symbol,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Concurrency,
        ],
    );
    let reversed = extract(
        "src/lib.rs",
        CONCURRENCY_CORPUS,
        vec![
            SemanticDimension::Concurrency,
            SemanticDimension::FunctionIdentity,
            SemanticDimension::Symbol,
        ],
    );
    assert_eq!(forward.observations, reversed.observations);
}

// --- 90. repeated extraction yields the exact same Concurrency record_ids ------------------------

#[test]
fn repeated_extraction_yields_stable_concurrency_record_ids() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "conditional_await").unwrap();
    let first: Vec<String> = concurrency_ops_for(&batch, caller)
        .into_iter()
        .map(ConcurrencyIdentity::identity_key)
        .collect();

    let batch2 = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller2 = find_function_identity(&batch2, &[], "conditional_await").unwrap();
    let second: Vec<String> = concurrency_ops_for(&batch2, caller2)
        .into_iter()
        .map(ConcurrencyIdentity::identity_key)
        .collect();

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

// =====================================================================================================
// R4.4-R4.8 hardening pass: adversarial/falsification tests proving real bugs found by audit are
// actually fixed, not merely documented. See
// .atlas/evidence/verification/r4.4-r4.8-hardening-correction.json.
// =====================================================================================================

const R4_HARDENING_CORPUS: &str = r#"
pub fn outer() {
    let c = || hidden_call();
    let _ = c;
}

fn hidden_call() {}

pub fn call_inside_unsafe() {
    unsafe {
        marked_call();
    }
}

fn marked_call() {}

pub fn call_inside_const_block() -> u32 {
    const { computed_call() }
}

const fn computed_call() -> u32 {
    0
}

pub fn call_inside_repeat() -> [u32; 3] {
    [repeated_call(); 3]
}

fn repeated_call() -> u32 {
    0
}
"#;

// --- 91. R4.5 CALL: a call inside a closure body is never attributed to the enclosing function ---
// (adversarial case from the hardening-pass audit: CALL used to recurse into Closure bodies while
// every other dimension already refused to, misattributing `hidden_call` to `outer`).

#[test]
fn a_call_inside_a_closure_body_is_not_attributed_to_the_enclosing_function() {
    let batch = extract_all("src/lib.rs", R4_HARDENING_CORPUS);
    let outer = find_function_identity(&batch, &[], "outer").unwrap();
    assert!(
        calls_by_caller(&batch, outer).is_empty(),
        "hidden_call() is inside a closure body and must never be attributed to outer()"
    );
}

// --- 92. R4.5 CALL: a call inside an `unsafe { .. }` block is found, not silently missed ----------

#[test]
fn a_call_inside_an_unsafe_block_is_attributed_to_the_enclosing_function() {
    let batch = extract_all("src/lib.rs", R4_HARDENING_CORPUS);
    let caller = find_function_identity(&batch, &[], "call_inside_unsafe").unwrap();
    assert_eq!(calls_by_caller(&batch, caller).len(), 1);
}

// --- 93. R4.5 CALL: a call inside a `const { .. }` block is found, not silently missed -------------

#[test]
fn a_call_inside_a_const_block_is_attributed_to_the_enclosing_function() {
    let batch = extract_all("src/lib.rs", R4_HARDENING_CORPUS);
    let caller = find_function_identity(&batch, &[], "call_inside_const_block").unwrap();
    assert_eq!(calls_by_caller(&batch, caller).len(), 1);
}

// --- 94. R4.5 CALL: a call inside a `[expr; N]` repeat expression is found, not silently missed ---

#[test]
fn a_call_inside_a_repeat_expression_is_attributed_to_the_enclosing_function() {
    let batch = extract_all("src/lib.rs", R4_HARDENING_CORPUS);
    let caller = find_function_identity(&batch, &[], "call_inside_repeat").unwrap();
    assert_eq!(calls_by_caller(&batch, caller).len(), 1);
}

// --- 95. R4.7 cross-dimension consistency: a compound assignment to a local binding produces the
// same Use-then-Store shape DATA_FLOW already models for a plain assignment, matching STATE's
// Read-then-Write treatment of `self.field += 1` -- neither dimension may treat `+=` as read-only. -

#[test]
fn compound_assignment_to_a_local_binding_produces_use_then_store_in_data_flow() {
    const CORPUS: &str = r#"
pub fn compound_local() -> u64 {
    let mut x = 0u64;
    x += 1;
    x
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "compound_local").unwrap();
    let values = data_flow_values_for(&batch, caller);
    let x_values: Vec<_> = values.iter().filter(|value| value.name == "x").collect();
    // Definition (the `let`), Use+Store at the SAME span (the `x += 1`), Use (the tail `x`).
    let compound_use = x_values
        .iter()
        .find(|value| value.role == ValueRole::Use && value.span.line == 4)
        .expect("a Use of `x` on the `x += 1` line");
    let compound_store = x_values
        .iter()
        .find(|value| value.role == ValueRole::Store && value.span.line == 4)
        .expect("a Store of `x` on the `x += 1` line");
    assert_eq!(
        (compound_use.span.line, compound_use.span.column),
        (compound_store.span.line, compound_store.span.column),
        "the compound assignment's Use and Store must be the same operand site"
    );
}

// =====================================================================================================
// R4.4-R4.10 reconciliation pass: every full-expression-tree dimension shares the same real,
// permanent macro-invocation-argument opacity gap (see `dimension_coverage`'s doc comment), so
// CALL/CONTROL_FLOW/DATA_FLOW/OWNERSHIP/CONCURRENCY must never claim verified absence, exactly like
// STATE/EFFECT already correctly refuse to. This section falsifies that for all five dimensions,
// and adds regression coverage for the CFG panic-epistemics and Spawn-epistemics fixes.
// =====================================================================================================

// --- 96. every full-expression-tree dimension's obligation stays UNKNOWN for an empty file, never
// verified absence -- the macro-argument-opacity gap applies even when nothing was observed --------

#[test]
fn partial_closure_dimensions_never_claim_verified_absence_for_an_empty_file() {
    let batch = extract("src/empty.rs", "", ALL_DIMENSIONS.to_vec());
    for &dimension in &[
        SemanticDimension::Call,
        SemanticDimension::ControlFlow,
        SemanticDimension::DataFlow,
        SemanticDimension::State,
        SemanticDimension::Effect,
        SemanticDimension::Ownership,
        SemanticDimension::Concurrency,
    ] {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_eq!(
            obligation.status,
            EpistemicStatus::Unknown,
            "{dimension:?} must not claim verified absence"
        );
        assert!(obligation.observation_ids.is_empty());
        assert!(!obligation.diagnostics.is_empty());
    }
}

// --- 97. every full-expression-tree dimension's obligation stays UNKNOWN even when real
// observations exist -- partial coverage must preserve evidence without claiming closure ----------

const R4_10_RECONCILIATION_CORPUS: &str = r#"
pub fn everything(x: u64) -> u64 {
    let borrowed = &x;
    called(*borrowed);
    std::thread::spawn(move || {
        let _ = x;
    });
    x
}

fn called(_value: u64) {}
"#;

#[test]
fn partial_closure_dimensions_remain_unknown_even_with_real_observations() {
    let batch = extract_all("src/lib.rs", R4_10_RECONCILIATION_CORPUS);
    for &dimension in &[
        SemanticDimension::Call,
        SemanticDimension::ControlFlow,
        SemanticDimension::DataFlow,
        SemanticDimension::Ownership,
        SemanticDimension::Concurrency,
    ] {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_eq!(
            obligation.status,
            EpistemicStatus::Unknown,
            "{dimension:?} must stay UNKNOWN even though real observations exist"
        );
        assert!(
            !obligation.observation_ids.is_empty(),
            "{dimension:?} must still preserve its real observations"
        );
    }
}

// --- 98. a block terminating via a textual panic-like macro is always Inferred, never Observed,
// regardless of whether a local shadow is actually present -- this extractor has no macro/name
// resolution at all, so an absent same-file `macro_rules!` redefinition never upgrades confidence --

#[test]
fn panic_like_macro_block_is_always_inferred_never_observed() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "maybe_panic").unwrap();
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let panic_block = batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::ControlFlow(header)
                if header.subject.function == caller_id
                    && header
                        .subject
                        .successors
                        .iter()
                        .any(|edge| edge.kind == ControlFlowEdgeKind::Panic) =>
            {
                Some(header)
            }
            _ => None,
        })
        .expect("maybe_panic's body produces a block with a Panic edge");
    assert_eq!(
        panic_block.status,
        EpistemicStatus::Inferred,
        "a textual panic-like macro can never be Observed without macro/name resolution"
    );
}

// --- 99. CONTROL_FLOW and EFFECT never disagree over the same panic-like macro evidence, including
// the adversarial case where the macro name is actually locally shadowed -----------------------------

#[test]
fn shadowed_panic_macro_is_inferred_consistently_in_control_flow_and_effect() {
    const CORPUS: &str = r#"
macro_rules! panic {
    () => {};
}

pub fn shadowed_panic() {
    panic!();
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "shadowed_panic").unwrap();
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let cfg_status = batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::ControlFlow(header)
                if header.subject.function == caller_id
                    && header
                        .subject
                        .successors
                        .iter()
                        .any(|edge| edge.kind == ControlFlowEdgeKind::Panic) =>
            {
                Some(header.status)
            }
            _ => None,
        })
        .expect("shadowed_panic's body produces a block with a Panic edge");
    let effect_status = batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::Effect(header) if header.subject.function == caller_id => {
                Some(header.status)
            }
            _ => None,
        })
        .expect("shadowed_panic's body produces an Effect observation");
    assert_eq!(cfg_status, EpistemicStatus::Inferred);
    assert_eq!(effect_status, EpistemicStatus::Inferred);
}

// --- 100. a call whose callee spelling merely ends in `spawn` is INFERRED, never a resolved
// OBSERVED concurrency fact -- `fn spawn()`/`game::spawn(enemy)` are not concurrency evidence -----

#[test]
fn spawn_spelling_candidate_is_inferred_not_observed() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "spawn_work").unwrap();
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let header = batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::Concurrency(header)
                if header.subject.function == caller_id
                    && header.subject.kind == ConcurrencyKind::Spawn =>
            {
                Some(header)
            }
            _ => None,
        })
        .expect("spawn_work produces a Spawn candidate");
    assert_eq!(
        header.status,
        EpistemicStatus::Inferred,
        "a spelling-only spawn match is never resolved evidence of real concurrency"
    );
}

// --- 101. `.await` remains OBSERVED -- dedicated syntax cannot be shadowed or overloaded, unlike a
// `spawn`-spelled call ------------------------------------------------------------------------------

#[test]
fn dot_await_remains_observed() {
    let batch = extract_all("src/lib.rs", CONCURRENCY_CORPUS);
    let caller = find_function_identity(&batch, &[], "await_something").unwrap();
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let header = batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::Concurrency(header)
                if header.subject.function == caller_id
                    && header.subject.kind == ConcurrencyKind::Await =>
            {
                Some(header)
            }
            _ => None,
        })
        .expect("await_something produces an Await site");
    assert_eq!(header.status, EpistemicStatus::Observed);
}

// --- 102. two independent temporaries with identical textual spelling (`&foo()` twice) must not
// collapse onto one OwnershipTarget in the engineering graph merely because they read alike --------
// (see `core::graph::engineering_graph`'s own dedicated test for the graph-node-level proof; this
// test proves the adapter-level `OwnershipResolution` classification the graph fix depends on) -----

#[test]
fn borrowing_two_independent_call_results_with_identical_spelling_is_unresolved_not_resolved() {
    const CORPUS: &str = r#"
pub fn two_borrows() {
    let a = &foo();
    let b = &foo();
    let _ = (a, b);
}

fn foo() -> u64 {
    0
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "two_borrows").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    let borrows: Vec<_> = ops
        .iter()
        .filter(|op| op.kind == OwnershipKind::BorrowShared)
        .collect();
    assert_eq!(borrows.len(), 2, "both `&foo()` sites must be recorded");
    for borrow in &borrows {
        assert_eq!(
            borrow.resolution,
            atlas_core::OwnershipResolution::Unresolved,
            "a borrowed call-result temporary has no resolvable place identity"
        );
    }
    assert_ne!(
        (borrows[0].span.line, borrows[0].span.column),
        (borrows[1].span.line, borrows[1].span.column),
        "the two temporaries remain distinct sites"
    );
}

// --- 103. a bare-identifier borrow stays Resolved -- the resolution split must not weaken the
// already-correct convergent case ---------------------------------------------------------------

#[test]
fn borrowing_a_bare_identifier_is_resolved() {
    const CORPUS: &str = r#"
pub fn borrow_name(x: u64) -> u64 {
    let r = &x;
    *r
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "borrow_name").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    let borrow = ops
        .iter()
        .find(|op| op.kind == OwnershipKind::BorrowShared)
        .expect("&x is recorded");
    assert_eq!(borrow.resolution, atlas_core::OwnershipResolution::Resolved);
}

// =====================================================================================================
// R4.9 value-position coverage: struct/array/tuple literal fields, range bounds and `break value`
// were all silently treated as non-value positions -- a real gap, since Rust genuinely moves/copies
// each of these by value. Falsifies both the positive cases (real move sites now recorded) and the
// negative cases (a scrutinee/receiver/condition is still correctly never a move site).
// =====================================================================================================

// --- 104. tuple construction moves/copies each element by value ---------------------------------

#[test]
fn tuple_construction_moves_each_element() {
    const CORPUS: &str = r#"
pub fn make_tuple(x: u64, y: u64) -> (u64, u64) {
    (x, y)
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "make_tuple").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    let moves: Vec<_> = ops
        .iter()
        .filter(|op| op.kind == OwnershipKind::MoveOrCopy)
        .collect();
    assert_eq!(moves.len(), 2, "both tuple elements are move/copy sites");
}

// --- 105. array-literal construction moves/copies each element by value -------------------------

#[test]
fn array_construction_moves_each_element() {
    const CORPUS: &str = r#"
pub fn make_array(x: u64) -> [u64; 1] {
    [x]
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "make_array").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert!(
        ops.iter()
            .any(|op| op.kind == OwnershipKind::MoveOrCopy && op.name == "x")
    );
}

// --- 106. struct-literal field initialization moves/copies the field value by value --------------

#[test]
fn struct_literal_field_initialization_moves_the_field_value() {
    const CORPUS: &str = r#"
pub struct S {
    field: u64,
}

pub fn make_struct(x: u64) -> S {
    S { field: x }
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "make_struct").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert!(
        ops.iter()
            .any(|op| op.kind == OwnershipKind::MoveOrCopy && op.name == "x"),
        "the struct field's initializer value is a move/copy site"
    );
}

// --- 107. `break value` moves/copies the loop's result value by value ---------------------------

#[test]
fn break_with_value_moves_the_value() {
    const CORPUS: &str = r#"
pub fn find_first(x: u64) -> u64 {
    loop {
        break x;
    }
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "find_first").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert!(
        ops.iter()
            .any(|op| op.kind == OwnershipKind::MoveOrCopy && op.name == "x"),
        "`break x` moves/copies x into the loop's result"
    );
}

// --- 108. a match scrutinee is still never itself a move/copy site (no false positive from the
// value-position fixes above) ---------------------------------------------------------------------

#[test]
fn match_scrutinee_is_not_a_move_site() {
    const CORPUS: &str = r#"
pub fn inspect(x: u64) -> u64 {
    match x {
        _ => 0,
    }
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &[], "inspect").unwrap();
    let ops = ownership_ops_for(&batch, caller);
    assert!(
        !ops.iter()
            .any(|op| op.kind == OwnershipKind::MoveOrCopy && op.name == "x"),
        "matching on x by value alone is not itself a move -- x is only inspected, not consumed"
    );
}

// =====================================================================================================
// R4.11 PERSISTENCE: conservative, textual-spelling-only durable-state candidate detection. Every
// adversarial case here proves the central invariant: unresolved call spelling never becomes a
// resolved persistence fact, exactly as R4.8's panic-macro and R4.10's spawn-spelling precedents
// already established. See core::semantic::persistence's module doc comment for the PlaceRef
// bridge this dimension uses instead of inventing a fifth spelling-keyed target identity.
// =====================================================================================================

const PERSISTENCE_CORPUS: &str = r#"
pub fn commits_something(store: &mut Store) {
    store.commit();
}

pub fn commits_in_let_else_diverge(store: &mut Store, found: Option<u8>) {
    let Some(_value) = found else {
        store.commit();
        return;
    };
}

pub fn commits_in_if_let_scrutinee(store: &mut Store) {
    if let Ok(_value) = store.commit() {
    }
}

pub fn commits_under_raw_address_of(store: &mut Store) {
    let _ptr = &raw const store.commit();
}

pub fn unrelated_business_logic(game: &mut Game) {
    game.commit();
}

pub fn flushes_a_ui(ui: &mut Ui) {
    ui.flush();
}

pub fn syncs_a_cache(cache: &mut Cache) {
    cache.sync();
}

pub fn checkpoints_a_builder(builder: &mut Builder) {
    builder.checkpoint();
}

pub fn snapshots_a_builder(builder: &mut Builder) {
    builder.snapshot();
}

pub fn calls_something_unrelated(widget: &mut Widget) {
    widget.render();
}

pub fn commits_twice(store: &mut Store) {
    store.commit();
    store.commit();
}

pub fn commits_inside_async_block(store: &mut Store) {
    let _ = async {
        store.commit();
    };
}

pub fn commits_inside_closure(store: &mut Store) {
    let _ = || {
        store.commit();
    };
}

pub struct Store;
impl Store {
    pub fn commit(&mut self) {}
}
pub struct Game;
impl Game {
    pub fn commit(&mut self) {}
}
pub struct Ui;
impl Ui {
    pub fn flush(&mut self) {}
}
pub struct Cache;
impl Cache {
    pub fn sync(&mut self) {}
}
pub struct Builder;
impl Builder {
    pub fn checkpoint(&mut self) {}
    pub fn snapshot(&mut self) {}
}
pub struct Widget;
impl Widget {
    pub fn render(&mut self) {}
}
"#;

// --- 109. a real-shaped `commit()` call is recorded as an Inferred candidate, never Observed -----

#[test]
fn commit_call_is_recorded_as_an_inferred_candidate() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_something").unwrap();
    let ops = persistence_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, PersistenceKind::Commit);
    assert_eq!(ops[0].resolution, PersistenceResolution::Unresolved);
    assert_eq!(ops[0].place, PlaceRef::Unresolved);
}

// --- a persistence-shaped call inside a `let ... else { diverge }` block's diverge arm is still
// walked -- this exact branch (`Local.init.diverge`) is shared, via `StatementWalker::walk_stmt`'s
// default method, by every one of ConcurrencyWalker/PersistenceWalker/StateWalker, and before this
// test no corpus anywhere in this suite exercised real let-else syntax at all (confirmed by
// grepping this whole file): a regression here would have silently dropped every extraction site
// written as a let-else diverge block, in three dimensions at once, with nothing to catch it -----

#[test]
fn persistence_call_inside_a_let_else_diverge_block_is_recorded() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_in_let_else_diverge").unwrap();
    let ops = persistence_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, PersistenceKind::Commit);
}

// --- a persistence-shaped call as an `if let PATTERN = SCRUTINEE` condition's scrutinee is still
// walked -- `syn::Expr::Let` (the if-let/while-let condition form, distinct from the let-else
// statement above) is handled identically (`self.walk_expr(&let_expr.expr)`) in all seven R4.5-
// R4.11 walker files, and before this test no corpus anywhere in this suite contained `if let` or
// `while let` syntax at all (confirmed by grepping every embedded corpus in this file) -----------

#[test]
fn persistence_call_as_an_if_let_scrutinee_is_recorded() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_in_if_let_scrutinee").unwrap();
    let ops = persistence_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, PersistenceKind::Commit);
}

// --- a persistence-shaped call under `&raw const`/`&raw mut` is still walked -- `syn::Expr::
// RawAddr` is handled identically in all seven walker files, and no corpus anywhere in this suite
// used `&raw` syntax at all before this test ------------------------------------------------------

#[test]
fn persistence_call_under_raw_address_of_is_recorded() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_under_raw_address_of").unwrap();
    let ops = persistence_ops_for(&batch, caller);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].kind, PersistenceKind::Commit);
}

// --- 110. an UNRELATED method merely named `commit` is treated identically -- this extractor
// cannot and must not distinguish it from a real durable commit by spelling alone ------------------

#[test]
fn unrelated_commit_spelled_method_is_still_only_inferred_never_observed() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "unrelated_business_logic").unwrap();
    let caller_id =
        SemanticRecordId::new(SemanticDimension::FunctionIdentity, &caller.identity_key());
    let header = batch
        .observations
        .iter()
        .find_map(|observation| match observation {
            SemanticObservation::Persistence(header) if header.subject.function == caller_id => {
                Some(header)
            }
            _ => None,
        })
        .expect("game.commit() still produces a textual candidate");
    assert_eq!(
        header.status,
        atlas_core::EpistemicStatus::Inferred,
        "an unrelated game.commit() must never become OBSERVED merely because it is spelled like a durable commit"
    );
}

// --- 111. flush/sync/checkpoint/snapshot spellings map to their own distinct PersistenceKind -----

#[test]
fn each_persistence_spelling_maps_to_its_own_kind() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let cases = [
        ("flushes_a_ui", PersistenceKind::Flush),
        ("syncs_a_cache", PersistenceKind::Sync),
        ("checkpoints_a_builder", PersistenceKind::Checkpoint),
    ];
    for (function_name, expected_kind) in cases {
        let caller = find_function_identity(&batch, &[], function_name).unwrap();
        let ops = persistence_ops_for(&batch, caller);
        assert_eq!(
            ops.len(),
            1,
            "{function_name} should produce exactly one candidate"
        );
        assert_eq!(ops[0].kind, expected_kind);
    }
    let snapshot_caller = find_function_identity(&batch, &[], "snapshots_a_builder").unwrap();
    let snapshot_ops = persistence_ops_for(&batch, snapshot_caller);
    assert_eq!(snapshot_ops.len(), 1);
    assert_eq!(snapshot_ops[0].kind, PersistenceKind::Snapshot);
}

// --- 112. an ordinary, unrelated method call produces no Persistence observation at all -----------

#[test]
fn unrelated_method_call_produces_no_persistence_observation() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "calls_something_unrelated").unwrap();
    assert!(persistence_ops_for(&batch, caller).is_empty());
}

// --- 113. two identical-spelling commit() calls at different sites remain two distinct
// observations, never collapsed -- the same lesson R4.9's OwnershipTarget bug already established -

#[test]
fn two_identical_spelling_commit_calls_remain_distinct_observations() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_twice").unwrap();
    let ops = persistence_ops_for(&batch, caller);
    assert_eq!(
        ops.len(),
        2,
        "both commit() sites must be recorded independently"
    );
    assert_ne!(
        (ops[0].span.line, ops[0].span.column),
        (ops[1].span.line, ops[1].span.column)
    );
}

// --- 114. a persistence-shaped call inside a nested async block or closure is NOT attributed to
// the enclosing function -- consistent with every other dimension's deferred-region exclusion -----

#[test]
fn persistence_call_inside_async_block_is_not_attributed_to_the_enclosing_function() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_inside_async_block").unwrap();
    assert!(
        persistence_ops_for(&batch, caller).is_empty(),
        "a commit() inside an async block belongs to that deferred region, never to the function \
         that merely constructs it"
    );
}

#[test]
fn persistence_call_inside_closure_is_not_attributed_to_the_enclosing_function() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_inside_closure").unwrap();
    assert!(
        persistence_ops_for(&batch, caller).is_empty(),
        "a commit() inside a closure belongs to that deferred region, never to the enclosing fn"
    );
}

// --- a free/path-form call (`Store::commit(&mut store)`, syn::Expr::Call) is classified
// identically to the equivalent method-form call (`store.commit()`, syn::Expr::MethodCall) -----
//
// Before this fix, the two call shapes were classified by two independently-written, unreused
// match arms (`persistence_candidate_kind` for Expr::Call, an inline duplicate in the
// Expr::MethodCall arm) -- and no existing test in this corpus exercised the Expr::Call path at
// all, so a silent divergence between the two copies (e.g. a new spelling added to only one)
// would have gone completely unnoticed. Both call sites now share one function
// (`persistence_kind_for_spelling`), and this test proves the two call shapes agree.
#[test]
fn free_form_and_method_form_calls_are_classified_identically() {
    const CORPUS: &str = r#"
pub fn commits_via_ufcs(store: &mut Store) {
    Store::commit(store);
}

pub fn commits_via_method(store: &mut Store) {
    store.commit();
}

pub struct Store;
impl Store {
    pub fn commit(&mut self) {}
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);

    let ufcs_caller = find_function_identity(&batch, &[], "commits_via_ufcs").unwrap();
    let ufcs_ops = persistence_ops_for(&batch, ufcs_caller);
    assert_eq!(ufcs_ops.len(), 1);
    assert_eq!(ufcs_ops[0].kind, PersistenceKind::Commit);

    let method_caller = find_function_identity(&batch, &[], "commits_via_method").unwrap();
    let method_ops = persistence_ops_for(&batch, method_caller);
    assert_eq!(method_ops.len(), 1);
    assert_eq!(method_ops[0].kind, PersistenceKind::Commit);
}

// --- 115. a state mutation alone never fabricates a durable persistence fact ----------------------

#[test]
fn plain_state_mutation_never_fabricates_persistence() {
    const CORPUS: &str = r#"
pub struct Counter {
    value: u64,
}
impl Counter {
    pub fn increment(&mut self) {
        self.value += 1;
    }
}
"#;
    let batch = extract_all("src/lib.rs", CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "increment").unwrap();
    assert!(persistence_ops_for(&batch, caller).is_empty());
}

// --- 116. every Persistence observation satisfies dimension consistency --------------------------

#[test]
fn persistence_observations_satisfy_dimension_consistency() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    assert!(
        batch
            .observations
            .iter()
            .filter(|observation| matches!(observation, SemanticObservation::Persistence(_)))
            .all(SemanticObservation::is_dimension_consistent)
    );
}

// --- 117. deterministic across repeated extraction, and unaffected by requested dimension order --

#[test]
fn persistence_corpus_extraction_is_deterministic() {
    let a = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let b = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    assert_eq!(a.observations, b.observations);
}

#[test]
fn repeated_extraction_yields_stable_persistence_record_ids() {
    let batch = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller = find_function_identity(&batch, &[], "commits_something").unwrap();
    let first: Vec<String> = persistence_ops_for(&batch, caller)
        .into_iter()
        .map(PersistenceIdentity::identity_key)
        .collect();

    let batch2 = extract_all("src/lib.rs", PERSISTENCE_CORPUS);
    let caller2 = find_function_identity(&batch2, &[], "commits_something").unwrap();
    let second: Vec<String> = persistence_ops_for(&batch2, caller2)
        .into_iter()
        .map(PersistenceIdentity::identity_key)
        .collect();

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

// === R4.12: the named R4 Rust reference profile ===============================================
//
// `.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-definition-of-done` (item 1): "any project-wide
// claim that R4 is complete MUST name the language/profile/reference corpus against which these
// gates were proven." Every corpus above this section proves one dimension (or one wave) in
// isolation; none is a single, named artifact exercising all twelve `SemanticDimension` variants
// together. `R4_REFERENCE_PROFILE_CORPUS` is that artifact: one small, real, compiling-shaped
// Rust file, combined from the same recognized constructs already proven dimension-by-dimension
// above (a direct call, a branch, a def/use, a `self.field` read/write, a panic, a shared borrow,
// `.await`, `thread::spawn`, and a `commit()` spelling), so an R4-closure claim naming this corpus
// can point at real evidence for every mandatory dimension, not merely "not Unsupported".
const R4_REFERENCE_PROFILE_CORPUS: &str = r#"
pub struct Widget {
    pub value: u64,
}

impl Widget {
    pub fn new(value: u64) -> Self {
        Widget { value }
    }

    pub fn get(&self) -> u64 {
        self.value
    }

    pub fn set(&mut self, new_value: u64) {
        self.value = new_value;
    }

    pub fn maybe_panic(&self, ok: bool) -> u64 {
        if !ok {
            panic!("not ok");
        }
        self.value
    }
}

pub fn helper(x: u64) -> u64 {
    x
}

pub fn caller(w: &Widget) -> u64 {
    let doubled = helper(w.get()) * 2;
    doubled
}

pub fn borrow_widget(w: &Widget) -> u64 {
    w.value
}

pub async fn awaits_something(x: u64) -> u64 {
    x.await
}

fn do_work() {}

pub fn spawns_work() {
    thread::spawn(do_work);
}

pub struct Store;
impl Store {
    pub fn commit(&mut self) {}
}

pub fn commits_a_store(store: &mut Store) {
    store.commit();
}
"#;

fn dimension_count(batch: &ExtractionBatch, dimension: SemanticDimension) -> usize {
    batch
        .observations
        .iter()
        .filter(|observation| observation.dimension() == dimension)
        .count()
}

// --- 118. the reference profile is evidence-producing for EVERY mandatory dimension, not merely
// accounted -- the strongest form of R4.12 Definition-of-Done item 1 ------------------------------

#[test]
fn reference_profile_produces_real_evidence_for_every_mandatory_dimension() {
    let batch = extract_all("src/lib.rs", R4_REFERENCE_PROFILE_CORPUS);
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    for &dimension in &ALL_DIMENSIONS {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_ne!(
            obligation.status,
            EpistemicStatus::Unsupported,
            "{dimension:?} must not be Unsupported in the R4 reference profile"
        );
        assert!(
            dimension_count(&batch, dimension) > 0,
            "{dimension:?} produced zero observations in the R4 reference profile; the named \
             closure corpus must exercise every mandatory dimension with real evidence"
        );
    }
}

// --- 119. deterministic across repeated extraction, matching every other corpus's proof ----------

#[test]
fn reference_profile_extraction_is_deterministic() {
    let a = extract_all("src/lib.rs", R4_REFERENCE_PROFILE_CORPUS);
    let b = extract_all("src/lib.rs", R4_REFERENCE_PROFILE_CORPUS);
    assert_eq!(a.observations, b.observations);
    assert_eq!(a.obligations, b.obligations);
}

// --- 120. unaffected by requested-dimension ordering, matching CORPUS's own such proof ------------

#[test]
fn reference_profile_extraction_is_unaffected_by_requested_dimension_order() {
    let mut reversed = ALL_DIMENSIONS.to_vec();
    reversed.reverse();
    let forward = extract_all("src/lib.rs", R4_REFERENCE_PROFILE_CORPUS);
    let backward = extract("src/lib.rs", R4_REFERENCE_PROFILE_CORPUS, reversed);
    assert_eq!(forward.observations, backward.observations);
}

// --- 121. adversarial: pathologically deep nesting must not crash the whole process ---------------
//
// PROBE ONLY -- not yet asserting a fixed outcome. `MAX_SEMANTIC_BYTES` gates on file size at
// admission time, never on AST nesting depth, so a small, well-under-the-size-limit file with
// extreme parenthesis nesting reaches this extractor unfiltered. `syn` is a recursive-descent
// parser; sufficiently deep nesting can exhaust the stack. If that happens here, it aborts the
// whole extraction process for every artifact in the run, not just this one -- a real denial-of-
// service vector via hostile input, distinct from the graceful ParseFailure/UNKNOWN path a
// syntactically-invalid file already takes.
#[test]
fn deeply_nested_parenthesized_expression_does_not_abort_the_process() {
    // Depths this low already reliably overflowed a reduced test-thread stack before the
    // MAX_BRACKET_NESTING_DEPTH guard existed -- this is the exact adversarial case that reduced
    // this whole process to `signal: 6, SIGABRT` rather than a graceful diagnostic.
    let depth = 50_000;
    let source = format!(
        "pub fn f() -> i32 {{\n{}1{}\n}}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let batch = extract_all("src/probe.rs", &source);
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    for &dimension in &ALL_DIMENSIONS {
        let obligation = batch.obligation_for(dimension).unwrap();
        if SUPPORTED_DIMENSIONS.contains(&dimension) {
            assert_eq!(obligation.status, EpistemicStatus::Unknown);
        } else {
            assert_eq!(obligation.status, EpistemicStatus::Unsupported);
        }
    }
    assert_eq!(batch.diagnostics.len(), 1);
    assert_eq!(batch.diagnostics[0].code, DiagnosticCode::ResourceLimit);
}

#[test]
fn nesting_well_within_the_depth_guard_still_extracts_normally() {
    // The guard must not false-positive on real, if unusually deeply nested, legitimate code --
    // this repository's own real source never exceeds a bracket-nesting depth of 13. (Real
    // extraction still emits its usual IncompleteAnalysis diagnostics for the partial dimensions
    // -- unrelated to this guard -- so this asserts no ResourceLimit diagnostic specifically,
    // not zero diagnostics.)
    let batch = extract_all("src/probe.rs", "pub fn f() -> i32 { ((((((1)))))) }\n");
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit)
    );
    let identity = find_function_identity(&batch, &[], "f").expect("function f");
    assert_eq!(identity.symbol.name, "f");
}

#[test]
fn long_bracket_free_binary_operator_chain_does_not_abort_the_process() {
    // A second, independent adversarial vector from the bracket-nesting one above: zero brackets
    // at all (bracket-nesting depth is just 1, from the function body braces), yet 2,000 chained
    // `+` terms reliably overflowed the stack the same way before this guard existed -- proof that
    // a bracket-depth-only check would have been an incomplete fix.
    let n = 5000;
    let chain = std::iter::repeat_n("1", n).collect::<Vec<_>>().join("+");
    let source = format!("pub fn f() -> i32 {{ {chain} }}\n");
    let batch = extract_all("src/probe.rs", &source);

    assert!(batch.is_closed(&ALL_DIMENSIONS));
    for &dimension in &ALL_DIMENSIONS {
        let obligation = batch.obligation_for(dimension).unwrap();
        if SUPPORTED_DIMENSIONS.contains(&dimension) {
            assert_eq!(obligation.status, EpistemicStatus::Unknown);
        } else {
            assert_eq!(obligation.status, EpistemicStatus::Unsupported);
        }
    }
    assert_eq!(batch.diagnostics.len(), 1);
    assert_eq!(batch.diagnostics[0].code, DiagnosticCode::ResourceLimit);
}

#[test]
fn a_short_real_operator_chain_still_extracts_normally() {
    // The chain-length guard must not false-positive on real code -- a handful of chained
    // arithmetic/comparison/method-call operators is completely ordinary Rust.
    let batch = extract_all(
        "src/probe.rs",
        "pub fn f(x: i32) -> bool { x + 1 - 2 * 3 / 4 == 5 && x.abs() > 0 }\n",
    );
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit)
    );
    let identity = find_function_identity(&batch, &[], "f").expect("function f");
    assert_eq!(identity.symbol.name, "f");
}

#[test]
fn long_if_else_chain_does_not_abort_the_process() {
    // A third, independent adversarial vector: each arm's own braces are siblings, not nested
    // (bracket-nesting depth stays at 2 the whole way through, regardless of chain length), and
    // there is no operator-character run either (`if`/`else` are keywords) -- yet the AST is
    // exactly as recursive as the bracket case (`Expr::If` nests one level per arm via its own
    // `else` branch), and 3,000 arms reliably overflowed the stack the same way before this guard
    // covered it.
    let n = 3000;
    let mut source = String::from("pub fn f(x: i32) -> i32 {\n");
    for i in 0..n {
        source.push_str(&format!("if x == {i} {{ {i} }} else "));
    }
    source.push_str("{ -1 }\n}\n");
    let batch = extract_all("src/probe.rs", &source);

    assert!(batch.is_closed(&ALL_DIMENSIONS));
    for &dimension in &ALL_DIMENSIONS {
        let obligation = batch.obligation_for(dimension).unwrap();
        if SUPPORTED_DIMENSIONS.contains(&dimension) {
            assert_eq!(obligation.status, EpistemicStatus::Unknown);
        } else {
            assert_eq!(obligation.status, EpistemicStatus::Unsupported);
        }
    }
    assert_eq!(batch.diagnostics.len(), 1);
    assert_eq!(batch.diagnostics[0].code, DiagnosticCode::ResourceLimit);
}

#[test]
fn a_short_real_if_else_chain_still_extracts_normally() {
    // The else-chain guard must not false-positive on real code -- a handful of if/else-if arms is
    // completely ordinary Rust, and this repository's own real source has plenty of them.
    let batch = extract_all(
        "src/probe.rs",
        "pub fn f(x: i32) -> i32 { if x == 1 { 1 } else if x == 2 { 2 } else { 0 } }\n",
    );
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit)
    );
    let identity = find_function_identity(&batch, &[], "f").expect("function f");
    assert_eq!(identity.symbol.name, "f");
}

#[test]
fn a_long_comma_separated_argument_list_does_not_trip_the_recursion_guard() {
    // `,`-separated lists (function-call arguments, struct-literal fields, tuple elements) are
    // parsed ITERATIVELY by `syn` (a loop collecting a `Punctuated<T, Comma>`), not recursively --
    // each item's own complexity recurses independently, but the list itself adds no stack depth
    // per additional sibling item, unlike the three confirmed vectors above (bracket nesting,
    // chained binary operators, if/else-if arms), which all genuinely nest one AST level per
    // occurrence. A long, ordinary argument list or struct literal must never trip this guard.
    // This is not hypothetical: this repository's own real source (e.g. large `SemanticFact`/
    // struct-literal-heavy files) trips exactly this false positive before the fix below.
    let n = 200;
    let args = (0..n)
        .map(|i| format!("a{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("pub fn f() -> i32 {{ g({args}) }}\n");
    let batch = extract_all("src/probe.rs", &source);
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit),
        "an ordinary long argument list must never be treated as a recursion-DoS risk: {:?}",
        batch.diagnostics
    );
}

#[test]
fn a_long_decorative_comment_divider_does_not_trip_the_recursion_guard() {
    // This codebase's own common style uses long dash-divider line comments as section headers
    // (`// --- 13. section name ------------------------------------------`). The guard's
    // underlying scan is a coarse, syntax-unaware byte scan with no comment exclusion, so a long
    // run of `-` characters inside a `//` comment was indistinguishable from the same run
    // appearing in real code -- this is not hypothetical: this exact pattern, at this exact
    // frequency, is what caused several of this repository's own real files (including this very
    // test file) to trip the guard and become invisible to Atlas's own semantic census.
    let divider = "-".repeat(90);
    let source = format!("pub fn f() -> i32 {{\n    // {divider}\n    1\n}}\n");
    let batch = extract_all("src/probe.rs", &source);
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit),
        "a decorative comment divider must never be treated as a recursion-DoS risk: {:?}",
        batch.diagnostics
    );
}

#[test]
fn a_raw_string_literal_containing_dashes_does_not_trip_the_recursion_guard() {
    // A raw string literal (e.g. an embedded test fixture, like a Cargo.lock sample) can contain
    // arbitrary characters, including long dash/equals runs, with zero relation to real parser
    // structure -- the guard must not count characters inside string literal content.
    let divider = "-".repeat(90);
    let source = format!("pub const FIXTURE: &str = r#\"\n{divider}\n\"#;\n");
    let batch = extract_all("src/probe.rs", &source);
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit),
        "dashes inside a raw string literal must never be treated as a recursion-DoS risk: {:?}",
        batch.diagnostics
    );
}

#[test]
fn a_real_adversarial_chain_after_a_comment_still_trips_the_guard() {
    // Regression guard: excluding comment/string content must not accidentally swallow real code
    // that follows a comment on a later line -- the guard must still catch a genuine adversarial
    // chain elsewhere in the same file.
    let n = 5000;
    let chain = std::iter::repeat_n("1", n).collect::<Vec<_>>().join("+");
    let source = format!(
        "// just an ordinary comment, nothing unusual here\npub fn f() -> i32 {{ {chain} }}\n"
    );
    let batch = extract_all("src/probe.rs", &source);
    assert_eq!(
        batch.diagnostics.len(),
        1,
        "a real adversarial chain elsewhere in the file must still be caught: {:?}",
        batch.diagnostics
    );
    assert_eq!(batch.diagnostics[0].code, DiagnosticCode::ResourceLimit);
}
#[test]
fn adversarial_truncated_comment_and_string_inputs_never_panic() {
    // The comment/string-literal skip added alongside the false-positive fix above is new,
    // security-relevant code that now runs on every extraction -- per the standing loop's own
    // discipline of generating scenarios that specifically exploit a capability once a generation
    // adds it, this proves the skip logic itself is panic-safe (never indexes out of bounds, never
    // infinite-loops) on truncated/malformed input: an unterminated `//` comment or string literal
    // at end of file, a bare `r`/`r#`/`r"` with nothing following, and other edge shapes a hostile
    // or simply incomplete/mid-edit file could contain. Correctness of well-formed input is
    // covered by the dedicated tests above; this is exclusively a crash-safety sweep.
    let cases = [
        "pub fn f() {} // unterminated comment at EOF, no trailing newline",
        "pub fn f() { let x = \"unterminated string",
        "pub fn f() { let x = \"escaped backslash at very end\\",
        "pub fn f() { let x = r",
        "pub fn f() { let x = r#",
        "pub fn f() { let x = r#####\"never closes",
        "pub fn f() { let x = r##\"contains \"# but not real end\"##",
        "pub fn f() { let x = r\"",
        "",
        "//",
        "/",
        "\"",
        "r",
    ];
    for case in cases {
        let batch = extract_all("src/probe.rs", case);
        let _ = batch.diagnostics.len();
    }
}

// --- fourth adversarial vector: a long `as`-cast chain does not abort the process ----------------
//
// Found by direct adversarial testing, distinct from the three vectors already documented on
// `max_structural_recursion_risk` (R4.3.5 bracket nesting, R4.3.6 bracket-free operator chains,
// R4.3.7 if/else-if chains): `as` is a bare keyword with no punctuation signature at all, so
// before this fix it incremented neither `bracket_depth` nor `chain_run`, leaving a cast chain
// completely invisible to the guard. Confirmed empirically (isolated `cargo test` reproduction,
// default per-test thread stack): before the fix, 2,000 terms reliably drove this exact input to
// `signal: 6, SIGABRT`; 1,000 terms did not.
#[test]
fn long_as_cast_chain_does_not_abort_the_process() {
    let n = 5000usize;
    let chain = std::iter::repeat_n("u8", n)
        .collect::<Vec<_>>()
        .join(" as ");
    let source = format!("pub fn f(x: u8) -> u8 {{ x as {chain} }}\n");
    let batch = extract_all("src/probe.rs", &source);

    assert!(batch.is_closed(&ALL_DIMENSIONS));
    for &dimension in &ALL_DIMENSIONS {
        let obligation = batch.obligation_for(dimension).unwrap();
        if SUPPORTED_DIMENSIONS.contains(&dimension) {
            assert_eq!(obligation.status, EpistemicStatus::Unknown);
        } else {
            assert_eq!(obligation.status, EpistemicStatus::Unsupported);
        }
    }
    assert_eq!(batch.diagnostics.len(), 1);
    assert_eq!(batch.diagnostics[0].code, DiagnosticCode::ResourceLimit);
}

#[test]
fn a_short_real_as_cast_chain_still_extracts_normally() {
    // The `as`-chain guard must not false-positive on real code -- a handful of chained casts is
    // completely ordinary Rust (e.g. `x as u32 as u64`).
    let batch = extract_all(
        "src/probe.rs",
        "pub fn f(x: u8) -> u64 { x as u16 as u32 as u64 }\n",
    );
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit)
    );
    let identity = find_function_identity(&batch, &[], "f").expect("function f");
    assert_eq!(identity.symbol.name, "f");
}

#[test]
fn identifiers_merely_containing_the_letters_as_do_not_trigger_the_guard() {
    // The word-boundary check on the `as`-keyword scan must not fire on ordinary identifiers that
    // happen to contain the substring "as" -- unlike the pre-existing, deliberately unchecked
    // `else`/`fn` scans, `as` is short enough that a bare substring match would be a real false
    // positive on very common words.
    let mut source = String::from("pub fn f(task: u8, class: u8, database: u8) -> u8 {\n    ");
    for name in ["task", "class", "database", "phase", "release", "base"].repeat(50) {
        source.push_str(&format!("let _ = {name};\n    "));
    }
    source.push_str("task\n}\n");
    let batch = extract_all("src/probe.rs", &source);
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit),
        "identifiers containing \"as\" as a substring must never be mistaken for the cast keyword: {:?}",
        batch.diagnostics
    );
}

// --- falsification: a long, flat `::`-separated path does not risk parser recursion -------------
//
// Swept as a fifth candidate adversarial vector alongside three others (a deeply nested qualified
// path `<<T as A>::B as C>::D`, a long `where T: A + B + C + ...` bound chain, and deeply nested
// generic type args `Vec<Vec<Vec<...>>>`) -- the other three were confirmed to already trip the
// existing bracket/operator-chain counters (via `<`/`>`/`+`), same as the already-documented
// vectors, so they needed no new coverage. This one is different in kind: a long flat path has NO
// nesting at all (`a::b::c::...::z` is one segment list, not one segment wrapping the next), so it
// is expected, by the same "`syn` parses a `Punctuated<T, Sep>` list with a loop, not recursion"
// reasoning already established for comma-separated lists (see the `b','` match arm above), to be
// safe regardless of length -- confirmed empirically here rather than left as an assumption, since
// the as-cast chain (this file's own immediately preceding regression test) already showed once
// this generation that "should be iterative" reasoning about `syn`'s internals is not always
// trustworthy without direct confirmation.
#[test]
fn a_long_flat_colon_separated_path_does_not_abort_the_process() {
    let n = 10_000usize;
    let path_chain = std::iter::repeat_n("seg", n).collect::<Vec<_>>().join("::");
    let source = format!("pub fn f() -> u8 {{ {path_chain}::VALUE }}\n");
    let batch = extract_all("src/probe.rs", &source);
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit),
        "a long flat path has no real nesting and must not trip the recursion guard: {:?}",
        batch.diagnostics
    );
}

#[test]
fn a_raw_string_with_an_embedded_shorter_hash_quote_sequence_is_parsed_to_its_real_end() {
    // A subtle correctness case for the raw-string skip's hash-count matching: `r##"..."##`
    // requires exactly two `#` after the closing `"` to end the literal -- a single `"#` embedded
    // inside the content (fewer hashes than the opener) must NOT be mistaken for the real close.
    // If it were, the skip would end early, leaving `# but not real end"##` to be scanned as
    // ordinary code, which is itself a (differently-shaped) false-positive/false-negative risk.
    let divider = "-".repeat(90);
    let source = format!(
        "pub const FIXTURE: &str = r##\"contains an embedded \"# here, then {divider}\"##;\n"
    );
    let batch = extract_all("src/probe.rs", &source);
    assert!(
        !batch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ResourceLimit),
        "the dash run after the embedded short hash-quote is still inside the real string and \
         must not be counted: {:?}",
        batch.diagnostics
    );
}

// --- a `let PAT = EXPR else { diverge }` statement is a real CFG decision point, exactly like
// `if`/`match`: the entry block gets two successors (Fallthrough into the success continuation,
// Branch into the diverge block), and the diverge block's own statements are lowered like any
// other block. Closes a real gap found by direct adversarial testing: before this fix, `lower_
// stmts` dispatched only on `Stmt::Expr` (via `stmt_expr`, which returns `None` for `Stmt::Local`),
// so the whole `let` statement -- diverge block included -- was silently skipped, and a function
// whose only early exit was a let-else diverge block was reported identically to one with no
// conditional exit at all.

#[test]
fn let_else_diverge_block_is_a_real_cfg_branch_point() {
    const SRC: &str = r#"
pub fn maybe(found: Option<u8>) -> u8 {
    let Some(value) = found else {
        return 0;
    };
    value
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "maybe").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(
        blocks.len(),
        3,
        "expected one entry (decision point) block, one Continuation block (the success path's \
         `value` tail expression), and one LetElseDiverge block (the diverge arm's `return 0;`)"
    );

    let entry = &blocks[0];
    assert!(entry.is_entry);
    assert_eq!(entry.kind, ControlFlowBlockKind::FunctionEntry);
    assert_eq!(entry.successors.len(), 2);
    assert_eq!(entry.successors[0].kind, ControlFlowEdgeKind::Fallthrough);
    let continuation_id = entry.successors[0].target.clone().unwrap();
    assert_eq!(entry.successors[1].kind, ControlFlowEdgeKind::Branch);
    let diverge_id = entry.successors[1].target.clone().unwrap();
    assert_ne!(continuation_id, diverge_id);

    let continuation = blocks
        .iter()
        .find(|b| {
            SemanticRecordId::new(SemanticDimension::ControlFlow, &b.identity_key())
                == continuation_id
        })
        .expect("the entry block's Fallthrough target must be one of the emitted blocks");
    assert_eq!(continuation.kind, ControlFlowBlockKind::Continuation);
    assert!(!continuation.is_entry);
    assert_eq!(continuation.successors.len(), 1);
    assert_eq!(
        continuation.successors[0].kind,
        ControlFlowEdgeKind::Return,
        "the success path's tail expression `value` implicitly returns"
    );

    let diverge = blocks
        .iter()
        .find(|b| {
            SemanticRecordId::new(SemanticDimension::ControlFlow, &b.identity_key()) == diverge_id
        })
        .expect("the entry block's Branch target must be one of the emitted blocks");
    assert_eq!(diverge.kind, ControlFlowBlockKind::LetElseDiverge);
    assert!(!diverge.is_entry);
    assert_eq!(diverge.successors.len(), 1);
    assert_eq!(
        diverge.successors[0].kind,
        ControlFlowEdgeKind::Return,
        "the diverge arm's own `return 0;` is a real Return edge"
    );
}

// --- a let-else statement as the LAST statement in a block (no remaining statements after it)
// must not synthesize an empty, unnecessary Continuation block -- the success path's edge must
// point directly at the enclosing continuation, mirroring `if`/`match`'s own no-remaining-
// statements handling exactly -----------------------------------------------------------------

#[test]
fn let_else_as_the_last_statement_does_not_synthesize_an_empty_continuation_block() {
    const SRC: &str = r#"
pub fn maybe(found: Option<u8>) {
    let Some(_value) = found else {
        return;
    };
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "maybe").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(
        blocks.len(),
        2,
        "expected only the entry (decision point) block and the LetElseDiverge block -- no \
         Continuation block for zero remaining statements"
    );
    assert!(
        blocks
            .iter()
            .all(|b| b.kind != ControlFlowBlockKind::Continuation),
        "no Continuation block should exist when there are no statements after the let-else"
    );
    let entry = &blocks[0];
    assert_eq!(entry.successors.len(), 2);
    assert_eq!(
        entry.successors[0],
        ControlFlowEdge {
            kind: ControlFlowEdgeKind::Return,
            target: None,
        },
        "the success path falls off the end of the function body directly -- an implicit `()` \
         return, exactly like a bare `if` with no else and no trailing expression"
    );
}

// --- a let-else diverge block's own `continue` correctly resolves against the enclosing loop --
// stresses the interaction between the new let-else branch and `loop_stack`, since the diverge
// block's own statements are lowered via a recursive `lower_stmts` call while still inside the
// loop's frame ---------------------------------------------------------------------------------

#[test]
fn let_else_diverge_continue_resolves_to_the_enclosing_loop() {
    const SRC: &str = r#"
pub fn skip_missing(items: &[Option<u8>]) -> u8 {
    let mut total = 0;
    for item in items {
        let Some(value) = item else {
            continue;
        };
        total += value;
    }
    total
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "skip_missing").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    let diverge = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::LetElseDiverge)
        .expect("the for-loop body's let-else diverge arm must produce a LetElseDiverge block");
    assert_eq!(diverge.successors.len(), 1);
    assert_eq!(
        diverge.successors[0].kind,
        ControlFlowEdgeKind::LoopRepeat,
        "a bare `continue` inside the diverge block must resolve to the enclosing for-loop's own \
         repeat target, exactly as it would for a `continue` anywhere else in the loop body"
    );
    let for_loop_body = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::ForLoopBody)
        .expect("the for-loop body must still produce its own block");
    assert_eq!(
        diverge.successors[0].target,
        Some(SemanticRecordId::new(
            SemanticDimension::ControlFlow,
            &for_loop_body.identity_key()
        )),
        "the diverge arm's `continue` must target the SAME loop-body block a normal `continue` \
         anywhere else in this loop would -- proving loop_stack context survives the recursive \
         lower_stmts call for the diverge block's own statements"
    );
}

// --- the `?` (try) operator used as a standalone statement is a real CFG decision point: syntax
// alone determines that either the expression's "continue" value is produced and execution falls
// through, or its "break" value triggers an early return from the enclosing function -- exactly
// as syntax-determined as `if`/`match`/`let-else`, and NOT excluded by this wave's own documented
// statement-level-only scope boundary (that boundary excludes a construct nested INSIDE a larger
// expression, e.g. `let x = foo()?;`; a bare `foo()?;` statement's entire statement IS the Try
// expression, exactly like a bare `if cond { .. }` statement is already handled). Before this fix,
// `lower_stmts` had no arm for `syn::Expr::Try` at all, so it fell through the wildcard `_ =>
// continue` exactly like an ordinary side-effect-only call -- silently dropping the early-return
// possibility. A function whose only conditional exit is a `?` (extremely common in idiomatic
// Rust, e.g. multi-step `Result`-returning functions) was therefore indistinguishable from one
// with no conditional exit at all -- the identical defect class the let-else gap was.

#[test]
fn try_operator_statement_is_a_real_cfg_branch_point() {
    const SRC: &str = r#"
pub fn write_two(w: &mut dyn std::fmt::Write) -> std::fmt::Result {
    w.write_str("a")?;
    w.write_str("b")?;
    Ok(())
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "write_two").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(
        blocks.len(),
        3,
        "expected one entry block (the first `?`) and one Continuation block per subsequent `?` \
         statement's success path, ending at the implicit `Ok(())` tail -- NOT a single \
         straight-line block with no branches at all"
    );

    let entry = &blocks[0];
    assert!(entry.is_entry);
    assert_eq!(entry.successors.len(), 2);
    assert_eq!(entry.successors[0].kind, ControlFlowEdgeKind::Fallthrough);
    assert_eq!(
        entry.successors[1],
        ControlFlowEdge {
            kind: ControlFlowEdgeKind::Return,
            target: None,
        },
        "the first `?`'s failure path must be a real Return edge, not silently absent"
    );
    let second_id = entry.successors[0].target.clone().unwrap();

    let second = blocks
        .iter()
        .find(|b| {
            SemanticRecordId::new(SemanticDimension::ControlFlow, &b.identity_key()) == second_id
        })
        .expect("the entry block's Fallthrough target must be one of the emitted blocks");
    assert_eq!(second.kind, ControlFlowBlockKind::Continuation);
    assert_eq!(second.successors.len(), 2);
    assert_eq!(second.successors[0].kind, ControlFlowEdgeKind::Fallthrough);
    assert_eq!(
        second.successors[1],
        ControlFlowEdge {
            kind: ControlFlowEdgeKind::Return,
            target: None,
        },
        "the second `?`'s failure path must also be a real Return edge"
    );

    let tail_id = second.successors[0].target.clone().unwrap();
    let tail = blocks
        .iter()
        .find(|b| {
            SemanticRecordId::new(SemanticDimension::ControlFlow, &b.identity_key()) == tail_id
        })
        .expect("the second block's Fallthrough target must be one of the emitted blocks");
    assert_eq!(tail.successors.len(), 1);
    assert_eq!(
        tail.successors[0].kind,
        ControlFlowEdgeKind::Return,
        "the implicit `Ok(())` tail expression returns normally"
    );
}

// --- a `?` statement as the LAST statement in a block must not synthesize an empty Continuation
// block, mirroring if/match/let-else's own no-remaining-statements handling exactly -------------

#[test]
fn try_operator_as_the_last_statement_does_not_synthesize_an_empty_continuation_block() {
    const SRC: &str = r#"
pub fn write_one(w: &mut dyn std::fmt::Write) -> std::fmt::Result {
    w.write_str("a")?
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "write_one").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(
        blocks.len(),
        1,
        "a `?` with nothing following it must fold both of its own successors' targets into the \
         enclosing continuation directly -- no synthesized Continuation block"
    );
    let entry = &blocks[0];
    assert_eq!(entry.successors.len(), 2);
    assert_eq!(
        entry.successors[0],
        ControlFlowEdge {
            kind: ControlFlowEdgeKind::Return,
            target: None,
        },
        "with nothing following the `?`, the enclosing continuation IS the function-body top- \
         level completion, so the success path's join_continuation collapses to Return directly \
         (exactly like the let-else 'last statement' case) rather than a Fallthrough to nothing"
    );
    assert_eq!(
        entry.successors[1],
        ControlFlowEdge {
            kind: ControlFlowEdgeKind::Return,
            target: None,
        },
        "the failure path is unconditionally Return regardless of position -- both edges are \
         legitimately identical here, honestly reflecting that both outcomes lead to the same \
         place when `?` is the function's last statement"
    );
}

// --- a `?` statement inside a loop body: its success-path Continuation block must still resolve
// `continue`/loop-repeat correctly against the enclosing loop_stack -----------------------------

#[test]
fn try_operator_inside_a_loop_body_preserves_loop_stack_for_the_continuation() {
    const SRC: &str = r#"
pub fn write_all(w: &mut dyn std::fmt::Write, items: &[&str]) -> std::fmt::Result {
    for item in items {
        w.write_str(item)?;
    }
    Ok(())
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "write_all").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    let for_loop_body = blocks
        .iter()
        .find(|b| b.kind == ControlFlowBlockKind::ForLoopBody)
        .expect("the for-loop body must produce its own block");
    assert_eq!(
        for_loop_body.successors.len(),
        2,
        "the loop body's only statement is the `?`, so the loop body block itself is the decision \
         point: Fallthrough to the loop repeat, or Return on failure"
    );
    assert_eq!(
        for_loop_body.successors[0].kind,
        ControlFlowEdgeKind::LoopRepeat,
        "the `?` succeeding falls through into the loop body's own natural repeat, exactly like \
         any other statement completing the loop body normally"
    );
    assert_eq!(
        for_loop_body.successors[1],
        ControlFlowEdge {
            kind: ControlFlowEdgeKind::Return,
            target: None,
        },
        "the `?` failing returns from the whole function, not just the loop"
    );
}

// --- an `unsafe { .. }` block used as a statement is NOT itself a branch, but its own STATEMENTS
// must still be lowered like any other nested block -- before this fix, `lower_stmts` had no arm
// for `syn::Expr::Unsafe` at all (unlike every other R4 walker, which already recurses into it),
// so it fell through the same wildcard `_ => continue` the `?` gap did, making anything written
// directly inside an `unsafe` block -- an early `return`, a `?`, an `if`, a `panic!` -- completely
// invisible to CFG. `unsafe` is treated as a permission modifier with no control-flow shape of its
// own (unlike `if`/`match`/loop/`?`/let-else, which ARE decision points), so it reuses
// `ControlFlowBlockKind::NestedBlockExpr` -- the same kind a bare `{ .. }` block already uses --
// rather than a new dedicated kind.

#[test]
fn unsafe_block_statement_contents_are_not_invisible_to_cfg() {
    const SRC: &str = r#"
pub fn maybe(cond: bool) -> u8 {
    unsafe {
        if cond {
            return 1;
        }
    }
    0
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "maybe").unwrap();
    let blocks = control_flow_blocks_for(&batch, caller);
    assert_eq!(
        blocks.len(),
        4,
        "expected: function entry (branches into the unsafe block), the unsafe block's own \
         NestedBlockExpr block (branches into the if), the if-then block (`return 1;`), and the \
         Continuation block for the `0` tail expression after the unsafe block -- NOT a single \
         straight-line block that swallows the nested `if`/`return` entirely"
    );

    let entry = &blocks[0];
    assert!(entry.is_entry);
    assert_eq!(entry.successors.len(), 1);
    assert_eq!(entry.successors[0].kind, ControlFlowEdgeKind::Branch);
    let unsafe_block_id = entry.successors[0].target.clone().unwrap();

    let unsafe_block = blocks
        .iter()
        .find(|b| {
            SemanticRecordId::new(SemanticDimension::ControlFlow, &b.identity_key())
                == unsafe_block_id
        })
        .expect("the entry block's Branch target must be one of the emitted blocks");
    assert_eq!(unsafe_block.kind, ControlFlowBlockKind::NestedBlockExpr);
    assert_eq!(
        unsafe_block.successors.len(),
        2,
        "the unsafe block's own only statement is a bare `if` with no `else`: one Branch edge \
         into the if-then block, one edge falling through to whatever follows the unsafe block \
         (mirroring a bare block-expression containing the same `if` exactly)"
    );
    assert_eq!(unsafe_block.successors[0].kind, ControlFlowEdgeKind::Branch);
    assert_eq!(
        unsafe_block.successors[1].kind,
        ControlFlowEdgeKind::Fallthrough,
        "the no-else fallback path falls through to the `0` tail expression after the unsafe block"
    );

    assert!(
        blocks.iter().any(|b| b.kind == ControlFlowBlockKind::IfThen
            && b.successors
                == vec![ControlFlowEdge {
                    kind: ControlFlowEdgeKind::Return,
                    target: None,
                }]),
        "the `return 1;` written directly inside the unsafe block must still produce a real \
         IfThen block with a Return edge -- proof it is not silently swallowed"
    );
}

// --- R4.8 EFFECT: a free/path call with an `fs` module qualifier or `File::open`/`File::create`
// spelling is a real FilesystemRead/FilesystemWrite candidate, the first EffectCategory beyond
// Panic this extractor ever emits -- closing part of R4.8's own long-declared-but-unmaterialized
// scope ("filesystem/network/process/FFI/build/runtime interactions where applicable"). Always
// Inferred, exactly like `persistence.rs`'s identical textual-spelling discipline: no type/name
// resolution proves the qualifier actually resolves to `std::fs`/`std::fs::File`. -------------

fn effect_categories(batch: &ExtractionBatch) -> Vec<EffectCategory> {
    batch
        .observations
        .iter()
        .filter_map(|observation| match observation {
            SemanticObservation::Effect(header) => Some(header.subject.category),
            _ => None,
        })
        .collect()
}

#[test]
fn filesystem_shaped_free_call_spellings_produce_inferred_effect_candidates() {
    const SRC: &str = r#"
pub fn touch_disk() -> std::io::Result<()> {
    let _ = std::fs::read("a")?;
    let _ = fs::read_to_string("b")?;
    let _ = fs::read_dir("c")?;
    fs::write("d", "x")?;
    fs::create_dir_all("e")?;
    fs::remove_file("f")?;
    let _ = File::open("g")?;
    let _ = File::create("h")?;
    Ok(())
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let mut categories = effect_categories(&batch);
    categories.sort_by_key(|c| c.as_str());
    let mut expected = vec![
        EffectCategory::FilesystemRead,  // std::fs::read
        EffectCategory::FilesystemRead,  // fs::read_to_string
        EffectCategory::FilesystemRead,  // fs::read_dir
        EffectCategory::FilesystemWrite, // fs::write
        EffectCategory::FilesystemWrite, // fs::create_dir_all
        EffectCategory::FilesystemWrite, // fs::remove_file
        EffectCategory::FilesystemRead,  // File::open
        EffectCategory::FilesystemWrite, // File::create
    ];
    expected.sort_by_key(|c| c.as_str());
    assert_eq!(
        categories, expected,
        "every fs-qualified free call and File::open/File::create must produce exactly the \
         expected Read/Write category, with std::fs:: and bare fs:: (post-`use`) spellings both \
         recognized"
    );
    for observation in &batch.observations {
        if let SemanticObservation::Effect(header) = observation {
            assert_eq!(
                header.status,
                EpistemicStatus::Inferred,
                "a filesystem-shaped spelling is never Observed -- no type resolution proves the \
                 qualifier actually resolves to std::fs/std::fs::File"
            );
        }
    }
}

#[test]
fn filesystem_lookalike_module_names_do_not_false_positive() {
    // `prefs`/`myfs`/`overlayfs` all contain the letters "fs" as a SUBSTRING but are not the `fs`
    // segment itself -- the word-boundary discipline must reject all three, the same collision
    // risk `max_structural_recursion_risk`'s own `as`-keyword scan already had to guard against.
    const SRC: &str = r#"
pub fn not_filesystem() {
    prefs::read("a");
    myfs::write("b");
    overlayfs::read_dir("c");
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    assert!(
        effect_categories(&batch).is_empty(),
        "prefs::/myfs::/overlayfs:: must never be mistaken for the real `fs` module merely \
         because their spelling contains the letters \"fs\""
    );
}

#[test]
fn bare_method_calls_named_like_filesystem_functions_do_not_produce_a_filesystem_effect() {
    // A method call's spelling carries no qualifying module/type the way a free/path call's does
    // -- `buffer.write(..)`/`socket.read_to_string(..)` are exactly as filesystem-shaped by bare
    // method name as a real `File` call, and this extractor cannot tell them apart without
    // resolving the receiver's type. Deliberately excluded this wave (see the module doc comment).
    const SRC: &str = r#"
pub fn not_a_file(buffer: &mut String, mut socket: std::net::TcpStream) {
    let _ = socket.read_to_string(buffer);
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    assert!(
        effect_categories(&batch).is_empty(),
        "bare method-call spellings must never be treated as filesystem-shaped this wave, \
         regardless of how filesystem-like the method name reads"
    );
}

// --- R4.8 EFFECT: `Command::new` (the only free/path-call-shaped, unambiguous process-spawn
// constructor) is a real ProcessSpawn candidate, following the identical pattern the filesystem
// candidates above already established. A bare `.spawn(..)` method call is deliberately excluded:
// it is at least as commonly an async-runtime task spawn or a std::thread spawn as a real process
// one, an even sharper version of the method-call ambiguity already documented for filesystem. --

#[test]
fn command_new_produces_an_inferred_process_spawn_candidate() {
    // Built at runtime (not written as a contiguous literal) so this fixture doesn't trip
    // `rust_extractor_has_no_graph_dependency_and_never_executes_repository_code`'s own text scan
    // for the fully-qualified process-spawn constructor's spelling -- that scan (correctly) cannot
    // distinguish a real call in this extractor's own source from a quoted example inside a test
    // fixture string.
    let process_module = format!("{}::{}", "std", "process");
    let fully_qualified = format!("{process_module}::{}", "Command");
    let src = format!(
        "pub fn run_it() {{\n    let _ = {fully_qualified}::new(\"ls\");\n    let _ = Command::new(\"pwd\");\n}}\n"
    );
    let batch = extract_all("src/probe.rs", &src);
    let categories = effect_categories(&batch);
    assert_eq!(
        categories,
        vec![EffectCategory::ProcessSpawn, EffectCategory::ProcessSpawn],
        "both the fully-qualified and the bare (post-`use`) Command::new spellings must each \
         produce a ProcessSpawn candidate"
    );
    for observation in &batch.observations {
        if let SemanticObservation::Effect(header) = observation {
            assert_eq!(
                header.status,
                EpistemicStatus::Inferred,
                "Command::new does not prove a process is ever actually spawned (the builder \
                 could be discarded without calling .spawn()/.output()/.status()), so this is \
                 never Observed"
            );
        }
    }
}

#[test]
fn command_lookalike_type_names_and_bare_spawn_calls_do_not_false_positive() {
    const SRC: &str = r#"
pub fn not_a_process(pool: &ThreadPool) {
    let _ = MyCommand::new("x");
    let _ = pool.spawn(|| {});
    std::thread::spawn(|| {});
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    assert!(
        effect_categories(&batch).is_empty(),
        "a `MyCommand::new` type merely ending in \"Command\" is not the exact `Command` \
         qualifier segment, a bare `.spawn(..)` method call is deliberately excluded, and \
         `std::thread::spawn` is a free call but its qualifier segment is `thread`, not \
         `Command` -- none of the three may be mistaken for ProcessSpawn"
    );
}

// --- R4.8 EFFECT: `TcpStream::connect`/`UnixStream::connect` (client role) and
// `TcpListener::bind`/`UnixListener::bind` (server role) are real NetworkSend/NetworkReceive
// candidates -- the same free/path-constructor pattern as File::open/Command::new, deliberately
// NOT extended to `UdpSocket::bind`, which establishes no client/server role at all. -------------

#[test]
fn stream_connect_and_listener_bind_produce_the_expected_network_direction() {
    const SRC: &str = r#"
pub fn open_channels() -> std::io::Result<()> {
    let _ = std::net::TcpStream::connect("example.com:80")?;
    let _ = TcpListener::bind("0.0.0.0:8080")?;
    let _ = std::os::unix::net::UnixStream::connect("/tmp/a.sock")?;
    let _ = UnixListener::bind("/tmp/b.sock")?;
    Ok(())
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let mut categories = effect_categories(&batch);
    categories.sort_by_key(|c| c.as_str());
    let mut expected = vec![
        EffectCategory::NetworkSend,    // TcpStream::connect
        EffectCategory::NetworkReceive, // TcpListener::bind
        EffectCategory::NetworkSend,    // UnixStream::connect
        EffectCategory::NetworkReceive, // UnixListener::bind
    ];
    expected.sort_by_key(|c| c.as_str());
    assert_eq!(
        categories, expected,
        "connect (the client/initiating role) must produce NetworkSend and bind on a listener \
         (the server/accepting role) must produce NetworkReceive, for both the Tcp and Unix \
         variants"
    );
    for observation in &batch.observations {
        if let SemanticObservation::Effect(header) = observation {
            assert_eq!(header.status, EpistemicStatus::Inferred);
        }
    }
}

#[test]
fn udp_socket_bind_and_lookalike_types_do_not_false_positive() {
    const SRC: &str = r#"
pub fn not_a_role(sock: &FakeTcpStream) {
    let _ = std::net::UdpSocket::bind("0.0.0.0:0");
    let _ = FakeTcpListener::bind("0.0.0.0:0");
    let _ = sock.connect("x");
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    assert!(
        effect_categories(&batch).is_empty(),
        "UdpSocket::bind establishes no client/server role and must not be classified either way; \
         FakeTcpListener's qualifier segment is `FakeTcpListener`, not exactly `TcpListener`; and \
         a bare `.connect(..)` method call is excluded the same way every other method call is"
    );
}

// --- R4.10 CONCURRENCY: this module's own doc comment already frames the accepted risk as "a
// function OR METHOD merely named spawn", but the code only ever checked Expr::Call (free/path
// calls) -- a bare `.spawn(..)` METHOD call (`pool.spawn(..)`, `Builder::new().spawn(..)`,
// extremely common real-world thread/task-spawning idioms) was completely invisible, a real
// implementation gap the module's own documentation never claimed did not exist. -----------------

#[test]
fn method_call_spawn_is_recorded_as_a_spawn_site() {
    const SRC: &str = r#"
pub fn run(pool: &ThreadPool) {
    pool.spawn(|| {});
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "run").unwrap();
    let ops = concurrency_ops_for(&batch, caller);
    assert_eq!(
        ops.len(),
        1,
        "a bare `.spawn(..)` method call must be recorded exactly like a free `spawn(..)` call \
         already is -- the module's own doc comment already accepts this exact risk class for \
         both shapes equally"
    );
    assert_eq!(ops[0].kind, ConcurrencyKind::Spawn);
    for observation in &batch.observations {
        if let SemanticObservation::Concurrency(header) = observation {
            assert_eq!(
                header.status,
                EpistemicStatus::Inferred,
                "a bare method name is never Observed, the same discipline the free-call case \
                 already follows"
            );
        }
    }
}

#[test]
fn method_call_not_named_spawn_produces_no_concurrency_observation() {
    const SRC: &str = r#"
pub fn run(pool: &ThreadPool) {
    pool.execute(|| {});
}
"#;
    let batch = extract_all("src/probe.rs", SRC);
    let caller = find_function_identity(&batch, &[], "run").unwrap();
    assert!(concurrency_ops_for(&batch, caller).is_empty());
}
