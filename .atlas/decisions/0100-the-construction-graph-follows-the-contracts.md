---
id: atlas.decision.0100.the-construction-graph-follows-the-contracts
type: decision
status: accepted
canonical: true
---
# ADR 0100 — The construction graph follows the contracts (G187)

## Context

ADR 0092 made CREATION generations pick the attack whose construction node is MISSING and reachable, by vertical value. Once the AtlasX validator M13 exists (G187, ADR 0099), every remaining CREATION attack names an unreachable node: M15 requires MIN_ATLASX, M16 requires M15, and M17 requires M16. The creation-selection test would then fail with no reachable CREATION attack.

MIN_ATLASX itself cannot move without the repository owner. Its mechanism is demonstrated only on a test fixture, whose seal rests on synthetic reports, a placeholder census digest and a test-key principal. A real parent needs a declared principal and a SelectedDesign the owner signs. Atlas never signs or holds keys (ADR 0085) and the AI never selects a design. Signing is also not enough on its own: the G186 self-scope gate is NOT_ELIGIBLE on CERTIFICATE_NOT_CLOSED, COVERAGE_UNKNOWN in 13 dimensions and MULTI_ENGINE_RECONCILIATION_ABSENT.

ADR 0092 said construction work without a node (target profile, the `.atlas` DESIGN section, transformation) stays HARDENING-labelled until its node is added. A read-only investigation of the contracts found three edges that are wrong or missing and five nodes the path needs.

## Decision

1. **Edges corrected from the contracts.**
   - M15 no longer requires MIN_ATLASX. The backend compiles a validated AtlasX root: "The delegated compiler remains a compiler backend over validated AtlasX" (COMPILER-ROADMAP), and "delegated source generation MUST NOT bypass AtlasX validation" (COMPILER-IR-PIPELINE). M10 to M13 exist on the fixture; the real parent is not what the backend needs.
   - FIRST_ARTIFACT requires MIN_ATLASX. "Every materialized artifact must have transformation lineage back to its sealed root" (UNIVERSAL-ENGINEERING-WORLD). A fixture-built artifact cannot claim the milestone.
   - M15 requires a target profile (M21). The compiler input identity includes the target triple and ABI profile (COMPILER-IR-PIPELINE), and the compiler must not use host defaults for semantic target decisions (ATLASX-FORMAT). DEBT-TARGET_PROFILE's claim that the host Rust target "never needed a profile" is corrected.
   - M15 requires bodies in the root, through M20 and M19. ATLAS-TO-ATLASX refuses to materialize an unknown required function body, and RES-G183-BODIES-NOT-ENCODED reopens at any consumer of a decoded FUNCTIONS object.
   - M16 requires M15 and M22. Filesystem confinement and toolchain identity do not use the backend.
   - MIN_ATLASX requires M24, which requires M23: the SEALED `.atlas` carries its selected design (ATLAS-TO-ATLASX).
2. **Six nodes added.**
   - M19, AtlasX function bodies (DEBT-ATLASX): attack NA-ATLASX-BODIES.
   - M20, construction IR read from a validated root (DEBT-CONSTRUCTION_IR): attack NA-HIR-FROM-ATLASX.
   - M21, target profile frozen into the root (DEBT-TARGET_PROFILE): NA-TARGET-PROFILE moves to the CREATION lane.
   - M22, confined sandbox run with per-run toolchain identity (DEBT-SANDBOXED_EXECUTION): NA-SANDBOX-FS-CONFINEMENT moves from M16 to M22.
   - M23, the `.atlas` DESIGN section (DEBT-ATLAS_COMPLETENESS): NA-ATLAS-DESIGN-SECTION moves to the CREATION lane.
   - M24, real scope sealed for an owner-selected design (DEBT-SEAL_GATE). It is `blocked_on = "OWNER"`, carries the owner's exact actions and the hardening blockers, and no CREATION attack names it.
   - M16 gains its own attack, NA-CONFINED-DELEGATED-BUILD.
3. **What is not added.** Weights (perception, off the construction path) and transformation (a leaf whose staleness would take a slot ahead of the M15 chain). Either can be added by a later decision.
4. **The owner-blocked step stays in the graph, not in a third lane.** ADR 0092 fixes two lanes. The test asserts that an owner-blocked node is never named by a CREATION attack and carries a non-empty owner action.

## Consequences

- The CREATION lane keeps vertical work that needs no owner action: after G187 the reachable candidates are NA-SANDBOX-FS-CONFINEMENT (M22), NA-ATLAS-DESIGN-SECTION (M23), NA-TARGET-PROFILE (M21) and NA-ATLASX-BODIES (M19), scored by the selection rule.
- M15 becomes reachable after M19, M20 and M21, without the owner. FIRST_ARTIFACT and MIN_ATLASX still wait on the owner's M24.
- The owner's first real seal will already embed its design (M23), so it needs no re-seal.
