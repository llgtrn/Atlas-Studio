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

Naming is resolved from Chronica's CN-0005 and DECISIONS 0044, 0115 and 0125 at
`d9e8a067f18cdb931715027631a2fb60125a6166`: Codara's native owners are
`physiology/capability/codara` and `phenotype/codara`; the current source examples use `.cdr`.
CHGC capsules and their existing SHA256 identity domains are a separate native format.
No approved native FUNCTIONS object extension or conversion into Codara semantic IR was found.
Such a migration needs a coordinated versioned contract and consumer proof, not a suffix change
to ATLSX bytes. Legacy format names/addresses remain confined to legacy input and evidence.
The older committed `fixture_verify.codara` is retained at its historical provenance locator;
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
it cannot prove FUNCTIONS decoding. Chronica's partial `legacy_artifact::verify_atlasx_object`
checks header/bounds/hash/address and explicitly excludes record semantics. A native reader
must reproduce these decoded values/refusals and the broader codec invariants before semantic
parity is claimed. Root manifest, cross-object resolution and admitted materialization remain
separate obligations even for the positive case.

Chronica owns the actual authenticated requester, current resource and lease/work bindings,
one `cpu_units` demand, budget/capacity/envelope checks, payload binding to durable work,
bounded output evidence and fail-closed retries. This corpus adds no shared wire fields,
authority schema, permission, host invocation, runtime source dependency or consumer cutover.
