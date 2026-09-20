---
id: ops-production-contracts-semantic-convergence
type: reference
status: active
canonical: true
---
# Development Cell / Package Semantic Convergence Contract

Status: **ACTIVE DEVELOPMENT CELL / PACKAGE CONTRACT**

This compatibility-path contract prevents a donor-derived Development Cell/package from recreating a Chronica behavior under a different name, model, command, state machine, service, event, policy or storage abstraction.

The goal is not superficial naming consistency. The goal is **intentional semantic convergence**:

```text
same meaning -> same canonical semantic
provider/donor vocabulary -> boundary alias only
richer reusable meaning -> propose upward into Chronica
truly product-local meaning -> remain product-local with an explicit boundary
```

Chronica remains the canonical semantic universe. Development Cells are proving grounds; Chronica Packages are domain/capability/adapter/projection compositions. They may discover stronger models, missing distinctions and better implementations, but they must not silently create a second vocabulary for the same world.

## 0. Fleet ownership before semantic convergence

Before comparing one donor against one Cell/package, establish fleet ownership:

```text
donor identity
-> one PRIMARY_ABSORBER
-> optional REFERENCE_CONSUMERS
```

A donor shared across several products is not a license for several independent donor-derived universal semantics. The primary absorber owns the deep donor census/proof. Secondary consumers may retain provider/product-local mechanics and later converge on the canonical result.

If an Cell Mirror report declares a primary donor that Fleet allocation assigns elsewhere, convergence is blocked.

## 0A. Three-way Mirror comparison

Semantic convergence is evaluated against three pinned realities, not by comparing donor names to package names in isolation:

```text
Chronica @ CHRONICA_REFERENCE_SHA
    canonical meaning + current implementation evidence

Cell/package @ CELL_SHA
    local product implementation + callers + tests

Donor @ DONOR_SHA
    proven source behavior + provenance
```

The Development Cell/package is the mutable target for the refactor slice. Chronica is read-only in that slice. A reusable general semantic discovered in the package becomes an explicit upward candidate; it does not silently rewrite Chronica.

The Ops Mirror Atlas should expose the exact pins plus local mapping/violation evidence so a later reviewer can reproduce which two repositories and which donor reality were compared.

## 1. Core invariant

For any meaningful package concept `o` and Chronica concept `c`:

```text
EquivalentMeaning(o, c) = true
=> Package MUST reuse, map to, or deliberately extend c
=> Package MUST NOT create a second canonical semantic merely because donor naming differs
```

Name equality is neither necessary nor sufficient:

```text
same name != same semantic
same semantic != same donor name
```

Semantic comparison therefore uses behavior, state and effects, not grep alone.

## 2. Semantic fingerprint

Before retaining or creating a material package abstraction, extract a semantic fingerprint from the real donor/package code.

At minimum inspect:

```text
current names / aliases
entity or resource acted upon
inputs
outputs
preconditions
postconditions
state transition
side effects
authority requirement
normative constraint where applicable
idempotency / correlation behavior
evidence produced
failure / UNKNOWN behavior
risk class where applicable
persistence ownership
provider or transport dependency
real callers
tests
```

The fingerprint is the comparison unit. A feature name is only one field.

## 3. Search Chronica by meaning and code

At an exact `CHRONICA_REFERENCE_SHA`, search both architecture and implementation reality.

Read relevant canonical owners, then inspect real code and tests for semantic equivalents. Search by:

```text
resource/entity meaning
state transition
command/query/event effect
invariant IDs
Rust/TypeScript symbols
API/MCP operation semantics
authority/evidence behavior
persistence effects
provider-independent postcondition
synonyms and donor terminology
```

Do not conclude `NO_MATCH` merely because the donor term does not appear verbatim in Chronica.

A semantic search must answer:

```text
Does Chronica already express this meaning?
Does Chronica implement it under another name?
Is Chronica's version narrower or broader?
Is the donor behavior provider-specific mechanics or reusable domain semantics?
Would keeping both create two ways to mean the same thing?
```

## 4. Required disposition

Every material donor/Ops semantic must be classified as exactly one primary disposition:

```text
REUSE_CHRONICA_SEMANTIC
ALIAS_AT_BOUNDARY
MAP_TO_CHRONICA_SEMANTIC
EXTEND_CHRONICA_SEMANTIC
KEEP_PRODUCT_LOCAL
KEEP_PROVIDER_MECHANIC
TEMPORARY_COMPATIBILITY_ALIAS
RETIRE_DUPLICATE_SEMANTIC
DEFER_UNRESOLVED_WITH_EVIDENCE
```

### REUSE_CHRONICA_SEMANTIC

Use the same canonical concept, vocabulary, contract and behavior directly.

### ALIAS_AT_BOUNDARY

A donor/provider term may survive only at the adapter/API compatibility boundary while internal package semantics use the Chronica term.

Example:

```text
provider "resolve_ticket"
        ↓ adapter translation
Chronica/package "conversation.close"
```

The provider term must not become a second internal domain primitive.

### MAP_TO_CHRONICA_SEMANTIC

The donor representation differs structurally but means the same thing. Translate it into the existing Chronica semantic and migrate callers toward that semantic.

### EXTEND_CHRONICA_SEMANTIC

The donor/package model contains a genuinely useful distinction that Chronica does not yet express.

Do not flatten the richer behavior merely for conformity. Instead:

```text
preserve donor evidence
-> model the extension explicitly
-> prove why existing Chronica semantics are insufficient
-> determine whether the distinction is universally reusable
-> if reusable, propose/implement it in Chronica's canonical owner
-> converge affected packages onto the resulting Chronica semantic
```

An Ops-local extension is temporary while a general canonical extension is being admitted. It must not quietly become a permanent competing ontology.

### KEEP_PRODUCT_LOCAL

The meaning is genuinely specific to the product projection and does not duplicate a universal/domain semantic already owned by Chronica. Its boundary to canonical Chronica semantics must remain explicit.

### KEEP_PROVIDER_MECHANIC

The behavior is provider/protocol mechanics. Keep it in an adapter and translate to canonical semantics before it crosses into the Ops application core.

## 5. Naming convergence gate

Before introducing a new public/internal semantic name, search Chronica and the Ops repository for semantic equivalents and aliases.

Fail the design review when:

```text
same state transition + same effect + same authority/evidence meaning
but different internal canonical names
```

unless the difference is explicitly classified as:

```text
provider alias
compatibility alias
versioned semantic change
real semantic distinction with evidence
```

Prefer one stable semantic name across:

```text
Ops application/domain code
UI commands
API commands
MCP tools
worker jobs
events
tests
docs
Chronica mapping
```

Transport/provider names may differ at adapters only.

## 6. Semantic parity record

Each substantial refactor vertical should leave a machine-readable or manifest-visible parity record containing enough evidence to answer:

```text
DONOR_TERMS
OPS_TERM_BEFORE
SEMANTIC_FINGERPRINT
CHRONICA_REFERENCE_SHA
CHRONICA_OWNER
CHRONICA_CODE_REFS
CHRONICA_TEST_REFS
MATCH_STATUS
DISPOSITION
CANONICAL_TERM_AFTER
TEMPORARY_ALIASES
EXTENSION_GAP_IF_ANY
RETIREMENT_TARGET_IF_ANY
```

`MATCH_STATUS` examples:

```text
EXACT_EQUIVALENT
EQUIVALENT_WITH_DIFFERENT_NAME
CHRONICA_NARROWER
CHRONICA_BROADER
PROVIDER_MECHANIC_ONLY
PRODUCT_LOCAL
NO_EQUIVALENT_FOUND
UNRESOLVED
```

`NO_EQUIVALENT_FOUND` is a search result, not permission to invent casually. Record the code/docs surfaces searched.

## 7. Ops can improve Chronica

Semantic convergence is intentionally bidirectional in learning, but unidirectional in canonical authority.

```text
Ops may discover better semantics
Ops may prove richer behavior
Ops may reveal missing invariants
Ops may implement a stronger candidate
        ↓
Chronica canonical owner reviews/adopts reusable meaning
        ↓
Ops converges onto the canonical result
```

Therefore:

```text
Ops discovery can improve Chronica
but Ops discovery does not automatically become canonical Chronica truth
```

This preserves the proving-ground role without creating permanent parallel universes.

## 8. Refactor algorithm

For every selected donor vertical:

```text
PIN CHRONICA_REFERENCE_SHA + OPS_SHA + DONOR_SHA
-> REFRESH OPS MIRROR ATLAS
-> READ REAL DONOR IMPLEMENTATION
-> EXTRACT SEMANTIC FINGERPRINT
-> SEARCH CHRONICA DOCS BY MEANING
-> SEARCH CHRONICA CODE + TESTS BY BEHAVIOR / SYMBOL / INVARIANT
-> CLASSIFY EQUIVALENCE
-> SELECT REQUIRED DISPOSITION
-> ADOPT CANONICAL VOCABULARY INTERNALLY
-> TRANSLATE DONOR/PROVIDER ALIASES AT BOUNDARY
-> IMPLEMENT / EXTEND
-> MIGRATE REAL CALLERS
-> MIGRATE UI/API/MCP/EVENT NAMES WHERE SAFE
-> TEST SEMANTIC PARITY
-> DELETE DUPLICATE LEGACY SEMANTIC
-> RECORD PARITY + RETIREMENT EVIDENCE
```

## 9. Semantic parity tests

Where donor and Chronica/package implementations coexist temporarily, differential tests should compare externally meaningful semantics rather than class names.

Check as applicable:

```text
same valid input -> same accepted semantic outcome
same invalid input -> equivalent rejection class
same precondition -> same state transition
same effect -> same evidence requirements
same authority state -> same admission result
same idempotency key -> equivalent replay behavior
provider timeout -> equivalent UNKNOWN/reconciliation behavior
same event meaning -> same canonical event classification
```

A renamed implementation is not parity proof.

## 10. Anti-patterns

Forbidden without explicit evidence-backed exception:

```text
Chronica has WorkRun; Ops invents JobExecution for the same semantic
Chronica has Binding; Ops invents ConnectorLink for the same meaning
Chronica uses Evidence; Ops invents AuditProof as a parallel universal primitive
Chronica derives Can(...); Ops creates a permanent Capability registry
Chronica has one state transition; Ops keeps donor transition under a second canonical command name
UI/API/MCP each expose different verbs for the same internal effect
```

Provider-specific DTO/action names may exist at adapters, but they must map into the shared semantic before application/core logic.

## 11. Freshness

Semantic convergence is not an intake-only activity.

Refresh the semantic comparison when:

```text
Chronica reference SHA materially advances
Ops introduces a new major abstraction
Ops absorbs another donor subsystem
an old compatibility alias is being retired
a richer Ops semantic is proposed for Chronica
an apparent duplicate semantic is discovered
```

## 12. Completion condition

A vertical is semantically converged only when:

```text
no unexplained duplicate meaning remains
canonical internal vocabulary is explicit
provider aliases are boundary-local
real callers use the converged semantic
parity tests/evidence exist where migration risk warrants them
temporary aliases/shims have retirement criteria
reusable richer semantics have a Chronica adoption decision
```

The terminal goal is not identical source code. It is **one intentional semantic language across Chronica and its Ops projections**.

# Final rule

**DO NOT ASK ONLY "DOES CHRONICA HAVE THIS NAME?" ASK "DOES CHRONICA ALREADY HAVE THIS MEANING, STATE TRANSITION, EFFECT, AUTHORITY, EVIDENCE OR FAILURE SEMANTIC?" IF YES, CONVERGE. IF OPS HAS A GENUINELY BETTER GENERAL SEMANTIC, PROVE IT, MOVE THE REUSABLE MEANING INTO CHRONICA, THEN CONVERGE OPS ONTO THAT CANONICAL RESULT.**
