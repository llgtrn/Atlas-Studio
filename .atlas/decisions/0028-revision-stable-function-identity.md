---
id: atlas.decision.0028.revision-stable-function-identity
type: decision
status: accepted
canonical: true
---
# ADR 0028 — Revision-stable function identity for self-recensus (SCIP descriptors, absorbed)

## Context

First-50 donor #7 (SCIP) was queued on the hypothesis that Atlas's semantic entities lose identity across revisions. G66 measured it on real Atlas source before deciding (`../evidence/campaign/07-scip.json`).

- **Identity embedded the revision and the span.** `FunctionIdentity::identity_key` includes both. Every commit re-identified all 1,811 functions. One inserted line gave 5 unchanged functions in the same file new identities. A rename or a move came out as a delete plus an unrelated create.
- **Some changes were invisible at entity level.** A literal-only body edit and a signature change showed no entity change at all.
- **Dropping the span is not a fix.** The span-free function key collides 39 times across 103 files (`main`, `root`, same-named test helpers). Scopes carry no module path.
- **Self-recensus was file-granular.** An unintended change to a second function inside a declared path was accepted silently.
- **The extraction cache never reuses a batch across a commit,** because batches embed the revision.

SCIP's own answer is that a symbol is a position-free descriptor chain (`scip.proto`, `Symbol`), and positions belong to occurrences. SCIP deliberately has no cross-revision correspondence: code modification is a non-goal (DESIGN.md), and the package version is part of every symbol.

## Decision

1. **Descriptors (absorbed from SCIP).** `core::recensus::entity::descriptor` builds, for every function and method: the package directory, the module namespaces (inferred from Cargo layout), the extractor's lexical scope segments, the name, and the method suffix, using SCIP escaping (`core language/adl/census/entity_name().`). Positions and revisions never enter it. The same function in another module is a different descriptor.

2. **Fingerprints.** Evidence for correspondence, never part of identity:
   - `signature_fingerprint` hashes the signature without the name: parameter types, return type, generics, ABI, qualifiers, declaration kind and owner.
   - `FunctionSignature::body_fingerprint`, emitted by the Rust extractor, is BLAKE3 over the body's token stream. It is position-, whitespace- and comment-insensitive, and literal-sensitive.
   - Visibility is tracked on its own.

3. **Correspondence (Atlas-native, not from SCIP).** `correspond` classifies from deterministic evidence only:
   - equal descriptors give SAME or CHANGED (signature, body, visibility);
   - otherwise, equal signature *and* body fingerprints give RENAMED, MOVED or MOVED_RENAMED;
   - anything else is DELETED or CREATED.

   It never forces a match:
   - several candidates, or a duplicated descriptor, give AMBIGUOUS;
   - bodiless declarations are never fingerprint-matched;
   - a split is reported as CHANGED + CREATED, never claimed as SPLIT.

4. **Self-recensus snapshot v3.**
   - `CensusSnapshot.entities` carries every function's descriptor, path and fingerprints; the census digest covers it.
   - Every non-SAME correspondence is an observed change `entity <KIND> <descriptor…>`. The intent must declare it in `entity_changes`, either exactly or as `<KIND|*> <descriptor prefix>*`. Declared entity changes that do not happen are "intended but unobserved".
   - Entity correspondence needs v3 on both sides. The v2 → v3 schema change is itself an observed change needing a reasoned acceptance.
   - v1/v2 snapshots keep their recorded digests.

## Consequences

- The same edit cases rerun through Atlas_N+1 report the ground truth (evidence file, phase 2):
  - A2: only the edited function;
  - C: RENAMED;
  - D: no change;
  - E: MOVED, with the visibility change as evidence;
  - F: the signature change;
  - G: CHANGED + CREATED;
  - H: MOVED_RENAMED;
  - a comment-only edit: nothing.

  An undeclared collateral change inside a declared file is now GENERATION_NOT_PROVEN.
- No descriptor collides across files in the self-scope (test-enforced).
- REFERENCE_ONLY: SCIP occurrences, the role bitset and position encoding, relationships, the protobuf exchange format, version-in-identity, and ordinal method disambiguators.
- Limits:
  - a local-variable rename is CHANGED body (no alpha-equivalence);
  - `#[path]` modules are not inferred;
  - types, traits and constants are not yet in the entity layer;
  - the `.atlas` container is unchanged, because the entity layer is a recensus projection and not packaged canonical state.
- P4: descriptor identity is the prerequisite for reuse across commits, and for invalidation that follows changed entities rather than changed files.
- Falsification: 23 mutants, all killed after one pattern fix. The runtime test also kills the module-path collision mutant.
