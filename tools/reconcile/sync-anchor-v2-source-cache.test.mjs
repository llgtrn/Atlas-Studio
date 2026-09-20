// sync-anchor-v2-source-cache.test.mjs — SourceCache (genuinely bounded, symlink-safe, and never
// silently-empty on failure) and listRustFiles (deterministic disclosed truncation,
// symlink-escape-safe enumeration) tests (repair rounds 3 and 5).
//
// Split out of sync-anchor-v2-shard.test.mjs purely to keep both files under this repo's
// ~500-line file-size convention.
import assert from 'node:assert/strict'
import { closeSync, openSync, rmSync, statSync, symlinkSync, writeFileSync, writeSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { BOUNDS, listRustFiles, SourceCache } from './sync-anchor-v2-shard.mjs'
import { makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

// ── SourceCache: genuinely bounded, symlink-safe, and never silently-empty on failure ───────────

test('SourceCache: an oversized REGULAR file never returns any content -- not even a truncated prefix of the real bytes', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-oversized-regular-')
  try {
    const bigPath = join(cratePath, 'src', 'big.rs')
    const marker = 'REAL_FILE_CONTENT_MARKER'
    writeFileSync(bigPath, marker.repeat(200_000)) // ~4.6MB of real, recognizable bytes
    const cache = new SourceCache({ bound: 1_000_000, root: join(cratePath, 'src') })
    const result = cache.read(bigPath)
    assert.equal(result, '', 'an oversized file must return empty content, never a truncated prefix that still contains real bytes')
    assert.ok(!result.includes(marker))
    assert.equal(cache.oversizedFiles.length, 1)
    assert.equal(cache.oversizedFiles[0].absPath, bigPath)
    assert.ok(cache.oversizedFiles[0].size_bytes > 1_000_000)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('SourceCache: an oversized SPARSE file is refused via stat alone, without a slow full read (never allocates the whole untrusted source)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-oversized-sparse-')
  try {
    const sparsePath = join(cratePath, 'src', 'sparse.rs')
    const declaredSize = 200 * 1024 * 1024
    const fd = openSync(sparsePath, 'w')
    writeSync(fd, Buffer.from('x'), 0, 1, declaredSize - 1)
    closeSync(fd)
    assert.equal(statSync(sparsePath).size, declaredSize, 'sanity: the sparse file reports its full logical size via stat')

    const cache = new SourceCache({ bound: BOUNDS.MAX_SOURCE_FILE_BYTES, root: join(cratePath, 'src') })
    const startedAt = Date.now()
    const result = cache.read(sparsePath)
    const elapsedMs = Date.now() - startedAt

    assert.equal(result, '')
    assert.equal(cache.oversizedFiles.length, 1)
    assert.equal(cache.oversizedFiles[0].size_bytes, declaredSize)
    assert.ok(elapsedMs < 1000, `expected a near-instant stat-only refusal, took ${elapsedMs}ms`)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('SourceCache: a file within the bound still reads its real content normally (no regression)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-within-bound-')
  try {
    const smallPath = join(cratePath, 'src', 'small.rs')
    writeFileSync(smallPath, 'pub fn run() -> u32 { 42 }\n')
    const cache = new SourceCache({ bound: BOUNDS.MAX_SOURCE_FILE_BYTES, root: join(cratePath, 'src') })
    const result = cache.read(smallPath)
    assert.match(result, /pub fn run/)
    assert.equal(cache.oversizedFiles.length, 0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('SourceCache: a symlink escaping the crate source tree is refused and recorded, never silently followed (repair round 5, item 2)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-source-symlink-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-source-symlink-outside')
  try {
    mkdirSync(outside, { recursive: true })
    const marker = 'OUTSIDE_SOURCE_TREE_MARKER'
    writeFileSync(join(outside, 'evil.rs'), `pub fn run() { /* ${marker} */ }\n`)
    const linkPath = join(cratePath, 'src', 'linked.rs')
    symlinkSync(join(outside, 'evil.rs'), linkPath)

    const cache = new SourceCache({ root: join(cratePath, 'src') })
    const result = cache.read(linkPath)
    assert.equal(result, '')
    assert.ok(!result.includes(marker))
    assert.equal(cache.unreadableFiles.length, 1)
    assert.equal(cache.unreadableFiles[0].reason, 'ESCAPES_ROOT')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('SourceCache: a broken symlink (final target vanished) is recorded in unreadableFiles, never silently folded into an empty-file read (repair round 5, item 3)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-source-unreadable-')
  try {
    const brokenLink = join(cratePath, 'src', 'broken_link.rs')
    symlinkSync(join(cratePath, 'src', 'this_target_does_not_exist.rs'), brokenLink)
    const cache = new SourceCache({ root: join(cratePath, 'src') })
    const result = cache.read(brokenLink)
    assert.equal(result, '')
    assert.equal(cache.unreadableFiles.length, 1)
    assert.equal(cache.unreadableFiles[0].absPath, brokenLink)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── listRustFiles: deterministic, disclosed truncation, symlink-escape-safe ─────────────────────

test('listRustFiles: a directory count exceeding an injected bound is truncated deterministically', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-bound-')
  try {
    for (let i = 0; i < 10; i += 1) writeFileSync(join(cratePath, 'src', `extra_${i}.rs`), 'pub fn run() {}\n')
    const { files, truncated } = listRustFiles(join(cratePath, 'src'), 5)
    assert.equal(truncated, true)
    assert.equal(files.length, 5)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listRustFiles: a symlinked .rs file escaping the source tree is refused, never included in the listing (repair round 5, item 2)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-symlink-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-listrustfiles-outside')
  try {
    mkdirSync(outside, { recursive: true })
    writeFileSync(join(outside, 'evil.rs'), 'pub fn run() {}\n')
    symlinkSync(join(outside, 'evil.rs'), join(cratePath, 'src', 'linked.rs'))

    const { files, unsafeEntries } = listRustFiles(join(cratePath, 'src'))
    assert.ok(!files.some((f) => f.endsWith('linked.rs')))
    assert.equal(unsafeEntries.length, 1)
    assert.equal(unsafeEntries[0].reason, 'ESCAPES_ROOT')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('listRustFiles: a symlinked DIRECTORY escaping the source tree is refused, never walked into (repair round 5, item 2)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-symlink-dir-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-listrustfiles-outside-dir')
  try {
    mkdirSync(outside, { recursive: true })
    writeFileSync(join(outside, 'evil.rs'), 'pub fn run() {}\n')
    symlinkSync(outside, join(cratePath, 'src', 'linked_dir'), 'dir')

    const { files, unsafeEntries } = listRustFiles(join(cratePath, 'src'))
    assert.ok(!files.some((f) => f.includes('linked_dir')))
    assert.equal(unsafeEntries.length, 1)
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('listRustFiles: a symlinked in-tree file (no escape) is still included normally (no false positive)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-symlink-ok-')
  try {
    writeFileSync(join(cratePath, 'src', 'target.rs'), 'pub fn run() {}\n')
    symlinkSync(join(cratePath, 'src', 'target.rs'), join(cratePath, 'src', 'alias.rs'))

    const { files, unsafeEntries } = listRustFiles(join(cratePath, 'src'))
    assert.ok(files.some((f) => f.endsWith('alias.rs')))
    assert.equal(unsafeEntries.length, 0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 7, item 4: directory-symlink cycles, depth/count caps, and symlink stat ────────
// ── failures are all structured findings -- never an infinite loop, stack overflow, or a ────────
// ── silent `continue` with zero trace ────────────────────────────────────────────────────────────

test('listRustFiles: a directory symlink CYCLE fully contained within the source tree (never escaping root) is refused as SYMLINK_CYCLE, never an infinite loop', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-cycle-')
  try {
    // src/a/loop -> src/a -- a self-referential directory symlink. resolveWithinRoot's containment
    // check alone would PASS this (it never leaves `src/`), so without a visited-real-path guard
    // the walk would recurse into src/a/loop/loop/loop/... without bound.
    mkdirSync(join(cratePath, 'src', 'a'), { recursive: true })
    writeFileSync(join(cratePath, 'src', 'a', 'real.rs'), 'pub fn run() {}\n')
    symlinkSync(join(cratePath, 'src', 'a'), join(cratePath, 'src', 'a', 'loop'), 'dir')

    const { files, unsafeEntries, truncated } = listRustFiles(join(cratePath, 'src'))
    assert.ok(files.some((f) => f.endsWith(join('a', 'real.rs'))), 'the real file is still found -- only the cyclic re-entry is refused')
    assert.ok(unsafeEntries.some((e) => e.reason === 'SYMLINK_CYCLE'), 'the cycle must be reported, not silently ignored or infinitely walked')
    assert.equal(truncated, false, 'a single contained cycle refuses only that one entry -- it does not need to refuse the whole crate')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listRustFiles: a two-hop directory symlink cycle (a/link -> b, b/link -> a) is also detected, not just the direct self-reference case', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-cycle-2hop-')
  try {
    mkdirSync(join(cratePath, 'src', 'a'), { recursive: true })
    mkdirSync(join(cratePath, 'src', 'b'), { recursive: true })
    writeFileSync(join(cratePath, 'src', 'a', 'real.rs'), 'pub fn run() {}\n')
    symlinkSync(join(cratePath, 'src', 'b'), join(cratePath, 'src', 'a', 'link_to_b'), 'dir')
    symlinkSync(join(cratePath, 'src', 'a'), join(cratePath, 'src', 'b', 'link_to_a'), 'dir')

    const { unsafeEntries, truncated } = listRustFiles(join(cratePath, 'src'))
    assert.ok(unsafeEntries.some((e) => e.reason === 'SYMLINK_CYCLE'))
    assert.equal(truncated, false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listRustFiles: a BROKEN symlink (final target does not exist) is a structured finding (caught by resolveWithinRoot\'s own realpath resolution), never a silent skip', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-broken-symlink-')
  try {
    symlinkSync(join(cratePath, 'src', 'does-not-exist.rs'), join(cratePath, 'src', 'broken.rs'))

    const { files, unsafeEntries } = listRustFiles(join(cratePath, 'src'))
    assert.ok(!files.some((f) => f.endsWith('broken.rs')))
    // resolveWithinRoot's own realpathSync already fails closed on a broken symlink's final
    // target (reason NOT_FOUND) before this walk's own statSync-failure handling would even be
    // reached -- either way the outcome is a structured, visible finding, never a silent
    // `continue` with zero trace (the round-7 item 4 defect: the statSync catch block that
    // existed for a SEPARATE narrow TOCTOU race -- realpath succeeds, a later stat on the same
    // path fails -- previously discarded that race's failure silently; this test proves the
    // more common broken-symlink case was ALREADY covered by the pre-existing containment check).
    assert.ok(unsafeEntries.some((e) => e.path.endsWith('broken.rs') && (e.reason === 'NOT_FOUND' || e.reason === 'STAT_FAILED')))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listRustFiles: an injected directory-COUNT bound truncates deterministically, independent of the file-count bound (repair round 7, item 4)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-dircount-')
  try {
    // 20 SEPARATE, file-sparse subdirectories -- a pathologically WIDE (not deep) tree that would
    // never trip a generous file-count bound but must still be caught by an independent
    // directory-count cap once that cap is small.
    for (let i = 0; i < 20; i += 1) {
      const dir = join(cratePath, 'src', `d${i}`)
      mkdirSync(dir, { recursive: true })
      writeFileSync(join(dir, 'mod.rs'), 'pub fn run() {}\n')
    }
    const { files, truncated } = listRustFiles(join(cratePath, 'src'), 1000, BOUNDS.MAX_DIR_DEPTH, 5)
    assert.equal(truncated, true, 'exceeding the injected directory-count bound must refuse the whole crate, exactly like the file-count bound does')
    assert.ok(files.length < 20, 'the walk must stop well before visiting every directory once the count bound is exceeded')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listRustFiles: an injected DEPTH bound truncates deterministically, protecting against stack-overflow-depth recursion independent of file/directory count', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-listrustfiles-depth-')
  try {
    // A single, deeply-nested chain of one-subdirectory-per-level -- low total directory count,
    // but deep enough to matter for stack safety if unbounded.
    let dir = join(cratePath, 'src')
    for (let i = 0; i < 10; i += 1) {
      dir = join(dir, `level${i}`)
      mkdirSync(dir, { recursive: true })
    }
    writeFileSync(join(dir, 'deep.rs'), 'pub fn run() {}\n')

    const { files, truncated } = listRustFiles(join(cratePath, 'src'), BOUNDS.MAX_SOURCE_FILES_PER_CRATE, 3, BOUNDS.MAX_DIR_COUNT)
    assert.equal(truncated, true, 'a tree deeper than the injected depth bound must refuse the whole crate')
    assert.ok(!files.some((f) => f.endsWith('deep.rs')), 'the file past the depth bound must never be reached')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('listRustFiles: production BOUNDS.MAX_DIR_DEPTH/MAX_DIR_COUNT carry real headroom over this repo\'s measured largest crate (depth 3, ~100 directories)', () => {
  assert.ok(BOUNDS.MAX_DIR_DEPTH >= 64)
  assert.ok(BOUNDS.MAX_DIR_COUNT >= 10_000)
})
