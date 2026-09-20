import { asArray, text } from './util.mjs'
import { CI_TIERS } from './contract-vocab.mjs'

const GLOBAL_HARD_PREFIXES = [
  'crates/chronica-security/',
  'crates/chronica-vault/',
  'crates/chronica-policy/',
  'crates/chronica-identity/',
  'crates/chronica-approvals/',
  'crates/chronica-authorization/',
]

function matchPrefix(path, prefixes) {
  return asArray(prefixes).some((prefix) => path === prefix || path.startsWith(prefix))
}

export function affectedFromPaths(ir, changedPaths) {
  const paths = asArray(changedPaths).map(text).filter(Boolean)
  const routing = asArray(ir.routing?.prefixes)
  const caps = new Set()
  for (const path of paths) {
    for (const row of routing) {
      if (matchPrefix(path, [row.prefix])) caps.add(text(row.cap))
    }
    for (const env of ir.envelopes || []) {
      if (matchPrefix(path, env.path_prefixes)) caps.add(text(env.cap))
    }
    for (const ev of ir.evidence || []) {
      if (text(ev.path) && (path === ev.path || path.startsWith(text(ev.path)) || text(ev.path).startsWith(path))) {
        for (const link of asArray(ev.proves)) {
          const flow = ir.flows.find((row) => row.id === text(link.flow))
          if (flow) {
            caps.add(text(flow.owner))
            for (const cap of asArray(flow.participants).map(text)) caps.add(cap)
          }
        }
      }
    }
  }
  return [...caps].filter(Boolean).sort()
}

export function affectedFlows(ir, caps) {
  const set = new Set(asArray(caps).map(text))
  return ir.flows.filter((flow) => set.has(text(flow.owner)) || asArray(flow.participants).map(text).some((cap) => set.has(cap)))
}

export function affectedGolden(ir, flowIds) {
  const set = new Set(asArray(flowIds).map(text))
  return ir.golden.filter((gbf) => asArray(gbf.flow_ids).some((id) => set.has(text(id))))
}

export function selectCiTiers(ir, changedPaths, caps, goldenHits) {
  const paths = asArray(changedPaths).map(text)
  const tiers = new Set(['TIER0_CONTRACT'])
  const contractOnly = paths.length > 0 && paths.every((path) => path.startsWith('tools/benchmark/'))
  if (!contractOnly && caps.length) tiers.add('TIER1_AFFECTED_CAP')
  if (paths.some((path) => GLOBAL_HARD_PREFIXES.some((prefix) => path.startsWith(prefix)))) tiers.add('TIER2_GLOBAL_HARD')
  if (goldenHits.length) tiers.add('TIER3_GOLDEN')
  if (paths.some((path) => path.startsWith('docs/benchmarks/'))) tiers.add('TIER4_FULL_TRUTH')
  return CI_TIERS.filter((tier) => tiers.has(tier))
}

export function buildCiMatrix(ir, changedPaths = []) {
  const seedCaps = affectedFromPaths(ir, changedPaths)
  const flows = affectedFlows(ir, seedCaps)
  const caps = [...new Set([
    ...seedCaps,
    ...flows.flatMap((flow) => [text(flow.owner), ...asArray(flow.participants).map(text)]),
  ])].filter(Boolean).sort()
  const golden = affectedGolden(ir, flows.map((flow) => flow.id))
  const tiers = selectCiTiers(ir, changedPaths, caps, golden)
  const commands = ['pnpm benchmark:contracts:verify']
  if (tiers.includes('TIER1_AFFECTED_CAP')) commands.push('pnpm benchmark:flows')
  if (caps.includes('CAP15')) commands.push('pnpm benchmark:web2app:test')
  if (caps.includes('CAP16')) commands.push('pnpm benchmark:employment:test')
  if (caps.includes('CAP17')) commands.push('pnpm benchmark:store:test')
  if (caps.includes('CAP18')) commands.push('pnpm benchmark:blackbox:test')
  if (tiers.includes('TIER4_FULL_TRUTH')) commands.push('pnpm benchmark:truth:test')
  return {
    changed_paths: asArray(changedPaths),
    required_ci_tier: tiers[tiers.length - 1] || 'TIER0_CONTRACT',
    tiers,
    affected_caps: caps,
    affected_flows: flows.map((flow) => flow.id),
    affected_golden_flows: golden.map((row) => row.id),
    required_commands: [...new Set(commands)],
    p0_authoritative_ci_touched: false,
    note: 'Benchmark runners only. Do not alter P0/#4762 workflows from this matrix.',
  }
}
