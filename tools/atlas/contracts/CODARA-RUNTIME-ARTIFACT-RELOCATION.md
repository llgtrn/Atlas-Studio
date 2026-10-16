# Runtime artifact physical residence and identity correspondence

This slice physically moves the existing filesystem artifact implementation from
`adapter/runtime/src/atlasx.rs` to `adapter/runtime/src/codara/artifact.rs` inside the
unchanged `runtime` package and `compiler.runtime` owner. It switches all 16 production CLI
references to `runtime::codara::artifact`. The old physical implementation file is removed;
there is one implementation, with unchanged production bodies and effects.

The baseline is Atlas `9bbc0e130c3375cae56a917cda9e3e23d29e1d24`. Its published static census
at `tools/atlas/evidence/census/codara-residences-source-0bed61a.json` remains immutable:
all 79 file pins, 41 directed call edges, 14 entrypoint groups and 24 reference locators
were rechecked against their exact `0bed61a` Git objects. This is the starting evidence,
not a new V1 denominator or a claim that every source residence is resolved.

## Physical location is separate from logical identity

The existing logical module is retained with `#[path = "codara/artifact.rs"] pub mod atlasx`.
The new `codara` module exports `pub use crate::atlasx as artifact`. Both public routes reach
the same definitions. This deliberate physical/logical distinction preserves existing Rust
type identity, including `runtime::atlasx::ParentFiles`, and the seven historical runtime
test selectors. It does not rename every logical module or retire the old supported API.
The new route is an alias to a physically moved implementation, not evidence of full
canonical Codara convergence or target semantic adoption.

The only edits inside the moved file adjust two test-relative fixture includes by one
directory level; the referenced fixture bytes are unchanged. Package names, Cargo paths,
dependencies, declared node IDs, CLI verbs, source codec/staging semantics and wire bytes
remain unchanged. Five existing CLI test references still exercise the old public route.
Three additional public API tests compare both routes over all seven complete oracle
reports, cross-typed admission inputs/results, historical type names, and immutable
object/payload replay with conflict refusal.

## Published migration grammar

The inspected central grammar pin is `.Ynventa-`
`7d8fd333f358344dab0a76df949ae55d2bc3478e`. The existing same-crate migration example is
`apps/cli/tests/same_crate_module_migration.rs`, blob
`e0b5f38a40566a1334ca188bf2e377b9002130f0`. It distinguishes package/node custody,
physical movement, actual consumer changes, artifact identity and qualified historical
proof locators. This slice's `#[path]` stays inside the same runtime crate and must remain
inside its current compiled and censused proof closure.

`compiler.runtime` is already canonical at `adapter/runtime`, and `w01-runtime` is already
Applied. This is a submodule residence change, not a move of that declared node. Creating
a new shim that expires on that node or old wave would make it expired immediately.
No fake wave, expiry event, node, role permission or grammar extension is introduced.
The existing logical API stays supported; future retirement requires separate caller and
lifecycle evidence. The actual selected design refines the earlier census proposal's
suggestion to rename the private test prefix: the logical `atlasx::tests::*` prefix remains
correct, and a zero-match `codara::artifact::tests::*` invocation is not passing evidence.

## Identity domains and portable evidence

| Domain | Before → after | Preservation claim |
|---|---|---|
| Package / declared owner | `runtime` / `compiler.runtime` → same | Package, ownership and dependency contracts unchanged. |
| Rust logical definitions | `runtime::atlasx` → same, new `runtime::codara::artifact` route | One definition set; historical type names and public API compatibility checked. |
| Native source artifact | `art1:2f32ffa5182e57c979455b044070c72a` at old path → same ID at new path | Existing engine reconciliation observes the exact byte move before edits; no replacement ID is minted for the implementation. |
| Runtime inventory coordinate | `artifact:06414b5fa3353077` → `artifact:f103e52ef74637a3` | These are path-derived coordinates. Both remain qualified by source/path; the explicit correspondence does not rewrite old records or pretend the coordinates are equal. |
| Function observations | Old revision/span → current revision/new span | `FunctionIdentity::identity_key` contains revision and span. No blanket observation-ID stability is claimed. |
| Recensus entity descriptors | Old file-derived namespace → new file-derived namespace | Current `module_namespaces` explicitly does not see `#[path]`; actual snapshot/proof correspondence must retain this limitation and any ambiguity. Rust logical identity alone cannot prove entity descriptor stability. |
| Encoded artifacts and historical evidence | Existing IDs, bytes, digests and qualified proof locators → unchanged | New current observations are appended; old receipts are never rebound to the changed candidate. |

The actual public `adapter::inventory_source` produced the two runtime coordinate IDs over
scoped before/after fixtures. This is an observation of existing behavior, not a change to
its identity algorithm. The native registry ID is a separate identity domain.

Native reconciliation uses an isolated witness with the complete Git-index file universe
and read-only source aliases. Its `.ynventa/state` is separate from every saved checkout;
the active checkout has no reconstructed registry. Starting from the preserved 2502-row
registry, baseline sync retained all 2502 IDs, added 38 genuinely later files and retired
none. The resulting 2540-row registry then observed exactly one byte-identical move:
zero births, zero edits, zero retirements and zero ambiguous matches; its entire ID set is
unchanged. Only afterward were fixture includes, module declarations and callers edited.
The established native matching algorithm, not a hand-edited registry map, carries the ID.

Portable raw baseline/move registries and `correspondence.json` live under
`tools/atlas/evidence/census/runtime-artifact-relocation/`. They are scoped observation
fixtures, not a new canonical registry or authority schema. They qualify the native
reconciliation observation; cross-checkout durable registry ownership remains separate.
The saved 2502-row original registry, held work and all historical source proofs stay intact.

Frozen data includes `atlas_core::atlasx::materialize`, the codec/staging schema strings,
container schema/history, exact `.atlas`/`.atlasx` basenames and payloads, signature/lineage
IDs, ordered diagnostic details/nulls and checked digest-stage absence. The materializer
string is persisted identity data, not a Rust locator to modernize during this move.

## Current evidence and remaining scope

Fresh source-qualified observations must execute the seven existing `atlasx::tests::*`
runtime tests, the three public API tests, all four real CLI integration tests, the eight
relevant CLI unit tests and the exact three Core schema/history compatibility tests.
They must preserve full report bytes, exit codes, immutable replay and refusal/no-write
behavior. Actual candidate census/replay and full source self-admission are recorded
separately with exact source revisions and results; no historical pass is reused as current.
Hosted checks do not establish target/native OS or physical hardware qualification.

Within the specifically inventoried **three remaining runtime implementation responsibilities**
at the baseline, this slice relocates **one**: artifact filesystem I/O, bounded reads,
precondition/materialization/validation and codec/payload outputs. **Two** remain physically
outside the runtime Codara residence: `adapter/runtime/src/atlas.rs` (container packaging)
and `adapter/runtime/src/self_reconstruction.rs` (lift, Rust shadow and hosted verification).
The historical logical `atlasx` API remains supported. Shared design, seal, verification,
extractors, UI and other source responsibilities were not swept into this move. These
numbers describe this bounded source census, never V1 completion or native capability parity.

`FUNCTIONS-EVIDENCE-BYTES-PROPOSAL.md` remains source-only. Both target 1024-byte limits,
native byte storage/retrieval, admitted request/outcome/disclosure bindings, rights/OD-15,
native acceptance and consumer/dependency retirement remain open. Atlas is `UNLICENSED`;
this physical source move grants no downstream copying/adoption rights or authority.
