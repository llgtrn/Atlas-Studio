---
id: atlas.decision.0026.adl-consumes-census-truth
type: decision
status: accepted
canonical: true
---
# ADR 0026 — ADL consumes census truth: dependency reconciliation and census-derived ADL

## Context

Exit criterion C of the first phase (`roadmap/PRIORITY.toml`) is "ADL consumes census truth". P2 needs source → census → typed facts → ADL declarations/constraints.

Until G63, the ADL compiler checked only declared materializations against observed files. Declared `depends_on` relations were never compared with the dependency census. The authored `system.adl` was missing two things the census observes, and nothing reported either gap:
- the `adapter` → `core` dependency;
- the whole `atlas-cli` workspace member.

## Decision

1. **Observed architecture.** `ObservedArchitecture::from_closure` reads the Cargo dependency census at member level:
   - the workspace members whose own manifests were read;
   - their runtime, build and proc-macro dependencies on each other. Dev-only dependencies are test scaffolding, not architecture.
   - members seen only as a provider. Their directory is not evidenced, so they are listed as unplaced.

2. **Correspondence by path, never by name.** An entity corresponds to a member only through `materialize X { path = "<member dir>" }`. Name equality is not a correspondence (ADL-TO-ATLAS.md, "Identity and references").

3. **Reconciliation** (`reconcile_dependencies`). Every authored `depends_on` between two member-materialized entities becomes a typed `ConstraintResult`, with rule `OBSERVED_DEPENDENCY`:

   | case | verdict | code |
   |---|---|---|
   | declared and observed | SATISFIED | none |
   | declared, not observed | VIOLATED | ATLAS-E060 |
   | observed, not declared | VIOLATED | ATLAS-E061 |
   | workspace member with no declared entity | VIOLATED | ATLAS-E062 |

   A declared relation that no census frontend can check (WebUI → Runtime: `apps/studio` is not a Cargo member) is recorded as a `DECLARED_DEPENDENCY_NOT_CENSUSABLE` delta. It is never passed and never failed.

   Reconciliation runs in `gather_census_inputs`, so `systemize`, `graph` and `code_analyze` all see it. A violation blocks coding admission like any failed constraint. Only a CLOSED closure is authoritative: a partial census could omit a real edge and manufacture a violation, and it already raises its own blocker.

4. **Census-derived ADL** (`derive_census_adl`, `atlas-systemizer adl derive [--check] [--out]`). It declares what the census observes and the authored ADL does not:
   - per undeclared member: an entity (`origin = census`, `package = "..."`), its materialization with the observed language, and a materialization constraint;
   - a `depends_on` for each undeclared observed dependency.

   Every declaration cites its census evidence in a comment. The output is committed as `.atlas/declared/census.adl`. The derivation reads the authored ADL without that file, so regenerating it is a fixed point.

5. **Drift is a failure, three times over:**
   - the `census_adl_is_current` test;
   - `adl derive --check` (exit `ADL_CENSUS_DRIFT`);
   - reconciliation itself. A stale derived edge is ATLAS-E060; a new undeclared dependency is E061. Either blocks coding admission, which the self-recensus forbids as a regression.

   So a dependency change cannot land without its declared counterpart, and the ADL change must also be declared in the generation's intent.

## Consequences

- On this repository the four member dependencies are declared and SATISFIED. `census.adl` declares the two census findings, and coding admission is allowed.
- Without `census.adl`, admission is blocked by ATLAS-E061 (adapter → core, atlas-cli → runtime) and ATLAS-E062 (atlas-cli). This was verified before the file was written.
- Scope of this first bridge: member-level architecture only. External packages, symbols, effects and state are not yet lowered to ADL. Each is a later extension of the same derivation, not a new mechanism.
- Falsification: 18 mutants; 17 killed. The survivor is equivalent: `by_member` holds every observed member, so the `None` branch of the undeclared-member filter is unreachable.
