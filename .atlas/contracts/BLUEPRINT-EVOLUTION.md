---
id: atlas.contract.blueprint-evolution
type: contract
status: active
canonical: true
---
# Evidence-Driven Blueprint Evolution Contract

## Purpose

Atlas blueprints are canonical execution/design intent for the current evidence state.

They are **not immutable dogma**.

Atlas is explicitly allowed to revise a blueprint when census, dependency census, experiments, proofs, benchmarks, runtime evidence, compiler evidence or absorbed donor technology demonstrates a materially better design.

The permission to improve a blueprint does NOT grant implementation agents permission to improvise architecture silently.

This contract defines the only acceptable path from:

~~~text
new evidence
→ candidate better mechanism/design
→ blueprint revision
→ new canonical blueprint
~~~

## Authority hierarchy

The repository distinguishes four levels:

~~~text
Constitution / hard system invariants
        ↓
Contracts / Genome requirements
        ↓
Blueprints / roadmaps / selected architecture
        ↓
Implementation
~~~

A blueprint MAY change when evidence supports a better design.

A blueprint revision MUST NOT silently violate a higher-level contract or Genome invariant.

If the better design requires changing a contract or Genome requirement, that is a separate explicit contract-revision decision. The implementation agent MUST NOT treat a blueprint edit as permission to bypass a contract.

## Canonical does not mean immutable

For blueprints:

~~~text
canonical = currently selected authoritative design
canonical ≠ permanently frozen design
~~~

A superseded blueprint remains historical evidence, but only the newly selected canonical revision governs future work.

The objective is not to preserve an old blueprint. The objective is to preserve Atlas invariants while continuously selecting the strongest evidence-backed design.

## Valid blueprint-revision triggers

A blueprint revision MAY be proposed when evidence reveals any of:

- a donor or transitive dependency implements a materially better mechanism;
- an existing Atlas mechanism has a correctness weakness;
- a simpler invariant can replace a more complex design;
- a different data structure or algorithm improves the declared objective;
- a better semantic representation removes duplication or ambiguity;
- a better binary/layout/compaction strategy preserves meaning while improving size, locality or query cost;
- a better incremental/fixed-point strategy reduces recomputation;
- a better compiler lowering or IR boundary improves correctness, optimization or backend independence;
- a better verification method closes an important proof gap;
- a better ownership/concurrency/persistence design improves safety or determinism;
- new dependency-census evidence shows the current attribution/ownership model is wrong;
- experiments or benchmarks falsify a blueprint assumption;
- an implementation attempt exposes a contradiction in the current blueprint;
- a later donor lane reveals a prerequisite or mechanism that should be pulled forward under the canonical absorption rules.

Novelty, aesthetics or model preference alone are not valid revision triggers.

## Evidence classes

A blueprint revision may use:

- exact pinned source observations;
- typed census facts;
- dependency-census attribution;
- test/runtime traces;
- binary/compiler metadata;
- reproducible benchmarks;
- formal proof/model checking;
- differential comparison;
- research/specification claims as supporting theory;
- model analysis as candidate reasoning only.

Model output, README prose or an unverified paper claim MAY suggest a candidate revision but MUST NOT independently authorize canonical replacement.

## BlueprintRevisionDecision

The machine schema is `../schemas/blueprint-revision-decision.schema.json`.

Every material blueprint change MUST have a durable revision decision conforming to that schema and carrying at least:

- revision identity;
- current blueprint identity/revision;
- proposed blueprint identity/revision;
- triggering discovery/evidence;
- exact provider/repository/revision when donor technology is involved;
- problem in the current blueprint;
- candidate mechanism/design;
- alternative designs considered;
- declared objective(s);
- correctness/invariant analysis;
- affected ArchitecturalIntegrityEnvelope identity and added/removed/changed/superseded invariant identities;
- load-bearing topology/equivalence impact;
- performance/resource analysis where applicable;
- affected native owners;
- affected R-wave/W-wave sequencing;
- affected semantic schemas/identities;
- affected binary/wire/storage formats;
- affected AtlasX/compiler stages;
- affected contracts/Genome requirements;
- migration/compatibility plan;
- recensus plan;
- verification/falsification plan;
- rollback plan when material;
- decision state;
- rationale.

Schema-validity does not by itself select the revision; the evidence/validation gates in this contract still apply.

## Decision states

Blueprint revision decisions use the following conceptual states:

~~~text
PROPOSED
EVIDENCE_GATHERING
VALIDATED
SELECTED
SUPERSEDED
REJECTED
~~~

Only SELECTED changes canonical blueprint authority.

PROPOSED or VALIDATED is not permission to rewrite production architecture as though the new blueprint were canonical.

## Better is profile-specific when necessary

"Better" must be tied to declared criteria.

Examples:

- correctness;
- semantic completeness;
- determinism;
- query latency;
- publication size;
- compile time;
- runtime latency;
- throughput;
- memory;
- startup;
- binary size;
- energy;
- implementation complexity;
- verification burden;
- dependency independence;
- portability.

A mechanism may dominate globally or only for a declared profile.

Do not replace a general blueprint with a benchmark-specific optimization while pretending it is universally better.

If alternatives serve materially different profiles, the blueprint may admit explicit strategy selection rather than forcing one universal mechanism.

## Required comparison against current blueprint

A proposal MUST explain why the current canonical design is insufficient.

The minimum comparison is:

~~~text
current design
vs
candidate design
vs
known viable alternatives
~~~

For each material axis, record:

- preserved invariants;
- strengthened invariants;
- weakened trade-offs;
- new dependencies;
- removed dependencies;
- migration cost;
- verification cost;
- performance/resource effect;
- compatibility impact.

A revision cannot be justified by describing only the candidate.

## Census may change the blueprint

This rule is explicit and intentional:

> During donor or dependency census, Atlas may discover a mechanism that is superior to the current blueprint. Atlas MAY revise the blueprint to adopt that mechanism or a synthesized Atlas-native variant, provided the revision passes this contract.

The normal loop is:

~~~text
canonical blueprint N
        ↓
census donors + dependencies
        ↓
discover better mechanism
        ↓
attribute actual provider
        ↓
deep census mechanism
        ↓
BlueprintRevisionDecision
        ↓
validate / benchmark / prove
        ↓
SELECTED
        ↓
canonical blueprint N+1
        ↓
update roadmap/contracts if required
        ↓
implement Atlas-native design
        ↓
recensus / verify
~~~

Blueprint evolution is therefore part of the Atlas self-building loop, not an exception to it.

## No direct donor architecture import

A donor may prove that the current blueprint should change.

That does NOT mean Atlas copies the donor's module/package topology.

The revision decision extracts:

- mechanism;
- invariant;
- algorithm;
- representation;
- trade-off;
- evidence.

Then maps it into Atlas-native ownership.

Permanent donor-named architecture remains forbidden.

## Contract-impact rule

If a proposed blueprint contradicts an active contract:

1. do not implement through the contradiction;
2. identify the exact conflicting contract clauses;
3. decide whether:
   - the blueprint candidate is invalid; or
   - the contract itself is now proven inadequate;
4. if the contract should change, create an explicit contract revision with compatibility/migration analysis;
5. only then select the new blueprint.

A blueprint PR MUST NOT silently weaken a contract.

## Architectural-integrity impact rule

ARCHITECTURAL-INTEGRITY.md is the canonical collapse-prevention contract.

A blueprint candidate that intentionally changes a HARD architectural invariant, load-bearing boundary, state owner, authority path, dependency direction, failure domain, persistence/concurrency boundary or lifecycle ordering MUST:

1. identify the exact current invariant identities affected;
2. state which invariants are preserved, added, changed or superseded;
3. define falsification/equivalence evidence for the proposed replacement;
4. update or replace the ArchitecturalIntegrityEnvelope only through the explicit revision decision;
5. recensus/revalidate the affected semantic impact closure after selection.

Before the BlueprintRevisionDecision reaches SELECTED, implementation that contradicts the current active envelope remains an architectural violation. Tests or performance gains do not authorize silent architecture mutation.

An implementation agent MUST NOT edit the envelope merely to make an already-written candidate pass.

## Schema/identity-impact rule

Blueprint changes affecting canonical identity or semantic schemas require explicit migration analysis.

Examples:

- FunctionIdentity shape;
- graph node/binding identity;
- semantic record identity;
- Atlas wire schemas;
- AtlasX identity;
- compiler IR identity;
- content addressing.

The revision MUST answer whether:

- old identities remain valid;
- new aliases/equivalence relations are needed;
- migration rewrites durable artifacts;
- old sealed Atlas roots remain readable;
- caches/shards can be reused;
- cross-revision lineage is preserved.

Silent identity reinterpretation is forbidden.

## R-wave/W-wave replanning

A selected blueprint revision MAY change future roadmap sequencing.

It may:

- split a planned wave;
- merge two planned implementation obligations;
- insert a prerequisite gate;
- pull forward a donor mechanism;
- defer a no-longer-needed wave;
- add a new bounded implementation wave.

However:

- the change MUST be explicit in canonical roadmap docs;
- the reason MUST link to evidence;
- already-completed capabilities MUST be recertified if the revision invalidates their assumptions;
- W-wave donor order remains distinct from R-wave Atlas maturity;
- a roadmap change MUST NOT retroactively claim unimplemented capabilities were complete.

## Same-wave emergency correction

If implementation discovers that the current blueprint is technically impossible or unsound, the agent MAY stop the current wave and prepare a blueprint revision.

It MUST NOT silently redesign while continuing to report the original wave as completed.

For a very small bounded correction that does not alter public identity/contracts/wave meaning, the revision may be included in the same PR only if the decision record and docs are updated before or alongside production code.

Material architectural changes require a docs/design selection step before dependent implementation continues.

## Recensus after blueprint revision

Any SELECTED blueprint revision that changes census semantics, identity, storage, materialization or compiler lowering MUST trigger targeted recensus.

At minimum consider:

- Atlas self-census;
- affected donor scopes;
- affected dependency scopes;
- Technology Genomes derived under old assumptions;
- selected absorption decisions;
- sealed Atlas/AtlasX compatibility;
- compiler differential fixtures;
- ArchitecturalIntegrityEnvelope/report roots and every affected load-bearing invariant/equivalence obligation.

A blueprint revision that invalidates prior evidence MUST mark the affected evidence/claims as superseded or requiring regeneration rather than silently carrying them forward.

## Blueprint revision and extinction

A donor scope MUST NOT be extinguished based on a blueprint that is still only PROPOSED/VALIDATED.

If a selected blueprint revision changes the Atlas-native replacement for a donor scope:

- reassess ABSORBED/EXTINCTION_READY state;
- recensus the replacement;
- rerun required tests/benchmarks/proofs;
- ensure durable knowledge still covers the mechanism;
- only then continue source deletion/extinction.

## Rollback

When a selected blueprint revision carries material execution risk, retain enough lineage to restore the prior selected design if verification fails.

Rollback does not erase evidence that the failed revision was attempted.

## Agent non-discretion rules

Implementation agents MUST NOT:

- treat canonical blueprints as permanently immutable;
- treat them as optional suggestions;
- silently replace them during implementation;
- claim a donor mechanism is "better" without declared criteria/evidence;
- use model preference as selection evidence;
- change identity/wire/compiler semantics without migration analysis;
- copy donor topology as the revised Atlas architecture;
- revise a blueprint to bypass a failing verification gate;
- use blueprint evolution to weaken evidence, census or extinction requirements.

## Final invariant

Atlas should preserve its **hard invariants**, not obsolete design choices.

When census proves a better mechanism, the correct behavior is:

~~~text
do not ignore it
do not copy it blindly
do not improvise secretly

deep-census it
compare it
prove it
revise the blueprint explicitly
implement it natively
recensus and verify
~~~

A blueprint that cannot evolve from better evidence would prevent Atlas from learning from the OSS world it is explicitly designed to census.
