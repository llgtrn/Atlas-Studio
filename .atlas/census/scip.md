---
id: donor-census-scip
type: reference
status: active
canonical: true
---
# Donor Census: SCIP

## Source

- Remote: https://github.com/sourcegraph/scip.git
- Commit: 4f50fbbd0ed945405cd8a4196af6477ea2a9f8b5
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED.

Deep census evidence: `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`.

Observed mechanisms: streaming `Index`, workspace metadata, canonical document paths, position encoding, symbol grammar, occurrence role bitset, symbol information and symbol relationships.

Decision: TARGET_MAPPED. SCIP principles feed `adapter/exchange/source_index`, `core/identity` symbol identity, and `core/model` symbol occurrence records. Runtime dependency remains `REFERENCE_ONLY`; SCIP protobuf is an exchange adapter, not Atlas canonical storage.

## Native Replacement

adapter/source-index exchange boundary

## Campaign decision (G66, first-50 #7)

The hypothesis was measured on real Atlas source before and after (`../evidence/campaign/07-scip.json`):

- **Before.** Function identity embedded the revision and the span. One inserted line gave 5 unchanged functions new identities. A rename or a move was a delete plus a create, and literal or signature edits were invisible at entity level. The span-free key collided 39 times across files. Self-recensus was file-granular.
- **Absorbed.** SCIP's position-free descriptor identity (package + namespaces + scope + name + method suffix), implemented natively as `core::recensus::entity`, with no SCIP code or dependency.
- **Atlas-native, not from SCIP.** Cross-revision correspondence from equal descriptors or equal (signature, body) token fingerprints. It never forces an ambiguous match.
- **REFERENCE_ONLY.** Occurrences, the role bitset and position encoding, relationships, the protobuf exchange format, and version-in-identity.

Terminal: ABSORBED. The checkout (226 files) was physically deleted; the commit sha, license and provenance remain recorded.

