---
id: atlas.contract.donor-to-language-genesis
type: contract
status: active
canonical: true
---
# Donor to Language Genesis Contract

## Purpose

Atlas Studio uses OSS and research donors to learn engineering mechanisms, invariants and trade-offs, then synthesizes Atlas-native technology and Atlas Development Language primitives.

Donors are evidence and reference implementations. They are not permanent runtime owners and do not dictate ADL syntax.

## Canonical flow

```text
approved donor set
      ↓
pin exact revision + license + provenance
      ↓
coarse census
      ↓
capability/dependency graph
      ↓
deep semantic census of selected mechanisms
      ↓
Technology Genome
      ↓
cross-donor comparison
      ↓
candidate Atlas primitive
      ↓
semantic + empirical validation
      ↓
Genome/language admission
      ↓
Atlas-native implementation
      ↓
differential tests / benchmarks / recensus
      ↓
donor scope ABSORBED
      ↓
donor runtime dependency = 0
```

Skipping directly from donor syntax/API to an ADL feature is forbidden.

## Technology Genome

For an absorbed mechanism, Atlas should retain a typed Technology Genome equivalent to:

```text
TechnologyGenome
├─ capability / problem
├─ semantic mechanism
├─ required invariants
├─ identity/scope model
├─ state/effect/resource model
├─ failure and recovery behavior
├─ concurrency/temporal behavior
├─ performance characteristics
├─ portability/ABI constraints
├─ evidence references
├─ donor revisions/licenses
├─ known trade-offs
├─ rejected alternatives
└─ dependency/extinction status
```

A Technology Genome describes why and how a capability works, not merely which donor function implements it.

## Mechanism before syntax

Examples:

- Rust may evidence ownership, borrow exclusion, region/lifetime constraints, trait dispatch and diagnostic strategies.
- LLVM may evidence SSA, optimization legality, target lowering and data-layout mechanisms.
- Wasmtime may evidence sandbox/resource/capability boundaries.
- Arrow may evidence columnar layout and vectorized data semantics.
- FlatBuffers may evidence schema/layout and zero-copy trade-offs.
- mold/object tooling may evidence linking, relocation, symbol and object-format mechanisms.
- compression/hash donors may evidence chunking, dictionaries, content addressing and integrity strategies.

Atlas extracts these mechanisms into its own semantic vocabulary before choosing an ADL surface form.

## Cross-donor comparison

A candidate primitive SHOULD compare more than one implementation/research source when practical.

Comparison asks:

- What invariant is universal?
- What behavior is implementation-specific?
- Which semantics are externally observable?
- What cost model/trade-off exists?
- Which failure modes are hidden by the donor API?
- Can the mechanism be represented more directly in Atlas's universal graph?
- Can Atlas remove historical abstraction boundaries while preserving behavior?

A popular donor is not automatically the correct design.

## ADL feature admission

A donor-informed ADL feature is admitted only when:

1. the semantic problem is explicit;
2. donor evidence is pinned and attributable;
3. the mechanism/invariants are represented independently of donor syntax;
4. the feature maps to Atlas typed semantic families;
5. interactions with type/effect/resource/state/concurrency semantics are defined;
6. diagnostics and unresolved behavior are defined;
7. lowering/materialization behavior is defined;
8. differential fixtures or other proof exist;
9. licensing/provenance obligations are preserved;
10. the resulting native capability does not require the donor runtime unless intentionally declared as an adapter.

## Extinction

Donor source may be deleted from `.atlas/temporary/` only after the relevant knowledge is durable in Atlas and the extinction gate in `BULK-DONOR-ABSORPTION.md` is satisfied.

The durable result is not "we once read the donor." It is:

```text
typed semantic knowledge
+ Technology Genome
+ evidence/provenance
+ native implementation
+ verification
```

## Research sources

Papers, RFCs, specifications and analyses can participate in Technology Genome construction, but they remain research claims until corroborated according to Atlas epistemic rules.

Research authority never upgrades itself to observed implementation truth.

## Anti-patterns

Forbidden as language-genesis strategy:

- copying donor surface syntax because it is familiar;
- wrapping donor APIs and calling the wrapper Atlas-native;
- treating transpilation as semantic absorption;
- storing only prose summaries of mechanisms;
- deleting donor source before evidence/mechanisms are durable;
- choosing a primitive only because one donor uses it;
- making ADL semantics depend on a donor's undocumented behavior.
