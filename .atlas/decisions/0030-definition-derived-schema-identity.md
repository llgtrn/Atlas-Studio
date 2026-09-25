---
id: atlas.decision.0030.definition-derived-schema-identity
type: decision
status: accepted
canonical: true
---
# ADR 0030 — `.atlas` schema identity is derived from the declared definition (Glean, absorbed)

## Context

ATLAS wire v1 requires that "a breaking change … get a new incompatible schema identity". G64 (ADR 0027) pinned `schema_id` as BLAKE3 of each section's *name*, a manual registry that depends on remembering to bump it.

First-50 donor #9 (Glean) recorded exactly this test as its cheapest falsification. G68 ran it (`../evidence/campaign/09-glean.json`):
1. Emulate a version that repurposes the fact fields' tags 4 and 5 (subject and predicate) in writer and reader together.
2. Pack the self-census with it.
3. Verify with the current reader.

The current reader **accepted** the file: it would have decoded every fact with subject and predicate swapped.

Glean derives a schema's identity from its definition and its dependencies (`glean/db/Glean/Database/Schema/ComputeIds.hs`: `hashBinary (ref, definition)`, with cycles folded through one hash).

## Decision

1. **The tables are the single source.** `core::atlas::schema` declares every section: its record kinds; each field's name, tag, wire type and requiredness; and the sections it depends on (the string table, which its indices reference). It is the only place a field name meets a tag.
2. **The codec goes through the tables.** Writer (`Record::of(..).string("subject", ..)`) and reader (`Fields::string("subject", ..)`) address fields by name. Tags, wire types and REQUIRED flags come from the tables, and table-driven records are emitted in ascending tag order.
3. **Schema ids are derived.** `schema_id` is the first 8 bytes of BLAKE3 over the canonical definition text, including the hashes of the sections it depends on. Any change to a tag, name, wire type or requiredness, or to a dependency, is a new identity. The reader refuses any identity it does not declare.
4. **Glean's cycle folding is not needed.** The dependency graph is acyclic.
5. **REFERENCE_ONLY:** Angle derived predicates and the query language (constraint derivation is already absorbed from Soufflé), the fact database, and the typed query IR.

## Consequences

- **The same emulation is now refused:** `section 6 has an unknown schema id`.
- **Older containers are refused explicitly, never misread.** Containers written before G68 (name-derived ids, e.g. the G64–G67 evidence roots) fail with "unknown schema id". Those roots stay valid as recorded identities of their commits, and `atlas pack` at a commit re-derives them.
- **A real bug caught mid-change.** The byte-flip sweep caught a reader panic introduced during the refactor (`schema_id` of a hostile section type). The reader now rejects unknown section types first.
- **Falsification:** 12 mutants, 11 killed. The survivor is equivalent: `expect_kind` guards every read position before the table lookup, so an undeclared record kind never reaches `Fields::new`.
