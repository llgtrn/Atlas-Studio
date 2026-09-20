# Code Atlas / Absorption Compiler Blueprint

Status: **ACTIVE ENGINEERING BLUEPRINT**

Canonical decision: ADR-0028.

## 1. Goal

Turn heterogeneous donor code into bounded, evidence-linked engineering work that is implemented only in Chronica.

~~~text
donor source
-> Code Atlas
-> BehaviorSlice
-> AbsorptionWorkPacket
-> CHRONICA_BUILD
-> proof
-> canonical merge
-> implementation evidence
-> donor extinction
~~~

This is not file-for-file transpilation.

## 2. Code graph stages

Wave A:

~~~text
file
-> language
-> symbol
-> import
-> call candidate
-> external-effect candidate
-> state-mutation candidate
~~~

Wave B:

~~~text
symbol + calls + effects + mutations
-> BehaviorSlice
-> risk tags
-> bounded dependency/evidence set
~~~

Wave C:

~~~text
BehaviorSlice
-> optional translation candidate
-> native Chronica implementation
-> differential / behavioral tests
-> rustc / clippy / Miri / Kani as applicable
~~~

Wave D:

~~~text
Network Code Atlas
-> duplicate semantics
-> missing native primitive
-> architecture drift
-> innovation opportunity
-> bounded engineering plan
~~~

## 3. Donor roles

~~~text
Tree-sitter   parser/frontend reference
ast-grep      structural matching/rewrite reference
Joern         semantic/code-property graph reference
C2Rust        C -> Rust candidate reference
py2many       Python -> Rust candidate reference
rust-analyzer Rust semantics/reference resolution
Miri          UB detection
Kani          bounded model checking
~~~

Every donor follows normal Chronica native-technology evolution and may be retired after native replacement proof.

## 4. Evidence contract

Every behavior candidate carries source ranges and confidence.

~~~text
LOW     weak lexical/call evidence
MEDIUM  explicit effect/state-mutation evidence
HIGH    reserved for stronger parser/semantic/differential proof
~~~

The current heuristic frontend does not emit HIGH behavioral confidence merely from lexical matching.

## 5. Dev Cell flow

~~~text
temporary/donors/Dxxx/source/
        ↓
Code Atlas
        ↓
evidence/donors/Dxxx.code-atlas.json
        ↓
bounded census
        ↓
evidence/donors/Dxxx.census.json
        ↓
work/donors/Dxxx.json
        ↓
CHRONICA_BUILD
~~~

The Dev repository remains evidence-only.

## 6. Admission

Translation/AI output is never auto-admitted.

A successful replacement needs at least:

~~~text
exact donor revision
source/effect/state evidence
bounded behavior contract
Chronica responsibility owner
native implementation
tests / differential fixtures
architecture/authority checks
component CI
rollback/recovery evidence where applicable
canonical merge evidence
~~~

High-risk authority, financial, privacy, or physical behavior requires stronger proof before ACT.

## 7. Extinction

Donor code may be removed only after the relevant behavior/technology is preserved by native implementation and evidence.

Code deletion without semantic replacement is not absorption.


## 8. Wave 2 materialization

Wave 2 is implemented as development tooling under `tools/system-atlas/`.

It adds four concrete surfaces:

~~~text
Backend Registry
  optional parser/semantic/proof backends
  required_for_runtime = false

Structural Findings
  rule-driven engineering hotspots
  evidence + confidence + REVIEW_REQUIRED

Code Property Graph
  FILE / SYMBOL / IMPORT / CALL / EFFECT / MUTATION / FINDING
  bounded dependency closure per BehaviorSlice

Developer Query
  symbol / path / risk / effect / rule / slice
  -> exact evidence + closure
~~~

Generation-2 donor census now requires:

~~~text
Dxxx.code-atlas.json
Dxxx.structural.json
Dxxx.code-property-graph.json
Dxxx.census.json
~~~

Only after all four validate may Fleet create a schema-valid AbsorptionWorkPacket.

The work packet carries a bounded dependency closure. That closure is the default maximum implementation scope for the following `CHRONICA_BUILD`. If engineering discovers a dependency outside the closure, the correct action is to regenerate/review the Code Atlas packet, not silently expand the task.


## 9. Deterministic Atlas rule

Host-installed tools are not canonical evidence.

System Atlas generation reads the backend registry but does not probe the host. A separate developer command may probe optional binaries for convenience. This prevents two machines at the same commit from producing different canonical Atlas shards merely because one machine has Joern/Miri installed and the other does not.
