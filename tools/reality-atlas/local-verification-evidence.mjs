import { readLocalVerificationEvidence } from './local-verification.mjs'

function ensureNode(family, id, kind, label, data = {}) {
  const full = `${family.name}:${id}`
  let node = family.nodes.find((candidate) => candidate.id === full)
  if (!node) {
    node = { id: full, family: family.name, kind, label, ...data }
    family.nodes.push(node)
  }
  return node
}

function ensureEdge(family, source, target, type, data = {}) {
  const id = `${family.name}:${source}:${type}:${target}`
  if (!family.edges.some((edge) => edge.id === id)) family.edges.push({ id, family: family.name, source, target, type, ...data })
}

function summarizeStatus(records) {
  const statuses = records.map((record) => record.status)
  if (statuses.length === 0) return 'NOT_RUN'
  if (statuses.some((status) => status === 'FAIL')) return 'FAIL'
  if (statuses.every((status) => status === 'PASS')) return 'PASS'
  return 'PARTIAL'
}

export function enrichLocalVerificationEvidence(root, coverage) {
  const evidence = readLocalVerificationEvidence(root)
  const family = coverage.families.test_evidence
  const localLayer = ensureNode(family, 'evidence_provider:local_execution', 'evidence_provider', 'local_execution', {
    source: evidence.source,
    currentSha: evidence.currentSha,
    currentWorkingTreeDirty: evidence.currentWorkingTreeDirty,
  }).id
  const sourceLayer = ensureNode(family, 'evidence_provider:source', 'evidence_provider', 'source_evidence').id
  const hostedLayer = ensureNode(family, 'evidence_provider:hosted_ci', 'evidence_provider', 'hosted_ci').id
  const productionLayer = ensureNode(family, 'evidence_provider:production_runtime', 'evidence_provider', 'production_runtime').id

  ensureEdge(family, sourceLayer, localLayer, 'ASSURANCE_NEXT')
  ensureEdge(family, localLayer, hostedLayer, 'ASSURANCE_NEXT')
  ensureEdge(family, hostedLayer, productionLayer, 'ASSURANCE_NEXT')

  for (const record of evidence.latestCurrentByCheck) {
    const node = ensureNode(
      family,
      `local_check:${record.check}`,
      'local_execution_evidence',
      record.check,
      {
        status: record.status,
        sha: record.sha,
        command: record.command,
        executor: record.executor,
        executedAt: record.executedAt,
        durationMs: record.durationMs,
        exitCode: record.exitCode,
        workingTreeDirty: record.workingTreeDirty ?? null,
        workingTreeStatusCount: record.workingTreeStatusCount ?? null,
        evidenceRefs: record.evidenceRefs,
      },
    )
    ensureEdge(family, node.id, localLayer, 'EXECUTED_AT')
  }

  const cleanStatus = summarizeStatus(evidence.cleanCurrentByCheck)
  const observedStatus = summarizeStatus(evidence.latestCurrentByCheck)
  const localStatus = evidence.currentWorkingTreeDirty
    ? observedStatus === 'NOT_RUN'
      ? 'NOT_RUN_DIRTY_WORKTREE'
      : `${observedStatus}_DIRTY_WORKTREE`
    : cleanStatus

  coverage.verification = {
    source: 'system_atlas_verification_projection',
    currentSha: evidence.currentSha,
    sourceEvidence: 'AVAILABLE',
    localExecution: {
      status: localStatus,
      assurance: evidence.currentWorkingTreeDirty ? 'WORKTREE_OBSERVATION_ONLY' : 'EXACT_CLEAN_SHA',
      workingTreeDirty: evidence.currentWorkingTreeDirty,
      workingTreeStatusCount: evidence.currentWorkingTreeStatusCount,
      checks: evidence.latestCurrentByCheck,
      exactCleanShaChecks: evidence.cleanCurrentByCheck,
      stats: evidence.stats,
    },
    hostedCi: {
      status: 'NOT_OBSERVED_BY_LOCAL_LEDGER',
      note: 'Hosted CI is an independent evidence provider and is not required for Atlas compilation.',
    },
    productionRuntime: {
      status: 'SEPARATE_EVIDENCE_SOURCE',
      note: 'Production runtime assurance comes from deployment/runtime evidence, not the local verification ledger.',
    },
  }

  family.nodes.sort((a, b) => a.id.localeCompare(b.id))
  family.edges.sort((a, b) => a.id.localeCompare(b.id))
  coverage.stats.familyStats.test_evidence = {
    nodes: family.nodes.length,
    edges: family.edges.length,
    artifacts: family.artifacts.length,
    gaps: family.gaps.length,
  }
  coverage.stats.totalNodes = Object.values(coverage.families).reduce((sum, item) => sum + item.nodes.length, 0)
  coverage.stats.totalEdges = Object.values(coverage.families).reduce((sum, item) => sum + item.edges.length, 0)
  return coverage
}
