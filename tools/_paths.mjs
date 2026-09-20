// _paths.mjs — single source of truth for the paths of the operational tracking
// inputs the build/parity/capability tooling reads and writes.
//
// WHY THIS EXISTS: the machine tracking inputs (the parity registry, the slice plan,
// the donor inventory/absorption/retirement maps, the continuous-execution state, the
// true-ideal-scope projection) live under `docs/_machine/` — NOT a human surface — so
// the human-facing tracking surfaces are the numbered canonical docs
// (docs/NNN-*.md) plus canonical JSONL shards under docs/capabilities-canonical/
// and docs/architecture-canonical/. All tooling imports paths from here so a move is
// a one-line change and the docs tree never holds machine inputs again. (Reference prose that no tool reads was archived to
// docs/.archive/_data-reference/.)
//
// The SQLite DB paths below are generated cache paths retained for compatibility.
// They are not canonical authority and cloud PRs must not include binary DB changes.
import { join } from 'node:path';

const ROOT = process.cwd();

export const DOCS = join(ROOT, 'docs');
export const DATA = join(DOCS, '_machine');
export const ARCHIVE = join(DOCS, '.archive');

// Machine-generated + hybrid (prose + live capabilities.db/architecture.db fenced
// block) numbered docs live here — everything docs:gen fully regenerates or
// fence-injects on every run, kept out of the hand-authored docs/ tree they used
// to share so a reader can tell "written by a human" from "projected from a DB"
// by directory alone.
export const GENERATED = join(DOCS, '_generated');

// The 79+ machine-generated per-crate atlas docs (one 24-section atom doc per live
// chronica-* crate, gen-crate-docs.mjs) get their own top-level folder rather than sharing
// docs/_generated/ with the domain-roadmap/hybrid docs -- this is the "how it's implemented"
// layer of the target docs/ structure (docs/_archive/, docs/benchmarks/, docs/doctrines/,
// docs/crates/ as the only 4 top-level prose folders). Still 100% machine-regenerated on
// every docs:gen run, same as everything else that used to live in docs/_generated/.
export const CRATE_DOCS = join(DOCS, 'crates');

// The "why/what" layer — doctrine, invariants, and operating-model docs consolidated
// out of docs/ root (and design/specs/adr/programs/audit/plans) into one place. Purely
// hand-authored prose; never fence-injected. Scan alongside DOCS/GENERATED wherever a
// tool enumerates the numbered canonical tree so a doc's move here doesn't silently
// drop it from the index / evidence-ref scan.
export const DOCTRINES = join(DOCS, 'doctrines');

// Benchmark methodology/doctrine — the OSS-reference-portfolio, gate-definition, and
// donor-portfolio docs that define HOW the benchmark tooling (tools/benchmark/) measures
// and compares, as opposed to docs/_machine/'s machine-generated ratio-snapshot data
// (gitignored, not a human surface). The last of the 4 top-level prose folders in the
// target docs/ structure (docs/_archive/, docs/benchmarks/, docs/doctrines/, docs/crates/).
// Some of these docs are still fence-injected hybrids (prose + a live capabilities.db
// block) migrated in place from docs/_generated/ — see docs/benchmarks/README.md.
export const BENCHMARKS = join(DOCS, 'benchmarks');

export const CAPABILITIES_DB = join(DOCS, 'capabilities.db');
export const ARCHITECTURE_DB = join(DOCS, 'architecture.db');

// ── machine tracking inputs (live under docs/_machine/) ───────────────────────
export const REGISTRY = join(DATA, 'parity', 'capabilities.registry.json');
export const TRUE_SCOPE_DOC = join(DATA, 'parity', 'TRUE-IDEAL-SCOPE.md');
export const EXECUTION_STATE = join(DATA, 'parity', 'continuous-execution-state.json');

export const SLICE_PLAN = join(DATA, 'implementation', '14-vertical-slice-plan.md');

export const DONOR_INVENTORY = join(DATA, 'donor-inventory', 'donor-inventory.generated.json');
export const DONOR_CAPABILITY_MAP = join(DATA, 'donor-inventory', 'donor-capability-map.generated.json');

export const ABSORPTION_MAP = join(DATA, 'donor-absorption', 'absorption-map.generated.json');
export const RETIREMENT_MAP = join(DATA, 'architecture', 'package-retirement-map.generated.json');

// Human+git-visible generated mirror of the agent_note table in CAPABILITIES_DB (which is
// itself gitignored -- see the DB-paths note above). This file IS the durable, git-tracked
// record of the Claude<->Codex coordination log, so unlike the .generated.json machine
// inputs above it belongs under docs/_machine/ as a real tracked artifact, not a root file.
export const AGENT_NOTES_MD = join(DATA, 'agent-notes.md');
