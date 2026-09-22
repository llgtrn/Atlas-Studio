---
id: donor-census-rkyv
type: reference
status: active
canonical: true
---
# Donor Census: rkyv

## Source

- Remote: https://github.com/rkyv/rkyv.git
- Repository: rkyv/rkyv
- Commit: 4845668ae9730a3987966769f6872d86b822dc41
- Branch: master
- Retrieved: 2026-09-22T15:53:06Z
- Staging mode: FULL_SOURCE_TREE
- Clone path: .atlas/temporary/donors/rkyv

### Provenance note

`rkyv/rkyv` (formerly authored under `djkoloski/rkyv`; the org is now `rkyv`) shows no fork/mirror indicators — README, `Cargo.toml` (`authors = ["David Koloski <djkoloski@gmail.com>"]`), and LICENSE all agree on the same canonical author/origin. Treated as canonical/original.

### Framing (explicit, per absorption brief)

Censused as a **Rust-native zero-copy archive format** reference, with an explicit goal of this census: **evaluate, using rkyv's own documentation, whether it is more suitable for internal/ephemeral cache artifacts than for a long-lived, cross-version-compatible, portable `.atlas`/`.atlasx` artifact.** The verdict below is derived from reading rkyv's own docs, not assumed from the task brief.

## Coarse Inventory

- Files observed: workspace of 6 crates — `rkyv` (core), `rkyv_derive` (proc-macro), `benchlib`, and three currently-disabled-in-workspace crates `rkyv_dyn`/`rkyv_dyn_derive`/`rkyv_dyn_test` (commented out of `[workspace.members]` in root `Cargo.toml` — trait-object support is not part of the actively-built workspace at this pin)
- Bytes observed: ~2.0 MB (`du -sh` = 2.0M)
- Languages/signals: Rust (source, `rkyv/src/`, `rkyv/tests/`, `rkyv/benches/`, `rkyv/examples/`), `mdbook` Markdown (`book_src/` — the canonical rkyv book, primary source for this census), TOML (`Cargo.toml` workspace + per-crate manifests)
- Top-level directories: `.cargo`, `.github`, `.vscode`, `benchlib`, `book_src`, `media`, `rkyv`, `rkyv_derive`, `rkyv_dyn`, `rkyv_dyn_derive`, `rkyv_dyn_test`
- Top-level files: `.gitattributes`, `.gitignore`, `Cargo.toml`, `LICENSE`, `README.md`, `SECURITY.md`, `book.toml`, `rustfmt.toml`
- **Current pinned version: 0.8.18** (`workspace.package.version` in root `Cargo.toml`), pre-1.0.

Full source tree staged with nested `.git` removed.

## Direct Dependencies

From workspace `Cargo.toml`/`rkyv/Cargo.toml` (read directly, not inferred): `bytecheck` (0.8, validation), `hashbrown` (0.17, alloc-free maps, optional), `munge` (0.4, field-projection macro), `ptr_meta` (0.3, fat-pointer manipulation), `rancor` (0.1, error handling), `rend` (0.5, endian-agnostic primitives), `rkyv_derive` (workspace-pinned, the `#[derive(Archive, Serialize, Deserialize)]` macro crate). `[patch.crates-io]` pins `bytecheck` to its git `master`, meaning **the released rkyv 0.8.x line depends on an unreleased/HEAD version of its own validation crate** — a live signal of API churn even at the "stable-ish" 0.8 line.

## License Evidence

- `.atlas/licenses/donors/rkyv/LICENSE` — full text read directly.
- **License: MIT.** "Copyright 2021 David Koloski", standard MIT text. `Cargo.toml` corroborates: `license.workspace = true` → `license = "MIT"`.

## Mechanisms Census

All read directly from `book_src/` (the canonical rkyv book shipped in this repo) and `rkyv/Cargo.toml`/`rkyv/src/lib.rs`.

### The `Archive`/`Serialize`/`Deserialize` trait model

**Provider: rkyv's own `book_src/architecture/archive.md`, `serialize.md`, `deserialize.md` (read directly).** Construction of an archived value happens in two explicit steps, and this two-step split is the mechanical core of the whole design:

1. **Serialize step**: dependencies of a value are serialized first (e.g. for a `String`, its bytes; for a `Vec`, its elements). Bookkeeping needed to finish the job later is captured in a `Resolver` value and held.
2. **Resolve step**: the original value plus its `Resolver` are combined to write the final archived representation into the output buffer.

The book gives a concrete, verified rationale for *why* this two-phase split is necessary rather than an implementation detail: naively serializing-then-immediately-finishing each subfield in sequence (e.g. for a tuple of two `String`s) would interleave unrelated data in the output buffer; the `Resolver` indirection lets a parent (the tuple) fully serialize *all* children first, then resolve them together, guaranteeing each child's own bytes stay contiguous. This is a real, load-bearing architectural insight about building composable zero-copy serializers, not boilerplate.

Derived archived types are `#[repr(C)]` for structs and `#[repr(N)]` (smallest sufficient of u8/u16/u32/u64/u128) for enums, with every primitive replaced by an endian-explicit, alignment-explicit wrapper type (verified in `book_src/format.md`) — e.g. a derived `struct Example { a: u32, b: String }` archives to `#[repr(C)] struct ArchivedExample { a: u32_le, b: ArchivedString }` under the (default) `little_endian` feature.

### Relative pointers — the mechanism that makes in-place use possible

**Provider: `book_src/architecture/relative-pointers.md` (read in full).** rkyv's central trick, stated by the book itself: ordinary (absolute) pointers cannot survive being reloaded at a different memory address (ASLR, or simply "you don't control where a memory-mapped file lands"), so rkyv's archived types store **relative offsets** (`RelPtr`) instead of addresses. The book gives an exact table of what survives a move: an absolute pointer survives *the referent* moving with it (rebasing an entire structure moves target too — wrong use case), while a **relative pointer survives *itself and its target* moving together as a unit**, which is exactly memory-mapping's guarantee. This is presented as an explicit, deliberate tradeoff against "abomonation"-style fixup-based zero-copy (cited by name in the book), which requires a *mutable* buffer for its fixup pass — rkyv's relative-pointer approach works on read-only, memory-mapped bytes with no fixup pass at all. **This is the single most reusable idea in this donor for a memory-mappable `.atlas`/`.atlasx` reader: relative-offset pointers, not absolute pointers, are what make a serialized structure directly usable after `mmap()` at an unpredictable address.**

### Object layout order

**Provider: `book_src/format.md`.** rkyv lays out subobjects **depth-first, leaves before root** — the root object ends up at the *end* of the buffer, not the beginning (worked example given: tree `a -> {b, c -> {d, e}}` serializes as bytes `b d e c a`). This is explicitly called out as deterministic specifically so that **no separate "root position" field needs to be stored** — a consumer just needs the buffer to end exactly at the end of the root object. This is the opposite convention from Cap'n Proto/FlatBuffers (which both place a root pointer/offset near the start) and is a genuine, non-obvious design choice worth flagging for Atlas rather than assuming "root-first" is universal.

### `bytecheck` validation of archived bytes before trusting them

**Provider: `book_src/validation.md` (read in full).** With the `bytecheck` feature (on by default), rkyv derives `CheckBytes` for every archived type, and exposes a safe `access::<Archived, Error>(buffer)` entry point that validates before returning a usable reference — explicitly positioned as the mechanism for handling **untrusted/malicious data** safely. Validation is more than a per-field bounds check: it enforces a **subtree-range ownership model** — the validator tracks which byte ranges are "available" for each subobject as it walks the tree, shrinking the available range as it descends and restoring it as it ascends, specifically to catch overlapping/aliasing pointer attacks (a pointer-based-format attack class directly analogous to the ones Cap'n Proto's traversal-limit defends against, verified independently in that donor's own census — the two libraries converge on "an adversarial pointer graph inside a flat buffer is a real attack surface" even though their specific defenses differ). Shared pointers get **additional, explicitly-acknowledged restrictions**: two shared pointers to the same bytes as *different* target types will fail validation, and the book is honest that this is a known limitation, not a bug, and suggests hash/signature-based alternatives for that specific use case.

The book's own FAQ (`book_src/faq.md`) is direct and unhedged about the safety tradeoff: skipping validation and using the `unsafe` fast-path access is genuinely unsafe on untrusted input; the safe path costs an extra validation pass but is still claimed faster end-to-end than deserializing with other high-performance formats. This is rkyv's own framing, not an outside assessment.

### Archived (in-place-usable) representation vs. a portable format — and rkyv's own admitted gap

**This is the central question the task asked to verify rather than assume, and the answer is found directly in the donor's own documentation: `book_src/feature-comparison.md`, "a best-effort feature comparison between rkyv, FlatBuffers, and Cap'n Proto," written by the rkyv maintainers themselves.** The table, read verbatim:

| Feature | rkyv | Cap'n Proto | FlatBuffers |
|---|---|---|---|
| Schema evolution | **no** | yes | yes |
| Cross-language | **no** | yes | yes |
| Reflection | **no** | yes | yes |
| Open type system | yes | no | no |
| Validation | upfront | on-demand | yes |
| Object order | bottom-up | either | bottom-up |

rkyv's own maintainers state, in their own comparison table, that rkyv has **no schema evolution story and no cross-language support**, in direct contrast to both Cap'n Proto and FlatBuffers, which the same table marks "yes" for both. This is not the census brief's framing being repeated as fact — it is rkyv's own maintainers' published self-assessment, confirmed by reading the file. Separately, `Cargo.toml` shows rkyv at **0.8.18, pre-1.0**, with its own validation dependency (`bytecheck`) pinned to an unreleased git `master` rather than a released version — a live, structural signal (not merely a version-number technicality) that the crate's public API and on-wire archived representation are still expected to move before 1.0. No dedicated `CHANGELOG.md` exists in this repository to cross-check specific past breaking changes (a genuine gap — see Known Risks below), but the combination of (a) rkyv's own "no schema evolution" self-rating, (b) pre-1.0 version number, and (c) a released version depending on an unreleased sibling crate is independently sufficient evidence for the same conclusion: **rkyv's archived representation should be treated as tied to a specific build/toolchain/version, not as a stable long-term interchange contract.**

**Verdict on the specific question asked**: rkyv's model — relative pointers plus a derive-generated, `#[repr(C)]`, directly-in-place-usable archived type, validated on demand via `bytecheck` — is well-suited to **internal/ephemeral cache artifacts produced and consumed by the same build of the same program** (e.g. a local systemize-output cache, where producer and consumer share a Rust toolchain, crate version, and target platform/endianness within a single machine or CI run). It is **not well-suited**, by rkyv's own admission, to a **long-lived, cross-version-compatible, portable `.atlas`/`.atlasx` artifact** that must remain readable by future Atlas versions or across independently-updated producer/consumer toolchains — that job wants Cap'n Proto's or FlatBuffers' explicit, load-bearing schema-evolution guarantees instead.

## Test / Benchmark Roots Detected

- `rkyv/tests/` — the core crate's test suite (not executed, read as directory listing only)
- `rkyv/benches/` — Criterion/`divan`-based benchmarks (dependency `divan` present in workspace `Cargo.toml`)
- `rkyv/examples/` — including `remote_types.rs` (cited directly by `book_src/derive-macro-features/remote-derive.md`)
- `benchlib/` — a dedicated top-level crate for cross-format serialization benchmarking

## Major Subsystem Roots

- `rkyv/src/` — the core `Archive`/`Serialize`/`Deserialize` trait implementations, relative-pointer (`RelPtr`) type, standard-library type support (String/Vec/Box/Option/HashMap via `hashbrown`/etc.)
- `rkyv_derive/src/` — the `#[derive(Archive, Serialize, Deserialize)]` proc-macro
- `book_src/` — the canonical rkyv book; this census's primary evidentiary source, covering architecture, format, validation, allocation tracking, and (crucially) the feature-comparison table against Cap'n Proto/FlatBuffers
- `rkyv_dyn/`, `rkyv_dyn_derive/`, `rkyv_dyn_test/` — trait-object support, **currently commented out of the active workspace** (`# "rkyv_dyn"` etc. in root `Cargo.toml`), i.e. not part of what actually builds by default at this pin

## Disposition per Concept

| Concept | Disposition | Notes |
|---|---|---|
| Relative-pointer (`RelPtr`) offset addressing for mmap-safe in-place structures | **ADAPT** | The clearest, most directly reusable idea in this donor: any Atlas format that wants to be usable directly off a memory-mapped, unpredictably-addressed buffer needs relative, not absolute, internal pointers. |
| Two-phase Serialize/Resolve construction discipline (Resolver bookkeeping) | **STUDY** | A genuinely useful pattern for implementing *any* writer that must guarantee child objects stay contiguous while being built bottom-up; worth understanding before writing Atlas's own writer, independent of whether rkyv itself is used. |
| `bytecheck`-style upfront validation with a subtree-range ownership model before trusting archived bytes | **ADAPT** | Directly relevant discipline for any `.atlas` reader accepting bytes from outside the current process (donor artifacts, remote caches); the "shrinking available range as you descend" technique is a concrete, implementable defense against pointer-aliasing/overlap attacks. |
| Depth-first, leaf-to-root object layout with root-at-buffer-end | **STUDY** | A legitimate alternative to root-first layout (as Cap'n Proto/FlatBuffers both use); worth an explicit Atlas decision rather than defaulting to whichever convention is copied first, since it changes how "is this buffer complete" gets checked. |
| The archived (`rkyv`) representation itself, as a **long-lived portable `.atlas`/`.atlasx` format** | **REJECT for that role** | rkyv's own feature-comparison table says "no" to schema evolution and cross-language support; a pre-1.0 crate whose own validation dependency pins to an unreleased git commit is not a stable interchange contract. |
| rkyv as an actual Cargo dependency for **internal/ephemeral cache artifacts** (e.g. a local systemize-output cache within one build) | **ADAPT — explicit exception to "study only"** | This is the one place in this batch of donors where taking rkyv as a real `Cargo.toml` dependency is plausibly warranted, precisely because the cache is same-process/same-build/short-lived and never needs to survive a toolchain or version change; if Atlas builds such a cache, evaluate `rkyv` directly against a hand-rolled equivalent rather than ruling it out on principle. |
| `rkyv_dyn` trait-object support | **DEFER** | Not part of the active workspace at this pin (commented out); not evaluated further. |

## Things NOT to Copy

- **Do not take rkyv as a Cargo dependency for the `.atlas`/`.atlasx` portable artifact itself** — that is precisely the role its own documentation says it is not designed for (no schema evolution, no cross-language guarantee, pre-1.0 with an unreleased-git dependency).
- Do not assume the archived byte layout is stable across rkyv versions when deciding whether *any* persisted-to-disk-long-term Atlas data should use it, even for caching, without pinning the exact rkyv version and invalidating the cache on every rkyv upgrade.
- Do not skip `bytecheck` validation ("unsafe fast path") for any archived bytes that cross a trust boundary (a different process, a different machine, a donor-supplied artifact) — rkyv's own FAQ is explicit that the fast path is genuinely unsafe on untrusted input.

## Known Risks / Gaps in This Census

- **No `CHANGELOG.md` exists in this repository at this pin** — the "weaker versioning story" conclusion is supported by rkyv's own feature-comparison self-rating ("Schema evolution: no"), the pre-1.0 version number, and the `[patch.crates-io]` pin of `bytecheck` to an unreleased git commit, but **could not be cross-checked against a documented list of actual past breaking changes**, because no such document is present in this clone. This is a real gap in census depth, not a claim that was verified against concrete historical diffs — flagged explicitly per the task's instruction to report what was actually found rather than assert more than was verified.
- `rkyv_dyn` (trait-object support) was not censused in depth since it is not part of the active workspace build at this pin.
- The core `rkyv/src/` Rust source (the actual trait implementations) was not read line-by-line; this census relies on the maintainers' own book documentation (`book_src/`) as the authoritative description of the architecture, cross-checked against `Cargo.toml` manifests for dependency/version facts. A future deeper pass should read `rkyv/src/rel_ptr.rs` (or equivalent) directly to verify the relative-pointer implementation matches the book's description.
