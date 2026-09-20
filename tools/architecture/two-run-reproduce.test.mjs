import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

import { reproduceTwoRun } from './two-run-reproduce.mjs'

const CLI_PATH = fileURLToPath(new URL('./two-run-reproduce.mjs', import.meta.url))
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')
const COMMITTED_EVIDENCE_PATH = join(
  REPO_ROOT,
  'docs',
  '_machine',
  'reconcile-reports',
  'cloud',
  'chronica-olap-architecture-two-run-evidence-pr2396-pass11.json',
)

function makeFixtureRoot() {
  const dir = mkdtempSync(join(tmpdir(), 'two-run-reproduce-fixture-'))
  // `buildArchitectureDb` requires a readable `Cargo.toml` at `root` (workspace member discovery
  // reads it unconditionally); an empty member list is enough to exercise the full pipeline
  // without depending on this repo's own (large, slower-to-scan) real crate tree.
  writeFileSync(join(dir, 'Cargo.toml'), '[workspace]\nmembers = []\n')
  return dir
}

test('reproduceTwoRun against a minimal fixture root rebuilds twice and reports a deterministic normalized digest', () => {
  const dir = makeFixtureRoot()
  try {
    const result = reproduceTwoRun({ root: dir })
    assert.equal(result.tool, 'tools/architecture/two-run-reproduce.mjs')
    assert.equal(result.digest_tool, 'tools/architecture/two-run-digest.mjs')
    assert.equal(result.root, dir)
    assert.equal(result.normalized_identical, true)
    // Two independent SQLite builds a few milliseconds apart are not byte-identical in practice
    // (re-stamped generated_at/verified_at values, differing internal page layout) -- this is the
    // expected, documented case this tool exists to distinguish from the normalized invariant.
    assert.equal(result.raw_identical, false)
    assert.ok(result.run1.counts.architecture_node > 0)
    assert.deepEqual(result.run1.counts, result.run2.counts)
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('reproduceTwoRun against the real repo root (live, not a fixture) is deterministic right now, at this exact commit', () => {
  const result = reproduceTwoRun({ root: REPO_ROOT })
  assert.equal(result.normalized_identical, true)
  assert.equal(result.raw_identical, false)
  assert.ok(result.run1.counts.architecture_node > 1000, 'the real repo has well over 1,000 nodes')
  assert.deepEqual(result.run1.counts, result.run2.counts)
})

test('CLI: spawning the real command against a fixture root writes --out with the full evidence shape', () => {
  const dir = makeFixtureRoot()
  const outPath = join(dir, 'evidence.json')
  try {
    const result = spawnSync(process.execPath, [CLI_PATH, '--root', dir, '--out', outPath], {
      encoding: 'utf8',
    })
    assert.equal(result.status, 0, `expected exit 0, got ${result.status}; stderr: ${result.stderr}`)
    assert.match(result.stdout, /normalized_identical=true/)
    assert.ok(existsSync(outPath))

    const written = JSON.parse(readFileSync(outPath, 'utf8'))
    assert.equal(written.tool, 'tools/architecture/two-run-reproduce.mjs')
    assert.equal(written.normalized_identical, true)
    assert.equal(written.root, resolve(dir))
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('the committed pass-11 architecture evidence file is exactly this tool\'s own output shape -- no hand-added fields', () => {
  assert.ok(
    existsSync(COMMITTED_EVIDENCE_PATH),
    `expected ${COMMITTED_EVIDENCE_PATH} to exist -- regenerate it via ` +
      '`node tools/architecture/two-run-reproduce.mjs --out <that path>` before running this test',
  )
  const committed = JSON.parse(readFileSync(COMMITTED_EVIDENCE_PATH, 'utf8'))

  const dir = makeFixtureRoot()
  try {
    const fresh = reproduceTwoRun({ root: dir })

    // Same top-level and nested key SETS as a fresh run -- the actual, testable form of "this file
    // is untouched tool output": a hand-added field (e.g. a narrative `purpose`/`non_claims`) would
    // show up here as a key present in `committed` but absent from `fresh`, and vice versa for a
    // field the tool emits that a manual edit stripped out.
    assert.deepEqual(Object.keys(committed).sort(), Object.keys(fresh).sort())
    assert.deepEqual(Object.keys(committed.run1).sort(), Object.keys(fresh.run1).sort())
    assert.deepEqual(Object.keys(committed.run2).sort(), Object.keys(fresh.run2).sort())
    assert.deepEqual(Object.keys(committed.run1.counts).sort(), Object.keys(fresh.run1.counts).sort())

    assert.equal(committed.tool, fresh.tool)
    assert.equal(committed.digest_tool, fresh.digest_tool)
    assert.equal(committed.spec_version, fresh.spec_version)
    assert.equal(typeof committed.normalized_identical, 'boolean')
    // This pass's own committed record must assert the deterministic invariant it exists to prove,
    // not a false or absent one.
    assert.equal(committed.normalized_identical, true)
    assert.match(committed.run1.raw_sha256, /^[0-9a-f]{64}$/)
    assert.match(committed.run1.normalized_sha256, /^[0-9a-f]{64}$/)
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})
