# Full source FUNCTIONS refusal evidence

`../full-evidence.json` is a test-only oracle, separate from every source and target wire
contract. Its seven cases pin the complete serialized source `CodecReport`, including all
ordered defect codes and details, every absent field, decoded records and `not_verified`.
The original thirteen legacy fixtures, `cases.json` and historical handoff remain unchanged.

The source reader is `core/src/codara/artifact/codec.rs` at
`706a1c431b8e7abbcd899ebb97893a681572b217`, SHA256
`1951bbeba8be36dc7a4024e5e7f37969efe2c85e8222ba586666b087168e62bd`.
The manifest SHA256 is `9c9784254f48f21a977b6c736d1c33a284ba3c6f3c396abe1e2e4f0eb6a77975`.
These pins identify input facts; a later publication commit contains this oracle extension.

The new 504-byte object has four 106-byte signatures in `[a,a,b,b]` order. Each signature
contains required function_id (`fixture:a` or `fixture:b`), name `f`, dispatch 1, visibility
`pub`, and one lineage record `semantic:X:1`. Records are framed independently with
little-endian kind/version/flags/length and fields with tag/wire/required/length; no source
encoder or decoder produced the fixture. The fixed 80-byte class-4/schema-1 header authenticates
the 424-byte payload. A separately compiled reference BLAKE3 1.8.7 hash supplied its digest.

Its canonical historical basename is
`8e6734dc26287d1f6484bf488c0e601420054e684178016d0b084477bd1986cd.atlasx`,
and its complete-object SHA256 is
`0cab67b87e6cb89f97702e0de150bd8bc0fdb1fda5fa034bb8acbcf66b0c70b3`.
The expected refusal contains both `DUPLICATE_IDENTITY / two records for fixture:a` and
`DUPLICATE_IDENTITY / two records for fixture:b`, in that order. Specifying the wrong filename
cannot replace these prior semantic diagnoses. Expected reports were specified from the source
contract and independently framed input, never copied from decoder output.

Each case supplies a `fixture` relative to the parent parity directory and the exact
`file_name` argument; those may differ deliberately. `input_bytes` is the complete input size.
`checked_evidence` is independent byte evidence, not fields added to the source report:

| Field | Meaning |
|---|---|
| `input_sha256` | SHA256 of every supplied input byte, even a refused/truncated input. |
| `file_name_sha256` | SHA256 of the exact UTF8 filename argument. |
| `expected_payload_blake3` | Header's claimed 32 bytes, absent before a complete header exists. |
| `computed_payload_blake3` | Independent reference hash only after this corpus's valid header and exact bounded lengths permit recomputation. |
| `checked_payload_sha256`, `checked_payload_length` | The same checked payload's SHA256 and length; absent when that check cannot run. |

The cases distinguish declared-only evidence on payload truncation, no digest evidence before
a complete header, mismatching expected/computed digests, and matching digests retained when
semantic or filename validation refuses. The source `CodecReport` still clears address,
digest, class, schema, length and records on every refusal. Its cleared fields must not erase
independently checked evidence; that evidence must not be presented as semantic acceptance.
The decoded baseline pins the complementary full positive report.

Run `python3 full-evidence/check.py` from the parent parity directory for all SHA256 pins,
and the core regression
`full_functions_evidence_oracle_preserves_diagnostics_and_check_boundaries`
for complete report equality, independent reference BLAKE3 and check-stage absence markers.
These are source tests. No target test was run to create or qualify this oracle.

Read-only target comparison is pinned to Chronica
`cf677cb9fe9435777cfbec4accf950aa586bf093`:
`physiology/capability/kernels/src/legacy_artifact.rs::verify_atlasx_functions` retains checked
digest options on refusal, but deduplicates generic duplicate-identity details.
`phenora/codara/src/run.rs::FunctionsResult` v1 retains input/filename/IR hashes, optional
expected/computed digests and one `first_defect`. It has no vector for identity-specific
ordered diagnosis details or full source report. Thus neither generic diagnosis deduplication
nor a first-defect receipt can represent this full oracle losslessly. Chronica must select an
explicit lossless mapping if full source evidence is required; no target version is chosen here.
An IR digest is not fabricated: it depends on the actual compiled/admitted target program.

This bounded corpus proves neither target parity nor native adoption, rights qualification,
root/reference resolution, admitted materialization, function bodies, live effects or UI cutover.
Source `UNLICENSED` and existing target rights obligations, including OD-15, remain unchanged.
