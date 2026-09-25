---
id: donor-census-semgrep
type: reference
status: active
canonical: true
---
# Donor Census: Semgrep

## Source

- Remote: https://github.com/semgrep/semgrep.git
- Commit: 0516c0f23a3dceac5c8f5ff3fecd402af4450182
- Branch: develop
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED for the hypothesis slice (G70): `src/matching` and `languages/rust`.

## Native Replacement

runtime/query semantic pattern projection

## Campaign decision (G70, first-50 #11)

- **What Semgrep does.** It matches pattern ASTs with metavariables against `AST_generic`. It also matches through `id_resolved` canonical names from file-local import resolution (`Pattern_vs_code.ml` `try_alternate_names`); that resolution uses no types. Its Rust frontend is tree-sitter.
- **What Atlas has.** Atlas's only source-pattern consumer is effect spelling recognition (a closed, qualified set).
- **What the scan found.** Across the workspace: 0 std fs functions imported by name, 0 aliases, and 0 shadowing `Command`/`File`/`TcpStream` types, while 320 qualified calls are already matched. The 70 bare `write`/`read_to_string` calls are Atlas's own functions, which a resolution-free pattern would wrongly flag.

REFERENCE_ONLY, with re-evaluation triggers: a user-facing rule/query surface, or donor census evidence of import-dependent effect spellings. The checkout (10,792 files, 589 MB) was physically deleted. Evidence: `../evidence/campaign/11-semgrep.json`.

