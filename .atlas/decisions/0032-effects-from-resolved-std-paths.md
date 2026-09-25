---
id: atlas.decision.0032.effects-from-resolved-std-paths
type: decision
status: accepted
canonical: true
---
# ADR 0032 — Filesystem effects derived from resolved standard-library paths

## Context

EFFECT observed only INFERRED panic candidates (`EffectCategory::Panic` from macro spellings). Every other category stayed unmaterialized.

The miri cycle (G76, `../evidence/campaign/15-miri.json`) measured where Atlas's effects actually come from: 504 call sites reach effectful std modules (fs 352, process 63, env 50, io 20, time 17, thread 2), and none is observed. Miri's own mechanism does not apply. There is no unsafe code to interpret, and its shims model effects at the libc boundary, not at the std API.

G77 then split those calls. Filesystem effects are overwhelmingly path calls: 314 of 352 fs calls, such as `fs::write(..)` or `File::open(..)`. Process spawning happens through methods (`.output()`, `.status()`), which need receiver types.

## Decision

1. **The resolver spells external paths.** Name resolution (ADR 0031) carries a canonical external path through imports, renames, `self` imports and `Type::assoc` when the path is rooted at `std`, `core` or `alloc`. `use std::fs; fs::write(..)` becomes `std::fs::write`.
   - A registry crate or a prelude name stays external with an unknown path.
   - A glob of an external module leaves the scope open, so nothing in it is claimed.
   - A workspace item of the same name shadows as Rust does.
2. **A declared std-path effect table** (`atlas_core::std_path_effects`) names the documented filesystem entry points:
   - the `std::fs` free functions;
   - `File::open`, `File::create` and `File::create_new`;
   - the platform `symlink` functions.

   Each maps to FILESYSTEM_READ and/or FILESYSTEM_WRITE; `copy` is both. A path absent from the table declares nothing, which is never "no effect".
3. **The resolution engine derives EFFECT.** `atlas.resolution.rust-paths` is now asked for CALL and EFFECT. A path call resolved to a declared path is an effect site: DERIVED, attributed to the caller its syntactic CALL claim names, and anchored at the call. An effectful call without a claim is a diagnosed disagreement. The engine's EFFECT obligation is UNKNOWN, because method calls and every other effect source are outside it.

## Consequences

- **Effects derived:** 323 filesystem effect sites on this repository (249 writes, 74 reads). Every one names the std path that rust-analyzer 1.90.0 SCIP resolves at the same anchor, and all 323 of the table-path calls in the profile are covered.
- **EFFECT has two engines,** so the certificate's multi-engine blocker stops naming it. Each Rust artifact gains the new engine's UNKNOWN EFFECT obligation.
- **Not covered:** environment and time reads have no EffectCategory, and process spawning is method-level. Both stay unobserved, never mislabeled.
- **Falsification:** 10 mutants, all killed. One survivor (an effect derived without a claim) was a real test gap, closed first.
