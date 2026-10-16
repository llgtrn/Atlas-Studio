# Runtime container source residence and current readback

This source slice starts at Atlas `839be083661e1f1f3d9633cc70e3756981608066` and
physically moves all 452 lines of `adapter/runtime/src/atlas.rs` to
`adapter/runtime/src/codara/container.rs`, byte for byte. It retains the existing
`runtime` package, Cargo dependencies and declared `compiler.runtime` owner. There is
one implementation. Root declares `#[path = "codara/container.rs"] pub mod atlas`;
`codara` exports `pub use crate::atlas as container`. The supported historical
`runtime::atlas` API and Rust type identity, including `runtime::atlas::PackOutcome`,
remain intact. The historical test selector remains
`atlas::tests::the_self_census_packages_into_a_verified_unsealed_container`.

Three real production CLI calls switch to `runtime::codara::container`: `atlas pack`,
`atlas verify`, and `census certificate --atlas`. One current regression in
`agent::tests::atlas_self_world_model_is_complete_and_deterministic` changes its exact
source-span assertion to the new file. That assertion must observe the current writer's
derived SYNC record at the new path; accepting either path would conceal a stale proof.
No qualified historical evidence, census, fixture, ledger, native knowledge or proof
locator is rewritten to make old evidence appear current.

## Unchanged input and output contracts

`atlas_of` consumes the existing `SystemizeReport`, genome schema/digest and
`CensusCertificate`, and produces the existing `CensusAtlas`. `census_atlas` consumes
a repository root and performs the same two census passes and certificate computation.
`publish` encodes that value, requires exact decode equality, writes/fsyncs a temporary
file and atomically publishes it, returning the unchanged `PackOutcome`. `pack` composes
those operations. `verify` reads the complete container; its optional repository root
requires equality with the current complete census container, including its manifest.
`certificate_with_atlas` returns the existing certificate with the verified root identity.

Existing CLI verbs, argument spellings, exit behavior, `.atlas` framing, section order,
IDs, schema/history encodings, typed records, evidence and diagnostic fields are unchanged.
The source writer still emits UNSEALED; readback and integrity evidence confer no Chronica
authority. No stricter immutable-publication guarantee is invented for the existing
transactional container writer. Native staging and immutable-object semantics remain
in their existing contracts. No rights, work budget, shared wire, grant schema or UI changes.

## Identity domains and actual reconciliation

The immutable starting census remains the published 79-file/41-edge source inventory at
`tools/atlas/evidence/census/codara-residences-source-0bed61a.json`, qualified by its recorded
revision. The previous artifact move and all its raw observations remain immutable.
New current correspondence is appended under
`tools/atlas/evidence/census/runtime-container-relocation/`.

| Domain | Historical locator → current locator | Meaning |
|---|---|---|
| Package and owner | `runtime` / `compiler.runtime` → same | Same package and declared ownership. |
| Rust definition | `runtime::atlas` → same, with new `runtime::codara::container` route | Same types/functions, not a second framework or retired API. |
| Native source artifact | `art1:26e8bacde47c550020d97668881a2c85`, old file → new file | Existing engine reconciliation retains the ID on an exact byte move. |
| Runtime inventory coordinate | `artifact:d595dda1ddf8c19b` → `artifact:22fd555aa325f788` | Path-derived coordinates genuinely differ; both are source/path qualified. |
| Function/entity observations | Old revision/span/file namespace → current revision/span/file namespace | Explicit census correspondence, not invented observation-ID equality. |
| Historical container root | Old encoded bytes/root/revision → still that historical observation | A fresh source census has its own qualified current root; old receipts are never rebound. |

Both files have SHA256
`e7ee8e43baa15489ae1885bb4038ba0739d1c9d348f1662af0624729cf096f02` and BLAKE3-256
`8b015331579cae0d356bbca1c80b7bd107f202aec1abfbc69eb53a099252c458`.
The two inventory coordinates were produced by the actual public
`adapter::inventory_source` over scoped old/new fixtures. Native registry IDs are separate
from those coordinates and are not fields invented inside `CensusAtlas`.

Native reconciliation uses a fresh isolated full-Git-index witness seeded from the prior
verified 2546-row edited-source registry, retaining that entire set and the original
2502 saved IDs. Baseline reconciliation observes 2553 artifacts. The actual byte move
reports zero births, one move, zero changes, zero retirements and zero ambiguous matches;
all 2553 IDs remain. Raw engine baseline/move registries are preserved byte-exactly.
Only afterward are module declarations, callers and the current proof assertion edited.
Neither the held source registry nor the previous witness is modified, and no registry
is reconstructed under the active checkout's `.ynventa/state`.

## Current proof and fresh container readback

Public API tests use both routes over the same admitted source inputs and actual complete
container values. Real CLI tests exercise pack, verify with root, and certificate with
container; they preserve actual blockers and refusal exits rather than fabricate a CLOSED
or SEALED fixture. Corruption, changed source and Git-revision drift must refuse current
root binding. The actual full library admission covers the preserved packaging test,
updated current world-model proof, workspace tests and exact schema/history checks.
Raw outcomes must remain source-qualified, including any failed checks.

`RootManifest.revision` stores `git:<HEAD>[+dirty]`. A container produced at a tested
prepublication candidate becomes revision-stale after an evidence-only publication commit,
even when its semantic census digest is unchanged. Therefore publication must be followed
by an actual fresh `atlas pack` and `atlas verify --root` at the exact final clean published
HEAD, in the pinned environment and same absolute root. A separate readback checks the
written bytes/root, complete payload, serialized identities/evidence, current revision,
UNSEALED status and current new-path persistence record. Keep that final artifact and its
receipt outside the repository and make no further commits afterward; otherwise it must
be qualified as historical. Snapshot digest equality alone does not prove this readback.

The inherited CLI `collapsible_if` lint failure is kept separate and disclosed. It is not
silenced by this move, and library admission is not an all-checks-pass or lint-clean claim.

## Migration and remaining scope

The inspected central migration grammar at `.Ynventa-`
`a35d9faf6e9178106271eece7ef11b7dfcebcd84` retains the same-crate movement and qualified
historical correspondence rules. `#[path]` stays inside the compiled runtime proof closure.
`compiler.runtime` is already canonical at `adapter/runtime`, and `w01-runtime` is already
Applied; this slice introduces no fake wave, expiry, node relocation or authority. Old API
retirement requires separate caller/lifecycle evidence. Physical relocation is no extinction.

Of the two scoped runtime responsibilities remaining at this wave's baseline, this slice
relocates one: census container packaging, publication, readback and root certificate.
One remains outside this runtime Codara residence:
`adapter/runtime/src/self_reconstruction.rs` (lift, Rust shadow and hosted verification).
Cumulatively, two of the original three inventoried runtime responsibilities have moved.
These are source-residence counts, never V1 completion, target parity or native acceptance.
Atlas UNLICENSED/OD-15, target admission and immutable output transport remain separate
decisions. This source move grants no downstream copying or execution rights.
