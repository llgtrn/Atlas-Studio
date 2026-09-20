// sync-anchor-v2-shard.mjs — workspace/crate discovery, shard-file (`.chronica/sub-cap-arch.jsonl`)
// loading, crate-doc capability-status loading, and the source-file cache for the sync-anchor-v2
// atom parser (docs/doctrines/023-five-dimension-cloud-shard-contract.md).
//
// Split out of sync-anchor-v2-lib.mjs (repair round 2), then further split into sibling modules
// sync-anchor-v2-schema.mjs (record_type vocabulary + JSON schema validation),
// sync-anchor-v2-sanitize.mjs (bounds + redaction primitives), and sync-anchor-v2-fs-safety.mjs
// (path-traversal/symlink-containment safety + bounded reads + capped directory enumeration,
// repair round 5) -- to keep each module under this repo's ~500-line convention and to isolate
// every place that reads attacker-influenceable PR content (docs/doctrines/023a-sync-anchor-v2-repair-log.md
// has the full history). Everything in this file follows one rule: a finding derived from shard
// content is reported with a FIXED, small-vocabulary reason code plus bounded numeric metadata
// (line/column/count) only — never the raw offending text, never a native error's message string
// (which can echo fragments of the input), and never silently dropped or coerced into a
// different, more favorable classification (e.g. AGREE).
import { existsSync, readFileSync, statSync } from 'node:fs'
import { join, relative, sep } from 'node:path'
import { isPathWithinRoot, listDirEntriesBounded, readBoundedWithinRoot, resolveWithinRoot } from './sync-anchor-v2-fs-safety.mjs'
import { BOUNDS, boundedText, CAPABILITY_KEY_ALLOW_RE, comparePlain, escapeMarkdownCell, redactValue, sanitizeCrateName, sanitizeEnumField, sanitizeErrorForDisplay, sanitizePathField } from './sync-anchor-v2-sanitize.mjs'
import { extractParseErrorColumn, findDuplicateTopLevelKeys, RECOGNIZED_RECORD_TYPES, validateShardRecordShape } from './sync-anchor-v2-schema.mjs'

export { isPathWithinRoot, resolveWithinRoot } from './sync-anchor-v2-fs-safety.mjs'
export {
  BOUNDS,
  boundedText,
  CAPABILITY_KEY_ALLOW_RE,
  comparePlain,
  escapeMarkdownCell,
  redactValue,
  sanitizeCapabilityKey,
  sanitizeCrateName,
  sanitizeEnumField,
  sanitizeErrorForDisplay,
  sanitizePathField,
  sanitizeTargetModule,
} from './sync-anchor-v2-sanitize.mjs'
export { extractParseErrorColumn, findDuplicateTopLevelKeys, RECOGNIZED_RECORD_TYPES, validateShardRecordShape } from './sync-anchor-v2-schema.mjs'

const slash = (path) => path.split(sep).join('/')

// ── workspace / crate discovery (traversal- AND symlink-escape-safe) ───────────────────────────

function parseWorkspaceMembers(toml) {
  const match = toml.match(/members\s*=\s*\[([\s\S]*?)\]/m)
  if (!match) return []
  return [...match[1].matchAll(/"([^"]+)"/g)].map((m) => m[1])
}

function parsePackageName(toml) {
  return toml.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1] ?? null
}

/** Discovers workspace member crates from the root Cargo.toml. Every manifest read (root AND each
 * member) goes through `readBoundedWithinRoot`, which resolves symlinks/junctions at every path
 * component (not just a lexical resolve()) and refuses anything that escapes `root`, is oversized,
 * or is otherwise unreadable -- repair round 5, item 2 (symlink/junction containment) and item 4
 * (every manifest read is bound-checked, never a bare readFileSync). A member whose Cargo.toml
 * escapes root (lexically OR via a symlink) is skipped and reported in `escapedMembers`, never
 * followed/read; an existing-but-refused member manifest (oversized/unreadable) is reported in
 * `manifestFindings`, also never partially read. The crate list is always sorted by name for
 * deterministic output regardless of `members` array order or filesystem enumeration order. */
export function discoverWorkspaceCratesWithFindings(root) {
  const rootManifestPath = join(root, 'Cargo.toml')
  const rootManifest = readBoundedWithinRoot(root, rootManifestPath, BOUNDS.MAX_MANIFEST_BYTES)
  if (!rootManifest.ok) {
    const reason = rootManifest.reason === 'OVERSIZED' ? 'MANIFEST_OVERSIZED' : 'MANIFEST_UNREADABLE'
    return {
      crates: [],
      escapedMembers: [],
      manifestFindings: [{ member: '(root)', reason, ...(rootManifest.size_bytes != null ? { size_bytes: rootManifest.size_bytes, limit_bytes: BOUNDS.MAX_MANIFEST_BYTES } : {}) }],
    }
  }

  const escapedMembers = []
  const manifestFindings = []
  const crates = parseWorkspaceMembers(rootManifest.text)
    .map((member) => {
      const cargoPath = join(root, member, 'Cargo.toml')
      const containment = resolveWithinRoot(root, cargoPath)
      if (!containment.ok) {
        if (containment.reason === 'ESCAPES_ROOT') {
          escapedMembers.push(member)
        } else {
          // repair round 6: NOT_FOUND ("member declared but no Cargo.toml/directory present at
          // all") and UNREADABLE (a real stat/realpath failure resolving the member's own path,
          // e.g. EACCES, ELOOP) were previously silently skipped here with zero trace -- a PR that
          // declares a workspace member whose manifest cannot even be located/resolved dropped that
          // crate from coverage with no operational finding at all, indistinguishable from a
          // legitimately absent optional member. Both are now real, structured, non-suppressible
          // findings (fail-closed: the member is still excluded from `crates` -- it genuinely
          // cannot be verified -- but that exclusion is now visible instead of invisible).
          manifestFindings.push({ member, reason: 'WORKSPACE_MEMBER_UNRESOLVED', code: containment.code ?? containment.reason ?? 'UNKNOWN' })
        }
        return null
      }
      const manifest = readBoundedWithinRoot(root, cargoPath, BOUNDS.MAX_MANIFEST_BYTES)
      if (!manifest.ok) {
        if (manifest.reason === 'OVERSIZED') {
          manifestFindings.push({ member, reason: 'MANIFEST_OVERSIZED', size_bytes: manifest.size_bytes, limit_bytes: BOUNDS.MAX_MANIFEST_BYTES })
        } else if (manifest.reason !== 'NOT_FOUND') {
          manifestFindings.push({ member, reason: 'MANIFEST_UNREADABLE', code: manifest.code ?? 'UNKNOWN' })
        }
        return null
      }
      const name = parsePackageName(manifest.text)
      if (!name) return null
      return { name, crate_path: slash(member), abs_path: join(root, member), manifest_text: manifest.text }
    })
    .filter(Boolean)
    .sort((a, b) => comparePlain(a.name, b.name))
  return { crates, escapedMembers: escapedMembers.sort(), manifestFindings: manifestFindings.sort((a, b) => comparePlain(a.member, b.member)) }
}

/** Convenience wrapper over discoverWorkspaceCratesWithFindings for callers (most of this
 * codebase, and every pre-existing test) that only need the crate list. */
export function discoverWorkspaceCrates(root) {
  return discoverWorkspaceCratesWithFindings(root).crates
}

// ── rust source scanning (sorted, size-bounded, cached) ─────────────────────────────────────

/** Lists every `.rs` file under `dir`, sorted for deterministic traversal (readdirSync order is
 * filesystem-dependent, not guaranteed sorted), bounded to BOUNDS.MAX_SOURCE_FILES_PER_CRATE so a
 * pathological source tree cannot force unbounded work. Each directory level is enumerated via
 * `listDirEntriesBounded` (an incrementally-capped `opendirSync` walk, never an unbounded
 * `readdirSync`+sort) at `BOUNDS.MAX_DIR_ENTRIES_PER_LEVEL` -- a single directory with an
 * enormous entry count cannot force unbounded memory/CPU just to be listed, independent of the
 * total-.rs-files bound. A symlinked entry (file or directory) is only followed after its
 * FULLY-RESOLVED real path is proven contained within `dir` itself (`resolveWithinRoot`, repair
 * round 5 item 2) -- an entry whose symlink escapes the crate's own source tree is refused and
 * recorded in `unsafeEntries`, never silently followed and never silently dropped.
 *
 * Repair round 7, item 4 hardening: (1) recursion depth (`BOUNDS.MAX_DIR_DEPTH`) and total
 * directory count (`BOUNDS.MAX_DIR_COUNT`) are both bounded independent of the file-count bound --
 * a pathologically deep or wide but file-SPARSE tree could previously recurse/loop without ever
 * tripping the file-count cap; (2) every symlinked DIRECTORY's fully-resolved real path is tracked
 * in a visited set -- a directory-symlink CYCLE (e.g. `a/loop -> a`, still fully CONTAINED within
 * `dir`, so `resolveWithinRoot`'s containment check alone does not catch it) is refused and
 * recorded in `unsafeEntries` the moment it would be revisited, instead of recursing forever;
 * (3) a symlink `statSync` failure (a broken symlink, a permission error) is now a structured
 * `unsafeEntries` finding, never a silent `continue` with zero trace. */
export function listRustFiles(dir, bound = BOUNDS.MAX_SOURCE_FILES_PER_CRATE, maxDepth = BOUNDS.MAX_DIR_DEPTH, maxDirCount = BOUNDS.MAX_DIR_COUNT) {
  if (!existsSync(dir)) return { files: [], truncated: false, unsafeEntries: [], unreadableDirs: [] }
  const out = []
  const unsafeEntries = []
  const unreadableDirs = []
  const visitedRealDirs = new Set()
  let truncated = false
  let dirEntriesTruncated = false
  let dirCount = 0
  const walk = (p, depth) => {
    if (truncated) return
    dirCount += 1
    if (depth > maxDepth || dirCount > maxDirCount) {
      truncated = true
      return
    }
    const { entries: rawEntries, truncated: levelTruncated, unreadable } = listDirEntriesBounded(p, BOUNDS.MAX_DIR_ENTRIES_PER_LEVEL)
    if (levelTruncated) dirEntriesTruncated = true
    // A directory level that itself could not be enumerated (opendir/readdir failure, not a
    // symlink escape or the entry-count cap) means the listing below `p` is unprovably incomplete
    // -- exactly like hitting the file-count bound, this makes the WHOLE crate's file listing
    // untrustworthy, so the walk stops here rather than silently treating `p` as empty.
    if (unreadable) {
      unreadableDirs.push({ path: p, code: unreadable.code })
      truncated = true
      return
    }
    const entries = rawEntries.sort((a, b) => comparePlain(a.name, b.name))
    for (const entry of entries) {
      if (truncated) return
      const full = join(p, entry.name)
      let isDir = entry.isDirectory()
      let isFile = entry.isFile()
      if (entry.isSymbolicLink()) {
        const containment = resolveWithinRoot(dir, full)
        if (!containment.ok) {
          unsafeEntries.push({ path: full, reason: containment.reason })
          continue
        }
        let real
        try {
          real = statSync(full)
        } catch (error) {
          unsafeEntries.push({ path: full, reason: 'STAT_FAILED', code: error.code ?? 'UNKNOWN' })
          continue
        }
        isDir = real.isDirectory()
        isFile = real.isFile()
        if (isDir) {
          if (visitedRealDirs.has(containment.realPath)) {
            unsafeEntries.push({ path: full, reason: 'SYMLINK_CYCLE' })
            continue
          }
          visitedRealDirs.add(containment.realPath)
        }
      }
      if (isDir) {
        walk(full, depth + 1)
      } else if (isFile && entry.name.endsWith('.rs')) {
        if (out.length >= bound) {
          truncated = true
          return
        }
        out.push(full)
      }
    }
  }
  walk(dir, 0)
  return { files: out, truncated: truncated || dirEntriesTruncated, unsafeEntries, unreadableDirs }
}

/**
 * A small read-through cache so a crate's source files are read from disk at most once per
 * verification pass, regardless of how many capabilities/checks re-read the same file (previously
 * classifyFileBody/isModuleReferencedElsewhere/hasTestEvidence each re-read every candidate file
 * per capability -- O(capabilities x files) reads on a crate with many capabilities).
 *
 * Reads are GENUINELY bounded (repair round 3, item 1): the file is `stat`-ed FIRST, and a file
 * whose size exceeds `bound` is never read at all -- not even a truncated prefix.
 *
 * When constructed with a `root`, every read also goes through `readBoundedWithinRoot` (repair
 * round 5, item 2): a symlinked source file/directory whose fully-resolved real path escapes
 * `root` is refused exactly like an oversized one, never silently followed. `root` is optional
 * (defaults to `null`, skipping containment) ONLY so tests that read an isolated fixture file
 * directly, with no crate-root concept, can still construct a cache -- every production call site
 * in this tool always passes a real root.
 *
 * A read that FAILS for any reason (oversized, escapes root, or a genuine stat/open/read error --
 * e.g. a broken symlink, a permissions error) is recorded in `unreadableFiles` (repair round 5,
 * item 3: a prior version silently returned '' for a stat/read error with no trace at all, which
 * is indistinguishable from "the file is genuinely empty" -- this cache never does that again).
 * `oversizedFiles` is kept as a distinct, separately-reported bucket since it has its own
 * pre-existing SOURCE_FILE_OVERSIZED reason code; every OTHER failure (unreadable, escapes root)
 * lands in `unreadableFiles` so the caller can surface an explicit SOURCE_FILE_UNREADABLE /
 * SOURCE_PATH_ESCAPES_ROOT operational finding instead of a silent empty string.
 */
export class SourceCache {
  constructor({ bound = BOUNDS.MAX_SOURCE_FILE_BYTES, root = null } = {}) {
    this.bound = bound
    this.root = root
    this.cache = new Map()
    this.oversizedFiles = []
    this.unreadableFiles = []
  }

  read(absPath) {
    if (this.cache.has(absPath)) return this.cache.get(absPath)
    const text = this.readBounded(absPath)
    this.cache.set(absPath, text)
    return text
  }

  readBounded(absPath) {
    if (this.root == null) return this.readBoundedNoRoot(absPath)
    const result = readBoundedWithinRoot(this.root, absPath, this.bound)
    if (result.ok) return result.text
    if (result.reason === 'OVERSIZED') {
      this.oversizedFiles.push({ absPath, size_bytes: result.size_bytes, limit_bytes: this.bound })
    } else {
      // Every caller in this tool only ever calls SourceCache.read() on a path that was already
      // established to exist -- a listRustFiles/readdir entry, or an existsSync-checked mod-decl
      // target -- so NOT_FOUND here means a genuine TOCTOU race or a broken symlink (its final
      // target vanished), not a speculative probe; it is reported exactly like ESCAPES_ROOT
      // (a symlink/junction resolving outside the crate's own source tree), UNREADABLE
      // (permissions, any other OS error), or NOT_A_FILE (a directory/device where a file was
      // expected) -- every one of these is a real, distinct failure that must be visible, never
      // folded into a silent '' with no trace (repair round 5, item 3).
      this.unreadableFiles.push({ absPath, reason: result.reason, code: result.code ?? null })
    }
    return ''
  }

  // Used only when this cache was constructed with no `root` (an isolated fixture read with no
  // crate-root concept, e.g. a narrow unit test) -- same stat-first/bounded discipline, but no
  // symlink/containment check, since there is no root to contain against.
  readBoundedNoRoot(absPath) {
    let size
    try {
      size = statSync(absPath).size
    } catch (error) {
      this.unreadableFiles.push({ absPath, reason: 'UNREADABLE', code: error.code ?? 'UNKNOWN' })
      return ''
    }
    if (size > this.bound) {
      this.oversizedFiles.push({ absPath, size_bytes: size, limit_bytes: this.bound })
      return ''
    }
    try {
      return readFileSync(absPath, 'utf8')
    } catch (error) {
      this.unreadableFiles.push({ absPath, reason: 'UNREADABLE', code: error.code ?? 'UNKNOWN' })
      return ''
    }
  }
}

// ── shard loading ────────────────────────────────────────────────────────────────────────────

/**
 * Loads and validates a crate's committed shard.
 *
 * Three kinds of finding are collected, never thrown, and never silently dropped:
 *   - `parseErrors`  -- a line is not even syntactically valid JSON (SHARD_PARSE_ERROR).
 *   - `recordErrors` -- a line parses as JSON but fails the shard record schema, including the
 *                       duplicate-top-level-key case (SHARD_RECORD_INVALID).
 *   - `duplicateCapabilityKeys` -- two or more otherwise-valid capability rows share the same
 *     `capability_key` (SHARD_CAPABILITY_KEY_DUPLICATE). Rows involved in a duplicate are
 *     EXCLUDED from `capabilities` (there is no principled way to pick a "winner"), so a
 *     duplicated capability_key can never silently resolve to AGREE for one arbitrary copy of
 *     itself -- it is reported explicitly instead.
 *
 * The raw file itself is bounded (BOUNDS.MAX_SHARD_BYTES / MAX_SHARD_LINES / MAX_LINE_LENGTH): an
 * oversized shard produces a single deterministic `oversized` finding instead of unbounded work.
 */
export function loadCrateShard(root, crate) {
  const path = join(root, crate.crate_path, '.chronica', 'sub-cap-arch.jsonl')
  const relPath = slash(relative(root, path))
  const empty = {
    path: relPath,
    exists: false,
    capabilities: [],
    parseErrors: [],
    recordErrors: [],
    duplicateCapabilityKeys: [],
    oversized: null,
    unreadable: null,
  }
  if (!existsSync(path)) return empty

  // readBoundedWithinRoot resolves every symlink/junction in `path` (not just a lexical check) and
  // refuses to read anything that escapes `root`, is oversized, or is otherwise unreadable --
  // repair round 5, item 2. `reason` distinguishes ESCAPES_ROOT from a genuine OS error/oversize so
  // the finding stays structured rather than a single generic "could not read" bucket.
  const read = readBoundedWithinRoot(root, path, BOUNDS.MAX_SHARD_BYTES)
  if (!read.ok) {
    if (read.reason === 'OVERSIZED') {
      return { ...empty, exists: true, oversized: { limit_bytes: BOUNDS.MAX_SHARD_BYTES, actual_bytes: read.size_bytes } }
    }
    return { ...empty, exists: true, unreadable: { code: read.code ?? 'UNKNOWN', reason: read.reason } }
  }

  const rawLines = read.text.split(/\r?\n/)
  if (rawLines.length > BOUNDS.MAX_SHARD_LINES) {
    return { ...empty, exists: true, oversized: { limit_lines: BOUNDS.MAX_SHARD_LINES, actual_lines: rawLines.length } }
  }

  const rows = []
  const parseErrors = []
  const recordErrors = []

  rawLines.forEach((rawLine, index) => {
    if (!rawLine) return
    const line = index + 1
    if (rawLine.length > BOUNDS.MAX_LINE_LENGTH) {
      recordErrors.push({ line, detail: 'LINE_TOO_LONG' })
      return
    }
    let parsed
    try {
      parsed = JSON.parse(rawLine)
    } catch (error) {
      parseErrors.push({ line, column: extractParseErrorColumn(error.message, rawLine.length) })
      return
    }
    const shape = validateShardRecordShape(parsed, rawLine)
    if (!shape.valid) {
      recordErrors.push({ line, detail: shape.detail })
      return
    }
    rows.push({ line, row: parsed })
  })

  const capabilityRows = rows.filter((r) => r.row.record_type === 'capability')
  const byKey = new Map()
  for (const entry of capabilityRows) {
    const key = entry.row.capability_key
    if (!byKey.has(key)) byKey.set(key, [])
    byKey.get(key).push(entry.line)
  }
  const duplicateCapabilityKeys = [...byKey.entries()]
    .filter(([, lines]) => lines.length > 1)
    .map(([key, lines]) => ({ capability_key: key, lines: [...lines].sort((a, b) => a - b) }))
    .sort((a, b) => comparePlain(a.capability_key, b.capability_key))
  const duplicatedKeys = new Set(duplicateCapabilityKeys.map((d) => d.capability_key))

  const capabilityByKey = new Map()
  for (const entry of capabilityRows) {
    if (!duplicatedKeys.has(entry.row.capability_key)) capabilityByKey.set(entry.row.capability_key, { ...entry.row })
  }

  for (const entry of rows) {
    if (entry.row.record_type !== 'capability_status_correction') continue
    const capability = capabilityByKey.get(entry.row.capability_key)
    if (capability) capability.status = entry.row.new_status
  }

  const capabilities = capabilityRows
    .filter((entry) => !duplicatedKeys.has(entry.row.capability_key))
    .map((entry) => capabilityByKey.get(entry.row.capability_key))

  return {
    path: relPath,
    exists: true,
    capabilities,
    parseErrors,
    recordErrors,
    duplicateCapabilityKeys,
    oversized: null,
    unreadable: null,
  }
}

// ── crate doc capability-status loading ─────────────────────────────────────────────────────

const KNOWN_DOC_STATUSES = new Set(['verified', 'implemented_unverified', 'unverified', 'implemented', 'unimplemented'])

function normalizeDocStatusText(text) {
  const value = String(text ?? '').trim().toLowerCase()
  if (value === 'verified') return 'verified'
  if (value === 'implemented_unverified' || value === 'unverified') return 'implemented_unverified'
  if (value === 'implemented') return 'implemented'
  if (value === 'unimplemented') return 'unimplemented'
  return null
}

/** Parses the `## N. capabilities.db row(s)` table generated by `gen-crate-docs.mjs`
 * (`| Key | Status | Money | Side effect | Target crate | Target module | Acceptance test |`).
 * A status cell that is missing, empty, or does not match one of the recognized status words is
 * recorded in `invalidStatuses` (raw text bounded/never propagated further than that map's own
 * value) instead of being silently normalized to `unimplemented` -- the prior behavior, which
 * made an unrecognized/garbage doc status indistinguishable from a real, honest "unimplemented"
 * claim. `statuses` therefore only ever contains one of the four real STATUS enum values.
 *
 * The `docs/crates/` directory is enumerated via `listDirEntriesBounded` (incrementally capped
 * at `BOUNDS.MAX_DOC_DIR_ENTRIES`, never an unbounded `readdirSync`+sort -- repair round 5, item
 * 4) and the matched doc file itself is read via `readBoundedWithinRoot` (symlink-safe,
 * size-bounded -- items 2 and 4). Every failure mode is reported in `docFinding`, never silently
 * swallowed: a truncated directory listing could hide the real doc file behind the cap
 * (`DOC_DIR_TRUNCATED`), an `opendir`/`readdir` failure on `docs/crates/` itself is reported too
 * (`DIR_ENUMERATION_UNREADABLE`, repair round 6 -- previously this case fell through
 * `listDirEntriesBounded`'s uncaught throw), and an oversized/unreadable/escaping doc file is
 * refused exactly like a shard is. */
export function loadCrateDocCapabilityStatus(root, crateName) {
  // crate atlas docs are machine-generated and live under docs/crates/, not docs/
  // directly (see tools/_paths.mjs).
  const docsDir = join(root, 'docs', 'crates')
  if (!existsSync(docsDir)) return { path: null, statuses: new Map(), invalidStatuses: new Map(), docFinding: null }

  const { entries: rawEntries, truncated, unreadable } = listDirEntriesBounded(docsDir, BOUNDS.MAX_DOC_DIR_ENTRIES)
  if (unreadable) {
    return { path: null, statuses: new Map(), invalidStatuses: new Map(), docFinding: { reason: 'DIR_ENUMERATION_UNREADABLE', path: 'docs/crates', code: unreadable.code } }
  }
  const entries = rawEntries.map((e) => e.name).sort()
  const fileName = entries.find((f) => new RegExp(`^\\d{3}-crate-${crateName}\\.md$`).test(f))
  if (!fileName) {
    const docFinding = truncated ? { reason: 'DOC_DIR_TRUNCATED', entry_limit: BOUNDS.MAX_DOC_DIR_ENTRIES } : null
    return { path: null, statuses: new Map(), invalidStatuses: new Map(), docFinding }
  }

  const docPath = join(docsDir, fileName)
  const relPath = slash(relative(root, docPath))
  const read = readBoundedWithinRoot(root, docPath, BOUNDS.MAX_DOC_BYTES)
  if (!read.ok) {
    const reason = read.reason === 'OVERSIZED' ? 'DOC_OVERSIZED' : 'DOC_UNREADABLE'
    const docFinding =
      reason === 'DOC_OVERSIZED'
        ? { reason, path: relPath, size_bytes: read.size_bytes, limit_bytes: BOUNDS.MAX_DOC_BYTES }
        : { reason, path: relPath, code: read.code ?? 'UNKNOWN' }
    return { path: relPath, statuses: new Map(), invalidStatuses: new Map(), docFinding }
  }

  const statuses = new Map()
  const invalidStatuses = new Map()
  for (const line of read.text.split(/\r?\n/)) {
    if (!line.startsWith('| ')) continue
    const cells = line.split('|').slice(1, -1).map((cell) => cell.trim())
    if (cells.length < 6) continue
    const [key, statusText] = cells
    if (!CAPABILITY_KEY_ALLOW_RE.test(key)) continue
    const normalized = normalizeDocStatusText(statusText)
    if (normalized) statuses.set(key, normalized)
    else invalidStatuses.set(key, boundedText(statusText, BOUNDS.MAX_TEXT_FIELD_LENGTH))
  }
  return { path: relPath, statuses, invalidStatuses, docFinding: null }
}

export const _internal = { KNOWN_DOC_STATUSES, normalizeDocStatusText }
