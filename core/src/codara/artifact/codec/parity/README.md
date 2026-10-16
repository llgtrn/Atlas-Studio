# Codara payload and legacy FUNCTIONS parity corpus

`codara-parity.cdr` is a current Codara 1 source program for the existing governed digest flow.
`codara-parity.payload` is its exact raw input byte oracle, not a new container format.
The thirteen objects under `legacy/` are synthetic, bounded historical AtlasX v1 FUNCTIONS
inputs for a native replacement's tests. They are never presented as canonical Codara output.
They are not construction output, historical function identities, resolved lineage, capsules,
authority or admission evidence. No function body or live effect is included.

The source contract is `core/src/atlasx/codec.rs` at Atlas-Studio revision
`b87472dee45b89e8dda355aad8c528b7b4cc64d4`, with the unchanged schema table
`FUNCTIONS_SCHEMA` (class 4, schema 1). The existing consumer is
`apps/cli/src/main.rs` → `adapter/runtime/src/atlasx.rs::export_codara_payload` →
`decode_named`. The workspace license is `UNLICENSED`; these owner-source test materials
carry no new license or downstream rights grant. A target transfer must retain exact source
and byte provenance and resolve its existing rights obligations.
The codec source SHA256 is `636464f0c41053fd37b6f796590f098623456fd7ea5d925840458545620f3ccc`.

The current source locator is `core/src/codara/artifact/codec.rs`; the complete codec fixture
subtree moved from `core/src/atlasx/codec/` to `core/src/codara/artifact/codec/` with its
saved-workspace ArtifactIds. The sibling pure root-manifest codec moved from
`core/src/atlasx/manifest.rs` to `core/src/codara/artifact/manifest.rs`, together with its tests.
The pinned path, revision and digest above describe historical provenance. Fixture basenames,
content bytes, encoded IDs and historical materializer identity are unchanged.

Naming is resolved from Chronica's CN-0005 and DECISIONS 0044, 0115 and 0125 at
`d9e8a067f18cdb931715027631a2fb60125a6166`: Codara's native owners are
`physiology/capability/codara` and `phenotype/codara`; the current source examples use `.cdr`.
CHGC capsules and their existing SHA256 identity domains are a separate native format.
No approved native FUNCTIONS object extension or conversion into Codara semantic IR was found.
Such a migration needs a coordinated versioned contract and consumer proof, not a suffix change
to ATLSX bytes. Legacy format names/addresses remain confined to legacy input and evidence.
The older committed `fixture_verify.codara` retains its historical basename in the relocated codec subtree;
new source programs in this corpus use `.cdr`.

Recommendation for the next naming decision: keep `.cdr` for text source, and give a new
compiled object representation a distinct suffix only with its approved semantic contract,
version, magic, identity preimage and old-to-new parity rules. `.cdx` is a compact candidate;
`.codarax` is a more explicit candidate. Neither is selected or emitted here. The existing
Codara capsule writer already emits CHGC bytes to an operator-supplied path; it is not a
FUNCTIONS object writer. A FUNCTIONS migration affects the source `encode_functions` and
`ObjectHeader::address`, runtime `encode_module`/export/staging/root validation, CLI codec
commands, and the target legacy-artifact reader and consumers. All must migrate together with
their filename checks, manifest entries and provenance; existing content-addressed input stays
available under the explicit legacy contract.

`cases.json` is a test-only list, not a new wire or admission contract. It pins each complete
legacy object's filename and SHA256, expected ordered `CodecDefect` names, and expected decoded
`FunctionSignatureRecord` values. Expectations were specified independently of decoder output.
The files were framed directly from the existing record/field table; their payloads were then
sealed with the existing `ObjectHeader::functions`. Each header, payload length, BLAKE3 digest
and address filename is valid, including every negative example. No old object was changed.

The positive case has two sorted signatures: Unicode text; empty optional text distinct from
absent text; parameters in declaration order (`z`, then `a`); sorted lineage; dispatch 1 and 3;
and a 32-byte fingerprint. The negative cases isolate unknown and overlong dispatch values,
missing lineage, unknown optional/required fields, reversed signature/lineage order, repeated
identity, a wrong embedded record kind, truncated field/record framing and an empty object.
The core test compares explicit values, canonical encoding, an independent BLAKE3 oracle and
refusal evidence clearing. Runtime and CLI tests use these same complete objects to check
exact payload export/replay and refusal without output creation or replacement of prior evidence.
This is a bounded corpus; existing codec tests cover further header, depth, field and value limits.

The source input is the complete `<lowercase BLAKE3(payload)>.atlasx` file. The existing
legacy decoder handles that format only; `.atlasx` is not Codara's canonical new output.
The existing `CodecReport` supplies semantic records only on `DECODED`, with unresolved owner/type/lineage
strings carried unchanged; on `REFUSED`, its semantic and address fields are absent. Header
defects precede bounds/hash checks, then framing, schema/value checks, canonical re-encoding
and finally filename verification. See the source table and tests for exact field flags and bounds.

The legacy export command writes only bytes after the 80-byte header. The positive case's
441-byte output must equal `codara-parity.payload`; `codara-parity.cdr` claims
`86e6111beb161e9b7241cf136ad8738eed8bbd3872707d6511dbcc192d6296d2`.
For Chronica's `codara run --payload FILE CAPSULE`, the claim is the header's 64-hex payload digest and the
input bound is the payload byte count. BLAKE3 comparison can match all thirteen payloads;
it cannot prove FUNCTIONS decoding. The generic `legacy_artifact::verify_atlasx_object`
checks header/bounds/hash/address only. Chronica `848149d0c2d2d8b8923b394533879faabc5b6fb1`
also contains the separate strict `verify_atlasx_functions`/`read_atlasx_functions` and governed
`legacy.functions-object` caller. Their presence does not establish complete source equivalence,
engine native qualification, consumer cutover or admission. Root manifest, cross-object resolution
and admitted materialization remain separate obligations even for the positive case.

`codara-functions.cdr` prepares the positive case for that strict caller: goal
`artifact.legacy-identity-verify`, `input_bytes = 521` and the exact historical `file_name`.
Pass the original complete `legacy/86e6111beb161e9b7241cf136ad8738eed8bbd3872707d6511dbcc192d6296d2.atlasx`
as `codara run --payload FILE CAPSULE` input. Do not pass the 441-byte raw export to this reader.
The target world's catalogue must actually select `legacy.functions-object`; the program does
not supply a binding or grant. Successful kernel output is the 441-byte payload. The target's
persisted `codara.legacy.functions.v1` result binds full-object, filename and IR evidence; it is
not the source `CodecReport` wire contract. The older `codara-parity.cdr` remains the separate
raw-payload digest flow, with a digest claim rather than a `file_name` fact.

Chronica owns the actual authenticated requester, current resource and lease/work bindings,
one `cpu_units` demand, budget/capacity/envelope checks, payload binding to durable work,
bounded output evidence and fail-closed retries. This corpus adds no shared wire fields,
authority schema, permission, host invocation, runtime source dependency or consumer cutover.

## Native reader handoff

The receiving implementation belongs to Chronica's native capability owner. This source
corpus supplies a bounded oracle, not another reader or a runtime source dependency.
FUNCTIONS has class 4/schema 1. Its 80-byte header uses format 1.0, BLAKE3 digest id 1,
codec NONE (0), equal encoded/decoded lengths, zero flags/reserved and a 64 MiB payload limit.
The payload contains version-1, zero-flag records with 16-byte headers; fields have 8-byte
headers. Field tags strictly ascend. Nesting is limited to depth 8.

| Record kind | Fields in tag order (`!` means required flag bit 0) |
|---|---|
| 1 FUNCTION_SIGNATURE | 1 function_id GLOBAL_ID!, 2 name UTF8!, 3 owner UTF8, 4 dispatch UVARINT!, 5 visibility UTF8!, 6 documentation UTF8, 7 params RECORD, 8 result UTF8, 9 body_fingerprint HASH32, 10 lineage RECORD! |
| 2 PARAM, embedded only | 1 name UTF8!, 2 type_spelling UTF8! |
| 3 LINEAGE_REF, embedded only | 1 record GLOBAL_ID! |

Dispatch codes are 1 free function, 2 inherent method, 3 associated function; integers must
be minimally encoded. GLOBAL_ID is nonempty UTF8, carried without target identity resolution.
Function names must also be nonempty. Required visibility and parameter name/type text can be
empty; the source does not impose target identifier/type syntax on these carried strings.
HASH32 is exactly 32 bytes. Parameters retain declaration order; function records sort by
identity bytes, while lineage sorts/deduplicates as a set. Empty optional sequences are absent;
empty optional text remains distinguishable from absent text. Minor 0 rejects unknown fields
even when marked optional. Canonical re-encoding must reproduce the complete input exactly.
The source report clears semantic/address fields on refusal and retains its `not_verified`
limits. Matching these thirteen cases is necessary bounded evidence, not full equivalence.

The source regression `multiple_duplicate_identities_keep_distinct_diagnostic_evidence` adds
the `[a,a,b,b]` counterexample: source retains two identity-specific DUPLICATE_IDENTITY defects.
The strict target at `848149d0` collapses those to one generic diagnosis after deduplication.
Both refuse; exact diagnostic multiplicity is a remaining compatibility decision. Source clears
semantic/address report fields on refusal, while target retains checked expected/computed
evidence. An explicit output mapping must preserve that evidence rather than claim wire parity.

## Accepted source subset and callers

Current engine `b1d8c58966cfcaad60f2c979c4258b585009d26a` accepts
`codec.executable-world-objects` only as the bounded historical FUNCTIONS contribution described
by census fact `b9939df4b81e68134b4037b8`, pinned to source
`fc8dbc4ee7c9cc94c50375672966f24b5ff1bbf7`. Package
`fc3f98883297411649967d46` is WAITING_OWNER; source metadata does not choose its target route.
The thirteen pinned cases, object bytes and expected values remain unchanged.

| Source caller | Actual contract |
|---|---|
| `core/src/codara/artifact/codec.rs::encode_functions`, `read`, `decode_named` | Pure canonical class-4/schema-1 signature codec, typed refusals and historical basename checks; no function bodies or target identity/authority resolution. |
| `adapter/runtime/src/atlasx.rs::decode_object`, `decode_at` | Bounded file read followed by strict named source decoding. |
| `adapter/runtime/src/atlasx.rs::export_codara_payload` | Read once, validate the complete named object, then write bytes after header through create-or-identical replay; refused input creates no output or parent and preserves prior evidence. |
| `apps/cli/src/main.rs` codec dispatch | Existing encode/decode/export commands call the source runtime. These are source callers, not a Chronica integration trace. |

The existing `compiler.core` provides and `compiler.runtime` requires declarations describe
that local source relationship. They declare no new technology, target native owner, permission,
cross-repository runtime edge or completed absorption. Other reserved classes, root/reference
resolution, complete donor census, rights qualification, zero external dependencies and target
consumer/denial/crash/replay evidence remain separate obligations.

## BLAKE3 scope and dependency closure

The existing source `hashing` donor row maps only public-byte unkeyed BLAKE3-256 tree hashing
to the current census capability `security.blake3-tree-hash`. Its existing source node is
`compiler.core`; this is a local locator, not the target native module or Chronica admission.
Current census description fact `080d15de6f87ff8341b9ba72` pins upstream
`f55849f89cd85083c1c9daa2c5de20766f309c00`; it does not rewrite the older source provenance
`6aab490a26124663329dfd3961b8469f8fdb158b`. Committed official vectors, streaming splits and
the high counter regression are the scoped source oracles. Keyed/MAC/KDF, SIMD dispatch,
parallel/mmap, hazmat composition, C implementations and CLI behavior are not mapped.
The source also implements public-byte XOF; its tests do not prove the separate keyed/KDF/XOF
family. Content-addressing and integrity remain required, unresolved application contracts.

Zero external dependencies is a target cutover requirement, not the current source state.
For the unchanged locked source manifests on x86_64-unknown-linux-gnu/default, the engine's
`deps closure --json` reports these dependency-class rows (classes overlap package identities):

| Engine class | Rows | Direct | Transitive |
|---|---:|---:|---:|
| RUNTIME | 22 | 8 | 14 |
| COMPILE_TIME | 5 | 0 | 5 |
| BUILD | 17 | 2 | 15 |
| DEVELOPMENT_TEST | 6 | 2 | 4 |

This is `ALL_PRODUCTION_TARGETS`, with no frozen V1 cohort, and includes one libc reference
classified `NOT_APPLICABLE_ON_HOST`; it is not a certificate for other targets. Core still uses
serde/serde_json, and its live BLAKE3 1.8.7 differential oracle remains a test edge. Adapter
uses parser/grammar crates; its build script consumes local core hashing. Other source paths
invoke Git, Node/npm/browser tools, cargo/rustc, sandbox tools and operator-selected verifier
programs. The payload export path itself invokes none of those services. No service installation
can replace a native dependency-cutover proof.

Cutover must demonstrate native bounded record/schema/value/canonical/refusal parity and
caller migration at pinned revisions, then root/reference/materialization behavior separately.
It must preserve source identities/rights and optimized behavior, copy permitted byte oracles
with provenance, replace live external test oracles with qualified pinned evidence, and remove
source-repository path/git/build/test/runtime/RPC edges. Existing world/requester admission,
bounded work/evidence and denial/crash/replay tests must pass in the target. Source verification,
target native verification, actual consumer binding, Chronica admission and all fourteen donor
extinction gates remain distinct. The source donor's unchanged CENSUSED claim and absent
cutover do not declare any of those later states complete.

## Pinned census handoff

This reviewed addendum is portable source evidence, not admission or an adoption/extinction
claim. Earlier package paragraphs retain their original historical pins. Target tests were
not run; no registry, old package metadata or extraction/admission gate was changed.

| Observation | Exact Git revision |
|---|---|
| Implementation source (Core and runtime) | `6e2ee1d3c5de427a2aed1fb9be7d1c02bbc93782` |
| Target implementation inspected | `cf677cb9fe9435777cfbec4accf950aa586bf093` |
| Central package metadata | `d66313834c6d584b259650033bcf72f16c4e34f7` |
| Retained target declarations used by package projection | `848149d0c2d2d8b8923b394533879faabc5b6fb1` |
| Atlas HEAD at projection start; concurrent source WIP was possible | `0cd2d93671d1231bc408d287e7209f73b981f125` |
| Historical FUNCTIONS source claim | `fc8dbc4ee7c9cc94c50375672966f24b5ff1bbf7` |
| Historical native-result source | `b87472dee45b89e8dda355aad8c528b7b4cc64d4` |

Current source paths are `core/src/codara/artifact/{codec.rs,manifest.rs,staging/**,
census_container/**}`, `core/src/codara/construction/**` and unchanged
`core/src/identity/blake3.rs`. Source callers are `adapter/runtime/src/atlasx.rs` and
`apps/cli/src/main.rs`; `atlas_core` and existing node identities are unchanged.

| Target file at pinned target revision | SHA256 |
|---|---|
| `phenora/codara/src/run.rs` | `7e132058fb19da5ba2c1b930ed6f832645390fed78e3c98e2a9938933d8d28f6` |
| `phenora/codara/src/cli.rs` | `ca065b6092e982252cbe69c1bf92cf6fcf3f60cd120c7f95906a7d2d8405505c` |
| `physiology/capability/kernels/src/blake3.rs` | `46e61f433e093ab8734b5c807531d236760436c4e55bcc0c73e74b2b7c8d73a1` |
| `physiology/capability/kernels/src/legacy_artifact.rs` | `703c8f0699bfb504e738bf1f29516312aa57aaab4d5ef01788af4c833ad72905` |

FUNCTIONS codec SHA256 at the implementation pin is `1951bbeba8be36dc7a4024e5e7f37969efe2c85e8222ba586666b087168e62bd`;
BLAKE3 source SHA256 is `401e1cf27d4800c2fb8e0bd770e51fd6d67b3b156c675440c30e8fe648d98ff4`.
All **15** corpus object/payload/program pins, **17** target provenance entries and **three**
Atlas native-result copies were verified from exact Git objects. The existing `cases.json`
provides the individual byte pins; `.atlas`/`.atlasx` formats and encoded IDs remain historical.

| Capability key | Package digest | Read-only derived state |
|---|---|---|
| `codec.executable-world-objects` | `031429d3ff517ec7ce87c879` | `READY_FOR_ABSORPTION`; native coverage empty, donor IN_PROGRESS/license UNKNOWN |
| `security.blake3-tree-hash` | `9a0a1d79002b65fb9709a22a` | `STATIC_READY`; target-key donor-derived oracle missing |

These are pinned metadata observations, not current native verification. Package identity
hashes knowledge/owner/family/citation IDs, not live Rust bytes or source HEAD; the projections
were not rerun when source digests were refreshed.

**BLAKE3 classification decision:** central facts `fcdece2c1791587076db0a10` and
`85fc0e5455061d8dc0f6238c` record ALTERNATIVE_FOR BLAKE2 and native SHA256. The projection
calls BLAKE3 covered by `security.blake2-hash-and-mac` as the same native function. At the
pinned target, `.ynventa/declared/absorption/security.blake2-hash-and-mac.jsonl:6`, package
`50f757eea6fcc1f46b563d9e`, member `security.blake3-tree-hash`, class
`OPTIONAL_SPECIALIZATION`, explicitly says BLAKE3 is a different algorithm with its own
capability, not implemented inside BLAKE2. Record-file Git blob:
`822e19b23f6f6fb396a623689dd71fcc0f5e74e3`; SHA256:
`9cedf4049ea36d362ef5f1257f6c84c0df49bbf89fea1ab2a83ee71242a48bca`.
The responsible owner must resolve this contradiction and attach the algorithm-specific
oracle/caller mapping. BLAKE2 cannot preserve historical BLAKE3 digest identities.

Use the existing `codara run --payload FILE CAPSULE`. Actual target callers are
`phenora/codara/src/{cli.rs,run.rs}`; kernels are
`physiology/capability/kernels/src/{blake3.rs,legacy_artifact.rs}`.

- Digest flow selects `digest.blake3-256-verify` / `blake3-256.portable-reference`: bounded
  public bytes, 64-hex `claimed` and `input_bytes`; computed 32-byte digest and match verdict.
- FUNCTIONS selects `artifact.legacy-identity-verify` / `legacy.functions-object`: the complete
  named historical object, `file_name` and `input_bytes`; success yields exact original payload
  after the 80-byte header. Digest-only success does not establish semantic decoding.

Both require existing authenticated requester scope, capsule/world/authority/budget/envelope
checks, capacity, one `cpu_units`, and resource lease/work/release bindings. Disclosure separately
requires `resource.observe`. Request binding covers domain, IR digest, implementation,
capability, selected kernel and actual payload SHA256; claim/filename are bound IR facts.
Preserve pending **Unknown** across retry/reopen; never rebind or rerun it as a new request.
Existing version-1 `Blake3Result`/`FunctionsResult` receipts have bounded 1024-byte decoders and
retain input/object/IR/filename digests, checked optional claim/computation, verdict and first
FUNCTIONS defect. No interface field is missing for the pinned corpus flow.

**FUNCTIONS mapping decision:** source `[a,a,b,b]` duplicates retain two identity-specific
`DUPLICATE_IDENTITY` diagnoses; target deduplicates generic diagnoses, while compact receipt
records only the first defect. Source clears semantic/address fields on refusal; target retains
checked digests. Preserve that evidence; full detail/multiplicity parity needs an explicit mapping.

Target validation remains required at the selected target revision: `phenora/codara/tests/`
`e2e_native_result.rs`, `e2e_source_native_result.rs`, `e2e_legacy_functions.rs`, and
`physiology/capability/kernels/tests/legacy_functions_parity.rs`.
Atlas remains UNLICENSED; no rights grant was added. BLAKE3 provenance retains upstream
`6aab490a26124663329dfd3961b8469f8fdb158b`; target CC0 election still records owner decision
OD-15 as open. Root/materialization replacement, body semantics, complete donor census,
consumer/dependency cutover and V1 extinction qualification remain separate obligations.
