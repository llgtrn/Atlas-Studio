#!/usr/bin/env node
// Thread4 FULL_BENCHMARK_REQUIREMENT_CENSUS. Does not mutate Benchmark IR or P0 CI.

import { mkdirSync, readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compileContracts } from '../engine/contract-compile.mjs'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..')
const OUT = join(ROOT, 'tools/benchmark/thread4')
const CLASS_MAP = {
  GLOBAL_LAW: 'APPLICABLE_HARD_REQUIREMENT',
  ABUSE_INVARIANT: 'APPLICABLE_HARD_REQUIREMENT',
  FLOW_REQUIREMENT: 'APPLICABLE_FLOW_REQUIREMENT',
  CAPABILITY_REQUIREMENT: 'APPLICABLE_FLOW_REQUIREMENT',
  MANUAL_REVIEW: 'APPLICABLE_MANUAL_REVIEW',
  EXPLANATORY_ONLY: 'EXPLANATORY_ONLY',
  DONOR_REFERENCE: 'DONOR_REFERENCE_ONLY',
}

function readJsonl(path) {
  if (!existsSync(path)) return []
  return readFileSync(path, 'utf8')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('#'))
    .map((line) => JSON.parse(line))
}

function classifyDoc(name) {
  const n = name.toLowerCase()
  if (/^20[5-9]-/.test(name) || name.includes('116-operating-world-class')) {
    return 'APPLICABLE_HARD_REQUIREMENT'
  }
  if (n.includes('fic') || n.includes('five-plane')) return 'APPLICABLE_HARD_REQUIREMENT'
  if (n.includes('oss') || n.includes('donor') || n.includes('facebook') || n.includes('parity')) {
    return 'DONOR_REFERENCE_ONLY'
  }
  if (n.includes('intent') || n.includes('portfolio') || n.includes('roadmap')) {
    return 'EXPLANATORY_ONLY'
  }
  return 'APPLICABLE_MANUAL_REVIEW'
}

function writeJsonl(path, rows) {
  writeFileSync(path, rows.map((row) => JSON.stringify(row)).join('\n') + (rows.length ? '\n' : ''))
}

function main() {
  mkdirSync(OUT, { recursive: true })
  const built = compileContracts({ root: ROOT, write: false, releaseId: 'BR-2026.08.21.1' })
  if (!built.ok) {
    console.error('BENCHMARK_COMPILER_GAP')
    console.error((built.errors || []).slice(0, 20).join('\n'))
    process.exitCode = 1
  }
  const ir = built.ir
  const release = built.release
  const prose = ir.prose || readJsonl(join(ROOT, 'tools/benchmark/contracts/source/prose-classification.jsonl'))
  const overlay = readJsonl(join(OUT, 'product-evidence.jsonl'))
  const docs = readdirSync(join(ROOT, 'docs/benchmarks')).filter((name) => name.endsWith('.md'))
  const census = []

  for (const law of ir.laws || []) {
    census.push({
      id: law.id,
      class: 'APPLICABLE_HARD_REQUIREMENT',
      source: 'compiler:global_law',
      standard: Object.keys(law.standards || {})[0] || 'GLOBAL',
      statement: law.statement,
      hard: true,
    })
  }
  for (const flow of ir.flows || []) {
    census.push({
      id: flow.id,
      class: 'APPLICABLE_FLOW_REQUIREMENT',
      source: 'compiler:flow',
      owner: flow.owner,
      derived_status: flow.derived_status,
      golden: flow.golden || [],
      missing: flow.missing_obligations || [],
    })
    for (const po of flow.obligations || []) {
      census.push({
        id: `${flow.id}:${po}`,
        class: 'APPLICABLE_PROOF_OBLIGATION',
        source: 'compiler:obligation',
        flow: flow.id,
        obligation: po,
        state: (flow.obligation_states || {})[po] || 'MISSING',
      })
    }
  }
  for (const gbf of ir.golden || []) {
    census.push({
      id: gbf.id,
      class: 'APPLICABLE_FLOW_REQUIREMENT',
      source: 'compiler:golden',
      title: gbf.title,
      flows: gbf.flow_ids,
      verdict: gbf.verdict,
    })
  }
  for (const edge of ir.cross_cap || []) {
    census.push({
      id: edge.id,
      class: 'APPLICABLE_HARD_REQUIREMENT',
      source: 'compiler:cross_cap',
      statement: edge.rule || edge.statement || edge.id,
    })
  }
  for (const meta of ir.anti_pattern_meta || []) {
    census.push({
      id: meta.id,
      class: meta.severity === 'HARD_FAIL' ? 'APPLICABLE_HARD_REQUIREMENT' : 'APPLICABLE_MANUAL_REVIEW',
      source: 'compiler:anti_pattern',
      statement: meta.statement || meta.id,
      hard: meta.severity === 'HARD_FAIL',
    })
  }
  for (const row of prose) {
    census.push({
      id: row.id,
      class: CLASS_MAP[row.class] || 'APPLICABLE_MANUAL_REVIEW',
      source: 'compiler:prose',
      standard: row.standard,
      statement: row.statement,
      ref: row.ref,
    })
  }
  for (const name of docs.sort()) {
    census.push({
      id: `DOC-BENCHMARKS-${name}`,
      class: classifyDoc(name),
      source: 'docs/benchmarks',
      path: `docs/benchmarks/${name}`,
    })
  }

  const unclassified = census.filter((row) => !row.class)
  const counts = {}
  for (const row of census) counts[row.class] = (counts[row.class] || 0) + 1

  const overlayByFlow = new Map()
  for (const ev of overlay) {
    for (const prove of ev.proves || []) {
      if (!overlayByFlow.has(prove.flow)) overlayByFlow.set(prove.flow, [])
      overlayByFlow.get(prove.flow).push(ev)
    }
  }

  const gaps = (ir.flows || []).map((flow) => {
    const overlayHits = overlayByFlow.get(flow.id) || []
    return {
      id: `GAP-${flow.id}`,
      benchmark_source: 'BR-2026.08.21.1',
      standard_version: release.standard_versions,
      flow_id: flow.id,
      cap_owner: flow.owner,
      participants: (flow.dependencies || []).map((dep) => dep.cap || dep.id).filter(Boolean),
      required_behavior: flow.title || flow.id,
      current_witness: flow.derived_status,
      missing_witness: flow.missing_obligations || [],
      thread4_product_witness: overlayHits.map((ev) => ev.id),
      severity: flow.value || 'MEDIUM',
      blocker: flow.blocked || 'NONE',
      next_action: overlayHits.length
        ? 'KEEP_PRODUCT; Benchmark evidence still TOOLING_ONLY'
        : 'SALVAGE_OR_IMPLEMENT',
    }
  })

  writeJsonl(join(OUT, 'requirement-census.jsonl'), census)
  writeJsonl(join(OUT, 'gap-graph.jsonl'), gaps)

  const summary = {
    generated_at: new Date().toISOString(),
    benchmark_release: release.id,
    source_hash: release.source_hash,
    census_rows: census.length,
    unclassified: unclassified.length,
    counts,
    flow_total: (ir.flows || []).length,
    flow_by_status: Object.fromEntries(
      Object.entries(
        (ir.flows || []).reduce((acc, flow) => {
          acc[flow.derived_status] = (acc[flow.derived_status] || 0) + 1
          return acc
        }, {}),
      ),
    ),
    golden_total: (ir.golden || []).length,
    overlay_evidence: overlay.length,
    docs_benchmarks: docs.length,
  }
  writeFileSync(join(OUT, 'census-summary.json'), JSON.stringify(summary, null, 2) + '\n')
  console.log(JSON.stringify(summary, null, 2))
}

main()
