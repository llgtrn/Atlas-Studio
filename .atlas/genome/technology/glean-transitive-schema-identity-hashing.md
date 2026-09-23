---
id: atlas.genome.technology.glean-transitive-schema-identity-hashing
type: technology-genome
status: active
canonical: true
---
# Technology Genome: cycle-aware, transitive structural hashing of a type/predicate dependency graph (Glean)

Donor: Glean (`facebookincubator/Glean`, commit `2a48dea4cddb316d3b8bd65b54965cb9a7855c52`),
source-intelligence lane, Wave 0/1.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It goes
beyond the existing deep census (`.atlas/census/donors/source-intelligence-lane-2026-09-20.md`'s Glean
section, lines 214-256, which names "typed fact schema" and "typechecked query IR" at a high level but
does not describe the identity-hashing mechanism) by reading the donor's actual identity-computation
source directly: `glean/angle/Glean/Angle/Types.hs` (`TypeId`/`PredicateId` definitions) and
`glean/db/Glean/Database/Schema/ComputeIds.hs` (152 of 1216 and 108-300 of a larger file, read
directly, not merely grepped) — a mechanism the existing census did not surface at all, found by
following "how is a `TypeId`/`PredicateId` actually computed" past the type definitions into their
construction logic.

## Capability / problem

Given a schema made of types and predicates (Glean's term for a typed relation/fact kind) that can
reference each other — including mutually, forming reference cycles — assign each type/predicate a
stable identity such that (1) two schemas that declare the exact same types/predicates, transitively,
always produce the same identities, (2) any change to a type/predicate's own definition or to anything
it depends on (directly or transitively) changes its identity, and (3) the computation is deterministic
even when the dependency graph contains cycles (mutually recursive type/predicate definitions).

## Semantic mechanism (as observed in the donor, evidence gathered directly from source in this pass)

- **Dual identity: a human-readable, schema-versioned reference plus a content-derived hash**
  (`Types.hs` lines 566-572, 585-594, read directly): `PredicateId { predicateIdRef :: PredicateRef,
  predicateIdHash :: Hash }` and `TypeId { typeIdRef :: TypeRef, typeIdHash :: Hash }`, where
  `PredicateRef`/`TypeRef` are name+version pairs (e.g. `python.Name.1`) and `Hash` is a structural
  fingerprint. `Hashable`/`Eq` for both types are defined **only over the hash component**
  (`TypeId _ a == TypeId _ b = a == b`) — the ref is retained for display/tooling but never
  participates in equality directly (though, per the next point, it is folded into what the hash is
  computed *from*).
- **The hash is computed transitively over the reference graph, not just the definition's own local
  shape** (`ComputeIds.hs` lines 112-274, read directly): `computeIds` first builds a graph of
  predicate/type definitions and their `PredicateRef`/`TypeRef` dependencies (lines 123-141), then
  topologically sorts it with `stronglyConnComp` (line 143) so every definition is processed only after
  everything it depends on already has a final `Hash`. `refsToIds` (called inside `resolveDef`, lines
  235-255) rewrites every reference inside a definition from a `PredicateRef`/`TypeRef` (name+version)
  to the dependency's already-computed `PredicateId`/`TypeId` (name+version+hash) before
  `fingerprintDef` (lines 268-274) hashes the whole thing — so a definition's own hash is a function of
  every dependency's hash, transitively, exactly the same shape as a Merkle DAG (each node's hash
  commits to its children's hashes, not just its own content).
- **`fingerprintDef`'s own comment states the identity contract precisely** (lines 264-267, quoted
  directly): *"two predicates or types are the same only if they [have] the same name, same version,
  and same representation"* — the ref (name+version) is hashed together with the (reference-resolved)
  structural body, not omitted from it; identity is neither pure-name nor pure-structure but both.
- **Cycles are handled explicitly and deterministically, with a documented reason for the extra
  indirection** (lines 150-174, read directly): when `stronglyConnComp` reports a `CyclicSCC` (mutually
  recursive definitions), the algorithm (1) sorts the cycle's definitions by ref for determinism, (2)
  assigns every ref in the cycle a placeholder `hash0` and computes each definition's own
  (self-referential-safe) hash, (3) hashes the sorted list of those per-definition hashes into one
  `cycleHash`, then (4) sets each definition's **final** hash to `hashBinary (per-def hash, cycleHash)`
  — combining a definition's own hash with the whole cycle's hash. The code's own comment explains why
  step 4 is necessary rather than just using the per-def hash from step 2: *"it's possible to have two
  predicates with the same name (different versions) in the cycle, and we definitely want them to end
  up with different hashes (unless they have identical representations)"* — i.e., without folding in
  the cycle hash, two structurally-similar-but-distinct members of the same reference cycle could
  collide.
- **A documented, deliberate identity-stability carve-out for one specific backward-compatibility
  case** (`ComputeIds.hs` lines 276-287+, "Note [overriding default deriving]", quoted): adding a
  `deriving P default` declaration to an *already-shipped* predicate `P` must **not** change `P`'s
  `PredicateId` (so existing stored facts and queries against the old ID keep working) — solved by
  excluding "default" deriving info from what `fingerprintDef` hashes, specifically and only for that
  one case, rather than a general rule that any metadata may be excluded from identity.

## Required invariants

- A definition's hash must never be computed before every definition it transitively references
  already has its own final hash — this is exactly what the topological sort (`stronglyConnComp`) plus
  processing order guarantees; computing hashes in declaration order instead (ignoring dependency
  order) would make a dependency's later hash change silently fail to propagate to definitions that
  reference it.
- Within a reference cycle, every member's final hash must depend on the cycle's combined hash, not
  only its own local content — the donor's own comment (quoted above) identifies the exact collision
  this invariant prevents.
- An identity-stability carve-out (like the `deriving default` case) must be scoped to the exact
  documented compatibility case it exists for, not generalized silently — the donor's comment frames
  it explicitly as a "hack . . . for now," which is itself useful signal: this is a known, bounded wart
  in the donor's own design, not a principle to imitate broadly.

## Identity/scope model

Directly relevant to any future Atlas ADL/schema versioning work — no equivalent mechanism exists in
Atlas today (confirmed by search: no `SchemaId`/`TypeRef`/schema-hash concept anywhere in `core/src` or
`adapter/src`). This is a different identity axis than anything the source-intelligence lane's other
genome records cover so far: BLAKE3 (`blake3-content-addressing.md`) content-addresses raw bytes; SCIP
and Kythe's records are about *symbol*/*occurrence* identity within one already-fixed schema. Glean's
mechanism is about giving the **schema itself** (its types and predicates) a stable, structural,
dependency-aware identity — directly relevant if Atlas's ADL or `corpus.atlas`/`.atlas` binary format
ever needs to answer "did this type/relation's meaning change between two Atlas versions, transitively,
including everything it depends on?" without a human maintaining a manual version-bump discipline.

## State/effect/resource model

`computeIds` is a pure, whole-schema batch computation (`[ResolvedSchemaRef] -> HashedSchema`) run once
per schema compilation, threading a `RefToIdEnv` accumulator through a `State` monad as it processes
definitions in dependency order — not an incremental, per-definition operation. Worth noting as a
scope boundary: this mechanism assumes the *entire* schema is available up front to build the
dependency graph and topological order; it does not describe how to incrementally re-hash a schema
after a small edit without recomputing the whole graph.

## Failure and recovery behavior

Not directly evaluated — `computeIds` operates on an already-resolved, presumably-already-validated
schema (`ResolvedSchemaRef`); malformed/unresolvable references are handled earlier in Glean's pipeline
(`resolveDef`'s own `lookupPredicateId`/`lookupTypeId` helpers in the surrounding file call `error` on
a missing ref, i.e. a partial function, not a typed error — flagged here as a property of the donor's
own implementation, not something to imitate; any Atlas-native reimplementation should make an
unresolvable schema reference a typed, recoverable error rather than a crash).

## Concurrency/temporal behavior

Not evaluated — single-threaded, whole-schema batch computation with no concurrency dimension
identified in the read portions of this mechanism.

## Performance characteristics

Not benchmarked. The topological-sort-then-linear-pass structure is asymptotically proportional to the
size of the schema's reference graph (standard SCC-decomposition complexity), which is a property of
the algorithm's shape, not a measured number — recorded because it is the natural complexity class a
native reimplementation should expect and be tested against, not because it was independently verified
here.

## Portability/ABI constraints

Haskell-specific implementation (`Data.Graph.stronglyConnComp`, Haskell's `Hashable`/`Binary` type
classes) with no bearing on a Rust port beyond the algorithm's own logical shape; Rust has no built-in
SCC decomposition in `std`, so a native implementation would need either a small hand-rolled
Tarjan's-algorithm pass or a graph crate — not evaluated further in this pass (out of scope; this
record is architecture study, not an implementation plan).

## Evidence references

- `.atlas/census/donors/source-intelligence-lane-2026-09-20.md` (existing deep census, Glean section,
  lines 214-256 — prior evidentiary source; did not itself surface the hashing mechanism this record
  adds)
- `.atlas/temporary/donors/glean/glean/angle/Glean/Angle/Types.hs` (1216 lines total; this record
  directly reads lines 566-652 for `PredicateId`/`TypeId`/`TypeRef` type-alias definitions and the
  `DeriveWhen` schema-migration enum at lines 607-629)
- `.atlas/temporary/donors/glean/glean/db/Glean/Database/Schema/ComputeIds.hs` (this record directly
  reads lines 1-300, covering `computeIds`, `fingerprintDef`, and the cycle-handling branch and its
  own explanatory comments, including "Note [overriding default deriving]")
- Confirmed via `grep` that no `SchemaId`/`TypeRef`/schema-hash concept exists anywhere in `core/src`
  or `adapter/src` today — this record captures genome ahead of any consumer, consistent with this
  session's established pattern for the source-intelligence lane.

## Donor revisions/licenses

facebookincubator/Glean, commit `2a48dea4cddb316d3b8bd65b54965cb9a7855c52`. License not re-verified in
this pass beyond the existing census/provenance record (`.atlas/provenance/donors/glean.json`); no
license claim is made or changed by this record. (`ComputeIds.hs`'s own header states BSD-style,
consistent with the existing provenance record's license mapping.)

## Known trade-offs

- Transitive, dependency-ordered hashing gives automatic propagation (a change anywhere in a type's
  dependency closure changes its identity with no manual version-bump discipline required) at the cost
  of needing the whole schema's reference graph up front and a topological pass before any single
  definition's identity is known — not an incremental, single-definition operation.
- The cycle-hash-folding step (combining a definition's own hash with its whole cycle's hash) adds one
  extra hashing pass specifically for mutually-recursive schemas, a case that is rare in most schemas
  but, per the donor's own comment, was judged necessary to avoid a real, concrete collision risk
  rather than a hypothetical one.
- The `deriving default` carve-out is explicitly labeled by its own author as "a hack . . . for now" —
  a documented example of a narrow, deliberate identity-stability exception being preferable to either
  (a) no exception (breaking every existing DB on a purely-additive schema change) or (b) a general,
  underspecified "exclude some metadata from identity" rule that would be harder to reason about.

## Rejected alternatives (for this pass)

- Glean's full Angle query language, its typechecking/compilation pipeline, and its RocksDB/LMDB
  storage backends — explicitly out of scope; the existing census already scoped this lane's Glean
  study to principle-extraction, and this record narrows further to the identity-hashing mechanism
  specifically, which the existing census had not yet covered at all.
- Adopting Glean's exact `Hash`/`hashBinary` implementation or the Haskell `Data.Graph` SCC algorithm as
  a dependency — not applicable; this is architecture study for a Rust-native mechanism, and unlike
  BLAKE3 there is no cryptographic-property argument for taking a specific hash implementation as a
  dependency here (the mechanism's value is the transitive/cycle-aware *hashing discipline*, not any
  particular hash function — Atlas's own `stable_id`/`BLAKE3` content-addressing choices, already
  genome-captured, would supply the actual hash primitive if this mechanism were ever implemented).
- Designing Atlas's own ADL schema-identity scheme now — explicitly rejected for this pass: no ADL
  schema-versioning work is underway, and this record's job is to make the mechanism available as
  evidence, not to commit Atlas to adopting it.

## Dependency/extinction status

`REFERENCE_ONLY` per the lane census's own decision (line 334, covering all six source-intelligence
lane donors), unchanged by this record. No runtime/build dependency exists today.

## Decision

**`ABSORB_LATER`**, native-implementation-only: this is a real, well-evidenced, and — per the search
confirming no equivalent exists in Atlas today — genuinely novel mechanism relative to everything else
genome-captured this session, but there is no current ADL/schema-versioning consumer to absorb it into.
Recorded now so that if/when Atlas's ADL or a future `.atlas`/`.atlasx` schema needs a "did this type's
meaning change, transitively, across a compilation" answer, the design does not have to rediscover
"hash transitively in dependency order, fold in a combined cycle hash for mutually-recursive
definitions, and reserve identity-stability carve-outs to specific, documented compatibility cases
only" from scratch. The one carve-out mechanism (excluding specific default-deriving-style metadata
from identity) should be treated as a narrow precedent for "if Atlas ever needs an identity-stability
exception, scope it to one documented case," not a general license to exclude arbitrary fields from a
future Atlas schema-identity hash.

Not `ABSORB_NOW`: no ADL/schema-hashing consumer exists yet, and building one now would be speculative
generality with no concrete requirement driving it — consistent with every other `ABSORB_LATER`
decision reached this session.

`glean`'s `census_status` in `donor-corpus.toml` remains `DEEP_CENSUSED` (this record adds the
transitive schema-identity-hashing mechanism, found by reading past what the existing census already
covered; the Angle query language, typechecking pipeline, and storage backends remain census-observed
but not separately genome-captured), with this genome record added as new evidence.
