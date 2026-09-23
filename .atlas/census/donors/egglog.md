---
id: atlas.census.donors.egglog
type: donor-census
status: active
canonical: true
---
# egglog Donor Census

## Source

- Donor: `egglog`
- Remote: `https://github.com/egraphs-good/egglog`
- Commit: `90635860397ce710f8c0a4eeb04154a8ebc3ac05`
- Clone path: `.atlas/temporary/donors/egglog`
- License: MIT, root `LICENSE` file, `Copyright (c) 2025 egraphs-good`
- Donor gap: `ATLAS_DECISION_LAYER_EQUIVALENCE_CLASS_AND_COST_EXTRACTION_MECHANISM`
- Quarantine: `.claude/` -> `_donor-quarantine.claude/` (dir), `CLAUDE.md` -> `CLAUDE.md.donor-untrusted` (file). Verifier: clean.

## Framing (read this before the rest)

**egglog is being censused as a candidate mechanism donor for Atlas's decision/optimization/architecture-exploration
machinery — the "Jev-class" structural decision reasoning that ranks and selects among alternative engineering
proposals (`.atlas/contracts/SELECTED-DESIGN.md`: "typed Jev-class decision proposals for rank/score/route
decisions").** It is explicitly **not** a census of egglog as a replacement for, or model of, Atlas's core program
IR (AtlasX/HIR/MIR/LIR). egglog's own README states it is "a language for writing equality saturation
applications... the successor to the Rust library egg... combines the power of equality saturation and Datalog."
The question this census answers is narrower: *does egglog's e-graph equivalence-class model + cost-based
extraction usefully inform how Jev could represent "these N alternative structural proposals are
known-equivalent/interchangeable" and search/rank among them with evidence-weighted extraction?* **Atlas is not
adopting egglog as a runtime dependency at any point in this census** — this is architecture study toward a
possible Atlas-native reimplementation for the decision layer only, evaluated against Atlas's own
`SelectedDesign`/`DecisionProposal`/`CandidateChangeSet` records already specified in `SELECTED-DESIGN.md`.

## Coarse Inventory

- Files observed (post `.git` removal): 569
- Bytes observed: ~11 MB (`du -sh` = 11M)
- Top-level crates (Cargo workspace members, from directory structure and `Cargo.toml`/`Cargo.lock`): `egglog` (the
  main crate, `src/`), `core-relations` (the relational/Datalog query engine), `union-find` (standalone union-find
  crate), `egglog-ast`, `egglog-bridge`, `egglog-reports`, `numeric-id`, `concurrency`
- Languages/signals: Rust (dominant; `src/`, `core-relations/src/`, `union-find/src/`, `egglog-bridge/src/`, etc.),
  `.egg` (egglog-language test/example programs, ~140 files under `tests/`), Python (small tooling: `scripts/`,
  `global-var-formatter/`), Racket (`scripts/minimize.rkt`, a test-case minimizer), HTML/JS (`www/`, `wasm-example/`
  — a WASM build target for running egglog in-browser, distinct from WebAssembly-the-donor-lane)
- Full source tree staged with nested `.git` removed.

## Mechanisms Census (read from real source under `src/`, `core-relations/src/`, `union-find/src/`)

### E-graph core: e-classes, e-nodes, union-find

- **Union-find** (`union-find/src/lib.rs`, 104 lines, standalone crate): a genuinely simple, verified-from-source
  implementation — `parents: Vec<Value>`, path-halving `find` (each step reparents to grandparent), and `union`
  that always attaches the numerically-larger id under the smaller ("union by min id" — the file's own doc comment
  is explicit that this is a chosen heuristic, "does not guarantee the same asymptotic complexity as... union by
  rank," adopted because it "reduce[s] the number of ids perturbed during congruence closure"). A second,
  concurrent variant (`union-find/src/concurrent/`) exists for the parallel (`-j`) execution path. This is the
  textbook e-class-representative mechanism: an e-class's identity *is* its union-find root.
- **E-nodes**: `Enode<'a>` is defined at `src/exec_state.rs:578`, alongside `FunctionEntry<'a>` — an e-node is
  represented as a row in a relational `Function` (table), not as a freestanding tree node; `Function` itself is
  defined at `src/lib.rs:351`. This is the key structural fact: **egglog's e-graph is not a graph-of-nodes in the
  classic egg sense — it is a Datalog database where each user-declared "constructor"/relation is a table, and an
  e-node is one row of that table whose columns are e-class ids (union-find values) for its children plus an
  output e-class id for itself.** Congruence closure (making sure two rows agree once their arguments' e-classes
  are found equal) is implemented as table *rebuilding*: `core-relations/src/table/rebuild.rs` (`do_rebuild`,
  `rebuild_incremental`, `rebuild_nonincremental`) walks a table's rows through the union-find's `Rebuilder` and
  either does a full or an incremental scan-and-rewrite of rows whose argument columns point through stale
  (non-canonical) union-find ids, verified directly from source, including a dedicated `rebuild_index` structure
  used to make incremental rebuilds efficient.

### Equality saturation (the algorithm)

- `src/scheduler.rs` (1162 lines) implements the `run-schedule`/`saturate`/`run` combinator language (egglog
  programs write schedules like `(run-schedule (saturate (run)))`, confirmed via grep hits at scheduler.rs:607 and
  :791) that drives the fixed-point loop: repeatedly matching rule left-hand sides against the current database,
  firing right-hand-side actions (which add rows / union e-classes), and rebuilding until no further additions
  occur (saturation) or a schedule-specified condition/limit is reached. `SchedulerContext` (`scheduler.rs:20`) and
  `Matches`/`Match` (`scheduler.rs:84`, `:99`) carry the per-round match results.

### Datalog integration (egglog's distinctive feature)

- **This is the single most load-bearing architectural fact in this census.** egglog is not "egg plus a bolted-on
  Datalog layer" — its rule-matching core *is* a Datalog-style relational query engine. `core-relations/src/`
  implements a general-purpose relational database (`Table`, `TableSpec`, `ColumnId`, `RowId`) with a **Free Join**
  query executor (`core-relations/src/free_join/mod.rs`, doc comment: "Execute queries against a database using a
  variant of Free Join" — a worst-case-optimal join algorithm, the generic-join/free-join family used by modern
  Datalog engines to avoid the naive pairwise-join blowup of classic e-graph matching). `src/core.rs` (1291 lines)
  compiles egglog's surface rule syntax down into this relational IR: `Query<Head, Leaf>` (core.rs:378),
  `GenericAtom`/`GenericAtomTerm` (core.rs:242, :322 — literally "atoms" in the Datalog sense: a relation name
  applied to variable/constant terms), `GenericCoreAction`/`GenericCoreActions` (core.rs:485, :502 — the RHS
  action list: inserting rows, unioning e-classes, deleting rows), and `GenericCoreRule` (core.rs:905) which pairs
  a `Query` (LHS pattern, evaluated via the Free Join engine over the table database) with `CoreActions` (RHS).
  Semi-naive evaluation and incremental hash-indexing (`core-relations/src/hash_index/`) are present, consistent
  with a production Datalog engine, not a toy pattern-matcher. **This is the mechanism that answers "how is
  egglog's e-graph matching different from egg's": egg matches patterns tree-recursively against an explicit
  e-graph; egglog compiles rule LHS patterns into Datalog queries and executes them with a worst-case-optimal join
  over a relational table store whose rows happen to be e-nodes.**

### Analysis / rewrite / extraction mechanisms

- **Rewrite rules**: surface syntax lives in `src/ast/` (`expr.rs`, `desugar.rs`, `parse.rs`); a `rewrite` command
  desugars to a `rule` (LHS query + RHS union action), consistent with the `GenericCommand` enum referenced in
  `src/lib.md` as the canonical language reference ("one variant per top-level egglog command (`Datatype`,
  `Function`, `Rule`, `RunSchedule`, ...)").
- **Analyses**: egglog's analog of egg's e-class analyses is expressed through ordinary egglog relations plus
  `:merge` functions on a `Function` declaration (a user-specified expression for combining two values written to
  the same slot when e-classes merge) — i.e. analysis propagation is not a separate built-in mechanism, it is the
  same relational-table-plus-rebuild machinery used for everything else. This is a meaningfully different design
  choice from egg's separate `Analysis` trait, and is a direct consequence of the Datalog-first architecture above.
- **Extraction**: `src/extract.rs` (1027 lines) is the term-selection layer, paired with `src/termdag.rs` (733
  lines, a `TermDag`/`TermId`-indexed shared-structure DAG used to reconstruct extracted terms without duplicating
  shared subexpressions). `lib.md` documents the public extraction surface directly: `EGraph::extract_value`
  (single value, default cost model), `EGraph::extract_best` / `extract_variants` (batch/variant extraction), and
  `_with_cost_model` variants accepting custom models — i.e. extraction is explicitly designed as a pluggable-cost
  operation from the top of the public API, not a single hardcoded policy.

### Cost models for extraction (read in full from `src/extract.rs`)

Verified, concrete, and directly relevant to a Jev-class "rank candidates" mechanism:

- `Cost: Clone + Ord` is the *minimum* bar — anything totally orderable can rank candidates.
- `MonoidCost: Cost` adds `identity()` and `combine(self, other) -> Self`, with an explicit doc-comment law
  ("associative, commutative, deterministic, non-panicking, monotone") that Rust cannot enforce but the trait
  documents as a contract — this is the algebraic structure that lets a DAG extractor charge a shared dependency's
  cost exactly once rather than once per use site (the classic tree-vs-DAG extraction cost-double-counting problem
  is explicitly named and solved via this monoid law).
- `DagCostModel<C: MonoidCost>` computes **marginal** costs (`base_value_cost`, `enode_cost` excluding children,
  `container_cost` excluding elements) — the model that charges shared structure once.
- `TreeCostModel<C: Cost>` is the tree-extraction-facing trait; it retains a separate `EnodeCost`/`ContainerCost`
  *annotation* type distinct from the final `Cost` type specifically so a node's cost computation can carry
  contextual information (e.g. "this is a constructor of kind X") without polluting the ranking type itself — a
  genuinely useful separation-of-concerns pattern for a ranking system that both scores and explains its scores.
  `TreeCostModelFromDag<M>` adapts a `DagCostModel` to tree extraction by literally folding marginal costs with
  `combine`.
- `AdditiveCostModel` (the shipped default: `node_cost: DefaultCost = u64`, default `1`) is the reference
  implementation: every constructor costs 1 unless overridden by a `:cost` declaration in the egglog program
  itself, and total cost is the sum over the extracted tree. `DEFAULT_COST_MODEL` is a `TreeCostModelFromDag`
  wrapping it.
- **Concrete takeaway for Jev-class design**: egglog's cost-model API cleanly separates (a) *what is being ranked*
  (any `Cost: Clone + Ord`, not hardcoded to a scalar), (b) *how a node's local contribution is computed*
  (`enode_cost`, given the whole `EGraph`/`Function`/`Enode` context — so a cost function can read arbitrary
  e-graph/database state, not just local syntax), and (c) *how child/element costs combine* (`fold_enode_cost`,
  which for DAG-derived models is required to be a monoid). This three-part shape (local score, aggregation law,
  pluggable score type) is a stronger and more explicit contract than a single opaque `score(candidate) -> f64`
  function, and is architecturally close to what a Jev-class "rank N equivalent structural proposals" mechanism
  would need if it wants both tree-shaped candidates (pick one whole proposal) and DAG-shaped candidates (reuse a
  sub-decision across multiple proposals without double-charging its cost/risk).

### How egglog represents equivalence

Equivalence is **not** a separate predicate or explicit equality edge — it is a value-level identification enforced
by the union-find: two terms are "equivalent" exactly when their e-class ids resolve to the same union-find root,
and this identification propagates automatically through congruence (table rebuilding) to every relation that
mentions either id. There is no separate "equivalence graph" data structure to reconcile against the "value graph"
— they are the same structure by construction. This is the crux of what would need re-deriving for Jev: an
e-class is simultaneously (1) an identity for "these representations are the same decision-relevant entity" and
(2) the join key that automatically fans out equivalence to every relation that references it, at zero extra
bookkeeping cost to rule authors.

## Atlas Comparison (decision-layer framing only)

- **Atlas today**: `SELECTED-DESIGN.md` already specifies `DecisionProposal`, `CandidateChangeSet`, and a
  candidate-set + decision-and-implementation-lineage model ("candidate set; DecisionProposal records used
  materially; CandidateChangeSet records...") produced by "a research provider, Jev-class decision provider,
  synthesis provider or verification provider," none of which may "silently grant itself selection authority."
  This is a **process/authorization** model for how a decision gets proposed, ranked, and confirmed (including a
  human-confirmation gate under `HYBRID` mode) — it does not yet specify a **data structure** for *how multiple
  known-equivalent/interchangeable structural alternatives are represented and efficiently re-ranked as new
  evidence or new candidates arrive*. This is exactly the gap egglog's e-graph + cost-model pair is a candidate
  mechanism for.
- **egglog's candidate contribution**: use an e-class-shaped equivalence relation over `CandidateChangeSet`/
  `DecisionProposal` identities (an Atlas-native "decision e-class") so that (a) discovering two proposals are
  interchangeable is a cheap union rather than a re-derivation of the candidate set, (b) evidence/cost accrual for
  a shared sub-decision is charged once via a monoid-cost extraction rather than once per proposal that reuses it,
  and (c) ranking becomes a pluggable-cost extraction over the resulting structure instead of a bespoke scoring
  pass per decision type. The Free Join / Datalog-relational substrate underneath is a secondary, more speculative
  idea: if Jev's rule/evidence matching ever needs to join many typed evidence relations to find applicable
  decision rules, egglog's worst-case-optimal join engine is a stronger reference than naive nested-loop matching
  — but Atlas has no such evidence-join engine today to compare against directly (REFERENCE_ONLY, no current Atlas
  mechanism to compare pass-by-pass).
- **What is explicitly NOT being proposed**: egglog's e-graph is not a candidate for Atlas's core program
  representation (AtlasX/HIR/MIR/LIR) — those already have their own contracts (`ATLAS-TO-ATLASX.md`,
  `COMPILER-IR-SCHEMAS.md`) and represent a single program's control/data flow, which is a different problem from
  "rank N alternative structures." Conflating the two would be a real design error this census is explicit about
  avoiding.

## Disposition (per concept)

| Concept | Disposition | Notes |
|---|---|---|
| E-class/union-find equivalence identity | **STUDY** | Directly relevant candidate shape for a Jev-class "these proposals are interchangeable" relation. Worth a small Atlas-native prototype before committing. |
| Congruence closure via table rebuild | **STUDY** | Elegant, but only pays off if Jev's decision records are numerous/interrelated enough to need automatic propagation; verify need before implementing. |
| Monoid cost-model API (Cost/MonoidCost/DagCostModel/TreeCostModel split) | **ADAPT** | The three-part shape (local cost, combination law, pluggable cost type) is a strong, small, directly portable API design for an Atlas-native ranking/extraction mechanism — reimplement the *shape*, not the Rust trait objects themselves. |
| AdditiveCostModel (default scalar sum) | **ADAPT** | Reasonable default/reference implementation to imitate for a first Jev-class scorer; trivial to reimplement natively. |
| Free Join / worst-case-optimal Datalog join engine | **DEFER** | Real and impressive, but Atlas has no current evidence-relation-joining workload to justify the complexity; recensus if/when Jev needs multi-relation rule matching at scale. |
| Datalog-as-substrate architecture (e-node = table row) | **STUDY** | The core insight worth carrying forward conceptually even without adopting Datalog: representing decision candidates as relational rows (not a bespoke tree) makes "is X equivalent to Y" and "what depends on X" uniform, cheap operations. |
| `.egg` surface language / parser / CLI | **REJECT** | Not relevant to Atlas; egglog's own DSL and tooling are out of scope for a decision-layer mechanism census. |
| Semi-naive incremental evaluation / hash indexing internals | **DEFER** | Implementation-detail-level optimization, only relevant once an Atlas-native reimplementation exists to optimize. |

### Things NOT to copy

- Do not add `egglog` (or `egg`) as a Cargo dependency of any Atlas crate. This census evaluates the *idea*
  (e-graph equivalence classes + cost-based extraction) for an Atlas-native reimplementation, not the crate.
- Do not port egglog's `.egg` surface syntax, parser, or CLI — Atlas's decision layer has its own
  `SelectedDesign`/`DecisionProposal` record shapes already specified in `SELECTED-DESIGN.md`; egglog's language
  is not a target syntax.
- Do not conflate this donor with Atlas's core program IR work — nothing here should be cited as evidence for or
  against AtlasX/HIR/MIR/LIR design decisions (`ATLAS-TO-ATLASX.md`, `COMPILER-IR-SCHEMAS.md`).
- Do not cite the Free Join/Datalog engine as "already proven at Atlas's scale" — it is proven at egglog's
  workload scale (rewrite-rule term optimization), which is not demonstrated to be the same shape as an
  evidence-relation-joining decision-layer workload.

## Known Risks / Gaps in This Census

- Read depth was targeted at the specific mechanisms the task asked for (e-graph core, equality saturation,
  Datalog integration, analysis/rewrite/extraction, cost models, equivalence representation); the `proofs/`
  subsystem (`src/proofs/`, proof-producing egglog — checking, encoding, extraction under proof mode) was located
  but not deep-read, and may be relevant to a future Atlas decision-lineage/audit-trail requirement
  (`SELECTED-DESIGN.md`'s "decision and implementation lineage" section) — flagged for recensus, not covered here.
- `core-relations`' parallel execution path (`parallel.rs`, `parallel_heuristics.rs`, the concurrent union-find)
  was identified but not deep-read; if Jev's decision layer ever needs concurrent candidate evaluation, this
  deserves a dedicated pass.
- No execution of any donor code was performed (per the mandatory quarantine instructions) — all findings are from
  static source reading; cost-model laws (associativity/commutativity/monotonicity) are verified as documented
  contracts, not as tested/proven properties in this repository.
- `egglog-bridge`, `egglog-reports`, and `numeric-id` crates were inventoried (directory-listed) but not read in
  depth; they appeared to be supporting/integration crates (numeric-id: typed-id newtype helpers; egglog-bridge:
  a higher-level Rust-embedding API over `core-relations`) rather than core-mechanism-bearing, but this is an
  inference from names/structure, not a verified read.
- This census does not verify egglog's own performance claims (CodSpeed benchmarks) or correctness — no benchmark
  or test was executed.

## Census State

CENSUSED. Targeted mechanism census complete for e-graph core (e-classes/e-nodes/union-find), equality saturation
scheduling, Datalog integration (the Free Join relational engine and Query/CoreRule IR), analysis/rewrite/
extraction mechanisms, and the cost-model API, all read from real source under `src/`, `core-relations/src/`, and
`union-find/src/`, explicitly scoped to Atlas's decision-layer donor gap and explicitly not extended to core-IR
comparisons.
