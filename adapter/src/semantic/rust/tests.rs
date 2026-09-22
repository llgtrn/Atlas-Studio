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
    ArtifactId, ContentFingerprint, EpistemicStatus, FunctionIdentity, FunctionSignature,
    RepositoryId, RevisionRef, SemanticDimension, SemanticObservation, SymbolIdentity, SymbolRole,
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

const UNSUPPORTED_DIMENSIONS: [SemanticDimension; 8] = [
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

    // Never claimed this wave: no Call/ControlFlow/DataFlow/State/Effect observation exists,
    // even though the body plainly contains a `todo!()` call expression.
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
