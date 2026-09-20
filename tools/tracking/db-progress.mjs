#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { openReadOnlyDatabase } from '../db/sqlite-open.mjs'
import { isUsableCanonicalCapabilityTable } from '../capabilities/canonical-fallback.mjs'

const args = process.argv.slice(2)
const valueAfter = (flag) => {
  const idx = args.indexOf(flag)
  return idx >= 0 ? args[idx + 1] : undefined
}
const root = valueAfter('--root') ?? process.cwd()
const markdown = args.includes('--markdown')

const pct = (part, whole) => {
  if (!whole) return 0
  return Number(((Number(part) / Number(whole)) * 100).toFixed(2))
}

const tableExists = (db, table) =>
  db.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(table).n > 0

const columns = (db, table) => {
  if (!tableExists(db, table)) return new Set()
  return new Set(db.prepare(`PRAGMA table_info(${table})`).all().map((row) => row.name))
}

const scalar = (db, sql, fallback = 0) => {
  try {
    const row = db.prepare(sql).get()
    if (!row) return fallback
    const [first] = Object.values(row)
    return first ?? fallback
  } catch {
    return fallback
  }
}

const rows = (db, sql) => {
  try {
    return db.prepare(sql).all()
  } catch {
    return []
  }
}

const readJson = (path) => {
  if (!existsSync(path)) return null
  return JSON.parse(readFileSync(path, 'utf8'))
}

const readCapabilities = () => {
  const path = join(root, 'docs', 'capabilities.db')
  const cloudMeta = readJson(join(root, 'docs', 'capabilities-cloud', 'meta.json'))
  if (!existsSync(path)) {
    return {
      present: false,
      path,
      authority: 'missing local docs/capabilities.db; use cloud snapshot as fallback context only',
      local_audit_required: true,
      canonical: {
        total: cloudMeta?.source?.canonical_capabilities ?? null,
        source: cloudMeta ? 'docs/capabilities-cloud/meta.json' : 'unavailable',
      },
    }
  }

  const db = openReadOnlyDatabase(path)
  try {
    // row-count-aware: a canonical_capability table that EXISTS but has 0 rows carries the same
    // "no real truth here" meaning as a MISSING table (issue #713's empty-shell trap) — treat it
    // identically so progress reporting doesn't claim a false zero-capability denominator.
    const hasCanonical = isUsableCanonicalCapabilityTable(db, 'canonical_capability')
    const hasOverrides = tableExists(db, 'canonical_status_override')
    const hasImplEvidence = tableExists(db, 'impl_evidence')
    const hasSource = tableExists(db, 'source_capability')
    const hasCensus = tableExists(db, 'donor_file_census')
    const canonicalColumns = columns(db, 'canonical_capability')
    const overrideColumns = columns(db, 'canonical_status_override')

    const canonicalTotal = hasCanonical
      ? scalar(db, 'SELECT count(*) n FROM canonical_capability')
      : (cloudMeta?.source?.canonical_capabilities ?? scalar(db, 'SELECT count(DISTINCT canonical_key) n FROM slice_canonical', null))

    const statusSource = hasCanonical && canonicalColumns.has('status')
      ? 'canonical_capability.status'
      : hasOverrides && overrideColumns.has('status')
        ? 'canonical_status_override.status'
        : 'none'
    const statusRows = statusSource === 'canonical_capability.status'
      ? rows(db, 'SELECT status, count(*) n FROM canonical_capability GROUP BY status ORDER BY n DESC')
      : statusSource === 'canonical_status_override.status'
        ? rows(db, 'SELECT status, count(*) n FROM canonical_status_override GROUP BY status ORDER BY n DESC')
        : []
    const status = Object.fromEntries(statusRows.map((row) => [row.status ?? '<null>', row.n]))

    const implEvidence = hasImplEvidence ? scalar(db, 'SELECT count(*) n FROM impl_evidence') : 0
    const implEvidenceCaps = hasImplEvidence ? scalar(db, 'SELECT count(DISTINCT canonical_key) n FROM impl_evidence') : 0

    const censusTotals = hasCensus
      ? db.prepare(`SELECT
          count(*) rows,
          count(DISTINCT donor) donors,
          sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) unread_pending,
          sum(CASE WHEN classification LIKE '%review_pending' THEN 1 ELSE 0 END) review_pending,
          sum(CASE WHEN classification='blocked' THEN 1 ELSE 0 END) blocked
        FROM donor_file_census`).get()
      : null

    const fullyReviewedDonors = hasCensus
      ? scalar(db, `SELECT count(*) n FROM (
          SELECT donor, sum(CASE WHEN read_status='unread_pending' THEN 1 ELSE 0 END) pending
          FROM donor_file_census GROUP BY donor HAVING pending=0
        )`)
      : 0

    const sourceRows = hasSource ? scalar(db, 'SELECT count(*) n FROM source_capability') : 0
    const sourceMoney = hasSource && columns(db, 'source_capability').has('moves_money')
      ? scalar(db, 'SELECT count(*) n FROM source_capability WHERE moves_money=1')
      : 0
    const sourceApproval = hasSource && columns(db, 'source_capability').has('requires_approval')
      ? scalar(db, 'SELECT count(*) n FROM source_capability WHERE requires_approval=1')
      : 0

    const canonicalVerified = status.verified ?? 0
    const implementationEvidencePercent = pct(implEvidenceCaps, canonicalTotal)
    const statusVerifiedPercent = pct(canonicalVerified, canonicalTotal)
    const fileReviewPercent = censusTotals ? pct(censusTotals.rows - censusTotals.unread_pending, censusTotals.rows) : 0

    return {
      present: true,
      path,
      local_audit_required: !hasCanonical,
      schema: {
        canonical_capability: hasCanonical,
        source_capability: hasSource,
        canonical_status_override: hasOverrides,
        impl_evidence: hasImplEvidence,
        donor_file_census: hasCensus,
      },
      canonical: {
        total: canonicalTotal,
        source: hasCanonical ? 'canonical_capability' : cloudMeta ? 'docs/capabilities-cloud/meta.json fallback' : 'slice_canonical distinct fallback',
      },
      status: {
        source: statusSource,
        counts: status,
        verified_percent_of_denominator: statusVerifiedPercent,
      },
      implementation_evidence: {
        rows: implEvidence,
        distinct_capabilities: implEvidenceCaps,
        percent_of_denominator: implementationEvidencePercent,
      },
      source_capabilities: {
        rows: sourceRows,
        moves_money: sourceMoney,
        requires_approval: sourceApproval,
      },
      donor_file_census: censusTotals
        ? {
            ...censusTotals,
            reviewed_or_classified: censusTotals.rows - censusTotals.unread_pending,
            fully_reviewed_donors: fullyReviewedDonors,
            review_percent: fileReviewPercent,
          }
        : null,
      write_health: hasCanonical
        ? 'canonical write surface present'
        : 'schema drift: canonical_capability table missing; verified writes can only use survivable side-tables until caps:schema/rebuild restores canonical rows',
    }
  } finally {
    db.close()
  }
}

const readArchitecture = () => {
  const path = join(root, 'docs', 'architecture.db')
  if (!existsSync(path)) {
    return { present: false, path, local_audit_required: true }
  }
  const db = openReadOnlyDatabase(path)
  try {
    const meta = tableExists(db, 'meta')
      ? Object.fromEntries(rows(db, 'SELECT k, v FROM meta ORDER BY k').map((row) => [row.k, row.v]))
      : {}
    const linkStatus = tableExists(db, 'capability_architecture_link')
      ? Object.fromEntries(rows(db, 'SELECT status, count(*) n FROM capability_architecture_link GROUP BY status ORDER BY n DESC').map((row) => [row.status ?? '<null>', row.n]))
      : {}
    const linkRelationship = tableExists(db, 'capability_architecture_link')
      ? Object.fromEntries(rows(db, 'SELECT relationship, count(*) n FROM capability_architecture_link GROUP BY relationship ORDER BY n DESC').map((row) => [row.relationship ?? '<null>', row.n]))
      : {}
    const gapsByKind = tableExists(db, 'architecture_target_gap')
      ? Object.fromEntries(rows(db, 'SELECT gap_kind, count(*) n FROM architecture_target_gap GROUP BY gap_kind ORDER BY n DESC').map((row) => [row.gap_kind ?? '<null>', row.n]))
      : {}
    return {
      present: true,
      path,
      meta,
      link_status: linkStatus,
      link_relationship: linkRelationship,
      gaps_by_kind: gapsByKind,
      coverage_percent: Number(meta.coverage_percent ?? 0),
      planned_gap_percent: pct(meta.gap_capabilities ?? 0, meta.canonical_capability_count ?? 0),
    }
  } finally {
    db.close()
  }
}

const capability = readCapabilities()
const architecture = readArchitecture()
const denominator = Number(capability.canonical?.total ?? architecture.meta?.canonical_capability_count ?? 0)
const progress = {
  generated_at: new Date().toISOString(),
  root,
  headline: {
    canonical_denominator: denominator || null,
    implementation_evidence_percent: pct(capability.implementation_evidence?.distinct_capabilities ?? 0, denominator),
    status_verified_percent: pct(capability.status?.counts?.verified ?? 0, denominator),
    donor_file_review_percent: capability.donor_file_census?.review_percent ?? null,
    architecture_coverage_percent: architecture.coverage_percent ?? null,
    architecture_planned_gap_percent: architecture.planned_gap_percent ?? null,
  },
  capability,
  architecture,
  interpretation: [
    'implementation_evidence_percent is the strongest DB progress meter for code actually recorded in caps DB',
    'status_verified_percent is only as strong as the status write surface; if canonical_capability is absent, it is a side-table override view',
    'architecture_coverage_percent means accounted/mapped, not implemented',
    'architecture_planned_gap_percent is the amount still mapped as planned logical-domain work',
    'donor_file_review_percent measures source-census reading progress, not implementation',
  ],
}

if (!markdown) {
  console.log(JSON.stringify(progress, null, 2))
} else {
  const h = progress.headline
  console.log(`# Chronica DB Progress\n`)
  console.log(`- Canonical denominator: ${h.canonical_denominator ?? 'unknown'}`)
  console.log(`- Implementation evidence: ${h.implementation_evidence_percent}%`)
  console.log(`- Status verified: ${h.status_verified_percent}%`)
  console.log(`- Donor file review: ${h.donor_file_review_percent ?? 'unknown'}%`)
  console.log(`- Architecture coverage: ${h.architecture_coverage_percent ?? 'unknown'}%`)
  console.log(`- Architecture planned gaps: ${h.architecture_planned_gap_percent ?? 'unknown'}%`)
  console.log(`- Capability write health: ${progress.capability.write_health ?? 'missing local capability DB'}`)
}
