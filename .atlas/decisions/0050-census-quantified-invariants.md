---
id: atlas.decision.0050.census-quantified-invariants
type: decision
status: accepted
canonical: true
---
# ADR 0050 — Census-quantified invariants with three-valued verdicts (G130, NA-CONSTRAINT-NONGROUND)

## Context

ADL constraints quantify only over declared entities (`forall x: Runtime where ... require ...`) and are decided by ground derivation (ADR 0003, ADR 0007).

- **The blind spot.** An invariant over census truth, such as "for all functions of Core ...", was not expressible. The seal policy needs exactly such architecture rules: layering and forbidden authority.
- **How long it stayed open.** `DEBT-CONSTRAINT_REASONING` had not advanced since G42. Semgrep, Verus and Z3 each left it unchanged.

## Decision

1. **Syntax.** ADL gains two census-quantified forms, parsed into `ConstraintCheck::CensusForbid`:
   - `forall f: function in <Entity> forbid effect <CATEGORY>`;
   - `forall f: function in <Entity> forbid call to <Entity>`.

   A malformed form is `ATLAS-E052`, never a check.
2. **The compiler alone cannot decide them.** It reports `UNKNOWN` (`ATLAS-E063`) and never a pass.
3. **`core::composition::quantified` decides them over the composed world model:**
   - **`VIOLATED` (`ATLAS-E064`)** when a definite counterexample exists, each one named. For an effect, that is an `OBSERVED` or `DERIVED` effect site. For a call, it is a resolved call into the other entity.
   - **`SATISFIED`** only when nothing that could be a counterexample is left unexamined:
     - a call is `SATISFIED` when no Cargo dependency path exists (resolution cannot cross it);
     - or when `CALL` is `OBSERVED` on every code-bearing file and no call site resolves to, or is spelled with, a function of the target;
     - an effect is `SATISFIED` when `EFFECT` is `OBSERVED` on every code-bearing file and no `INFERRED` site of the category exists.
   - **`UNKNOWN`** otherwise, naming the residual.

   Only data and configuration files (TOML, JSON, Markdown, CSS, YAML) are exempt from the coverage condition. A script, an included Rust fragment or an unknown language can hide a function.
4. **Where decisions happen.** `runtime` decides them in one shared step (`decide_census_invariants`) that `systemize`, `graph` and `code analyze` each call before reading `adl.constraint_results`, so admission and every view agree. Composition takes its inputs as parts (`compose_input`) so this can happen before the report exists.
5. **In the world model** a decided invariant is `INV-ADL:<name>`: `DEPENDENCY` for a call rule and `AUTHORITY` for an effect rule, `DERIVED` when `SATISFIED` and `CONFLICT` when `VIOLATED`.
   - **What this exposed in G129.** The impact closure (ADR 0049) treated every `INV-ADL` invariant as reading only the file set, which is no longer sound once an invariant reads calls and effects.
   - **The fix.** An `INV-ADL` invariant is now affected whenever the file set changes or a subsystem it names has changed paths or changed calls.
6. **Atlas declares its own layering** in `.atlas/declared/system.adl`: Core calls no Adapter, Runtime or CLI; Adapter calls no Runtime or CLI; Runtime calls no CLI. All six are `SATISFIED` from the Cargo closure, and admission stays allowed.

## Evidence

A fixture test covers seven invariants over a three-crate workspace. Every verdict is identical in `systemize` and `code analyze` and is `UNKNOWN` from the ADL compiler alone:

- **`SATISFIED`** through the Cargo closure;
- **`SATISFIED`** through exhaustive call observation although a Cargo path exists;
- **`VIOLATED`** by a named call;
- **`VIOLATED`** by a named file write;
- **`UNKNOWN`** from partial `EFFECT`;
- **`UNKNOWN`** from an unresolved call spelled like a target function;
- **`UNKNOWN`** for an undeclared entity.

Admission raises both blockers, and the world model carries the invariants as `DERIVED`, `CONFLICT` and `UNKNOWN`. A counterfactual check shows that the same census with `EFFECT` observed decides the effect invariant `SATISFIED`.

Nine mutants were each caught by a test:

- the parser dropping the form;
- the compiler passing it;
- the Cargo shortcut dropped;
- the effect residual ignored;
- the call residual ignored;
- decisions never applied;
- counterexamples read in the wrong direction;
- census invariants typed `CONSTRUCTION`;
- the closure reading `INV-ADL` from the file set only.

## What stays open

`DEBT-CONSTRAINT_REASONING` rises to `M3_SINGLE_ENGINE_BOUNDED` and stays open:

- **Effect invariants.** They cannot be `SATISFIED` until `EFFECT` is exhaustive (`DEBT-EFFECT`).
- **Quantifiers.** Only the universal forbid over functions exists. There is no existential form, no quantification over state, types or calls between arbitrary sets, and no solver.
- **Oracle.** Verdicts are checked by the fixture and by the self census, not by an independent engine.
