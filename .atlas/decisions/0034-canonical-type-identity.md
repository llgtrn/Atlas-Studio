---
id: atlas.decision.0034.canonical-type-identity
type: decision
status: accepted
canonical: true
---
# ADR 0034 — Canonical type identity from native name resolution

## Context

TYPE identity was spelling-level. `TypeIdentity.canonical` was always `None`, so one type written three ways (`EpistemicStatus`, `crate::EpistemicStatus`, `crate::schema::EpistemicStatus`) was three unrelated identities.

The egglog cycle (G82, `../evidence/campaign/19-egglog.json`) measured this: 49 multi-spelling classes covering 598 occurrences, 13 of them differing only inside generic arguments. It also showed that string normalization is unsound, because `&std::path::Path` and `&syn::Path` share a stripped form.

Every such equality comes from name resolution. Bottom-up canonicalization therefore gives congruence by construction, and no e-graph is needed.

## Decision

1. **Canonicalize bottom-up** (`adapter::semantic::rust::resolve`). Every type occurrence is resolved in its lexical scope:
   - a workspace type becomes `<package> <module path>/<Name>#` (the G66 descriptor shape and SCIP's type suffix), following transparent aliases;
   - a std-rooted type or prelude type (`Vec`, `String`, `Box`, `Option`, `Result`) becomes its std path;
   - primitives stay as spelled;
   - references, slices, arrays, tuples, pointers and generic arguments compose structurally, with lifetimes dropped.
2. **No guesses.** These yield no canonical identity:
   - a generic parameter, a generic `Self`, or an alias whose meaning a bare path cannot carry;
   - a qualified path, a trait object, or `impl Trait`;
   - a glob-provided name in an open module.

   An explicit item in an open module is certain and does resolve.
3. **A per-file, all-occurrences claim.** The resolution engine is asked for TYPE as well. It claims `TypeIdentity { name: S, canonical: C, path: "" }` (DERIVED, shared across files) when three things hold:
   - the syntactic extractor recorded spelling S in the artifact;
   - every occurrence of S in that artifact resolves;
   - every occurrence resolves to C.

## Consequences

- **Claims:** 1,723 canonical type claims cover 819 canonical types. 84 of those types unite 176 spellings, including spellings that differ only in lifetimes or are `Self`.
- **Verified:** 3,077 named type occurrences agree with rust-analyzer 1.90.0 SCIP at the same anchor, with 0 disagreements. Three `std::io::Error` spellings name the public re-export of `std::io::error::Error`, the same type.
- **TYPE has two engines,** so the certificate's multi-engine blocker stops naming it. Each Rust artifact gains the engine's UNKNOWN TYPE obligation.
- **Falsification:** 11 mutants, all killed. Two survivors were real test gaps, closed first: shadowing of an in-scope type, and a spelling the extractor never recorded.
