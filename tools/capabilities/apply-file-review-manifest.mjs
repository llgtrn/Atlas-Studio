#!/usr/bin/env node
// Apply an audited donor file-review manifest to docs/capabilities.db.
// The manifest is read from stdin so long reviewer payloads do not hit Windows
// command-line length limits.
import Database from 'better-sqlite3'
import { createHash } from 'node:crypto'
import { readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'

const input = await new Promise((resolve) => {
  let data = ''
  process.stdin.setEncoding('utf8')
  process.stdin.on('data', (chunk) => { data += chunk })
  process.stdin.on('end', () => resolve(data))
})

const manifest = JSON.parse(input)
const db = new Database(join(process.cwd(), 'docs', 'capabilities.db'))
const now = manifest.reviewed_at || new Date().toISOString()
const reviewedBy = manifest.reviewed_by || 'codex:file-exhaustive-review'
const createdPass = manifest.created_pass || 'file-exhaustive-review-2026-06-03'
const donor = manifest.donor
if (!donor) throw new Error('manifest.donor is required')

const asArray = (value) => {
  if (value === undefined || value === null || value === '') return []
  return Array.isArray(value) ? value : [value]
}

function mappedIds(row) {
  return [
    ...asArray(row.ids),
    ...asArray(row.mapped_source_ids),
    ...asArray(row.mapped_source_keys),
  ]
}

function expandFileRows(rows = []) {
  return rows.flatMap((row) => {
    const paths = row.paths || (row.path ? [row.path] : [])
    if (!paths.length) return []
    return paths.map((path) => ({
      ...row,
      path,
      ids: mappedIds(row),
      evidence: row.evidence || row.evidence_note || row.reason || '',
    }))
  })
}

const sourceRows = manifest.sources || manifest.source_inserts || []
const ensureFileRows = expandFileRows(manifest.ensure_files || [])
const reviewRows = expandFileRows(manifest.reviews || manifest.files || [])

const readStatus = (classification, read = true) =>
  classification === 'generated_vendor_build_artifact'
    ? 'classified_not_read'
    : (read ? 'reviewed' : 'classified_not_read')

const donorPath = (rel) => join(process.cwd(), 'Temporary', donor, ...rel.split('/'))

function fileInfo(rel) {
  const abs = donorPath(rel)
  const st = statSync(abs)
  const data = readFileSync(abs)
  return {
    size: st.size,
    mtime: Math.trunc(st.mtimeMs),
    sha: createHash('sha256').update(data).digest('hex'),
  }
}

function canonicalTarget(canonicalId) {
  return db.prepare('SELECT target_crate,target_module FROM canonical_capability WHERE id=?').get(canonicalId) || {}
}

function ensureSource(row) {
  const existing = db.prepare('SELECT id FROM source_capability WHERE donor=? AND canonical_name=?')
    .get(donor, row.canonical_name)
  if (existing) return existing.id
  const c = canonicalTarget(row.canonical_id)
  const info = db.prepare(`INSERT INTO source_capability (
    donor,module,canonical_name,source_files,source_symbols,business_behavior,technical_behavior,
    inputs,outputs,persistence,surface,side_effect_class,moves_money,requires_approval,external_services,
    target_crate,target_module,canonical_id,created_pass
  ) VALUES (@donor,@module,@canonical_name,@source_files,@source_symbols,@business_behavior,@technical_behavior,
    @inputs,@outputs,@persistence,@surface,@side_effect_class,@moves_money,@requires_approval,@external_services,
    @target_crate,@target_module,@canonical_id,@created_pass)`).run({
      donor,
      module: row.module || null,
      canonical_name: row.canonical_name,
      source_files: row.source_files,
      source_symbols: row.source_symbols || null,
      business_behavior: row.business_behavior || null,
      technical_behavior: row.technical_behavior || null,
      inputs: row.inputs || null,
      outputs: row.outputs || null,
      persistence: row.persistence || null,
      surface: row.surface || null,
      side_effect_class: row.side_effect_class || 'none',
      moves_money: row.moves_money || 0,
      requires_approval: row.requires_approval || 0,
      external_services: row.external_services || null,
      target_crate: row.target_crate ?? c.target_crate ?? null,
      target_module: row.target_module ?? c.target_module ?? null,
      canonical_id: row.canonical_id,
      created_pass: createdPass,
    })
  const id = Number(info.lastInsertRowid)
  db.prepare('INSERT OR IGNORE INTO provenance (source_id, canonical_id) VALUES (?,?)').run(id, row.canonical_id)
  return id
}

const sourceKeys = new Map()
function normalizeIds(ids = []) {
  const raw = Array.isArray(ids) ? ids : String(ids).split(',')
  return raw.flatMap((id) => {
    if (typeof id === 'number') return [id]
    const text = String(id).trim()
    const range = text.match(/^(\d+)-(\d+)$/)
    if (range) {
      const start = Number(range[1])
      const end = Number(range[2])
      if (end < start) throw new Error(`invalid descending source id range ${text}`)
      return Array.from({ length: end - start + 1 }, (_, i) => start + i)
    }
    return [text]
  }).filter((id) => id !== '')
}

function resolveIds(ids = []) {
  return normalizeIds(ids).map((id) => {
    if (typeof id === 'number') return id
    if (/^\d+$/.test(String(id))) return Number(id)
    if (!sourceKeys.has(id)) throw new Error(`unknown source key ${id}`)
    return sourceKeys.get(id)
  })
}

function idsText(ids = []) {
  return resolveIds(ids).join(',')
}

function ensureCensusFile(file) {
  const fi = fileInfo(file.path)
  const mapped = idsText(file.ids || [])
  db.prepare(`INSERT OR IGNORE INTO donor_file_census
    (donor,path,is_directory,kind,classification,read_status,mapped_source_ids,exclusion_reason,size_bytes,mtime_ms,sha256,reviewed_by,reviewed_at)
    VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)`).run(
      donor,
      file.path,
      0,
      file.kind || 'source',
      file.classification,
      readStatus(file.classification, file.read ?? true),
      mapped,
      file.reason || file.evidence || '',
      fi.size,
      fi.mtime,
      fi.sha,
      reviewedBy,
      now,
    )
}

function updateReview(review) {
  const row = db.prepare('SELECT donor,path FROM donor_file_census WHERE donor=? AND path=?').get(donor, review.path)
  if (!row) throw new Error(`missing census row for ${donor}/${review.path}`)
  const mapped = idsText(review.ids || [])
  const evidence = review.evidence || review.reason || ''
  db.prepare(`UPDATE donor_file_census
    SET classification=?, read_status=?, mapped_source_ids=?, exclusion_reason=?, reviewed_by=?, reviewed_at=?
    WHERE donor=? AND path=?`).run(
      review.classification,
      readStatus(review.classification, review.read ?? true),
      mapped,
      evidence,
      reviewedBy,
      now,
      donor,
      review.path,
    )
  for (const sourceId of resolveIds(review.ids || [])) {
    const source = db.prepare('SELECT id FROM source_capability WHERE id=?').get(sourceId)
    if (!source) throw new Error(`mapped source id ${sourceId} missing for ${donor}/${review.path}`)
    const symbol = (review.symbol || evidence || `review:${review.path}`).slice(0, 180)
    db.prepare(`INSERT OR IGNORE INTO source_file_capability_link
      (source_id,donor,path,symbol,evidence_note) VALUES (?,?,?,?,?)`).run(
        sourceId,
        donor,
        review.path,
        symbol,
        evidence || `file-level review for ${review.path}`,
      )
  }
}

function refreshMeta() {
  db.prepare(`UPDATE canonical_capability SET donor_count=(
    SELECT COUNT(DISTINCT donor) FROM source_capability WHERE canonical_id=canonical_capability.id
  )`).run()
  const sourceCount = db.prepare('SELECT COUNT(*) n FROM source_capability').get().n
  const canonicalCount = db.prepare('SELECT COUNT(*) n FROM canonical_capability').get().n
  const moneyCount = db.prepare('SELECT COUNT(*) n FROM canonical_capability WHERE moves_money=1').get().n
  const rows = db.prepare('SELECT COUNT(*) n FROM donor_file_census').get().n
  const donors = db.prepare('SELECT COUNT(DISTINCT donor) n FROM donor_file_census').get().n
  const unread = db.prepare("SELECT COUNT(*) n FROM donor_file_census WHERE read_status='unread_pending'").get().n
  const fully = db.prepare(`SELECT COUNT(*) n FROM (
    SELECT donor, SUM(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) pending
    FROM donor_file_census GROUP BY donor HAVING pending=0
  )`).get().n
  const setMeta = db.prepare('INSERT INTO meta(k,v) VALUES (?,?) ON CONFLICT(k) DO UPDATE SET v=excluded.v')
  setMeta.run('source_capability_count', String(sourceCount))
  setMeta.run('canonical_capability_count', String(canonicalCount))
  setMeta.run('money_canonical_count', String(moneyCount))
  setMeta.run('true_ideal_denominator', String(canonicalCount))
  setMeta.run('file_census_rows', String(rows))
  setMeta.run('file_census_donors_scanned', String(donors))
  setMeta.run('file_census_unread_pending', String(unread))
  setMeta.run('file_census_fully_reviewed_donors', String(fully))
  setMeta.run('file_census_last_reviewed_at', now)
  setMeta.run(
    'note',
    `Source-cited capability census: ${sourceCount} capabilities across ${donors} donors -> ${canonicalCount} canonical Chronica capabilities. Money flags corrected to ${moneyCount}. Literal file-level exhaustiveness is tracked separately in donor_file_census and is incomplete until file_census_unread_pending=0.`,
  )
}

const tx = db.transaction(() => {
  for (const source of sourceRows) {
    sourceKeys.set(source.key, ensureSource(source))
  }
  for (const file of ensureFileRows) ensureCensusFile(file)
  for (const review of reviewRows) updateReview(review)
  if (manifest.require_zero_pending) {
    const pending = db.prepare("SELECT path FROM donor_file_census WHERE donor=? AND read_status='unread_pending' ORDER BY path").all(donor)
    if (pending.length) {
      throw new Error(`${donor} still has unread pending rows: ${pending.map((r) => r.path).join(', ')}`)
    }
  }
  refreshMeta()
})

tx()

console.log(JSON.stringify({
  donor,
  source_keys: Object.fromEntries(sourceKeys),
  donor_summary: db.prepare(`SELECT donor, COUNT(*) rows,
    SUM(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) pending,
    SUM(CASE WHEN classification IN ('mapped','duplicate_variant_mapped') AND COALESCE(mapped_source_ids,'')='' THEN 1 ELSE 0 END) mapped_missing_source_ids
    FROM donor_file_census WHERE donor=? GROUP BY donor`).get(donor),
  total_unread_pending: db.prepare("SELECT COUNT(*) n FROM donor_file_census WHERE read_status='unread_pending'").get().n,
  fully_reviewed_donors: db.prepare(`SELECT COUNT(*) n FROM (
    SELECT donor, SUM(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) pending
    FROM donor_file_census GROUP BY donor HAVING pending=0
  )`).get().n,
}, null, 2))

db.close()
