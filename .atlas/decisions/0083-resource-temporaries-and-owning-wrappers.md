---
id: atlas.decision.0083.resource-temporaries-and-owning-wrappers
type: decision
status: accepted
canonical: true
---
# ADR 0083 — A temporary is released at its statement's end, and an owning wrapper carries its resource (G169)

## Context

G169 took NA-RESOURCE-RESIDUAL, the native queue head the pressure map selected after replay R10. DEBT-RESOURCE names its own falsification condition: a release that rustc's MIR places elsewhere than the RESOURCE record claims.

Atlas E16 recorded 8 acquisitions on itself and released 2 of them. Read against the source, the 6 without a release are these:

- **3 owning wrappers.** A file is taken straight into `BufReader::new` or `BufWriter::new`, and the wrapper is let-bound (`census_file`, and the payload and writer of `construct_file`).
- **1 hand-on.** A let-bound file is taken by `Read::take` into a let-bound adapter (`read_file_content`).
- **1 statement temporary.** `File::open(parent)?.sync_all()?;` (`publish`).
- **1 detached thread** (`drain`). Its `JoinHandle` is dropped, which detaches the thread, so this correctly has no release (ADR 0072).

rustc's MIR, emitted with source spans (`RUSTC_BOOTSTRAP=1 -Zunpretty=mir -Zmir-include-spans`), shows where each of these values is dropped. It is the oracle here, independent of Atlas.

## Decision

1. **An owning wrapper carries its resource.** Two constructions take a file or socket by value and own it, so dropping the result drops the resource:
   - the std constructors `BufReader::new`, `BufWriter::new` and `LineWriter::new`, and their `with_capacity` forms (the owned argument is the last one);
   - the `Read` adapters `take`, `bytes` and `chain`.

   An acquisition wrapped this way in a `let` initializer makes that `let` its holder.
2. **A holder handed on is followed.** A let-bound holder may be taken, as the whole initializer of a later `let` in the same block, into an owning wrapper or adapter. Its uses up to that `let` are checked as before, and then the new binding is the holder. A holder moved anywhere else, or released before the hand-on, still claims nothing.
3. **A temporary is released at its statement's end.** An acquisition that is only the receiver of a method borrowing it, inside an expression statement or a `let` initializer, is dropped at that statement's `;`. The release is STATEMENT_END, a new release kind, with holder `(temporary)`, and its status is INFERRED because by-value methods are told apart by name. A spawned thread's handle joined in the same statement is released at the join (JOIN, DERIVED). The rule does not apply in these places, whose temporaries follow other rules:
   - nested blocks and closures;
   - the conditions of `if` and loops, and `match` scrutinees;
   - macro arguments;
   - a `let` whose initializer is a borrow or a struct, tuple or array literal, which may extend a temporary's lifetime;
   - a consuming method (`into_*`, `take`, `bytes`, `chain`, `into`, `try_into`).

## Consequences

- **Atlas on itself: 2 → 7 releases, every one where MIR drops the resource.** All 7 match at the exact line and column:
  - the 2 G157 releases (a block end, and an explicit `drop`, which is a call to `std::mem::drop` in MIR);
  - 3 wrapper releases;
  - 1 hand-on through `take`;
  - 1 statement temporary.

  The only acquisition left without a release is the detached thread.
- **A fixture crate compiled by rustc confirms each rule.**
  - All 7 claims agree with MIR: 3 SCOPE_END through wrappers and a hand-on, and 4 STATEMENT_END, including two guards in one statement.
  - Each refused case drops elsewhere in MIR:
    - an `if` condition's guard drops after the condition, not at the `;`;
    - a wrapper moved into a callee is dropped inside it;
    - `Option::take` reached through a guard is refused as a by-value name. MIR drops that guard at the `;`, so it stays UNKNOWN rather than being claimed wrongly.
- **Falsification.** 6 mutants were killed:
  - consuming methods claimed;
  - `if` conditions entered;
  - no hand-on;
  - wrappers not followed;
  - the release placed at the acquisition;
  - wrappers of guards accepted.

  A release before a hand-on accepted cannot occur in compiling code.
- **Vocabulary.** `ResourceRelease` gains STATEMENT_END. It is a vocabulary value, not a new field, so `.atlas` resource records keep their schema.
- **What stays UNKNOWN:**
  - temporaries in conditions, tails and macro arguments;
  - holders moved into callees or stored;
  - fields and statics;
  - detached threads;
  - non-std resources;
  - FFI acquire/release pairs;
  - child processes.

  Reconciliation with OWNERSHIP move records and a MIR engine in the census itself remain next steps for DEBT-RESOURCE. The MIR oracle stays a test-time instrument, never a census input.
- **Capability epoch E17.** Release points are placed for owning wrappers, hand-ons and statement temporaries. The processed replays' triggers name FFI pairs, process resources and non-std handles, not these, so every verdict stays CURRENT.
