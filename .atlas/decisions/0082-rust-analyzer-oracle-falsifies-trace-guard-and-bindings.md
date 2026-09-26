---
id: atlas.decision.0082.rust-analyzer-oracle-falsifies-trace-guard-and-bindings
type: decision
status: accepted
canonical: true
---
# ADR 0082 — Realizable traces, a chain link costs a sixth of a bracket, and a call binds what Cargo and coherence select (G168)

## Context

FULL_OSS_REPLAY R10 put rust-lang/rust-analyzer, at pin `86493cee4d89`, in front of Atlas E15. It was the first First-50 donor decided ABSORBED (G75, ADR 0031: native path resolution) and was rank 1 of the recursive revalidation queue. Its own SCIP indexer (rust-analyzer 1.90.0, run over the pin) was the oracle.

Atlas E15 censused the whole pin:
- 31,250 functions and 25,359 INVOKES;
- 103,522 unresolved call sites, each recorded with its reason.

The challenge was: how does an LSP `textDocument/definition` request reach `ide::goto_definition`?

The oracle comparison worked at function level. A SCIP reference spelled as a call was attributed to the innermost `fn` item containing it; a syn parse of the donor gave the item extents, since the SCIP output carries no enclosing ranges. On that basis:

- **Precision:** 24,075 of Atlas's 24,167 call edges agree with the oracle (99.62%).
- **Recall:** 44.1% of the oracle's 54,597 call edges.
- **Missed edges:** of the 30,522 the oracle has and Atlas lacks, Atlas reports 28,397 as unresolved by name and 2,090 not at all.

Four defects were found:

1. **The trace lens answered the challenge with a false path.** It reported `PATH_OBSERVED`, `DERIVED`, eight steps. The path entered shared callees (`TextEdit::builder`, `Semantics::new`) and left through their return values to *other* callers. Every step was true, but no execution realizes the chain. A returned value is SUPPLIES_DATA from callee to caller (`composition::compose`), and the breadth-first search followed every relation forward.
2. **The generated AST was refused whole.** `crates/syntax/src/ast/generated/nodes.rs` (11,047 lines) and `crates/parser/src/syntax_kind/generated.rs` were refused by the stack-safety pre-scan, which invisibly removed 2,565 functions. The first scored 76, from `matches!(kind, A | B | …)` with 73 alternatives three brackets deep; the limit is 64. The scan charged one bracket level per operator link. Yet the guard's own documented measurements, on the same reduced (~2 MiB) stack, crash at 300 bracket levels (100 survive) but at 2,000 chain links (1,000 survive). Of the oracle edges Atlas lacked, 6,390 had an end in a refused file.
3. **`Type::name` picked the one visible impl of a generic trait.** rust-analyzer's `DefWithBodyId::from(function)` calls an `impl From<FunctionId>` that `impl_from!` expands. Atlas bound it to the only visible `impl From<EnumVariantId>`. Of the 109 call edges Atlas drew into `from`, 29 were wrong. Only 73% of these were right, against 99.7% for all other edges.
4. **A registry dependency bound a same-named workspace member.** rust-analyzer declares `lsp-server = { version = "0.7.9" }` and keeps `lib/lsp-server` (0.10.0) as a member. Cargo builds against the registry package; Atlas bound the member, producing 15 wrong edges. The same happens silently with `text-size` 1.1.1, `smol_str` 0.3.6, `la-arena` 0.3.1 and `line-index` 0.1.2. Their `Cargo.lock` holds both the member and the registry package at one version, and the SCIP symbol (package name and version, no source) cannot tell them apart.

## Decision

1. **A trace is realizable.** A SUPPLIES_DATA step is a return when the call site it cites is an INVOKES from its target to its source; otherwise it is an argument. A path may take a return only before its first descent (call, argument, closure, dispatch). After a descent, a return could reach only the caller it came from, which the search has already reached. STATE_FLOW passes through shared state, which carries no call context, so returns are open again after it. The search runs over (function, may-return) states. This is standard matched call/return reachability, written natively.
2. **A chain link costs a sixth of a bracket level.** `CHAIN_LINKS_PER_LEVEL = 6` covers operator and `as` chains; bracket nesting and `else` runs keep their one-for-one cost, and the limit stays 64.
   - At the documented crash depths, 2,000 links / 6 ≥ 300 brackets, so the charge never admits a chain deeper, in stack, than the bracket nesting already admitted.
   - Both the extractor and the resolver parse on the 256 MiB extraction stack.
3. **A generic trait's impl is not picked by method name.** When the unique trait-impl candidate for `Type::name` belongs to an impl of a trait spelled with generic arguments (`From<X>`, `TryFrom<X>`, `rustc_type_ir::inherent::Ty<I>`), the call is left `generic-trait-impl`. Such a trait may be implemented for one type many times: by macro-expanded impls, or by impls another crate declares under the orphan rule. Which one runs depends on argument types. A trait without generic arguments is implemented at most once per type, so its unique visible impl is still the impl.
4. **A dependency binds the package directory its declaration names.** That is its own `path`, else the `[workspace.dependencies]` entry it inherits (renames included), else a `[patch.*]` entry redirecting its package. A version, git or registry requirement binds no workspace library, whatever its name. Only an inherited entry whose workspace root manifest is not in view still binds by package name. The adapter records a `DeclaredSource` per dependency and reads `WorkspaceDependencies` from a root manifest.

## Consequences

- **The same pin, E15 against E16, compared against the same oracle:**
  - functions: 31,250 → 33,816 (the two generated files are admitted; `minicore.rs`, which syn cannot parse, stays refused);
  - INVOKES: 25,359 → 26,055;
  - DISPATCHES_TO: 696 → 1,462;
  - edges the oracle confirms: 24,075 → 24,888 (1,215 gained, none gained wrong);
  - precision: 99.62% → 99.82%;
  - recall: 44.1% → 45.6%;
  - Atlas-only edges: 92 → 46. None of the 46 is wrong: 25 call items declared inside function bodies, which SCIP names only as locals; the rest are on lines the oracle does not resolve (cfg- or feature-gated crates such as `proc-macro-srv`, `debug_assertions` bodies).
  - edges the oracle has and Atlas lacks without any record: 2,090 → 1,101.
- **Every one of the 46 wrong edges is gone,** and 402 edges the oracle confirmed are now reported unresolved by name (none silently):
  - 111 bind a registry package whose same-version workspace copy the oracle conflates with it. `Cargo.lock` decides for the registry, so the oracle is UNDECIDABLE there and E16 follows Cargo.
  - 239 were generic-trait picks that happened to be right: 133 through `rustc_type_ir::inherent::Ty<I>`, 58 `From`, 17 `TryFrom`, 15 `inherent::Const`, 8 `inherent::Region`, 6 `FromIterator`, 2 `Span`.
  - 52 follow from those as receivers typed through a refused call.

  Atlas's rule is never a pick. An impl is chosen only when coherence or Cargo establishes it.
- **The challenge.**
  - E15 answered with a false DERIVED path.
  - E16 answers `NO_PATH_OBSERVED` / UNKNOWN from the request handlers: the origin has 1,130 unresolved call sites, among them `snap.analysis.goto_definition`, a parameter's field whose type lives in another workspace crate. From `ide`'s `Analysis` it finds the DERIVED step from its closure into `goto_definition`.
  - The unresolved link is NA-CALL-TYPE-RESIDUAL's to take.
- **Atlas on itself is unchanged:** 6,250 functions and 5,132 INVOKES under both binaries. Atlas declares path dependencies, has no generic-trait picks, and has no file the pre-scan refused.
- **Tests and mutants.** Four tests pin the four rules, and 16 mutants were killed:
  - trace: returning after a descent, a state flow that keeps the descent, returns taken as descents, and the phase dropped;
  - guard: divisors 5 and 7, the operator arm or the `as` arm unweighted, and floor for ceil;
  - generic trait: the guard dropped or inverted;
  - binding: a registry entry by name, an inherited registry entry by name, `[patch]` ignored, the no-root fallback dropped, and an inherited path by key.

  Outside the process, with syn 3.0.6 on a 2 MiB thread, every newly admitted maximum (378-link `as`, `+`, `|` and or-pattern chains) parsed, was walked and was dropped. 300 nested brackets still aborted.
- **What stays UNKNOWN on rust-analyzer:**
  - 28,573 call sites, reported unresolved by name, most of them method calls on receivers Atlas does not type;
  - calls into macro-expanded items;
  - the `impl_from!` impls themselves;
  - generic-trait impl selection, which needs argument types;
  - which feature-gated code (`proc-macro-srv` behind `in-rust-tree`) is compiled at all;
  - the 657 files no frontend reads, 533 of them `.rast` parser expectations.
- **Capability epoch E16.** Traces are realizable; files with long flat chains are extracted; generic-trait impls and registry dependencies are no longer picked. Every processed donor's revalidation trigger was checked against these; see the R10 evidence.
