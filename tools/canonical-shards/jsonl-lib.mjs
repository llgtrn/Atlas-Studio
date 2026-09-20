import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative, sep } from 'node:path'

export const CAPABILITY_CANONICAL_DIR = join('docs', 'capabilities-canonical')
export const ARCHITECTURE_CANONICAL_DIR = join('docs', 'architecture-canonical')

export const CAPABILITY_STATUSES = new Set([
  'unimplemented',
  'implemented_unverified',
  'verified',
  'blocked',
  'excluded',
  'doc_only',
  'tracking_only',
  'partial',
  'local_audit_required',
])

export function slash(path) {
  return String(path).split(sep).join('/')
}

export function rel(root, path) {
  return slash(relative(root, path))
}

export function parseArgs(argv) {
  const out = { _: [] }
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    if (!token.startsWith('--')) {
      out._.push(token)
      continue
    }
    const eq = token.indexOf('=')
    if (eq >= 0) {
      out[token.slice(2, eq)] = token.slice(eq + 1)
      continue
    }
    const key = token.slice(2)
    const next = argv[i + 1]
    if (next == null || next.startsWith('--')) {
      out[key] = true
      continue
    }
    out[key] = next
    i += 1
  }
  return out
}

export function cleanString(value) {
  if (value == null) return null
  const text = String(value).trim()
  return text.length ? text : null
}

export function cleanRequiredString(value, fallback = '') {
  return cleanString(value) ?? fallback
}

export function boolFromDb(value) {
  if (typeof value === 'boolean') return value
  if (typeof value === 'number') return value !== 0
  if (value == null) return false
  return /^(1|true|yes)$/i.test(String(value).trim())
}

export function intFromBool(value) {
  return value ? 1 : 0
}

export function normalizeArray(value) {
  if (value == null) return []
  if (Array.isArray(value)) return uniqueStrings(value)
  if (typeof value !== 'string') return uniqueStrings([String(value)])
  const trimmed = value.trim()
  if (!trimmed) return []
  if (trimmed.startsWith('[')) {
    try {
      const parsed = JSON.parse(trimmed)
      if (Array.isArray(parsed)) return uniqueStrings(parsed)
    } catch {
      // Fall through to loose text splitting.
    }
  }
  return uniqueStrings(trimmed.split(/\r?\n|[;,]/g))
}

export function uniqueStrings(values) {
  const seen = new Set()
  const out = []
  for (const value of values) {
    const text = cleanString(value)
    if (!text || seen.has(text)) continue
    seen.add(text)
    out.push(text)
  }
  return out
}

export function addUnique(target, values) {
  for (const value of normalizeArray(values)) {
    if (!target.includes(value)) target.push(value)
  }
}

export function sanitizeShardName(value) {
  return cleanRequiredString(value, 'unknown')
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'unknown'
}

export function stableJson(value, space = 2) {
  return JSON.stringify(sortJson(value), null, space)
}

function sortJson(value) {
  if (Array.isArray(value)) return value.map(sortJson)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, sortJson(value[key])]),
  )
}

export function writeJsonFile(path, value) {
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, `${stableJson(value)}\n`, 'utf8')
}

export function writeJsonl(path, records) {
  mkdirSync(dirname(path), { recursive: true })
  const body = records.map((record) => JSON.stringify(record)).join('\n')
  writeFileSync(path, body.length ? `${body}\n` : '', 'utf8')
}

export function emptyDirectory(path) {
  rmSync(path, { recursive: true, force: true })
  mkdirSync(path, { recursive: true })
}

export function listJsonlFiles(path) {
  if (!existsSync(path)) return []
  const out = []
  const walk = (dir) => {
    for (const ent of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, ent.name)
      if (ent.isDirectory()) {
        walk(full)
      } else if (ent.isFile() && ent.name.endsWith('.jsonl')) {
        out.push(full)
      }
    }
  }
  walk(path)
  return out.sort((a, b) => slash(a).localeCompare(slash(b)))
}

export function readJsonlFile(path) {
  const raw = readFileSync(path, 'utf8')
  const entries = []
  raw.split(/\r?\n/).forEach((line, index) => {
    if (!line.trim()) return
    try {
      entries.push({ record: JSON.parse(line), line: index + 1, file: path })
    } catch (error) {
      const parseError = new Error(`${slash(path)}:${index + 1}: invalid JSONL: ${error.message}`)
      parseError.code = 'JSONL_PARSE_ERROR'
      throw parseError
    }
  })
  return entries
}

export function loadJsonlTree(path) {
  const files = listJsonlFiles(path)
  const entries = []
  for (const file of files) entries.push(...readJsonlFile(file))
  return { files, entries }
}

export function makeTempDbPath(prefix, fileName) {
  const dir = mkdtempSync(join(tmpdir(), prefix))
  return { dir, path: join(dir, fileName) }
}

export function validateArrayOfStrings(errors, record, field, path) {
  if (!Array.isArray(record[field])) {
    errors.push(`${path}.${field} must be an array`)
    return []
  }
  const invalid = record[field].filter((item) => typeof item !== 'string' || item.trim().length === 0)
  if (invalid.length) errors.push(`${path}.${field} must contain only non-empty strings`)
  return record[field]
}

export function validateString(errors, record, field, path, { nullable = false } = {}) {
  const value = record[field]
  if (nullable && value == null) return null
  if (typeof value !== 'string' || value.trim().length === 0) {
    errors.push(`${path}.${field} must be a non-empty string`)
    return null
  }
  return value
}

export function validateBoolean(errors, record, field, path) {
  if (typeof record[field] !== 'boolean') errors.push(`${path}.${field} must be boolean`)
}

export function validateSorted(errors, entries, keyFn, label) {
  let previous = null
  for (const entry of entries) {
    const key = keyFn(entry.record)
    if (previous && key.localeCompare(previous) < 0) {
      errors.push(`${rel(process.cwd(), entry.file)}:${entry.line}: ${label} order is not deterministic`)
    }
    previous = key
  }
}

export function loadCapabilityShards(root = process.cwd()) {
  const shardRoot = join(root, CAPABILITY_CANONICAL_DIR, 'domains')
  const { files, entries } = loadJsonlTree(shardRoot)
  const metaPath = join(root, CAPABILITY_CANONICAL_DIR, 'meta.json')
  const meta = existsSync(metaPath) ? JSON.parse(readFileSync(metaPath, 'utf8')) : null
  return { root: shardRoot, metaPath, meta, files, entries }
}

export function loadArchitectureShards(root = process.cwd()) {
  const shardRoot = join(root, ARCHITECTURE_CANONICAL_DIR)
  const metaPath = join(shardRoot, 'meta.json')
  const meta = existsSync(metaPath) ? JSON.parse(readFileSync(metaPath, 'utf8')) : null
  const files = ['nodes.jsonl', 'links.jsonl', 'gaps.jsonl', 'evidence.jsonl']
    .map((name) => join(shardRoot, name))
    .filter((path) => existsSync(path))
  const entries = []
  for (const file of files) entries.push(...readJsonlFile(file))
  return { root: shardRoot, metaPath, meta, files, entries }
}
