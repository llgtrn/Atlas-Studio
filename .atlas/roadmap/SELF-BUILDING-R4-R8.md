---
id: atlas.roadmap.self-building-r4-r8
type: blueprint
status: canonical
canonical: true
---
# Atlas Studio Self-Building Execution Map — R4 through R8

## Purpose

This document is the canonical execution map for building **Atlas Studio itself** from R4 through R8 by repeatedly censusing approved OSS donors and the complete admitted dependency graphs that actually provide their behavior.

It exists to prevent implementation agents from inventing their own sequencing, redefining census, treating donor repositories as permanent architecture, or declaring absorption/extinction without proof.

This document does not replace the global phase meanings in `ROADMAP.md`, the dependency rules in `../contracts/DEPENDENCY-CENSUS.md`, or the donor state machine in `DONOR-ABSORPTION-ROADMAP.md`. It fixes how those contracts execute together from R4 through R8.

## Canonical line of record

`main` is the sole canonical line. A same-named branch, `claude/r4-semantic-materialization-jz2z44`, was merged into `main` once (PR #26) and has since re-diverged independently, accumulating 216 of its own commits with no unique capability over `main` -- every one of them either predates a refactor/hardening/gap-closure `main`'s line already performed, converged on content already identical on `main`, or is stale documentation contradicted by its own code (full per-file evidence: `.atlas/evidence/verification/main-vs-r4-semantic-materialization-branch-reconciliation.json`). That branch was intentionally left unmerged, undeleted and un-force-pushed; a future session reusing that branch name should reset it from the current `main` tip rather than building further on its old, fully-superseded tip.

## Scope

The current construction target is **Atlas Studio itself**.

The R4→R8 self-building loop is:

~~~text
current Atlas capability
        ↓
admitted donor roots
        ↓
transitive dependency closure for every admitted resolution context
        ↓
census at the deepest semantic level Atlas currently supports
        ↓
discover mechanisms / invariants / useful ideas / missing capabilities
        ↓
attribute each discovery to its actual provider scope
        ↓
explicit absorption disposition
        ↓
deep census selected provider scope and required dependency subtree
        ↓
Technology Genome / durable evidence
        ↓
Atlas-native design
        ↓
implementation under core/runtime/adapter/apps
        ↓
verification / benchmark where material
        ↓
recensus Atlas
        ↓
prove donor source/runtime dependency is zero for the absorbed scope
        ↓
durable-knowledge gate
        ↓
physical donor-source deletion
        ↓
post-delete recensus
        ↓
EXTINCT
        ↓
stronger Atlas capability
        ↺
~~~

Census is therefore not a one-time import phase. Atlas builds itself by repeatedly improving census, recensusing donors and dependencies, absorbing selected mechanisms, and deleting donor source only after the extinction gate closes.

The normative recursive-generation law is `../contracts/RECURSIVE-SELF-CENSUS.md`. Every admitted generation becomes a stronger observation instrument; candidate self-analysis cannot be sole proof, and stronger perception MUST revisit affected prior closure/extinction conclusions.

## Mandatory generational execution

~~~text
G_n stable Atlas
  ↓ self/dependency/donor census + S_n
typed capability/extinction gaps
  ↓ bounded work order
C_n+1 candidate
  ↓ G_n-applicable gates + independent new-capability evidence
admission
  ↓
G_n+1
  ↓ mandatory stronger recensus
new obligations / revised donor dispositions / S_n+1
  ↓
repeat until multi-generation fixed point
~~~

R4 already participates: every material semantic-perception improvement triggers recensus. R5 reduces recomputation cost; R6 strengthens falsification; R7 automates bounded generation work; R8 strengthens source-independent rebuild/extinction proof.

A clean generation is not convergence. Unattended POLICY_AUTO requires at least two consecutive convergence-clean promoted generations under the recursive contract.

### What a stronger generation must revisit

A promoted generation MUST revisit any prior claim whose truth depended on a weaker observation horizon, including:

- negative evidence in the changed semantic dimension;
- dependency absence claims affected by new dynamic/build observation;
- Technology Genomes inferred from weaker donor census;
- ABSORBED and EXTINCTION_READY scopes;
- architecture falsification scenarios whose observability improved.

A new gap discovered after promotion is not a regression in the process. Hiding it would be the regression.

## Two independent axes

Do not confuse Atlas construction revisions with donor execution waves.

~~~text
R4 / R5 / R6 / R7 / R8
= maturity of Atlas's own native capabilities

W0 / W1 / W2 / ...
= donor-technology execution lanes

dependency closure
= breadth: what engineering world must be censused

R4 semantic dimensions
= depth: how deeply each admitted scope can currently be understood
~~~

A donor wave may span several R revisions. An R revision may use several donor waves. The numbering systems MUST remain distinct.

## Cross-cutting dependency gate — DC1

`DC1 — Dependency Census Runtime` is a cross-cutting implementation gate, not an R4 semantic dimension and not a donor W-wave.

DC1 is production-real only when Atlas can perform, for each admitted dependency-resolution context:

~~~text
root inventory
→ workspace/package/build declarations
→ deterministic direct dependency resolution
→ deterministic transitive dependency expansion
→ stable dependency identities
→ source-backed dependency admission
→ explicit non-source/toolchain/system/service terminals
→ dynamic/build-generated dependency obligations
→ repeat until dependency fixed point
→ expanded federated inventory
~~~

The normative semantics are in `../contracts/DEPENDENCY-CENSUS.md`.

A manifest or lockfile alone does not satisfy DC1.

A root repository boundary does not satisfy DC1.

DC1 is required before W0 may claim `COARSE_CENSUSED` for the full donor corpus.

## When real donor census starts

Atlas MUST NOT wait until R4, R5 or R6 are complete before censusing OSS.

The required sequencing is:

~~~text
R4.5 CALL semantics materialized
        ↓
R4.6 CONTROL_FLOW may proceed
        │
        └──────────────┐
                       ↓
                    DC1 real
                       ↓
══════════════════════════════════════════════
START W0 REAL DONOR CENSUS ACROSS DEPENDENCIES
══════════════════════════════════════════════
                       ↓
coarse census every admitted donor
+ every active direct/transitive dependency edge
+ every admitted source-backed dependency node
                       ↓
continue R4 semantic depth
                       ↓
recensus targeted donor/dependency scopes after each capability improvement
~~~

R4.x improves semantic depth. DC1 expands census breadth. They are complementary, not sequential substitutes.

## R4 — semantic census depth

R4 owns evidence-producing semantic extraction, typed preservation, deterministic normalization inputs, and the semantic depth required to understand donor implementations.

### R4.3.x — real Rust semantic bootstrap — materialized

Current materialized sequence:

- R4.3 — first real Rust semantic extractor;
- R4.3.1 — canonical production census wiring;
- R4.3.2 — lossless typed records through Census and normalization;
- R4.3.3 — raw observation identity, typed obligation lineage, typed closure accounting, and typed engineering-graph boundary;
- R4.3.4 — a `Parsed`, language-tagged artifact with no registered `SemanticExtractor` for its language now produces an explicit `UNSUPPORTED`-obligation `ExtractionBatch` (`runtime::census::extraction::unsupported_language_batch`, `DiagnosticCode::UnsupportedLanguageOrProfile`) instead of silently vanishing from `CensusExtractionAccounting` -- a real, live gap in this repository's own inventory (its own `toml`/`markdown`/`json` `Parsed` artifacts), not a hypothetical corpus;
- R4.3.5 — a pre-parse bracket-nesting-depth guard closed a real stack-overflow denial-of-service for explicit bracket nesting: `MAX_SEMANTIC_BYTES` bounded admitted-artifact byte size but never structural nesting depth, and a small pathologically-nested file reliably aborted the whole extraction process (`SIGABRT`, unrecoverable -- a Rust stack overflow cannot be caught by `catch_unwind`) rather than producing a diagnostic;
- R4.3.6 — R4.3.5's bracket-only guard was incomplete: continued adversarial testing found a bracket-free chained binary-operator expression (`1+1+1+...`) drives the identical crash with a bracket-nesting depth of just 1. `stacker`-based stack growth was tried and found ineffective (`syn`'s own recursion never calls back out to request more stack, so growing only around the outer call does nothing -- confirmed empirically). Replaced with `adapter::semantic::rust::max_structural_recursion_risk` (`MAX_STRUCTURAL_RECURSION_RISK = 64`), one combined metric covering both bracket nesting and operator-chain length. Empirically confirmed: bracket nesting crashed a reduced test-thread stack at 300 levels (100 safe); the operator chain crashed at 2,000 terms (1,000 safe); this repository's own real source never exceeds bracket depth 13 or a comparable chain length;
- R4.3.7 — a third vector, again found by continued adversarial testing: an `if ... else if ... else if ... else { .. }` chain drives the identical crash with neither elevated bracket depth (each arm's braces are siblings, not nested -- stays at 2) nor an operator-character run (`if`/`else` are keywords). 3,000 arms reliably overflowed the same stack. `max_structural_recursion_risk` now also tracks a run of `else` keyword occurrences (reset at `;`/`fn`, not at brace boundaries, since chain arms are siblings). Three real vectors found in direct succession is itself evidence this syntax-unaware text-scan approach cannot be exhaustive; this is recorded as an explicit, open, honestly-acknowledged residual risk (`.atlas/contracts/SEMANTIC-EXTRACTION.md#failure-semantics`) rather than a claim of complete closure;
- R4.3.8 — the predicted fourth vector materialized: continued adversarial testing (directly targeting the `max_structural_recursion_risk` doc comment's own "explicitly NOT claimed complete" warning) found a chained cast expression (`x as T as T as T ...`) drives the identical crash. `as` is a bare keyword with no punctuation signature at all -- unlike bracket/operator/else, it incremented neither counter, leaving it completely invisible to the guard. Confirmed empirically: 2,000 terms reliably overflowed the same reduced test-thread stack (1,000 safe). `max_structural_recursion_risk` now also tracks a word-boundary-checked run of `as` keyword occurrences (sharing `chain_run`, since a real cast chain has no intervening bracket to reset it, same as the operator-byte run) -- word-boundary checking is new to this scan specifically because `as`, at two bytes, is far more collision-prone against ordinary identifiers (`task`, `class`, `database`, ...) than `else`/`fn` ever were; a dedicated regression test (`identifiers_merely_containing_the_letters_as_do_not_trigger_the_guard`) proves this directly. Self-census against this repository's own real source (`cargo run -p atlas-cli -- systemize --root .`) confirms zero `RESOURCE_LIMIT` false positives after this change. Four real vectors found across three generations strengthens, not weakens, the prior generation's own honest assessment: this residual risk remains open and explicitly acknowledged, not closed.

Currently materialized Rust semantic dimensions are:

- SYMBOL (exhaustive over the declared declaration-level profile);
- TYPE (exhaustive over the declared declaration-level profile);
- FUNCTION_IDENTITY (exhaustive over the declared declaration-level profile);
- FUNCTION_SIGNATURE (exhaustive over the declared declaration-level profile);
- partial CALL;
- partial CONTROL_FLOW;
- partial DATA_FLOW;
- partial STATE;
- partial EFFECT;
- partial OWNERSHIP;
- partial CONCURRENCY;
- partial PERSISTENCE.

R4.5 observes real function-body call sites while leaving targets UNRESOLVED where source evidence is insufficient. R4.6 and R4.7 materialize control/data-flow structure. R4.8 produces useful STATE/EFFECT observations; R4.9/R4.10 produce useful OWNERSHIP/CONCURRENCY observations; R4.11 produces useful PERSISTENCE candidates. A full R4.4-R4.10 reconciliation audit confirmed that CALL/CONTROL_FLOW/DATA_FLOW/OWNERSHIP/CONCURRENCY share the same real, permanent gap already known for STATE/EFFECT (a call/access/site written only inside a macro invocation's arguments is structurally invisible without macro expansion, which this extractor never performs) -- all eight full-expression-tree dimensions (PERSISTENCE included, which additionally has no dedicated syntax at all and no resolved-API adapter) therefore deliberately keep their obligation UNKNOWN until their declared closure gaps are resolved, per `dimension_coverage` in `adapter::semantic::rust::mod`. Only the four declaration-level dimensions (SYMBOL/TYPE/FUNCTION_IDENTITY/FUNCTION_SIGNATURE) are currently exhaustive over their own explicitly-scoped profile.

These facts are increasingly useful for mechanism absorption, but partial evidence in any of the seven expression-tree dimensions must not be mistaken for full semantic closure.

### R4.4 — Function Identity Closure — materialized

R4.4 is materialized on canonical main.

The production identity model now distinguishes source-observable declaration context through typed declaration kind, owner/trait context and function generics while preserving repository/revision/scope identity and R4.3.3 raw-observation separation.

Materialized declaration kinds include:

- free function;
- inherent method;
- associated function;
- trait method declaration;
- trait default method;
- trait implementation method.

R4.4 intentionally does not claim compiler DefId-level equivalence, type-alias equivalence, macro-expanded declarations or monomorphized instance identity.

CALL remains the next semantic relation.

### R4.5 — Call Semantics — bootstrap materialized, closure remains open

R4.5's bootstrap is materialized on canonical main.

Production Rust semantic extraction now observes real function/method-body call sites and attributes them to the enclosing FunctionIdentity through the canonical SourceFrontend → SemanticExtractor → Census → Normalize → graph path. A full R4.4-R4.10 reconciliation audit confirmed a closure a bare "materialized" label previously implied but never proved: a call written only inside a macro invocation's arguments (`my_macro!(hidden_call())`) is structurally invisible to this extractor without macro expansion, which it never performs. This is a real, permanent gap shared by every full-expression-tree dimension (CALL/CONTROL_FLOW/DATA_FLOW/STATE/EFFECT/OWNERSHIP/CONCURRENCY alike), not unique to R4.5 -- see `dimension_coverage` in `adapter::semantic::rust::mod`, the one canonical place answering whether a dimension may treat zero observations as verified absence.

Current source-only resolution discipline is conservative:

- real call-site identity is OBSERVED;
- caller identity is preserved;
- dispatch remains UNRESOLVED when target resolution is not proven;
- unresolved callees are not fabricated;
- graph projection contains CallSite nodes and caller→MAKES_CALL edges without invented callee edges;
- CALL observations may be emitted while the dimension obligation remains UNKNOWN; zero observations are not verified absence.

Deeper target resolution may improve through later semantic/compiler evidence without changing the identity of the already-observed call site.

R4.12 closed CALL's own named argument-binding gap: `CallSiteIdentity.arguments: Vec<PlaceRef>` carries one entry per syntactic argument position, `PlaceRef::Resolved { dimension: DataFlow, record_id }` for a simple single-identifier argument, `PlaceRef::Unresolved` for anything requiring deeper analysis. The `record_id` is computed by reusing `ValueIdentity::identity_key()` itself (never a hand-duplicated formula), and `spelling::simple_path_ident` (the SAME recognizer DATA_FLOW's own `Expr::Path` arm now also calls, factored out to guarantee the two walkers cannot silently drift apart) -- proven, not merely designed, by `adapter::semantic::rust::tests::call_argument_place_ref_converges_on_the_data_flow_uses_own_record_id`, which asserts the CALL argument's `PlaceRef` names the EXACT SAME node DATA_FLOW's own pass produced. `PlaceRef` itself moved to `core::semantic::place` (out of `persistence.rs`) once it gained this second real consumer.

Result-binding is closed too: `CallSiteIdentity.result: PlaceRef` resolves to the DATA_FLOW `Definition`/`Store` record_id a call's return value directly becomes -- `let y = helper(x);` (Definition), `y = helper(x);` (Store) -- narrowly scoped to the case where the call expression IS the entire initializer/right-hand side (never a subexpression: `let y = helper(x) + 1;` stays `Unresolved`, since no single value the call's result "becomes" exists there), and only when the binding pattern reduces to one simple identifier (`spelling::simple_binding_ident`; a destructured `let (a, b) = two_args_tuple(x, x);` stays `Unresolved` even though DATA_FLOW itself binds every sub-identifier, since a call's single return value has no well-defined mapping onto more than one bound name). Proven by `adapter::semantic::rust::tests::call_result_place_ref_converges_on_the_data_flows_own_definition_record_id`/`..._store_record_id_for_assignment`. Self-census against this repository's own real source confirms both halves fire genuinely: 1638 real call results resolved, 3668 correctly `Unresolved`, zero new duplicates or conflicts at scale (26541 typed observations).

### R4.6 — Control Flow — bootstrap materialized, closure remains open

CONTROL_FLOW's bootstrap is materialized on canonical main with:

- deterministic block identity;
- entry/exit;
- branches;
- loops;
- return edges;
- panic-like-macro edges;
- explicit unresolved control constructs;
- function-to-CFG closure for the declared statement-level profile.

CFG identity MUST be stable for identical pinned input and MUST NOT depend on traversal/hash iteration order.

This wave's own scope was always statement-level only (a construct nested inside a larger expression, e.g. `let x = if c { a() } else { b() };`, is not given its own CFG blocks -- an honestly documented gap, not a silent one), and shares CALL's macro-invocation-opacity gap. A block whose only successor is a textual panic-like macro invocation now carries `EpistemicStatus::Inferred`, not `Observed`, on the whole block -- the same limitation R4.8 EFFECT already documents for the identical evidence, since the block's own successor set genuinely depends on whether the macro actually diverges (see `core::semantic::control_flow`). CONTROL_FLOW observations may be emitted while the dimension obligation remains UNKNOWN; zero observations are not verified absence.

A third gap, found by direct adversarial testing after the syntax-coverage sweep this session already ran against every other R4.5-R4.11 walker, was closed the same session it was found: a `let PAT = EXPR else { diverge }` statement is now lowered as a real decision point in `lower_stmts`, exactly like `if`/`match`/loop constructs -- the entry block gets a `Fallthrough` edge to the success-path join continuation (reusing the identical empty-remaining-statements handling `if`/`match`/loop already use, so no synthesized block appears when the let-else is the last statement) and a `Branch` edge to a new `ControlFlowBlockKind::LetElseDiverge` block whose own statements are recursively lowered like any other block. Verified against a plain diverge case, the no-remaining-statements case, and a `continue` inside the diverge block resolving correctly against the enclosing loop's `loop_stack` context (`adapter::semantic::rust::cfg`'s test suite: `let_else_diverge_block_is_a_real_cfg_branch_point`, `let_else_as_the_last_statement_does_not_synthesize_an_empty_continuation_block`, `let_else_diverge_continue_resolves_to_the_enclosing_loop`), with the full pre-existing CFG test suite unchanged and passing throughout.

A fourth gap was found and closed the same way, in the very next generation after a repository-reconciliation obligation was resolved (see "Canonical line of record" above): a bare `expr?;` statement (the `?`/try operator used as a whole statement, not nested inside a larger expression such as `let x = foo()?;`, which remains covered by the existing statement-level-only scope exclusion) had no arm in `lower_stmts` at all and fell through the wildcard `_ => continue` exactly like an ordinary side-effect-only call -- silently dropping the early-return possibility. Since `?` is fully syntax-determined (either its "continue" value is produced and execution falls through, or its "break" value returns from the enclosing function, never just an enclosing loop/block) and is one of the most common sources of conditional exit in idiomatic Rust, this was arguably a higher-impact instance of the identical defect class the let-else gap was: a function whose only conditional exit is a `?` was indistinguishable from one with no conditional exit at all. Closed by adding a `syn::Expr::Try` arm reusing the same join-continuation split (now factored into a shared `join_continuation` helper used by let-else, `if`/`match`/loop, and `?` alike, closing a pre-existing 2-way copy of that logic in the same change), with the `?`'s failure path a `Return` edge (the `ControlFlowEdgeKind::Return` doc comment now names this as its third cause, alongside an explicit `return` and a block's implicit top-level completion, the same "identical target and effect" principle `LoopRepeat` already applies to its own two causes). Verified against a plain two-`?`-statements case, the no-remaining-statements case (correctly collapsing both the success and failure edges to `Return` when nothing follows), and a `?` inside a loop body resolving `LoopRepeat` correctly on its success path (`try_operator_statement_is_a_real_cfg_branch_point`, `try_operator_as_the_last_statement_does_not_synthesize_an_empty_continuation_block`, `try_operator_inside_a_loop_body_preserves_loop_stack_for_the_continuation`), with the full pre-existing CFG test suite (490 tests workspace-wide) unchanged and passing throughout.

A fifth gap, the direct answer to the open question the `?` fix's own evidence record explicitly recorded for the next census ("does the same wildcard fallthrough hide any OTHER syntax-determined branch point?"), was found and closed immediately: `syn::Expr::Unsafe` (an `unsafe { .. }` block used as a statement) had no arm in `lower_stmts`/`lower_divergent` at all, unlike every other R4 walker (CALL/CONTROL_FLOW's own sibling dimensions DATA_FLOW/STATE/EFFECT/OWNERSHIP/CONCURRENCY/PERSISTENCE), which already recurse into it. Its entire contents -- an early `return`, a `?`, an `if`/`match`, a `panic!`, anything -- fell through the same wildcard and were completely invisible to CFG, a strictly worse instance of the same defect class (hiding an entire nested subtree, not just one branch). `unsafe` is a permission modifier, not a control-flow shape of its own, so it now shares `Expr::Block`'s existing handling and `ControlFlowBlockKind::NestedBlockExpr` rather than a dedicated kind. Verified against an `unsafe` block whose only statement is an `if` with an early `return`, proving the entire nested `if`/`return` is now real, connected CFG structure rather than swallowed evidence (`unsafe_block_statement_contents_are_not_invisible_to_cfg`), with the full pre-existing test suite (491 tests workspace-wide) unchanged and passing throughout. This generation also re-enumerated `syn` 3.0.6's full 39-variant `Expr` definition against `lower_stmts`'s arms directly from the vendored crate source (not from memory): every remaining un-arm'd variant is either genuinely non-branching (falls correctly to the wildcard), already covered by the documented Closure/Async/Const attribution-domain exclusion, or structurally inapplicable at statement level (`Expr::Let`, `Expr::Group`) -- except two unstable-Rust-feature variants (`Expr::TryBlock`, `Expr::Yield`) explicitly recorded as real but deliberately deferred given their negligible real-world prevalence, not silently missed (full detail: `.atlas/evidence/verification/cfg-unsafe-block-gap-closed.json`).

### R4.7 — Data Flow — bootstrap materialized, closure remains open

DATA_FLOW's bootstrap is materialized on canonical main with:

- values;
- definitions/uses;
- parameter flow;
- return flow;
- load/store relationships, including compound-assignment Use+Store;
- local propagation;
- typed unresolved/alias ambiguity where deeper analysis is unavailable.

Do not claim compiler-complete alias analysis unless actually evidenced.

R4.7 shares CALL's macro-invocation-opacity gap. R4.12 closed its own previously-documented gap:
tuple/tuple-struct/struct/slice destructuring, `&`/parenthesized wrapping and `ident @ sub_pattern`
bindings in a `let`/match-arm/`for`/parameter position now each emit a real Definition
(`adapter::semantic::rust::dataflow::DataFlowWalker::walk_binding_pat`), so their subsequent uses
resolve instead of staying explicitly UNRESOLVED. CALL↔DATA_FLOW binding (both argument- and
result-binding halves) is now closed too (see R4.5's section above). DATA_FLOW observations may be emitted while the dimension
obligation remains UNKNOWN; zero observations are not verified absence.

### R4.8 — State and Effect — bootstrap materialized, closure remains open

Canonical main now has a useful R4.8 bootstrap: single-level self.field READ/WRITE extraction, compound-assignment read-modify-write hardening, and conservative panic-like macro candidates. This is not R4.8 semantic closure.

Until deeper resolution exists:

- STATE/EFFECT observations may be emitted while the dimension obligation remains UNKNOWN;
- zero observations in a partially covered STATE/EFFECT dimension are not verified absence;
- a textual panic-like macro spelling is INFERRED unless macro/name resolution proves the actual panic effect;
- closures/async/const attribution and unmodeled state forms remain explicit gaps.

R4.8 closure still requires materializing/accounting:

- state identities;
- reads and writes;
- transitions/create/delete where applicable;
- externally observable effects;
- filesystem/network/process/FFI/build/runtime interactions where applicable;
- failure effects;
- explicit unknown/dynamic behavior;
- closure evidence sufficient to justify any verified negative fact.

This is the minimum point at which many donor mechanisms become semantically useful for absorption because Atlas can connect implementation behavior to state change and external effect, but the closure claim remains profile-scoped and evidence-gated.

### R4.9 — Ownership and Resource Semantics — bootstrap materialized, closure remains open

Canonical main now has a useful R4.9 bootstrap: `&`/`&mut` borrow-site detection (`OwnershipKind::BorrowShared`/`BorrowMut`, fully syntax-determined) and an honestly-ambiguous `MoveOrCopy` for a bare-identifier by-value use, since whether it moves or copies depends on `Copy`-ness this extractor cannot resolve. This is not R4.9 semantic closure.

R4.9 closure still requires materializing/accounting:

- allocation/free/resource acquisition/release;
- resource ownership transfer;
- lifetime/region facts only to the level supported by admitted evidence;
- explicit unresolved ownership where compiler-grade analysis is absent.

Do not claim rustc-equivalent borrow checking merely because ownership facts exist.

### R4.10 — Concurrency Semantics — bootstrap materialized, closure remains open

Canonical main now has a useful R4.10 bootstrap: `Await` for every `.await` (dedicated syntax, fully syntax-determined) and `Spawn` for a callee spelling ending in `spawn` (the same name-based risk class R4.8's panic-macro detection already accepts). This is not R4.10 semantic closure.

R4.10 closure still requires materializing/accounting:

- locks/unlocks;
- channels;
- atomics;
- synchronization/ordering relationships beyond `Await`/`Spawn`;
- concurrent state interaction;
- dynamic/unresolved concurrency obligations.

### R4.11 — Persistence and Recovery Semantics — bootstrap materialized, closure remains open

Canonical main now has a useful R4.11 bootstrap: `Commit`/`Flush`/`Sync`/`Checkpoint`/`Snapshot`
textual callee-spelling candidates, always `EpistemicStatus::Inferred` (never `Observed` --
`game.commit()` and `wal.commit()` are equally uncertain to this extractor). This is not R4.11
semantic closure.

**Design guard enforced before implementation**: R4.11 does NOT introduce a fourth/fifth
independent `name: String`-keyed target identity alongside DATA_FLOW's `ValueIdentity`, STATE's
`StateAccessIdentity` and OWNERSHIP's `OwnershipIdentity`. Instead, `PersistenceIdentity.place:
PlaceRef` (`core::semantic::persistence`) is a small bridge: `PlaceRef::Resolved { dimension,
record_id }` lets a persistence operation point at an EXISTING dimension's own already-canonical
record when evidence allows (proven by a dedicated graph test converging a PERSISTENCE operation
onto the SAME `StateAccess` node a STATE observation produced), and `PlaceRef::Unresolved` names
"no canonical place" explicitly rather than fabricating one from spelling -- this extractor's only
mode this wave, since it has no resolved-API adapter yet. A full first-class `PlaceIdentity` shared
natively by all four dimensions remains TARGET work (see
`.atlas/evidence/verification/r4.4-r4.10-second-hardening-pass-correction.json`); `PlaceRef` is
explicitly the bridge, not the destination.

Until deeper resolution exists:

- PERSISTENCE observations may be emitted while the dimension obligation remains UNKNOWN;
- zero observations in the partially covered PERSISTENCE dimension are not verified absence;
- a textual `commit`/`flush`/`sync`/`checkpoint`/`snapshot` spelling is `Inferred` unless a
  resolved/admitted API adapter proves the actual durable operation;
- a bare STATE mutation (`self.counter += 1`) never fabricates a PERSISTENCE fact, and a durability
  boundary (a resolved commit) does not retroactively claim every prior STATE write as durable --
  the two dimensions stay typed and separate, linkable only through `PlaceRef` where evidence
  proves the same location;
- closures/async blocks get no PERSISTENCE attribution of their own, consistent with every other
  dimension's deferred-region exclusion (a `persist().await` inside `async { .. }` belongs to that
  region, never to the function that merely constructs it).

R4.11 closure still requires materializing/accounting:

- `DurableRead`/`DurableWrite`/`JournalAppend`/`TransactionBegin`/`TransactionAbort`/`Recover`/
  `Restore` (declared in `PersistenceKind`, never emitted -- would need resolved-API evidence this
  extractor does not have);
- durability ordering relationships (write → flush → commit; concurrent writers vs. a checkpoint);
- recovery/failure-path relationships (a restore that recovers-from a checkpoint/journal);
- external persistence boundaries (files/objects/blobs, database tables/keys) as resolvable
  `PlaceRef` targets, not just STATE-shared ones;
- connection to `FailureScenario`/`VerificationWorld`/obligation/evidence
  (`VERIFICATION-METRICS-PERFORMANCE.md`) -- e.g. an obligation "committed writes survive process
  restart" backed by a crash-then-recover `FailureScenario`'s evidence, not by one passing test
  treated as universal truth;
- closure evidence sufficient to justify any verified negative fact.

This is the minimum point at which Atlas can connect implementation behavior to durable-state
claims for mechanism absorption, but the closure claim remains profile-scoped and evidence-gated,
exactly like R4.8/R4.9/R4.10 before it.

### R4.12 — R4 Semantic Closure — infrastructure gate materialized, one item remains a named bridge

R4 closes only after the declared Rust reference profile satisfies the canonical R4 acceptance
contract (`.atlas/contracts/SEMANTIC-EXTRACTION.md#r4-definition-of-done`). R4.12 operates one
level above any single dimension's own remaining closure status (R4.5-R4.11 each keep their own
"closure remains open" state, unaffected by this section): it is the pipeline-level gate --
determinism, non-omission, dedup accounting, conflict preservation, graph-path purity, and a named
reference corpus -- not a claim that every dimension has exhausted Rust's construct space.

Named Rust reference profile: `R4_REFERENCE_PROFILE_CORPUS`
(`adapter::semantic::rust::tests`, mirrored in `runtime::census::tests` for the full
Census→Normalize path), the first single corpus proven evidence-producing -- not merely accounted
-- for all twelve `SemanticDimension` variants together. Every project-wide R4-complete claim
names this corpus, per the contract's own closing requirement.

Status against the 9-item Definition of Done, this wave:

- all mandatory R4 dimensions evidence-producing or explicitly accounted: **done**, and now proven
  in the strongest form via the named reference profile;
- every discovered function with stable identity/signature or explicit unresolved state: **done**
  (R4.4, unchanged);
- deterministic normalization: **done** (unchanged);
- exact semantic duplicate policy: **partial** -- the narrowest safe case (byte-identical raw
  observations) is now real, counted (`NormalizationReport.exact_duplicates_merged`), and the
  closure invariant (`typed_semantics_closed()`) correctly accounts for the collapse
  (`input == normalized + exact_duplicates_merged`, matching
  `.atlas/contracts/NORMALIZATION.md#closure-invariants` verbatim). `NORMALIZATION.md`'s broader
  rule -- merging same-extractor observations that agree on identity/payload/status but differ
  only in evidence, unioning that evidence -- remains explicit **bridge/target**: no current Rust
  walker's output shape ever triggers it (each emits exactly one evidence ref per observation), and
  self-census against this repository's own ~25k observations confirms zero real occurrence, so it
  is named rather than guessed at;
- multi-extractor observations preserved: **done** (R4.3.3 onward, unchanged);
- conflict candidates preserved for reconciliation: **done**, newly this wave --
  `NormalizationReport.conflict_candidates` groups typed records by `record_id` and flags any group
  disagreeing on typed payload (`SemanticObservation::subject_repr()`), proven against the exact
  "extractor call-target sets" disagreement example `NORMALIZATION.md` itself names
  (`CallSiteIdentity.dispatch`/`callees` differing under one call-site identity), and against real
  corroboration (identical payload, different extractor) correctly NOT flagging. Deliberately
  over-inclusive rather than per-dimension-nuanced: reconciliation (R6) decides, this wave only
  detects and preserves;
- dynamic/unresolved facts explicit: **done** (unchanged, e.g. `CallDispatchKind`'s four states);
- reference corpus: **done**, newly this wave -- see `R4_REFERENCE_PROFILE_CORPUS` above;
- deterministic accounting/closure tests: **done**, newly this wave -- the named corpus closes this
  gap directly (`reference_profile_produces_real_evidence_for_every_mandatory_dimension`,
  `reference_profile_survives_census_and_normalization_with_no_duplicates_or_conflicts`);
- graph construction only from normalized typed truth: **done** (R4.3.3, unchanged);
- no compatibility `SemanticFact` authority over typed semantics: **done** (R4.3.3, unchanged).

R4.12 is **materialized, not fully closed**: 8 of 9 items are genuinely done; the exact-duplicate
equivalence-class rule remains a named bridge, not silently omitted or overclaimed.
`.atlas/evidence/verification/r4.12-verification-record.json` records the full gap audit, the
self-census cross-check at real scale, and the bug found and fixed this pass (the closure
invariant did not yet account for the exact-duplicate delta the contract's own formula requires).

R4 closure is profile-scoped. Do not claim universal language/compiler completeness.

## R5 — incremental query and fixed-point closure

R5 adds the ability to reason and recensus incrementally over the already-growing donor/dependency corpus.

Required capabilities:

- dependency-aware query invalidation;
- revision-scoped cached derivation;
- recursive/fixed-point semantic derivation;
- incremental recensus of changed source and affected dependents;
- deterministic propagation;
- evidence/provenance lineage through derived facts.

Primary donor lane: W3 — salsa, datafrog, differential-dataflow, souffle, buck2.

Every W3 donor is still censused through its admitted transitive dependency closure. If a dependency actually provides a mechanism of interest, attribute the mechanism to that dependency rather than the top-level donor.

After every material R5 capability:

~~~text
implement
→ verify
→ recensus Atlas
→ recensus affected donors/dependencies
→ update Technology Genomes/decisions
~~~

## R6 — reconciliation, adversarial closure and CensusCertificate

R6 turns accounted observations into a closure claim that can be independently checked.

Required capabilities:

- preserve independent extractor identities;
- explicit CONFLICT;
- cross-scope reconciliation;
- top-down claim decomposition;
- bottom-up aggregation;
- adversarial gap queries;
- fixed-point closure;
- dependency closure included in closure proof;
- CensusCertificate issuance;
- Genome policy enforcement for UNKNOWN/UNSUPPORTED.

Primary support donor lane: W4 — kani, miri, verus.

R6 does not mean "choose a winner whenever extractors disagree." Conflict remains conflict until evidence supports reconciliation.

R6 is the first point at which Atlas can make strong proof-producing statements that a declared census scope is closed for a declared policy/profile.

## VP1 — verification, metrics and performance semantics (cross-cutting R6→R8)

Before the mature autonomous creation loop may be claimed, Atlas must make verification/measurement first-class under ../contracts/VERIFICATION-METRICS-PERFORMANCE.md.

VP1 is part of `*.atlas` construction. It is not a promise to create a final artifact first and test it afterward.

VP1 includes, at the contract/runtime maturity appropriate to the active wave:

- Obligation distinct from Test/Evidence;
- MetricContract distinct from Observation and Objective;
- Workload and VerificationWorld/EnvironmentGraph;
- failure scenarios;
- semantic verification plans capable of CI/compose-grade system tests without making a vendor/config format canonical;
- TheoreticalCost / PredictedPerformance / EmpiricalObservation separation;
- CostModel, uncertainty and calibration;
- multi-objective optimization/search as replaceable solver backends;
- seal/admission policies that bind claims to exact workload/environment/evidence scope;
- CandidateAtlas → SealEligibleAtlas → SEALED logical Atlas state separation;
- VERIFY / BENCH / PROVE logical engines reusable both inside construction and through future CLI/API surfaces;
- evidence/attestation roots bound by the seal without turning the immutable artifact into a mutable telemetry database.

Analytic models may prune candidate space. Final performance claims remain evidence-scoped. A failed required metric/obligation returns the candidate to the repair loop; it does not create a final `*.atlas`.
## AH1 — embedded agent-host execution fabric (cross-cutting R4→R8)

Atlas MUST be runnable as an embedded subsystem inside the coding environment that is already performing the work.

Normative host/runtime semantics are defined by `../contracts/AGENT-HOST-EMBEDDED-RUNTIME.md`.

The primary early deployment target is:

~~~text
coding agent in local/cloud host sandbox
→ local MCP/API adapter
→ AtlasCore
→ Atlas CandidateWorkspace / SandboxBackend
→ Census / verify / admission / seal
~~~

The host may supply compute, checkout and an outer security sandbox. Atlas supplies the durable semantic world, uncertainty/closure, candidate isolation semantics, evidence binding, admission and artifact lifecycle.

AH1 MUST preserve:

- agent host is infrastructure, not canonical authority;
- coding provider is replaceable temporary intelligence;
- MCP/API/CLI/Studio are adapters over one AtlasCore;
- mutable worktree is candidate state until admitted;
- provider-created commit is not AdmissionTransaction;
- outer host sandbox is not the same thing as Atlas job isolation or VerificationWorld;
- nested Docker/KVM/privileged sandbox capability is never assumed;
- verification binds an exact frozen candidate;
- sealed Atlas/evidence state outlives the originating agent session;
- heavy jobs may scale out without changing semantic identity.

AH1 allows Atlas V1 to borrow cloud-agent compute rather than requiring an Atlas cloud platform on day one. Scale-out workers become an implementation option when workloads exceed the embedded host.

Atlas Studio is a later native frontend over the same core, not a prerequisite for Atlas and not a separate truth system.

## AIF1 — multi-AI construction fabric (cross-cutting R6→R8, load-bearing in R7)

Normative orchestration semantics are defined by `../contracts/MULTI-AI-CONSTRUCTION-FABRIC.md`.

Atlas MUST be able to decompose one construction objective into a typed task graph and route independent tasks to heterogeneous workers without granting any worker canonical authority.

Target execution classes include:

~~~text
HOST_NATIVE_AI_WORKER
REMOTE_AI_PROVIDER_WORKER
REMOTE_ATLAS_EXECUTION_WORKER
DETERMINISTIC_EXECUTION_WORKER
~~~

AIF1 MUST preserve:

- provider role is distinct from vendor/model identity;
- every material worker invocation has attributable task/provider lineage;
- tool/filesystem/network-capable subagents run under bounded leases/capability envelopes;
- child agents cannot widen parent authority or budget;
- agent-to-agent durable coordination uses typed Atlas records, not a shared chat transcript;
- context windows receive compiled semantic slices, while Atlas remains durable memory;
- Jev-class decisions emit `DecisionProposal`, never direct SelectedDesign/seal authority;
- candidate branches remain identity/evidence separated;
- raw provider credentials are brokered/scoped rather than sprayed into coding sandboxes where possible;
- budget exhaustion is explicit and cannot weaken verification/closure policy;
- remote provider or worker fallback does not change task semantic identity;
- provider consensus is evidence, not seal authority.

The intended early embedded shape can therefore be:

~~~text
Claude-oriented cloud AgentHost
├─ host-native Claude/coding subagents
├─ AtlasCore + local MCP
├─ ProviderRouter
│  ├─ remote GPT-class provider
│  ├─ remote Gemini-class provider
│  ├─ Jev-class provider
│  └─ remote Atlas workers
└─ Atlas CandidateWorkspaces / verification backends
~~~

Names above are deployment examples, never required dependencies.


## AI1 — Architectural Integrity / Collapse Prevention (cross-cutting R4→R8)

AI1 is governed by ../contracts/ARCHITECTURAL-INTEGRITY.md. It is distinct from VP1: VP1 asks whether obligations/metrics are verified; AI1 asks whether the exact candidate still obeys the selected load-bearing system architecture.

The implementation sequence is intentionally distributed across the existing R-waves rather than inventing a new maturity number:

- **R4 observation substrate** — typed call/control/data/state/effect/ownership/concurrency/persistence semantics must be rich enough to observe architecture-relevant topology instead of guessing from folders or prose.
- **R5 impact closure** — dependency-aware incremental invalidation must compute which architecture invariants can be affected by a semantic delta and prove when cached unaffected results are reusable.
- **R6 reconciliation** — multi-observer evidence must reconcile architecture-relevant facts, preserve CONFLICT/UNKNOWN, and produce the observed-architecture root needed by an ArchitecturalIntegrityReport.
- **R7 admission** — CandidateChangeSet/ACP transactions must be checked on isolated post-change candidate state against a pinned ArchitecturalIntegrityEnvelope. HARD violation rejects admission; intentional architecture change requires a SELECTED BlueprintRevisionDecision.
- **R8 durable seal/materialization** — logical Atlas binds the exact envelope/report/equivalence evidence roots; AtlasX materialization revalidates invariants affected by selected bindings, profiles, partial closure or deterministic expansion.

AI1 starts as CONTRACT now. No wave may claim the production capability merely because these docs/schemas exist.

## R7 — research, typed decision, synthesis, self-build control and admission

R7 connects observed donor reality to research and Human+AI design decisions without allowing research/model claims to impersonate observation. R7 is also the first maturity level at which Atlas may run a complete bounded self-coding loop.

Required capabilities:

- ResearchClaim distinct from ObservedEvidence;
- Technology Genome comparison;
- Atlas capability-gap graph;
- SelfBuildController governed by ../contracts/SELF-BUILD-CONTROLLER.md;
- typed SelfBuildWorkOrder records that pin parent revision, scope, authority and gates;
- Human/AI typed intent and constraint envelopes;
- research-provider integration over Atlas knowledge + OSS + external references;
- candidate mechanism/design records;
- typed DecisionProposal records for Jev-class rank/score/route decisions;
- ProviderReceipt lineage;
- CandidateChangeSet records;
- external synthesis/code providers producing real candidate implementation;
- generated implementation ingested and censused as untrusted source;
- declared generated intent compared with observed generated semantics;
- CandidateAtlas state after generated-code census;
- analytic CostModel/constraint pruning before expensive materialization where useful;
- construction-time VERIFY / BENCH / PROVE and VP1 gates;
- security/dependency/license/test/benchmark/proof evidence bound to named obligations;
- explicit Human/Policy/Hybrid selection authority;
- AdmissionTransaction governed by ../contracts/ADMISSION-TRANSACTION.md for canonical Atlas source mutation;
- post-apply recensus and expected-vs-observed semantic-delta verification;
- evidence-linked absorption and blueprint-revision decisions.

Normative contracts:

- ../contracts/HUMAN-AI-ADL-AUTHORING.md;
- ../contracts/ATLAS-CREATION-PIPELINE.md;
- ../contracts/EXTERNAL-PROVIDER-TRUST.md;
- ../contracts/AGENT-HOST-EMBEDDED-RUNTIME.md;
- ../contracts/MULTI-AI-CONSTRUCTION-FABRIC.md;
- ../contracts/SELECTED-DESIGN.md;
- ../contracts/SELF-BUILD-CONTROLLER.md;
- ../contracts/ADMISSION-TRANSACTION.md;
- ../contracts/VERIFICATION-METRICS-PERFORMANCE.md;
- ../contracts/ARCHITECTURAL-INTEGRITY.md.

Primary donor lane: W5 — openrewrite, c2rust, crubit, py2many — plus explicitly admitted research/decision/synthesis/verification provider adapters.

R7 MUST preserve:

~~~text
Observed implementation
≠ ResearchClaim
≠ DecisionProposal
≠ CandidateDesign
≠ CandidateChangeSet
≠ SelectedDesign
≠ AdmissionTransaction
~~~

A fast decision provider may reduce search cost. A synthesis provider may write real candidate code. Neither is canonical truth. A SelectedDesign is still not a canonical repository mutation until its AdmissionTransaction commits and the exact resulting tree passes recensus/verification.

No research page, paper, search result, model answer, provider score or README directly upgrades a donor/generated implementation claim to OBSERVED.
## R8 — durable ATLAS / AtlasX substrate

R8 implements the durable semantic/evidence carrier required for Atlas knowledge to outlive donor checkout deletion at scale and makes the Atlas→AtlasX handoff implementable without hidden design invention.

Required capabilities:

- typed binary `*.atlas`;
- lossless semantic compaction under `../contracts/ATLAS-SEMANTIC-COMPACTION.md`;
- content-addressed records/blocks/shards;
- integrity hashes;
- transactional publication;
- logical root manifests;
- stable cross-shard identity/bindings;
- explicit SelectedDesign identity under `../contracts/SELECTED-DESIGN.md`;
- selected implementation semantics and provider/candidate lineage complete before seal;
- final exact-candidate seal-eligibility gate after candidate implementation census/verification;
- logical Atlas seal that binds SelectedDesign + required obligation/evidence commitments;
- logical Atlas seal binds the active ArchitecturalIntegrityEnvelope and exact eligible integrity/equivalence evidence roots;
- canonical `*.atlas` publication only after seal;
- deterministic provider-independent mechanical compaction after seal;
- deterministic Atlas→AtlasX materialization under `../contracts/ATLAS-TO-ATLASX.md`, including materialization-critical architectural revalidation;
- canonical AtlasX object/manifest validation under `../contracts/ATLASX-FORMAT.md`;
- canonical AtlasX v1 bytes/root hashing under `../contracts/ATLASX-BINARY-WIRE-FORMAT.md`;
- parent/lineage retention;
- partial materialization without competing truth;
- compiler handoff governed by `../contracts/COMPILER-IR-PIPELINE.md` and v1 IR records/opcodes governed by `../contracts/COMPILER-IR-SCHEMAS.md`.

Primary donor lane: W6 — flatbuffers, arrow, zstd, blake3, object, regalloc2, mold.

R8 does not authorize mechanical donor translation. It provides the durable Atlas-native carrier and deterministic executable projection needed for source-independent continuation.

The R8 storage/materialization blueprint is explicitly revisable if census demonstrates a better mechanism, but revision must follow `../contracts/BLUEPRINT-EVOLUTION.md`.

## Mature self-building acceptance loop

Atlas may claim a mature bounded self-building loop only when the following exact cycle is executable without hidden manual semantic invention:

~~~text
self-census / closure / metrics
→ capability gap
→ SelfBuildWorkOrder
→ research / donor census
→ candidate set / typed decision
→ Atlas-managed candidate workspace / SandboxBackend
→ synthesis CandidateChangeSet
→ untrusted census
→ CandidateAtlas
→ CostModel / constraint pruning
→ VERIFY / BENCH / PROVE required obligations
→ repair loop until admissible
→ authorized SelectedDesign
→ AdmissionTransaction when self-source changes
→ exact applied-tree recensus / reverify
→ SealEligibleAtlas
→ SEALED logical Atlas
→ deterministic compacted *.atlas when publication is required
→ admitted Atlas revision / durable artifact
→ recompute gaps
↺
~~~

R8 then makes the selected semantics/evidence durable as Atlas/AtlasX; it does not replace the admission boundary.

## Human-AI creation maturity rule

Provider integration MUST follow the maturity of Atlas's semantic/verification substrate.

Atlas may experiment with research/decision/synthesis providers earlier, but MUST NOT claim the mature autonomous creation loop until it can:

- type the constraint envelope;
- preserve provider receipts;
- produce typed candidate/decision/change-set records;
- census generated implementation deeply enough for the active profile;
- enforce security/dependency/license gates;
- compare declared intent against observed implementation;
- validate candidate semantics;
- select under explicit authority;
- seal without requiring future provider availability.

The provider layer accelerates engineering; it does not weaken census requirements.

## Evidence-driven blueprint evolution

Canonical blueprints are authoritative for the current evidence state, but they are not immutable.

During any R4→R8 census/recensus, Atlas may discover a mechanism, representation, compiler strategy, storage layout, verification method or dependency architecture that is materially better than the current blueprint.

When that happens Atlas MUST NOT ignore the evidence merely to preserve an older plan, and MUST NOT silently redesign in implementation code.

The required path is governed by `../contracts/BLUEPRINT-EVOLUTION.md`:

~~~text
current canonical blueprint
→ donor/dependency census discovers better mechanism
→ attribute actual provider
→ deep census
→ compare current vs candidate vs alternatives
→ evidence / benchmark / proof
→ BlueprintRevisionDecision
→ SELECTED
→ update canonical blueprint/roadmap/contracts if required
→ implement
→ recensus / verify
~~~

A blueprint revision MAY alter future sequencing, insert prerequisites, split/merge waves or change implementation strategy when evidence justifies it.

A blueprint revision MUST NOT silently violate higher-level contracts/Genome invariants. If the better design requires a contract change, that contract change is explicit and compatibility/migration analysis is mandatory.

"Canonical" therefore means "currently selected authoritative design", not "frozen forever".

## Continuous census and recensus rule

Every material Atlas capability improvement from R4 through R8 MUST consider recensus impact.

The default loop is:

~~~text
stronger Atlas capability
→ recensus Atlas itself
→ recensus affected donor scopes
→ recensus affected source-backed dependency scopes
→ compare new evidence with prior observations
→ deepen Technology Genome where justified
→ reconsider pending absorption decisions
~~~

A capability wave that changes what Atlas can observe but never recensuses relevant donors is incomplete as a self-building wave.

## Discovery is allowed; uncontrolled drift is not

Census may reveal technology not listed in the authored donor plan.

Examples include:

- an unexpected algorithm;
- a better incremental strategy;
- an important data structure;
- a compiler or query technique;
- a verification method;
- a storage/layout mechanism;
- a hidden provider dependency;
- an Atlas capability gap not previously recognized.

Discovery does not automatically change Atlas architecture and does not automatically create a donor.

Every material discovery receives an explicit disposition.

## Discovery dispositions

The canonical planning dispositions are:

~~~text
ABSORB_NOW
ABSORB_LATER
REFERENCE_ONLY
EXTERNAL_BOUNDARY
REJECT
~~~

### ABSORB_NOW

Use only when the discovered technology:

- removes or materially reduces a current R4→R8 blocker or capability gap;
- is a prerequisite for the active self-building path, or is clearly high-leverage and bounded;
- has an identifiable actual provider scope;
- has a plausible Atlas-native owner;
- can be deep-censused to the semantics required for safe absorption;
- has a credible verification path;
- has a credible dependency-removal/extinction path.

### ABSORB_LATER

Use when the mechanism is valuable to Atlas but:

- depends on capabilities not yet mature;
- would violate current sequencing;
- has too large an unresolved scope;
- is not required for the active construction path.

The discovery remains durable and queued; it is not silently forgotten.

### REFERENCE_ONLY

Use when the donor/dependency is useful as:

- oracle;
- comparison;
- test reference;
- explanatory implementation evidence;

but Atlas has not selected its technology for native ownership.

Local source retained for reference means the scope is not EXTINCT.

### EXTERNAL_BOUNDARY

Use when Atlas intentionally keeps a technology external through an explicit adapter/capability boundary.

External technology MUST remain explicitly identified, versioned and censused according to policy. It MUST NOT be renamed Atlas-native.

### REJECT

Use when the mechanism is:

- irrelevant;
- redundant;
- inferior for the active constraints;
- incompatible with Atlas invariants;
- unjustified by evidence;
- legally/provenance constrained in a way that prevents the intended absorption;
- outside the current Atlas self-building objective.

Rejection retains the evidence and rationale needed to avoid rediscovering and re-evaluating the same dead end without cause.

## Required discovery decision record

Every material discovery selected for planning MUST retain enough information to answer:

- what was discovered;
- exact provider identity/revision/version;
- provider scope;
- how it was discovered;
- evidence references;
- whether the provider is the root donor or a dependency;
- proposed Atlas capability;
- proposed native owner;
- current disposition;
- decision rationale;
- required semantic depth;
- prerequisites/blockers;
- verification plan;
- dependency-removal plan;
- extinction implications.

The physical data type may be added in a later implementation wave. The semantic obligations are fixed here.

## Discovered dependency is not automatically an absorption donor

All active dependencies are census subjects.

They are not automatically donor-admission subjects.

Example:

~~~text
Donor A
→ dependency B
→ dependency C
~~~

A, B and C are all dependency-census nodes when active in an admitted context.

If census discovers that C owns a mechanism Atlas wants to absorb, C MUST be explicitly promoted through donor admission before absorption work begins.

Promotion requires:

- exact provider identity;
- exact version/revision;
- license;
- provenance;
- selected mechanism/scope;
- Atlas capability target;
- native owner;
- required semantic depth;
- verification plan;
- extinction/dependency plan.

No implementation agent may silently promote a dependency to donor status.

## Idea-driven pull-forward rule

W-wave order is the default donor execution order, not an excuse to ignore a prerequisite.

A mechanism from a later donor lane may be pulled forward only when all of the following are true:

- it is a concrete prerequisite/blocker for the active R-wave, or its leverage on the active Atlas capability is clear;
- the scope is bounded;
- the actual provider is identified;
- evidence is available;
- the Atlas-native owner is known;
- required semantic depth can be achieved;
- verification is definable;
- the decision is recorded as ABSORB_NOW.

Otherwise use ABSORB_LATER.

"Interesting" alone is never sufficient reason to pull a donor/mechanism forward.

## Deep-census gate before absorption

Atlas MUST NOT select a mechanism for native implementation from README/API shape/function names/model description alone.

Before absorption, the provider scope must be censused to the depth required to identify the mechanism's essential semantics.

Depending on the mechanism this may require:

- participating types/functions;
- call relationships;
- control flow;
- data flow;
- state reads/writes;
- effects;
- ownership/resource assumptions;
- concurrency;
- persistence/recovery;
- failure paths;
- tests/runtime/binary evidence;
- dependency-provided behavior;
- constraints/invariants;
- trade-offs;
- unresolved facts.

The required depth is mechanism-specific. Not every mechanism requires every S10 atom. Essential dependencies and unknowns may not be silently omitted.

## Absorption state machine

Canonical scope-level progression:

~~~text
STAGED
  ↓
COARSE_CENSUSED
  ↓
DEEP_CENSUS_ACTIVE
  ↓
TECHNOLOGY_GENOME_CAPTURED
  ↓
NATIVE_IMPLEMENTED
  ↓
ABSORBED
  ↓
EXTINCTION_READY
  ↓
SOURCE_DELETED
  ↓
EXTINCT
~~~

`EXTINCTION_READY` is a non-terminal state. It exists so Atlas can truthfully distinguish "native replacement and dependency-removal proof are complete" from "the donor source has actually been deleted and post-delete verification succeeded."

A scope may skip a long residence in EXTINCTION_READY when all deletion prerequisites are already satisfied, but it may not skip SOURCE_DELETED.

## ABSORBED definition

A scope is ABSORBED only when:

- required mechanism/invariants are durably captured;
- Atlas-native implementation exists in the intended owner;
- required tests/proofs/benchmarks pass;
- Atlas recensus agrees with selected design within declared policy;
- donor runtime/build/test source dependency is zero for the absorbed scope;
- remaining donor-only knowledge for the scope is captured, explicitly deferred, or explicitly rejected.

ABSORBED is not EXTINCT.

## EXTINCTION_READY definition

A scope is EXTINCTION_READY when the ABSORBED conditions hold and deletion can proceed once the durable-knowledge and physical-deletion gates are satisfied.

Before R8, a scope may remain EXTINCTION_READY when the canonical durable Atlas carrier required to survive source deletion is not yet mature for that scope.

Do not weaken the durable-knowledge requirement merely to report faster extinction.

## EXTINCT definition

EXTINCT is physical.

An extinct scope MUST satisfy all of:

- exact extinct donor/revision/scope recorded;
- relevant dependency closure accounted;
- durable Technology Genome/evidence/native replacement retained;
- zero runtime/build/test dependency on the donor source for the extinct scope;
- donor OSS source files for the extinct scope physically removed from Atlas-controlled active storage;
- no substitute Atlas-controlled source archive/cache/snapshot/vendor copy retained as a hidden equivalent;
- source-path absence explicitly checked;
- post-delete Atlas recensus passes;
- required tests/proofs/benchmarks still pass.

Renaming, ignoring, unreferencing, vendoring elsewhere or archiving the same source does not satisfy extinction.

Historical Git objects are governed separately by repository-history policy and do not make active-tree deletion false.

## Scope-level versus repository-level extinction

Absorption/extinction is fundamentally scope-level.

A repository may contain:

~~~text
scope A  EXTINCT
scope B  ABSORBED but source retained for another unresolved scope
scope C  REFERENCE_ONLY
scope D  not yet deep-censused
~~~

Do not label the whole donor repository EXTINCT while required donor source remains under Atlas control.

Repository-level EXTINCT is valid only when all retained scopes satisfy the repository-level deletion rule.

## Dependency-aware extinction

Donor A does not own mechanisms merely because A depends on them.

If:

~~~text
A
→ B
→ C
~~~

and B provides the selected mechanism, absorption/extinction attribution belongs to B's provider scope.

If Atlas intentionally retains B as an external dependency, B remains an explicit dependency.

If Atlas selects B for native absorption, B receives its own donor admission, Technology Genome, native implementation, verification, recensus and extinction lifecycle.

Third-party dependency code MUST NOT be silently relabeled Atlas-native.

## R4→R8 donor-lane map

The default mapping is:

| Atlas maturity | Primary donor lane | Purpose |
| --- | --- | --- |
| R4 + DC1 | W0, W1, W2 | corpus breadth, source intelligence, semantic/compiler understanding |
| R5 | W3 | incremental query, fixed point, build/dependency reasoning |
| R6 | W4 where relevant | verification/safety evidence supporting reconciliation and closure |
| R7 | W5 | transformation/migration mechanisms and evidence-linked selection |
| R8 | W6 | durable binary/storage/data-layout/backend mechanisms |
| later/default | W7 | security/trust breadth unless a bounded prerequisite is pulled forward |
| later/default | W8 | Studio/editor projection unless a bounded prerequisite is pulled forward |

This table is a default execution map, not permission to skip dependency closure and not permission to pull work forward without the recorded ABSORB_NOW criteria.

## Required self-building loop at every implementation wave

Before coding, the agent MUST identify:

- exact canonical main SHA;
- active R-wave or cross-cutting gate;
- Atlas-native capability being made real;
- donor/dependency evidence that motivates it, if applicable;
- native owner: core/runtime/adapter/apps;
- semantic truth that becomes more complete;
- remaining UNKNOWN/UNSUPPORTED states;
- donor scopes to recensus afterward;
- whether any discovery decision changes;
- whether any absorbed scope becomes EXTINCTION_READY;
- whether any EXTINCTION_READY scope may safely progress through SOURCE_DELETED to EXTINCT.

After coding, the agent MUST report the same items with evidence.

An agent MUST NOT invent a new architectural owner, new semantic path, new donor status, or extinction claim to make a wave appear complete.

## Implementation truth vocabulary

Use these maturity words precisely:

- DOCUMENTED — contract/blueprint exists;
- TYPED — type/API exists;
- IMPLEMENTED — working logic exists;
- CALLED_IN_PRODUCTION — canonical runtime path invokes it;
- TESTED — behavior is covered by verification;
- VERIFIED — exact candidate/revision has durable verification evidence.

Do not use "done" to collapse these distinctions.

## Prohibited shortcuts

The following are forbidden:

- waiting until R4/R5/R6 finish before starting donor census;
- stopping census at a donor repository boundary;
- treating a lockfile as dependency closure;
- silently omitting build/proc-macro/native/dynamic dependencies;
- automatically admitting every transitive dependency as an absorption donor;
- absorbing a mechanism from README/API/prose alone;
- copying donor topology into Atlas architecture;
- wrapping a donor library permanently and calling it Atlas-native;
- allowing donor-named production ownership outside provenance/reference roles;
- using model output as OBSERVED donor implementation truth;
- implementing a native replacement without recensus;
- claiming ABSORBED when donor runtime/source dependence remains;
- claiming EXTINCT while donor source remains under Atlas control;
- moving donor source into another cache/vendor/archive and calling it deleted;
- declaring an entire repository extinct because one mechanism is extinct;
- pulling an interesting later donor forward without prerequisite/high-leverage evidence;
- creating a second semantic truth path.

## Completion through R8

By the end of R8, Atlas should have progressed from typed Rust source observations to a system that can:

~~~text
expand donor roots through admitted dependency closure
→ census implementation semantics deeply
→ incrementally query and recensus
→ reconcile and prove closure
→ compare observed mechanisms and research
→ explicitly select what Atlas should absorb
→ encode durable Technology Genomes/evidence in real ATLAS
→ implement Atlas-native replacements
→ recensus those replacements
→ physically extinguish eligible donor source scopes
→ continue with a stronger Atlas
~~~

R8 is not the end of census. It is the point where census-derived knowledge has a durable native carrier suitable for large-scale source-independent continuation.

## Construction protocol roadmap (added alongside R4.8; does not renumber any existing wave)

`.atlas/contracts/ASIR-CONSTRUCTION-MODEL.md` locks the architecture for the layer between ADL authoring and sealed `.atlas` construction: ACP (the provider wire protocol) and ASIR (construction-time typed semantic state, the same vocabulary `COMPILER-IR-SCHEMAS.md`'s HIR already locks post-seal). None of the items below are implemented yet; they are locked architecture awaiting production code, sequenced to land once R4's extraction-path dimensions are far enough along that construction-time (provider-proposed, not only extracted) semantics become load-bearing:

- **donor census, recursive dependency closure** — semantic-representation donor lane (MLIR/IRDL, xDSL, egglog, WASM Component Model, Cap'n Proto, AgentIR, FlatBuffers, rkyv, WASM spec; `.atlas/census/donors/`), each with real recursive dependency-closure accounting, not top-level-repository-only census.
- **ACP transport + typed construction-operation decode** — `atlas-construction-operation.schema.json` exists; the Rust decode/validation path from a provider's wire payload into that typed shape does not yet exist.
- **ASIR in-memory construction state** — a mutable, pre-seal typed graph distinct from today's per-dimension `SemanticObservation` records (which represent *extracted*, not *constructed*, semantics); does not exist yet.
- **dialect registry** — a real, queryable registry of the `atlas.*` namespaces `ASIR-CONSTRUCTION-MODEL.md` reserves, with the dialect-qualified-name-to-HIR-node-kind mapping table encoded as data, not only prose.
- **construction-operation verifier** — schema/type/semantic/security/obligation validation stages per `ASIR-CONSTRUCTION-MODEL.md`'s admission pipeline.
- **architectural-integrity verifier** — isolated post-transaction observed-architecture derivation, impact closure, HARD-invariant falsification, load-bearing equivalence and `ArchitecturalIntegrityReport` production under `ARCHITECTURAL-INTEGRITY.md`; not implemented yet.
- **typed effect/capability binding at construction time** — today's R4.8 EFFECT/STATE dimensions are extracted from existing source; construction-time proposals need the same typed effect/capability discipline enforced *before* admission, not only observed after the fact.
- **obligation attachment at construction time** — extending `SemanticObligationRecord` lineage to construction-proposed operations, not only extraction obligations.
- **evidence/provenance attachment at construction time** — wiring `ProviderReceipt`/evidence lineage through `AtlasConstructionOperation.evidence_refs`/`provider_receipt_ref` end to end.
- **semantic transactions** — the `CandidateChangeSet.semantic_changes[].construction_ops_refs` field exists (additive schema patch); the code path that batches/applies a transaction with `expect` clauses against a declared ASIR base does not.
- **Jev typed decision integration** — `DecisionProposal` already exists and already fits; what's missing is the actual construction-time caller that populates it from real alternative `AtlasConstructionOperation`/`CandidateChangeSet` candidates, plus (TARGET, evidence-gated) an egglog-informed equivalence-class/cost-extraction mechanism for enumerating alternatives, per `.atlas/census/donors/egglog.md`.
- **canonical textual debug printer** — `atlas inspect`/`atlas disasm`/`atlas explain`-style human-readable projections of sealed semantic state, explicitly never a canonical source-of-truth format.
- **canonical binary encoding + versioning** — already governed by `ATLASX-BINARY-WIRE-FORMAT.md`/`ATLAS-BINARY-WIRE-FORMAT.md`; ASIR-to-binary is a new producer of that same target format, not a new format.
- **deterministic canonicalization at the construction layer** — extending `ATLAS-SEMANTIC-COMPACTION.md`'s existing determinism target so construction-time nondeterminism (transaction ordering, provider phrasing variance) cannot leak into sealed output; currently untested at this layer.
- **provider-independent compaction** — confirming (with a real test, not only a doc claim) that ASIR-to-`.atlas` compaction never re-invokes a provider mid-compaction, matching `ATLAS-FORMAT.md`'s existing "post-seal compaction run MUST NOT invoke research/decision/synthesis/coding/verification models" rule.
- **extinction integration** — once a donor's studied semantics are Atlas-natively implemented, verified, and evidence-complete per `.atlas/contracts/BULK-DONOR-ABSORPTION.md`/the extinction contract, the same STAGED -> ... -> EXTINCT progression already governs these donors; none of the donors staged for this lane are extinction-eligible yet (they were staged for architecture study, not for absorption of specific mechanisms).

## Final invariant

Atlas Studio is not built first and used to census OSS later.

Atlas Studio is built **by** repeatedly censusing OSS donors and their complete admitted dependency graphs, learning from them under evidence discipline, selectively absorbing mechanisms into Atlas-native ownership, proving the replacements, and physically deleting donor source only when extinction is true.
