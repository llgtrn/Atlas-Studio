---
id: atlas.decision.0009.indexed-graph-node-dedup
type: decision
status: accepted
canonical: true
---
# ADR 0009 — Indexed node deduplication in engineering-graph construction

## Context

G39 measured the whole recensus pipeline (ADR 0008). `summarize_system_graph_with_dependencies` took about 166 s of a 172 s debug `systemize` on this repository. Of that, `build_system_graph` alone took 145 s, for 47,176 nodes and 60,333 edges.

The cause was `ensure_node`. It deduplicated by scanning every existing node linearly, so construction was quadratic: about 2.2×10⁹ string comparisons at this size. The cost grows with every observation Atlas extracts. It was the dominant R5 edit-to-recensus cost and the bottleneck that the Salsa and differential-dataflow blocking questions were really measuring.

## Decision

`EngineeringGraph` gains a derived, non-serialized `node_index: NodeIdIndex`. It is a set of node ids that catches up incrementally on appended nodes, so `ensure_node`'s membership check is O(1) amortized. It is excluded from equality and serialization.

- **Contract.** `nodes` is append-only between lookups. That is the only way Atlas mutates it: nodes are pushed through `ensure_node` or directly, and are never re-identified, reordered or replaced.
- **Correctness under that contract.**
  - The index catches up on direct pushes.
  - It rebuilds from scratch when `nodes` shrinks.
  - Debug builds assert a sentinel (the last indexed id), so a future in-place rewrite fails loudly in tests rather than silently corrupting deduplication.

## Evidence

- **Differential output.** The full `atlas-systemizer graph --root .` JSON is byte-identical between the old binary (HEAD `14cc307b9d`) and the new one. The `systemize` reports are identical.
- **Speed** (debug build):

  | Command | Before | After | Speedup |
  |---|---|---|---|
  | `systemize` | 187 s | 12.9 s | 14.5× |
  | `graph` | 196 s | 9.9 s | 20× |

  Graph summarization itself dropped from 166 s to 0.75 s.
- **Tests.**
  - A property test against the linear-scan rule under random appends, direct pushes and duplicates.
  - A shrink-rebuild test.
  - A debug-sentinel test.
  - A 100,000-node scaling guard.
- **Mutation.** Restoring the linear scan fails the scaling guard, a non-catching-up index fails the property tests, and skipping the shrink rebuild fails the shrink test.

## Consequences

The remaining debug profile is about 13.5 s:

| Stage | Time |
|---|---|
| git snapshot | 1.6 s |
| extraction (the ADR 0008 cache applies here) | 2.35 s |
| census | 1.0 s |
| normalization | 0.9 s |
| graph | 0.75 s |
| accounting | 0.5 s |
| report serialization and output | ~6 s |

Recompute is now cheap enough that dependency-tracked incremental invalidation (Salsa, differential-dataflow) has no measured justification at this repository's scale. Their blocking questions are re-aimed at corpus scale: the donor workspaces.
