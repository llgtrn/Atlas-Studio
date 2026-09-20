import { asArray, text } from './util.mjs'
import { CAP_IDS, PROOF_OBLIGATIONS, PROSE_CLASSES } from './contract-vocab.mjs'
import { unifiedAntiPatternIds } from './contract-gates.mjs'

function countBy(rows, keyFn) {
  const out = {}
  for (const row of rows) {
    const key = keyFn(row)
    out[key] = (out[key] || 0) + 1
  }
  return out
}

export function renderReports(ir, release) {
  const byStatus = countBy(ir.flows, (flow) => flow.derived_status)
  const byCap = Object.fromEntries(CAP_IDS.map((cap) => [cap, ir.flows.filter((flow) => flow.owner === cap).length]))
  const prose = countBy(ir.prose, (row) => text(row.class))
  for (const cls of PROSE_CLASSES) if (prose[cls] == null) prose[cls] = 0
  const apIds = unifiedAntiPatternIds()
  const meta = new Map(ir.anti_pattern_meta.map((row) => [row.id, row]))
  const manual = ir.anti_pattern_meta.filter((row) => text(row.review) === 'MANUAL_REVIEW' || text(row.detector) === 'MANUAL_REVIEW')
  const unclassifiedHard = ir.prose.filter((row) => !PROSE_CLASSES.includes(text(row.class)))
  const poCoverage = Object.fromEntries(
    PROOF_OBLIGATIONS.map((po) => [po, ir.flows.filter((flow) => asArray(flow.obligations).includes(po)).length]),
  )

  const architecture = [
    '# Benchmark Contract Architecture',
    '',
    'Generated from Contract IR. Not product authority. Not Engine V2.',
    '',
    'BENCHMARK KNOWLEDGE → SEMANTIC STANDARDS → MACHINE CONTRACT IR → FLOW CONTRACTS → PROOF OBLIGATIONS → CAP PACKETS → CI MATRIX → IMPLEMENTATION AGENTS',
    '',
    `Release: ${release.id} sha=${release.benchmark_sha} source_hash=${ir.source_hash}`,
    '',
    'L0 semantic law / L1 capability contract / L2 flow / L3 abuse invariant.',
    'CAPABILITY != FLOW. GENERATED PACKET != SOURCE. FIXTURE != RUNTIME.',
    '',
  ].join('\n')

  const acceptance = [
    '# CAP1–CAP18 Acceptance Matrix',
    '',
    '| CAP | Flows | Spec | Partial | Blocked | Runtime |',
    '| --- | ---: | ---: | ---: | ---: | ---: |',
    ...CAP_IDS.map((cap) => {
      const rows = ir.flows.filter((flow) => flow.owner === cap)
      const n = (st) => rows.filter((flow) => flow.derived_status === st).length
      return `| ${cap} | ${rows.length} | ${n('SPEC_ONLY') + n('NO_RUNTIME')} | ${n('PARTIAL')} | ${n('BLOCKED')} | ${n('RUNTIME_CONNECTED') + n('VERIFIED')} |`
    }),
    '',
  ].join('\n')

  const cross = [
    '# Cross-CAP Contract Matrix',
    '',
    '| Edge | Producer | Consumer | Owner | Flow | Operation |',
    '| --- | --- | --- | --- | --- | --- |',
    ...ir.cross_cap.map((edge) => `| ${edge.id} | ${edge.producer} | ${edge.consumer} | ${edge.semantic_owner} | ${edge.flow} | ${edge.operation} |`),
    '',
  ].join('\n')

  const golden = [
    '# Golden Business Flow Matrix',
    '',
    '| GBF | Title | Flows | Status mix |',
    '| --- | --- | --- | --- |',
    ...ir.golden.map((gbf) => {
      const statuses = asArray(gbf.flow_ids).map((id) => ir.flows.find((flow) => flow.id === id)?.derived_status || 'MISSING')
      return `| ${gbf.id} | ${gbf.title || ''} | ${asArray(gbf.flow_ids).join(', ')} | ${gbf.verdict || statuses.join(', ')} |`
    }),
    '',
  ].join('\n')

  const po = ['# Proof Obligation Coverage', '', JSON.stringify(poCoverage, null, 2), ''].join('\n')

  const automation = [
    '# Automation coverage',
    '',
    `- anti_pattern_ids: ${apIds.length}`,
    `- meta_rows: ${ir.anti_pattern_meta.length}`,
    `- manual_review_meta: ${manual.length}`,
    `- prose_classified: ${ir.prose.length}`,
    `- unclassified_hard_prose: ${unclassifiedHard.length}`,
    `- fixture_kind evidence: ${ir.evidence.filter((ev) => ev.kind === 'benchmark_fixture').length}`,
    `- product-shaped evidence: ${ir.evidence.filter((ev) => ev.kind !== 'benchmark_fixture').length}`,
    `- evidence_graph_nodes: ${ir.evidence_graph?.nodes ?? 0}`,
    `- evidence_graph_edges: ${ir.evidence_graph?.edges ?? 0}`,
    '',
  ].join('\n')

  const coverage = [
    '# Flow coverage',
    '',
    JSON.stringify({ total: ir.flows.length, byStatus, byCap, golden: ir.golden.length, evidence: ir.evidence.length }, null, 2),
    '',
  ].join('\n')

  const changelog = [
    '# Benchmark Release Changelog',
    '',
    `id: ${release.id}`,
    `benchmark_sha: ${release.benchmark_sha}`,
    `source_hash: ${release.source_hash}`,
    `hash: ${release.hash}`,
    '',
    'First contract-compiler release. Pins WEB2APP 1.1.0, Employment/Store/BlackBox 1.0.0.',
    '',
  ].join('\n')

  const p0 = [
    '# P0 CI integration handoff',
    '',
    'P0_AUTHORITATIVE_CI_TOUCHED: NO',
    'P0_INTEGRATION_REQUIRED: YES',
    '',
    'Do not modify `.github/workflows` from the benchmark branch.',
    'When a later explicit task authorizes P0 integration:',
    '1. Add one data-driven job that runs `pnpm benchmark:contracts:verify`.',
    '2. Consume `benchmark:ci-matrix` JSON for affected CAP sharding.',
    '3. Keep TIER2 global hard invariants as merge gates.',
    '4. Do not create one workflow file per flow.',
    '',
  ].join('\n')

  const template = [
    '# CAP agent instruction template',
    '',
    'You are CAP<N>. Implement against the pinned BENCHMARK_RELEASE and BENCHMARK_SHA in your packet.',
    'Read `tools/benchmark/contracts/generated/packets/CAP<N>.json` only.',
    'Do not reinterpret standards 205–208. Select the highest-value ACTIONABLE flow.',
    'Implement missing obligations. Run required affected-flow tests. Report exact evidence.',
    'If the benchmark appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.',
    '',
  ].join('\n')

  const review = [
    '# BENCHMARK_MANUAL_REVIEW_QUEUE',
    '',
    ...ir.prose
      .filter((row) => text(row.class) === 'MANUAL_REVIEW')
      .map((row) => `- ${row.id}: ${row.statement || row.ref}`),
    ...ir.flows
      .filter((flow) => asArray(flow.semantic_review_required).length)
      .map((flow) => `- ${flow.id}: ${asArray(flow.semantic_review_required).join('; ')}`),
    '',
  ].join('\n')

  void meta
  return {
    'architecture.md': architecture,
    'cap-acceptance-matrix.md': acceptance,
    'cross-cap-matrix.md': cross,
    'golden-flow-matrix.md': golden,
    'proof-obligation-coverage.md': po,
    'automation-coverage.md': automation,
    'flow-coverage.md': coverage,
    'release-changelog.md': changelog,
    'p0-ci-integration-handoff.md': p0,
    'cap-agent-template.md': template,
    'manual-review-queue.md': review,
    'prose-classification.json': `${JSON.stringify(prose, null, 2)}\n`,
  }
}
