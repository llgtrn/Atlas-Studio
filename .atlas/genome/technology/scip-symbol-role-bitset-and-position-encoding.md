---
id: atlas.genome.technology.scip-symbol-role-bitset-and-position-encoding
type: technology-genome
status: active
canonical: true
---
# Technology Genome: combinable symbol-role bitset and explicit position encoding (SCIP)

Donor: SCIP (`sourcegraph/scip`, commit `4f50fbbd0ed945405cd8a4196af6477ea2a9f8b5`), source-intelligence
lane, Wave 0/1.

This record follows the mandatory schema in `.atlas/contracts/DONOR-TO-LANGUAGE-GENESIS.md`. It
restructures and sharpens the existing deep census
(`.atlas/census/donors/source-intelligence-lane-2026-09-20.md`'s SCIP section, lines 123-166) into
the canonical genome shape, verified directly against the donor's own `scip.proto` (read in full for
the `Occurrence`, `SymbolRole`, `Document`, `PositionEncoding`, `Symbol`, and `Descriptor` messages)
rather than relying on the census summary alone. Unlike this session's earlier binary-format genome
cluster (BLAKE3/rkyv/FlatBuffers/Cap'n Proto), this donor sits on the source-intelligence lane's own
stated critical path: its synthesis section (`source-intelligence-lane-2026-09-20.md` lines 317-322)
names SCIP-like symbol-occurrence exchange import as the second immediate implementation target,
directly after the source-ingestion work the same document already confirms is substantially real in
Atlas today.

## Capability / problem

Given source code already parsed into symbols and occurrences by some upstream indexer, exchange that
information in a form that (1) lets one occurrence carry more than one simultaneous fact about itself
(a reference can be a write access, inside generated code, and inside test code, all at once), and (2)
lets a byte-offset-based range be interpreted unambiguously by a consumer written in a different
language than the producer, without either side needing to renegotiate encoding out of band.

## Semantic mechanism (as observed in the donor, evidence already gathered in this pass)

- **`SymbolRole` is a bitset, not a discriminated enum** (`scip.proto` lines 524-546, read directly):
  `Definition = 0x1`, `Import = 0x2`, `WriteAccess = 0x4`, `ReadAccess = 0x8`, `Generated = 0x10`,
  `Test = 0x20`, `ForwardDefinition = 0x40`. The proto's own comment (lines 521-523) gives the
  canonical read pattern: `const isImportRole = (role.value & SymbolRole.Import.value) > 0`.
  `Occurrence.symbol_roles` (line 728) is a single `int32` that can carry any combination of these —
  a write access to a symbol inside generated test code is one value, not three separate occurrence
  records.
- **Position encoding is an explicit, declared field on every `Document`, not an implicit assumption**
  (`Document.position_encoding`, line 118, of type `PositionEncoding`, lines 122-146): the proto
  documents, with a worked emoji example for each of the three variants, that a `character` offset
  means a different byte/code-unit count depending on whether the indexer is UTF-8-natured (Go, Rust,
  C++), UTF-16-natured (JVM, .NET, JavaScript/TypeScript), or UTF-32-natured (Python) — and requires
  every document to say which one it used rather than leaving it to producer/consumer convention.
- **A quantified, donor-documented format-evolution lesson on range encoding** (`Occurrence` message,
  lines 692-722, read directly): the original schema used a typed `Range{start, end Position}`
  message mirroring LSP; benchmarking showed switching to a flat `repeated int32` cut total index
  payload size by 50%, but the resulting loss of type safety was significant enough that the donor
  later reintroduced `single_line_range`/`multi_line_range` as a typed `oneof` alternative — at a
  documented "single-digit percent" size cost over the untyped array, because ranges are only a
  fraction of a typical index's total payload. The untyped `repeated int32 range` field is kept only
  as a deprecated fallback for backward wire compatibility.
- **Symbol identity is a structured, descriptor-chain string grammar**, not an opaque ID
  (`Symbol`/`Package`/`Descriptor` messages, lines 148-230, read directly): `scheme` + `package`
  (manager/name/version) + a repeated chain of typed `Descriptor`s (`Namespace`, `Type`, `Term`,
  `Method`, `TypeParameter`, `Parameter`, `Meta`, `Local`, `Macro`), each carrying its own
  disambiguator. `Local` descriptors are explicitly scoped to a single document and must never be
  treated as cross-document identity — the donor's own grammar comment (line 188) states this as a
  hard rule, not a convention.

## Required invariants

- Role membership must be tested by bitwise-AND against a specific role's value, never by equality
  against the whole `symbol_roles` field — an occurrence with multiple roles set will never equal any
  single role constant, so an equality check silently under-reports every multi-role occurrence.
- A range's `character` offset must always be interpreted using the `PositionEncoding` declared on its
  enclosing `Document`, never a hardcoded assumption — the donor's own worked examples show the same
  numeric offset resolves to different characters under UTF-8 vs UTF-16 vs UTF-32 interpretation for
  any string containing a character outside the ASCII/BMP range.
- A `Local`-scoped symbol descriptor must never be compared for identity across documents; only
  non-local (package-qualified) symbols carry cross-document identity.

## Identity/scope model

Directly comparable to Atlas's own `core::semantic::symbol::SymbolIdentity`/`SymbolRole`
(`core/src/semantic/symbol.rs`, read in full this pass): Atlas's `SymbolRole` is a four-variant
mutually-exclusive `enum` (`Definition | Declaration | Reference | Unresolved`), and `SymbolIdentity`
carries exactly one `role` field. This is a genuine, concrete representational gap relative to SCIP's
model: Atlas today has no way to represent "this occurrence is simultaneously a write access, in
generated code, and in test code" — only one of {Definition, Declaration, Reference, Unresolved} can
be recorded per identity. Whether this gap needs closing depends on whether a future
`SymbolOccurrence` type (distinct from today's `SymbolIdentity`, per the lane synthesis's own item 5,
`source-intelligence-lane-2026-09-20.md` line 310) adopts a combinable role set; `SymbolIdentity`
itself, which is about symbol *identity* (name/scope/repository/revision), is a different concept from
per-occurrence role classification and this record does not propose changing it.

## State/effect/resource model

Pure data-exchange schema; no execution semantics. Reading a SCIP index does not require running the
indexer that produced it (which is exactly why the lane census's own REJECT items for every lane
donor forbid executing donor tooling during ingestion — SCIP is the one lane donor whose *entire*
mechanism is designed around not needing to).

## Failure and recovery behavior

Not the focus of this mechanism; SCIP does not specify a validation/untrusted-input discipline of its
own (contrast with the Cap'n Proto genome record's traversal-limit/depth-limit mechanism) — an
`adapter/exchange/source_index` importer would need to supply its own bounds/sanity checking over
SCIP-shaped input, consistent with this session's established `untrusted_input_default_deny` posture,
since nothing in `scip.proto` itself provides it.

## Concurrency/temporal behavior

Not evaluated — `Index`/`Document`/`Occurrence` are static, already-produced records with no
concurrency dimension of their own; `Metadata` documents that it must appear first in a streamed
`Index` specifically so a streaming consumer can act on project-level context before per-document
records arrive, which is a streaming-protocol property rather than a concurrency one.

## Performance characteristics

The 50%-payload-size-versus-type-safety trade-off (see Semantic mechanism above) is the one
donor-quantified performance data point in this record, and it is a real, shipped format's own
retrospective, not a synthetic benchmark — directly useful precedent if a future Atlas exchange-import
or corpus format faces the same typed-range-vs-compact-array choice.

## Portability/ABI constraints

`scip.proto` is deliberately protocol-buffer-based specifically for multi-language portability
(`bindings/go`, `bindings/rust`, `bindings/typescript` all exist per the census's observed topology);
this is a property of the *exchange* boundary, not of Atlas's own internal representation — consistent
with the lane census's own REWRITE item ("Atlas's canonical source graph must not be SCIP protobuf.
SCIP is an import/export adapter and donor principle").

## Evidence references

- `.atlas/census/donors/source-intelligence-lane-2026-09-20.md` (existing deep census, SCIP section,
  lines 123-166 — primary prior evidentiary source)
- `.atlas/temporary/donors/scip/scip.proto` (846 lines; this record directly reads lines 26-118
  (`Index`/`Metadata`/`Document`), 122-146 (`PositionEncoding`), 148-230 (`Symbol`/`Package`/
  `Descriptor`), and 692-810 (`Occurrence`/`SyntaxKind`/`Diagnostic`) rather than relying on the
  census summary alone)
- `core/src/semantic/symbol.rs` (116 lines, read in full — Atlas's own current `SymbolRole`/
  `SymbolIdentity`, the direct comparison point for this record's Identity/scope model section)
- Confirmed via `grep`/`find` that no `adapter/exchange`, `adapter/source_index`, or SCIP-referencing
  module exists yet in `adapter/src` or `core/src` — this record captures genome ahead of the
  consumer, per the lane synthesis's own sequencing, not after the fact.

## Donor revisions/licenses

sourcegraph/scip, commit `4f50fbbd0ed945405cd8a4196af6477ea2a9f8b5`. License not re-verified in this
pass beyond the existing census/provenance record (`.atlas/provenance/donors/scip.json`); no license
claim is made or changed by this record.

## Known trade-offs

- A combinable role bitset (SCIP) versus a mutually-exclusive role enum (Atlas today) trades
  representational richness for match-exhaustiveness simplicity: a `match` over Atlas's `SymbolRole`
  is trivially exhaustive and cannot silently ignore a role Atlas doesn't handle; a bitset requires
  every consumer to remember to test with bitwise-AND rather than equality, and untested role
  combinations are easy to under-represent (as the invariant above states directly).
- Explicit per-document position encoding costs one more field every producer must set correctly and
  every consumer must read before interpreting any range — donor-justified by the worked examples
  showing silent misinterpretation is otherwise possible for any non-ASCII source file.
- The 50%-size/type-safety range-encoding trade-off (see Semantic mechanism) is itself evidence that
  neither extreme (fully typed, fully packed) was judged acceptable by the donor's own maintainers —
  the shipped answer is a typed `oneof` over two typed range messages, not a return to the untyped
  array nor a refusal to ever pack.

## Rejected alternatives (for this pass)

- Adopting `scip.proto`/the generated bindings as an actual Atlas dependency — rejected, unchanged
  from the existing census's own REWRITE conclusion: SCIP is exchange-boundary precedent, not a
  canonical Atlas type source. A future `adapter/exchange/source_index` reads SCIP-*shaped* input into
  Atlas-native typed facts; it does not re-export SCIP's own protobuf types as Atlas's model.
  Unlike BLAKE3, there is no cryptographic-property argument for a direct dependency here either — a
  bitset role field and an encoding-tagged range are both ordinary, independently-testable
  representational choices.
- Copying SCIP's exact symbol string grammar verbatim as Atlas's own symbol-ID format — not evaluated
  in depth in this pass; the existing census already flags "SCIP string grammar" as something Atlas
  symbol IDs should be "not only" (line 161), leaving the exact shape of a future Atlas symbol-ID
  scheme as an open design question this record does not resolve.
- Redesigning `core::semantic::symbol::SymbolRole` into a bitset now — explicitly rejected for this
  pass: no `SymbolOccurrence`/exchange-import consumer exists yet to justify the change, and
  `SymbolIdentity`'s current single-role field is used correctly today for what it actually models
  (identity of one declaration/reference site, not multi-fact classification of an occurrence). See
  Decision.

## Dependency/extinction status

`REFERENCE_ONLY` per the lane census's own decision (line 334: "Runtime status remains
`REFERENCE_ONLY`; principles are approved for Atlas-native reimplementation"), unchanged by this
record. No runtime/build dependency exists today; no `adapter/exchange` module exists yet to depend on
anything.

## Decision

**`ABSORB_LATER`**, native-implementation-only, but flagged as the closest-to-triggering item in this
session's genome work so far: the lane synthesis's own "Immediate implementation target" sequencing
(`source-intelligence-lane-2026-09-20.md` lines 317-322) names symbol-occurrence exchange import as
step 2, directly after step 1 (source ingestion), which the same document already confirms is
substantially real in Atlas today. This is a materially shorter distance to a real consumer than the
binary-format genome cluster (BLAKE3/rkyv/FlatBuffers/Cap'n Proto), whose trigger is the entirely
unbuilt `.atlas`/`.atlasx` format work.

Concretely, when `SymbolOccurrence`/`adapter/exchange/source_index` work begins, this record's
practical output is: (1) give occurrence-level role its own type distinct from
`core::semantic::symbol::SymbolRole` — a `SymbolOccurrenceRoles` bitset (Rust `bitflags`-style, no
crate dependency needed for something this small) rather than overloading or replacing
`SymbolIdentity`'s existing single-role field, since the two model genuinely different things
(declaration-site identity vs. per-occurrence fact classification); (2) require every ingested source
unit to carry an explicit, typed position-encoding tag before any range on it is interpreted, mirrored
directly from `PositionEncoding`; (3) when designing how ranges are stored, treat the donor's own
50%-size/type-safety trade-off as precedent for preferring a typed representation unless a concrete,
measured size problem justifies revisiting it — not the reverse.

Not `ABSORB_NOW`: no `adapter/exchange` or symbol-occurrence-import module exists yet (confirmed by
direct search), and `core::semantic::symbol::SymbolRole` is used correctly today for its actual,
narrower purpose — changing it now, with no real consumer needing the richer model, would be
speculative generality this session's own engineering-quality standard (`.atlas/contracts/
DONOR-TO-LANGUAGE-GENESIS.md` anti-patterns; also this repository's general no-premature-abstraction
convention) forbids.

`scip`'s `census_status` in `donor-corpus.toml` remains `DEEP_CENSUSED` (this record covers the
role-bitset and position-encoding mechanisms specifically; the full symbol string grammar and the
`Relationship`/`SymbolInformation`/documentation fields remain census-observed but not separately
genome-captured), with this genome record added as new evidence.
