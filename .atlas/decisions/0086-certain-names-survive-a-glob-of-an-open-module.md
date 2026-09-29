---
id: atlas.decision.0086.certain-names-survive-a-glob-of-an-open-module
type: decision
status: accepted
canonical: true
---
# ADR 0086 — A certain name survives a glob of an open module (G172, replay R12)

## Context

G172 is FULL_OSS_REPLAY R12 of rust-lang/miri, pinned at `c8c5364581b0`. It is rank 1 of the recursive revalidation queue: the G76 REFERENCE_ONLY verdict has been REVALIDATION_REQUIRED since G150.

Miri's crate root glob-imports an external crate (`pub use rustc_const_eval::interpret::*`), which opens it. Nearly every module then writes `use crate::*`. Since G162, a glob of an open module opens the importer, and in an open module every glob-provided name was withheld.

At E18, Atlas withheld 2,385 path calls in 100 modules on miri. 2,106 of them, in 90 modules, were withheld for this reason alone: they were calls to names the root binds itself. For example, `use crate::*` in `concurrency/genmc/helper.rs` reads `AtomicReadOrd`, which the root re-exports by a named `pub use`.

rust-analyzer's SCIP index of miri's crate is the oracle, independent of Atlas. It binds those names to the workspace items the root names.

The rule was more conservative than Rust:
- A named import or item shadows a glob, so a name the root binds that way is the root's, whatever its external glob holds.
- In the importing module, a second glob binding the same used name to a different item is an ambiguity error (E0659), not a silent rebinding.
- Only an unseen item or named import in the importer can shadow a glob-provided name, and that happens only when the importer is open for its items (an unbounded item macro, a missing or refused file).

## Decision

1. **Two kinds of open.**
   - A module open before imports resolve, or with no import fixed point, is *items-open* (`Module::items_open`).
   - A module opened by a glob of an open or unknown source is open only for names it does not bind.
2. **Certain glob names.** A glob-provided entry records whether its source binds it certainly (`Entry::certain`): by an item or a named import, or by a certain glob of a module that is not open at all.
   - Certainty never passes through a glob of an open source. A glob-vs-glob clash reached through a re-export chain may be only the `ambiguous_glob_imports` lint in rustc, not an error.
   - An equal-priority binding of the same definition along two paths is certain only if both paths are.
3. **One question.** `uncertain_entry(module, name, entry)` replaces the four "glob entry in an uncertain module" checks: in lexical lookup, prefix resolution, type canonicalization and path-call outcome. A glob-provided name is withheld when:
   - its module is items-open;
   - an item macro there may define the name;
   - its module is open and the name is not certain.

   A name no scope binds in an open module is withheld as before.

## Consequences

- **Miri (`src/`, same pin), against its own SCIP index:**
  - INVOKES 898 → 1,029; withheld path calls 2,385 → 1,767; open modules 100 → 99.
  - Function-level edges 558 → 667, with 550 → 655 confirmed by the oracle. No edge was lost and none was found wrong.
  - All 12 edges the oracle lacks were checked in the source and are correct:
    - 6 go through impls of external rustc traits (`AllocBytes`, `Idx`), including the 4 new ones;
    - 4 are in `#[cfg(feature = "stack-cache")]` code the oracle's project does not enable;
    - 2 go through a std `Default` impl, where SCIP names the trait declaration.
  - The 13 new edges into modules the oracle does not index (cfg-gated `genmc`) were each read against the source.
- **Revalidation R6 at the exact G76 pin (`c63eee41f54d`):** INVOKES 899 → 1,030. In the shim module, 134 → 183 resolved call edges against 8,152 unresolved call sites. REFERENCE_ONLY holds.
- **Falsification:**
  - A regression test covers the miri shape: named re-exports and root items resolve through `use crate::*`; a root glob-only name and an external name stay withheld; a block's glob of an unknown enum still withholds.
  - The open-scope test now checks that a workspace glob beside an external glob resolves, while the same glob in a macro-opened module does not.
  - 4 mutants killed: nothing certain; certainty through an open source (the first version of this change, caught by the test); items-open ignored; items-open never set.
- **Capability epoch E19.** The processed replays' triggers name receiver typing, macro expansion, dispatch and resource shapes, not glob-opened modules, so every verdict stays CURRENT.
- **What stays UNKNOWN on miri:**
  - `this.method()` on the interpreter context: 1,220 of the 2,066 oracle edges Atlas leaves unresolved by name. Its type is the external `InterpCx<MiriMachine>`, and an unseen inherent method on it could win over miri's extension traits (the G167 external-declaration boundary, NA-CALL-TYPE-RESIDUAL).
  - Names that reach a module only through an external glob.
