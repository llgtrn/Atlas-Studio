# Proposed lossless FUNCTIONS evidence handoff

Status: **PROPOSED semantic mapping and acceptance obligations**. This document does not
select a shared wire format, receipt version, sidecar, binary encoding, size allowance,
authority schema or downstream license. Chronica owns its native implementation and admission.
The source-side contribution is a bounded, immutable test oracle and real caller coverage.

## Immutable scope and provenance

The source snapshot is Atlas-Studio
[`bd744371a640ae96288fbf5deb771358aafbf28a`](https://github.com/llgtrn/Atlas-Studio/tree/bd744371a640ae96288fbf5deb771358aafbf28a).
The oracle and caller tests were published in source commit
`5fee71078975a73ba27b11d818a4c8b3c982ddc0`; the later snapshot retains them.
The authoritative seven cases are
[`core/src/codara/artifact/codec/parity/full-evidence.json`](https://github.com/llgtrn/Atlas-Studio/blob/bd744371a640ae96288fbf5deb771358aafbf28a/core/src/codara/artifact/codec/parity/full-evidence.json),
Git blob `a4cc76d89333ca30597275e76a3f30f198dab189`, SHA256
`9c9784254f48f21a977b6c736d1c33a284ba3c6f3c396abe1e2e4f0eb6a77975`.
Its `test_only_oracle = atlas.functions-full-evidence.v1` identifies test data, not a new
execution or evidence wire contract.

The manifest's `source_revision = 706a1c431b8e7abbcd899ebb97893a681572b217` and codec SHA256
`1951bbeba8be36dc7a4024e5e7f37969efe2c85e8222ba586666b087168e62bd` retain the original
reader provenance. Its `reference_target_revision = cf677cb9fe9435777cfbec4accf950aa586bf093`
is the historical comparison pin. Neither is rewritten to imply a new assay.
All existing fixture bytes, historical `.atlasx` names, encoded identities, source reports,
ArtifactIds and previously issued receipts remain unchanged.

A later read-only comparison at Chronica
[`0b65c7b1c532e473e9d9157b170f1a43c92fe6bd`](https://github.com/llgtrn/Chronica/tree/0b65c7b1c532e473e9d9157b170f1a43c92fe6bd)
confirms the representation gap:

| Existing owner and immutable file | Git blob | Observed limitation |
|---|---|---|
| [`phenora/codara/src/run.rs`](https://github.com/llgtrn/Chronica/blob/0b65c7b1c532e473e9d9157b170f1a43c92fe6bd/phenora/codara/src/run.rs) | `ca78b4231e1d0820186346c24ebca5c1f989b391` | `FunctionsResult` v1 stores one `first_defect`, with no detail vector or complete decoded record tables; its decoder rejects input larger than 1024 bytes. |
| [`physiology/capability/kernels/src/legacy_artifact.rs`](https://github.com/llgtrn/Chronica/blob/0b65c7b1c532e473e9d9157b170f1a43c92fe6bd/physiology/capability/kernels/src/legacy_artifact.rs) | `a81433628039fc4fbe3f699b2ef0fca4250b302c` | Duplicate identities use the generic detail `two signatures have one identity`; refusals are sorted and deduplicated, collapsing distinct source identity diagnoses. |

These observations do not replace the narrower claims those existing kernels and receipts
already support. No target execution or lossless target qualification is asserted here.

## Input and evidence boundaries

Each case supplies the existing oracle coordinates below. These names describe the source
oracle; they are not newly required request fields in Chronica's live interface.

| Existing case field | Required meaning for a lossless comparison |
|---|---|
| `case` | Stable test label, not a function identity, work ID or authorization. |
| `fixture` | Relative path under the pinned parity directory to the complete input bytes. Do not substitute a raw payload for the object. |
| `file_name` | Exact UTF8 name argument passed to the named reader. It can deliberately differ from the fixture's basename. |
| `input_bytes` | Exact length of that complete input, including any header bytes present. It is not a new admitted capacity or budget. |
| `checked_evidence` | Independently observed byte checks, kept separate from semantic decoding and authority. |
| `source_report` | The full expected source `CodecReport`, including explicit nulls and all ordered diagnoses. |

The existing strict target flow remains `codara run --payload FILE CAPSULE`, with goal
`artifact.legacy-identity-verify`, selected implementation `legacy.functions-object`, and
existing `file_name` and `input_bytes` facts. The positive input is the complete 521-byte named
object; its successful kernel output is the 441-byte payload after the header. The separate
`digest.blake3-256-verify` flow instead receives raw payload bytes and a 64-hex `claimed` fact.
A digest match alone does not prove FUNCTIONS decoding. No new missing input field is
established for either existing bounded flow.

Chronica still supplies the actual authenticated requester, current capsule/world binding,
authority, resource lease/work/release bindings, capacity, one `cpu_units` demand,
budget/envelope checks, durable request/payload binding and bounded result evidence.
Disclosure additionally requires its existing `resource.observe` admission. A pending or
unknown durable outcome must remain unknown across retry/reopen; do not rebind it, synthesize
a grant or rerun it as a new request. This handoff adds no live invocation or external effect.

### Independent checked evidence

| Existing `checked_evidence` field | Complete preservation obligation |
|---|---|
| `input_sha256` | SHA256 of every supplied input byte, including refused/truncated inputs. Never confuse it with the payload BLAKE3 address. |
| `file_name_sha256` | SHA256 of the exact UTF8 `file_name` argument, not the fixture path or normalized filesystem name. |
| `expected_payload_blake3` | Header's claimed 32 bytes as lowercase 64-hex text when a complete header exists; explicit null before that stage. A present claim is not a verified digest. |
| `computed_payload_blake3` | Independently recomputed BLAKE3 only when this corpus's valid header and exact bounded payload permit that check; explicit null when the check could not run. |
| `checked_payload_sha256` | SHA256 of that same checked payload; explicit null when there is no checked payload. |
| `checked_payload_length` | Exact checked payload length; explicit null when no bounded payload check completed. Do not substitute a declared length or zero. |

Matching, mismatching and absent expected/computed digests are different observations.
Semantic or filename refusal must not erase completed byte-check evidence. Conversely,
retaining checked digests must not turn a refusal into accepted semantic records.
BLAKE2, SHA256 and BLAKE3 remain distinct algorithms and identity domains; none substitutes
for BLAKE3 byte parity. An actual IR digest depends on the compiled/admitted target program;
this oracle supplies none and does not authorize fabricating one.

### Complete source report projection

A proposed native mapping must permit reconstruction of every field below with the exact
values in `source_report`. Its internal representation is undecided. Compare parsed JSON
values with exact strings, array order, multiplicity and explicit nulls; JSON object member
order or pretty-print whitespace is not a new identity preimage.

| Existing source report field | Preservation obligation |
|---|---|
| `schema` | Exactly `atlas.atlasx-codec.v1`, identifying the source view rather than renaming the target's receipt. |
| `verdict` | Exactly `DECODED` or `REFUSED` as pinned for that input and filename. |
| `defects` | Complete ordered array of `(code, detail)` pairs. Preserve each exact detail and each distinct identity diagnosis; a first-code summary, generic message or additional deduplication is insufficient. |
| `address` | Positive case's lowercase historical payload address; explicit null on every refusal. |
| `digest` | Positive case's exact `blake3-256:` identity text; explicit null on every refusal. |
| `object_class` | Integer 4 on the positive case; explicit null on every refusal. |
| `object_schema_version` | Integer 1 on the positive case; explicit null on every refusal. |
| `decoded_length` | Integer 441 on the positive case; explicit null on every refusal, even if a separate bounded payload check completed. |
| `records` | Complete two-record positive table described below; explicit null on every refusal, not an empty decoded table. |
| `not_verified` | All eight exact source caveat strings in their pinned order. Keep source limitations distinguishable from any additional target limitations. |

Each positive `FunctionSignatureRecord` preserves `function_id`, `name`, `owner`, `dispatch`,
`visibility`, `documentation`, ordered `params` with exact `name` and `type_spelling`, `result`,
`body_fingerprint` and ordered `lineage`. Preserve Unicode, empty optional text versus null,
parameter declaration order and every carried identity/type spelling. In this fixture,
`fixture:alpha` precedes `fixture:beta`; alpha's name is `α`, owner/documentation are empty
strings, parameters are `z: u8` then `a: &str`, and lineage is `fixture:lineage:a` then
`fixture:lineage:z`. Beta's owner/documentation/result/fingerprint are null and its parameter
list is empty. The exact remaining values are in the pinned oracle. A carried fingerprint is
not a function body, resolved lineage, admitted materialization or authority.

## Seven expected cases and measured sizes

These are measurements of the pinned oracle, not protocol limits. `report` and `checked` are
UTF8 byte lengths of compact JSON with literal Unicode, no insignificant whitespace and no
trailing newline. They describe a comparison view, not a proposed native binary encoding.

| Case | Input bytes | Defect count | Report bytes | Checked bytes | Checked payload bytes |
|---|---:|---:|---:|---:|---:|
| `distinct-duplicate-identities` | 504 | 2 | 1214 | 476 | 424 |
| `semantic-refusal-precedes-wrong-filename` | 504 | 2 | 1214 | 476 | 424 |
| `payload-digest-mismatch` | 521 | 1 | 1228 | 476 | 441 |
| `filename-refusal-after-semantic-success` | 521 | 1 | 1231 | 476 | 441 |
| `pre-header-refusal` | 7 | 1 | 1161 | 291 | null |
| `bounds-refusal-before-payload-hash` | 520 | 1 | 1153 | 353 | null |
| `decoded-comparison` | 521 | 0 | 1836 | 476 | 441 |

The complete ordered expected diagnoses are:

```json
[
  {
    "case": "distinct-duplicate-identities",
    "defects": [
      [
        "DUPLICATE_IDENTITY",
        "two records for fixture:a"
      ],
      [
        "DUPLICATE_IDENTITY",
        "two records for fixture:b"
      ]
    ]
  },
  {
    "case": "semantic-refusal-precedes-wrong-filename",
    "defects": [
      [
        "DUPLICATE_IDENTITY",
        "two records for fixture:a"
      ],
      [
        "DUPLICATE_IDENTITY",
        "two records for fixture:b"
      ]
    ]
  },
  {
    "case": "payload-digest-mismatch",
    "defects": [
      [
        "DIGEST_MISMATCH",
        "the payload does not hash to 86e6111beb161e9b7241cf136ad8738eed8bbd3872707d6511dbcc192d6296d2"
      ]
    ]
  },
  {
    "case": "filename-refusal-after-semantic-success",
    "defects": [
      [
        "ADDRESS_MISMATCH",
        "`other.atlasx` is not `86e6111beb161e9b7241cf136ad8738eed8bbd3872707d6511dbcc192d6296d2.atlasx`"
      ]
    ]
  },
  {
    "case": "pre-header-refusal",
    "defects": [
      [
        "TRUNCATED",
        "7 bytes, shorter than the header"
      ]
    ]
  },
  {
    "case": "bounds-refusal-before-payload-hash",
    "defects": [
      [
        "TRUNCATED",
        "440 payload bytes of 441"
      ]
    ]
  },
  {
    "case": "decoded-comparison",
    "defects": []
  }
]
```

- `distinct-duplicate-identities` and `semantic-refusal-precedes-wrong-filename` retain both
  `fixture:a` and `fixture:b` diagnoses in that order. Their complete source reports are equal;
  the wrong filename does not replace a prior semantic refusal. Input SHA256s match, while
  filename SHA256s differ. Checked expected/computed BLAKE3 values match and remain present.
- `payload-digest-mismatch` retains distinct expected and computed BLAKE3 values; their exact
  hex strings are pinned in the oracle. Hash refusal precedes semantic decoding.
- `filename-refusal-after-semantic-success` retains matching checked digests but returns
  `ADDRESS_MISMATCH` with all source semantic/address fields null. Filename checking follows
  successful hash, framing, schema and canonical-form checks.
- `pre-header-refusal` has null `expected_payload_blake3`, `computed_payload_blake3`,
  `checked_payload_sha256` and `checked_payload_length`. Its complete-input and exact-filename
  SHA256 evidence still exists.
- `bounds-refusal-before-payload-hash` retains the header's expected BLAKE3 claim. Its computed
  BLAKE3, checked payload SHA256 and checked length remain null: 440 available payload bytes
  are not a completed check of the declared 441-byte payload.
- `decoded-comparison` has no defects, both complete signature records and a 441-byte payload
  identical to `codara-parity.payload` and the original complete object's bytes after byte 80.

The source codec's existing bounds remain class 4/schema 1, an 80-byte format-1.0 header,
BLAKE3 digest ID 1, codec NONE, equal encoded/decoded lengths, zero flags/reserved,
67,108,864 maximum decoded bytes, 16-byte record headers, 8-byte field headers and nesting
depth 8. The largest permitted complete object under that existing payload bound is
67,108,944 bytes; the runtime reads at most one extra byte to detect an overlong object.
This seven-case corpus instead has inputs of only 7–521 bytes and checked payloads of
424 or 441 bytes. It does not establish arbitrary-size completeness or a new output allowance.

The existing v1 target decoder's **1024-byte ceiling is unchanged**. Every measured compact
source report above exceeds 1024 bytes; the positive source report is 1836 bytes before any
wrapper or native execution evidence. Those JSON bytes cannot simply be inserted unchanged
into the existing bounded v1 decoder. Another encoding might have different sizes, but no
encoding, extension, sidecar, compression, size limit or automatic widening is selected here.
Any proposed representation must receive joint review of its bounds, completeness, identity
preimage, decoder behavior, evidence retention and compatibility with historical v1 bytes.

## Proposed target acceptance obligations and decisions still open

1. Select and document an explicit, bounded mapping that reconstructs all seven complete source
   reports plus their independent checked evidence. Run field-by-field equality against the
   immutable source oracle, preserving strings, ordered arrays, multiplicity, empty values,
   explicit nulls, positive record fields and exact payload output. No `first_defect`-only or
   generic-dedup projection may be labeled full parity.
2. Exercise semantic-before-filename refusal, mismatch, pre-header absence and bounds-before-hash
   absence separately. A normalizer that drops the second identity diagnosis, replaces details,
   fills an absent computation, clears completed byte evidence or emits records on refusal
   must fail the full-evidence comparison.
3. Keep existing v1 receipt bytes, IDs and narrower semantics valid under their existing
   decoder, including its 1024-byte bound. Any approved new representation must not silently
   reinterpret or overwrite those historical records. Additional target evidence can remain
   separately identifiable; its exact residence and wire representation are unresolved.
4. Retain actual target object/filename/IR digests and current durable request binding. The v1
   `object_sha256` and `file_name_sha256` correspond to the oracle's input and filename hashes;
   optional `expected` and `computed` correspond to their completed check stages. The actual
   `ir_digest` is extra target execution evidence, never an oracle-supplied invented value.
   `first_defect` remains a bounded historical summary, not the complete diagnosis vector.
5. Chronica must provide tests through its actual native kernel and governed caller, with an
   explicit source-to-native comparison and any agreed decoder/sidecar bounds. Source tests
   below establish source behavior only. No target test name, capsule, binding, grant or full
   native acceptance result is fabricated by this handoff.
6. Resolve the downstream rights decision separately. Atlas-Studio remains `UNLICENSED`; OD-15
   is not closed here. This document grants no adoption rights, elects no license and instructs
   no copying of source implementation or test assets into a downstream product. It supplies
   immutable references for the owners' review.

Existing positive digest/FUNCTIONS flows need no invented input field to address this
representation gap. Full lossless evidence mapping, its bounded native representation and
qualification remain genuine blockers. Root manifest/reference resolution, admitted
materialization, function-body semantics, consumer/dependency retirement, rights qualification
and full native hardware/journey acceptance are separate work; the seven cases certify none
of those outcomes.

## Portable source validation commands

Run from an already available clean checkout of the immutable Atlas-Studio snapshot above.
These commands are provided for reproduction, **not executed by this handoff draft**. They
require its existing Rust 1.90-compatible offline dependencies and Python 3. No host-specific
`/workspace` path, branch change, live target invocation or external/device effect is required.
A missing offline tool/dependency is `NOT_RUN`, never a passing comparison.

```sh
# Confirm the source tree under test; do not substitute a later tree without a recorded comparison.
test "$(git rev-parse HEAD)" = bd744371a640ae96288fbf5deb771358aafbf28a
python3 core/src/codara/artifact/codec/parity/full-evidence/check.py
cargo test -p core --lib --offline -- \
  codara::artifact::codec::tests::full_functions_evidence_oracle_preserves_diagnostics_and_check_boundaries \
  --exact --format=pretty --color=never
cargo test -p runtime --lib --offline -- \
  atlasx::tests::codara_export_enforces_the_legacy_functions_corpus \
  --exact --format=pretty --color=never
cargo test -p atlas-cli --test full_evidence --offline -- \
  --format=pretty --color=never
```

Require the asset checker to pass, the exact core test to execute once, the exact runtime test
to execute once, and both CLI integration tests to execute with zero failures or ignored tests.
A zero-test success or narrower subset is not evidence. The core test checks independent
reference BLAKE3 and the stage-specific absence markers as well as complete source reports.
The real CLI tests compare all seven printed report values and exact success/refusal exits;
they also exercise payload replay and refusal without creating output or replacing prior
bytes. Retain the command, exact source commit, tool versions, exit status, counts and logs.

The per-case JSON-size measurements above can be reproduced without invoking a decoder:

```sh
python3 - <<'PY'
import json
from pathlib import Path
oracle = json.loads(Path('core/src/codara/artifact/codec/parity/full-evidence.json').read_text())
def size(value):
    return len(json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf-8'))
for case in oracle['cases']:
    report, checked = case['source_report'], case['checked_evidence']
    print(case['case'], case['input_bytes'], len(report['defects']),
          size(report), size(checked),
          [name for name, value in checked.items() if value is None])
PY
```

These size measurements and source-hosted test results are not native hardware certification,
a target adoption result or a replacement for Chronica's governed admission and evidence.
