#!/usr/bin/env node
// current-state.mjs — freshly-regenerated current-state projection (CHRONICA HARD SEMANTIC
// REFOUNDATION CORRECTION directive, 2026-09-06, Section 27).
//
// docs/_machine/graph-refoundation/subsystem-convergence.jsonl is an append-only HISTORICAL
// ledger -- a 2026-09-06 audit (this same correction pass) confirmed its append-only discipline
// is genuinely sound (corrections are recorded as new entries referencing what they correct, never
// as edits to old lines), but also found its one attempt at a point-in-time "current state"
// snapshot (a `program_checkpoint_summary` entry) went stale within hours: it claimed cap_count
// 328 and "exactly one" chronica-core-world-graph dependent at the time it was written, both
// already false by the time of the very next audit.
//
// This script is the fix: it never itself becomes a historical record and is never hand-edited --
// every value is computed fresh, on demand, from the live repository. Nothing in this file is
// authoritative TRUTH about architecture quality (that requires human/semantic judgment -- see the
// M4/M5/M9 note in verify-responsibility-boundaries.mjs's module doc); it is a fast, honest report
// of what a machine CAN check right now, so nobody has to trust a JSONL line written hours or days
// ago.
//
// Usage:
//   node tools/ci/current-state.mjs             -- human-readable report to stdout
//   node tools/ci/current-state.mjs --json       -- the same data as one JSON object
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = join(SCRIPT_DIR, '..', '..')
const JSON_MODE = process.argv.includes('--json')

function git(args) {
  try {
    return execFileSync('git', args, { cwd: REPO_ROOT, encoding: 'utf8' }).trim()
  } catch {
    return null
  }
}

function runNodeScript(relPath, args = []) {
  try {
    const out = execFileSync('node', [join(REPO_ROOT, relPath), ...args], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
    })
    return { ok: true, exitCode: 0, output: out.trim() }
  } catch (err) {
    return { ok: false, exitCode: err.status ?? 1, output: `${err.stdout ?? ''}${err.stderr ?? ''}`.trim() }
  }
}

function countCapDirectories() {
  const dir = join(REPO_ROOT, 'crates', 'cap')
  if (!existsSync(dir)) return 0
  return readdirSync(dir, { withFileTypes: true }).filter((e) => e.isDirectory()).length
}

function loadJsonIfExists(relPath) {
  const abs = join(REPO_ROOT, relPath)
  if (!existsSync(abs)) return null
  try {
    return JSON.parse(readFileSync(abs, 'utf8'))
  } catch {
    return null
  }
}

function countJsonlLines(relPath) {
  const abs = join(REPO_ROOT, relPath)
  if (!existsSync(abs)) return 0
  return readFileSync(abs, 'utf8').split('\n').filter((l) => l.trim().length > 0).length
}

function main() {
  const head = git(['rev-parse', 'HEAD'])
  const branch = git(['rev-parse', '--abbrev-ref', 'HEAD'])
  const dirty = git(['status', '--porcelain'])

  const capDirectoryCount = countCapDirectories()

  const topology = runNodeScript('tools/ci/verify-repository-topology.mjs')
  const layers = runNodeScript('tools/refoundation/validate-refoundation-layers.mjs')
  const responsibility = runNodeScript('tools/ci/verify-responsibility-boundaries.mjs', ['--report'])

  const responsibilityBaseline = loadJsonIfExists('tools/ci/responsibility-baseline.json')
  const layerDebt = loadJsonIfExists('tools/refoundation/refoundation-layer-debt.json')

  const ledgerEntryCount = countJsonlLines('docs/_machine/graph-refoundation/subsystem-convergence.jsonl')

  const state = {
    current_head: head,
    current_branch: branch,
    working_tree_dirty: Boolean(dirty && dirty.length > 0),
    cap_directory_count: capDirectoryCount,
    misplaced_cap_package_count: responsibilityBaseline?.misplaced_cap_package_count ?? null,
    misplaced_cap_package_count_by_root: responsibilityBaseline?.misplaced_cap_package_count_by_root ?? null,
    world_graph_live_dependency_count: responsibilityBaseline?.world_graph_live_dependency_count ?? null,
    forbidden_core_io_count: responsibilityBaseline?.forbidden_core_io_count ?? null,
    layer_debt_edge_count: Array.isArray(layerDebt?.forbidden_edges) ? layerDebt.forbidden_edges.length : null,
    topology_gate: topology.ok ? 'GREEN' : 'FAILED',
    refoundation_layer_gate: layers.ok ? 'GREEN' : 'FAILED',
    responsibility_gate: responsibility.ok ? 'GREEN' : 'FAILED',
    subsystem_convergence_ledger_entry_count: ledgerEntryCount,
    binding_model_count: 'NOT_MACHINE_CHECKABLE -- requires semantic judgment, see verify-responsibility-boundaries.mjs module doc (M5)',
    caller_farming_suspect_count: 'NOT_MACHINE_CHECKABLE -- requires semantic judgment, see verify-responsibility-boundaries.mjs module doc (M9)',
    canonical_graph_owner_count: 'NOT_MACHINE_CHECKABLE -- requires semantic judgment, see verify-responsibility-boundaries.mjs module doc (M4)',
    generated_at: new Date().toISOString(),
  }

  if (JSON_MODE) {
    console.log(JSON.stringify(state, null, 2))
    return
  }

  console.log('[current-state] freshly computed, not a cached/historical value:')
  console.log(`  current_head                      = ${state.current_head}`)
  console.log(`  current_branch                     = ${state.current_branch}`)
  console.log(`  working_tree_dirty                 = ${state.working_tree_dirty}`)
  console.log(`  cap_directory_count                = ${state.cap_directory_count}`)
  console.log(`  misplaced_cap_package_count        = ${state.misplaced_cap_package_count} (by root: ${JSON.stringify(state.misplaced_cap_package_count_by_root)})`)
  console.log(`  world_graph_live_dependency_count  = ${state.world_graph_live_dependency_count}`)
  console.log(`  forbidden_core_io_count            = ${state.forbidden_core_io_count}`)
  console.log(`  layer_debt_edge_count              = ${state.layer_debt_edge_count}`)
  console.log(`  topology_gate                      = ${state.topology_gate}`)
  console.log(`  refoundation_layer_gate            = ${state.refoundation_layer_gate}`)
  console.log(`  responsibility_gate                = ${state.responsibility_gate}`)
  console.log(`  subsystem_convergence_ledger_entry_count = ${state.subsystem_convergence_ledger_entry_count}`)
  console.log(`  binding_model_count                = ${state.binding_model_count}`)
  console.log(`  caller_farming_suspect_count       = ${state.caller_farming_suspect_count}`)
  console.log(`  canonical_graph_owner_count        = ${state.canonical_graph_owner_count}`)
  console.log(`  generated_at                       = ${state.generated_at}`)
}

main()
