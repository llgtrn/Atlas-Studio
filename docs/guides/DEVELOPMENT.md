---
id: atlas.guide.development
type: runbook
status: active
canonical: true
---
# Atlas Development Guide

## Preconditions

Resolve the configured fleet, choose the intended repository, read its North Star, system architecture, system blueprint and system contract, and resolve the exact selected-repository base SHA.

## Documentation Gate

Run docs audit. Any missing control document, required heading, frontmatter field, duplicate ID, broken internal reference or missing supersession link blocks coding admission.

## Implementation

Prepare work for one repository only. Scope implementation to the selected repo and bounded plan. Backend engineering implementation is Rust; Graph Studio implementation is TypeScript/TSX.

## Verification

Run repository-local tests, Atlas standards checks, contract tests and evidence checks appropriate to the scope.

## Reconciliation

After verified semantic change, update the existing canonical docs in the same repository so North Star, architecture, blueprint and contracts remain current.

## Rollback

If implementation or proof fails, do not merge. Preserve or discard the isolated branch/worktree and regenerate from the unchanged canonical base as appropriate.
