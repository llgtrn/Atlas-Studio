# Atlas Studio



**Architecture reference:** Read the [full Blueprint](https://github.com/llgtrn/.Ynventa-/blob/main/BLUEPRINT-V1.md) for the complete design under explicit owner instructions and applicable accepted decisions. The [nomenclature architecture atlas](https://github.com/llgtrn/.Ynventa-/blob/main/NOMENCLATURE-ARCHITECTURE-ATLAS.md) is derived navigation for currently registered names and bindings, not the full architecture or a ceiling on its requirements. Proposals retain their recorded status; a design requirement alone neither registers an identity nor proves implementation.

## Canonical Ynventa/Chronica integration — registered Latin nomenclature

**One Digital Holobiont.** [Chronica's nomenclature registry](https://github.com/llgtrn/Chronica/blob/main/.ynventa/declared/nomenclature/concepts.jsonl) and [frozen-form policy](https://github.com/llgtrn/Chronica/blob/main/.ynventa/declared/nomenclature/policy.jsonl) provide admitted stable names, identities and current declared bindings (DECISION 0115); their current entry count does not limit the Blueprint's design scope or prove executable bindings. **Norl is cognition hosted within Chronica.** Chronica owns canonical identity, history, authority and governed execution. Generic biological words explain functions without creating competing identities, implementation homes or permission.

**This checkout's contribution:** This repository is temporary source/donor material. **Codara** is the canonical semantic-language concept and implementation under Chronica, not a promise that every compiler source here was absorbed. `apps/studio` contains historical placeholder metadata and design tokens, not an executable frontend or route-parity oracle. Preserve that evidence and unique source semantics; all new product UI is owned exclusively by `Chronica/phenotype/`.

**ONE body, MANY Phenora, ONE product-UI code owner.** All frontends, dashboards, charts, studios and operator UI eventually reside under `Chronica/phenotype/`; native algorithms and typed contracts can remain specialized capability implementations until converged. One common physical source grammar applies to all eight repositories including ECDEV; current old source roots are **migration inputs** and have no permanent architectural exemption. A README is not evidence that code migration, protocol relock, user-interface cutover or V1 completion has occurred. [Architecture](https://github.com/llgtrn/.Ynventa-/blob/main/BLUEPRINT-V1.md) · [Morphology](https://github.com/llgtrn/.Ynventa-/blob/main/REPOSITORY-PROTOCOL-V1.md) · [Concept vocabulary](https://github.com/llgtrn/.Ynventa-/blob/main/SEMANTIC-MODEL-V1.md).

```mermaid
flowchart LR
    subgraph SOURCE["Atlas-Studio: current development and migration input"]
      A["Source compiler / design evidence"] --> B["Codara parity candidates"]
    end
    B -. "PROPOSED migration; runtime join NOT YET PROVEN" .-> V{"Independent identity, behavior, authority and consumer proof"}
    V -. "candidate only; evidence grants no authority" .-> C["One Chronica world: verified target binding"]
    subgraph CANON["Required governed relationships; these dashed edges are not source integration calls"]
      C -. "typed WorkRequest and current authority" .-> P["Thymora / Porion: pre-effect admission"]
      P -. "sealed plan and bounds" .-> M["Metabion: bound execution"]
      M -. "success, failure or unknown effect" .-> L["Lineara: evidence, reconciliation and canonicalization"]
      L -. "accepted canonical event" .-> H["One history; replay-derived World"]
      H -. "derived permitted projection" .-> F["Studio / compiler Phenora; shared UI at Chronica/phenotype"]
    end
```



## Source execution and convergence — Codara, not an independent Atlas authority

The ADL source compiler resides at `core/src/codara`, exposed as `atlas_core::codara::adl`
within the existing `core` package. Its parser, compiler and census bridge retain their
algorithms and wire contracts. Active Rust callers use this namespace without a `language`
alias. This bounded source naming slice establishes no Chronica grammar parity, native
absorption or runtime admission.

| Historical source locator | Current source locator |
|---|---|
| `core/src/language/mod.rs` | `core/src/codara/mod.rs` |
| `core/src/language/adl/mod.rs` | `core/src/codara/adl/mod.rs` |
| `core/src/language/adl/census.rs` | `core/src/codara/adl/census.rs` |
| `core/src/language/adl/census/tests.rs` | `core/src/codara/adl/census/tests.rs` |

The four files moved byte-for-byte with their saved-workspace ArtifactIds. Historical
observations keep their original IDs and locators; `atlas.adl.*` schemas, source-span data,
the census derivation ID and `tools/atlas/declared/census.adl` remain unchanged.

**The existing repository is a temporary development/source checkout.** Codara is the named canonical semantic-intent capability in Chronica, with real implementation under `physiology/capability/codara` and a governed `phenotype/codara` CLI. Similar historical names are not proof that this checkout's compiler code has been absorbed. Retain source identity, candidate artifacts, oracles, licenses and performance data, but do not describe this repository as owning semantic truth, permission, canonical admission, or a permanent independent coding environment.

A coding-agent session can call a locally bounded MCP/API adapter while preparing **candidate artifacts**. Those source artifacts and tests remain scoped and non-authoritative until Chronica graph/capability verification, Thymora/Porion admission and Lineara evidence accept the specified behavior. Remote providers, including coding agents, may make proposals; neither consensus nor an independent local `Atlas Core` can authorize changes to the body.

The historical object codec and pure root-manifest codec now live together under
`core/src/codara/artifact` inside the same `atlas_core` crate. The 25 existing files moved
byte-for-byte before caller updates, preserving their saved-workspace ArtifactIds. Their
source relocation is explicit:

| Historical source locator | Current source locator |
|---|---|
| `core/src/atlasx/codec.rs` | `core/src/codara/artifact/codec.rs` |
| `core/src/atlasx/codec/**` | `core/src/codara/artifact/codec/**` |
| `core/src/atlasx/manifest.rs` | `core/src/codara/artifact/manifest.rs` |
| `core/src/atlasx/manifest/tests.rs` | `core/src/codara/artifact/manifest/tests.rs` |

Current Rust callers use `codara::artifact` directly. The nine sealed-parent prerequisite,
materialization and root-validation files now form `codara::artifact::staging`, with the same
saved-workspace ArtifactIds and the following explicit source relocation:

| Historical source locator | Current source locator |
|---|---|
| `core/src/atlasx/mod.rs` | `core/src/codara/artifact/staging/mod.rs` |
| `core/src/atlasx/precondition.rs` | `core/src/codara/artifact/staging/precondition.rs` |
| `core/src/atlasx/precondition/tests.rs` | `core/src/codara/artifact/staging/precondition/tests.rs` |
| `core/src/atlasx/materialize.rs` | `core/src/codara/artifact/staging/materialize.rs` |
| `core/src/atlasx/materialize/tests.rs` | `core/src/codara/artifact/staging/materialize/tests.rs` |
| `core/src/atlasx/materialize/census_records.json` | `core/src/codara/artifact/staging/materialize/census_records.json` |
| `core/src/atlasx/validate.rs` | `core/src/codara/artifact/staging/validate.rs` |
| `core/src/atlasx/validate/tests.rs` | `core/src/codara/artifact/staging/validate/tests.rs` |
| `core/src/atlasx/fixture.rs` | `core/src/codara/artifact/staging/fixture.rs` |

The existing source gate still checks the declared seal policy, pinned envelope, signed design,
parent binding and report consistency before materialization. Its ADMITTED result qualifies
that historical source projection; it supplies no Chronica requester, resource, lease or work
authority. The persisted materializer identity `atlas_core::atlasx::materialize` remains an
opaque historical identity, independent of the current Rust module path. The file adapter
`runtime::atlasx` and historical `atlasx` CLI commands retain their existing format interface.
Historical `.atlasx` formats, schema names, encoded IDs, object bytes and pinned provenance stay
historical. This source naming step establishes no native Codara format, consumer cutover or
source extinction. Existing declared ownership and proof selectors still address the same core.

The following command is a legacy AtlasX source-reader/export interface. It does not write
Codara's canonical artifact format. New Codara programs in the
`core/src/codara/artifact/codec/parity/` corpus use `.cdr`; its `.atlasx` objects are explicitly legacy
inputs and the exported `.payload` file is a raw byte oracle for the existing digest flow.
No native Codara FUNCTIONS object format or semantic conversion is established by this export.

`atlas-systemizer atlasx codec --codara-payload OBJECT --out FILE` exports a codec-validated,
address-named FUNCTIONS object's raw payload for Chronica's existing
`digest.blake3-256-verify` kernel. The output parent must exist; identical replays succeed and
conflicting files are preserved. Refused objects write nothing. Standard output is the existing
codec report: its `address` is the program's `claimed` fact, and `decoded_length` is its
`input_bytes` fact. The digest covers bytes after the 80-byte header, so the exported file is
fed unchanged to `codara run --payload FILE CAPSULE`; the complete `.atlasx` file is retained as source
evidence. `core/src/codara/artifact/codec/fixture_verify.codara` is a reusable source fixture for the
committed codec object (385 payload bytes), with target-world aliases `agent` and `cpu-1`.
Its one-step budget is one `cpu_units`, matching the existing verifier's declared demand;
the target still checks resource capacity and its current envelope.
The operator must supply the target world's actual declarations, binding, current authority,
authentication and admission, and build a capsule there. This export creates no capsule or grant.
Digest comparison verifies content identity only; it does not import, execute or establish
semantic parity of AtlasX records. Historical formats and IDs remain unchanged.

**PROPOSED convergence interaction, not an implemented source-to-Chronica call trace.** The sequence expresses required review/authority/evidence obligations; it does not establish automatic central-engine admission or actual consumer cutover.

```mermaid
sequenceDiagram
    participant A as Coding agent or external provider
    participant S as Atlas-Studio frozen donor/source checkout
    participant E as Shared .ynventa Work and evidence engine
    participant C as Chronica Codara and Ontogen
    participant R as Chronica Thymora / Porion
    participant X as Bounded Metabion execution
    participant L as Lineara provenance and replay
    A->>S: Candidate code and typed intent with scoped provenance
    S->>E: Pinned source revision, tests, license and oracles
    E->>E: Compare identity, parity and existing native implementations
    alt Unverified or duplicate without benefit
       E-->>S: BLOCKED, no second language authority
    else Unique proven semantic technology
       E->>C: Proposed native capability and candidate capsule
       C->>R: Request admission with complete evidence and resource scope
       alt Denied, unresolved or untrusted
          R-->>E: Refuse, preserve history without effect
       else Authorized
          R->>X: Admit a bounded attempt after required proof and caller checks
          X->>L: Observed result and scoped evidence, then owner reconciliation
          L-->>S: Verified outcome and consumer-cutover obligations
       end
    end
```

## Agent construction and provider routing

Multi-agent construction is an **orchestration implementation concern**, not a new species, a second canonical graph, or an autonomous programming authority. Coding sessions receive bounded Work/Lease/Result contracts, pinned read/write sets and narrowly scoped semantic context. Any former in-checkout task graph, blackboard or provider router must be **classified as source technology**, compared with the central evolution machinery and absorbed only when independently useful.

**PROPOSED construction relationships, not a current integrated runtime.** Actual source-local agent tooling and central Work mechanisms must be verified separately.

```mermaid
flowchart LR
    NEED["Typed change intent and graph affected closure"] --> WORK["Central Ynventa Work with exclusive source write scope"]
    WORK --> AI["Host-native / remote / local agents as bounded proposal generators"]
    AI --> CAND["Candidate implementations with exact source and tool provenance"]
    CAND --> CHECK{"Identity, semantics, consumer and safety proof?"}
    CHECK -->|"no"| REFUSE["Reject or scoped repair; never bypass admission"]
    CHECK -->|"yes"| COD["Chronica Codara and Ontogen capability binding"]
    COD --> POR["Thymora grants and Porion effect admission"]
    POR --> LINE["Lineara evidence, history and feedback"]
```

The historical `tools/atlas/` and `*.atlas` filenames are **existing source/evidence locators**, not canonical subsystem branding or a second approved control plane. Only machine-readable identifiers explicitly frozen in the registry policy are preserved as data; canonical architecture and UI ownership are Chronica-wide. The Studio at `apps/studio` remains migration source until its unique UI functionality moves into Chronica Phenora, verified consumers switch and its independent bundle is retired.


## Core invariants

- `.ynventa/` is the only knowledge authority (typed facts, never Markdown; `ynventa context`, `ynventa knowledge view`). It is this shard's protocol projection only (`protocol.lock`, `declared/`, `knowledge/`): the Ynventa engine is central (the sibling checkout `.Ynventa-`) and interprets the shard through `protocol.lock`; no engine source is ever added under `.ynventa/`. Run it from the shard root as `cargo run -q --manifest-path ../.Ynventa-/Cargo.toml --bin ynventa -- <cmd> --root .` (or the built `../.Ynventa-/target/release/ynventa <cmd> --root .`); `tools/atlas/` is a source-code control/evidence locator from the previous implementation (not an authority or an approved destination root); `*.atlas` is a dense binary engineering artifact. The `.atlas/` directory is retired (ADR 0104, a DECISION fact).
- Every admitted artifact and discovered function is accounted for. UNKNOWN is permitted only explicitly; silent omission is forbidden.
- Universal graph primitives include identity, scope, node, edge, binding, state, event, temporal, evidence, provenance, constraint/invariant, interface/capability, effect and materialization.
- Source repositories remain temporary development checkouts; Chronica owns the single sovereign semantic graph and authorization.
- Physical source checkouts may be distributed; their code and evidence converge into Chronica's **one** semantic and authority universe.
- Completeness outranks file size; semantic interning/dedup/deltas/compression reduce duplication rather than delete meaning.
- Source compiler artifacts can provide candidate executable representations; their semantics and native ownership require Codara parity proof, real callers and authorized Chronica admission.
- Optimization may use graph-level semantics and native compiler techniques such as LTO or PGO only when the specific implementation, production caller and reproducible benchmark establish a benefit; no unimplemented optimizer is implied.
- Backend/compiler/runtime bootstrap is Rust. Canonical frontend bootstrap is TypeScript/TSX inside Chronica/phenotype ONLY. C is an explicit low-level boundary by default.
- Donor OSS is evidence/reference and is removed from the temporary workbench after native absorption proof.

The physical source binary identifiers `atlas-systemizer` and `atlas.systemizer.cli.v1` remain source-code locators for comparison and must not be described as current canonical subsystem names.
