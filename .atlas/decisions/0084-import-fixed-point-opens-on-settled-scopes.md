---
id: atlas.decision.0084.import-fixed-point-opens-on-settled-scopes
type: decision
status: accepted
canonical: true
---
# ADR 0084 — A glob import opens its module only against settled scopes (G170, replay R11)

## Context

G170 is FULL_OSS_REPLAY R11 of rust-lang/rust. It has a sparse materialization of `compiler/rustc_hir_typeck` at the current master head (`75a75c3e0a67`). The oracle is rust-analyzer 1.90.0's SCIP index of that crate, independent of Atlas.

At E17 Atlas withheld 9 path calls in `expectation.rs` because the "glob import `super::Expectation::*` does not resolve". The oracle binds every one of those names to `expectation/Expectation#<Variant>#`, so the glob does resolve. `Expectation` is an enum defined in that same file, and the crate root imports it with `use crate::expectation::Expectation;`.

The defect is in the import fixed point (G75, G162). Each round recomputes every module's scope from the previous round's scopes. A glob whose prefix fails, or reads a crate outside the workspace, opens its module, and opening is permanent.

In round one the crate root's scope holds only its own items. The root's named import of `Expectation` has not been applied yet, so `super::Expectation` is not found and the module was opened for good.

The same happens to `use tools::*` after `use super::helpers as tools;` in the same module. In round one, `tools` falls through to the extern prelude, and the glob "reads a crate outside the workspace".

## Decision

1. **A glob's cause is taken only from a settled round.** Causes are applied only in a round in which no module's scope or shadow set changed, which is the fixed point computed on the previous round's scopes. They are also applied in a merged period-2 oscillation, as before.
2. **Opening a module starts a new round.** The next settled round judges globs again against the new open set. Opening stays monotonic, so the iteration terminates. The 64-round bound still opens every module if no fixed point is reached.
3. **A glob whose prefix never resolves still opens its module.** The cause text is unchanged.

## Consequences

- **On the R11 pin:** 9 calls move from `open-scope` to `constructor`, which is what SCIP says they are. GAP-OPEN-SCOPE falls from 13 to 12 components, and withheld path calls from 511 to 502.
- **Differential:** 1,566 of 1,566 function-level call edges agree with the oracle both before and after (precision 100%, recall 83.4%). No edge was gained or lost, because constructors are not function edges.
- **The same result at the G73 pin (`d287eb7a292c`):** revalidation R5, 510 → 501 withheld path calls, the INVOKES count unchanged at 1,609.
- **Falsification:** a regression test covers an enum's variants through the parent's named import, a module renamed by an import in the same module, and a glob that never resolves. 2 mutants were killed:
  - causes applied every round, the E17 behaviour, which fails the new test;
  - causes never applied, which fails 4 existing open-scope tests.
- **On Atlas itself:** measured with the E17 and E18 binaries over the same tree. Two test modules were open only because of round order (`core/src/census/delta.rs`, `core/src/language/adl/mod.rs`), and E18 lifts both. INVOKES rise 5,090 → 5,093; the 3 new edges were each read against the source, and no edge is lost. The G170 self-recensus is PROVEN.
- **Capability epoch E18.** The processed replays' triggers name receiver typing, macro-expanded items, dispatch and resource shapes, not import-order opening, so every verdict stays CURRENT.
