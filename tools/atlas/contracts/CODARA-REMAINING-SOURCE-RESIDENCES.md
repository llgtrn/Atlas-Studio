# Read-only Atlas Codara source census

Source: `llgtrn/Atlas-Studio` main at **`0bed61abbf05df6b0a323cfcaf7013e371548408`**. This is a bounded source inspection, not a canonical semantic census, target parity assay or native completion result. No repository edits, Cargo tests, heavy self-census or live effects were performed.

The existing pure compiler/artifact implementation is already under `core/src/codara`. The smallest remaining structural candidate is the real filesystem artifact adapter in `adapter/runtime/src/atlasx.rs`; moving a namespace wrapper alone would not move that implementation. Target output/evidence design is a separate lane.

## Instructions and ownership

No applicable `AGENTS.md` or `.agents` skill was found at the workspace/repository root or scoped paths. Current source contribution guidance and the declared Core/Runtime/CLI package ownership were read. The requested original Blueprint at `7828a94acc0a9f94029eac7e0e4439180e3c0e43` was read locally; current central main was independently resolved to `7fe659cdc755afde233b420de4ae814ea2c6b0ce`, with Blueprint blob `5f9c3266519216229fd91c2e0f0f6be137ae4c7b`. Its Codara section keeps canonical typed intent/compiler ownership in Chronica and distinguishes Atlas physical source facts from native adoption. Compiled candidates confer no authority. These pins are observations, not instructions to synchronize other repositories.

`.ynventa/declared/repository.rs` owns `compiler.core` at `core` and `compiler.runtime` at `adapter/runtime`; `tools/atlas/declared/system.adl` similarly separates pure Core semantics, Adapter observation and Runtime orchestration. Shared design/seal/verification and source-extraction boundaries should not be swept into a compiler namespace solely because they consume compiler evidence. No auxiliary UI work is proposed.

## Residences

| Residence | Current role | Bounded count / status |
|---|---|---|
| `core/src/codara/adl/**` | Legacy `atlas 1` ADL lexer/parser/compiler and census bridge; root `atlas_core` re-exports remain real caller APIs | 3 tracked files |
| `core/src/codara/construction/**` | Pure construction IR, validation/identity, bounded HIR and Rust emission | 3 tracked files; target remains `RUST`, not native activation |
| `core/src/codara/artifact/**` | Historical census container/schema history, FUNCTIONS codec, root manifest and pure sealed-parent staging | 48 tracked files including fixtures/tests; no target wire selection |
| `core/src/codara/mod.rs` | Existing pure source ownership root | 1 file; total pure Codara tree **55 files** |
| `adapter/runtime/src/atlasx.rs` | Filesystem METHODS: declared-parent inputs, stage/read-root/validate, immutable encode/decode/payload export | 1008 lines / 47385 bytes; actual implementation still outside a runtime Codara residence |
| `adapter/runtime/src/atlas.rs` | Repository census → legacy .atlas packaging/readback/publish/verify/certificate | 452 lines / 17204 bytes; actual implementation still outside a runtime Codara residence |
| `adapter/runtime/src/self_reconstruction.rs` | Typed records → bounded construction lift/Rust shadow and hosted verification orchestration | 1606 lines / 63226 bytes; actual implementation still outside a runtime Codara residence |
| `apps/cli/src/main.rs` | Actual command dispatch and JSON/exit/output contracts | Calls the current public runtime modules directly |
| `apps/cli/tests/full_evidence.rs` | Real binary process caller oracle | Seven complete reports plus payload success/replay/refusal evidence |
| `adapter/src/source/frontend.rs` and runtime extraction dispatch | Bootstrap `.cdr`/`.codara`/`.py` recognition and explicit unsupported obligations | Family recognition is L2 admission, not native Codara/Python parsing/lowering; do not relocate these shared adapters as if they were a compiler |


The machine inventory includes all 55 pure-tree files and 24 adjacent boundary/caller/documentation files with exact Git blobs, SHA256, sizes and source pins, all read from immutable `git show 0bed61…:PATH` contents. Concurrent parent-authorized working-tree changes are excluded. Test/fixture source locations are indexed separately from remaining production implementations.

## Actual production caller edges

The following relations were verified from dispatch and call expressions, without executing them. The JSON report records **41 directed calls across 14 entrypoint groups**, including exact file/line locators. Its navigation-stage lists may branch; they are not asserted to be linear runtime traces.

| Existing entrypoint | Real production edges / boundary |
|---|---|
| `parse` | CLI → `runtime::parse` → `adapter::read_adl_sources` and `atlas_core::codara::adl::parse_adl_source` |
| `systemize`, `graph`, `code analyze` | Actual runtime entrypoints → shared `gather_census_inputs` → `atlas_core::codara::adl::compile_adl` via the existing root re-export; source observation and extraction remain separate inputs |
| `adl derive` | CLI → `runtime::derive_census_adl` → pure ADL census derivation, with dependency closure reconciliation |
| `atlas pack` | CLI → `runtime::atlas::pack`; one branch calls `census_atlas` → `atlas_of`, the other `publish` → pure container `write/read` equality → transactional output |
| `atlas verify`, `census certificate --atlas` | CLI → `runtime::atlas::verify`; certificate route calls `certificate_with_atlas` → `verify`; container read and optional exact current census comparison remain required |
| `atlasx precondition/materialize/validate` | CLI → matching `runtime::atlasx` function → pure `codara::artifact::staging` callback over exact file bytes; materialize additionally stages/readbacks objects and manifest |
| `atlasx codec --encode` | CLI → `runtime::atlasx::encode_module` → pure artifact encode; immutable addressed-object create/replay |
| `atlasx codec --decode` | CLI → `decode_object` → `decode_at` → pure `decode_named`; full typed report and immutable report-output contract |
| `atlasx codec --codara-payload` | CLI → `export_codara_payload` → named pure decode; only DECODED reaches immutable `write_object_or_replay` with original `bytes[80..]` |
| `self-reconstruct candidates/roots` | CLI → `runtime::self_reconstruction::candidates` or `target_roots`; roots also read a verified container through `runtime::design` |
| `self-reconstruct attempt` | CLI → `attempt`; it calls `lift`, pure construction Rust `emit`, `write_shadow` and `run_differential` as distinct stages; source oracle reads belong only to verification |
| Shared design/seal | `runtime::seal::gate_container` → `runtime::design::read_container` → pure census-container `read`; these preserve policy/authority/integrity distinctions |


Within this inspected workspace, the direct production `runtime::atlasx::*` consumers are in CLI dispatch/argument projection; other appearances include tests. `runtime::atlas` and reconstruction are likewise real CLI-backed source surfaces. This statement is workspace-scoped and does not assert that downstream consumers outside the repository do not exist.

## Identities, provenance and compatibility routes

The machine inventory attaches historical saved-workspace `art1:` IDs where available from the retained `artifacts-final.jsonl` manifest. That manifest is historical relocation evidence, not a fresh native census; several source files have since changed and their recorded old digests are explicitly distinct from the newly measured current hashes.

| Current file | Retained historical saved-workspace ID |
|---|---|
| `adapter/runtime/src/atlasx.rs` | `art1:2f32ffa5182e57c979455b044070c72a` |
| `adapter/runtime/src/atlas.rs` | `art1:26e8bacde47c550020d97668881a2c85` |
| `adapter/runtime/src/self_reconstruction.rs` | `art1:5840ab4120952af87788c8ac9c138fe1` |


Runtime inventory IDs are a separate domain: `adapter/src/source/mod.rs::artifact_id` hashes the relative source path. A filesystem move alone therefore changes new runtime inventory identity/provenance; it does not automatically preserve every identity domain. Any later move must freeze and account for both domains using the established engine-owned ID migration/relocation mechanism, retain qualified old locators and never rewrite historical census/container evidence. No identity-schema change is proposed. If that preservation map cannot be made explicit, the move is not ready.

Preserve the container schema/history, section IDs and digests, exact `.atlas`/`.atlasx` bytes and basenames, signature/lineage IDs, construction module/report identity preimages and serialization ordering. `module_identity` and `report_identity` clear their own ID before hashing compact serialization; path-derived targets/oracle references must remain qualified by the source revision that produced them.

The persisted `MATERIALIZER_IDENTITY = atlas_core::atlasx::materialize` is opaque historical data. It is not a live Rust module route and must not be “fixed” during a source relocation. Likewise, keep `atlas.construction-ir.v1`, `atlas.self-reconstruction-report.v1`, `atlas.construction.rust-backend.v1`, all codec schema names, original fixtures, full ordered diagnoses/nulls and existing receipt history. BLAKE3 identities cannot be substituted with BLAKE2/SHA256. Rights remain `UNLICENSED`/OD-15 unresolved; source relocation grants no target adoption rights.

| Route or locator | Actual status |
|---|---|
| `atlas_core::codara::{adl,artifact,construction}` | Current pure source module paths |
| Root `atlas_core::{compile_adl,parse_adl_source,...}` | Existing retained ADL re-export API, still used in production; no Rust deprecation attribute observed |
| `runtime::{atlas,atlasx,self_reconstruction}` and historical CLI verbs | Current supported source interfaces with historical naming; no deprecation enforcement observed, so do not silently remove them |
| Historical `core/src/{language,atlas,atlasx,construction}` source paths | Qualified migration/history locators; those old Core modules are not currently declared in `core/src/lib.rs` |
| `atlas_core::atlasx::materialize` persisted identity | Frozen materializer data only, even though the implementation now lives under Codara |
| Old pinned test commands in immutable evidence | Historical instructions for their named source; module moves require new current selectors and actual nonzero execution evidence |

## Smallest eligible structural proposal

Recommend one bounded **physical relocation of the existing runtime artifact implementation** from `adapter/runtime/src/atlasx.rs` into `adapter/runtime/src/codara/artifact.rs`, within the same runtime package. Add only the necessary module declarations, switch current CLI production calls to `runtime::codara::artifact`, and retain `runtime::atlasx` as a compatibility re-export of that same implementation. A retained old-locator shim can be used if required by the reviewed identity plan; it must not leave two implementations or be described as full convergence. No move is performed by this report.

This is smaller and more independently testable than moving container packaging or the much larger reconstruction orchestration simultaneously. It moves real file-effect code and actual consumers, while preserving `ParentFiles`, `inputs`, public type/re-export identities, bounded reads, immutable replay/refusal behavior, source precondition semantics, JSON reports, CLI flags and exit codes. It neither designs a native object output nor bypasses the shared lossless-evidence decision.

Keep `atlas.rs`, `self_reconstruction.rs`, design, seal, verification, shared extractors and UI unchanged in the first slice. They remain explicitly inventoried follow-up residences; root/container packaging and reconstruction require separate caller, oracle-path and identity review.

Required evidence before that proposal could be published:


1. Freeze exact source/fixture blobs and the reviewed saved-workspace ID/path map; document the separate runtime path-derived inventory identity impact. Preserve every historical encoded record and receipt.

2. Compile both the new runtime API and retained `runtime::atlasx` API against identical types/results. Switch the actual CLI calls to the new path; a namespace alias over unmoved code is not sufficient.

3. Run all seven existing artifact-runtime tests under their new exact module selector, including bounded object/root reads, write conflicts/replay, named decode, full corpus reports and staging rollback. Require seven executed/passed tests, zero failures/ignored; zero matches under retired `atlasx::tests::` is not passing evidence.

4. Run the complete current real CLI `full_evidence` integration suite (two tests at the inspected baseline, subsequently extended by the capacity work), the codec decode-output alias/conflict tests, existing encode/decode/payload caller regressions and the sealed-parent materialize/validate caller test. Preserve all seven cases, full JSON, stdout, exit, payload and refusal/no-write behavior.

5. Run the current schema/history compatibility producer with its exact three required Core tests; any source relocation changes the candidate/provenance, so produce current pinned evidence rather than relabeling earlier results.

6. Run formatting/compile checks and established native protocol/identity-preservation checks, then the required fresh actual self-admission workload after a coherent source commit. Hosted success remains scoped development evidence, not Chronica target or native hardware qualification.


No executable implementation, target receipt version, authority interface, rights grant or full V1 result is selected here. Remaining target mapping and rights obligations stay open independently of this source-only structural proposal.

## Deliverables and limits

`tools/atlas/evidence/census/codara-residences-source-0bed61a.json` contains exact file pins,
source/Blueprint pins, directed callsite records, historical-ID qualifications and the
proposal. This report is a human-readable view of those scoped observations. All 79 pins
and caller/reference line locators are taken from the immutable baseline commit, independently
checked against Git objects. Concurrent capacity changes and the new lossless-byte test
fixtures are outside that baseline and are not presented as baseline source facts. They
add no production Codara module. This is not an executable semantic test.
