// sync-anchor-v2-schema.mjs — shard record_type vocabulary, JSON schema validation (including
// fail-closed duplicate-top-level-key detection), and bounded JSON.parse error diagnostics for the
// sync-anchor-v2 atom parser.
//
// Split out of sync-anchor-v2-shard.mjs (repair round 5) purely to keep both files under this
// repo's ~500-line file-size convention -- this module answers "is this one already-parsed row
// schema-valid", independent of how/where the raw shard bytes were read from disk.
import { BOUNDS } from './sync-anchor-v2-sanitize.mjs'

// The full, authoritative record_type vocabulary a crate shard can carry, taken from
// tools/subdb/subdb-lib.mjs (the shard-generation tool itself, `recordKey`/`buildRecords`) and
// independently confirmed against every record_type value actually present in every crate's
// checked-in shard as of the current main tip (`meta` x80, `capability` x2161,
// `architecture_node` x2709, `architecture_edge` x3210, `capability_architecture_link` x3150,
// `architecture_target_gap` x3, `architecture_evidence` x2534 -- 13847 total records).
// `architecture_evidence` was added to every crate shard by the subdb:gen refresh that closed the
// 0-evidence-rows gap (see docs/architecture-canonical/evidence.jsonl sharding in
// tools/subdb/subdb-lib.mjs's architectureEvidenceRows()) -- before that refresh every shard had
// zero such rows, so this vocabulary gap was real but untriggered until real data exercised it.
// `capability_status_correction` is the append-only overlay record written by
// subdb-apply-agreed-corrections.mjs. Only `capability` rows and correction rows affect this
// parser's classification; the other six types are recognized-but-ignored structural content, not
// errors -- an EARLIER version of this file only recognized `capability`/`meta` and would have
// wrongly flagged over 9000 real architecture/link/gap rows as SHARD_RECORD_INVALID.
export const RECOGNIZED_RECORD_TYPES = new Set([
  'meta',
  'capability',
  'capability_status_correction',
  'architecture_node',
  'architecture_edge',
  'capability_architecture_link',
  'architecture_target_gap',
  'architecture_evidence',
])

const REQUIRED_CAPABILITY_STRING_FIELDS = ['capability_key', 'target_module', 'status']
const REQUIRED_CORRECTION_STRING_FIELDS = ['capability_key', 'previous_status', 'new_status', 'reason_code', 'match_confidence']

function isPlainObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

// Decodes JSON string escape sequences (`\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`)
// in a raw (still-escaped) key-name substring into the actual string it represents. `rawKeyText`
// is always a slice of a line that has ALREADY passed a real `JSON.parse` call by the time this
// runs (findDuplicateTopLevelKeys is only ever invoked from validateShardRecordShape, itself only
// reached after loadCrateShard's own `JSON.parse(rawLine)` succeeded) -- so every escape sequence
// here is guaranteed well-formed per JSON's own grammar; this is a narrow, bounded, single-purpose
// re-decode of one already-validated substring, never a second untrusted-input JSON.parse call
// over the whole line, and its result is used ONLY for equality comparison (never displayed --
// validateShardRecordShape's only caller checks `.length`, discarding the actual key text).
function decodeJsonKeyText(rawKeyText) {
  let out = ''
  let i = 0
  while (i < rawKeyText.length) {
    const ch = rawKeyText[i]
    if (ch !== '\\') {
      out += ch
      i += 1
      continue
    }
    const next = rawKeyText[i + 1]
    if (next === 'u') {
      const hex = rawKeyText.slice(i + 2, i + 6)
      out += String.fromCharCode(parseInt(hex, 16))
      i += 6
      continue
    }
    const simple = { '"': '"', '\\': '\\', '/': '/', b: '\b', f: '\f', n: '\n', r: '\r', t: '\t' }[next]
    // `simple` is always defined here given already-valid JSON input; the fallback exists only so
    // a defensive caller change can never turn this into a thrown exception.
    out += simple ?? next ?? ''
    i += 2
  }
  return out
}

/** Detects a top-level (depth-1, i.e. directly inside the outermost `{...}`) JSON object key that
 * appears more than once on one line -- after JSON string escape DECODING, so a key spelled once
 * as the literal text `capability_key` and a second time with, say, the `_` written as the escape
 * sequence backslash-u-0-0-5-f (four hex digits for code point U+005F) is still recognized as the
 * SAME key, not silently treated as two distinct ones (repair round 7, item 2: a raw-byte-text
 * comparison alone -- this function's round-5/6 behavior -- can be defeated by encoding just one
 * occurrence with a Unicode escape that decodes to the same characters, letting a genuine
 * last-value-wins ambiguity slip past undetected). JSON.parse silently applies "last value wins"
 * for duplicate keys (an ambiguous, ECMA-262-defined-but-surprising behavior) -- this scan runs
 * on the raw text BEFORE
 * that information is lost, so the ambiguity itself can be reported and the record treated as
 * SHARD_RECORD_INVALID (fail-closed: never guess which of two conflicting values was meant). This
 * is a bracket-depth-aware scan, not a full JSON grammar validator -- sufficient for the flat
 * capability/meta record shape this shard format actually uses. */
export function findDuplicateTopLevelKeys(line) {
  const seen = new Set()
  const dupes = new Set()
  let depth = 0
  let inString = false
  let escape = false
  let i = 0
  const len = Math.min(line.length, BOUNDS.MAX_LINE_LENGTH)
  while (i < len) {
    const ch = line[i]
    if (inString) {
      if (escape) escape = false
      else if (ch === '\\') escape = true
      else if (ch === '"') inString = false
      i += 1
      continue
    }
    if (ch === '"') {
      const start = i + 1
      let j = start
      let localEscape = false
      while (j < len) {
        const c = line[j]
        if (localEscape) localEscape = false
        else if (c === '\\') localEscape = true
        else if (c === '"') break
        j += 1
      }
      const value = decodeJsonKeyText(line.slice(start, j))
      let k = j + 1
      while (k < len && /\s/.test(line[k])) k += 1
      if (depth === 1 && line[k] === ':') {
        if (seen.has(value)) dupes.add(value)
        seen.add(value)
      }
      i = j + 1
      continue
    }
    if (ch === '{' || ch === '[') depth += 1
    else if (ch === '}' || ch === ']') depth -= 1
    i += 1
  }
  return [...dupes].sort()
}

/** Validates one already-JSON-parsed row against the shard schema. Returns `{ valid: true }` or
 * `{ valid: false, detail }` with `detail` drawn from a small fixed vocabulary -- never an
 * echo of the row's own content. A `capability`-typed row additionally requires
 * capability_key/target_module/status to all be present and be strings (a TYPE check only --
 * an empty string passes here and is a separate, existing semantic concern handled downstream by
 * classifyCapability's enum validation, so this never re-classifies previously-valid empty-string
 * inputs). */
export function validateShardRecordShape(row, rawLine) {
  if (!isPlainObject(row)) return { valid: false, detail: 'NOT_AN_OBJECT' }

  const dupeKeys = findDuplicateTopLevelKeys(rawLine)
  if (dupeKeys.length) return { valid: false, detail: 'DUPLICATE_JSON_OBJECT_KEY' }

  if (typeof row.record_type !== 'string' || !RECOGNIZED_RECORD_TYPES.has(row.record_type)) {
    return { valid: false, detail: 'UNRECOGNIZED_RECORD_TYPE' }
  }

  if (row.record_type === 'capability') {
    for (const field of REQUIRED_CAPABILITY_STRING_FIELDS) {
      if (typeof row[field] !== 'string') {
        return { valid: false, detail: `MISSING_OR_NON_STRING_FIELD:${field}` }
      }
    }
  }

  if (row.record_type === 'capability_status_correction') {
    for (const field of REQUIRED_CORRECTION_STRING_FIELDS) {
      if (typeof row[field] !== 'string') {
        return { valid: false, detail: `MISSING_OR_NON_STRING_FIELD:${field}` }
      }
    }
  }

  return { valid: true }
}

// ── malformed-JSON parse-error diagnostics (bounded, never the raw message) ────────────────────

/** Extracts only a bounded numeric column from a native JSON.parse SyntaxError message, and
 * discards the message text itself -- V8's message can echo a fragment of the offending input
 * (e.g. "Unexpected token o in JSON..."), which this module's contract forbids exposing. Returns
 * null if no column/position is present in the message (still safe: no text is ever kept). */
export function extractParseErrorColumn(message, lineLength) {
  const columnMatch = /column (\d+)/.exec(String(message ?? ''))
  if (columnMatch) return Math.max(0, Math.min(Number(columnMatch[1]), lineLength))
  const positionMatch = /position (\d+)/.exec(String(message ?? ''))
  if (positionMatch) return Math.max(0, Math.min(Number(positionMatch[1]) + 1, lineLength))
  return null
}

export const _internal = { isPlainObject }
