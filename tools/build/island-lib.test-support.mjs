export { default as test } from 'node:test'
export { default as assert } from 'node:assert/strict'
export { spawnSync } from 'node:child_process'
import assert from 'node:assert/strict'
import { dirname, join } from 'node:path'
export { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { computeIslandEvidenceFingerprint } from './island-lib.mjs'
export {
  applyAllowlistPolicy,
  classifyWorkspace,
  computeIslandEvidenceFingerprint,
  ISLAND_SCAN_POLICY_VERSION,
  stableStringify,
  validateAllowlistDoc,
} from './island-lib.mjs'

export const here = dirname(fileURLToPath(import.meta.url))
export const repoRoot = join(here, '..', '..')

// ── fixture builders ────────────────────────────────────────────────────────
// Shapes mirror real `cargo metadata --format-version 1 --no-deps` output
// closely enough for classifyWorkspace(), without needing a real cargo toolchain.

export function pkgId(name) {
  return `path+file:///fixture/crates/${name}#0.1.0`
}

export function makePkg(name, { bin = false, lib = true, procMacro = false, version = '0.1.0', deps = [] } = {}) {
  const targets = []
  if (lib) targets.push({ kind: ['lib'], crate_types: [procMacro ? 'proc-macro' : 'lib'], name: name.replace(/-/g, '_') })
  if (bin) targets.push({ kind: ['bin'], crate_types: ['bin'], name })
  if (targets.length === 0) targets.push({ kind: ['test'], crate_types: ['bin'], name: `${name}_test` })
  return {
    name,
    version,
    id: pkgId(name),
    manifest_path: `/fixture/crates/${name}/Cargo.toml`,
    dependencies: deps.map((d) => ({
      name: d.name,
      kind: d.kind ?? null,
      optional: d.optional ?? false,
      target: d.target ?? null,
    })),
    targets,
  }
}

export function makeMetadata(pkgs) {
  return {
    packages: pkgs,
    workspace_members: pkgs.map((p) => p.id),
    workspace_root: '/fixture',
  }
}

export function todayIso(offsetDays = 0) {
  const d = new Date(Date.UTC(2026, 6, 16)) // 2026-07-16, matches this task's fixed "now"
  d.setUTCDate(d.getUTCDate() + offsetDays)
  return d.toISOString().slice(0, 10)
}

export function fingerprintFor(scan, crateName) {
  const c = scan.crates.find((c) => c.crate === crateName)
  assert.ok(c, `fixture bug: no crate named ${crateName} in scan`)
  return computeIslandEvidenceFingerprint(c)
}

export const DUMMY_FINGERPRINT = 'a'.repeat(64)

export function validEntry(overrides = {}) {
  return {
    crate: 'chronica-fixture-lib',
    reason: 'test fixture exception',
    evidence: 'zero reverse dependents in fixture graph',
    evidence_fingerprint: DUMMY_FINGERPRINT,
    owner: 'test-owner',
    created: todayIso(-1),
    expires: todayIso(13),
    issue: 'https://github.com/llgtrn/Chronica/issues/1556',
    ...overrides,
  }
}

export function knownFor(...crates) {
  return crates
}

// ── current-repo truth ──────────────────────────────────────────────────────
// Runs the real gate against the real workspace. Requires a cargo toolchain
// (present in the rust.yml CI lane this gate is wired into); self-skips
// elsewhere, mirroring how the DB-gated Rust tests self-skip without DATABASE_URL.
