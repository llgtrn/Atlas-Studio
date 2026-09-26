---
id: atlas.decision.0064.selected-design-and-selection-authority
type: decision
status: accepted
canonical: true
---
# ADR 0064 — SelectedDesign and its selection authority event (G148)

## Context

Construction node M7 ("SelectedDesign with selection authority event", DEBT-SELECTED_DESIGN, DEBT-IDENTITY_AUTHORITY) is next on the FIRST_ARTIFACT critical path. Its prerequisite M1 has existed since G147: a verified census container carries every typed record.

`contracts/SELECTED-DESIGN.md` requires four things:
- a design names its parent Atlas root and the semantic roots it selects, and those roots must resolve;
- a SELECTED design rests on validation evidence;
- a SELECTED design needs an authority event. HUMAN_REQUIRED needs "an authorized human selection event", and "no provider may grant itself selection authority";
- the identity is a digest of what the design selects, never its display name.

No principal, authority event or design record existed.

## Decision

1. **`core::design`.**
   - Typed lifecycle: `DesignState` (CANDIDATE, VALIDATED, SELECTED, REJECTED, SUPERSEDED). It is registered as a vocabulary carrier and is never an epistemic status.
   - Types: `AuthorityMode`, `PrincipalKind` (HUMAN, POLICY, PROVIDER), `BindingKind`, `SemanticRoot { dimension, record_id }`, `AuthorityEvent`, `SelectedDesign`.
   - `design_identity` is a BLAKE3 digest over schema, parent root, candidate, scope, target kind, variant, sorted roots and sorted bindings. State, rationale, evidence and authority are facts about a design and stay outside it.
   - `event_identity` covers principal, mode, design, generation and statement.
2. **`validate(design, container, root_id, report, registry)`** returns typed violations:
   - the recorded identity must be recomputed;
   - the parent root must be the verified container's root, and the candidate that container's census digest;
   - at least one root, each resolving to a typed record of that record id and dimension in the container;
   - the candidate set includes the design;
   - VALIDATED and SELECTED need evidence that references a report which admits this exact candidate;
   - only SELECTED carries an event. That event must name this design and its mode and must match its own identity, and its principal must be declared in the registry;
   - HUMAN_REQUIRED and HYBRID need a HUMAN principal, and a PROVIDER never selects;
   - POLICY_AUTO is refused while no bounded policy envelope is defined;
   - no UNRESOLVED_BLOCKER binding may be selected.

   `coordinate_conflicts` allows one non-superseded SELECTED design per scope, target kind and variant.
3. **Principal registry.** `.atlas/declared/principals.json` is declared by the repository owner, and a provider never adds itself. It is committed empty, so nothing can be SELECTED until the owner declares a principal.
4. **Runtime and CLI.**
   - `runtime::design` reads the container from its bytes, so the root identity is recomputed and never trusted. It also reads the report (bare, or wrapped as `verification self` writes it) and the registry.
   - `atlas-systemizer design roots --function` lists roots by name from a verified container.
   - `design propose` produces a VALIDATED design. `design check` exits DESIGN_REJECTED unless the design is accepted in its state.

## Evidence

`evidence/design/G148-selection-boundary.json`:
- **Proposal.** A VALIDATED design over the G147 container selects the container builder and its publisher (`atlas_of`, `publish`). Its roots were found by name in the verified container and its evidence is the G147 self-scope report. It is ACCEPTED.
- **Attempted selection.** The same design SELECTED by the proposing provider is REJECTED: PRINCIPAL_IS_PROVIDER and PRINCIPAL_UNREGISTERED.
- **Tests.** Core tests reject every falsification: an absent root, a root under another dimension, another parent root, another candidate, a tampered binding, blocked, missing or foreign evidence, a missing event, a provider, a policy principal under HUMAN_REQUIRED, POLICY_AUTO, an event for another design, another mode, an edited event, an unresolved blocker, an event on an unselected design, and a coordinate conflict. A runtime test runs propose and check end to end over a published fixture container.
- **Mutants.** 10 mutants are killed.

## Consequences

- **What M7 provides.** M7 exists as a mechanism: a design can be proposed, validated and checked, and a selection is accepted exactly when a declared human principal's event commits to it.
- **User authorization.** Selecting is the repository owner's decision. The AI proposes and never selects. This is a user-authorization boundary, not missing effort.
- **Deferred:**
  - principal authentication beyond the declared registry (for example signed commits);
  - the bounded policy envelope for POLICY_AUTO;
  - DecisionProposal and ProviderReceipt lineage;
  - `IntegrityEnvelope.selected_design_ref`, which stays unset while no design is SELECTED.
- **Next on the critical path:** the seal policy (M3, M4) and the seal eligibility gate (M8).
