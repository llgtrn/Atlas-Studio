---
id: atlas.decision.0081.the-format-reference-falsifies-the-weight-reader
type: decision
status: accepted
canonical: true
---
# ADR 0081 — The format's own reader falsifies the weight reader: its whole dtype table, sub-byte sizes and its header bound (G166)

## Context

FULL_OSS_REPLAY R9 put huggingface/safetensors, at pin `e246a2560645`, in front of Atlas E14. Atlas's SafeTensors reader (E12, G163, ADR 0078) had been written from the format's published description alone. The format's donor was the falsification.

Atlas E14 went first. Its census of the donor's code (524 functions, 215 resolved INVOKES) located `SafeTensors::read_metadata`, `Metadata::validate` and the header tests. Its lenses do not list an enum's variants, so the element-type table was read by hand (recorded).

The differential ran the donor's own Rust reader, built from the pinned crate as a black box, against Atlas on 44 hand-written containers. The two agreed on 34:
- the reference admits 22 element types and Atlas knew 15. F4, F6_E2M3 and F6_E3M2 are sub-byte; F8_E8M0, F8_E4M3FNUZ and F8_E5M2FNUZ are 8-bit; C64 is 64-bit. Atlas refused all seven, and refused misaligned sub-byte tensors only because it did not know their type;
- the reference bounds the header at 100,000,000 bytes. Atlas's bound of 100 MiB admitted a 100,000,001-byte header the format refuses, so Atlas could have constructed such a file;
- the reference keeps the last of two duplicate keys silently; Atlas refuses them;
- the reference ignores an unknown field in a tensor entry; Atlas refuses it.

## Decision

1. `TensorDtype` carries the format's whole table. A tensor's size is its element count times the dtype's **bits**, in whole bytes. A sub-byte tensor that does not fill whole bytes is refused for that reason (`SizeError::Misaligned`), as the reference requires; an overflow stays `SizeError::Overflow`.
2. `MAX_HEADER_BYTES` is the format's own 100,000,000 bytes. It is checked before a byte of the header is read.
3. **Duplicate keys stay refused.** The classification is ATLAS_CORRECT: a file with two entries for one name is ambiguous, and the reference silently drops one.
4. **Unknown tensor fields stay refused.** The classification is BOTH_CORRECT_DIFFERENT_ABSTRACTION. The reference reads the tensors and ignores the field; Atlas claims a lossless construction and cannot preserve a field it does not model.

## Consequences

- **The same corpus at E14 and E15:** 34 → 42 of 44 files agree with the reference. The two that differ are the deliberate refusals above. Three misaligned sub-byte files are now refused for misalignment, not for an unknown type.
- **Construction.** Every one of the 26 files Atlas accepts and constructs is accepted by the reference reader, and each was re-censused with its content preserved.
- **Falsification.** All 7 mutants were killed:
  - the old header bound;
  - the whole-byte check dropped;
  - three wrong bit widths;
  - a wrong byte count;
  - a misnamed dtype.
- **The donor's code census** is unchanged at E15: the capability that changed reads the format, not code.
- **Still unknown or out of scope:**
  - Python (43 files) has no frontend, so the bindings' Python side is OUT_OF_SCOPE;
  - block-quantized formats (GGUF) remain W5;
  - roles stay UNKNOWN;
  - nothing of the donor's implementation is copied: the dtype names are the format's vocabulary, and the reader is Atlas's own.
