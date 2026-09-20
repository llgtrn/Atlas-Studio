---
id: adr.0028
type: decision
status: accepted
canonical: true
---
# ADR-0028 — Code Atlas Is an Evidence-Linked ANALYZE Projection for Native Absorption

## Context

System Atlas can already measure repository topology, donor inventory, Development Cell drift, and responsibility-scoped CI. That is not enough for native absorption. A worker still needs bounded evidence about symbols, calls, state mutation, external effects, dependency closure, and behavior before reimplementing donor technology in Rust.

External projects such as Tree-sitter, ast-grep, Joern, C2Rust, py2many, rust-analyzer, Miri, and Kani provide useful technology references, but Chronica must not create a permanent donor-owned semantic/translation/proof substrate.

## Decision

Chronica adds Code Atlas as an executable System Atlas shard.

Code Atlas follows:

```text
source code
-> parse/census
-> evidence-linked code graph
-> BehaviorSlice candidate
-> AbsorptionWorkPacket
-> CHRONICA_BUILD
-> component CI / behavioral proof
-> canonical merge
-> implementation evidence
-> donor extinction when proven
```

### Truth boundary

Code Atlas output is ANALYZE.

It may describe:

- files and languages;
- symbols and imports;
- candidate call edges;
- external-effect candidates;
- state-mutation candidates;
- behavior slices;
- risk tags;
- translation candidates;
- proof requirements.

It is never canonical World truth, runtime authority, or execution admission.

Every inferred edge/slice carries evidence and confidence. Low-confidence analysis remains explicitly low-confidence.

### Donor use

D346-D353 are strategic technology donors for the Code Atlas wave.

Chronica may learn parsing, structural matching, semantic graph, translation, Rust semantic-analysis, UB-detection, and model-checking mechanics from them.

They are not terminal dependencies.

### Translation law

A transpiled artifact is only a candidate.

```text
C/C++/Python/etc
-> candidate translation
-> native Chronica refactor
-> behavioral/differential proof
-> Rust proof
-> admission
```

Mechanical translation alone cannot claim native absorption.

### Development Cell boundary

Generation-2 Dev repositories may store donor source and Code Atlas evidence.

They may not store product implementation.

A donor census is incomplete until both exist:

```text
evidence/donors/Dxxx.code-atlas.json
evidence/donors/Dxxx.census.json
```

Absorption work packets always target `llgtrn/Chronica`.

### Authority

```text
Code Atlas / AI / transpiler -> ANALYZE
WorkPacket                  -> ANALYZE
candidate Rust              -> ANALYZE
CI / proof                  -> EVIDENCE
merge/integration           -> ACT
Chronica main               -> canonical repository state
```

No parser, model, transpiler, or static analyzer receives merge authority.

## Sequencing

1. Native heuristic Code Atlas foundation.
2. Ingest/census strategic analysis donors.
3. Replace heuristics with stronger parser/semantic primitives where evidence supports it.
4. Add bounded data/control-flow and differential proof.
5. Add innovation/opportunity detection across the Network Atlas.
6. Retire donor runtime dependence once native parity is proven.

This extends ADR-0026 and ADR-0027.
