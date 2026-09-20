import { readdirSync, readFileSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { asArray, isObject, text } from './util.mjs'
import { RECORD_TYPES } from './contract-vocab.mjs'
import { validateContractRecord } from './contract-validate.mjs'

export function defaultSourceDir(root) {
  return join(root, 'tools/benchmark/contracts/source')
}

export function defaultGeneratedDir(root) {
  return join(root, 'tools/benchmark/contracts/generated')
}

export function engineRoot() {
  return resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..')
}

function parseJsonl(raw, file) {
  const rows = []
  const errors = []
  raw.split(/\r?\n/).forEach((line, index) => {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) return
    try {
      rows.push(JSON.parse(trimmed))
    } catch (error) {
      errors.push(`${file}:${index + 1}: ${error.message}`)
    }
  })
  return { rows, errors }
}

export function loadJsonFile(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

export function loadContractSources(sourceDir, ctx = {}) {
  const errors = []
  const records = []
  if (!existsSync(sourceDir)) return { ok: false, errors: [`missing source dir ${sourceDir}`], records: [] }
  const files = readdirSync(sourceDir)
    .filter((name) => name.endsWith('.jsonl') || name.endsWith('.json'))
    .sort()
  for (const name of files) {
    if (name === 'path-routing.json' || name === 'standards.json') continue
    const path = join(sourceDir, name)
    const raw = readFileSync(path, 'utf8')
    if (name.endsWith('.jsonl')) {
      const parsed = parseJsonl(raw, `source/${name}`)
      errors.push(...parsed.errors)
      records.push(...parsed.rows)
    } else {
      const body = JSON.parse(raw)
      if (Array.isArray(body)) records.push(...body)
      else if (isObject(body) && RECORD_TYPES.includes(text(body.type))) records.push(body)
    }
  }

  const routing = existsSync(join(sourceDir, 'path-routing.json'))
    ? loadJsonFile(join(sourceDir, 'path-routing.json'))
    : { prefixes: [] }
  const standards = existsSync(join(sourceDir, 'standards.json'))
    ? loadJsonFile(join(sourceDir, 'standards.json'))
    : {}

  for (const record of records) {
    const result = validateContractRecord(record, ctx)
    if (!result.ok) errors.push(...result.errors)
  }

  const byType = Object.fromEntries(RECORD_TYPES.map((type) => [type, []]))
  for (const record of records) {
    const type = text(record.type)
    if (byType[type]) byType[type].push(record)
  }

  return { ok: errors.length === 0, errors, records, byType, routing, standards, sourceDir }
}

export function groupById(records, key = 'id') {
  const map = new Map()
  const dupes = []
  for (const record of asArray(records)) {
    const id = text(record[key]) || text(record.cap)
    if (!id) continue
    if (map.has(id)) dupes.push(id)
    else map.set(id, record)
  }
  return { map, dupes }
}
