// Diff two truth snapshots. CODE / EVIDENCE / CLASSIFICATION / DENOMINATOR stay separate.

import { DIFF_CLASSES } from './vocab.mjs'
import { asArray, text } from './util.mjs'

function keyEdge(edge) {
  return text(edge.edge_id || edge.id)
}

function classBucket() {
  return Object.fromEntries(DIFF_CLASSES.map((name) => [name, []]))
}

export function diffSnapshots(previous, current) {
  const buckets = classBucket()
  const summary = {
    stronger: [],
    weaker: [],
    disappeared: [],
    newly_proven: [],
    demoted: [],
    stale: [],
    contracts_satisfied: [],
    contracts_conflict: [],
  }

  const prevSha = text(previous?.provenance?.repository_sha)
  const curSha = text(current?.provenance?.repository_sha)
  if (prevSha && curSha && prevSha !== curSha) {
    buckets.CODE_CHANGE.push({ field: 'repository_sha', from: prevSha, to: curSha })
  }

  const prevDen = previous?.fic?.denominator
  const curDen = current?.fic?.denominator
  if (prevDen != null && curDen != null && prevDen !== curDen) {
    buckets.DENOMINATOR_CHANGE.push({ field: 'fic.denominator', from: prevDen, to: curDen })
  }
  const prevFam = previous?.families?.family_count
  const curFam = current?.families?.family_count
  if (prevFam != null && curFam != null && prevFam !== curFam) {
    buckets.DENOMINATOR_CHANGE.push({ field: 'families.family_count', from: prevFam, to: curFam })
  }

  const prevFic = previous?.fic?.l3_or_higher
  const curFic = current?.fic?.l3_or_higher
  if (prevFic != null && curFic != null && prevFic !== curFic) {
    buckets.CLASSIFICATION_CHANGE.push({ field: 'fic.l3_or_higher', from: prevFic, to: curFic })
    if (curFic > prevFic) summary.stronger.push('FIC')
    if (curFic < prevFic) {
      summary.weaker.push('FIC')
      summary.demoted.push({ field: 'FIC', from: prevFic, to: curFic })
    }
  }

  const prevFresh = text(previous?.provenance?.freshness)
  const curFresh = text(current?.provenance?.freshness)
  if (prevFresh && curFresh && prevFresh !== curFresh) {
    buckets.CLASSIFICATION_CHANGE.push({ field: 'freshness', from: prevFresh, to: curFresh })
    if (curFresh === 'STALE') summary.stale.push({ field: 'freshness', reason: current?.provenance?.stale_reason })
  }

  const prevEdges = new Map(asArray(previous?.first_vertical?.edges).map((edge) => [keyEdge(edge), edge]))
  const curEdges = new Map(asArray(current?.first_vertical?.edges).map((edge) => [keyEdge(edge), edge]))
  for (const [id, prev] of prevEdges) {
    const next = curEdges.get(id)
    if (!next) {
      summary.disappeared.push(id)
      buckets.CLASSIFICATION_CHANGE.push({ field: `edge.${id}`, from: prev.current_status, to: null })
      continue
    }
    if (text(prev.evidence) !== text(next.evidence)) {
      buckets.EVIDENCE_CHANGE.push({ field: `edge.${id}.evidence` })
    }
    if (text(prev.runtime_owner) !== text(next.runtime_owner) && text(next.runtime_owner) !== 'UNSTATED') {
      buckets.CODE_CHANGE.push({ field: `edge.${id}.runtime_owner`, from: prev.runtime_owner, to: next.runtime_owner })
    }
    if (text(prev.current_status) !== text(next.current_status)) {
      buckets.CLASSIFICATION_CHANGE.push({ field: `edge.${id}.status`, from: prev.current_status, to: next.current_status })
      if (next.current_status === 'LIVE' && prev.current_status !== 'LIVE') summary.newly_proven.push(id)
      if (['MISSING', 'ISLAND', 'BLOCKED'].includes(next.current_status) && prev.current_status === 'LIVE') {
        summary.demoted.push({ field: id, from: prev.current_status, to: next.current_status })
        summary.weaker.push(id)
      }
      if (next.current_status === 'LIVE' || next.current_status === 'LIVE_BUT_UNVERIFIED') summary.stronger.push(id)
    }
  }
  for (const [id, next] of curEdges) {
    if (!prevEdges.has(id) && next.current_status === 'LIVE') summary.newly_proven.push(id)
  }

  const prevConflicts = asArray(previous?.contracts?.errors)
  const curConflicts = asArray(current?.contracts?.errors)
  for (const err of curConflicts) {
    if (/DUPLICATE_EXECUTION_SUBSTRATE|CONFLICTS/.test(err) && !prevConflicts.includes(err)) {
      summary.contracts_conflict.push(err)
      buckets.CLASSIFICATION_CHANGE.push({ field: 'contracts', to: err })
    }
  }
  if (prevConflicts.length && curConflicts.length === 0) {
    summary.contracts_satisfied.push('no duplicate execution substrate')
  }

  const mixed = []
  for (const [cls, rows] of Object.entries(buckets)) {
    for (const row of rows) {
      if (row.class && row.class !== cls) mixed.push(row)
    }
  }

  return {
    classes: buckets,
    summary,
    mixed: mixed.length === 0,
  }
}

export function assertUnmixed(diff) {
  const errors = []
  if (diff.mixed !== true) errors.push('diff classes mixed')
  const seen = new Set()
  for (const cls of DIFF_CLASSES) {
    for (const row of diff.classes[cls] || []) {
      const id = JSON.stringify(row)
      if (seen.has(`${cls}:${id}`)) continue
      for (const other of DIFF_CLASSES) {
        if (other === cls) continue
        if ((diff.classes[other] || []).some((item) => JSON.stringify(item) === id && other !== cls && item.field === row.field && item.from === row.from && item.to === row.to && cls !== other)) {
          // same field may appear in one class only
        }
      }
      seen.add(`${cls}:${id}`)
    }
  }
  const fields = {}
  for (const cls of DIFF_CLASSES) {
    for (const row of diff.classes[cls] || []) {
      const field = row.field
      if (!field) continue
      if (fields[field] && fields[field] !== cls) {
        errors.push(`field ${field} mixed across ${fields[field]} and ${cls}`)
      }
      fields[field] = cls
    }
  }
  return { ok: errors.length === 0, errors }
}
