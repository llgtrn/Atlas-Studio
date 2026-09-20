#!/usr/bin/env node
// verify-responsibility-boundaries.mjs — anti-gaming / responsibility CI gate (CHRONICA HARD
// SEMANTIC REFOUNDATION CORRECTION directive, 2026-09-06).
//
// This gate exists because the topology gate (verify-repository-topology.mjs) tracks only ONE
// metric -- the physical crates/cap/ directory count -- and a large migration wave this session
// proved that metric alone is gameable: ~262 `chronica-cap-*`-named crates were moved from
// crates/cap/ into crates/core/ with their package name, internal shape, and doc comments left
// completely unchanged (many still literally say "the X persistence adapter" in their own
// lib.rs). The physical cap-count metric improved (355 -> 41) while the real architecture did
// not: crates/core/ still holds hundreds of single-table Postgres CRUD adapters that are
// core-shaped in name only.
//
// This gate tracks the metrics a directory move cannot fake:
//   M2  misplaced_cap_package_count -- a package literally named `chronica-cap-*` living
//       anywhere OTHER than crates/cap/ (crates/core/, crates/runtime/, crates/adapter/). This is
//       migration debt inherited from before this gate existed; it is NEVER allowed to increase,
//       and a fresh decrease must update the baseline in the same change (matching
//       topology-baseline.json's cap-count convention).
//   M3  world_graph_live_dependency_count -- crates with a REAL (non-dev) Cargo dependency on
//       chronica-core-world-graph, the pre-refoundation graph substrate slated for retirement in
//       favor of chronica-core-graph. Per a dedicated 2026-09-06 audit, D0/D1/D2(candidate-side)
//       and its `canonical.rs` primitives are genuinely load-bearing in production today (the
//       machine-connector and digital-twin subsystems, plus chronica-cap-world-admission) -- this
//       crate cannot be deleted wholesale, but its live-dependency surface must never grow while
//       the convergence to chronica-core-graph is in flight. Only D3, graph_pattern.rs, and
//       binding.rs are confirmed to have zero real callers today.
//   M7  forbidden_core_io_count -- crates/core/* crates with a REAL (non-dev) dependency on an
//       I/O-performing crate (sqlx, reqwest, axum, hyper, redis, rusqlite). core/ is supposed to
//       be pure domain/value/state logic with no external-system I/O; a 2026-09-06 audit found
//       ~192 such crates (mostly satellite Postgres adapters left behind by the same relocation
//       wave). This gate does NOT attempt the audit's full semantic classification (some of these
//       hits are legitimate -- e.g. a crate depending on sqlx purely for `impl From<sqlx::Error>`
//       type mapping, never issuing a real query) -- CONFIRMED_IO_SEAM_EXEMPTIONS below records
//       the specific crates independently verified to have zero real I/O despite the dependency
//       hit, so the mechanical scan doesn't flag known-clean crates. This debt count is frozen as
//       a baseline and must never increase; decomposing one of these crates into a real
//       core/runtime/adapter split is real progress and should lower the baseline in that same
//       change.
//
// M1 (cap_directory_count) and M8 (known_layer_debt_edge_count) are already tracked by
// verify-repository-topology.mjs's cap-monotonic check and
// validate-refoundation-layers.mjs's debt-edge baseline respectively -- this gate does not
// duplicate them, it reports them for a single combined snapshot (see --report).
//
// M4 (canonical_graph_owner_count), M5 (duplicate_binding_model_count), and M9
// (caller_farming_suspect_count) require semantic judgment a mechanical script cannot safely
// automate (is a "Binding"-named struct actually a duplicate canonical model, or a legitimately
// distinct concept? did a new HTTP route get added because of real product need, or to flip a
// reachability metric?) -- these are tracked by hand in
// docs/_machine/graph-refoundation/current-state.json (see that file's own header) and reviewed
// at every capability-wiring/migration-batch pull request, not gated here.
//
// Usage:
//   node tools/ci/verify-responsibility-boundaries.mjs           -- gate mode, exits 1 on a new violation
//   node tools/ci/verify-responsibility-boundaries.mjs --report  -- print current metric values, exit 0
//
// RESPONSIBILITY_GATE_AUDIT_MODE=true forces report-only (never set in CI) for local debugging of
// a new, not-yet-dispositioned finding.
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = join(SCRIPT_DIR, '..', '..')
const BASELINE_PATH = join(SCRIPT_DIR, 'responsibility-baseline.json')

const AUDIT_MODE = process.env.RESPONSIBILITY_GATE_AUDIT_MODE === 'true'
const REPORT_ONLY = process.argv.includes('--report')

const PRODUCT_CRATE_ROOTS = ['core', 'runtime', 'adapter']

// Crates independently source-read (2026-09-06 core-purity audit) and confirmed to have a real
// I/O-crate Cargo dependency that is NEVER used for real I/O -- e.g. `sqlx` pulled in solely for
// `impl From<sqlx::Error> for CoreError`, no query/pool/transaction code anywhere in the crate.
// Grown only when a specific crate is re-verified the same way; never as a blanket allowlist.
const CONFIRMED_IO_SEAM_EXEMPTIONS = new Set([
  'chronica-core-errors', // sqlx dep used only for error-type mapping (impl From<sqlx::Error>)
  'chronica-core-network-identity-error', // same pattern as chronica-core-errors
])

const IO_DEPENDENCY_NAMES = new Set(['sqlx', 'reqwest', 'axum', 'hyper', 'redis', 'rusqlite'])

function listCrateDirs(root) {
  const abs = join(REPO_ROOT, 'crates', root)
  if (!existsSync(abs)) return []
  return readdirSync(abs, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => ({ dirName: e.name, absPath: join(abs, e.name) }))
}

// Minimal, dependency-free TOML [dependencies]/[dev-dependencies] section reader. Good enough for
// this workspace's consistently-formatted Cargo.toml files (one dependency per line, `name = ...`
// or `name = { ... }`); does not attempt full TOML parsing.
function readCargoToml(absCrateDir) {
  const path = join(absCrateDir, 'Cargo.toml')
  if (!existsSync(path)) return { packageName: null, realDeps: new Set(), devDeps: new Set() }
  const text = readFileSync(path, 'utf8')
  const lines = text.split('\n')
  let packageName = null
  let section = null // 'package' | 'dependencies' | 'dev-dependencies' | 'features' | other
  const realDeps = new Set()
  const devDeps = new Set()
  for (const rawLine of lines) {
    const line = rawLine.trim()
    if (line.startsWith('#') || line.length === 0) continue
    const sectionMatch = line.match(/^\[([^\]]+)\]$/)
    if (sectionMatch) {
      const name = sectionMatch[1]
      if (name === 'package') section = 'package'
      else if (name === 'dependencies') section = 'dependencies'
      else if (name === 'dev-dependencies') section = 'dev-dependencies'
      else section = 'other'
      continue
    }
    if (section === 'package' && packageName === null) {
      const m = line.match(/^name\s*=\s*"([^"]+)"/)
      if (m) packageName = m[1]
    }
    if (section === 'dependencies' || section === 'dev-dependencies') {
      const m = line.match(/^([A-Za-z0-9_-]+)\s*=/)
      if (m) {
        const depName = m[1]
        const isOptional = /optional\s*=\s*true/.test(line)
        // An `optional = true` dependency only becomes real when a non-default feature enables
        // it -- most `db`-feature-gated crates in this workspace follow exactly that shape. We
        // still count it as a real dependency for this gate's purposes: the concrete adapter code
        // that uses it physically lives in this crate either way (default-off does not mean
        // absent), matching how the 2026-09-06 core-purity audit itself counted these crates.
        void isOptional
        ;(section === 'dependencies' ? realDeps : devDeps).add(depName)
      }
    }
  }
  return { packageName, realDeps, devDeps }
}

function countMisplacedCapPackages(violations, warnings) {
  const perRoot = {}
  let total = 0
  const examples = []
  for (const root of PRODUCT_CRATE_ROOTS) {
    let count = 0
    for (const { dirName, absPath } of listCrateDirs(root)) {
      const { packageName } = readCargoToml(absPath)
      const name = packageName ?? dirName
      if (name.startsWith('chronica-cap-')) {
        count++
        if (examples.length < 5) examples.push(`crates/${root}/${dirName}`)
      }
    }
    perRoot[root] = count
    total += count
  }
  return { total, perRoot, examples }
}

function countWorldGraphLiveDependents() {
  const dependents = []
  for (const root of [...PRODUCT_CRATE_ROOTS, 'cap']) {
    for (const { dirName, absPath } of listCrateDirs(root)) {
      const { packageName, realDeps } = readCargoToml(absPath)
      if (packageName === 'chronica-core-world-graph') continue // self
      if (realDeps.has('chronica-core-world-graph')) {
        dependents.push(`crates/${root}/${dirName}`)
      }
    }
  }
  return dependents
}

function countForbiddenCoreIo() {
  const hits = []
  for (const { dirName, absPath } of listCrateDirs('core')) {
    const { packageName, realDeps } = readCargoToml(absPath)
    const name = packageName ?? dirName
    if (CONFIRMED_IO_SEAM_EXEMPTIONS.has(name)) continue
    for (const dep of realDeps) {
      if (IO_DEPENDENCY_NAMES.has(dep)) {
        hits.push({ crate: name, dep })
        break // one hit per crate is enough for a count metric
      }
    }
  }
  return hits
}

function loadBaseline() {
  if (!existsSync(BASELINE_PATH)) return null
  return JSON.parse(readFileSync(BASELINE_PATH, 'utf8'))
}

function writeBaseline(metrics) {
  writeFileSync(
    BASELINE_PATH,
    JSON.stringify(
      {
        _note:
          'Debt baseline for tools/ci/verify-responsibility-boundaries.mjs. Every field is a ' +
          'monotonic-non-increase ceiling: a real decrease must update the recorded number in ' +
          'the same change that caused it (never let a real reduction silently allow drifting ' +
          'back up); an increase fails the gate. Regenerated fields are computed, not hand-edited.',
        misplaced_cap_package_count: metrics.misplaced.total,
        misplaced_cap_package_count_by_root: metrics.misplaced.perRoot,
        world_graph_live_dependency_count: metrics.worldGraphDependents.length,
        forbidden_core_io_count: metrics.coreIoHits.length,
        recorded_at: new Date().toISOString().slice(0, 10),
      },
      null,
      2,
    ) + '\n',
  )
}

function main() {
  const misplaced = countMisplacedCapPackages()
  const worldGraphDependents = countWorldGraphLiveDependents()
  const coreIoHits = countForbiddenCoreIo()
  const metrics = { misplaced, worldGraphDependents, coreIoHits }

  if (REPORT_ONLY) {
    console.log('[responsibility] current metric snapshot:')
    console.log(`  M2 misplaced_cap_package_count       = ${misplaced.total} (core=${misplaced.perRoot.core ?? 0}, runtime=${misplaced.perRoot.runtime ?? 0}, adapter=${misplaced.perRoot.adapter ?? 0})`)
    console.log(`  M3 world_graph_live_dependency_count = ${worldGraphDependents.length}`)
    console.log(`  M7 forbidden_core_io_count            = ${coreIoHits.length}`)
    process.exit(0)
  }

  const violations = []
  const warnings = []

  const baseline = loadBaseline()
  if (!baseline) {
    writeBaseline(metrics)
    warnings.push(`no responsibility-baseline.json found -- wrote one recording current metrics as the debt baseline (M2=${misplaced.total}, M3=${worldGraphDependents.length}, M7=${coreIoHits.length})`)
  } else {
    if (misplaced.total > baseline.misplaced_cap_package_count) {
      violations.push(
        `misplaced_cap_package_count increased: baseline ${baseline.misplaced_cap_package_count} -> current ${misplaced.total}. ` +
          `A package literally named chronica-cap-* must never be newly created outside crates/cap/ -- ` +
          `moving a legacy cap crate's folder without renaming it is path relocation, not migration. ` +
          `Decompose it into real core/runtime/adapter owners (by actual responsibility) and delete the old package, ` +
          `or rename it only once its responsibility has genuinely changed. Examples of current debt: ${misplaced.examples.join(', ')}`,
      )
    } else if (misplaced.total < baseline.misplaced_cap_package_count) {
      warnings.push(
        `misplaced_cap_package_count decreased: baseline ${baseline.misplaced_cap_package_count} -> current ${misplaced.total} -- ` +
          `update tools/ci/responsibility-baseline.json's misplaced_cap_package_count (and by-root breakdown) to ${misplaced.total} in this same change.`,
      )
    }

    if (worldGraphDependents.length > baseline.world_graph_live_dependency_count) {
      violations.push(
        `world_graph_live_dependency_count increased: baseline ${baseline.world_graph_live_dependency_count} -> current ${worldGraphDependents.length}. ` +
          `chronica-core-world-graph is migration debt slated for retirement into chronica-core-graph -- no NEW crate ` +
          `may take a real dependency on it. Current live dependents: ${worldGraphDependents.join(', ')}`,
      )
    } else if (worldGraphDependents.length < baseline.world_graph_live_dependency_count) {
      warnings.push(
        `world_graph_live_dependency_count decreased: baseline ${baseline.world_graph_live_dependency_count} -> current ${worldGraphDependents.length} -- ` +
          `update tools/ci/responsibility-baseline.json's world_graph_live_dependency_count to ${worldGraphDependents.length} in this same change.`,
      )
    }

    if (coreIoHits.length > baseline.forbidden_core_io_count) {
      const newOnes = coreIoHits.map((h) => `${h.crate} (${h.dep})`)
      violations.push(
        `forbidden_core_io_count increased: baseline ${baseline.forbidden_core_io_count} -> current ${coreIoHits.length}. ` +
          `crates/core/ must contain no real I/O (sqlx/reqwest/axum/hyper/redis/rusqlite performing an actual query/` +
          `request/connection, not just a type-mapping dependency). If this is a confirmed false positive (the ` +
          `dependency is never used for real I/O), add the crate to CONFIRMED_IO_SEAM_EXEMPTIONS in this script after ` +
          `re-verifying its source. Otherwise the crate belongs under crates/adapter/. Current hits: ${newOnes.slice(0, 10).join(', ')}${newOnes.length > 10 ? `, +${newOnes.length - 10} more` : ''}`,
      )
    } else if (coreIoHits.length < baseline.forbidden_core_io_count) {
      warnings.push(
        `forbidden_core_io_count decreased: baseline ${baseline.forbidden_core_io_count} -> current ${coreIoHits.length} -- ` +
          `update tools/ci/responsibility-baseline.json's forbidden_core_io_count to ${coreIoHits.length} in this same change.`,
      )
    }
  }

  if (warnings.length > 0) {
    console.log(`[responsibility] ${warnings.length} known/tracked item(s):`)
    for (const w of warnings) console.log(`  - ${w}`)
  }

  if (violations.length > 0) {
    console.error(`[responsibility] ${violations.length} violation(s) of the responsibility-boundary contract:`)
    for (const v of violations) console.error(`  - ${v}`)
    if (AUDIT_MODE) {
      console.error('[responsibility] AUDIT MODE: reporting only, not failing the build.')
      process.exit(0)
    }
    process.exit(1)
  }

  console.log('[responsibility] clean: no new responsibility-boundary violations found.')
  process.exit(0)
}

main()
