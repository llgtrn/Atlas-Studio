---
id: atlas.decision.0029.identity-carries-location
type: decision
status: accepted
canonical: true
---
# ADR 0029 — Symbol and type identity carry their source location (Kythe VName path, absorbed)

## Context

First-50 donor #8 (Kythe) was expected to be REFERENCE_ONLY. The condition recorded for absorbing it was "a concrete defect traced to string-joined identity keys that `escape_identity_field` does not already prevent". G67 checked the self-census for exactly that (`../evidence/campaign/08-kythe.json`).

- **The keys never collide lexically.** 64,407 typed records produce no id shared by distinct subjects.
- **But 203 ids were shared by records from different files with identical subjects.** 64 were symbol definitions and uses, and 139 were type spellings. For example, `tests::report` DEFINITION in `core/src/visual/mod.rs` and in `core/src/census/delta.rs` had one id. `SymbolIdentity` and `TypeIdentity` carried a lexical scope but no location, and scopes carry no module. So 4,142 symbol records collapsed to 3,999 ids and 2,675 type records to 2,298.
- **The damage.** `engineering_graph::ensure_node` keeps the first node per id: the other definitions never became graph nodes, and their provenance was lost. Normalization reads a shared `record_id` as one semantic claim (its multi-engine agreement contract), so different claims were merged as agreement.

Kythe's VName (`kythe/proto/storage.proto`) makes `path`, "the relative path to the file containing the code", a first-class identity field. It deliberately keeps time out of names: "a name should say what something is, not when". G66 already applied that second principle where revision-free identity is consumed.

## Decision

1. `SymbolIdentity.path` and `TypeIdentity.path` hold the source artifact that spells them. The Rust extractor sets them, and the field is escaped into `identity_key`.
2. **An unresolved type spelling is file-scoped:** its meaning depends on that file's imports. A resolved `canonical` type is one type everywhere, so its key ignores the path.
3. **Record identity stays revision-scoped** as NORMALIZATION.md requires.
4. **REFERENCE_ONLY:**
   - VName corpus and root (identities already carry `RepositoryId`);
   - the language field (`FunctionIdentity` has it);
   - opaque analyzer signatures;
   - the unified fact/edge Entry store.

## Consequences

- Every symbol and type record has its own id (4,147 → 4,147 and 2,675 → 2,675), and no id spans two files. The engineering graph gains the 609 previously dropped nodes (72,836 → 73,445).
- Test-enforced at three levels:
  - key unit tests;
  - the adapter, which checks that the same source in two files gives disjoint ids;
  - the self-scope, which checks that no symbol or type id spans files.
- Falsification: 7 mutants, all killed after closing one real test gap. The symbol embedded in `FunctionIdentity` had no assertion; its path is identity-redundant (the function key includes `span.path`) but is now asserted as data.
