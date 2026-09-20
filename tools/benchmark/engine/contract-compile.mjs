import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { asArray, text } from './util.mjs'
import { CAP_IDS, CONTRACT_SCHEMA, CONTRACT_SCHEMA_VERSION, STANDARD_CATALOG } from './contract-vocab.mjs'
import { overlayHardGates, unifiedHardGates } from './contract-gates.mjs'
import { sha256Hex } from './contract-hash.mjs'
import { defaultGeneratedDir, defaultSourceDir, engineRoot, groupById, loadContractSources } from './contract-load.mjs'
import {
  checkEvidencePaths,
  deriveFlowStatus,
  evaluateObligations,
  flowVerdict,
  goldenVerdict,
  indexEvidence,
  validateWitnessKinds,
} from './contract-eval.mjs'
import { renderCapPacket } from './contract-packet.mjs'
import { buildReleaseManifest } from './contract-release.mjs'
import { renderReports } from './contract-reports.mjs'

function duplicateErrors(label, dupes) {
  return dupes.map((id) => `duplicate ${label} id ${id}`)
}

export function compileContracts(options = {}) {
  const root = options.root || engineRoot()
  const sourceDir = options.sourceDir || defaultSourceDir(root)
  const outDir = options.outDir || defaultGeneratedDir(root)
  const write = options.write !== false
  const repositorySha = text(options.repositorySha) || 'UNBOUND'
  const releaseId = text(options.releaseId) || 'BR-2026.08.21.1'
  const knownHardGates = unifiedHardGates()
  const loaded = loadContractSources(sourceDir, { knownHardGates })
  const errors = [...loaded.errors]
  const warnings = []

  const laws = groupById(loaded.byType.global_law)
  const families = groupById(loaded.byType.family)
  const envelopes = groupById(loaded.byType.cap_envelope, 'cap')
  const flows = groupById(loaded.byType.flow)
  const golden = groupById(loaded.byType.golden_flow)
  const evidence = groupById(loaded.byType.evidence)
  const edges = groupById(loaded.byType.cross_cap_edge)
  errors.push(...duplicateErrors('law', laws.dupes))
  errors.push(...duplicateErrors('family', families.dupes))
  errors.push(...duplicateErrors('envelope', envelopes.dupes))
  errors.push(...duplicateErrors('flow', flows.dupes))
  errors.push(...duplicateErrors('golden', golden.dupes))
  errors.push(...duplicateErrors('evidence', evidence.dupes))
  errors.push(...duplicateErrors('edge', edges.dupes))

  for (const cap of CAP_IDS) {
    if (!envelopes.map.has(cap)) errors.push(`missing CAP envelope ${cap}`)
  }

  const evidencePath = checkEvidencePaths([...evidence.map.values()], root)
  errors.push(...evidencePath.errors)
  errors.push(...validateWitnessKinds([...evidence.map.values()]).errors)

  const goldenByFlow = new Map()
  for (const gbf of golden.map.values()) {
    for (const id of asArray(gbf.flow_ids).map(text)) {
      if (!goldenByFlow.has(id)) goldenByFlow.set(id, [])
      goldenByFlow.get(id).push(text(gbf.id))
    }
  }

  const byFlowEvidence = indexEvidence([...evidence.map.values()])
  const evaluatedFlows = []
  for (const flow of flows.map.values()) {
    const ev = byFlowEvidence.get(text(flow.id)) || []
    const obligations = evaluateObligations(flow, byFlowEvidence)
    const status = deriveFlowStatus(flow, obligations, ev)
    const hardGateFailures = []
    if (text(flow.claimed_maturity) && obligations.missing.length && /[WESB][5-6]/.test(text(flow.claimed_maturity))) {
      hardGateFailures.push('SCORE_OVERRIDES_HARD_GATE')
    }
    const goldenIds = [...new Set([...asArray(flow.golden).map(text), ...(goldenByFlow.get(text(flow.id)) || [])])]
    evaluatedFlows.push({
      ...flow,
      golden: goldenIds,
      derived_status: status,
      obligation_states: obligations.states,
      missing_obligations: obligations.missing,
      verdict: flowVerdict(obligations, hardGateFailures),
      hard_gate_failures: hardGateFailures,
      evidence_ids: ev.map((row) => row.id),
    })
  }

  for (const flow of evaluatedFlows) {
    if (!families.map.has(text(flow.family))) errors.push(`${flow.id}: unknown family ${flow.family}`)
    for (const gate of asArray(flow.hard_gates).map(text).filter(Boolean)) {
      if (!knownHardGates.has(gate)) errors.push(`${flow.id}: unknown hard gate ${gate}`)
    }
    for (const gbf of asArray(flow.golden).map(text).filter(Boolean)) {
      if (!golden.map.has(gbf)) errors.push(`${flow.id}: unknown golden ${gbf}`)
    }
  }

  for (const gbf of golden.map.values()) {
    for (const flowId of asArray(gbf.flow_ids).map(text)) {
      if (!flows.map.has(flowId)) errors.push(`${gbf.id}: unknown flow ${flowId}`)
    }
  }

  for (const edge of edges.map.values()) {
    if (!flows.map.has(text(edge.flow))) errors.push(`${edge.id}: unknown flow ${edge.flow}`)
    const flow = flows.map.get(text(edge.flow))
    if (flow) {
      const parts = new Set([text(flow.owner), ...asArray(flow.participants).map(text)])
      for (const cap of [text(edge.producer), text(edge.consumer), text(edge.semantic_owner)]) {
        if (cap && !parts.has(cap)) errors.push(`${edge.id}: ${cap} not a participant of ${edge.flow}`)
      }
    }
  }

  for (const ev of evidence.map.values()) {
    for (const link of asArray(ev.proves)) {
      if (!flows.map.has(text(link.flow))) errors.push(`${ev.id}: proves unknown flow ${link.flow}`)
    }
    if (text(ev.kind) === 'benchmark_fixture' && text(ev.runtime_kind) === 'PRODUCT_RUNTIME') {
      errors.push(`${ev.id}: FIXTURE_AS_RUNTIME`)
    }
  }

  const metaById = new Map(loaded.byType.anti_pattern_meta.map((row) => [text(row.id), row]))
  for (const gate of overlayHardGates()) {
    if (!metaById.has(gate)) errors.push(`overlay anti-pattern ${gate} missing meta`)
  }
  for (const meta of loaded.byType.anti_pattern_meta) {
    const fixture = text(meta.fixture)
    const review = text(meta.review) === 'MANUAL_REVIEW' || text(meta.detector) === 'MANUAL_REVIEW'
    if (text(meta.severity) === 'HARD_FAIL' && !review) {
      if (!fixture) errors.push(`${meta.id}: HARD_FAIL needs fixture or MANUAL_REVIEW`)
      else if (!existsSync(join(root, fixture))) errors.push(`${meta.id}: missing fixture ${fixture}`)
    }
  }

  const ir = {
    schema: CONTRACT_SCHEMA,
    schema_version: CONTRACT_SCHEMA_VERSION,
    standards: STANDARD_CATALOG,
    laws: [...laws.map.values()],
    families: [...families.map.values()],
    envelopes: CAP_IDS.map((cap) => envelopes.map.get(cap)).filter(Boolean),
    flows: evaluatedFlows,
    golden: [...golden.map.values()],
    evidence: [...evidence.map.values()],
    cross_cap: [...edges.map.values()],
    prose: loaded.byType.prose_statement,
    anti_pattern_meta: loaded.byType.anti_pattern_meta,
    routing: loaded.routing,
    source_hash: sha256Hex({
      laws: [...laws.map.values()],
      families: [...families.map.values()],
      envelopes: [...envelopes.map.values()],
      flows: loaded.byType.flow,
      golden: [...golden.map.values()],
      evidence: [...evidence.map.values()],
      cross_cap: [...edges.map.values()],
      prose: loaded.byType.prose_statement,
      anti_pattern_meta: loaded.byType.anti_pattern_meta,
    }),
  }

  const flowsById = new Map(evaluatedFlows.map((flow) => [flow.id, flow]))
  ir.golden = ir.golden.map((gbf) => ({ ...gbf, verdict: goldenVerdict(gbf, flowsById) }))
  ir.evidence_graph = {
    nodes: ir.flows.length + ir.evidence.length + ir.cross_cap.length,
    edges:
      ir.evidence.reduce((n, ev) => n + asArray(ev.proves).length, 0) + ir.cross_cap.length,
  }

  const packets = ir.envelopes.map((envelope) => renderCapPacket(envelope, ir, repositorySha, releaseId))
  const packetHashes = Object.fromEntries(packets.map((packet) => [packet.cap, packet.packet_hash]))
  const release = buildReleaseManifest({
    ir,
    packetHashes,
    repositorySha,
    generatedAt: options.generatedAt,
    releaseId,
  })
  const reports = renderReports(ir, release)

  if (write) {
    mkdirSync(join(outDir, 'packets'), { recursive: true })
    mkdirSync(join(outDir, 'reports'), { recursive: true })
    writeFileSync(join(outDir, 'ir.json'), `${JSON.stringify({ schema: CONTRACT_SCHEMA, source_hash: ir.source_hash, flow_ids: ir.flows.map((f) => f.id) }, null, 2)}\n`)
    writeFileSync(join(outDir, 'release.json'), `${JSON.stringify(release, null, 2)}\n`)
    for (const packet of packets) {
      writeFileSync(join(outDir, 'packets', `${packet.cap}.json`), `${JSON.stringify(packet, null, 2)}\n`)
      writeFileSync(join(outDir, 'packets', `${packet.cap}.md`), packet.markdown)
    }
    for (const [name, body] of Object.entries(reports)) {
      writeFileSync(join(outDir, 'reports', name), body.endsWith('\n') ? body : `${body}\n`)
    }
  }

  return {
    ok: errors.length === 0,
    errors,
    warnings,
    ir,
    packets,
    release,
    reports,
    broken_evidence: evidencePath.broken,
    sourceDir,
    outDir,
  }
}
