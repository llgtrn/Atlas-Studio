---
id: atlas.roadmap.r4-completion-matrix
type: blueprint
status: active
canonical: true
---
# R4 Completion Matrix

R4 is complete only when contract, runtime, evidence, and graph behavior agree. Documentation alone is not completion.

| Wave | Scope | Contract gate | Runtime gate | Current state |
| --- | --- | --- | --- | --- |
| R4.0 | ontology lock | semantic/status/normalization/extractor/certificate contracts canonical | core exposes matching typed contracts | COMPLETE after this change |
| R4.1 | typed semantic records | Function/Symbol/Type/Call/CFG/DataFlow/State/Effect/Ownership/Concurrency/Persistence defined | records can be serialized deterministically | FOUNDATION |
| R4.2 | Rust semantic extraction | extractor obligations fixed | real Rust source produces typed symbol/type/function/call facts | NOT STARTED |
| R4.3 | control/data/state/effect | coverage rules fixed | CFG/dataflow/state/effect facts emitted with evidence | NOT STARTED |
| R4.4 | normalization | canonical identity/equivalence/conflict rules fixed | deterministic IDs, equivalence accounting, conflict candidates | BOOTSTRAP |
| R4.5 | reconciliation | multi-engine rules fixed | disagreements and dynamic unknowns reconciled to fixed point | NOT STARTED |
| R4.6 | CensusCertificate | schema fixed | certificate emitted from real census/reconciliation state | SCHEMA READY |
| R4.7 | graph materialization | projection-only rule fixed | typed reconciled semantics materialize without competing truth | BOOTSTRAP |
| R4.8 | acceptance | all above satisfied for required corpus/profile | tests and evidence prove closure | NOT READY |

## Non-negotiable acceptance checks

R4 cannot be marked complete while any of these are true:

- function semantics exist only as generic subject/predicate/object triples;
- required semantic dimensions are silently absent;
- UNKNOWN and UNSUPPORTED are conflated;
- normalization changes epistemic status;
- conflicting extractor output is silently collapsed;
- graph state cannot be rebuilt from normalized/reconciled semantic records;
- CensusCertificate is documentation-only with no machine schema/runtime owner;
- a semantic extractor writes directly into graph truth.

## Current permitted bootstrap

The existing SemanticFact envelope and normalized graph path may remain while typed extractors are being materialized.

They are compatibility infrastructure, not the final R4 semantic representation.
