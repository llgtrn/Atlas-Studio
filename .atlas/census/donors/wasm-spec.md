---
id: atlas.census.donors.wasm-spec
type: donor-census
status: active
canonical: true
---
# WebAssembly Specification Donor Census

## Source

- Donor: `WebAssembly spec`
- Remote: `https://github.com/WebAssembly/spec`
- Commit: `ba9fd9f5c23e569201265d5bda6fb8dde18ad8c0`
- Clone path: `.atlas/temporary/donors/wasm-spec`
- License: **multiple, per top-level directory**, declared in root `LICENSE` (an index, not a blanket grant):
  `document/` = W3C Software and Document Notice and License; `spectec/`, `interpreter/`, `test/` = Apache License
  2.0 (full text verified in `interpreter/LICENSE`); `proposals/` = Creative Commons CC0 1.0 Universal; `papers/` =
  Creative Commons Attribution 4.0 International (CC BY 4.0). Any future absorption decision must track which
  subdirectory a given file came from.
- Donor gap: `ATLAS_VALIDATION_BINARY_ENCODING_SEPARATION_PRIOR_ART`
- Quarantine: none required — verifier reported no `.claude`/`.codex`/`.cursor`/`CLAUDE.md`/`AGENTS.md`/
  `.mcp.json` paths anywhere in this donor's tree.
- Note: this is a shallow, top-level-only clone; `.gitmodules` references were **not** initialized (no submodule
  checkout was performed), so any content that lives only in a submodule is not present in this staged tree.

## Framing (read this before the rest)

This donor is censused **purely as a validation/binary-format-separation prior-art reference** for Atlas's own
"ASIR is verified/canonicalized before mechanical binary encoding to `.atlas`" design
(`ATLAS-TO-ATLASX.md`'s canonical stage order begins "M0 Verify parent Atlas + Genome + schemas... verify Atlas
wire/root integrity... verify required shards are present/resolvable... verify materialization-critical
obligations are closed" — verification precedes and gates materialization/encoding, exactly mirroring the
structure censused below). **This census explicitly does NOT recommend adopting Wasm's actual instruction-set
semantics.** Atlas's semantic dialects operate at a richer, source-level vocabulary than Wasm's low-level
stack-machine opcodes (`i32.add`, `local.get`, etc.); the only thing being studied here is the *architectural
separation* between (a) a declarative type-system specification of what makes a program valid, (b) an efficient
single-pass algorithm that implements that specification, and (c) a wholly separate binary encoding chapter that
assumes validity and only concerns itself with byte layout. Atlas is **not** adopting the Wasm spec, its reference
interpreter, or its binary format as a runtime dependency.

## Coarse Inventory

- Files observed: 1294
- Bytes observed: ~51 MB (`du -sh` = 51M) — large relative to the other two donors in this batch; the bulk is the
  `document/` directory's build tooling and multiple versioned specification snapshots
  (`specification/wasm-1.0`, `wasm-2.0`, `wasm-3.0`, `wasm-latest`), the `papers/` PDFs (`oopsla2019.pdf`,
  `pldi2017.pdf`, `pldi2024.pdf`), and a large `test/core`/`test/js-api` conformance-test corpus.
- Top-level structure: `document/core/` (the reST-sourced core specification — the primary census target),
  `interpreter/` (an OCaml reference interpreter, Apache-2.0, not read in depth here), `spectec/` (a
  formal-spec-generation toolchain that produces the reST from a typed spec DSL — present but not deep-read),
  `test/` (the official conformance-test suite, `.wast` format), `proposals/` (in-progress extension proposals,
  CC0), `papers/` (academic papers backing the spec, CC BY 4.0), `specification/` (versioned HTML/build snapshots)
- `document/core/` internal structure (verified via directory listing): `intro/`, `syntax/`, `valid/`, `exec/`,
  `binary/`, `text/`, `appendix/`, `util/` — this five-way split (abstract syntax / validation / execution
  semantics / binary encoding / text format) is itself a directly relevant structural fact: **the spec's own
  top-level organization already separates "what is valid" from "how it is encoded" into different directories,
  not different sections of one document.**

## Mechanisms Census (read from `document/core/valid/`, `document/core/binary/`, `document/core/appendix/algorithm.rst`)

### Opcode / instruction model

- `document/core/binary/instructions.rst` (467 lines in the syntax counterpart, binary chapter verified directly):
  "Instructions are encoded by *opcodes*. Each opcode is represented by a single byte, and is followed by the
  instruction's immediate arguments, where present." Structured control instructions (`block`/`loop`/`if`) are the
  one exception, encoded as *paired* opcodes bracketing a nested instruction sequence rather than a single flat
  opcode+immediates — verified directly from source, not inferred.
- The document is explicit and self-aware about a real historical wart, worth carrying forward as a design lesson:
  "The byte codes chosen to encode instructions are historical and do not follow a consistent pattern... instructions
  are hence not presented in opcode order, but instead grouped consistently with other sections." This is a
  concrete cautionary data point: an opcode numbering scheme that isn't assigned by a consistent rule (e.g.
  grouped-by-category, sorted-by-frequency, etc.) becomes permanently undocumentable-by-pattern and has to be
  memorized/indexed separately (the appendix carries a dedicated opcode index, `appendix/index-instructions.py`
  confirmed present) — a caution for Atlas's own `.atlas` binary opcode/tag numbering if/when it exists.
- **Instruction *typing*** (as opposed to encoding) is governed separately in `document/core/valid/instructions.rst`
  (1429 lines, the single largest validation document): every instruction has an **instruction type**
  `t_1* ->_(x*) t_2*` describing the operand-stack types it consumes, the types it produces, and the local-variable
  indices `x*` it initializes. This is a real, formal stack-effect type signature per instruction, not an informal
  description — verified directly, including a precise definition of two flavors of polymorphism: *value-
  polymorphic* (one operand's type is unconstrained, e.g. `drop`/`select`) and *stack-polymorphic* (the entire
  instruction's stack effect is unconstrained, e.g. `unreachable`/`br`/`return` — control instructions that
  perform unconditional transfer). Both are illustrated with concrete worked examples showing exactly which
  sequences type-check and which don't (e.g. `unreachable; i64.const 0; i32.add` is explicitly shown **invalid**
  because no consistent type can be picked for `unreachable`'s polymorphic output that also satisfies the
  following `i32.add`).

### The validation algorithm (type-checking a stack machine)

- `document/core/appendix/algorithm.rst` is the standout mechanism for this donor's framing, and matches the
  requested census point almost exactly. Its own opening lines state the separation being censused, verbatim:
  "The specification of WebAssembly validation is purely *declarative*. It describes the constraints that must be
  met... This section sketches the skeleton of a sound and complete *algorithm* for effectively validating code...
  the algorithm is expressed over the flat sequence of opcodes as occurring in the binary format, and performs
  only a single pass over it. Consequently, it can be integrated directly into a decoder." — i.e. the spec itself
  draws a hard line between (1) the *declarative type system* (`valid/instructions.rst`, `valid/modules.rst`,
  expressed as inference rules, not an algorithm) and (2) a *separate, explicitly-labeled algorithmic appendix*
  that gives an efficient single-pass implementation of that same type system, engineered to be foldable directly
  into a binary decoder. This is a genuine, verified, three-way document separation (declarative typing rules /
  algorithmic validator / binary encoding), not a two-way one, and is the single strongest concrete parallel to
  Atlas's own verify-then-encode pipeline design.
- The algorithm itself is concretely specified as typed pseudocode operating over explicit data structures
  (verified from source): `val_type = num_type | vec_type | ref_type | Bot` (with a `Bot` bottom type used to
  represent the "unreachable"/polymorphic stack-top state cleanly, rather than as a special case scattered through
  the algorithm), `func_type`/`struct_type`/`array_type`/`comp_type`/`sub_type`/`rec_type`/`def_type` for the
  composite/recursive type system, plus an explicit closedness/canonicalization assumption: "We assume that all
  types have been *canonicalized*, such that equality on two type representations holds if and only if their
  closures are syntactically equivalent, making it a constant-time check" — i.e. the algorithm's efficiency
  depends on types being pre-canonicalized so type-equality is a cheap structural/pointer check, not a recursive
  structural comparison performed repeatedly during validation. **This canonicalize-once-then-compare-cheaply
  pattern is directly relevant to Atlas's own ASIR verification design**, if Atlas's own type/shape identities are
  (or should be) canonicalized once before repeated validation passes reference them.

### Sectioned binary format

- `document/core/binary/modules.rst` (verified directly, `binary-module` anchor): "The encoding of a module starts
  with a preamble containing a 4-byte magic number (the string `\0asm`) and a version field. The current version
  of the WebAssembly binary format is 1." followed by "a sequence of sections. Custom sections may be inserted at
  any place in this sequence, while other sections must occur at most once and in the prescribed order. All
  sections can be empty."
- The full section table was verified directly by id: custom sections = id 0 (unordered, repeatable, the only
  extension point that doesn't require a format-version bump); type = 1; import = 2; function = 3; table = 4;
  memory = 5; global = 6; export = 7; start = 8; element = 9; code = 10; data = 11; data-count = 12; tag = 13.
  Each section's binary chapter opens with "the actual *contents*, whose structure is dependent on the section
  id" — i.e. a section is generically `(id: byte, size: u32, contents: bytes)` with content interpretation
  dispatched on `id`, a clean, minimal, forward-compatible envelope structure (unknown/custom sections can always
  be skipped by size without understanding their contents).
- A cross-section **consistency invariant is stated explicitly and is validation-adjacent, not merely
  descriptive**: "The lengths of lists produced by the (possibly empty) function and code sections must match up,"
  and "the optional data count must match the length of the data segment list... it must be present if any data
  index occurs in the code section" — i.e. even the binary-format chapter itself encodes cross-section structural
  invariants that a decoder must check, showing the validation/encoding separation is not perfectly clean at the
  edges (some structural checks are binary-format-shaped, not purely semantic) — a realistic caution for Atlas's
  own design: expect a small number of legitimately cross-cutting invariants that don't sort neatly into "pure
  semantic validation" vs. "pure binary layout," and plan for where those live rather than assuming a perfectly
  clean split is achievable.
- The format is explicitly designed to be forward-extensible without a version bump: "such changes are expected to
  occur very infrequently, if ever. The binary format is intended to be extensible, such that future features can
  be added without incrementing its version" — extension happens via new section ids / new opcodes within the
  existing envelope, not via version-number branching, a deliberate design choice worth noting for `.atlas`'s own
  binary format evolution strategy.

### The explicit separation itself (the core point of this census)

Structurally verified across three independent pieces of evidence, not just asserted: (1) `document/core/` puts
`valid/` (declarative typing rules) and `binary/` (encoding) in **separate top-level directories**, each with its
own independent chapter structure; (2) `appendix/algorithm.rst` is a **third, separately-labeled document** that
bridges the two — an algorithmic validator explicitly designed to be foldable into a binary decoder, but which is
itself neither the declarative rules nor the encoding spec; (3) the algorithm's own stated design goal — "a single
pass... integrated directly into a decoder" — shows the spec authors deliberately engineered validation to be
decoder-compatible in *complexity* (single-pass, streaming-friendly) while keeping it specified independently of
the decoder's actual byte-level concerns (opcode values, LEB128 encoding, section framing). **This is the closest
existing prior art to Atlas's own "ASIR is verified/canonicalized before mechanical binary encoding to `.atlas`"
pipeline**: a declarative correctness specification, an efficient algorithm implementing it, and a binary format
chapter that assumes the algorithm has already run and only concerns itself with byte layout — three cleanly
separated concerns, each independently document-owned.

## Atlas Comparison

- **Atlas today** (`ATLAS-TO-ATLASX.md`): the ATLAS→ATLASX materialization contract's canonical stage order opens
  with a verification phase (M0: "Verify parent Atlas + Genome + schemas," "verify Atlas wire/root integrity,"
  "verify Genome/schema compatibility," "verify required shards are present/resolvable," "verify
  CensusCertificate/seal policy," "verify the SelectedDesign refers only to identities reachable from the pinned
  Atlas root," "verify materialization-critical obligations are closed") before later stages build the canonical
  AtlasX object model and manifest — i.e. Atlas's contract already specifies verification-before-materialization
  as a matter of policy. What this donor adds is not the *idea* (Atlas already has it) but a **concretely
  verified, working example of the same architecture at a different layer of abstraction** (instruction-level
  type-checking of a stack machine, rather than shard/schema/lineage verification), plus a specific, reusable
  technique: canonicalize types once so that repeated validation-time equality checks are cheap, and keep the
  validation algorithm's data structures (e.g. an explicit `Bot`/bottom type for polymorphic states) simple enough
  to remain a genuine single pass.
- **What is explicitly NOT being proposed**: adopting Wasm's actual instruction set, its stack-machine execution
  model, or its opcode numbering as any part of Atlas's semantic dialects or ASIR. Atlas's dialects are
  source-level and richer than Wasm's low-level ops; this donor is a structural/process parallel only, at the
  altitude of "how do you organize a verify-then-encode pipeline across documents/stages," not "what should the
  instruction set look like."

## Disposition (per concept)

| Concept | Disposition | Notes |
|---|---|---|
| Declarative validation rules / algorithmic validator / binary encoding as three separately-owned documents | **STUDY** | The core donor value. Worth an explicit design review of whether Atlas's own ASIR-verify → materialize pipeline documentation should adopt the same three-way split (declarative invariant spec, algorithmic verifier, binary encoding chapter) rather than blending them. |
| Single-pass, decoder-foldable validation algorithm design goal | **STUDY** | Directly relevant engineering constraint if Atlas's own ASIR verifier is meant to run efficiently/streamingly as part of materialization, not as a separate offline pass. |
| Canonicalize-types-once-for-cheap-equality pattern | **ADAPT** | A small, concrete, portable technique; worth adopting explicitly if Atlas's own type/shape identities are repeatedly compared during verification. |
| Explicit polymorphic/bottom-type handling (value- vs. stack-polymorphic) | **STUDY** | Useful vocabulary and a clean technique (a `Bot` type) for representing "unconstrained by control flow" states cleanly in a verifier, if Atlas's ASIR verification ever needs to reason about unreachable/divergent code paths. |
| Sectioned binary envelope (id + size + contents, skippable unknown sections) | **ADAPT** | A minimal, well-proven, forward-compatible binary envelope shape; a reasonable reference for `.atlas`'s own section/shard framing if not already settled by `ATLASX-BINARY-WIRE-FORMAT.md` (not read in this census). |
| Cross-section structural invariants living partly in the binary chapter (function/code length match, data-count consistency) | **STUDY** | A realistic caution, not a mechanism to copy: expect some invariants to not sort cleanly into "semantic" vs. "binary," and decide deliberately where Atlas's own analogous checks live. |
| Actual instruction set / opcode semantics (`i32.add`, `local.get`, etc.) | **REJECT** | Explicitly out of scope per this census's framing — Atlas's semantic dialects are a different, richer, source-level vocabulary. Do not port Wasm ops. |
| Opcode numbering scheme (historical, inconsistent, separately indexed) | **REJECT** (as a pattern to imitate) | Explicitly flagged in the spec's own text as inconsistent; a cautionary example, not a template, for any future `.atlas` opcode/tag numbering. |
| Concurrency/threads, SIMD, GC, exception-handling proposals (`proposals/`) | **DEFER** | Not read in this census; out of scope for the validation/binary-separation focus. |
| OCaml reference interpreter (`interpreter/`) | **DEFER** | Not read in depth; could be a future donor for "how do you build a reference implementation that both an algorithmic validator and an executor agree with," but not covered here. |

### Things NOT to copy

- Do not adopt Wasm's instruction set, opcode values, or stack-machine execution semantics into Atlas's semantic
  dialects or ASIR — this donor is a structural/process parallel (verify-then-encode), not a semantics donor.
- Do not add the Wasm reference interpreter, `spectec` toolchain, or any part of this repository as an Atlas
  runtime or build-time dependency.
- Do not port the historical, admittedly-inconsistent opcode numbering scheme; if anything, treat it as a
  cautionary example for how *not* to assign `.atlas` binary tags.
- Do not treat the sectioned binary format's specific section ids/ordering as binding on `.atlas`'s own format —
  only the general shape (typed envelope, skippable unknown sections, explicit cross-section invariants) is the
  donor idea.

## Known Risks / Gaps in This Census

- This census targeted exactly the requested mechanisms (opcode/instruction model, validation algorithm, sectioned
  binary format, validation/encoding separation) from `document/core/valid/` and `document/core/binary/` plus the
  appendix algorithm; `document/core/exec/` (the full operational/execution semantics) and `document/core/text/`
  (the text format) were located but not deep-read, since execution semantics is squarely the "Wasm instruction
  set" territory this census explicitly declines to recommend adopting.
- `interpreter/` (the OCaml reference implementation) and `spectec/` (the spec-generation toolchain that produces
  the reST source) were inventoried but not read — `spectec` in particular could be independently interesting
  later as a donor for "how do you generate a validation algorithm from a formal type-system spec," but that is
  speculative and not evaluated here.
- The large `test/core`/`test/js-api` conformance-test corpus (a significant share of the 51 MB tree size) was not
  read; it was not needed for the architectural-separation question this census answers.
- Because this is a shallow, non-submodule clone, any content that lives only via `.gitmodules` (if any
  spec-adjacent tooling is vendored as a submodule) is absent from this staged tree and was not censused.
- No execution of any interpreter, build script, or test was performed, per the mandatory quarantine instructions
  — all findings are from static reading of the reST specification source.

## Census State

CENSUSED. Targeted mechanism census complete for the opcode/instruction model, the validation algorithm (typed
pseudocode single-pass stack-machine type-checker), the sectioned binary format, and the explicit
declarative-rules/algorithm/binary-encoding separation, all read from real source under `document/core/valid/`,
`document/core/binary/`, and `document/core/appendix/algorithm.rst`, explicitly scoped to Atlas's
verify-then-encode pipeline donor gap and explicitly declining to recommend Wasm's instruction-set semantics.
