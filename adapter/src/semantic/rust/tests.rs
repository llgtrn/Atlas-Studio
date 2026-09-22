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
    ArtifactId, CallDispatchKind, CallSiteIdentity, ContentFingerprint, EpistemicStatus,
    FunctionDeclarationKind, FunctionIdentity, FunctionSignature, RepositoryId, RevisionRef,
    SemanticDimension, SemanticObservation, SemanticRecordId, SymbolIdentity, SymbolRole,
    TypeIdentity,
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

const UNSUPPORTED_DIMENSIONS: [SemanticDimension; 7] = [
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
    // with R4.5 CALL support, this body contributes no Call observation. ControlFlow/DataFlow/
    // State/Effect remain wholly unclaimed this wave regardless of body content.
    assert!(batch.observations.iter().all(|observation| !matches!(
        observation,
        SemanticObservation::Call(_)
            | SemanticObservation::ControlFlow(_)
            | SemanticObservation::DataFlow(_)
            | SemanticObservation::State(_)
            | SemanticObservation::Effect(_)
    )));
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
    // with R4.5 CALL support; ControlFlow/DataFlow/State/Effect remain wholly unclaimed this wave.
    assert!(batch.observations.iter().all(|observation| !matches!(
        observation,
        SemanticObservation::Call(_)
            | SemanticObservation::ControlFlow(_)
            | SemanticObservation::DataFlow(_)
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
