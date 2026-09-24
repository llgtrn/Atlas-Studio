---
id: donor-census-souffle
type: reference
status: active
canonical: true
---
# Donor Census: Souffle

## Source

- Remote: https://github.com/souffle-lang/souffle.git
- Commit: a1303be3c0166400dee3d1f36f0d96abe03e6901
- Branch: master
- Retrieved: 2026-09-20T12:08:55.4960343Z

## Census State

Status: COARSE_CENSUSED (advanced from SKELETON). One essential mechanism is now understood and
evidenced -- see `.atlas/genome/technology/souffle-provenance-backed-derivation.md`:

- provenance-backed derivation (`src/ast2ram/provenance/`): every derived tuple in a
  provenance-enabled relation carries two fixed, inline auxiliary columns (`rule_num`, `level_num`)
  identifying which rule and how deep its own derivation was, plus an auto-generated per-rule
  "subproof" subroutine that re-executes that rule as a targeted existence check to reconstruct a
  specific tuple's justification on demand, recursively, with termination guaranteed by the level
  metadata's strict-descent property. This is a real, shipped mechanism for making DERIVED (not just
  observed) facts carry a reconstructable evidence trail, directly relevant to R5's own
  "evidence-preserving query dependencies" goal.

Souffle is a large, AOT-compiling C++ Datalog engine (AST -> RAM relational-algebra IR -> interpreted
or C++-synthesized execution). Still needing classification: the RAM IR itself, the interpreter, the
C++ synthesiser (`src/synthesiser/` -- directly relevant to Atlas's own eventual compiler-backend
work if ever revisited), the parser/AST layer, and the plain (non-provenance) `seminaive` translation
strategy.

## Native Replacement

runtime rule/inference substrate -- provenance-backed derivation is a candidate mechanism for making
a future Atlas-native incremental/derived-fact layer (R5/R6) carry reconstructable justification for
every derived fact, not only leaf observations, complementing rather than competing with the
Salsa/datafrog/differential-dataflow mechanisms already censused in the same W3 lane.

