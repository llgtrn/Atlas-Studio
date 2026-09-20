---
id: atlas.reports.current-repo-census-2026-09-20
type: report
status: active
canonical: true
---
# Current Repository Census

## Exact Base

- Branch: `main`
- HEAD: `c0613afb4c4b069c9eed53599f5e31eacd3882da`
- Status: clean against `origin/main`
- Recent head commit: `c0613af Refresh systemize evidence after unified graph`

## Atlas Control Root

`.atlas/` is present and remains the canonical knowledge root. It contains the active repo manifest, architecture, blueprints, contracts, census, provenance, evidence, reports, guides, references, licenses, artifacts and declared language material.

The current `.atlas/ops_production/` tree is supporting production/operations knowledge for building the Atlas system. It is not a production implementation root and should not be promoted to top-level ownership.

## Chronica Layout Comparison

The local Chronica repository is organized around responsibility roots:

- `core/`
- `runtime/`
- `adapter/`
- `apps/`
- `graph/`
- `bindings/`
- `deploy/`
- `organism/`

Atlas should mimic this responsibility-first structure where the responsibility exists. `organism/` is Chronica-specific and should not be created in Atlas until there is a real Atlas organism responsibility.

## Current Atlas Roots

- `.atlas/`: canonical control, architecture, provenance, census, evidence and reports.
- `core/`: Rust core semantics, currently still too monolithic in `core/src/lib.rs`.
- `runtime/`: Rust orchestration and CLI command implementation, currently has `runtime/src/main.rs` as the compatibility CLI binary.
- `adapter/`: Rust filesystem/Git/source observation adapters.
- `apps/ui/`: TypeScript UI seed.
- `graph/`: static graph definitions root, present but not yet integrated into the Rust engine.
- `bindings/`: static binding declarations root.
- `tools/`: transitional repository engineering machinery and Chronica-era imported tooling.

## Migration Mismatches Observed At Start

- `core/src/lib.rs` was a mixed-responsibility implementation file.
- `.atlas/declared/system.atlas` used the old declared language extension and needed to become `.adl`.
- Root `tests/` and `benches/` directories are present in the local filesystem but currently empty/untracked; they should not become universal dumping grounds.
- `Cargo.toml` still exposes only `core`, `runtime`, and `adapter`; `apps/cli` and `apps/mcp` are future application-root moves.
- `.atlas/repo.toml` listed `test_roots = ["tests"]`, which should be retired once owner-local test placement is enforced.

## Immediate Refoundation Slice

This wave starts the Chronica-style module decomposition inside `core/` without changing public CLI behavior:

- Move ADL lexer/parser/compiler ownership into `core/src/language/adl/`.
- Move engineering model/report types into `core/src/model/`.
- Move stable identity helpers and first typed IDs into `core/src/identity/`.
- Keep `core/src/lib.rs` as a module/export surface.
- Migrate declared source from `.atlas/declared/system.atlas` to `.atlas/declared/system.adl`.
- Retire `.atlas/repo.toml` root `tests` ownership in favor of owner-local test roots.

## Slice Results

- Completed `core/src/lib.rs` decomposition into `core/src/identity/`, `core/src/language/adl/`, and `core/src/model/`.
- Migrated declared ADL source to `.atlas/declared/system.adl`; `.atlas/declared/system.atlas` is removed.
- Updated `.atlas/repo.toml` to owner-local test roots.
- Refreshed ADL check, repository graph and systemize evidence from the `.adl` source.
