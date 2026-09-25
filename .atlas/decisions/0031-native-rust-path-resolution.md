---
id: atlas.decision.0031.native-rust-path-resolution
type: decision
status: accepted
canonical: true
---
# ADR 0031 — Rust path calls are resolved natively by a second CALL engine (rust-analyzer, absorbed)

## Context

Atlas recorded 12,671 CALL sites (16,072 after G74's anchor fix), and none named its callee. The syntactic extractor sees where a call is and who makes it, never whom it calls.

G73 measured two things (`../evidence/campaign/14-rust.json`):
- The pinned toolchain's rust-analyzer resolves about 89% of those calls.
- Stable rustc exposes no machine-readable resolution.

G74 rejected putting the oracle into the census. CI's pinned toolchain has no rust-analyzer, so a census built on it would depend on the environment.

First-50 donor #4 (rust-analyzer) recorded the blocking question: is there a bounded slice of name resolution that moves CALL observations off UNRESOLVED without crossing the never-rustc boundary? `hir-def` answers it: `nameres` resolves paths by module and import rules alone. Type inference (`hir-ty`) is needed only for method calls and `Type::assoc`.

## Decision

1. **Native resolver** (`adapter::semantic::rust::resolve`). It takes `hir-def`'s DefMap, reduced to what path calls need:
   - every crate target's module tree, with `mod x;` located as `mod_resolution.rs` does (mod-rs and non-mod-rs rules, `#[path]`);
   - item scopes in the types and values namespaces, with visibility;
   - `use` imports (named, renamed, grouped, `self`, globs), recomputed to a fixed point, where an item shadows a named import and a named import shadows a glob;
   - block scopes;
   - the extern prelude of workspace crates, from the manifests' dependency keys;
   - the crate-wide inherent impl lookup, then trait impls, for `Type::f` and `Self::f`.

   It runs under the syntactic extractor's recursion-risk pre-scan, on the same large stack.
2. **Never a guess.** These cases stay unresolved with a reason:
   - a local binding or parameter of the name anywhere in the function;
   - a generic-parameter path;
   - an OPEN scope (item macros, an unresolvable glob, a missing module file), which never falls through to an outer scope and whose glob-provided names are not trusted;
   - an unresolvable named import, which still binds its name;
   - conflicting globs, which are AMBIGUOUS (`hir-def` keeps the first);
   - trait dispatch;
   - constructors.
3. **A second engine, not an overwrite** (`runtime::census::resolution`, `atlas.resolution.rust-paths`).
   - Each resolution observes the syntactic extractor's own CALL claim (same `record_id`) with `STATIC_RESOLVED`, and names the callee by the extractor's own FunctionIdentity record.
   - A resolution that matches no claim is a diagnosed engine disagreement.
   - The engine is asked for CALL only, and accounting closure holds each engine to what it was asked for.
   - Normalization treats UNRESOLVED as "no callee claim": it cannot disagree with a resolution, but two different resolutions can disagree.
   - The graph adds a CALLS edge from the call site to the callee.
4. **Honest certificate.** MULTI_ENGINE_RECONCILIATION_ABSENT now names every evaluated dimension that has no second engine. A second engine on one dimension reconciles that dimension only. Before this change, one dimension's second engine would have cleared the blocker for all twelve.
5. **Oracle, not input.** The pinned rust-analyzer SCIP index is the differential verification oracle, run at evidence time and never in the census.
6. **REFERENCE_ONLY:**
   - `hir-ty` inference and method resolution (the 11,704 method calls);
   - macro and proc-macro expansion;
   - salsa-backed IDE queries;
   - base-db and project-model (the Cargo dependency census owns the crate graph).

## Consequences

- **Resolved calls:** 3,731 of the workspace's 6,165 profile path calls resolve.
  - All 3,731 agree with rust-analyzer 1.90.0 SCIP at the same anchor.
  - Recall is 95.1% of oracle-resolvable workspace calls. Every miss is a refusal: macro-generated items, derived impls, locals, cfg alternates.
- **A defect found by the oracle** was fixed before promotion. A block import `use super::super::tests::element` was mis-resolved: chained `super` was not honored, and the failed import let the lookup fall through to an outer glob.
- **CALL has two engines.** The certificate's multi-engine blocker now names the eleven single-engine dimensions.
- **Falsification:** 22 mutants, all killed. Two survivors (open-glob propagation, attachment to a missing claim) were real test gaps, closed first.
- **Extinction:** the rust-analyzer checkout is physically deleted (2,345 files).
