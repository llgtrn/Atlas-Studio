---
id: atlas.decision.0102.the-chronica-repository-grammar-places-runtime-in-the-adapter-plane
type: decision
status: accepted
canonical: true
---
# ADR 0102 — The Chronica repository grammar governs physical layout; Runtime is an adapter-plane node at `adapter/runtime`

## Status

Accepted. **SUPERSEDES** the local four-root physical layout rule — "production architecture converges on four owners: `core/`, `runtime/`, `adapter/`, `apps/`" — as stated in `architecture/SYSTEM.md` (§Responsibilities, §Native capability ownership), `architecture/README.md`, `architecture/responsibilities.md`, `blueprints/PHYSICAL-REFOUNDATION.md` (§Purpose and its completion criterion "canonical behavior lives under `core/runtime/adapter/apps`") and `tools/README.md`. Those documents are kept; where they name `runtime/` as a root directory they now point here. The crate names (`core`, `runtime`, `adapter`, `atlas-cli`) and the logical owner vocabulary (`runtime/census`, `runtime/query`, ... in roadmap and genome records) are not changed by this decision.

## Context

Atlas Studio is one physical shard (`atlas-studio`) of the logical system Chronica. Its `.ynventa/` subsystem (protocol v1, byte-identical across every Chronica shard) declares one canonical repository grammar for all shards: the roles, their directories and the plane order in `.ynventa/src/repository/mod.rs` (`ROLES`, `ROOT_FILES`, `FORBIDDEN_SEGMENTS`). The owner decided that this global grammar overrides older local layout decisions.

Atlas's declaration (`.ynventa/declared/repository.rs`) classified the `runtime` crate (node `compiler.runtime`, `ynv://chronica/compiler/runtime`, `yn1:b085d64930bd4d4df465ff2f1967003f`) as SUBSTRATE with canonical path `substrate/runtime`. `ynventa verify` reported:

- `PLANE_VIOLATION compiler.runtime`: a SUBSTRATE node depends on the ADAPTER node `compiler.adapter`; SUBSTRATE may depend only on KERNEL and SUBSTRATE.
- `LEGACY_PLACEMENT compiler.runtime` and `LEGACY_ROOT runtime`: `runtime/` is not a canonical root.

The classification, not the dependency, was wrong:

- **Not SUBSTRATE.** The grammar defines substrate as cross-domain machinery (storage, execution, events, projection) that depends only on the kernel and other substrate. Runtime is Atlas-specific orchestration — census, reconciliation, seal, AtlasX materialization, verification, recensus, agent missions — and it is effectful at the boundary: it calls the `adapter` crate at about 126 sites (filesystem and repository audit, Cargo dependency census, semantic extractors, browser instruments) and itself spawns `git`, `cargo`, `rustc`, `unshare` and `df` and reads and writes the filesystem. Removing the adapter dependency would not make it substrate; it would move the same effects into Runtime.
- **Not APPLICATION.** The grammar defines an application as an entry point that nothing depends on (APPLICATION may not depend on APPLICATION). Runtime is the engine that `apps/cli` (`atlas-systemizer`, a thin command projection) calls, and the README's execution model makes the same Runtime the engine behind the CLI, CI/API integrations, the MCP adapter and the future Studio IDE "without creating parallel truth systems". Folding it into `apps/cli` would either make those surfaces depend on an application or duplicate it, and it would retire an active node identity.
- **ADAPTER.** The grammar's adapter plane is "the boundary to the outside world: protocols, providers, engines", and an adapter may depend on KERNEL, SUBSTRATE, DOMAIN and ADAPTER, while applications may depend on adapters. Runtime is the Atlas engine at that boundary.

## Decision

1. The `.ynventa` repository grammar is the authority for the physical layout of this repository. A local document or configuration that contradicts it is superseded and updated with it.
2. `compiler.runtime` is classified `NodeKind::Adapter`, canonical path `adapter/runtime`. Its key, and so its NodeId, is unchanged (`yn1:b085d64930bd4d4df465ff2f1967003f`); only `kind`, `path` and `canonical_path` change.
3. The crate moves from `runtime/` to `adapter/runtime/` by `ynventa migrate apply`. The package name stays `runtime`, so `use runtime::...` callers and `cargo -p runtime` are unchanged. `adapter/runtime/` is nested inside the role directory that the root adapter node `compiler.adapter` (`adapter/`) also owns; ownership is by the longest declared path, so the two nodes stay distinct.
4. Local enforcement moves with it: the workspace members and the `atlas-cli` path dependency, `.atlas/repo.toml` `[code]` roots, the ADL materialization `materialize Runtime { path = "adapter/runtime" }` in `.atlas/declared/system.adl` (and the derived `.atlas/declared/census.adl`), the visual-instrument registry fixture path in `.atlas/roadmap/CREATOR-INSTRUMENTS.toml`, the document path-reference roots in `adapter::audit_docs`, and the tests that locate the workspace root from `CARGO_MANIFEST_DIR`.

## Consequences

- `PLANE_VIOLATION compiler.runtime`, `LEGACY_PLACEMENT compiler.runtime` and `LEGACY_ROOT runtime` close; every declared node is at its canonical path.
- History is not rewritten. ADRs 0001–0101, `.atlas/evidence/**`, census records, generation proofs and fixtures keep the `runtime/...` paths they recorded; for any record made before this decision, `runtime/<rest>` is today's `adapter/runtime/<rest>`.
- Residual (not done here): Runtime still mixes pure Atlas orchestration semantics with effects. Extracting effect-free semantics into a DOMAIN node (which may not depend on adapters) and keeping only the effectful engine in the adapter plane is a later, separate refoundation step with its own decision.
