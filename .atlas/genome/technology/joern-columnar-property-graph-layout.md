---
id: atlas.genome.technology.joern-columnar-property-graph-layout
type: technology-genome
status: active
canonical: true
---
# Technology Genome: columnar property-graph storage for large-scale traversal (Joern/flatgraph)

Donor: Joern (`joernio/joern`, commit `b381922638ae436fcb6862a90241c8f7ce508894`), source-intelligence
lane, Wave 0/1.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
restructures and sharpens the existing deep census
(`.atlas/census/donors/source-intelligence-lane-2026-09-20.md`'s Joern section, lines 258-301, which
already names the flatgraph migration and cites its changelog) by reading the changelog directly in
full (`changelog/4.0.0-flatgraph.md`, 44 lines) and comparing its quantified numbers against Atlas's
own current graph representation (`core/src/graph/mod.rs`, read fresh this pass), which the existing
census had not done.

## Capability / problem

Given a code property graph (CPG) large enough that a per-node/per-edge heterogeneous property-bag
representation becomes a real memory and traversal-speed bottleneck (the donor's own numbers: 48M
nodes with 630M properties, 431M edges with 115M properties, for one large real-world codebase), store
and traverse it in a way that reduces both memory footprint and traversal latency at that scale,
without changing what the graph model can express for an ordinary user of it.

## Semantic mechanism (as observed in the donor, evidence read directly in this pass)

- **Row-oriented to columnar migration, quantified with real numbers** (`changelog/4.0.0-flatgraph.md`,
  read in full): Joern replaced its prior graph backend (overflowdb, a JVM object-graph-style store)
  with flatgraph, whose central idea is holding node/edge data in "few (albeit very large) arrays" —
  i.e. columnar layout, one array per property/kind rather than one heterogeneous object per node. The
  donor's own measured results for one large codebase (Linux kernel-scale, ~48M nodes / 431M edges):
  heap after import 33g → 20g (~40% reduction), minimum heap required for import 80g → 30g, import time
  18 minutes → 11 minutes, on-disk file size 2600M → 400M (~85% reduction). These are the donor's own
  reported, workstation-measured numbers, not independently reproduced here — recorded as donor claim,
  not verified benchmark.
- **A deliberately abandoned feature is itself evidence**: overflowdb's disk-overflow mechanism
  (handling graphs larger than available memory by spilling to disk) was *not* reimplemented in
  flatgraph, with the donor's own stated reason quoted directly: "it sounds nice ... but in practice
  it was too slow to be useful." A donor explicitly declining to carry forward one of its own prior
  features, with a concrete stated reason, is stronger evidence than a feature that was simply never
  built.
- **The columnar layout imposes a real, donor-documented schema constraint**: "Edges can only have
  zero or one properties" in flatgraph, versus overflowdb's prior unrestricted per-edge property count
  — the donor notes this was already true of every edge type in their own schema in practice, so the
  migration cost them nothing, but the constraint is real and would bind any schema that needed
  multi-property edges.
- **CPG as one unified projection over several traditionally-separate source views** (existing
  census's own framing, corroborated): AST, CFG, call graph, and data-flow are all represented as one
  graph with different edge kinds, rather than as separate, independently-maintained structures —
  independent confirmation of a "different views over one shared fact substrate" pattern this lane's
  synthesis already names as its own goal (`CodeGraphProjection`, item 9,
  `source-intelligence-lane-2026-09-20.md` line 314).

## Required invariants

- A columnar/array-of-properties layout requires a stable, dense index space for nodes/edges (an
  array position *is* part of the identity of the access path) — this is an implementation-shape
  invariant the donor's migration notes don't spell out explicitly but is inherent to "hold everything
  in a few large arrays": unlike a `BTreeMap<String, String>` per node, adding a genuinely new property
  to a columnar store is a schema-level operation (a new array), not a per-instance one.
- Any schema built on a flatgraph-style store must respect its structural constraints (e.g. zero-or-one
  properties per edge) at the schema-design stage, not discover a violation at query time.

## Identity/scope model

Not primarily an identity mechanism (contrast with the SCIP/Kythe/Glean genome records from this same
lane); flatgraph's node/edge identity is unchanged by the migration per the changelog's own framing —
this record is entirely about *storage layout*, not identity.

## State/effect/resource model

Directly comparable to Atlas's own current `EngineeringGraph` (`core/src/graph/mod.rs`, read fresh in
this pass): `pub struct Node { attributes: BTreeMap<String, String>, ... }`,
`pub struct Edge { attributes: BTreeMap<String, String>, ... }`, with `EngineeringGraph { nodes:
Vec<Node>, edges: Vec<Edge>, ... }`. This is structurally the row-oriented, per-instance
heterogeneous-property-bag shape flatgraph's migration moved *away from* — a `BTreeMap` per node is,
architecturally, the same shape class as overflowdb's per-node object, not flatgraph's columnar arrays.
This is not a defect: Atlas's current graph is nowhere near the 48M-node/630M-property scale the
donor's numbers describe, and `BTreeMap<String, String>`'s heterogeneity is exactly what lets
`EngineeringGraph` stay schema-flexible while the R4 semantic model itself is still evolving. Recorded
as a concrete, quantified precedent for *when this trade-off would need revisiting* (real, large-graph
memory/traversal pressure), not as a current problem.

## Failure and recovery behavior

Not evaluated — out of scope for this specific mechanism; the changelog covers a storage-layout
migration, not error handling.

## Concurrency/temporal behavior

Not evaluated in this pass — not covered by the changelog excerpt read.

## Performance characteristics

The donor's own quantified numbers are the core evidence of this record (see Semantic mechanism above)
— all donor-reported, workstation-measured, not independently reproduced. Recorded precisely with their
scale (48M nodes, a single large real-world codebase) so a future comparison against Atlas's own graph
size is apples-to-apples rather than a vague "columnar is faster" claim.

## Portability/ABI constraints

JVM-specific implementation detail (arrays on the JVM heap); the *idea* (columnar layout for a
large property graph) is language-independent and has well-known analogues outside the JVM (Apache
Arrow's columnar layout, already genome-relevant to this repository's own donor set per
`.atlas/census/donors/arrow.md`, though not cross-referenced in depth here — flagged as a natural future
cross-donor comparison, not performed in this pass).

## Evidence references

- `.atlas/census/donors/source-intelligence-lane-2026-09-20.md` (existing deep census, Joern section,
  lines 258-301 — prior evidentiary source, already named the flatgraph migration at a high level)
- `.atlas/temporary/donors/joern/changelog/4.0.0-flatgraph.md` (44 lines, read in full — this record's
  primary new evidence: the quantified memory/time/disk numbers, the abandoned-disk-overflow decision,
  and the "zero or one properties per edge" constraint)
- `core/src/graph/mod.rs` (68 lines, read fresh this pass — `Node`/`Edge`/`EngineeringGraph`'s current
  `BTreeMap`-per-instance shape, the direct comparison point for this record's State/effect/resource
  model section)

## Donor revisions/licenses

joernio/joern, commit `b381922638ae436fcb6862a90241c8f7ce508894`. License not re-verified in this pass
beyond the existing census/provenance record (`.atlas/provenance/donors/joern.json`); no license claim
is made or changed by this record.

## Known trade-offs

- Columnar layout trades per-instance schema flexibility (add any property to any node ad hoc, as
  `BTreeMap<String, String>` allows today) for memory density and traversal speed at scale — the donor's
  own "zero or one properties per edge" constraint is the direct, concrete cost of that trade for their
  schema.
- Abandoning disk-overflow support trades "can technically exceed available memory" for "is simple and
  fast within memory" — the donor's own stated reason (the feature was "too slow to be useful" in
  practice) is evidence this specific trade-off was validated against real use, not assumed.

## Rejected alternatives (for this pass)

- Adopting flatgraph or overflowdb as an actual Atlas dependency — rejected; both are JVM libraries,
  and even setting portability aside, no cryptographic-property argument applies here (this is an
  ordinary, independently-testable storage-layout technique, same category as the FlatBuffers/rkyv
  genome records' "mechanism before syntax, native reimplementation" conclusion).
- Redesigning `EngineeringGraph`/`Node`/`Edge` toward a columnar layout now — explicitly rejected for
  this pass: Atlas's graph is far below the scale where the donor's own numbers show this mattering,
  and doing so now would be speculative generality with no concrete performance problem driving it.
- Joern's full CPG generator/language-frontend architecture and its query-pack (`querydb`) ecosystem —
  out of scope; the existing census already scoped this to principle-extraction and explicitly rejected
  embedding Joern's Scala runtime or CPG generators, which this record does not revisit.

## Dependency/extinction status

`REFERENCE_ONLY` per the lane census's own decision (line 334), unchanged by this record. No runtime/
build dependency exists today.

## Decision

**`ABSORB_LATER`**, and narrower than a typical "native reimplementation, no dependency" verdict: this
record's practical output is not an implementation technique to schedule but a **quantified trigger
condition** for when Atlas's own graph storage should be reconsidered. Concretely: if/when
`EngineeringGraph` (or a future `.atlas`/`.atlasx`-backed graph) approaches a scale where per-node
`BTreeMap<String, String>` allocation and traversal become measurably expensive — the donor's own
numbers (tens of millions of nodes, hundreds of millions of properties) are the right order-of-magnitude
reference point, not a specific Atlas number this record invents — that is the trigger to revisit
columnar layout as a design option, informed by this record's evidence (real measured savings, the
disk-overflow feature's own donors abandoned as not worth it, and the "zero or one properties per edge"
constraint a columnar design would need to either accept or specifically design around).

Not `ABSORB_NOW`: Atlas's current graph is far below the scale where this trade-off has been shown to
matter, and no performance problem exists today to justify the redesign cost.

`joern`'s `census_status` in `donor-corpus.toml` remains `DEEP_CENSUSED` (this record adds the
flatgraph columnar-storage mechanism with quantified numbers; the CPG generator architecture, data-flow
engine, and query-pack ecosystem remain census-observed but not separately genome-captured), with this
genome record added as new evidence.
