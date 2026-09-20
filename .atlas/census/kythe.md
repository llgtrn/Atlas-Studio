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

