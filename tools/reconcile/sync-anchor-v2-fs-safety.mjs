// sync-anchor-v2-fs-safety.mjs — path-traversal AND symlink/junction-escape containment, plus
// bounded (stat/open/fstat-capped) reads and incrementally-capped directory enumeration, for every
// filesystem target this tool reads that is influenceable by PR content: workspace members,
// Cargo.toml manifests, a crate's committed shard, its `.rs` source tree, and its crate doc.
//
// Split out as its own module (repair round 5) because it is the single trust boundary every other
// sync-anchor-v2-*.mjs read goes through: isPathWithinRoot alone (a lexical resolve()+relative()
// check) proves nothing about a SYMLINK/JUNCTION pointing outside root at any path component -- a
// round-5 audit's item 2 finding -- so every actual read in this tool now goes through
// `resolveWithinRoot`/`readBoundedWithinRoot`/`listDirEntriesBounded` instead of a bare
// existsSync+readFileSync pair.
import { closeSync, fstatSync, opendirSync, openSync, readSync, realpathSync, statSync } from 'node:fs'
import { isAbsolute, join, relative, resolve, sep } from 'node:path'

/** True only if `candidate` resolves to a path at or under `root`, by pure string/segment
 * comparison after `path.resolve()` -- no filesystem I/O, no symlink resolution. This is a cheap
 * FIRST-PASS filter (rejects an obvious `../../etc/passwd`-shaped traversal without touching disk),
 * never a complete containment proof on its own: a symlink at any path component can point outside
 * root while still lexically resolving under it. Real containment for an actual read always also
 * goes through `resolveWithinRoot`, which additionally resolves every symlink in the path. */
export function isPathWithinRoot(root, candidate) {
  const resolvedRoot = resolve(root)
  const resolvedCandidate = resolve(candidate)
  if (resolvedCandidate === resolvedRoot) return true
  const rel = relative(resolvedRoot, resolvedCandidate)
  return rel !== '' && rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel)
}

/**
 * Resolves EVERY symlink/junction in both `root` and `candidate` (via `realpathSync`, which walks
 * and resolves the full path, not just the final component) and verifies the fully-resolved
 * candidate is contained within the fully-resolved root. Fails closed on any error: a candidate
 * that does not exist, cannot be stat-ed, or resolves outside root are all refused identically from
 * the caller's point of view (a `{ ok: false }` result, never a thrown exception, never a silent
 * fallback to the unresolved lexical path).
 *
 * Returns `{ ok: true, realPath }` or `{ ok: false, reason, code }` where `reason` is one of
 * `NOT_FOUND` (candidate/root does not exist), `ESCAPES_ROOT` (resolves outside root -- the
 * symlink/junction-escape case), or `UNREADABLE` (any other stat/resolve failure, e.g. EACCES,
 * ELOOP for a symlink cycle).
 */
export function resolveWithinRoot(root, candidate) {
  if (!isPathWithinRoot(root, candidate)) {
    // An obvious lexical escape (e.g. a literal "../" in the path) is refused without ever
    // touching the filesystem for the candidate -- fail closed on the cheapest check first.
    return { ok: false, reason: 'ESCAPES_ROOT', code: null }
  }
  let resolvedRoot
  try {
    resolvedRoot = realpathSync(root)
  } catch (error) {
    return { ok: false, reason: 'UNREADABLE', code: error.code ?? 'UNKNOWN' }
  }
  let resolvedCandidate
  try {
    resolvedCandidate = realpathSync(candidate)
  } catch (error) {
    return { ok: false, reason: error.code === 'ENOENT' ? 'NOT_FOUND' : 'UNREADABLE', code: error.code ?? 'UNKNOWN' }
  }
  if (!isPathWithinRoot(resolvedRoot, resolvedCandidate)) {
    return { ok: false, reason: 'ESCAPES_ROOT', code: null }
  }
  return { ok: true, realPath: resolvedCandidate }
}

/**
 * Reads `path` as UTF-8 text, but only after (a) proving full symlink-resolved containment within
 * `root` via `resolveWithinRoot`, and (b) confirming the actual opened file descriptor refers to
 * that SAME validated real path (by comparing device+inode of the open fd's fstat against a fresh
 * stat of the resolved real path) -- this closes most of the TOCTOU window between the containment
 * check and the read itself: if a symlink is swapped after validation but before the read, the
 * opened file's identity will not match what was validated, and this fails closed rather than
 * silently reading whatever the swapped symlink now points to. Every read this tool performs on
 * PR-influenceable content (a manifest, a shard, a source file, a doc) goes through this one
 * function so the same bound/containment/error-shape guarantee applies everywhere.
 *
 * Size is checked via `fstat` BEFORE any byte is read -- an oversized file is refused with zero
 * content bytes ever loaded, exactly like the stat-first discipline SourceCache introduced in
 * repair round 3.
 *
 * Returns `{ ok: true, text }` or `{ ok: false, reason, code, size_bytes? }` where `reason` is one
 * of `NOT_FOUND`, `ESCAPES_ROOT`, `OVERSIZED` (exceeds `bound`; `size_bytes` carries the measured
 * size), `NOT_A_FILE` (a directory/device/etc. at that path), or `UNREADABLE` (open/stat/read
 * failure, `code` carries the OS errno when available).
 */
export function readBoundedWithinRoot(root, path, bound) {
  const containment = resolveWithinRoot(root, path)
  if (!containment.ok) return { ok: false, reason: containment.reason, code: containment.code }

  let fd
  try {
    fd = openSync(path, 'r')
  } catch (error) {
    return { ok: false, reason: error.code === 'ENOENT' ? 'NOT_FOUND' : 'UNREADABLE', code: error.code ?? 'UNKNOWN' }
  }
  let result
  try {
    const fdStat = fstatSync(fd)
    let expectedStat
    try {
      expectedStat = statSync(containment.realPath)
    } catch (error) {
      result = { ok: false, reason: 'UNREADABLE', code: error.code ?? 'UNKNOWN' }
      return result
    }
    if (fdStat.dev !== expectedStat.dev || fdStat.ino !== expectedStat.ino) {
      // The file identity changed between the containment check and this open -- a TOCTOU
      // symlink swap. Refuse rather than trust an fd that was never actually validated.
      result = { ok: false, reason: 'ESCAPES_ROOT', code: null }
      return result
    }
    if (!fdStat.isFile()) {
      result = { ok: false, reason: 'NOT_A_FILE', code: null }
      return result
    }
    if (fdStat.size > bound) {
      result = { ok: false, reason: 'OVERSIZED', code: null, size_bytes: fdStat.size }
      return result
    }

    const buf = Buffer.alloc(fdStat.size)
    let offset = 0
    while (offset < buf.length) {
      const n = readSync(fd, buf, offset, buf.length - offset, offset)
      if (n === 0) break
      offset += n
    }
    result = { ok: true, text: buf.toString('utf8', 0, offset) }
    return result
  } catch (error) {
    result = { ok: false, reason: 'UNREADABLE', code: error.code ?? 'UNKNOWN' }
    return result
  } finally {
    try {
      closeSync(fd)
    } catch (error) {
      // repair round 7, item 4: a bare `closeSync(fd)` here previously let a close-time OS error
      // (e.g. EIO) THROW straight out of this function -- a `throw` from a `finally` block
      // supersedes whatever the try/catch above already decided to return, silently turning a
      // structured `{ok:false,...}` contract into an uncaught exception. An explicit `return`
      // inside `finally` is the one construct that can override an already-queued return value
      // deliberately: if the read itself had succeeded, downgrade to a structured UNREADABLE
      // (never trust a descriptor whose lifecycle could not be fully accounted for -- fail closed,
      // exactly like the TOCTOU dev/ino check above); if the read had already failed for its own
      // reason, that original structured failure is left untouched.
      if (result?.ok) return { ok: false, reason: 'UNREADABLE', code: error.code ?? 'UNKNOWN' }
    }
  }
}

/**
 * Lists up to `bound` directory entries of `dir` via `fs.opendirSync`'s incremental iterator
 * (`dir.readSync()` one entry at a time), stopping the moment `bound` is reached -- unlike
 * `readdirSync`, which always materializes the FULL entry list into memory in one syscall before a
 * caller can bound/sort/filter anything, this never reads or holds more than `bound + 1` entries
 * regardless of how large the real directory is, so a pathological directory with millions of
 * entries cannot force unbounded memory work just to be listed.
 *
 * Never throws (repair round 6 correction: `opendirSync`/`readSync` can both fail mid-walk --
 * ENOENT if `dir` vanishes in a TOCTOU race, EACCES/EPERM on a permission-denied directory, ENOTDIR
 * if a path component turned into a file, EMFILE/ENFILE on fd exhaustion -- none of which were
 * previously caught, so any of them crashed the whole scan with an uncaught native exception
 * instead of the structured, value-free finding every other read failure in this tool produces).
 * Returns `{ entries, truncated, unreadable }`: `entries` are `fs.Dirent` objects in raw (not
 * sorted) directory order, always whatever was successfully read before any failure -- callers
 * that need deterministic order sort the (already-bounded) result themselves; `unreadable` is
 * `null` on full success or `{ code }` (the OS errno, never a raw error message) the moment
 * opendir/readdir itself failed -- callers must treat a non-null `unreadable` as fail-closed
 * (an INCOMPLETE listing, indistinguishable in cause from "empty" without this field), never as a
 * silently-successful empty/partial directory.
 */
export function listDirEntriesBounded(dir, bound) {
  let handle
  try {
    handle = opendirSync(dir)
  } catch (error) {
    return { entries: [], truncated: false, unreadable: { code: error.code ?? 'UNKNOWN' } }
  }
  const entries = []
  let truncated = false
  let unreadable = null
  try {
    let entry
    while ((entry = handle.readSync()) !== null) {
      if (entries.length >= bound) {
        truncated = true
        break
      }
      entries.push(entry)
    }
  } catch (error) {
    unreadable = { code: error.code ?? 'UNKNOWN' }
  } finally {
    try {
      handle.closeSync()
    } catch (error) {
      // repair round 7, item 4: previously silently discarded. If the read loop itself had
      // already failed, the handle is already known-broken and this adds no new information; but
      // if the read loop SUCCEEDED, a close-time failure must still downgrade the result to
      // unreadable rather than reporting a listing as clean when its handle could not be fully
      // accounted for -- fail closed, the same principle applied to readBoundedWithinRoot.
      if (unreadable == null) unreadable = { code: error.code ?? 'UNKNOWN' }
    }
  }
  return { entries, truncated, unreadable }
}
