---
id: donor-census-kythe
type: reference
status: active
canonical: true
---
# Donor Census: Kythe

## Source

- Remote: https://github.com/kythe/kythe.git
- Commit: 69141f022689a611e8a4a1d9b08a3783a2e8a9ed
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED.

Deep census evidence: `.atlas/census/donors/source-intelligence-lane-2026-09-20.md`.

Observed mechanisms: VName identity axes, graph `Entry` facts/edges, rewrite rules, graph serving, xref decorations, cross-reference queries and paged edge lookup.

Decision: TARGET_MAPPED. Kythe principles feed `core/identity`, `core/model`, `runtime/link` and future ATLASX graph/xref projections. Runtime dependency remains `REFERENCE_ONLY`; Kythe tickets are not Atlas canonical user-facing IDs.

## Native Replacement

source graph symbol identity

## Campaign decision (G67, first-50 #8)

The recorded absorption condition occurred. Symbol and type identity keys lacked a location, so 203 ids were shared by definitions and spellings in different files. The engineering graph kept only the first of each (609 nodes lost), and normalization merged them as one claim.

Kythe's VName `path` was absorbed as an identity field: `SymbolIdentity.path` and `TypeIdentity.path`, where unresolved spellings are file-scoped and resolved canonical types are shared. REFERENCE_ONLY: corpus, root, language, opaque signatures and the Entry store.

Terminal: ABSORBED. The checkout (2,396 files, 22 MB) was physically deleted. Evidence: `../evidence/campaign/08-kythe.json`.

