---
id: atlas.decision.0094.lowerable-bodies-reach-sh1
type: decision
status: accepted
canonical: true
---
# ADR 0094 — Lowerable bodies: the construction IR carries method bodies, and SR1 reaches SH1 (G181)

## Context

ADR 0069 left one gap in SR1: `BODY_UNOBSERVED` on `MaterializationMode::is_local`. The census held a function's identity, its signature and a fingerprint of its body, but nothing a backend could lower. Construction node M14 (executable body semantics, DEBT-CONSTRUCTION_IR) was MISSING, and ADR 0092 made its smallest slice a CREATION attack: NA-SELF-RECONSTRUCTION-BODIES.

## Decision

1. **The census records a lowerable body, or none.** The Rust extractor adds `body` to the FUNCTION_SIGNATURE record (`FunctionSignature.body`, a tree of `BodyNode`). It is recorded only when the whole body is one tail expression inside a small subset: `true`/`false`, a path without generic arguments, `==` or `!=`, and a `match` whose arms have path, or-path or `_` patterns and no guard. Anything else leaves `body` absent: a statement, a call, a macro, another operator or literal, parentheses, a binding. There is never a partial body. The body is syntax as written; it is not part of the record's identity. `BodyNodeKind` (LITERAL, PATH, BINARY, MATCH, ARM, WILDCARD) is a vocabulary enum. The container schema declares the field and the nested `body-node` record as schema generation G181. The field is optional, so older containers still read.
2. **The IR carries typed HIR bodies** (`atlas.construction-ir.v1`). An `IrFunction` may carry a `HirBody`: nodes in post-order, each with a kind (`HirNodeKind`: CONST, COPY, INTRINSIC, MATCH, a subset of COMPILER-IR-SCHEMAS "Core HIR node kinds v1"), a result type, operands, arms and lineage. The lifter resolves every path against the module it builds: `self` becomes a COPY of the receiver, `Self::V` or `Type::V` a CONST naming a unit variant, `==`/`!=` an INTRINSIC (`HirIntrinsic` EQ/NE). A path that names nothing in the module, or a signature that is generic, async, unsafe, extern or takes other parameters, leaves the body unlifted.
3. **`validate_module` refuses a bad body with a typed defect** (`BodyDefect`): BODY_MALFORMED_NODE, BODY_DANGLING_CHILD (a child not before its parent, or a root outside the body), BODY_UNTYPED_NODE, BODY_UNRESOLVED_REFERENCE, BODY_TYPE_MISMATCH, BODY_CAPABILITY_UNOBSERVED (a comparison over a type not observed to derive `PartialEq`, a copy of one not observed to derive `Copy`), BODY_NON_EXHAUSTIVE_MATCH and BODY_GAP_CONTRADICTION (a body and a `BODY_UNOBSERVED` gap at once). A function without a body still needs its gap (`SILENT_GAP`). The lifter runs the same validation: a body the IR refuses is dropped whole and becomes the function's `BODY_UNOBSERVED` gap, with the defect as the reason.
4. **The Rust backend emits the methods it can** (`atlas.construction.rust-backend.v1`): an inherent method with a body, an emitted owner type, and either no parameters or `self` by value, in an `impl` block after its type. Any other function is omitted with its reason.
5. **Verification covers the method.** The generated differential runs the shadow's method against the compiled original's over every variant (`method_<name>`). The semantic check censuses the shadow, compares the method's signature, and lowers its body again through the same lifter; the result must equal the module's body. What the shadow does not reconstruct is declared: a doc comment the census records over several lines comes back as rustdoc's one-line summary. The objective named the original tests; the core tests that reach `is_local` do so only through donor code and cannot run against a shadow without replacing the original (SH4), so the exhaustive differential over the method's whole domain (five variants) is the verification used, and the objective is met in that stated sense.

## SR1-3

Attempt 3 uses the same target, `core::donor::MaterializationMode`, over a census container of the G181 tree. `is_local` is `self != Self::RemoteMetadata`. It is lifted from its FUNCTION_SIGNATURE record (the body), the type's SYMBOL record (derives `Copy` and `PartialEq`) and the `RemoteMetadata` variant's SYMBOL record. The module has no gap. The backend emits the type and `is_local`. The shadow's `is_local` agrees with the original on all five variants, and every other check is EQUIVALENT. The doc comment's wrapping is the declared variation, so the verdict is `RECONSTRUCTED_WITH_DECLARED_VARIATION`. By the contract that verdict reaches SH1.

A body outside the subset keeps its gap. In the same file, every method of `StorageState` stays `BODY_UNOBSERVED`: string literals, a call, `Some(..)` and `matches!` are all outside it.

## Falsification

- The extractor test records the subset and refuses twelve near misses: a call, arithmetic, a macro, a statement, a guard, a binding, parentheses, a number, a generic path, `!`, `&&` and `<`. The record id does not depend on the body.
- The container round-trips a nested body exactly.
- The IR tests refuse a malformed, dangling, untyped, unresolved, mistyped, underived or non-exhaustive body with its defect, and refuse lineage outside the inputs.
- The runtime tests lift the real `is_local`, keep `StorageState`'s gaps, drop a body over a type without `PartialEq`, and catch a shadow method with another body, a body the census cannot lower, or another signature.
- Mutants over each new rule were killed and restored byte-identical.

## Consequences

- **M14 is EXISTS** for this subset. DEBT-CONSTRUCTION_IR gains a body IR, but only for pure, call-closed tail expressions. Calls, bindings, statements and data-carrying variants stay outside it; each is a residual, not a guess.
- **SH1 is reached.** SH2 (a whole module) is next for this lane. `BODY_UNOBSERVED` closes for SR1, and stays the gap kind for any body outside the subset.
- **Capabilities are conservative.** A copy of `self` needs an observed `Copy` derive, although `!=` only borrows. This may refuse a body that would compile. It never admits one that would not.
