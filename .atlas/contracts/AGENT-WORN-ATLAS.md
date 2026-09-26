---
id: atlas.contract.agent-worn-atlas
type: contract
status: active
canonical: true
---
# Agent-Worn Atlas: the AI works through Atlas

**AI IS NOT ATLAS. ATLAS IS NOT THE AI.**

The AI is the pilot: it brings mission intent, hypotheses, designs and creative choices. Atlas is the engineering body the AI wears. It provides perception, semantic memory, a composed world model, constraints, invariants, impact, verification and recensus.

The AI reasons *through* Atlas while working. It does not read the repository first and then run Atlas afterwards as a check.

~~~text
MISSION ─▶ understand(target, mission) ─▶ MissionContext ─▶ AI decides
        ─▶ impact(change) ─▶ AI implements ─▶ verify(before, after) ─▶ recensus prove
~~~

## Semantic composition (DEBT-SEMANTIC-COMPOSITION)

`core/src/composition` composes the census's typed records into five levels:

| Level | Object | Source of truth |
| --- | --- | --- |
| 1 | `FunctionBehavior` | FUNCTION_IDENTITY / FUNCTION_SIGNATURE plus every dimension's records joined by `function` |
| 2 | `ComponentBehavior` | one source artifact: its functions, interface, effects, state, dependencies and per-dimension coverage |
| 3 | `SubsystemModel` | an ADL entity with a materialization path: declared responsibility, observed interface, effects, owned state |
| 4 | `CapabilityModel` | a declared ADL capability: providers and consumers; realization is `UNKNOWN` until mapped |
| 5 | `ArchitectureModel` | declared dependencies reconciled with Cargo and resolved calls |

Around these levels sit:

- **Typed relations.** `INVOKES`, `SUPPLIES_DATA`, `STATE_FLOW`, and (G141, mission M5) two more:
  - `DISPATCHES_TO`, from a trait method declaration to each implementing method. It is INFERRED: the impl's spelled trait name joins the one workspace declaration of that trait name and method, and there is no link when the join is ambiguous.
  - `ENCLOSES`, from a function to the closure regions it defines. It is DERIVED from the closure's scope, but it is not an invocation.

  A trace through either is INFERRED. None of these relations implies cause, ordering or authority.
- **Field reads** (G146, mission M6). Each `FunctionBehavior` carries two lists, read from the projections of its DATA_FLOW uses (OBSERVED syntax):
  - `projections`: the field chains the function reads (`report.census.facts`);
  - `whole_parameter_uses`: the parameters it uses whole.

  A whole use means the value's content may be read wherever the value goes. Call arguments are not bound to callee parameters (GAP-ARGUMENT-BINDING), so a field is never claimed unread beyond the function itself.
- **Typed invariants.** The kinds are `STATE`, `ORDERING`, `DEPENDENCY`, `AUTHORITY`, `RESOURCE`, `SAFETY` and `CONSTRUCTION`, each with a status, evidence and a residual.
- **`UnderstandingGap` records.** Each names a question class Atlas cannot answer yet and the debt that owns it.

### Epistemic rules

1. **Status and evidence on every claim.** Every composed claim carries an `EpistemicStatus` and the record ids, spans or manifests it rests on.
2. **Purpose.** Purpose comes only from a declaration: an ADL `responsibility` for a subsystem, or, for a component, the documentation its author wrote (G128: the file's own `//!` text, else the `///` text on the `mod` item declaring it, censused as DECLARED text on SYMBOL records outside their identity); for a function, its own `///` text (G132, carried on the symbol its FUNCTION_IDENTITY embeds). A declared purpose is the author's statement, never checked against behavior. Otherwise it is `UNKNOWN`. It is never named from an identifier, a path or a file name.
3. **Universal claims.** A claim such as "written only by" or "originates only in" is `DERIVED` only when the dimension it ranges over is `OBSERVED` on every artifact in scope. Otherwise it is `INFERRED`, and its residual names the files with unproven coverage and the unresolved call sites.
4. **Dependency invariants.** "A cannot invoke B" is `OBSERVED` from the Cargo closure only when no dependency path exists. Any resolved call from A to B turns it into `CONFLICT`, with that call as evidence.
5. **Absence.** "No path observed" or "no effect observed" is `UNKNOWN` while unresolved call sites exist. It is never reported as absence.
   - **Callee spelling (G123).** Every CALL record carries its callee as written (OBSERVED syntax). An unresolved site spelled with a function's name makes its caller an `INFERRED` candidate, listed apart from resolved callers and never merged with them.
   - **Unattributed sites.** An unresolved site without a name (a closure, a function pointer) may reach anything. It is counted as unresolved minus named, never read from a stored counter, so a model written before spellings existed claims no narrowing.
   - **Closure regions (G133).** A closure is its own executable region: a `CLOSURE` function named `{closure@line:column}`, whose calls, effects, state, data flow and the rest are its own, never the enclosing function's, and whose `enclosing` names the region that defines it. Calls inside `async` blocks are not censused, and every impact residual says so.
   - **State identity.** State is `<self type>.<field>`: a `self.field` touched by an inherent and a trait impl is one piece of state. Identity is by spelling, so same-named types of different modules share a key.
6. **Determinism.** Composition is deterministic and independent of record order. Its totals (`composed:*`) are part of the census snapshot, so a change in composed understanding is a census change that `recensus prove` must see intended.

## Operations (`atlas-systemizer agent <op>`)

| Operation | Answers |
| --- | --- |
| `understand --target T --mission M` | the bounded `MissionContext` for one mission |
| `explain --target T` | the composed object(s) a selector denotes |
| `impact --target T[,T…]` | resolved transitive callers (a lower bound), components, state readers, invariants at risk |
| `trace --from A --to B` | shortest typed path, or `NO_PATH_OBSERVED` with status `UNKNOWN` |
| `why --from A --to B` | direct relations, dependency edges and invariants between two targets |
| `invariants`, `unknowns`, `effects`, `state`, `dependencies`, `capabilities --target T` | projections of the model |
| `resources`, `causal` | the gap that owns the question (`GAP-RESOURCE`, `GAP-CAUSALITY`), never an answer |
| `compare --from A --to B` | two targets side by side |
| `plan --target T` | a `DERIVED` checklist: what to preserve, re-verify and resolve first; the design stays the agent's |
| `hypothesis --claim C` | an agent hypothesis checked: `VALIDATED`, `FALSIFIED` or `STILL_HYPOTHESIZED` |
| `benchmark [--cases <json>]` | the Agent-Utility benchmark (`evidence/agent/benchmark.json`): each case's answer, correctness, evidence count and, for unknown-honesty cases, whether Atlas declined |
| `verify --before <model.json>` | invariant regressions, new authority and dependency changes; exits non-zero on regression |
| `model` | the whole `WorldModel` (`--model` reuses one) |

**Targets.** Targets are exact structural selectors: `subsystem:<Name>`, `path:<file or directory>`, `fn:<name>` or `fn:<Owner>::<name>`, `state:<self type>.<field>`, and `capability:<Name>`. An unknown selector is an error, never a fuzzy match. A path prefix matches only at a separator.

## MissionContext

A `MissionContext` is the primary payload the agent reasons over, not the graph. It holds:

- the scope and the target and neighbour subsystems;
- entry points and the public surface;
- critical calls and data paths crossing the scope, and the state the scope touches;
- effects, constraints and invariants;
- unknowns (coverage and unresolved counts), the impact frontier, next questions, and semantic compression (raw records per composed object).

Lists are bounded, and the rest is counted.

The mission text is recorded as the agent's `DECLARED` intent and never selects facts. The `structural_digest` excludes it, so two missions over the same target of the same reality with the same Atlas version produce the same structural digest.

## Hypotheses stay apart

The agent may hypothesize freely: purpose, mechanisms, designs. Atlas records a hypothesis as `HYPOTHESIS` and checks structured forms against the model:

- `writes-only:<state>:<fn,...>`
- `never-invokes:<A>:<B>`
- `invokes:<fn>:<fn>`
- `effect:<fn>:<CATEGORY>`

The outcome is Atlas's. A hypothesis is never merged into the world model.

## Understanding failure becomes work

When the agent asks a question the model cannot answer, the answer is the owning `UnderstandingGap`. The gap is routed to essential debt, Atlas is improved, and the question is retried.

Repeatedly bypassing Atlas with manual reading is evidence of a missing capability, recorded per mission as `manual_source_reads`. Direct source reading remains legitimate for `UNKNOWN`, falsification and deep dives.

## External models

When an external AI provider takes part in `.atlas` creation or engineering work, record:

- the provider, the model and the input class;
- the output provenance, trust level and verification status (`contracts/EXTERNAL-PROVIDER-TRUST.md`).

Provider text is never Atlas truth by itself.

## Not Agent-Worn Atlas

None of these is Agent-Worn Atlas:

- dumping records or JSON into the model;
- grep or embedding search over source;
- RAG over files;
- calling census after the agent has already finished;
- calling generic graph nodes a "world model".

## Missions

An `AGENT_MISSION` generation records `evidence/agent/M<n>-*.json`:

- the Atlas-first sequence of operations, and what each one answered;
- what Atlas knew, and what stayed `UNKNOWN`;
- every manual source read, with its reason: `IMPLEMENTATION_DETAIL`, `UNKNOWN` or `FALSIFICATION`;
- the gaps found, and whether each was fixed in the mission or routed to a debt;
- a challenge question Atlas was expected to fail, answered without invented confidence;
- the benchmark result.

A `verify` regression caused by an intended identity change (for example, re-keyed state invariants) is named in the record, never hidden.

