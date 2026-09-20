export const REVIEW_CLASSIFICATIONS = new Set([
  'mapped',
  'duplicate_variant_mapped',
  'behavioral_excluded',
  'non_behavioral_support',
  'generated_vendor_build_artifact',
  'blocked',
])

export const REVIEW_STATUSES = new Set([
  'reviewed',
  'classified_not_read',
  'blocked',
])

export function normalizeReviewPath(path) {
  return String(path || '')
    .replaceAll('\\', '/')
    .replace(/^\/+/, '')
    .replace(/\/+/g, '/')
}

function stripDonorPrefix(path, donor) {
  const donorPath = normalizeReviewPath(donor)
  if (!donorPath) return path
  for (const prefix of [`Temporary/${donorPath}/`, `temporary/${donorPath}/`, `${donorPath}/`]) {
    if (path.toLowerCase().startsWith(prefix.toLowerCase())) return path.slice(prefix.length)
  }
  return path
}

function normalizeMappedSourceIds(value) {
  if (Array.isArray(value)) return value.map(v => String(v).trim()).filter(Boolean).join(',')
  return String(value || '').split(',').map(v => v.trim()).filter(Boolean).join(',')
}

function normalizeReadStatus(value, classification) {
  const readStatus = String(value || '').trim()
  if (readStatus !== 'read') return readStatus
  return classification === 'blocked' ? 'blocked' : 'reviewed'
}

export function normalizeReviewManifest(input) {
  const donor = String(input?.donor || '').trim()
  const manifest = {
    donor,
    file_reviews: (input?.file_reviews || []).map(row => {
      const classification = String(row.classification || '').trim()
      return {
        path: stripDonorPrefix(normalizeReviewPath(row.path), donor),
        classification,
        read_status: normalizeReadStatus(row.read_status, classification),
        mapped_source_ids: normalizeMappedSourceIds(row.mapped_source_ids),
        exclusion_reason: String(row.exclusion_reason || '').trim(),
        evidence_note: String(row.evidence_note || '').trim(),
      }
    }),
    links: (input?.links || []).map(row => ({
      source_id: Number(row.source_id),
      path: stripDonorPrefix(normalizeReviewPath(row.path), donor),
      symbol: String(row.symbol || '').trim(),
      evidence_note: String(row.evidence_note || '').trim(),
    })),
    new_source_capabilities_needed: input?.new_source_capabilities_needed || [],
    blocked: input?.blocked || [],
  }
  return manifest
}

export function validateReviewManifest(input) {
  const manifest = normalizeReviewManifest(input)
  if (!manifest.donor) throw new Error('donor is required')
  for (const row of manifest.file_reviews) {
    if (!row.path) throw new Error('file review path is required')
    if (!REVIEW_CLASSIFICATIONS.has(row.classification)) {
      throw new Error(`unsupported classification: ${row.classification || '(empty)'}`)
    }
    if (!REVIEW_STATUSES.has(row.read_status)) {
      throw new Error(`unsupported read_status: ${row.read_status || '(empty)'}`)
    }
    if (row.classification === 'blocked' && row.read_status !== 'blocked') {
      throw new Error(`blocked classification requires blocked read_status: ${row.path}`)
    }
    if (row.classification === 'mapped' && !row.mapped_source_ids) {
      throw new Error(`mapped review requires mapped_source_ids: ${row.path}`)
    }
    if (!row.exclusion_reason && !row.evidence_note) {
      throw new Error(`review requires evidence: ${row.path}`)
    }
  }
  for (const link of manifest.links) {
    if (!Number.isInteger(link.source_id) || link.source_id <= 0) {
      throw new Error(`link source_id must be a positive integer: ${link.path}`)
    }
    if (!link.path) throw new Error('link path is required')
    if (!link.evidence_note) throw new Error(`link evidence_note is required: ${link.path}`)
  }
  return manifest
}

export function applyReviewManifest(db, input, options = {}) {
  const manifest = validateReviewManifest(input)
  const now = options.reviewedAt || new Date().toISOString()
  const reviewer = options.reviewedBy || 'donor-file-review'
  db.exec('CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT NOT NULL)')

  const updateFile = db.prepare(`UPDATE donor_file_census
    SET classification=?, read_status=?, mapped_source_ids=?, exclusion_reason=?, reviewed_by=?, reviewed_at=?
    WHERE donor=? AND path=?`)
  const insertLink = db.prepare(`INSERT OR REPLACE INTO source_file_capability_link(source_id, donor, path, symbol, evidence_note)
    VALUES (?, ?, ?, ?, ?)`)
  const setMeta = db.prepare('INSERT OR REPLACE INTO meta(k,v) VALUES(?,?)')
  const sourceExists = db.prepare('SELECT count(*) n FROM source_capability WHERE id=?')

  const tx = db.transaction(() => {
    let updated = 0
    for (const row of manifest.file_reviews) {
      if (row.mapped_source_ids) {
        for (const id of row.mapped_source_ids.split(',').filter(Boolean)) {
          if (sourceExists.get(Number(id)).n === 0) throw new Error(`source_capability id not found: ${id}`)
        }
      }
      const info = updateFile.run(
        row.classification,
        row.read_status,
        row.mapped_source_ids,
        row.exclusion_reason || row.evidence_note,
        reviewer,
        now,
        manifest.donor,
        row.path
      )
      if (info.changes === 0) throw new Error(`donor_file_census row not found: ${manifest.donor}/${row.path}`)
      updated += info.changes
    }

    let linked = 0
    for (const link of manifest.links) {
      if (sourceExists.get(link.source_id).n === 0) throw new Error(`source_capability id not found: ${link.source_id}`)
      insertLink.run(link.source_id, manifest.donor, link.path, link.symbol, link.evidence_note)
      linked++
    }

    const totals = db.prepare(`SELECT count(*) rows, count(DISTINCT donor) donors,
      sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending
      FROM donor_file_census`).get()
    const fullyReviewed = db.prepare(`SELECT count(*) n FROM (
      SELECT donor, sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) pending
      FROM donor_file_census GROUP BY donor HAVING pending=0
    )`).get().n
    setMeta.run('file_census_rows', String(totals.rows || 0))
    setMeta.run('file_census_donors_scanned', String(totals.donors || 0))
    setMeta.run('file_census_unread_pending', String(totals.unread_pending || 0))
    setMeta.run('file_census_fully_reviewed_donors', String(fullyReviewed || 0))
    setMeta.run('file_census_last_reviewed_at', now)
    return { donor: manifest.donor, updated, linked, totals, fullyReviewed }
  })

  return tx()
}
