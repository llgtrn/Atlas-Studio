# Bounded FUNCTIONS evidence bytes proposal

Status: **PROPOSED source-local serialization profile and executable test vectors**.
`atlas.functions-evidence-bytes.proposal.v1` is not a Chronica receipt, storage format,
registered artifact kind, endpoint or adopted shared wire. Chronica must review and select
its native representation, durable owner and admission before any target implementation.
The earlier `FUNCTIONS-LOSSLESS-EVIDENCE-HANDOFF.md` remains the complete semantic obligation;
this proposal supplies one concrete way to carry those values without a lossy projection.

## Existing owners and blockers

The source baseline is Atlas-Studio `0bed61abbf05df6b0a323cfcaf7013e371548408`.
The seven-case oracle remains `core/src/codara/artifact/codec/parity/full-evidence.json`,
SHA256 `9c9784254f48f21a977b6c736d1c33a284ba3c6f3c396abe1e2e4f0eb6a77975`.
All historical object names, inputs, reports, IDs and prior evidence stay unchanged.

Read-only Chronica comparison pin: `ce3034687fadaf19f12ed094d263c20ec47631e3`.

| Existing owner | Observed contract | Consequence |
|---|---|---|
| `phenora/codara/src/run.rs`, `FunctionsResult` | Version 1 has one `first_defect`; decoder bound 1024 bytes. | Preserve existing v1 bytes and narrower meaning. Do not send this proposal to that decoder. |
| `lineara/src/work_receipt.rs` | `MAX_RESULT_BYTES = 1024` independently bounds opaque result bytes on encode and read. | Changing only the FUNCTIONS decoder cannot carry larger evidence. |
| `physiology/world/src/artifact.rs`, `ArtifactBook` | Stores SHA256, size, label, source locator, kind and provenance; kinds describe Native/Oci/Wasm/Model/Script. | Metadata is not retained blob content; JSON evidence is not silently classified as an executable artifact. |
| `physiology/host/src/artifact.rs` | `register_artifact` reads an absolute file under `artifact.register`; `artifact` resolves metadata. | A file locator grants neither durable byte retention nor authorized content retrieval. |
| `lineara/src/run_log.rs` | Rotating, deletable chunks and truncated lines. | Run logs cannot preserve these immutable evidence bytes. |

No existing public canonical content/chunk reader was found at this pin. This is a genuine
target contract dependency. The fixture's offset model below is not a new production reader.

## Exact source-local byte profile

Each `lossless-bytes/*.evidence.json` is one complete UTF8 document. Its keys are sorted
lexicographically, object separators are comma and colon without spaces, Unicode is literal,
and exactly one LF follows the object. Duplicate JSON keys, non-finite numbers, invalid UTF8,
unknown fields and profile versions, noncanonical bytes and trailing data are refused.
The enclosing profile does not normalize, sort or deduplicate source report content.
The generic fixture decoder checks the envelope and top-level report shape. The separate
seven-case oracle comparison checks complete nested source/checked values with JSON types
preserved; the actual CLI comparison checks exact report bytes. This is not a second codec
schema implementation or a validator for arbitrary source reports.

| Packet member | Meaning |
|---|---|
| `profile` | Exactly `atlas.functions-evidence-bytes.proposal.v1`. |
| `source` | Exact repository, baseline revision, source codec SHA256 and oracle SHA256, pinned in `vectors.json`. These are provenance, not authority or an admitted IR digest. |
| `case` | Existing seven-case oracle label. |
| `input` | Exact `file_name`, complete object `bytes` count and `sha256`. The name is not normalized. |
| `checked_evidence` | All existing independent checked fields, with exact null/present distinctions. Claimed, computed, mismatched and absent digests remain different. |
| `source_report_utf8` | The complete actual `atlasx codec --decode` report text, including its final LF. Decoding this JSON string and UTF8-encoding it reconstructs the original report bytes exactly. |

`source_report_utf8` carries every ordered `(code, detail)` diagnosis, all eight caveats,
explicit nulls, positive record fields, parameter/lineage order and Unicode. The two distinct
duplicate-identity diagnoses remain distinct. A complete semantic refusal is still useful
evidence and must be retained; successful transport cannot turn it into `DECODED`.
The packet also retains completed byte checks on a semantic/filename refusal and retains
absence markers when header/bounds checks prevented hashing. It contains no manufactured
target IR digest, requester, WorkRun, grant, proof strength or canonicalization outcome.

The exact packet bytes have a separate SHA256 content digest and byte length, recorded in
`vectors.json`. That digest is neither the input SHA256 nor its payload BLAKE3 address, and
does not change any historical identity. SHA256 matches the existing target artifact digest
algorithm; this observation does not register a new artifact kind or content store.

| Existing case | Actual CLI report bytes | Complete packet bytes | 512-byte chunks |
|---|---|---|---|
| `distinct-duplicate-identities` | 1350 | 2500 | 5 |
| `semantic-refusal-precedes-wrong-filename` | 1350 | 2452 | 5 |
| `payload-digest-mismatch` | 1340 | 2476 | 5 |
| `filename-refusal-after-semantic-success` | 1343 | 2436 | 5 |
| `pre-header-refusal` | 1273 | 2217 | 5 |
| `bounds-refusal-before-payload-hash` | 1265 | 2289 | 5 |
| `decoded-comparison` | 2236 | 3487 | 7 |

## Bounded offset model and refusal

This proposed fixture profile allows at most **8192 total packet bytes**, at most **16 chunks**
and exactly **512 bytes per full chunk**, with a shorter nonempty last chunk. These are test
profile bounds, not new target receipt limits or admitted work budgets. No compression,
truncation, diagnostic dropping or silent fallback is allowed. A larger complete result
requires an explicitly reviewed future profile or is refused whole.

`describe(bytes, capacity)` refuses empty data, total size over 8192, or insufficient supplied
capacity before exposing a descriptor. Its descriptor contains exact full SHA256/length and
ordered chunk entries `(index, offset, bytes, sha256)`. Each offset is `index * 512`; chunk
count is `ceil(length / 512)`. Every digest covers raw bytes, with no domain prefix bytes.

The executable test model `read_chunk(blob, descriptor, index, capacity)` first verifies the
complete bounded blob against its descriptor. It returns exactly the recorded byte interval
only when index and response capacity are valid. Repeated reads return identical bytes.
There is no path, URL, provider invocation, regeneration or authorization fallback.
Chunk boundaries may split a multibyte UTF8 character: chunks are opaque bytes and UTF8/JSON
is decoded only after complete verified assembly. Each chunk response's transport envelope,
authentication and disclosure costs are deliberately not claimed to fit a 1024-byte receipt.

Assembly requires exactly one response per descriptor entry in index order. Missing,
duplicate, reordered, substituted, oversized, truncated or cross-artifact chunks refuse
without yielding a partial document. Both each chunk SHA256 and the final length/SHA256
must match before the profile decoder exposes the report. An identical chunk retry is safe
at retrieval; the final assembly still has one occurrence per index. Descriptor and chunk
counts are bounded before allocation/join. These test-only checks are not a production store.

Transport refusal, unknown/missing durable evidence and a complete source `REFUSED` verdict
are three separate outcomes. No partial retrieval, capacity error or missing artifact may
be interpreted as a successful work result, settled effect, fresh execution or authority.

## Existing caller capacity boundary

The source CLI adds only this optional local preflight:

```text
atlas-systemizer atlasx codec --decode FILE [--out REPORT] [--max-report-bytes DECIMAL]
```

The limit counts the actual complete pretty report UTF8 bytes plus terminal LF. Canonical
ASCII unsigned decimal is required; `0` is valid, signs, leading zeroes, whitespace,
duplicates and overflow refuse. The flag is valid only for the sole `--decode` mode.
Over-capacity reports exit 2 with `ATLASX_REPORT_CAPACITY_EXCEEDED: bytes=N; max=M`, before
writing report bytes, creating output parents, or replacing existing evidence. No output
is truncated. Sufficient capacity preserves the original report bytes and success/refusal
exit. Omission preserves historical behavior and the immutable create-or-identical replay
rule. Capacity failure is distinct from a semantic codec refusal.
This is an output preflight after decoding/serialization, not an additional input-size,
allocation or CPU budget. Existing codec input bounds still apply independently.

This source limit does not enlarge the target's 1024-byte bound. The source report and the
proposal packet have different lengths; they must each fit their own explicitly checked
capacity. A later adequate local read is not permission to rerun an unknown governed effect.

## Target integration dependencies and rights

Before adopting this or another complete representation, Chronica must decide:

1. The canonical native owner that retains immutable bytes and resolves content digests,
   with crash/reopen, corruption, retention/legal-hold and unavailable-content behavior.
2. The approved typed reference/result version and bounded content/chunk request/response,
   preserving historical receipt bytes and both current 1024-byte limits. No such shared
   interface is edited or declared implemented by this source proposal.
3. Exact binding to the admitted durable request, complete input and filename, actual target
   IR digest, EffectRef and known settled outcome. Source provenance cannot supply these.
4. Current requester and resource scope, `resource.observe` and applicable disclosure
   admission; existing artifact metadata lookup is not an authorized byte-disclosure API.
5. Total byte/chunk/storage budgets and failure ordering. Capacity must be checked before
   exposing a successful reference. Crash/unknown states remain unknown; no retry may
   rematerialize bytes or repeat a settled/unknown effect in order to fill a missing chunk.

Atlas remains `UNLICENSED` and OD-15 remains open. This proposal adds no license, downstream
copy permission or source semantic adoption. Target tests and implementation are separately
owned and require that gate; source fixture parity grants no rights or execution authority.
UI ownership remains exclusively Chronica. There is no source-to-target runtime dependency.

## Direct checks

```text
python3 core/src/codara/artifact/codec/parity/lossless-bytes/check.py --cli /ABS/PATH/atlas-systemizer
```

The checker validates all seven exact packet/full-report/check-evidence vectors, compares
the actual existing CLI reports byte for byte, and tests capacity/boundary/corruption and
incomplete retrieval refusals. Existing core full-evidence regression independently checks
payload BLAKE3 and stage absence. Existing CLI integration tests exercise the optional
capacity preflight and preserved ordinary behavior. These are bounded source observations,
not target lossless qualification or renewed full source self-admission after this change.
