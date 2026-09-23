//! R4.3 correctness corpus and invariant tests for `RustSemanticExtractor`.
//!
//! A small, hand-curated set of real Rust fixtures (not a benchmark) exercising every construct
//! named in `.atlas/contracts/SEMANTIC-EXTRACTION.md`'s R4.3 minimum: free/public/async/unsafe/
//! extern/generic functions, struct/generic-struct/enum/trait/impl/impl-method/trait-method/
//! type-alias/const/static declarations, nested modules, references, tuples, `Option`/`Result`,
//! and one malformed file. Nothing here executes the fixture source; it is only ever parsed.

use super::super::batch::ExtractionBatch;
use super::super::extractor::{ExtractionInput, SemanticExtractor};
use super::{RustSemanticExtractor, spelling};
use atlas_core::{
    ArtifactId, CallDispatchKind, CallSiteIdentity, ContentFingerprint, ControlFlowBlockIdentity,
    ControlFlowBlockKind, ControlFlowEdgeKind, DataFlowResolution, EffectCategory, EffectIdentity,
    EpistemicStatus, FunctionDeclarationKind, FunctionIdentity, FunctionSignature, RepositoryId,
    RevisionRef, SemanticDimension, SemanticObservation, SemanticRecordId, StateAccessIdentity,
    StateAccessKind, StateResolution, SymbolIdentity, SymbolRole, TypeIdentity, ValueIdentity,
    ValueRole,
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

const UNSUPPORTED_DIMENSIONS: [SemanticDimension; 3] = [
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
/// unresolved use (no matching local definition), and a not-modeled tuple-destructuring pattern
/// (documented gap: no Definitions emitted for its sub-bindings).
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
    ] {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_eq!(obligation.status, EpistemicStatus::Unknown);
        assert!(!obligation.diagnostics.is_empty());
    }
    for &dimension in &UNSUPPORTED_DIMENSIONS {
        assert_eq!(
            batch.obligation_for(dimension).unwrap().status,
            EpistemicStatus::Unsupported
        );
    }
}

// --- 20. unsupported dimensions are always explicit, never silently dropped --------------------

#[test]
fn unsupported_dimensions_are_always_explicit() {
    let batch = extract_all("src/lib.rs", CORPUS);
    assert!(batch.is_closed(&ALL_DIMENSIONS));
    for &dimension in &UNSUPPORTED_DIMENSIONS {
        let obligation = batch.obligation_for(dimension).unwrap();
        assert_eq!(obligation.status, EpistemicStatus::Unsupported);
        assert!(!obligation.diagnostics.is_empty());
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

// --- 57. a tuple-destructuring pattern is a documented, honest gap: no Definitions for its
//     sub-bindings, so subsequent uses of them are explicitly UNRESOLVED, never fabricated -------

#[test]
fn tuple_destructuring_pattern_is_not_modeled_and_its_uses_stay_unresolved() {
    let batch = extract_all("src/lib.rs", DATA_FLOW_CORPUS);
    let caller = find_function_identity(&batch, &[], "destructures_a_tuple").unwrap();
    let values = data_flow_values_for(&batch, caller);

    assert!(
        values.iter().all(|v| v.role != ValueRole::Definition),
        "a `let (a, b) = ..;` pattern must not produce fabricated Definitions this wave"
    );
    let uses: Vec<_> = values.iter().filter(|v| v.role == ValueRole::Use).collect();
    assert_eq!(
        uses.len(),
        2,
        "uses of both `a` and `b` in the tail expression"
    );
    assert!(
        uses.iter()
            .all(|u| u.resolution == DataFlowResolution::Unresolved),
        "with no Definition ever registered for a/b, their uses must stay explicitly UNRESOLVED"
    );
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

// --- 67. a nested field chain (`self.inner.value`) reports a Read of the outer field only -- the
// documented gap, not a silent one ----------------------------------------------------------------

#[test]
fn nested_field_chain_reports_only_the_outer_field_as_a_documented_gap() {
    let batch = extract_all("src/lib.rs", STATE_EFFECT_CORPUS);
    let caller = find_function_identity(&batch, &["impl:Counter"], "read_nested").unwrap();
    let accesses = state_accesses_for(&batch, caller);
    assert_eq!(accesses.len(), 1);
    assert_eq!(accesses[0].kind, StateAccessKind::Read);
    assert_eq!(
        accesses[0].name, "inner",
        "only the outer field is modeled this wave"
    );
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
