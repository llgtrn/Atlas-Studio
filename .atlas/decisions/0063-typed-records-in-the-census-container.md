---
id: atlas.decision.0063.typed-records-in-the-census-container
type: decision
status: accepted
canonical: true
---
# ADR 0063 — Typed records, evidence and diagnostics in the census container (G147)

## Context

The census container (ADR 0027) carried facts, typed obligations, the declared ADL graph and the certificate. It did not carry the census's typed semantic records.

Mission M6 (G146) confirmed this through the agent interface: `atlas_of` reads no typed-record field, and the records reach the container only through the census digest in its manifest. The self census has 148,000 typed records.

Construction node M1 ("typed-record `.atlas` sections", DEBT-ATLAS_COMPLETENESS) is the first missing node on the FIRST_ARTIFACT critical path. SelectedDesign roots, seal content and the AtlasX materializer all need typed identities inside a verified root.

The wire contract requires the complete typed record, and forbids JSON or display text as a payload.

## Decision

1. **Declared kinds, field by field.**
   - SEMANTIC_RECORDS gains one record kind per typed family, kinds 2 to 13. Each is named as its serde tag and has a shared header.
   - Subjects and nested structs are embedded-only kinds numbered 100 and above: revision, scope, extractor, provenance, span, place reference, documentation, type, symbol, function owner, function identity, parameter, signature, call site, control-flow edge and block, value, state access, effect, ownership, concurrency, persistence.
   - New sections: EVIDENCE (10) and DIAGNOSTICS (11).
   - Wire types: `RECORD` (a sequence of embedded records of the declared kind), `PACKED` (minimal uvarints) and `BOOL`, from the AtlasX wire table.
   - A field's shape (`record=`, `repeated`, `element=`) enters the definition text only when present, so every earlier schema identity is unchanged.
   - Schema history generation G147 is recorded. The G68 definitions still conform (kinds were only added).
2. **A schema-directed codec** (`core::atlas::typed`). A value's serde form is walked against its declared kind:
   - A key the kind does not declare, a null or missing required field, and a wrongly typed value are refused at write. A typed field the schema has not caught up with is never dropped.
   - An empty sequence is an absent field. An enum kind carries its variant name, plus the payload in the field named after the variant.
   - Decoding rebuilds the serde form and deserializes it: `decode(encode(r))` equals `r`.
   - `serde_json` becomes a dependency of `core`. It is pure computation, and is already in the workspace build.
3. **Canonical order and integrity.**
   - Typed records are ordered by their serde form, `typed_key`, because `record_id` alone is not unique. Evidence and diagnostics are ordered by id.
   - Every evidence reference must resolve: the writer refuses a dangling one and the reader rejects it.
   - An embedded-only kind at top level, a fact after the typed records, a minor above 1, and typed content under minor 0 are all rejected.
   - `FORMAT_MINOR` is 1.
4. **Production path.**
   - `atlas_of` packages every typed record, evidence record and diagnostic of the census, and a census whose typed records would collapse is refused.
   - `PackOutcome` reports their counts.
   - `atlas verify --root` compares the full content, typed records included.

## Evidence

- **Core tests.**
  - The fixture holds 31 records of all 12 families from the self census: the largest and smallest of each, both `PlaceRef` variants, present and absent optional fields, and a projection. Its evidence and diagnostics round-trip byte-deterministically.
  - No record's serde text appears in the container.
  - The corruption and truncation batteries still pass.
  - Undeclared, null-required, missing, wrongly typed and undeclared-variant values are refused.
  - Swapped typed records, a fact after the typed records, an embedded kind alone, a dangling evidence reference, minor 2 and minor-0 typed content are rejected.
- **Runtime self census.** Every typed record, all 12 families, comes back from the published container equal to the census list, together with every evidence record and diagnostic.
- **Mutants**: see GENERATIONS G147.

## Consequences

- **M1 exists.** The container carries every typed identity a SelectedDesign root or a seal can name.
- **Deferred:**
  - IDENTITY/TYPE/SYMBOL interning tables: identities are inline through the string table;
  - the DESIGN section, which belongs to M7;
  - compression, as the container grows with the typed records.
- **Next on the critical path:** M7, SelectedDesign.
