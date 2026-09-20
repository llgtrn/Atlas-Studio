import { asArray, text } from './util.mjs'
import { CAP_NAMES, CONTRACT_SCHEMA, CONTRACT_SCHEMA_VERSION, STANDARD_CATALOG } from './contract-vocab.mjs'
import { sha256Hex } from './contract-hash.mjs'

function ownedFlows(ir, cap) {
  return ir.flows.filter((flow) => text(flow.owner) === cap)
}

function participatingFlows(ir, cap) {
  return ir.flows.filter((flow) => text(flow.owner) !== cap && asArray(flow.participants).map(text).includes(cap))
}

function nextQueue(flows) {
  return [...flows]
    .filter((flow) => ['SPEC_ONLY', 'NO_RUNTIME', 'PARTIAL', 'RUNTIME_CONNECTED'].includes(flow.derived_status))
    .sort((a, b) => rank(b) - rank(a) || a.id.localeCompare(b.id))
}

export function rankFlow(flow) {
  let score = 0
  if (asArray(flow.golden).length) score += 1000
  if (flow.value === 'HIGH') score += 80
  if (flow.value === 'MEDIUM') score += 30
  if (['SPEC_ONLY', 'NO_RUNTIME', 'PARTIAL'].includes(flow.derived_status)) score += 200
  score += asArray(flow.missing_obligations).length * 15
  if (flow.derived_status === 'BLOCKED') score -= 10000
  return score
}

function rank(flow) {
  return rankFlow(flow)
}

export function renderCapPacket(envelope, ir, repositorySha, releaseId) {
  const cap = text(envelope.cap)
  const owned = ownedFlows(ir, cap)
  const participating = participatingFlows(ir, cap)
  const actionable = nextQueue(owned).slice(0, 8)
  const missing = owned.flatMap((flow) => asArray(flow.missing_obligations).map((po) => `${flow.id}:${po}`))
  const blocked = owned.filter((flow) => flow.derived_status === 'BLOCKED')
  const requiredPos = [...new Set(owned.flatMap((flow) => asArray(flow.obligations)))]
  const body = {
    schema: CONTRACT_SCHEMA,
    generated: true,
    source_of_truth: 'tools/benchmark/contracts/source',
    cap,
    name: CAP_NAMES[cap],
    benchmark_release: releaseId || null,
    benchmark_sha: repositorySha,
    contract_schema_version: CONTRACT_SCHEMA_VERSION,
    source_hash: ir.source_hash,
    ownership: asArray(envelope.owns),
    non_ownership: asArray(envelope.does_not_own),
    standards: envelope.standards || {},
    catalog_versions: Object.fromEntries(Object.entries(STANDARD_CATALOG).map(([k, v]) => [k, v.version])),
    families: asArray(envelope.families),
    owned_flow_ids: owned.map((flow) => flow.id),
    participating_flow_ids: participating.map((flow) => flow.id),
    hard_gates: asArray(envelope.hard_gates),
    anti_patterns: asArray(envelope.anti_patterns),
    forbidden_architecture: asArray(envelope.forbidden_architecture),
    flow_status: Object.fromEntries(owned.map((flow) => [flow.id, flow.derived_status])),
    missing_witnesses: missing,
    required_proof_obligations: requiredPos,
    blocked_flows: blocked.map((flow) => ({ id: flow.id, ...flow.blocked })),
    next_actionable: actionable.map((flow) => ({
      id: flow.id,
      status: flow.derived_status,
      missing: flow.missing_obligations,
      value: flow.value || 'MEDIUM',
      golden: asArray(flow.golden),
    })),
    negatives: owned.flatMap((flow) => asArray(flow.negative_cases).map((neg) => ({ flow: flow.id, case: neg }))),
    this_packet_is_not_source: true,
  }
  const hashable = { ...body }
  delete hashable.benchmark_sha
  const packet_hash = sha256Hex(hashable)
  const markdown = [
    `# ${cap} ${CAP_NAMES[cap]} — acceptance packet`,
    '',
    `GENERATED. Source of truth is Contract IR (${CONTRACT_SCHEMA}). Do not hand-edit.`,
    '',
    `- BENCHMARK_RELEASE: \`${releaseId || 'UNBOUND'}\``,
    `- BENCHMARK_SHA: \`${repositorySha}\``,
    `- SOURCE_HASH: \`${ir.source_hash}\``,
    `- PACKET_HASH: \`${packet_hash}\``,
    `- STANDARDS: ${JSON.stringify(body.standards)}`,
    '',
    '## Owns',
    ...asArray(envelope.owns).map((row) => `- ${row}`),
    '',
    '## Does not own',
    ...asArray(envelope.does_not_own).map((row) => `- ${row}`),
    '',
    '## Hard gates',
    ...asArray(envelope.hard_gates).map((row) => `- ${row}`),
    '',
    '## Owned flows',
    ...owned.map((flow) => `- \`${flow.id}\` ${flow.derived_status} missing=${asArray(flow.missing_obligations).join(',') || 'none'}`),
    '',
    '## Next actionable',
    ...actionable.map((flow) => `- \`${flow.id}\` (${flow.derived_status})`),
    '',
    'If the standard appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.',
    '',
  ].join('\n')
  return { ...body, packet_hash, markdown }
}
