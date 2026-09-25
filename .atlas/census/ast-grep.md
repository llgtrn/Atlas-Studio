---
id: donor-census-ast-grep
type: reference
status: active
canonical: true
---
# Donor Census: ast-grep

## Source

- Remote: https://github.com/ast-grep/ast-grep.git
- Commit: 6175e07b668b388a1a326d8fc543c4856eb5f54b
- Branch: main
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: DEEP_CENSUSED for the hypothesis slice (G71): `crates/core` (match_tree, meta_var, replacer).

## Native Replacement

runtime/refactor structural rewrite primitives

## Campaign decision (G71, first-50 #12)

- **What ast-grep does.** It matches tree-sitter nodes structurally with meta variables. `MatchStrictness` (Cst, Smart, Ast, Relaxed, Signature, Template) chooses which trivia a match ignores, and a template replacer rewrites.
- **Why Atlas does not need it now.** No Atlas stage rewrites source: construction (P6) is blocked behind `.atlasx`. The only source edits are the coordinator's external scratch tooling. That tooling's textual-pattern misses after rustfmt reflow (about 8 across G63–G70, plus one truncation) were all detected, so they are not an Atlas correctness gap. Token-level comparison is already native (G66 body fingerprints).

REFERENCE_ONLY, until Atlas gains a native mutation or construction capability. The checkout (318 files) was physically deleted. Evidence: `../evidence/campaign/12-ast-grep.json`.

