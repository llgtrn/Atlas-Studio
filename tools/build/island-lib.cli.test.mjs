import {
  test,
  assert,
  spawnSync,
  join,
  applyAllowlistPolicy,
  classifyWorkspace,
  computeIslandEvidenceFingerprint,
  ISLAND_SCAN_POLICY_VERSION,
  stableStringify,
  validateAllowlistDoc,
  here,
  repoRoot,
  makePkg,
  makeMetadata,
  todayIso,
  fingerprintFor,
  DUMMY_FINGERPRINT,
  validEntry,
  knownFor,
} from './island-lib.test-support.mjs'
import { readFileSync } from 'node:fs'

test('CLI: --scan-only produces valid JSON with no allowlist/policy fields and exits 0', () => {
  const scriptPath = join(here, 'island-scan.mjs')
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    return // covered by the current-repo-truth skip above
  }
  const result = spawnSync('node', [scriptPath, '--scan-only'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stderr)
  const parsed = JSON.parse(result.stdout)
  assert.ok(Array.isArray(parsed.crates))
  assert.equal(parsed.pass, undefined)
})

test('CLI: replay via two real subprocess invocations is byte-identical', () => {
  const scriptPath = join(here, 'island-scan.mjs')
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    return
  }
  const first = spawnSync('node', [scriptPath, '--scan-only'], { cwd: repoRoot, encoding: 'utf8' })
  const second = spawnSync('node', [scriptPath, '--scan-only'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(first.status, 0, first.stderr)
  assert.equal(second.status, 0, second.stderr)
  assert.equal(first.stdout, second.stdout)
})

test('CLI: --scan-only is working-directory independent (script-relative, not cwd-relative)', () => {
  const scriptPath = join(here, 'island-scan.mjs')
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    return
  }
  const fromRoot = spawnSync('node', [scriptPath, '--scan-only'], { cwd: repoRoot, encoding: 'utf8' })
  const fromSubdir = spawnSync('node', [scriptPath, '--scan-only'], { cwd: join(repoRoot, 'core'), encoding: 'utf8' })
  assert.equal(fromRoot.status, 0, fromRoot.stderr)
  assert.equal(fromSubdir.status, 0, fromSubdir.stderr)
  assert.equal(fromRoot.stdout, fromSubdir.stdout)
})

test('CLI: the real repo gate passes today with the committed allowlist (real fingerprints match)', () => {
  const scriptPath = join(here, 'island-scan.mjs')
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    return
  }
  const result = spawnSync('node', [scriptPath, '--report', '--today', '2026-07-16'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stderr)
  const parsed = JSON.parse(result.stdout)
  assert.equal(parsed.pass, true)
  assert.deepEqual(parsed.stale_exceptions, [])
})

test('CLI: the real repo gate fails once every current allowlist entry has expired', () => {
  const scriptPath = join(here, 'island-scan.mjs')
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    return
  }
  const result = spawnSync('node', [scriptPath, '--today', '2099-01-01'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 1)
  const parsed = JSON.parse(result.stdout)
  assert.equal(parsed.pass, false)
  assert.ok(parsed.blocked_crates.length >= 4)
})

test('CLI: --print-fingerprint prints the real fingerprint for a currently-ISLAND crate and matches the committed allowlist entry', () => {
  const scriptPath = join(here, 'island-scan.mjs')
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    return
  }
  const result = spawnSync('node', [scriptPath, '--print-fingerprint', 'core'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stderr)
  const fp = result.stdout.trim()
  assert.match(fp, /^[0-9a-f]{64}$/)

  const allowlistPath = join(repoRoot, 'tools', 'build', 'island-scan-allowlist.json')
  const allowlist = JSON.parse(readFileSync(allowlistPath, 'utf8'))
  const entry = allowlist.exceptions.find((e) => e.crate === 'core')
  assert.equal(fp, entry.evidence_fingerprint)
})

test('CLI: --print-fingerprint refuses a crate that is not currently ISLAND', (t) => {
  const scriptPath = join(here, 'island-scan.mjs')
  const probe = spawnSync('cargo', ['--version'], { encoding: 'utf8' })
  if (probe.status !== 0) {
    return
  }
  const scanOnly = spawnSync('node', [scriptPath, '--scan-only'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(scanOnly.status, 0, scanOnly.stderr)
  const scan = JSON.parse(scanOnly.stdout)
  const nonIsland = scan.crates.find((c) => c.status !== 'ISLAND')
  if (!nonIsland) {
    // 2026-09-16 hard refoundation (issue #5324): the minimal bootstrap
    // workspace (core/runtime/adapter/organism) has no shipped entry point
    // yet, so every real crate is currently ISLAND — there is no non-ISLAND
    // crate to exercise this refusal against. Restore/un-skip once a real
    // entrypoint lands and this stops being vacuously true.
    t.skip('no non-ISLAND crate exists in the current bootstrap workspace (tracked by issue #5324)')
    return
  }
  const result = spawnSync('node', [scriptPath, '--print-fingerprint', nonIsland.crate], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 2)
  assert.match(result.stderr, /not ISLAND/)
})

test('sanity: ISLAND_SCAN_POLICY_VERSION is exported and stable', () => {
  assert.equal(typeof ISLAND_SCAN_POLICY_VERSION, 'number')
})
