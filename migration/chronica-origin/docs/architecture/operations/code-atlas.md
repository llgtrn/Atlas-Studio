# Code Atlas and Native Absorption Compiler

Status: **ACTIVE EXECUTABLE ARCHITECTURE**

Owner: System Atlas / refoundation tooling.

Code Atlas is the semantic engineering projection that turns source evidence into bounded Chronica work. It does not become a second truth system and it does not authorize repository mutation.

## Contract

```chronica-contract
{
  "id": "INV-SYSTEM-ATLAS-CODE-ABSORPTION",
  "owner": "operations",
  "kind": "engineering_control",
  "severity": "HARD",
  "runtime_owner": [
    "tools/system-atlas/code-atlas.mjs",
    "tools/system-atlas/code-atlas-backends.mjs",
    "tools/system-atlas/code-atlas-structural.mjs",
    "tools/system-atlas/code-property-graph.mjs",
    "tools/system-atlas/code-atlas-analyze.mjs",
    "tools/system-atlas/code-atlas-query.mjs",
    "tools/system-atlas/code-atlas-isolation.mjs",
    "tools/system-atlas/code-atlas-work-packet.mjs",
    "tools/system-atlas/fleet/fleet-plan.mjs"
  ],
  "verification": [
    "tools/system-atlas/code-atlas.test.mjs",
    "tools/system-atlas/code-atlas-wave2.test.mjs",
    "tools/system-atlas/code-atlas-work-packet.test.mjs",
    "tools/system-atlas/dev-cell-audit.test.mjs"
  ]
}
```

## Pipeline

~~~text
tracked source
  ↓
Code Atlas frontend
  ↓
files / languages
  ↓
symbols / imports
  ↓
call candidates
  ↓
effect + state-mutation candidates
  ↓
BehaviorSlice
  ↓
AbsorptionWorkPacket
  ↓
CHRONICA_BUILD
  ↓
component CI + parity/proof
  ↓
canonical merge
~~~

Every inferred semantic item carries source evidence. A parser finding is an observation/analysis artifact, not canonical World state.

## Donor technology map

~~~text
D346 Tree-sitter    -> multi-language parsing mechanics
D347 ast-grep       -> structural matching/rewrite mechanics
D348 Joern          -> semantic/code-property graph mechanics
D349 C2Rust         -> C-to-Rust translation candidate mechanics
D350 py2many        -> Python-to-Rust translation candidate mechanics
D351 rust-analyzer  -> Rust semantic-analysis mechanics
D352 Miri           -> undefined-behavior proof mechanics
D353 Kani           -> bounded Rust model-checking mechanics
~~~

The terminal architecture is Chronica-native. Donor binaries/libraries are permitted as bounded bootstrap/reference mechanisms only while their native replacement remains incomplete.

## Honesty states

The first executable frontend is intentionally reported as:

~~~text
parser_backend = NATIVE_HEURISTIC
~~~

This is not equivalent to a concrete syntax tree or full semantic graph.

Tree-sitter/ast-grep/Joern readiness is reported separately. A missing donor source/tool remains GAP.

## BehaviorSlice

A BehaviorSlice is an ANALYZE candidate derived from one source symbol and linked evidence. It can include:

~~~text
symbol
calls
external effects
state mutations
risk tags
confidence
source ranges
~~~

It never grants authority and never claims full semantic equivalence.

## Work packet

A valid work packet hard-codes:

~~~text
status = ANALYZE_ONLY
target.repo = llgtrn/Chronica
production_code_in_dev_repo = false
donor_runtime_dependency_allowed = false
canonical_truth_may_be_inferred_from_source = false
~~~

Backend target responsibilities use Rust. APP projection work uses TypeScript/TSX.

## Generation-2 Dev Cells

For donor `Dxxx`, Code Atlas evidence is stored as:

~~~text
evidence/donors/Dxxx.code-atlas.json
~~~

and the bounded human/machine census as:

~~~text
evidence/donors/Dxxx.census.json
~~~

Both are required before the donor census becomes complete.

Product source, patches, and implementation remain forbidden in Dev repositories.

## RECORD / ANALYZE / ACT

~~~text
RECORD
  tracked source revision
  donor provenance
  CI/proof evidence

ANALYZE
  Code Atlas graph
  BehaviorSlice
  work packet
  transpilation
  AI rewrite

ACT
  repository mutation
  candidate integration
  canonical merge
~~~

ANALYZE never upgrades itself into ACT.


## Wave 2 — developer semantic graph

Wave 2 keeps Atlas explicitly outside the Chronica runtime.

~~~text
tools/system-atlas/
  code-atlas-backends.yaml
  code-atlas-backends.mjs
  code-atlas-structural-rules.yaml
  code-atlas-structural.mjs
  code-property-graph.mjs
  code-atlas-analyze.mjs
  code-atlas-query.mjs
~~~

These modules are development tooling. `core/`, `runtime/`, `adapter/`, and `organism/` must not import them.

Generated Wave-2 shards are:

~~~text
code-backends.json
code-structural-findings.json
code-property-graph.json
~~~

The Code Property Graph is a rebuildable ANALYZE projection, not a canonical world graph.

For one donor source tree, the standard dev command concept is:

~~~text
system-atlas:code:analyze
  donor source
  -> Dxxx.code-atlas.json
  -> Dxxx.structural.json
  -> Dxxx.code-property-graph.json
~~~

A developer/agent can then query the evidence by symbol, path, risk, effect, structural rule, or BehaviorSlice before producing a work packet.

Dependency closure is bounded by both depth and node limit. If the closure is truncated or unresolved calls remain, the work packet must expose that uncertainty rather than silently widen scope.

### Dev-tool isolation invariant

~~~text
Chronica runtime
  -X-> Code Atlas tool backend

Code Atlas tooling
  -> reads repository/donor source
  -> emits ANALYZE/evidence projections
  -> informs bounded engineering work
~~~

Tree-sitter, ast-grep, Joern, rust-analyzer, Miri and Kani may support development/proof workflows. Their availability never becomes a production runtime requirement.


### Runtime isolation proof

Runtime isolation is mechanically measured by `tools/system-atlas/code-atlas-isolation.mjs`.

It scans production responsibility roots:

~~~text
core/
runtime/
adapter/
organism/
apps/ui/
~~~

for direct Code Atlas/System Atlas dependency references. Any such dependency is HARD because Atlas is development tooling, not product infrastructure.

The generated shard is:

~~~text
code-runtime-isolation.json
~~~

and canonical census exposes both scanned-file count and hard-violation count.


### Deterministic generation versus host probing

Canonical generation never probes the developer host for optional binaries.

~~~text
atlas:generate
  -> repository-declared backend registry
  -> deterministic for the same repository revision

system-atlas:code:backends
  -> explicit developer host probe
  -> may report Tree-sitter / ast-grep / Joern / C2Rust / py2many /
     rust-analyzer / Miri / Kani availability
~~~

Host capability is transient development context. It is not canonical repository truth and is not included as a runtime dependency requirement.
