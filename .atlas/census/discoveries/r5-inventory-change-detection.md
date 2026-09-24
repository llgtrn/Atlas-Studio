---
id: atlas.discovery.r5-inventory-change-detection
type: discovery-record
status: absorbed
canonical: true
---
# Discovery record: inventory change detection (R5 slice)

Fields follow `.atlas/roadmap/DONOR-ABSORPTION-PLAN.toml`'s `[discovery_record]` schema.

## discovery_identity

`r5-inventory-change-detection`: Created, Modified and Deleted artifacts between two inventory snapshots of one root.

## provider_identity

`rust-analyzer` (`rust-lang/rust-analyzer`), `crates/vfs` only.

## provider_revision_or_version

`aaddfb73fd95f2c0bf001b474dca91ae28bcce3a`.

## provider_scope

Absorbed:

- the `Change` taxonomy (`crates/vfs/src/lib.rs:150-168`);
- the `set_file_contents` decision rule (lib.rs:216-235);
- the default of an unseen path to `Deleted` (lib.rs:300-305).

Mechanism dispositions:

| Mechanism | Disposition | Reason |
| --- | --- | --- |
| Within-cycle merge table (lib.rs:245-278) | REJECT | Snapshots have no intermediate events. |
| `FxHasher` content hash | REJECT | BLAKE3-256 is already owned natively. |
| `PathInterner`/`FileId` | REJECT | Process- and order-dependent; Atlas identity is `stable_id(path)`. |
| `Excluded` state | REFERENCE_ONLY | Close to `IgnoredByExplicitPolicy`, but a policy flip must still surface as a change. |
| `take_changes` feed, loader/notify, `FileSet`, `AnchoredPath`/`VfsPath` | REJECT | Not applicable to a complete-report diff. |

## evidence_refs

- G37 vfs lane report, with line citations for every claim (summarised in ADR 0006).
- `.atlas/genome/technology/rust-analyzer-tree-sitter-source-identity.md`. It has been corrected.
- `.atlas/evidence/verification/r5-inventory-change-detection-absorption.json`.

## root_donor_or_dependency_attribution

Root donor.

## atlas_capability_target

R5 incremental recensus: the "changed source" half.

## atlas_native_owner

- `core::census::delta`
- `runtime::systemize_since`
- `atlas-systemizer systemize --previous`

## disposition

`ABSORB_NOW`, now absorbed at mechanism level.

## decision_rationale

This is the strict prerequisite for any Salsa-shaped cache, and for measuring differential-dataflow's workload question. Vfs has no tests pinning its own semantics (its tests only seed fixtures), so Atlas wrote its own invariant tests.

## required_semantic_depth

Two properties must hold for random edit scripts:

- Replaying the delta onto the previous snapshot reproduces the current snapshot exactly.
- No path reported as changed is actually unchanged.

## prerequisites_or_blockers

None for this slice.

## verification_plan

Executed. See the evidence file.

## dependency_removal_plan

None needed. The donor was never a dependency.

## extinction_implications

The checkout stays for now. It is a DC1 real-donor Cargo corpus, and its HIR and name-resolution surface is not absorbed.
