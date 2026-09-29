---
id: atlas.decision.0088.standard-macro-arguments-reach-every-resolver-pass
type: decision
status: accepted
canonical: true
---
# ADR 0088 — Standard macro arguments reach every resolver pass (G174)

## Context

G174 is replay R13 of `immunant/c2rust`, pinned at `7fe0cf95` (upstream master). The C-to-Rust translator had been REVALIDATION_REQUIRED since G150, at rank 1.

Since G119 (ADR 0041), the syntactic extractor has read the arguments of the standard macros (`assert*`, `debug_assert*`, `dbg`, `eprint*`, `format`, `format_args`, `panic`, `print*`, `todo`, `unimplemented`, `unreachable`, `vec`, `write*`). It parses each as its documented input and claims a CALL site for every call inside it. G120 (ADR 0042) gave each argument a role:
- **Evaluated:** by value.
- **Formatted:** through `format_args!`, by shared reference.
- **Compared:** by `assert_eq!` or `assert_ne!`, by shared reference.
- **WriteTarget:** the `write!` destination.
- **FormatString:** the format string itself.

The path-call resolver never walked those arguments. It skipped `Expr::Macro` and `Stmt::Macro`, so every call site inside a standard macro stayed unresolved, whatever its form. The only place it read inside a macro was the resource holder analysis, which scanned the macro's tokens for the holder's name.

On c2rust, the missed calls were dominated by known sound classes. The pattern stood out on Atlas's own crates, measured against the rust-analyzer index: about 2,500 oracle-confirmed workspace calls sat inside `assert_eq!`, `assert!`, `vec!`, `format!`, `assert_ne!` and `println!`, with no Atlas resolution.

**The oracle was also wrong.** The replay found an error in the oracle recipe used since G168. Its `rust-project.json` named the toolchain's `sysroot` but no `sysroot_src`, so rust-analyzer loaded no standard library and indexed nothing inside any standard macro. With `sysroot_src` set, the index sees those calls. Recall figures measured with the old recipe (replays R10 to R12, and this replay's first pass) were measured against an oracle blind inside std macros. Precision checks are unaffected: they only ask whether the oracle agrees with an edge Atlas claims.

## Decision

1. **Expansion.** After parsing each module file, the resolver rewrites its own copy of the syntax tree:
   - Every standard macro invocation that `macros::recover` recovers becomes the evaluation it performs: a tuple of its arguments, marked `#[atlas_expanded_macro]`.
   - Evaluated arguments stay by value. Formatted and Compared arguments become `&arg`. The `write!` destination becomes `&mut arg`, the autoref of `write_fmt`. The format string evaluates nothing and is dropped.
   - The tuple keeps the invocation's delimiter span, and each argument keeps its own tokens and positions.
   - A statement macro becomes an expression statement with the same `;`.
2. **Every pass sees the arguments.** Bindings, block scopes and block items, closures, typed locals, receiver typing, the method probe and its autoref guard, holder moves and releases, and the std-path effect and concurrency tables all read the arguments where the extractor claims its CALL sites.
   - A holder named by value in `vec!` or `dbg!` is moved.
   - A holder named in a formatting or comparing macro is borrowed.
   - A `write!` destination is borrowed mutably.
3. **What stays opaque.** A bare macro name the crate may bind to another macro stays opaque. The crate's files are discovered first, unexpanded, and `macros::shadowing_macro_names` collects from them:
   - every `macro_rules!` name they define. Textual scope reaches child modules, and `#[macro_use] mod` reaches parents.
   - every name a `use` binds, as its last segment or its rename. A standard macro imported from `std`, `core` or `alloc` under its own name is exempt.
   - every bare name, when an `extern crate` other than `std`, `core` or `alloc` carries `#[macro_use]`. The macros it brings in are not read.

   A path-qualified `std::`, `core::` or `alloc::` form still names the standard macro. Any other macro stays opaque. So does an invocation whose format string holds implicit captures (`{x}`, `{:w$}`): a capture has no expression of its own to place at its position, so the invocation is left as it was.

   The syntactic extractor applies the same set within each file it reads. It sees one file, so a `macro_rules!` of another file stays the queued residual below.
4. **Arguments evaluated only on some paths are conditional.** A release inside one is never claimed where it is written: it leaves the holder's release unclaimed, like a release inside a nested block. These arguments are:
   - the message arguments of a failing `assert*!`;
   - every argument of `debug_assert*!`, which is compiled out without debug assertions;
   - the right operand of `&&` and `||`, in any expression. This was an older hole, reachable before this change through plain code.

   The resolver marks such an argument `#[atlas_conditional]`, and the holder analysis enters it one level deeper.
5. **Temporaries keep the old rule.** The G169 statement-release analysis treats the marked tuple as the opaque invocation it replaced.
   - A formatting macro drops its temporaries inside std's expansion. rustc 1.90.0's MIR drops the temporary of `println!("{:?}", File::open(p)?.metadata()?)` in `std/src/macros.rs`, not at the source `;`.
   - So no STATEMENT_END release is claimed for a temporary acquired inside any standard macro's arguments.
6. **The oracle recipe names `sysroot_src`.** The recipe is the toolchain's `lib/rustlib/src/rust/library`. rust-analyzer's SCIP output remains a test-time differential oracle, never a census input.

## Evidence

- **Atlas itself.** The G173 resolver and this one ran over one snapshot of HEAD `0a217dec` and were checked against rust-analyzer 1.90.0's SCIP index of that snapshot.
  - Resolved path calls: 7,644 → 9,182 (+1,538).
  - Withdrawn: 0. Retargeted: 0.
  - 9,178 of 9,182 claimed edges are confirmed. The other 4 sit on lines holding a multi-byte `±`, where the index records no occurrence at Atlas's column. Each reads correct; one of them is G173's known case.
  - Withheld calls: 39 → 63. Unresolved: 2,039 → 2,912. The newly visible calls that Atlas cannot justify are reported, not guessed.
- **c2rust at `7fe0cf95`, function level,** against the corrected oracle for each crate:
  - c2rust-transpile: 1,215 → 1,228 edges, 1,178 → 1,191 agreed, recall 58.75% → 59.40%.
  - c2rust-refactor: 2,016 → 2,035 edges, 2,012 → 2,031 agreed, recall 60.73% → 61.30%.
  - No edge was lost or retargeted.
  - The 41 edges only Atlas holds read correct. Most are calls through the single impl of `StructuredStatement` that the index names by the trait declaration. The rest are nested functions and local impls that it conflates or does not name.
- **Review.** An independent adversarial review of the first version found two holes. Each was reproduced against rustc 1.90.0's MIR and closed here:
  - **H1.** A `join` or `drop` in an `assert!` message, or in any `debug_assert!` argument, was claimed as the holder's release. On the passing path, or in a release build, it never runs. The review also found the same shape in the older `a || h.join().is_ok()`.
  - **H2.** A bare `dbg!`, `vec!` or `println!` bound to another macro was expanded as the standard one:
    - by `use m::dbg`;
    - by `use std::vec as println`;
    - by a `macro_rules!` of another file reaching the invoking file through `#[macro_use] mod`.

    In each case rustc reports the argument function never used.

  The review found the rest sound, including:
  - the typed-local guard (`let v = vec![S::new()]` is never typed `S`);
  - the G169 temporaries;
  - `write!` destinations under `?`;
  - hygiene;
  - positions.
- **After the fixes,** re-measured on the same inputs:
  - Atlas's own edges are unchanged: 9,182, with none withdrawn.
  - c2rust loses 5 resolved calls. They sit in a cross-checks crate whose root carries `#[macro_use] extern crate`, and are now opaque.
  - The function-level differentials are unchanged.
- **Tests:**
  - `standard_macro_arguments_resolve_where_the_expansion_evaluates_them`: calls, typed receivers, block items, closures, shadowing, implicit captures, a file-local `vec!`, an unknown macro.
  - `standard_macro_arguments_move_or_borrow_holders_as_their_expansion_does`: the `write!` destination, `assert_eq!` operands, `vec!` and `dbg!` moves.
  - `a_release_inside_a_conditionally_evaluated_argument_is_never_claimed`: H1 and the short-circuit case.
  - `a_standard_macro_name_the_crate_may_rebind_stays_opaque`: H2's shapes, an external `use other::assert`, `#[macro_use] extern crate`, a std macro imported under its own name.
  - The G169 `println!` temporary case.
- **Mutants: 21 killed.**
  - Ten against the first version:
    - no expansion; statement macros skipped; expression macros skipped;
    - captures expanded; local macros ignored;
    - temporaries see the expansion; marker not matched;
    - formatted by value; write target by value; evaluated by reference.
  - Eleven against the review fixes:
    - debug arguments or messages not conditional; conditional ignored by holders; short-circuit not a branch;
    - no crate discovery; use leaves ignored; a std own-name import shadowing; a std rename ignored; the use path root forgotten;
    - `#[macro_use]` ignored; the every-bare-name entry applied to paths.

## Consequences

- **DEBT-CALL:** the resolver reaches every call site the syntactic extractor claims inside a recovered standard macro. Capability epoch E21.
- **Still opaque:**
  - `macro_rules!` macros of the workspace or its dependencies (c2rust's `match_or!`, `log`'s `info!`);
  - every bare standard name in a crate with a `#[macro_use] extern crate`, since the dependency's exported macro names are not read;
  - std macros outside the G119 list (`matches!`, `concat!`, `env!`);
  - invocations with implicit format captures;
  - temporaries acquired inside any macro's arguments.
- **Queued:** NA-WORKSPACE-MACRO-ARGUMENTS, with two parts:
  - argument recovery for a workspace `macro_rules!` whose transcriber places a fragment in an expression position;
  - the crate-wide shadowing set for the per-file syntactic extractor. Since G119 it claims call sites inside a bare standard name that another file's `macro_rules!` rebinds; such claims have a callee but no resolution.
- **Replay figures:** recall figures in replays R10 to R12 were measured against an oracle without the standard library. Their precision verdicts stand.
