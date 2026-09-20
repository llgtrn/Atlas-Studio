#!/usr/bin/env node
// first-vertical-gap.mjs — doctrine 183 first production vertical gap graph validator.
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

export const EDGE_STATUSES = Object.freeze([
  'LIVE',
  'LIVE_BUT_UNVERIFIED',
  'PARTIAL_RUNTIME',
  'ISLAND',
  'FAKE_COUPLING',
  'MISSING',
  'BLOCKED',
])

const ROOT = path.resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')

export function loadFirstVertical(root = ROOT) {
  return JSON.parse(readFileSync(join(root, 'docs/_machine/first-vertical-gap-graph.json'), 'utf8'))
}

export function validateFirstVertical(doc) {
  const errors = []
  if (doc.schema !== 'chronica.first_vertical_gap_graph.v1') errors.push('invalid schema')
  if (doc.doctrine !== 'docs/doctrines/183-roadmap-world-class-infrastructure-90-coverage.md') {
    errors.push('must cite doctrine 183')
  }
  const seen = new Set()
  for (const edge of doc.edges || []) {
    if (seen.has(edge.id)) errors.push(`duplicate edge ${edge.id}`)
    seen.add(edge.id)
    if (!EDGE_STATUSES.includes(edge.status)) errors.push(`${edge.id}: invalid status ${edge.status}`)
    if (!edge.from || !edge.to) errors.push(`${edge.id}: missing from/to`)
    if (!edge.fic_domains || !edge.fic_domains.length) errors.push(`${edge.id}: missing fic_domains`)
    if (!edge.evidence) errors.push(`${edge.id}: missing evidence`)
  }
  const board = doc.board_authority_decision || {}
  if (board.authority_model && board.authority_model !== 'EXISTING_MODEL_SUFFICIENT') {
    errors.push('authority_model must stay EXISTING_MODEL_SUFFICIENT; do not invent BoardAuthority')
  }
  if (board.execution_grant_bridge && board.execution_grant_bridge !== 'NOT_YET_PROVEN') {
    errors.push('execution_grant_bridge must be NOT_YET_PROVEN')
  }
  if (doc.roadmap_priority_change != null) {
    errors.push('this freeze forbids ROADMAP_PRIORITY_CHANGE')
  }
  return { ok: errors.length === 0, errors }
}

export function summarize(doc) {
  const counts = Object.fromEntries(EDGE_STATUSES.map((s) => [s, 0]))
  for (const edge of doc.edges || []) counts[edge.status] += 1
  return { total: (doc.edges || []).length, counts }
}

function main() {
  const doc = loadFirstVertical()
  const result = validateFirstVertical(doc)
  const summary = summarize(doc)
  console.log(`first-vertical-gap: edges=${summary.total}`)
  for (const [k, v] of Object.entries(summary.counts)) console.log(`  ${k}: ${v}`)
  if (!result.ok) {
    console.error(result.errors.join('\n'))
    process.exitCode = 1
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
