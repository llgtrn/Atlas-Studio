---
id: atlas.genome.technology.kythe-vname-identity-and-entry-unification
type: technology-genome
status: active
canonical: true
---
# Technology Genome: VName identity scoping and fact/edge storage unification (Kythe)

Donor: Kythe (`kythe/kythe`, commit `69141f022689a611e8a4a1d9b08a3783a2e8a9ed`), source-intelligence
lane, Wave 0/1.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
restructures and sharpens the existing deep census
(`.atlas/census/donors/source-intelligence-lane-2026-09-20.md`'s Kythe section, lines 168-212) into
the canonical genome shape, verified directly against `kythe/proto/storage.proto` (`VName`, `Entry`,
`VNameMask` messages read in full) rather than the census summary alone. The lane synthesis names
Kythe's VName separation as direct design input to `core/identity`'s `SourceIdentity`/`SymbolIdentity`
work (`source-intelligence-lane-2026-09-20.md` line 201), so this record includes a direct comparison
against Atlas's own current `core::identity` module, read fresh for this pass.

## Capability / problem

Given a graph fact or edge produced by static analysis, name the node(s) it is about in a way that
(1) is stable and comparable across corpora/projects without collision, (2) does not conflate identity
with time (so the same logical entity observed at two different revisions can still be recognized as
"the same thing" when that is what a consumer needs), and (3) stores both node-attached facts and
edges-between-nodes through one uniform wire representation rather than two incompatible ones.

## Semantic mechanism (as observed in the donor, evidence already gathered in this pass)

- **`VName` is a five-field structured identity, not an opaque string**
  (`kythe/proto/storage.proto` lines 38-91, read directly): `signature` (analyzer-assigned,
  language-specific), `corpus` (e.g. `"github.com/creachadair/imath"` — the label `"kythe"` is
  explicitly reserved), `root` (a corpus-specific sub-collection label, e.g. for distinguishing
  generated-file subgroups or subprojects within one corpus), `path` (corpus/root-relative, explicitly
  required to be normalized with no `""`, `"."`, or `".."` elements, and explicitly allowed to be an
  ad-hoc abstract ID for non-file "figments" rather than a real path), and `language`. `VNameMask`
  (lines 93-99) lets a caller select a subset of these five fields for partial-match queries.
- **Identity deliberately excludes revision/timestamp, by explicit donor design rationale, quoted
  directly** (lines 85-90): *"We have intentionally NOT included a revision or timestamp here. Time
  should be recorded as facts belonging to the appropriate Nodes and Edges... a name should say 'what'
  something is, not 'when'."* This is the single most load-bearing design principle in this record —
  see Identity/scope model below for how it compares against Atlas's own canonical, documented choice.
- **One `Entry` message unifies node-facts and edges** (lines 101-119, read directly): `Entry` has
  `source: VName`, `edge_kind: string`, `target: VName`, `fact_name: string`, `fact_value: bytes`. The
  proto states as an explicit invariant that `edge_kind`/`target` "must either be both empty, or both
  nonempty" — when both are empty, the `Entry` is a fact directly attached to `source` (e.g.
  `fact_name = "/kythe/node/kind"`); when both are set, the same message shape represents an edge from
  `source` to `target` of kind `edge_kind`. One wire type, one storage/query path, for two conceptually
  different graph primitives.
- **`fact_name` has its own restrictive grammar** (lines 110-116): `name = "/" | 1*path`, each path
  segment a `"/"`-prefixed run of letters/digits/`[-.@#$%&_+:()]` — a closed character class, not
  arbitrary text, which is what makes `fact_prefix`-based scanning (`ScanRequest.fact_prefix`, line
  167) a safe, well-defined query primitive rather than an ad-hoc string-matching convention.

## Required invariants

- `Entry.edge_kind` and `Entry.target` must be either both empty (a fact) or both nonempty (an edge) —
  never one without the other; a producer or consumer that allows this to diverge silently corrupts the
  fact/edge distinction the entire storage model depends on.
- `VName.path`, when present, must already be normalized (no `""`/`"."`/`".."` segments) before being
  used as an identity component — the proto pushes this normalization requirement onto the producer,
  not the identity type itself.
- `fact_name` must conform to its declared grammar for `fact_prefix` scan queries to remain
  well-defined; an unconstrained fact-name string would make prefix-based fact discovery ambiguous.

## Identity/scope model

Directly comparable to Atlas's own `core::identity` (`core/src/identity/mod.rs`, read in full this
pass): today `RepositoryId`, `RevisionId`, `NodeId`, `SymbolId`, etc. are single opaque `String`
newtypes (the `typed_id!` macro), not structured five-field records like `VName`. More significant is
a direct, evidenced **tension** this comparison surfaces rather than resolves: Kythe's explicit
design principle is that identity must exclude revision ("a name should say 'what', not 'when'"), but
Atlas's own canonical contract, `.atlas/contracts/NORMALIZATION.md#identity` (line 48, read directly),
states the opposite as a deliberate requirement: *"A normalized identity is scoped by enough
information to avoid accidental aliasing: repository, **revision**, namespace/module/type/function
scope, declaration identity... or content fingerprint as applicable."* `core::semantic::symbol::
SymbolIdentity::identity_key()` (`core/src/semantic/symbol.rs`, read in full this pass) concretely
implements this by hashing `revision.kind`/`revision.value` directly into the identity string. This is
not a defect to fix — `NORMALIZATION.md`'s own `Deduplication`/`Equivalence` sections require this so
that the same declaration observed at two different revisions can be tracked as two distinct
provenance-bearing records rather than silently collapsing into one, which is exactly the kind of
epistemic-preservation guarantee this repository's own canonical contracts require elsewhere. Recorded
here as a genuine, evidenced cross-donor divergence with a documented rationale on Atlas's side, so a
future generation does not "fix" `SymbolIdentity` toward Kythe's principle without first re-reading
`NORMALIZATION.md` and realizing the fusion is deliberate.

## State/effect/resource model

Pure data model; `VName`/`Entry` describe facts, they do not execute anything. `GraphStore`'s read/
write/scan RPCs (not evaluated in depth in this pass — out of scope, see Rejected alternatives) are
the only stateful surface in `storage.proto`.

## Failure and recovery behavior

Not specified by the donor for `VName`/`Entry` themselves; the invariants above (both-or-neither
edge fields, normalized path, constrained fact-name grammar) are producer-side obligations the proto
documents but cannot itself enforce at the wire level — any Atlas-native reimplementation would need
to enforce them structurally (e.g. an enum distinguishing a `Fact` variant from an `Edge` variant,
making the both-or-neither invariant a type-level guarantee rather than a documented convention two
optional fields could still violate).

## Concurrency/temporal behavior

Not evaluated — `VName`/`Entry` are immutable, already-produced records with no concurrency dimension
of their own; the `GraphStore` RPC layer (`ReadRequest`/`WriteRequest`/`ScanRequest`/`ShardRequest`,
lines 126-186) is explicitly out of scope for this record (see Rejected alternatives).

## Performance characteristics

Not benchmarked in this pass. `ReadRequest`'s own doc comment (line 127-128) states read operations
"should be implemented with time complexity proportional to the size of the return set" — a donor-
stated performance contract for implementors, not a measured number; recorded because it is a concrete,
checkable property a future Atlas-native fact store could hold itself to.

## Portability/ABI constraints

Protobuf-based (`kythe/proto/storage.proto`), consistent with the lane's other exchange-oriented
donors (SCIP); not independently evaluated for cross-language binding maturity in this pass — the
existing census's observed topology notes a Go implementation (`kythe/go/indexer`,
`kythe/go/serving/xrefs`) and a C++ verifier (`kythe/cxx/verifier`), no Rust implementation.

## Evidence references

- `.atlas/census/donors/source-intelligence-lane-2026-09-20.md` (existing deep census, Kythe section,
  lines 168-212 — primary prior evidentiary source)
- `.atlas/temporary/donors/kythe/kythe/proto/storage.proto` (this record directly reads lines 38-99
  (`VName`/`VNameMask`), 101-119 (`Entry`), and 126-186 (`GraphStore` RPC request/reply shapes) rather
  than relying on the census summary alone)
- `core/src/identity/mod.rs` (152 lines, read in full — Atlas's own current typed-ID model, the direct
  comparison point for this record's Identity/scope model section)
- `core/src/semantic/symbol.rs` and `.atlas/contracts/NORMALIZATION.md#identity` (line 48, read
  directly) — the concrete evidence for the identity/revision-fusion divergence this record documents
- `.atlas/genome/technology/scip-symbol-role-bitset-and-position-encoding.md` — sibling
  source-intelligence-lane genome record from this same session; both independently converge on
  "the exchange donor's identity/role model differs from Atlas's own in specific, evidenced ways,"
  reinforcing that comparing against Atlas's real current code (not just the donor) is the load-bearing
  step for this lane's genome work.

## Donor revisions/licenses

kythe/kythe, commit `69141f022689a611e8a4a1d9b08a3783a2e8a9ed`. License not re-verified in this pass
beyond the existing census/provenance record (`.atlas/provenance/donors/kythe.json`); no license claim
is made or changed by this record.

## Known trade-offs

- Excluding revision from identity (Kythe) keeps a name stable across time at the cost of needing a
  separate mechanism (facts on the node) to answer "when did this exist/change" -- Atlas's opposite
  choice (revision fused into identity scope) gets per-revision record distinction for free at
  `identity_key()` call sites, at the cost of the same logical entity across revisions requiring
  explicit cross-revision linking elsewhere when that view actually is wanted (not evaluated here
  whether such linking exists today; flagged as a natural follow-up question, not answered by this
  record).
- Unifying facts and edges into one `Entry` shape (both-or-neither `edge_kind`/`target`) minimizes
  wire/storage surface area at the cost of the invariant being enforced only by convention/documentation
  rather than by the type system — a `oneof`-style or enum-tagged representation would make the
  distinction structurally unbreakable, at the cost of a slightly larger generated type. Recorded as a
  concrete design choice for a future Atlas-native fact/edge representation to consider explicitly
  (prefer the type-safe variant, per this session's own established preference for structural
  invariants over documented conventions — the same preference the Cap'n Proto and FlatBuffers genome
  records independently arrived at for their own mechanisms).

## Rejected alternatives (for this pass)

- The full `GraphStore`/`XRefService`/`GraphService` RPC/serving layer — explicitly out of scope, not
  evaluated; the existing census already flags this lane as principle-extraction, not service-adoption,
  and this record does not revisit that scope boundary.
- Adopting Kythe's exact VName field set (`signature`/`corpus`/`root`/`path`/`language`) verbatim as
  Atlas's `SourceIdentity` shape — not decided here; the lane synthesis's own `SourceIdentity` item
  (line 306) already lists a different, Atlas-specific field set (repository partition, source path,
  content fingerprint, integrity digest, language, revision provenance) that deliberately includes
  revision, consistent with the identity/revision-fusion divergence this record documents rather than
  contradicts. This record's contribution is the *tickets-are-structured-not-opaque* and
  *facts-and-edges-share-one-shape* principles, not the specific five Kythe fields.
- Taking `kythe`'s Go/C++ implementation as a dependency — rejected, unchanged from the existing
  census's own REWRITE conclusion (no Bazel/Kythe extraction as Atlas's ingestion substrate). No
  cryptographic-property argument applies here either.

## Dependency/extinction status

`REFERENCE_ONLY` per the lane census's own decision (line 334), unchanged by this record. No runtime/
build dependency exists today.

## Decision

**`ABSORB_LATER`**, native-implementation-only, and specifically narrower than initially expected: the
most load-bearing finding in this record is not "adopt VName's field set" but the **documented tension
between Kythe's identity/time-separation principle and Atlas's own canonical, deliberate choice to fuse
revision into identity scope** (`NORMALIZATION.md#identity`). That tension is resolved in Atlas's favor
by Atlas's own existing contract, not left open by this record — the practical output is a caution, not
a design mandate: do not "fix" `core::identity`/`SymbolIdentity` toward Kythe's name/time separation
without first checking whether `NORMALIZATION.md`'s deduplication/equivalence requirements (which
depend on revision-scoped identity) would be broken by doing so.

The two mechanisms this record does recommend absorbing when `core/identity`'s `SourceIdentity` design
work actually begins (lane synthesis item 1): (1) a structured, multi-field identity type rather than
a single opaque string, when the identity genuinely has independent axes (Kythe's
corpus/root/path/language is one instance; Atlas's own `RepositoryId`/`RevisionId`/scope/name/role
already form a similar informal tuple today via string concatenation in `identity_key()` — the
donor comparison suggests a first-class structured type is worth considering over string-joining once
that design work starts, not before); (2) if Atlas ever builds a unified fact/edge storage
representation, prefer a type-enforced fact-vs-edge distinction (enum/`oneof`) over Kythe's
documented-but-unenforced both-or-neither convention.

Not `ABSORB_NOW`: no `SourceIdentity` redesign is underway, and the one concrete "fix" this comparison
surfaced (identity/revision fusion) turns out, on checking canonical truth, to already be Atlas's
correct, deliberate, documented choice — exactly the kind of finding `DONOR-TO-LANGUAGE-GENESIS.md`'s
process is meant to produce (a donor comparison that clarifies rather than blindly imports).

`kythe`'s `census_status` in `donor-corpus.toml` remains `DEEP_CENSUSED` (this record covers the VName
identity model and the Entry fact/edge unification specifically; the `GraphService`/`XRefService`
serving layer and the C++ verifier's Souffle-backed assertion support remain census-observed but not
separately genome-captured), with this genome record added as new evidence.
