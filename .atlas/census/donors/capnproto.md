---
id: donor-census-capnproto
type: reference
status: active
canonical: true
---
# Donor Census: Cap'n Proto

## Source

- Remote: https://github.com/capnproto/capnproto.git
- Repository: capnproto/capnproto
- Commit: 7fff7b6482a19bdedb18884d13f66a1a4f5e5650
- Branch: master
- Retrieved: 2026-09-22T15:53:06Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/capnproto

### Provenance note

`capnproto/capnproto` is the canonical, original upstream (maintained by Kenton Varda and Cloudflare; the copyright header names "Sandstorm Development Group, Inc.; Cloudflare, Inc.; and other contributors"). No fork/mirror indicators found.

### Framing (explicit, per absorption brief)

This donor is censused **purely as a binary/canonical-encoding reference** for Atlas's own `.atlas` wire format design: its pointer-based, in-place-traversable message layout; its explicit, load-bearing canonicalization rules (useful for hashing/signing Atlas artifacts); and its schema-evolution discipline. It is explicitly **NOT** a donor for Atlas's semantic ontology/type system, its RPC layer, or its C++ code generator implementation.

## Coarse Inventory

- Files observed (post `.git` removal): dominated by `c++/src/capnp/` and `c++/src/kj/` (the C++ reference implementation and its `kj` support library), `doc/` (Jekyll docs site, the primary source for this census), `doc/samples`, `highlighting/` (editor syntax files, not relevant), `security-advisories/`
- Bytes observed: ~12 MB (`du -sh` = 12M)
- Languages/signals: C++ (majority), `.capnp` schema files (`c++/src/capnp/schema.capnp`, `rpc.capnp`, `stream.capnp`, `persistent.capnp`, benchmark schemas), Markdown (doc site), Ruby (Jekyll config), Python (test/tool scripts)
- Top-level files: `.cirrus.yml`, `CMakeLists.txt`, `CONTRIBUTORS`, `LICENSE`, `README.md`, `RELEASE-PROCESS.md`, `mega-test*.{cfg,py}`, `release.sh`, `style-guide.md`, `super-test.sh`
- **No Rust implementation in this repository** — the canonical Rust binding is the separate `capnproto-rust` project (not part of this clone; not donated here). This clone is the C++ reference implementation plus documentation only.

Full source tree staged with nested `.git` removed.

## Build Systems Detected

- `c++/CMakeLists.txt` (primary CMake build), `c++/CMakeLists.txt` includes a full autotools-era layout under `c++/` (`configure`-style artifacts referenced in `RELEASE-PROCESS.md`) — not executed, only read as text per the hard no-execution rule.
- `.cirrus.yml` — Cirrus CI configuration.

## License Evidence

- `.atlas/licenses/donors/capnproto/LICENSE` — full text read directly.
- **License: MIT.** Copyright header: "Copyright (c) 2013-2017 Sandstorm Development Group, Inc.; Cloudflare, Inc.; and other contributors. Each commit is copyright by its respective author or author's employer." Standard MIT permission/warranty text follows, verified verbatim.

## Mechanisms Census

All of the following is read directly from `doc/encoding.md` and `doc/language.md` (the project's own normative wire-format and language-evolution specification), not summarized from README/marketing text.

### Schema model (`.capnp` schema language)

**Provider: Cap'n Proto's own schema compiler front end (`c++/src/capnp/compiler/`, driven by `.capnp` files like `c++/src/capnp/schema.capnp`).** Structs, enums, interfaces, unions and groups are declared with explicit **ordinal numbers** (`@0`, `@1`, ...) that are permanent identity markers for each member, independent of source-code order or field name. This numbering is the single mechanism that makes every later evolution rule work: field *position on the wire* is derived from ordinal + type, not from declaration order, so members can be freely reordered in source as long as the ordinals are preserved (verified in `doc/language.md` lines ~714-813, "Evolving Your Protocol").

### Binary encoding — pointer-based, in-place-traversable "infinity byte" wire format

**Provider: Cap'n Proto's own `doc/encoding.md` (read in full, 428 lines).** Key structural facts, all verified from the spec text itself:

- The unit of communication is a **message**: a tree of objects (never a graph — "objects and the pointers connecting them form a tree, not a graph"), with the root always a struct, optionally split across multiple **segments** (flat byte blobs) for incremental-allocation reasons.
- Every value larger than a primitive is reached through a **pointer**: a single 64-bit word encoding a 2-bit tag (struct=0, list=1, far-pointer=2, capability/other=3) plus an offset. Struct pointers encode `(offset, data-section-word-count, pointer-section-word-count)`; list pointers encode `(offset, element-size-tag, element-count-or-word-count)`.
- A struct's content is split into a **data section** (packed scalars) followed immediately by a **pointer section** — this split is exactly what allows a struct to be **traversed and copied without knowing its type** (verified quote: "This split allows structs to be traversed (e.g., copied) without knowing their type").
- **Field placement rule** that is the direct mechanical enabler of forward/backward compatibility: "The position of each field depends only on its definition and the definitions of lower-numbered fields, never on the definitions of higher-numbered fields." Later-numbered fields may be packed into padding left by earlier fields; a struct never has more than 63 bits of padding since objects round up to whole words anyway.
- **Default-via-XOR + all-zero-is-default**: every field is stored XOR'd with its schema-declared default, so a "default struct is always all-zeros." This is deliberately exploited three ways: (1) it makes Cap'n Proto's "packing" compression scheme (below) extremely effective since zero bytes compress trivially; (2) newly-allocated structs need only be zeroed, not field-by-field initialized; (3) **a newly-added field placed in former padding is correctly read as its default by old binaries that don't know about it, because unset memory is already zero** — this is a subtle, load-bearing design choice, not an accident.
- **Zero-sized structs get a special-cased offset of -1** (not an all-zero pointer, which is reserved to mean "null") — the doc includes an explicit "Historical explanation" of why this distinction was necessary (null = default value of the *field*; empty struct = default value of the *type* — not necessarily the same thing). This is a genuinely subtle wire-format lesson about the difference between "absent" and "present-but-empty" in a pointer-based format.
- **Struct lists must be encoded with element-tag C=7 (composite), never any other width**, specifically because allowing narrower encodings (as pre-0.5 Cap'n Proto did) makes canonicalization impossible without knowing the schema. This is a direct, named precedent for "an optimization that breaks canonicalization must be forbidden by the format, not left to encoder discretion" — directly relevant to Atlas's own canonical-form design.
- **Inter-segment pointers ("far pointers")**, including a "double-far" landing-pad convention for the rare case where even one word can't be allocated in the target segment — a real answer to "what happens when your bump allocator's current segment is full and you need to point at something elsewhere," relevant if `.atlas`/`.atlasx` ever supports multi-segment/streaming construction.
- **Packing scheme**: a simple byte-oriented RLE-of-zero-bytes scheme (tag byte + nonzero bytes only) applied on top of the base encoding for bandwidth-constrained transmission, with two special tag values (0x00 = run of all-zero words, 0xff = span of literal/incompressible words) chosen specifically so encode/decode only need to branch once per *word*, not once per *byte* — a concrete, measured performance-vs-simplicity tradeoff worth studying if Atlas ever wants an optional compact-transmission mode distinct from its primary on-disk format.

### Canonical message representation

**Provider: `doc/encoding.md`, "Canonicalization" section — verified verbatim, this is the single most directly relevant section for Atlas's hashing/signing needs.** Cap'n Proto messages have a **well-defined canonical form that encoders are not required to produce by default**, but that any implementation can derive from an arbitrary valid message *without knowing its schema*. The exact rules:

1. Object tree encoded in **preorder** with respect to pointer order within each object.
2. Message must be a **single segment** — the segment table is explicitly excluded from what gets signed/hashed, "because it would be redundant."
3. **Trailing zero-valued words in a struct's data/pointer sections must be truncated.** Because zero is always the default, this doesn't change meaning, and critically means **adding a new field to a struct does not change the canonical encoding of old messages that don't set it** — canonicalization and forward-compatible evolution are designed together, not as separate concerns.
4. The same trailing-zero truncation applies per-column across all elements of a struct list (all structs in a struct list have equal size, so a trailing word can only be dropped if it's zero in *every* element).
5. Zero-sized struct pointers get offset -1 (ties back to the zero-sized-struct rule above).
6. **Canonical messages are never packed** — packing is purely a transmission-time optimization; verifying a signature requires unpacking first.

The doc also records a real historical mistake and its fix, useful process precedent: prior to Cap'n Proto 0.5, struct lists could be encoded with any element width for compactness, but this made schema-free canonicalization impossible; 0.5 forced C=7 (composite) encoding for all struct lists specifically to make canonicalization tractable, accepting a data-that-was-compact-becomes-slightly-larger tradeoff for a from-day-one-broken cross-cutting property (canonicalization). **This is a direct, load-bearing precedent for the principle "compactness optimizations that don't compose with canonicalization must be forbidden by the format spec, not left as an encoder choice," directly applicable to `.atlas` if Atlas wants any content-addressing/signing guarantee over its own binary artifacts.**

### Zero/low-copy access

The struct data/pointer-section split plus the pointer-based tree structure together give **O(1) random-access field reads and no required parse pass**: a reader with the pointer to the message root can directly read fields it cares about via `(offset, size)` arithmetic, without walking or copying the rest of the message — this is the same core property Atlas is presumably after for `.atlas`/`.atlasx` reader-side performance. The security section is direct, useful engineering advice for anyone implementing such a reader over untrusted bytes (see below).

### Evolution/versioning rules

**Provider: `doc/language.md`, "Evolving Your Protocol" section (lines 714-813, read in full) — this is the normative rule set, not a paraphrase.** Backward/forward-compatible changes that also **preserve canonical encoding**:

- New types/constants/aliases anywhere (no wire effect).
- New fields/enumerants/methods, as long as each new member's ordinal is **larger than all previous members' ordinals** for that scope.
- New method parameters must go at the end of the parameter list and have default values.
- Members may be **reordered in source code** freely as long as ordinals are unchanged — ordinal is the only thing that matters for wire compatibility, not declaration position.
- Symbolic (name) changes are free — wire encoding is keyed on ordinal/type ID, never on name.
- A field may be **replaced by a group/union containing an equivalent field plus new fields**, with an explicit caveat documented about partial forward-compatibility breakage for old readers that don't know about the new union tag — a concrete example of the library being honest about a rule's limits rather than oversimplifying.
- Separately: changes that are backward-compatible but **do change the canonical encoding** are called out explicitly (e.g., upgrading `List(T)` to `List(struct{T,...})` when `T` is primitive) — the doc is careful to separate "wire compatible" from "canonically-identical," which is a distinction Atlas will likely also need if it wants both evolvability and stable content hashes.

### Security considerations for untrusted-input readers

Directly relevant to any Atlas reader that must accept `.atlas`/`.atlasx` bytes from an untrusted or at-least-unverified source (e.g. a donor artifact, a remote build cache entry):

- **Pointer validation must be lazy** (checked when a getter is called, not via an upfront full-message scan), specifically to preserve Cap'n Proto's headline O(1)-parse property — an upfront validation pass would defeat the format's whole point.
- **Amplification-attack defense**: a running "traversal limit" counter incremented by the size of data each dereferenced pointer points to, capped (C++ impl default 64 MiB), because cyclic/overlapping pointers or deeply-shared substructures can make a tiny message expand to unbounded apparent work. A named special case: lists of `Void` or zero-sized structs can have huge element counts at effectively zero wire cost, so the traversal-limit accounting explicitly charges a minimum of one word per element even for zero-sized elements (traced to a specific named upstream commit, i.e. this was a real discovered vulnerability class, not a theoretical concern).
- **Stack-overflow DoS defense**: pointer-nesting depth is tracked and capped (C++ impl default 64) to prevent recursive-descent readers from being driven into a stack overflow by adversarially deep nesting.

## Test / Benchmark Roots Detected

- `c++/src/capnp/*-test.c++` (per-module unit tests, C++ Google-Test style, inferred from filenames — not executed)
- `c++/src/benchmark/` — `carsales.capnp`, `catrank.capnp`, `eval.capnp` schema-driven micro-benchmarks
- `super-test.sh`, `mega-test.py`, `mega-test*.cfg` — cross-platform CI test-matrix orchestration scripts (read as text only, never executed per the hard no-execution rule)

## Major Subsystem Roots

- `c++/src/capnp/` — the C++ reference implementation of the encoding, compiler, and RPC layer
- `c++/src/kj/` — Cap'n Proto's own general-purpose C++ support library (async I/O, memory, string handling) that `capnp` is built on
- `doc/` — the Jekyll-based documentation site; `doc/encoding.md` and `doc/language.md` are the two files this census is built from
- `security-advisories/` — a real, dated record of past vulnerabilities in the C++ implementation, useful as a "known attack classes against pointer-based binary formats" reading list if Atlas wants to threat-model its own reader

## Disposition per Concept

| Concept | Disposition | Notes |
|---|---|---|
| Pointer-based tree encoding, struct data/pointer-section split | **STUDY** | Strong direct precedent for an O(1)-random-access `.atlas` binary layout; adapt the *principle* (split fixed data from variable/pointer content), not the exact bit-packing. |
| Ordinal-based field identity (independent of declaration order) | **ADAPT** | Directly reusable idea for `.atlas` schema evolution: number fields explicitly, derive wire position from number, allow free source reordering. |
| Canonicalization rules (preorder, single-segment, trailing-zero truncation, forced composite-list encoding) | **ADAPT** | This is the clearest, most load-bearing lesson in the whole donor for Atlas's hashing/signing needs; the "forbid optimizations that break canonicalization" principle should become an explicit Atlas format-spec rule, not left to encoder discretion. |
| XOR-against-default + all-zero-is-default field storage | **STUDY** | Elegant but has real subtlety (interacts with compression, with "newly-added field lands in old padding" correctness); worth understanding deeply before deciding whether `.atlas` needs the same trick or can use a simpler "explicit presence bitmap" instead. |
| Far-pointer / multi-segment addressing | **DEFER** | Only relevant if `.atlas`/`.atlasx` ever needs incremental multi-buffer construction; not needed for an initial single-buffer format. |
| Packing (RLE-of-zero-words) compression scheme | **DEFER** | A nice, cheap, well-specified transmission-time optimization, but orthogonal to the core format decision; revisit only if bandwidth becomes a concern separate from on-disk size. |
| Pointer validation / traversal-limit / depth-limit security discipline | **ADAPT** | Directly actionable checklist for any `.atlas` reader that must accept untrusted bytes: lazy pointer validation, a traversal-limit counter (with the zero-sized-element special case), and a nesting-depth cap. |
| Full RPC layer (`rpc.capnp`, `persistent.capnp`, level-3 promise pipelining) | **REJECT (out of scope)** | Not evaluated in depth; Atlas's donor need here is encoding/canonicalization only, not distributed RPC. Noted as a gap, not dismissed as bad — simply outside this census's mandate. |
| C++ reference implementation / `kj` library as code | **REJECT as dependency** | Never a runtime dependency target; this is architecture study of the *format*, not adoption of Cap'n Proto's C++ toolchain. |

## Things NOT to Copy

- **Never take a runtime/build dependency on `capnproto`'s C++ library, `kj`, or the `capnp` compiler binary.** This census is format-design study only.
- Do not adopt Cap'n Proto's RPC/capability system (`rpc.capnp`, promise pipelining) as a model for anything in Atlas's construction pipeline — it wasn't censused here and is a different problem domain (distributed object capabilities, not static binary-artifact encoding).
- Do not copy the exact bit-widths/tag values verbatim into `.atlas` without an independent design pass — the *principles* (ordinal-based fields, preorder canonicalization, forbidding non-canonicalizable optimizations, lazy validation with traversal/depth limits) are the transferable asset, not Cap'n Proto's specific 2-bit-tag/30-bit-offset pointer layout, which was optimized for Cap'n Proto's own constraints (e.g. 32-bit-vs-64-bit segment addressing tradeoffs that may not apply to Atlas).

## Known Risks / Gaps in This Census

- The RPC layer (`rpc.capnp`, `persistent.capnp`, three-party handoff) was not read in depth — out of scope per the task framing, but flagged here so no one assumes it was evaluated and rejected on the merits.
- The C++ reference-implementation source (`c++/src/capnp/*.c++`) was not read in detail; all encoding/canonicalization claims are sourced from the normative `doc/encoding.md`/`doc/language.md` spec text, which is Cap'n Proto's own documented contract, not independently verified against the implementation's actual behavior (no code was executed, per the hard no-execution rule).
- No Rust binding exists in this repository (`capnproto-rust` is a separate project not donated here), so no Rust-specific implementation lessons could be drawn from this donor — the FlatBuffers and rkyv donors carry that responsibility for this batch.
- Field-offset computation ("Field offsets are computed by the Cap'n Proto compiler. The precise algorithm is too complicated to describe here") is explicitly left unspecified by the donor's own documentation; a from-scratch Atlas implementation of an equivalent layout algorithm would need to derive it independently or study the compiler source directly, which this census did not do.
