---
id: atlas.contract.asir-construction-model
type: contract
status: active
canonical: true
---
# Atlas Semantic Intermediate Representation (ASIR) and the Construction Protocol (ACP)

## Purpose

This contract locks the missing layer between `ADL-TO-ATLAS.md` (what a human+AI author *intends*) and `ATLAS-FORMAT.md`/`ATLAS-CREATION-PIPELINE.md` (what the *sealed* `.atlas` binary contains): the typed semantic vocabulary and wire protocol a provider (Claude, GPT, Codex, a local model, or a future provider) actually constructs *against* while Atlas is still open for construction, and the rule that a provider proposes typed operations, never canonical bytes or canonical prose.

It does not restate the ADL/`.atlas`/`.atlasx` artifact boundary; `ADL-TO-ATLAS.md`, `ATLAS-FORMAT.md`, `ATLAS-TO-ATLASX.md`, `ATLASX-FORMAT.md` and `ATLASX-BINARY-WIRE-FORMAT.md` remain the canonical source for that boundary. It does not restate provider trust/capability rules; `EXTERNAL-PROVIDER-TRUST.md` remains canonical for those. It does not invent a second typed-IR vocabulary; `COMPILER-IR-SCHEMAS.md`'s HIR node/type kinds remain the canonical closed vocabulary this contract's construction operations target.

RFC 2119 MUST/MUST NOT/SHOULD language below is normative. Every claim is labeled NOW (implemented in production code today), CONTRACT (locked, not yet implemented), or TARGET (roadmap direction, may still change through `BLUEPRINT-EVOLUTION.md`).

## The missing layer, precisely

```text
ADL                              -- desired truth (WHAT SHOULD EXIST)
  |
  v
Atlas Construction                -- research, census, Jev-class decisions, coding, testing
  |         (this contract: ACP transport + ASIR object model)
  v
ASIR (in-memory, mutable, pre-seal)
  |         verify + normalize + canonicalize + seal
  v
.atlas                           -- exact truth (WHAT EXACTLY EXISTS), sealed and immutable
  |         package/closure/sign
  v
.atlasx                          -- closed-world system capsule
  |         compiler pipeline (COMPILER-IR-PIPELINE.md, R5-R8, TARGET)
  v
HIR -> MIR -> LIR -> MachineIR
```

`ADL-TO-ATLAS.md` already locks the ADL boundary. `ATLAS-FORMAT.md` already locks the sealed-artifact boundary. Neither document says what a provider actually emits *during* construction, before anything is sealed, or what typed vocabulary that emission is checked against. That is this contract's job. [CONTRACT]

## ASIR and HIR are the same vocabulary at different lifecycle stages, not two vocabularies

`COMPILER-IR-SCHEMAS.md` already locks a closed, versioned, typed node/type-kind vocabulary for `HirProgram`/`HirModule`/`HirType`/`HirFunction`/HIR body nodes (`CONST`, `LET`, `ASSIGN`, `CALL`, `IF`, `MATCH`, `LOOP`, `STATE_READ`, `STATE_WRITE`, `STATE_TRANSITION`, `EFFECT`, `RESOURCE_ACQUIRE`, `SPAWN`, `LOCK`, `TX_BEGIN`, `FIELD_GET`, ... — the full v1 list in that document). That vocabulary already maps almost exactly onto the R4 semantic dimensions materialized in production today (`SYMBOL`/`TYPE`/`FUNCTION_IDENTITY`/`FUNCTION_SIGNATURE`/`CALL`/`CONTROL_FLOW`/`DATA_FLOW`/`STATE`/`EFFECT`, R4.3-R4.8) and the dimensions the roadmap still owes (`OWNERSHIP`, `CONCURRENCY`, `PERSISTENCE`, R4.9-R4.11).

`COMPILER-IR-SCHEMAS.md` currently scopes HIR as **compiler-owned and post-seal**: `HirProgram.atlasx_root_id` is required, meaning today's HIR contract assumes a sealed `.atlasx` already exists and HIR is *derived from* it (`ATLAS-TO-ATLASX.md` -> compiler pipeline -> HIR -> MIR -> ...). That scoping is correct and MUST NOT change for compiled/executable HIR.

**ASIR is the same node/type-kind vocabulary used one lifecycle stage earlier: construction-time, mutable, pre-seal, with no `atlasx_root_id` yet because none exists.** An ASIR graph under active construction has:

- an in-progress, potentially incomplete/inconsistent set of modules/types/functions/body nodes, exactly the closed vocabulary `COMPILER-IR-SCHEMAS.md#hir-v1` already defines;
- open obligations (unresolved symbols, unbound effects, unproven preconditions) that a sealed HIR/`.atlas` MUST NOT carry;
- a construction history (which `ACP` transaction created/mutated which node) that a sealed artifact does not need to retain as *live* state, only as *provenance*.

Sealing an ASIR graph is the exact moment it becomes eligible to be canonicalized into the typed knowledge layers `ATLAS-FORMAT.md#required-knowledge-layers` already requires inside a logical `.atlas`, and — once a `.atlasx` selects and closes it — the exact moment it becomes eligible to be read back out as compiler-input HIR per `COMPILER-IR-SCHEMAS.md`. **One vocabulary, three lifecycle positions: ASIR (open, pre-seal) -> sealed `.atlas` typed layer (closed, canonical) -> compiler HIR (closed, `atlasx_root_id`-anchored).** [CONTRACT — this reconciliation is locked; the actual Rust types implementing an in-memory mutable ASIR graph distinct from the current per-dimension `SemanticObservation` records are TARGET, not yet implemented; see "Relationship to current production code" below.]

This is why this contract does not define a competing `AtlasOp`/`AtlasRegion`/`AtlasBlock`/`AtlasValue`/`AtlasType` object model from scratch: that would be exactly the "parallel duplicate concept" `BLUEPRINT-EVOLUTION.md`'s discipline forbids inventing without evidence that the existing schema is inadequate. The MLIR/xDSL/IRDL donor census (`.atlas/census/donors/mlir.md`, `.atlas/census/donors/xdsl.md`) is real prior-art evidence for *how* a typed op/region/block/value model with dialect extensibility is usually built, but the concrete vocabulary Atlas needs already exists in `COMPILER-IR-SCHEMAS.md`.

## Dialect-qualified naming over the closed v1 enum

`COMPILER-IR-SCHEMAS.md`'s node-kind enum is closed and versioned (`"A construct outside this set requires a versioned schema extension or explicit unsupported compiler diagnostic."`). That is correct for compiler-input HIR: the compiler backend only needs to handle a bounded set of shapes.

Construction-time proposals are a different situation: a provider or a donor-informed extension author needs a namespace to propose *new* candidate operation kinds (e.g. a donor-informed `atlas.component.import` shape, evidenced by the WASM Component Model census) without waiting for a full HIR schema revision to experiment. ACP therefore uses **dialect-qualified opcode names** — `<dialect>.<opcode>`, e.g. `atlas.cf.cond_branch`, `atlas.effect.fs.read`, `atlas.state.write`, `atlas.security.require` — as the wire-level operation identifier, with an explicit, machine-checked mapping from dialect-qualified names onto the closed HIR node-kind enum for anything that is meant to become canonical HIR:

```text
atlas.cf.branch          -> HIR node kind not yet in v1 (unconditional branch; today's CFG
                             materialization, R4.6, represents this as a ControlFlowEdgeKind::
                             Fallthrough/Branch edge, not a HIR node -- HIR does not yet exist
                             in production; this is a TARGET mapping)
atlas.cf.cond_branch      -> HIR IF
atlas.cf.switch           -> HIR MATCH
atlas.cf.loop             -> HIR LOOP
atlas.cf.return           -> HIR RETURN
atlas.cf.panic            -> HIR FAIL
atlas.call.static         -> HIR CALL
atlas.call.dynamic        -> HIR DYNAMIC_CALL
atlas.state.read          -> HIR STATE_READ
atlas.state.write         -> HIR STATE_WRITE
atlas.state.transition    -> HIR STATE_TRANSITION
atlas.effect.*            -> HIR EFFECT (category carried as an attribute, mirroring
                             `EffectCategory` in core/src/semantic/effect.rs today)
atlas.cap.acquire         -> HIR RESOURCE_ACQUIRE
atlas.cap.release         -> HIR RESOURCE_RELEASE
```

Namespaces reserved by this contract (candidate set; extending this list is additive and does not require a breaking revision):

```text
atlas.core.*          structural/module/symbol operations
atlas.type.*          type construction/reference
atlas.cf.*            control flow
atlas.call.*          call sites
atlas.state.*         state access (mirrors R4.8 StateAccessKind)
atlas.effect.*        observable effects (mirrors R4.8 EffectCategory)
atlas.cap.*           capability/resource acquisition and release
atlas.security.*      security boundaries: require, deny, boundary, taint, sanitize
atlas.obligation.*    obligation attachment (mirrors SemanticObligationRecord)
atlas.provenance.*    provenance/evidence attachment (mirrors ProviderReceipt/evidence lineage)
atlas.decision.*      Jev-class decision operations: alternative, constraint, evidence,
                       propose, select, reject (mirrors DecisionProposal, see below)
atlas.genome.*        Genome-affecting operations (highest authority bar; see
                       EXTERNAL-PROVIDER-TRUST.md's canonical-write prohibition)
atlas.interface.*      component/interface boundary (WIT-informed, see
                       .atlas/census/donors/wasm-component-model.md)
atlas.component.*     component composition
atlas.ffi.*            foreign-function/external-boundary operations
```

Numeric opcode encoding is explicitly deferred: `.atlas`'s physical binary wire format already interns strings/symbols (`ATLAS-BINARY-WIRE-FORMAT.md`), so a dialect-qualified name can be encoded as an interned symbol reference without needing a separately-assigned numeric opcode space at this stage. Locking numeric opcode ranges before the dialect set has evidence from real construction traffic would risk exactly the "arbitrary opcode numbers" trap this contract is instructed to avoid. [CONTRACT — naming locked; numeric encoding explicitly TARGET/deferred.]

## Relationship to the multi-AI construction fabric

`MULTI-AI-CONSTRUCTION-FABRIC.md` owns orchestration: task decomposition, provider routing, bounded worker leases, context slicing, candidate branching and Jev-class decision flow.

This contract owns the typed construction boundary once a provider proposes semantic work.

~~~text
ConstructionTaskGraph / ProviderRouter
        ↓
provider proposes typed operation(s)
        ↓
ACP transport
        ↓
AtlasConstructionOperation decode/validation
        ↓
ASIR candidate state
~~~

A ProviderRouter decision MUST NOT leak provider-specific envelopes into ASIR identity. The same semantic operation proposed by different admitted providers must decode against the same typed construction vocabulary.

Agent-to-agent chat is not ACP and is not ASIR. If one provider consumes another provider's output, Atlas should pass the typed record/evidence reference rather than make an informal transcript the semantic handoff.

## ACP — Atlas Construction Protocol

ACP is the provider/wire protocol. It is transport, not semantics.

### Transport (NOW-compatible shape, CONTRACT status)

ACP operations are schema-constrained JSON objects (or an equivalent typed tool-call payload for providers that support native tool calling), each identifying a dialect-qualified opcode plus typed operands/results/attributes:

```json
{
  "op": "atlas.cf.cond_branch",
  "operands": [{"kind": "value_ref", "id": "v17"}],
  "attributes": {"true_target": {"block": "b3"}, "false_target": {"block": "b4"}},
  "effects": [],
  "required_capabilities": [],
  "evidence": ["evidence:..."]
}
```

**JSON is transport only.** At the Atlas boundary an ACP operation MUST decode into a typed `AtlasConstructionOperation` record (schema: `.atlas/schemas/atlas-construction-operation.schema.json`, added by this contract) before anything downstream of the transport layer ever inspects it. Provider-specific quirks (a particular model's preferred field ordering, a particular tool-calling framework's envelope shape) MUST NOT leak past this decode step. This mirrors, at the operation level, the same discipline `ATLAS-DEVELOPMENT-LANGUAGE.md`/`ADL-TO-ATLAS.md` already apply at the whole-artifact level: natural-language/provider-specific surface area does not become canonical semantics merely by existing.

### The typed construction-operation envelope

`.atlas/schemas/atlas-construction-operation.schema.json` (added by this contract) defines the typed shape every ACP operation MUST decode into:

- `op_id` — stable identity of this specific proposed operation (not yet a canonical `SemanticRecordId`; that identity is assigned only on admission, per `.atlas/contracts/UNIVERSAL-GRAPH-CONTRACT.md`'s identity discipline);
- `dialect` / `opcode` — the dialect-qualified name;
- `operands[]` / `results[]` / `result_types[]` — typed value references, not raw provider prose;
- `regions[]` / `successors[]` — nested control structure where applicable (mirrors `ControlFlowEdge`/HIR body-node child references);
- `attributes{}` — typed, schema-validated key/value data (never a place to smuggle free-form unvalidated content past the boundary);
- `effects[]` — every effectful operation MUST enumerate its own effect set here; this is checked, not merely declared (see "Admission pipeline" below);
- `required_capabilities[]` — every privileged effect MUST bind to an explicit capability/authority reference here, never an implicit ambient permission;
- `preconditions[]` / `postconditions[]` / `invariants[]` — carried through to the sealed artifact's typed constraint/invariant layer (`ATLAS-FORMAT.md#required-knowledge-layers`), never dropped silently;
- `obligations[]` — carried through to `SemanticObligationRecord` lineage;
- `evidence[]` / `provenance[]` — carried through to `ProviderReceipt`/evidence lineage; an operation with no evidence chain to a `ProviderReceipt` is not admissible.

This schema is intentionally close to the `AtlasOp` shape sketched in the architectural discussion that motivated this contract, but its actual field set is the mechanical union of what `COMPILER-IR-SCHEMAS.md`'s HIR node fields already require plus what `EXTERNAL-PROVIDER-TRUST.md`'s admission rules already require (effects, capabilities, evidence) — it is a transport-and-proposal envelope around the existing typed vocabulary, not a new vocabulary. [CONTRACT, schema added, no Rust implementation yet — TARGET for the decode/admission code path.]

### Semantic transactions, not whole-program regeneration

A provider proposal is a **batch** of construction operations against a declared base, with explicit expectations, not a single free-standing operation and not a whole-program rewrite:

```text
txn {
    base = semantic_hash:<content hash of the ASIR state this txn was proposed against>
    ops  = [create_symbol, create_block, add_operation, attach_effect,
            attach_capability, attach_evidence, ...]
    expect = [no_unresolved_symbols, no_unbound_effects, ...]
}
```

This is deliberately the same shape `.atlas/schemas/candidate-change-set.schema.json` already has at the file/module granularity (`parent_repository_revision` = base; `file_changes`/`semantic_changes` = ops; `unknowns`/`tests` = expectations). **This contract does not introduce a second transaction envelope.** A construction transaction is a `CandidateChangeSet` whose `semantic_changes[]` entries carry typed `AtlasConstructionOperation` references instead of (or in addition to) today's free-text `summary` field.

This contract adds exactly one additive, backward-compatible field to `candidate-change-set.schema.json`: `semantic_changes[].construction_ops_refs` (array of strings, optional, references into `atlas-construction-operation.schema.json` records). Existing `CandidateChangeSet` consumers that only read `kind`/`summary` are unaffected; nothing required becomes optional and nothing optional becomes required. [CONTRACT, schema patched; the code path that materializes/validates `construction_ops_refs` is TARGET.]

### Admission pipeline (CONTRACT, TARGET implementation)

```text
provider proposal (ACP wire payload)
    |  decode into typed AtlasConstructionOperation -- transport-specific shape discarded
    v
schema validation      -- atlas-construction-operation.schema.json
    |
type validation        -- operand/result types check against ASIR's current type table
    |
semantic validation     -- op is well-formed against the target dialect's own rules
    |                      (e.g. atlas.cf.cond_branch's true/false targets must exist)
architectural validation -- evaluate the isolated post-transaction candidate against the
    |                        pinned ArchitecturalIntegrityEnvelope and affected impact closure
    |                        under ARCHITECTURAL-INTEGRITY.md; HARD violations reject the txn
    |
security validation     -- effects/capabilities declared here match what static analysis
    |                      of any attached generated code actually exhibits (see
    |                      "Generated code is untrusted input" below)
obligation validation   -- every declared obligation has a resolvable owner/verification path
    |
selection authority     -- EXTERNAL-PROVIDER-TRUST.md's role/capability rules decide who
    |                      may admit this specific kind of change (a SYNTHESIS_PROVIDER's
    |                      own proposal never self-admits; see that contract's
    |                      "Canonical-write prohibition")
    v
apply transaction        -- ASIR state advances; only now does this construction's
                            content become eligible for the sealed .atlas
```

A proposal that fails any stage is rejected with a typed diagnostic, exactly as `.atlas/contracts/SEMANTIC-EXTRACTION.md`'s existing `ExtractionDiagnostic`/`DiagnosticCode` discipline already handles malformed source: explicit, never silent.

## Normative rules (CONTRACT, restating and sharpening EXTERNAL-PROVIDER-TRUST.md at the operation level)

- A provider MUST NOT author canonical `.atlas` bytes directly. `EXTERNAL-PROVIDER-TRUST.md`'s canonical-write prohibition already forbids this at the artifact level; this contract forbids it at the operation level too: no ACP payload field is ever "raw bytes to append to the sealed artifact."
- A provider MUST NOT establish canonical semantics through prose. A `CandidateChangeSet.semantic_changes[].summary` string remains allowed as a human-readable label, exactly as today, but it is never itself the semantic claim; the semantic claim is the typed `AtlasConstructionOperation` (once this contract's schema/code path exists) or the typed `SemanticObservation` (today, for extracted/existing source).
- Every ACP operation MUST be schema-valid against `atlas-construction-operation.schema.json` before any downstream stage inspects it.
- Every operation MUST be independently type-checkable where its dialect defines a type-checking rule.
- Every transaction that can affect architecture MUST be evaluated on an isolated post-transaction semantic state against the pinned ArchitecturalIntegrityEnvelope from `ARCHITECTURAL-INTEGRITY.md`; architecture is a transaction/global property, not merely an operation-local annotation.
- A HARD architectural violation rejects the entire transaction even if type checks, tests or benchmarks pass. Provider self-asserted equivalence is never sufficient evidence.
- Architectural impact closure MUST be derived from observed semantic relationships; an operation's declared `invariants[]` may contribute intent but cannot prove its own compliance.
- Every effectful operation MUST expose its own effect set in `effects[]` — never an effect discovered only by executing the operation.
- Every privileged effect MUST bind to an explicit capability/authority reference in `required_capabilities[]` — never an implicit ambient permission.
- Every imported/generated semantic claim MUST preserve provenance/evidence through to a `ProviderReceipt` — no operation is admissible with an evidence chain that terminates in nothing.
- No provider output becomes canonical merely because the provider emitted it. `EXTERNAL-PROVIDER-TRUST.md`'s role/capability rules already say this at the artifact level (`SYNTHESIS_PROVIDER`: "May not write canonical main ... directly"); this contract confirms the same holds per-operation.
- Only Atlas's own verification and authorized selection may admit a construction operation into canonical ASIR state. "Authorized" is exactly `EXTERNAL-PROVIDER-TRUST.md`'s existing role/authority model — this contract does not invent a second authority system.

## Generated code is a donor too

External-provider-generated source code MUST NOT go `provider -> canonical Atlas` directly, for the same reason a cloned OSS donor does not: both are untrusted candidate material admitted through the same standard, not two different standards. Concretely, generated code follows:

```text
provider -> CandidateChangeSet -> census (SEMANTIC-EXTRACTION.md's own pipeline,
            applied to the generated source exactly as it is applied to any other
            source) -> semantic extraction -> type/security/effect/obligation checks
            -> tests/falsification -> selection -> canonical ASIR
```

This is not a new rule; `.atlas/contracts/SEMANTIC-EXTRACTION.md`'s "OSS is untrusted input" section and `EXTERNAL-PROVIDER-TRUST.md`'s `SYNTHESIS_PROVIDER` role already establish it. This contract makes explicit that the *admission standard* for provider-generated code and the *admission standard* for a cloned donor's code are the same standard, applied to material from a different source — never a laxer path for "code we just asked an AI to write."

## Jev-class decision reasoning already has a home

Section-10-style "Jev" reasoning — enumerate alternatives, weigh constraints, use evidence, propose a structure — is not a new schema. It is `.atlas/schemas/decision-proposal.schema.json`, already locked, already used by `DECISION_PROVIDER` (`EXTERNAL-PROVIDER-TRUST.md`). The mapping is direct:

| Conceptual Jev field    | Existing `DecisionProposal` field                                   |
|--------------------------|-----------------------------------------------------------------------|
| alternatives              | `candidate_ids[]`                                                     |
| constraints                | `criteria[]` (`hard_constraint: bool`, optional `weight`)             |
| evidence                   | `evidence_refs[]`                                                     |
| proposed selection         | `selected_for_next_step_candidate_id`                                 |
| rejected alternatives + why| `ranking[]` entries not chosen, each carrying its own `rationale`     |
| authority gate              | `authority_requirement` (`HUMAN_REQUIRED` / `POLICY_AUTO` / `HYBRID`) |

No new decision schema is added by this contract. A construction-phase Jev decision is simply a `DecisionProposal` whose `candidate_ids` reference `AtlasConstructionOperation`/`CandidateChangeSet` alternatives instead of, say, donor-absorption alternatives — the schema is already generic over what a "candidate" is. `DecisionProposal.status` (`PROPOSED`/`USED_FOR_EXPLORATION`/`SUPERSEDED`/`REJECTED`) already prevents a proposal from silently acquiring final authority: only an explicit downstream admission step (governed by `EXTERNAL-PROVIDER-TRUST.md`'s authority model) turns a proposal's `selected_for_next_step_candidate_id` into anything canonical.

egglog's e-graph/equality-saturation/cost-extraction model (`.atlas/census/donors/egglog.md`) is real evidence for a *future* mechanism Jev could use internally to enumerate and rank structurally-equivalent alternatives before emitting a `DecisionProposal.ranking` — that is a TARGET research direction the census records, not a dependency Atlas adopts by this contract.

## Relationship to current production code (NOW)

As of this contract's writing, R4.3-R4.8 materialize typed `SemanticObservation` records (`SemanticDimension::{Symbol,Type,FunctionIdentity,FunctionSignature,Call,ControlFlow,DataFlow,State,Effect}`) directly from *existing* source via the `Inventory -> SourceFrontend -> SemanticExtractor -> Census -> Normalize -> Reconcile -> graph` path (`.atlas/decisions/0001-one-normalized-semantic-path.md`). That path has no ACP/ASIR construction-transaction step because it extracts semantics from source that already exists — there is no provider "constructing" anything, only an extractor observing.

ACP/ASIR become load-bearing the moment Atlas needs a provider to **propose new** semantic content (new functions, new modules, generated implementations for missing logic — `ATLAS-CREATION-PIPELINE.md`'s "candidate implementation synthesis" step) rather than only observing what already exists. That is future roadmap work; see `.atlas/roadmap/SELF-BUILDING-R4-R8.md`'s newly added construction-protocol roadmap items. Nothing in this contract changes, weakens, or bypasses the current extraction path, its multi-extractor evidence discipline, its obligation lineage, or its donor-quarantine rules.

## Canonicalization

An ASIR graph, once sealed, MUST canonicalize deterministically under `ATLAS-SEMANTIC-COMPACTION.md`'s existing rules: same semantic input + same Atlas semantic/compiler version => same canonical semantic representation. Construction-time artifacts (timestamps, transaction ordering, which of several equivalent provider phrasings produced an operation, donor file ordering) MUST NOT leak into the canonical result. This is not a new invariant; it is `ATLAS-SEMANTIC-COMPACTION.md`'s existing determinism target, restated here because ACP/ASIR is the layer where such nondeterminism would first be introduced if left unchecked. Current production code does not yet implement or test end-to-end determinism at this layer — TARGET, not NOW.

## Blueprint evolution

This contract's dialect-namespace list, ACP transport shape, and admission-pipeline stage ordering are the current best evidence-backed design, not permanently frozen. A revision follows `BLUEPRINT-EVOLUTION.md`: discovery, actual-provider attribution, deep census, current-vs-candidate-vs-alternatives comparison, invariant analysis, falsifiable evidence, `BlueprintRevisionDecision`, then implementation. No revision may silently violate `COMPILER-IR-SCHEMAS.md`'s closed HIR vocabulary, `EXTERNAL-PROVIDER-TRUST.md`'s authority model, or `.atlas/contracts/UNIVERSAL-GRAPH-CONTRACT.md`'s identity discipline without explicitly revising those contracts first.
