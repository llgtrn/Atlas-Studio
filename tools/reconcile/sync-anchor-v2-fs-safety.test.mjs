// sync-anchor-v2-fs-safety.test.mjs — path-traversal AND symlink/junction-escape containment,
// bounded reads, and incrementally-capped directory enumeration (repair round 5, items 2 and 4).
import assert from 'node:assert/strict'
import { closeSync, mkdirSync, mkdtempSync, openSync, rmSync, symlinkSync, writeFileSync, writeSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { isPathWithinRoot, listDirEntriesBounded, readBoundedWithinRoot, resolveWithinRoot } from './sync-anchor-v2-fs-safety.mjs'

function tmpRoot(prefix) {
  return mkdtempSync(join(tmpdir(), prefix))
}

// ── isPathWithinRoot: cheap lexical first-pass filter ───────────────────────────────────────────

test('isPathWithinRoot: a child path is within root', () => {
  assert.equal(isPathWithinRoot('/repo', '/repo/crates/foo'), true)
})

test('isPathWithinRoot: root itself is within root', () => {
  assert.equal(isPathWithinRoot('/repo', '/repo'), true)
})

test('isPathWithinRoot: a sibling/escaping path is rejected', () => {
  assert.equal(isPathWithinRoot('/repo', '/etc/passwd'), false)
  assert.equal(isPathWithinRoot('/repo', '/repo/../etc/passwd'), false)
  assert.equal(isPathWithinRoot('/repo', '/repository-sibling'), false)
})

// ── resolveWithinRoot: real (symlink-resolved) containment ──────────────────────────────────────

test('resolveWithinRoot: a plain file within root resolves ok', () => {
  const root = tmpRoot('chronica-fs-safety-plain-')
  try {
    const target = join(root, 'a.txt')
    writeFileSync(target, 'hi')
    const result = resolveWithinRoot(root, target)
    assert.equal(result.ok, true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('resolveWithinRoot: a nonexistent path is NOT_FOUND, never a thrown exception', () => {
  const root = tmpRoot('chronica-fs-safety-missing-')
  try {
    const result = resolveWithinRoot(root, join(root, 'does-not-exist.txt'))
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'NOT_FOUND')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('resolveWithinRoot: a lexically-escaping path is refused without touching the filesystem', () => {
  const root = tmpRoot('chronica-fs-safety-lexical-')
  try {
    const result = resolveWithinRoot(root, join(root, '..', 'etc', 'passwd'))
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'ESCAPES_ROOT')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('resolveWithinRoot: a symlink whose target resolves OUTSIDE root is refused (the core repair-round-5 item-2 case)', () => {
  const root = tmpRoot('chronica-fs-safety-symlink-escape-')
  const outside = tmpRoot('chronica-fs-safety-symlink-outside-')
  try {
    writeFileSync(join(outside, 'secret.txt'), 'outside-root-content')
    const linkPath = join(root, 'innocuous-looking.txt')
    symlinkSync(join(outside, 'secret.txt'), linkPath)

    const result = resolveWithinRoot(root, linkPath)
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'ESCAPES_ROOT')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('resolveWithinRoot: a symlink whose target resolves INSIDE root is accepted (no false positive on a legitimate in-root symlink)', () => {
  const root = tmpRoot('chronica-fs-safety-symlink-ok-')
  try {
    writeFileSync(join(root, 'real.txt'), 'inside-root-content')
    const linkPath = join(root, 'alias.txt')
    symlinkSync(join(root, 'real.txt'), linkPath)

    const result = resolveWithinRoot(root, linkPath)
    assert.equal(result.ok, true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('resolveWithinRoot: a symlinked DIRECTORY escaping root is refused, not just a symlinked file', () => {
  const root = tmpRoot('chronica-fs-safety-symlink-dir-escape-')
  const outside = tmpRoot('chronica-fs-safety-symlink-dir-outside-')
  try {
    writeFileSync(join(outside, 'file.txt'), 'x')
    const linkDir = join(root, 'linked-dir')
    symlinkSync(outside, linkDir, 'dir')
    const result = resolveWithinRoot(root, join(linkDir, 'file.txt'))
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'ESCAPES_ROOT')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

// ── readBoundedWithinRoot: symlink-safe, stat-first-bounded reads ───────────────────────────────

test('readBoundedWithinRoot: reads a normal in-root file', () => {
  const root = tmpRoot('chronica-fs-safety-read-ok-')
  try {
    const target = join(root, 'a.txt')
    writeFileSync(target, 'hello world')
    const result = readBoundedWithinRoot(root, target, 1000)
    assert.equal(result.ok, true)
    assert.equal(result.text, 'hello world')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('readBoundedWithinRoot: refuses a symlink escaping root, never reads the outside content', () => {
  const root = tmpRoot('chronica-fs-safety-read-escape-')
  const outside = tmpRoot('chronica-fs-safety-read-escape-outside-')
  try {
    const marker = 'OUTSIDE_ROOT_SECRET_MARKER'
    writeFileSync(join(outside, 'secret.txt'), marker)
    const linkPath = join(root, 'looks-safe.txt')
    symlinkSync(join(outside, 'secret.txt'), linkPath)

    const result = readBoundedWithinRoot(root, linkPath, 1000)
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'ESCAPES_ROOT')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('readBoundedWithinRoot: an oversized file is refused via stat alone, zero content bytes ever read', () => {
  const root = tmpRoot('chronica-fs-safety-read-oversized-')
  try {
    const target = join(root, 'big.txt')
    const marker = 'REAL_CONTENT_MARKER'
    writeFileSync(target, marker.repeat(1000))
    const result = readBoundedWithinRoot(root, target, 100)
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'OVERSIZED')
    assert.ok(result.size_bytes > 100)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('readBoundedWithinRoot: an oversized SPARSE file is refused via stat alone, without a slow full read', () => {
  const root = tmpRoot('chronica-fs-safety-read-sparse-')
  try {
    const target = join(root, 'sparse.bin')
    const declaredSize = 200 * 1024 * 1024
    const fd = openSync(target, 'w')
    writeSync(fd, Buffer.from('x'), 0, 1, declaredSize - 1)
    closeSync(fd)
    const startedAt = Date.now()
    const result = readBoundedWithinRoot(root, target, 1024 * 1024)
    const elapsedMs = Date.now() - startedAt
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'OVERSIZED')
    assert.equal(result.size_bytes, declaredSize)
    assert.ok(elapsedMs < 1000, `expected a near-instant stat-only refusal, took ${elapsedMs}ms`)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('readBoundedWithinRoot: a nonexistent path is NOT_FOUND', () => {
  const root = tmpRoot('chronica-fs-safety-read-missing-')
  try {
    const result = readBoundedWithinRoot(root, join(root, 'nope.txt'), 1000)
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'NOT_FOUND')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('readBoundedWithinRoot: a directory (not a file) at the path is NOT_A_FILE', () => {
  const root = tmpRoot('chronica-fs-safety-read-dir-')
  try {
    const dirPath = join(root, 'a-directory')
    mkdirSync(dirPath)
    const result = readBoundedWithinRoot(root, dirPath, 1000)
    assert.equal(result.ok, false)
    assert.equal(result.reason, 'NOT_A_FILE')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── listDirEntriesBounded: incremental cap, never an unbounded readdirSync+sort ─────────────────

test('listDirEntriesBounded: lists every entry when under the bound', () => {
  const root = tmpRoot('chronica-fs-safety-listdir-under-')
  try {
    for (let i = 0; i < 5; i += 1) writeFileSync(join(root, `f${i}.txt`), 'x')
    const { entries, truncated } = listDirEntriesBounded(root, 100)
    assert.equal(truncated, false)
    assert.equal(entries.length, 5)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listDirEntriesBounded: truncates deterministically at the bound, never more', () => {
  const root = tmpRoot('chronica-fs-safety-listdir-over-')
  try {
    for (let i = 0; i < 20; i += 1) writeFileSync(join(root, `f${i}.txt`), 'x')
    const { entries, truncated } = listDirEntriesBounded(root, 7)
    assert.equal(truncated, true)
    assert.equal(entries.length, 7)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listDirEntriesBounded: succeeds with an unreadable:null result under normal conditions', () => {
  const root = tmpRoot('chronica-fs-safety-listdir-ok-')
  try {
    writeFileSync(join(root, 'f.txt'), 'x')
    const { unreadable } = listDirEntriesBounded(root, 100)
    assert.equal(unreadable, null)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// repair round 6, item (b): opendirSync/readSync can both fail (ENOENT on a vanished/nonexistent
// path, ENOTDIR when the path is not actually a directory, EACCES/EPERM/ELOOP on real repos) --
// none of this was previously caught, so any of them crashed the whole scan with an uncaught
// native exception instead of the structured finding every other read failure in this tool
// produces. ENOENT/ENOTDIR are used here (not EACCES/chmod) because they are deterministic and
// portable regardless of the user running the test (root bypasses Unix permission bits, so a
// chmod-000-based EACCES test would not reliably fail even on a real defect).

test('listDirEntriesBounded: a nonexistent directory never throws, returns a structured unreadable result', () => {
  const missing = join(tmpdir(), 'chronica-fs-safety-listdir-missing-does-not-exist-xyz')
  const { entries, truncated, unreadable } = listDirEntriesBounded(missing, 100)
  assert.deepEqual(entries, [])
  assert.equal(truncated, false)
  assert.ok(unreadable, 'opendirSync ENOENT must be caught and reported, never thrown')
  assert.equal(unreadable.code, 'ENOENT')
})

test('listDirEntriesBounded: a path that is a FILE, not a directory, never throws, returns a structured unreadable result', () => {
  const root = tmpRoot('chronica-fs-safety-listdir-notadir-')
  try {
    const filePath = join(root, 'actually-a-file.txt')
    writeFileSync(filePath, 'x')
    const { entries, truncated, unreadable } = listDirEntriesBounded(filePath, 100)
    assert.deepEqual(entries, [])
    assert.equal(truncated, false)
    assert.ok(unreadable, 'opendirSync ENOTDIR must be caught and reported, never thrown')
    assert.equal(unreadable.code, 'ENOTDIR')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
