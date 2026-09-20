import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

import Database from 'better-sqlite3'
import { digestPair, normalizedSha256, rawSha256, tableCounts } from './two-run-digest.mjs'

const CLI_PATH = fileURLToPath(new URL('./two-run-digest.mjs', import.meta.url))

/** Spawn the real CLI as a real child process -- proves the `pathToFileURL`-based main-guard
 * actually enters `main()` when invoked the documented way, not merely that the imported
 * functions behave correctly in-process. */
function runCli(args) {
  return spawnSync(process.execPath, [CLI_PATH, ...args], { encoding: 'utf8' })
}

function makeDb(path, { nodeId, verifiedAt, generatedAt, nodeName = 'a' }) {
  const db = new Database(path)
  db.exec(`
    CREATE TABLE architecture_node (id INTEGER PRIMARY KEY, name TEXT);
    CREATE TABLE architecture_evidence (id INTEGER PRIMARY KEY, node TEXT, verified_at TEXT);
    CREATE TABLE meta (k TEXT, v TEXT);
  `)
  db.prepare('INSERT INTO architecture_node (id, name) VALUES (?, ?)').run(nodeId, nodeName)
  db.prepare('INSERT INTO architecture_evidence (id, node, verified_at) VALUES (?, ?, ?)').run(
    nodeId + 100,
    nodeName,
    verifiedAt,
  )
  db.prepare('INSERT INTO meta (k, v) VALUES (?, ?)').run('generated_at', generatedAt)
  db.prepare('INSERT INTO meta (k, v) VALUES (?, ?)').run('schema_version', '1')
  db.close()
}

test('normalizedSha256 is identical across two DBs differing only in id/verified_at/generated_at', () => {
  const dir = mkdtempSync(join(tmpdir(), 'two-run-digest-'))
  try {
    const db1 = join(dir, 'run1.db')
    const db2 = join(dir, 'run2.db')
    makeDb(db1, { nodeId: 1, verifiedAt: '2026-01-01T00:00:00Z', generatedAt: '2026-01-01T00:00:00Z' })
    makeDb(db2, { nodeId: 999, verifiedAt: '2026-06-06T00:00:00Z', generatedAt: '2026-06-06T00:00:00Z' })

    assert.notEqual(rawSha256(db1), rawSha256(db2), 'raw digests must differ (different id/timestamps)')
    assert.equal(
      normalizedSha256(db1),
      normalizedSha256(db2),
      'normalized digests must match once id/verified_at/generated_at are excluded',
    )
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('normalizedSha256 differs when real content (a node name) differs', () => {
  const dir = mkdtempSync(join(tmpdir(), 'two-run-digest-'))
  try {
    const db1 = join(dir, 'run1.db')
    const db2 = join(dir, 'run2.db')
    makeDb(db1, { nodeId: 1, verifiedAt: 'x', generatedAt: 'x', nodeName: 'module_a' })
    makeDb(db2, { nodeId: 1, verifiedAt: 'x', generatedAt: 'x', nodeName: 'module_b' })

    assert.notEqual(
      normalizedSha256(db1),
      normalizedSha256(db2),
      'normalized digests must differ when a real (non-excluded) value differs',
    )
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('rawSha256 is identical for byte-identical files and differs for any byte difference', () => {
  const dir = mkdtempSync(join(tmpdir(), 'two-run-digest-'))
  try {
    const dbA = join(dir, 'a.db')
    const dbB = join(dir, 'b.db')
    const dbC = join(dir, 'c.db')
    makeDb(dbA, { nodeId: 1, verifiedAt: 'x', generatedAt: 'x' })
    // Copy dbA's exact bytes to dbB via a fresh identical build (same inputs -> same file
    // bytes for this simple, timestamp-free construction).
    makeDb(dbB, { nodeId: 1, verifiedAt: 'x', generatedAt: 'x' })
    makeDb(dbC, { nodeId: 2, verifiedAt: 'x', generatedAt: 'x' })

    assert.equal(rawSha256(dbA), rawSha256(dbB))
    assert.notEqual(rawSha256(dbA), rawSha256(dbC))
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('digestPair/tableCounts compute the full evidence record directly, not only the hash helpers', () => {
  const dir = mkdtempSync(join(tmpdir(), 'two-run-digest-'))
  try {
    const db1 = join(dir, 'run1.db')
    const db2 = join(dir, 'run2.db')
    makeDb(db1, { nodeId: 1, verifiedAt: 'a', generatedAt: 'a' })
    makeDb(db2, { nodeId: 2, verifiedAt: 'b', generatedAt: 'b' })

    const counts1 = tableCounts(db1)
    assert.equal(counts1.architecture_node, 1)
    assert.equal(counts1.architecture_evidence, 1)
    assert.equal(counts1.meta, 2)

    const pair = digestPair(db1, db2)
    assert.equal(pair.tool, 'tools/architecture/two-run-digest.mjs')
    assert.equal(pair.run1.path, db1)
    assert.equal(pair.run2.path, db2)
    assert.equal(pair.raw_identical, false)
    assert.equal(pair.normalized_identical, true)
    assert.deepEqual(pair.run1.counts, tableCounts(db1))
    assert.equal(pair.run1.raw_sha256, rawSha256(db1))
    assert.equal(pair.run1.normalized_sha256, normalizedSha256(db1))
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('CLI: spawning the real command with matching DBs writes --out and prints a match', () => {
  const dir = mkdtempSync(join(tmpdir(), 'two-run-digest-cli-'))
  try {
    const db1 = join(dir, 'run1.db')
    const db2 = join(dir, 'run2.db')
    const outPath = join(dir, 'evidence.json')
    makeDb(db1, { nodeId: 1, verifiedAt: 'a', generatedAt: 'a' })
    makeDb(db2, { nodeId: 2, verifiedAt: 'b', generatedAt: 'b' })

    const result = runCli([db1, db2, '--out', outPath])

    assert.equal(result.status, 0, `expected exit 0, got ${result.status}; stderr: ${result.stderr}`)
    assert.match(result.stdout, /normalized_identical=true/)
    assert.match(result.stdout, /✓ normalized digests match/)
    assert.ok(existsSync(outPath), '--out file must exist after a successful run')

    const written = JSON.parse(readFileSync(outPath, 'utf8'))
    assert.equal(written.tool, 'tools/architecture/two-run-digest.mjs')
    assert.equal(written.normalized_identical, true)
    assert.equal(written.run1.raw_sha256, rawSha256(db1))
    assert.equal(written.run2.raw_sha256, rawSha256(db2))
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('CLI: spawning the real command with mismatched content exits non-zero and still writes --out', () => {
  const dir = mkdtempSync(join(tmpdir(), 'two-run-digest-cli-'))
  try {
    const db1 = join(dir, 'run1.db')
    const db2 = join(dir, 'run2.db')
    const outPath = join(dir, 'evidence.json')
    makeDb(db1, { nodeId: 1, verifiedAt: 'x', generatedAt: 'x', nodeName: 'module_a' })
    makeDb(db2, { nodeId: 1, verifiedAt: 'x', generatedAt: 'x', nodeName: 'module_b' })

    const result = runCli([db1, db2, '--out', outPath])

    assert.equal(result.status, 1)
    assert.match(result.stderr, /normalized digests differ/)
    assert.ok(existsSync(outPath), '--out file must still be written on a normalized-mismatch exit')
    const written = JSON.parse(readFileSync(outPath, 'utf8'))
    assert.equal(written.normalized_identical, false)
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})

test('CLI: spawning the real command with a missing argument exits 2 with a usage message', () => {
  const result = runCli(['/nonexistent/only-one-arg.db'])
  assert.equal(result.status, 2)
  assert.match(result.stderr, /usage: node tools\/architecture\/two-run-digest\.mjs/)
})
