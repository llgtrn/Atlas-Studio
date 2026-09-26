---
id: atlas.decision.0070.foreign-functions-and-unrecognized-source
type: decision
status: accepted
canonical: true
---
# ADR 0070 — Foreign functions, include splicing and unrecognized source (G154, replay R3)

## Context

Replay R3 put tree-sitter through Atlas at E3. Tree-sitter is the syntax substrate Atlas has linked since E2, and the donor was pinned at the exact version Atlas links (v0.25.10, `da6fe9beb4f7`).

The challenge was: what executes when `Parser::parse` runs, and what does a change to `lib/src/parser.c` put at risk? Atlas_N answered as follows:
- `Parser::parse` reaches `parse_with_options`, which ends at an unresolved `ts_parser_parse_with_options`.
- Atlas held no record of any function declared in an `extern` block: 0 of 165.
- Atlas held no component for any of 63 C and header files, and refused impact on `lib/src/parser.c`.

Nothing was invented, but three things were silently absent:
- **The extractor skipped foreign modules by design.** They were out of R4.3's minimum set.
- **The path resolver did not follow `include!("./bindings.rs")`**, which is how the binding's `ffi` module gets its declarations.
- **The world model listed only censused languages**, so the pilot saw a Rust and JavaScript repository.

## Decision

1. **FOREIGN_FUNCTION.** A function declared in an `extern "ABI" { ... }` block is a function record of the new kind `FOREIGN_FUNCTION`:
   - It is a DECLARATION.
   - Its signature carries the block's ABI and `is_extern`.
   - It has no body fingerprint.

   A foreign static is a declared symbol. The implementation is outside the census, and a call to one crosses a language boundary.
2. **The resolver binds foreign functions** as call targets in their module.
3. **Include splicing.** An item-position `include!("literal")` splices that file's items into the including module, as rustc does.
   - The path is taken relative to the including file.
   - A computed argument (`concat!`, `env!`) is not followed. It stays a macro call, so nothing is guessed.
4. **`GAP-UNRECOGNIZED-SOURCE`.** Artifacts in the inventory with no registered source frontend are listed by extension in a world-model understanding gap owned by DEBT-MULTILANGUAGE. They were already accounted as UNKNOWN in the census. The pilot is now told they exist.

This is the capability of epoch E4: Atlas sees the Rust side of any `extern` boundary, and states what it cannot see.

## Evidence

The same pin at E3 and at E4 (`evidence/replay/R3-tree-sitter.json`):

| Measure | E3 | E4 |
|---|---|---|
| Foreign declarations | 0 | 165 (156 with resolved Rust callers) |
| Resolved call edges | 2,133 | 2,322 |
| Relations | 5,303 | 5,571 |
| Unrecognized artifacts stated | none | 171 (c 28, h 35, plus Zig, Go, Swift, Python and others) |

Against the C sources:
- 162 of the 165 declarations are defined in `lib/src`.
- The other three are `_open_osfhandle` (Windows CRT), `getuid` (libc) and `tree_sitter_PARSER_NAME` (a template placeholder).

Atlas claims none of them is implemented inside the census, which is ATLAS_CORRECT.

Falsification: 7 mutants were run, and all 7 were killed:
- foreign items not bound;
- include not spliced;
- include path taken from the crate root;
- foreign function recorded as a definition;
- block ABI dropped;
- recognized files counted;
- computed include followed.

## Consequences

- **Tree-sitter is decided EXTERNAL_BOUNDARY at E4.** It is the linked syntax substrate, and a parse is never semantic support. G65's REFERENCE_ONLY verdict stays as history. The verdict re-opens with a C frontend or with a native TypeScript/JavaScript parser.
- **The source is extinct.** Atlas links the published crates.
- **Still unknown:** what the C does, and resource acquisition and release across the boundary (`ts_parser_new` / `ts_parser_delete`; DEBT-RESOURCE).
- **GitNexus was re-checked at E4 and stays CURRENT.** Its triggers are TypeScript module resolution, execution flows and hunk seeds.
- **The replay ledger gains a QUEUED state**, with exactly one donor queued. The next replay is `salsa-rs/salsa`.
