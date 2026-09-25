---
id: atlas.decision.0055.effect-envelopes-reconciled-with-census
type: decision
status: accepted
canonical: true
---
# ADR 0055 — Effect envelopes lowered into ADL and reconciled with the census (G137, NA-ADL-BEYOND-DEPENDENCIES)

## Context

`DEBT-ADL_BEYOND_DEPENDENCIES` had not advanced since G63. The ADL consumed census truth only for workspace-member dependencies (`depends_on`, `census.adl` membership; ADR 0026), and exit criterion C was left open: external packages, symbols, effects and state were not lowered to ADL.

G130 (ADR 0050) added `forbid effect`, a universal claim. That claim cannot be `SATISFIED` while `EFFECT` is partial on every Rust file, which it is today. So an effect declaration derived from the census would always be `UNKNOWN`, and an `UNKNOWN` invariant blocks coding admission (ADR 0007). A derivation of universal effect invariants would lock the repository without telling it anything new.

## Decision

1. **A new ADL form.** `forall f: function in <Entity> observed effect within <C>, <C>` (or `within none`) is the declared/observed effect reconciliation.
   - **Parsed as.** `ConstraintCheck::CensusForbid { forbidden: CensusForbidden::ObservedEffectOutside { allowed } }`. The categories are sorted and de-duplicated. A malformed list is `ATLAS-E052`.
2. **The decision** (`core::composition::quantified`) covers the definite sites only:
   - a definite (`OBSERVED` or `DERIVED`) effect site outside the envelope is `VIOLATED`, and each such site is named;
   - otherwise it is `SATISFIED`, and the basis names what the claim does not cover (the `INFERRED` sites and the files where `EFFECT` is not `OBSERVED`);
   - it is `UNKNOWN` only when the entity is not in the composed census.

   The claim is weaker than `forbid effect` by design and says so: it is always decided, and it never guesses.
3. **Derivation** (`language::adl::census::derive_effect_envelopes`, which `runtime::derive_census_adl` calls).
   - **What is lowered.** Every composed subsystem with functions and no authored envelope gets `invariant <Entity>EffectEnvelope` in `census.adl`, listing each category its functions definitely perform.
   - **No counts.** The file lists categories only, so it changes when an envelope does, not whenever a function or a site is added.
   - **Fixed point.** The model is composed from the authored ADL plus the freshly derived membership (never from the committed `census.adl`), so regeneration is a fixed point.

## Evidence

On this repository, the four Rust subsystems are enveloped and all four envelopes are `SATISFIED`; coding admission stays allowed.

| Subsystem | Envelope |
|---|---|
| Adapter | `CLOCK_READ, ENVIRONMENT_READ, FILESYSTEM_READ, FILESYSTEM_WRITE` |
| AtlasCli | `CLOCK_READ, ENVIRONMENT_READ, FILESYSTEM_READ, FILESYSTEM_WRITE` (a census-derived member) |
| Core | `CLOCK_READ` (a scaling test's timing read) |
| Runtime | `CLOCK_READ, ENVIRONMENT_READ, FILESYSTEM_READ, FILESYSTEM_WRITE` |

A fixture test covers the rest:

- **Authored envelopes are reconciled.** A violation names `save@core/src/store.rs` performing `FILESYSTEM_WRITE`. A satisfied envelope names its uncovered `INFERRED` site.
- **Derivation.** An authored envelope is not derived again. A provider-only member gets no envelope.
- **Seeded mismatch.** A new `std::env::var` read in Core, against the committed envelope, is `VIOLATED` with the site named. It blocks admission and drifts the derivation.

Five mutants were each caught by a test:

- an envelope never violated;
- `INFERRED` sites counted as counterexamples;
- `INFERRED` categories lowered;
- authored envelopes derived again;
- categories not normalized.

## What stays open

`DEBT-ADL_BEYOND_DEPENDENCIES` advances, and exit criterion C still needs three things:

- **State.** The census has no program-state declaration form yet. `INV-STATE` exists only in the composed model.
- **External packages.** External package dependencies are still not lowered.
- **Symbols.** Symbols are not lowered.

The effect envelope also inherits `DEBT-EFFECT`'s partiality: an effect the census cannot see is outside every claim, named in the basis but not checked.
