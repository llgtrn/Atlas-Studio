import { CONTRACT_SCHEMA, CONTRACT_SCHEMA_VERSION, STANDARD_CATALOG } from './contract-vocab.mjs'
import { sha256Hex } from './contract-hash.mjs'

export function releaseIdFromParts(day, n = 1) {
  return `BR-${day}.${n}`
}

export function buildReleaseManifest({ ir, packetHashes, repositorySha, generatedAt, releaseId }) {
  const day = (generatedAt || '2026-08-21T00:00:00Z').slice(0, 10).replaceAll('-', '.')
  const id = releaseId || releaseIdFromParts(day, 1)
  const manifest = {
    schema: CONTRACT_SCHEMA,
    type: 'benchmark_release',
    id,
    contract_schema_version: CONTRACT_SCHEMA_VERSION,
    benchmark_sha: repositorySha,
    standard_versions: Object.fromEntries(Object.entries(STANDARD_CATALOG).map(([k, v]) => [k, v.version])),
    source_hash: ir.source_hash,
    flow_registry_hash: sha256Hex(ir.flows.map((flow) => ({ id: flow.id, owner: flow.owner, obligations: flow.obligations, hard_gates: flow.hard_gates }))),
    envelope_hashes: Object.fromEntries(ir.envelopes.map((env) => [env.cap, sha256Hex(env)])),
    packet_hashes: packetHashes,
    counts: {
      flows: ir.flows.length,
      envelopes: ir.envelopes.length,
      golden: ir.golden.length,
      laws: ir.laws.length,
      evidence: ir.evidence.length,
      cross_cap: ir.cross_cap.length,
    },
    this_release_is_not_product_authority: true,
  }
  return { ...manifest, hash: sha256Hex(manifest), generated_at: generatedAt || null }
}

export function verifyReleaseReproducible(a, b) {
  const errors = []
  for (const key of ['source_hash', 'flow_registry_hash', 'hash', 'id', 'contract_schema_version']) {
    if (a[key] !== b[key]) errors.push(`release ${key} drifted`)
  }
  return { ok: errors.length === 0, errors }
}

export function detectStaleBenchmark(pinnedReleaseId, currentReleaseId) {
  if (!pinnedReleaseId || !currentReleaseId) return { stale: true, tag: 'STALE_BENCHMARK' }
  if (pinnedReleaseId !== currentReleaseId) return { stale: true, tag: 'STALE_BENCHMARK' }
  return { stale: false, tag: null }
}
