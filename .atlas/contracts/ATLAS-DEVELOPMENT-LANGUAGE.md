---
id: atlas.contract.development-language
type: contract
status: active
canonical: true
---
# Atlas Development Language Contract

## Purpose

Atlas Development Language (ADL) is the semantics-first development language of Atlas Studio. It exists so humans, agents and Atlas itself can author systems directly in the same typed semantic universe that strict census reconstructs from existing source.

ADL is not defined by donor syntax and is not merely an architecture-description DSL. Its authority comes from Atlas semantic contracts, Genome policy and admitted evidence.

ADL is explicitly a **Human + AI collaborative authoring surface**. The normative collaboration model is `HUMAN-AI-ADL-AUTHORING.md`.

It MUST NOT degrade into either:

- a human-only text language with an AI autocomplete feature;
- an AI-only prompt language that hides engineering meaning from humans.

Humans steer intent/trade-offs. External AI researches/synthesizes/codes candidates. Atlas validates/adopts semantics.

## One semantic world

Existing source and ADL converge before canonical publication:

```text
Rust / C / C++ / TypeScript / other admitted source
        ↓ inventory + semantic census
typed OBSERVED semantic records ───────┐
                                       │
ADL source                             │
        ↓ parse + elaborate            │
typed DECLARED semantic records ───────┤
                                       ↓
                         canonical census world
                                       ↓
                          normalize / reconcile
                                       ↓
                            selected design
                                       ↓
                               *.atlas
```

ADL does not create a second graph, type system or truth vocabulary beside Census/ATLAS.

## Collaborative authoring model

The mature authoring environment supports multiple synchronized projections:

~~~text
Human conversation
ADL text
Studio graph
constraints
semantic diff
evidence
benchmark results
agent structured edits
        ↓
typed candidate semantics
        ↓
one canonical Atlas path
~~~

A conversational request is not canonical meaning by itself.

Material AI-assisted authoring MUST produce typed candidate artifacts governed by:

- `HUMAN-AI-ADL-AUTHORING.md`;
- `ATLAS-CREATION-PIPELINE.md`;
- `EXTERNAL-PROVIDER-TRUST.md`.

External providers may generate real implementation candidates during authoring. Generated code is then ingested/censused/verified before selection and seal.

## ADL0 bootstrap subset

The current `.atlas/declared/*.adl` syntax and `core/src/language/adl` implementation are **ADL0**, the bootstrap declaration subset.

ADL0 currently expresses architectural declarations such as:

- entity;
- relation;
- capability;
- binding;
- constraint/invariant;
- transform;
- materialization.

ADL0 is useful and remains supported, but successful parsing of ADL0 MUST NOT be interpreted as completion of the full Atlas Development Language.

ADL0 is a strict subset/front-end of the future language family, not an independent semantic authority.

## Semantic-first language model

Full ADL MUST be able to represent, directly or through typed lowerings, the semantic families required by Atlas:

- repository/module/scope and stable identity;
- functions/methods and signatures;
- symbols and types;
- values, blocks and control flow;
- call and dispatch semantics;
- data flow and alias relationships;
- state and state transitions;
- effects and external interactions;
- resource/ownership/borrowing/lifetime semantics where applicable;
- capabilities, interfaces and bindings;
- concurrency, synchronization and ordering;
- persistence, transaction and recovery semantics;
- temporal events and lifecycle;
- constraints, invariants and authority barriers;
- evidence/provenance requirements;
- materialization and target-profile requirements.

The source syntax for these families may evolve. Their semantic meaning MUST map to the universal Atlas spine rather than creating syntax-owned truth.

## Types, effects and resources

ADL's mature type system is not frozen by this document. It MUST eventually define explicit semantics for:

- nominal/structural identity as selected by Genome;
- generic parameters and constraints;
- callable signatures and ABI boundaries;
- mutability and aliasing;
- resource ownership/lifetime/region behavior;
- effects;
- capability authority;
- concurrency safety;
- persistence/transaction boundaries;
- failure and recovery.

Language features are admitted only when their semantic contract is explicit enough to lower without hidden runtime meaning.

## Donor independence

Rust, LLVM, Wasmtime, Arrow and other donors may provide evidence for mechanisms, invariants and trade-offs. They do not define ADL syntax or canonical semantics.

For example, Atlas may learn from Rust borrowing without copying Rust's surface syntax. The admitted ADL primitive must be defined in Atlas terms: identity, capability, aliasing, lifetime/region, mutation rights, failure conditions and evidence.

No ADL feature may require a donor runtime merely because that donor inspired the feature.

## Compilation and creation authority

Pure ADL lowering remains:

~~~text
ADL source
→ syntax tree
→ name/scope resolution
→ typed semantic elaboration
→ obligation/evidence checks
→ canonical typed Atlas records
~~~

But material product creation may include a larger Human+AI engineering loop before seal:

~~~text
typed intent / ADL
→ constraint envelope
→ research / OSS discovery
→ candidate mechanisms
→ typed decision proposal
→ external synthesis / code
→ CandidateChangeSet
→ generated-source census
→ validation
→ SelectedDesign
→ SEALED logical Atlas
→ mechanical compaction
→ *.atlas
→ *.atlasx/
→ HIR/MIR/LIR/Machine IR or delegated backend
~~~

The larger creation path is normative in `ATLAS-CREATION-PIPELINE.md`.

A parser AST is not canonical truth. A pretty-printer string is not canonical truth. A provider response is not canonical truth. The typed semantic records plus admitted evidence are the language/compiler boundary.

## Epistemic status

Authored ADL claims enter the canonical path as `DECLARED` unless a deterministic compiler pass derives a new record under a named rule.

Compilation MUST NOT silently promote declarations to `OBSERVED`. Observation still requires admitted evidence such as source census, compiler metadata, tests, traces or other allowed evidence paths.

## Feature admission

A new ADL feature requires:

1. a typed semantic definition;
2. identity/scope rules;
3. interaction with status/evidence/provenance;
4. normalization/reconciliation behavior;
5. lowering or materialization semantics;
6. failure/UNKNOWN behavior;
7. compatibility/versioning rules;
8. verification fixtures;
9. no hidden donor-runtime dependency.

Features learned from donors additionally require the donor-to-language genesis process.

## Non-goals

ADL is not:

- a textual serialization of `SemanticFact { subject, predicate, object }`;
- a wrapper around Rust syntax;
- an LLVM IR skin;
- a donor API aggregation language;
- a UI state format;
- a shortcut that bypasses Census, normalization or reconciliation;
- a prompt archive;
- a provider-specific language;
- an opaque AI-generated code container.

## Current foundation

R4.5 establishes real typed Rust semantics through SYMBOL, TYPE, FUNCTION_IDENTITY, FUNCTION_SIGNATURE and CALL, including strengthened R4.4 function declaration identity and real R4.5 function-body call-site extraction through the canonical Census/Normalization path.

CALL targets remain explicitly unresolved where source syntax does not prove them.

This is a prerequisite for language genesis, not completion of ADL: CONTROL_FLOW, DATA_FLOW, STATE, EFFECT, OWNERSHIP, CONCURRENCY and PERSISTENCE remain to be materially implemented before the full language can be claimed.

## Readiness gates

Full ADL v1 is not ready to claim until Atlas has at least:

- lossless typed semantic records through Census/Normalization;
- stable universal identity/scope rules;
- typed function/type/call/control/data/state/effect families;
- explicit resource/ownership/effect semantics for the declared profile;
- deterministic ADL → semantic-record lowering;
- semantic diagnostics and source mapping;
- ATLAS publication capable of preserving all language meaning;
- differential verification against selected reference workloads.

Until then, ADL0 remains the bootstrap declaration subset.
