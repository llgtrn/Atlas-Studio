---
id: atlas.decision.0077.import-cycles-and-open-scope-causes
type: decision
status: accepted
canonical: true
---
# ADR 0077 — Import resolution converges on re-export cycles, and every open scope names its cause (G162)

## Context

FULL_OSS_REPLAY R7 put zed-industries/zed, at its admitted legacy pin `0eda7703f6c8`, in front of Atlas E10. Zed is an async-heavy editor: GPUI executors, tasks, futures, file handles and locks.

The challenge question came from what E6 (RESOURCE) and E8 (async regions) had added: when Zed saves or atomically writes a file, which file handles are acquired, and where are they released?

Atlas E10 was used first, and it could not answer at any scale:

- **The whole workspace.** The census of all 1,968 Rust files (1.67 M lines) was killed at 15 GB after 157 s. On the gpui crate the census holds about 19 KB per source line: the extraction batches, the census, the normalized facts and the graph are all alive together. Extrapolated to the whole workspace, that is about 32 GB.
- **The fs crate alone** (775 functions, 44 async blocks). It recorded no RESOURCE record at all. `RealFs::atomic_write` calls `std::fs::File::create_new` on Windows; the call is in the declared std-path table, yet it was withheld.
- **The diagnosis took hand bisection.** It found that one `slotmap::new_key_type! { .. }` at item position makes the module *open*. Once a module is open, every lookup that falls through to a glob import or to the extern prelude (`std` included) is withheld, and nothing told the pilot so.
- **A 17-crate slice of the same pin** (402 files, 306 k lines: gpui, fs, worktree, workspace, language, …). Here E10 withheld almost everything. The import fixed point of the whole slice failed after 64 rounds, so every module of the workspace was opened. The cause was a period-2 oscillation: zed's `language` crate re-exports `crate::LanguageRegistry` from `buffer`, and the crate root glob-imports `buffer`. The placeholder that an early round binds for the not-yet-resolved named import then travels around that cycle in the value namespace and flips every round.

`rustc` was the oracle for the refusal, and Atlas's rule matches it:

| Case | rustc result |
|---|---|
| A macro-expanded `mod std` | shadows the extern prelude silently |
| A macro-expanded item against a glob import | shadows the glob silently |
| A macro-expanded item against an item of the module | E0428, a compile error |
| A macro-expanded item against an explicit import | E0255, a compile error |

So an open module must withhold glob and extern-prelude lookups, and may keep the names it defines or imports explicitly. The withholding in `fs.rs` is therefore **correct**. `new_key_type!` is a dependency's macro, and its expansion is not visible without the dependency's source.

## Decision

1. **Import resolution recognizes a period-2 oscillation.** Each round recomputes every scope from the previous round's. When a round repeats the state from two rounds back, the two alternating states are merged per module and the iteration stops:
   - a name both phases bind to the same definition keeps it;
   - any other name either phase binds stays bound, to `Unknown`.

   An `Unknown` binding resolves nothing and never lets a lookup fall through to an outer scope or the extern prelude. The merge is therefore never a false claim. A workspace with no oscillation is resolved exactly as before, and a longer cycle still opens every module.
2. **Every open scope records its cause, and every withheld path names it.** A module is opened by one of:
   - an item macro whose expansion the pass cannot bound, which is a dependency's or another crate's macro, an invocation with a non-allowlisted attribute, or an unbounded definition;
   - a glob that does not resolve, reads a crate outside the workspace, or reads an open module;
   - a missing or unparsable module file;
   - an import fixed point that is not reached.

   The resolver records the first cause. A path call withheld in an open scope is a `WithheldPath` carrying the cause. A multi-segment path whose first segment an open scope may shadow is now reported `open-scope`, no longer `unresolved-prefix`.
3. **The census and the world model carry the explanation.**
   - The path-resolution engine adds one `INCOMPLETE_ANALYSIS` diagnostic per artifact with withheld calls. The diagnostic carries the declared prefix `OPEN_SCOPE_DIAGNOSTIC` and gives, for each cause, how many calls it withholds and the first of them.
   - The composed component carries these diagnostics as `withheld`, and the unknowns lens lists them.
   - `GAP-OPEN-SCOPE` counts the components that carry them.
   - `GAP-RESOURCE` is corrected. It still said there was no RESOURCE dimension; now it names the residual of the G157 profile, and its magnitude counts acquisitions beyond their function's recorded releases.

## Consequences

- **The same zed slice at E10 and E11:**
  - resolved INVOKES relations: 7,648 → 9,059;
  - unresolved call sites: 109,980 → 107,841;
  - RESOURCE records: 0 → 6 (thread spawns, and one JOIN release, checked against the source);
  - open files: every file → 65, and each of the 65 names its cause.

  The 5,942 path calls still withheld have these causes:
  - dependency globs such as `rand::prelude::*`;
  - `actions!` (its expansion derives the proc macro `gpui::Action`);
  - `slotmap::new_key_type!`, `inventory::collect!` and `db::static_connection!`;
  - `super::*` globs of those modules.
- **The fs challenge.** Atlas now answers why the file acquisitions in `fs.rs` are not claimed: 281 calls are withheld under `slotmap::new_key_type!` at line 440. The async file handles (`smol::fs::File`, `tempfile::NamedTempFile`) are non-std resources, outside the declared table.
- **Scale remains a fired scale trigger.** The whole zed workspace does not fit in 15 GB, and a staged census that drops each representation once the next is built is the attack on it.
- **Macro expansion remains an oracle boundary.** Bounding a dependency's macro or a proc-macro derive needs its expansion.
