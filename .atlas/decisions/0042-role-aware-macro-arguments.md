---
id: atlas.decision.0042.role-aware-macro-arguments
type: decision
status: accepted
canonical: true
---
# ADR 0042 — Standard macro arguments carry each dimension's own semantics (G120)

## Context

G119 (ADR 0041) recovered the arguments of standard expression macros for CALL only. The other seven expression walkers still treated every macro as opaque:

- CONTROL_FLOW, DATA_FLOW, STATE, EFFECT, OWNERSHIP, CONCURRENCY and PERSISTENCE.

Feeding the same expression list into all seven would have invented semantics. Two examples:

- a format argument is borrowed, never moved;
- `write!(dst, ..)` makes `dst` the `&mut` receiver of `write_fmt`, which is a different access from reading it.

## Decision

1. **Roles.** `macros::recover` gives every argument the role its documented expansion defines:
   - `Evaluated`: taken by value;
   - `Formatted`: taken by shared reference;
   - `Compared`: an `assert_eq!`/`assert_ne!` operand, taken by reference;
   - `WriteTarget`: the `&mut` receiver of `write_fmt`;
   - `FormatString`: the format string itself.

   It also returns the implicit captures of the format string (`{x}`, `{:>w$}`), each at the exact source column of the identifier. Explicit named arguments are not captures.
2. **Per-dimension semantics.** Each walker applies its own dimension's meaning to a role:

   | Dimension | What a recovered macro contributes |
   | --- | --- |
   | DATA_FLOW | Every role is a Use; every implicit capture is a Use of the captured binding. |
   | OWNERSHIP | Only `Evaluated` is a value position (a move or copy). Formatted and compared operands and write targets are never moves; the implicit borrow is not claimed. |
   | STATE | Arguments are walked, but a `WriteTarget` is not claimed, because whether it mutates depends on the resolved `Write` impl. |
   | EFFECT | The `assert*!`/`debug_assert*!` family is a PANIC site (INFERRED, like the other macro spellings), and effects inside arguments belong to the caller. |
   | CONTROL_FLOW | An `assert*!(..);` statement is a statement-level decision point: fall through, or PANIC (INFERRED). |
   | CONCURRENCY, PERSISTENCE | Arguments are walked. |
3. **Opaque stays opaque.** These are never recovered:
   - unknown macros;
   - procedural and attribute macros;
   - locally shadowed standard macros;
   - arguments that fail to parse.

   Every partial dimension's UNKNOWN diagnostic now names the file's opaque macro/attribute residual. No dimension claims coverage from this change.
4. **Scope coverage.** The G119 rule (best engine per artifact, then the worst artifact) is now tested adversarially for every dimension. It also covers UNSUPPORTED, which never outranks, and never drags down, a supported artifact.
5. **Derived metrics.** The census snapshot carries `records:<DIM>|<engine>` and `obligations:<DIM>|<engine>|<STATUS>` totals.
   - A generation's metrics are `[[generation.metric]]` entries, checked against its own pre and post snapshots.
   - The dimension table's UNKNOWN and UNSUPPORTED counts are checked against the head snapshot.
   - This fixes G119's two mismatched hand-written numbers. The exact G119 delta is +4,561 syntactic CALL records, of which 4,363 are inside macro arguments (`evidence/census/G119/call-metric-reconciliation.json`).

## Consequences

- **DATA_FLOW differential.** Checked against rust-analyzer's SCIP local references (an offline, macro-expanding differential), every new DATA_FLOW record is accounted for, and no record is lost (`evidence/census/G120/data-flow-scip-differential.json`). SCIP emits no occurrence for implicit captures, so each capture was verified against the literal source text instead.
- **No oracle yet for six dimensions.** CONTROL_FLOW, STATE, EFFECT, OWNERSHIP, CONCURRENCY and PERSISTENCE still have no independent oracle. No second engine is claimed for them; this stays recorded oracle debt.
- **Debts advanced.** CONTROL_FLOW, DATA_FLOW, STATE, EFFECT and OWNERSHIP advance at G120 on measured deltas. CONCURRENCY and PERSISTENCE are handled but did not change materially on the workspace. No debt closes.
- **Falsification.** 12 mutants, all killed.
