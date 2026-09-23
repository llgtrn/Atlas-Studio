# Atlas Studio

Atlas Studio is a product-neutral engineering-world compiler and system invention environment.

Digital Organism is a first-class Atlas target, but Atlas remains independent of any generated organism or external runtime. An organism may use external LLM/model APIs, self-hosted models, deterministic cognition or hybrid bindings; weights are optional organs, not the organism itself.

It performs strict, evidence-linked census from multi-repository scope down to every discovered function and required semantic atom; reconciles gaps/conflicts to a fixed point; synthesizes and validates target-native designs; publishes them as a dense logical `*.atlas`; materializes selected executable worlds as `*.atlasx/`; and compiles them into verified target products.

```text
Reality / repositories / donors / research
        ↓
strict census + CensusCertificate
        ↓
observed/research/invention graph
        ↓
SEALED logical *.atlas
        ↓
content-addressed shards when needed
        ↓
deterministic *.atlasx/
        ↓
world/semantic optimization
        ↓
HIR → MIR → LIR → Machine IR
        ↓
codegen → LTO → link → post-link
        ↓
binary / library / WASM / UI product
        ↓
runtime profiling / PGO / auto-tuning
        ↺
evidence + recensus
```

## Execution model — subsystem first, IDE later

Atlas Core is designed to run as a tool/runtime subsystem inside the environment where engineering is already happening.

A first-class target is an embedded coding-agent session:

~~~text
Claude Code / another coding agent
        ↓ local MCP / API adapter
Atlas Core in the same outer sandbox
        ↓
candidate workspaces + Census + verification + admission
        ↓
SEALED logical Atlas → canonical *.atlas
~~~

The cloud coding session may supply compute, checkout and an outer sandbox. Atlas still owns semantic identity, uncertainty/closure, candidate verification and admission. A mutable agent worktree is candidate state until admitted.

MCP is an agent-facing adapter, not Atlas's canonical semantic protocol. The same Atlas Core is intended to serve CLI, CI/API integrations and a future Atlas Studio IDE without creating parallel truth systems.

Ephemeral agent context is not project memory. Sealed Atlas artifacts and their evidence/lineage are the durable machine-readable engineering state that a later session can load and incrementally recensus.

See .atlas/contracts/AGENT-HOST-EMBEDDED-RUNTIME.md.

## Core invariants

- `.atlas/` is the repository control root; `*.atlas` is a dense binary engineering artifact.
- Every admitted artifact and discovered function is accounted for. UNKNOWN is permitted only explicitly; silent omission is forbidden.
- Universal graph primitives include identity, scope, node, edge, binding, state, event, temporal, evidence, provenance, constraint/invariant, interface/capability, effect and materialization.
- Independently generated repositories remain sovereign but cross-repository composable through stable identities/bindings.
- Logical Atlas may be physically sharded across locations without becoming multiple truth systems.
- Completeness outranks file size; semantic interning/dedup/deltas/compression reduce duplication rather than delete meaning.
- AtlasX is selected executable representation, not prose.
- Compiler optimization begins at world/graph level and continues through machine code, LTO, post-link, PGO and evidence-backed auto-tuning.
- Backend/compiler/runtime bootstrap is Rust. Frontend bootstrap is TypeScript/TSX. C is an explicit low-level boundary by default.
- Donor OSS is evidence/reference and is removed from the temporary workbench after native absorption proof.

The current compatibility binary remains `atlas-systemizer` and API `atlas.systemizer.cli.v1` while the native Atlas compiler/runtime is built.
