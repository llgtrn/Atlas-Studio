---
id: atlas.decision.0035.schema-evolution-conformance
type: decision
status: accepted
canonical: true
---
# ADR 0035 — `.atlas` schema evolution conforms (FlatBuffers, absorbed)

## Context

The wire contract says new record kinds and fields "use schema versions and compatibility rules", with only breaking changes needing a new incompatible identity. G68 (ADR 0030) derives each section's `schema_id` from its full definition, and the reader accepts only the current one.

So any change, even appending an optional field, makes every existing container unreadable by the new reader. Nothing distinguished a compatible evolution from a breaking one, and nothing stopped a breaking edit to `SECTIONS` from being committed.

First-50 donor #22 (FlatBuffers) recorded this as its one surviving ABSORB_LATER item: a `flatc --conform`-style checker, triggered by the first section writer. That writer has existed since G64.

`Parser::ConformTo` (`src/idl_parser.cpp`) enforces these rules: a surviving field keeps its id, type and default; a rename keeps id and type; deletion is refused (deprecate instead); enum values never change. Vtable-indirected field access adds nothing over the contract's tag-length-value framing, where an absent optional field is the default.

## Decision

1. **Conformance** (`core::atlas::schema::conforms`). A recorded definition conforms to the current one when all of the following hold:
   - the section keeps its name and dependencies;
   - no record kind disappears;
   - every field keeps its tag and wire type, is never removed, and never becomes required;
   - every added field is optional.

   A rename with the same tag and wire type conforms, since the wire is tag-addressed.
2. **History.** `core/src/atlas/schema_history.rs` records every definition ever written, as `generation <id>` blocks of `definition_text`, in one compiled-in string constant (a Rust source, so the census parses it like every other file). A test fails unless the latest generation equals the current `SECTIONS`, so a definition change must be recorded first. Another test fails unless every recorded generation conforms, so a breaking edit cannot land silently.
3. **Reader acceptance** (`accepted_schema_ids`). The reader accepts the current identity plus every recorded identity whose definition conforms to the current one, and decodes such content with the current tables. Every other identity is refused, as before.
4. **REFERENCE_ONLY:**
   - vtables and offset-indirected field access;
   - flexbuffers;
   - reflection;
   - code generation.

## Consequences

- **Compatible evolution is possible:** appending an optional field no longer orphans the containers written before it, and a breaking change fails the build unless the wire version or identity is deliberately broken.
- **Tested end to end:** a container stamped by an older, conforming definition is read with the history that records it and refused without it.
- **Falsification:** 9 mutants, all killed. Two survivors (a new required field, a dropped dependency) were real test gaps, closed first.
