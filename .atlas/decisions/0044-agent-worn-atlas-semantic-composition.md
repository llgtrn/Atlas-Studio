---
id: atlas.decision.0044.agent-worn-atlas-semantic-composition
type: decision
status: accepted
canonical: true
---
# ADR 0044 — Agent-Worn Atlas: semantic composition and the agent interface (G122)

## Context

By G121 the census held more than 100,000 typed records across twelve dimensions. The AI building Atlas still understood its work by reading source files, and ran Atlas afterwards to verify the change. Nothing composed the records into anything larger than a record. For example, no model could answer:

- which subsystem owns this state;
- where filesystem authority originates;
- what reaches this function;
- why A depends on B.

The directive of this generation makes that composition essential: the AI must reason *through* Atlas.

## Decision

1. **A new essential debt, `DEBT-SEMANTIC-COMPOSITION` (P0).** Its first attack, `NA-SEMANTIC-COMPOSITION`, lands in G122 as `core/src/composition`. Composition builds five levels:
   - function behavior;
   - component behavior;
   - subsystem model (declared ADL entities with materialization paths);
   - capability model;
   - architecture model.

   Around them sit typed relations (`INVOKES`, `SUPPLIES_DATA`, `STATE_FLOW`), typed invariants (`STATE`, `DEPENDENCY`, `AUTHORITY`, `SAFETY`, `CONSTRUCTION`; `ORDERING` and `RESOURCE` stay unrepresented gaps) and six `UnderstandingGap` records routed to debts.
2. **Epistemic discipline** (see `contracts/AGENT-WORN-ATLAS.md`):
   - purpose only from a declaration;
   - universal claims `INFERRED` with named residuals unless their dimension is `OBSERVED` in scope;
   - dependency invariants `OBSERVED` from the Cargo closure and turned into `CONFLICT` by any counterexample call;
   - "no path observed" is `UNKNOWN`;
   - causation is never claimed.
3. **The agent interface.** `atlas-systemizer agent` exposes `understand` (a bounded `MissionContext` whose structural digest excludes the mission text), `explain`, `impact`, `trace`, `why`, projections, `compare`, `plan`, `hypothesis` and `verify`. `resources` and `causal` answer with their owning gap. The operations read the composed model only: no text search.
4. **Composition is recensused.** The census snapshot carries `composed:*` totals (objects per level, relations per kind, invariants per kind and status, dependency verdicts). A change in composed understanding is therefore a census change that the self-recensus must see intended.
5. **Generation classes and ordering.** A generation is one of:
   - `NATIVE_ATTACK`;
   - `COMPOSITION_ATTACK` (a native attack on `DEBT-SEMANTIC-COMPOSITION`);
   - `AGENT_MISSION`;
   - `REVALIDATION`;
   - `DEBT_TRIGGERED_DONOR_ATTACK`;
   - `AUDIT`;
   - `DONOR` (still blocked).

   The rules are machine-enforced in `runtime/src/lib.rs` (`essential_complexity`):
   - the pressure-selected queue head is planned for the next generation, or the one after it when exactly one `REVALIDATION`, `AGENT_MISSION` or `DEBT_TRIGGERED_DONOR_ATTACK` generation is planned first;
   - no more than two consecutive generations are outside the native classes;
   - once the interface exists (G122), an `AGENT_MISSION` must occur at least every five generations.

   Following the directive's order, G122 bootstraps composition ahead of `NA-QUANTITY-IN-CORE`. G123 is the first agent mission on Atlas itself. `NA-QUANTITY-IN-CORE` stays the queue head, planned for G124.
6. **`DEBT_TRIGGERED_DONOR_ATTACK`: the only donor work allowed while new-donor progression is blocked, besides triggered revalidation.** Each one must:
   - name an open essential debt and a falsifiable hypothesis;
   - pin an exact commit and materialize a bounded scope;
   - produce a mechanism decision and delete the source again with a post-delete self-recensus.

   One donor at a time. Frontier counting is never a motive.
7. **Revalidation recomputed.** A new capability milestone, `SEMANTIC_COMPOSITION` (G122, debt `DEBT-SEMANTIC-COMPOSITION`), was added to `RECURSIVE-DONOR-REVALIDATION.toml`. No audited donor's historical verdict depended on that debt, so no status changed. Historical code-understanding donors (sourcetrail G72, joern G69) are candidates for a debt-triggered attack on `DEBT-SEMANTIC-COMPOSITION`, not for revalidation. Their verdicts answered other questions.

## Findings of the first composition of Atlas itself

- **A false conflict from a direct-edge check.** A direct-edge check reported `AtlasCli -> Core` as an undeclared dependency: 19 resolved calls reach Core functions through Runtime's re-exports. The verdict is now `TRANSITIVE_VIA_REEXPORT` (`DERIVED`) when a Cargo path exists. `CALL_WITHOUT_DEPENDENCY` (`CONFLICT`) is reserved for calls with no dependency path. The resolution engine was shown never to resolve across a missing dependency.
- **Unplaced members.** A workspace member seen only as a Cargo provider has no evidenced directory (`ObservedArchitecture::unplaced_members`). It is reported, never placed by its name.
- **What the model cannot answer yet.** Every `STATE` and `AUTHORITY` invariant of Atlas is `INFERRED`: `STATE` and `EFFECT` coverage is `UNKNOWN` on every file, and 19,722 call sites are unresolved. The model says so instead of claiming ownership.

## Falsification

Thirteen mutants were each caught by a test:

- signatures joined by their own record id;
- unresolved calls not counted;
- state writer sets always `DERIVED`;
- an effect site keeping the first engine's status;
- every undeclared call treated as a re-export;
- dependency counterexamples ignored;
- a path prefix without a separator;
- mission text in the structural digest;
- "no path" reported as observed absence;
- extra observed writers ignored;
- a built undeclared dependency accepted;
- purpose named from a module identifier;
- the impact frontier without its lower-bound residual.

The effect-status mutant survived the first run and is now killed by an order-independence property.

## Consequences

- The next generations use `agent understand` / `impact` / `verify` while coding. Each mission records what Atlas knew, what stayed `UNKNOWN`, and what source the agent read manually. Gaps route to debt.
- `DEBT-IMPACT_CLOSURE` is not advanced by this generation: `impact` is a call-graph lower bound, not the census affected-set equality that debt requires.
