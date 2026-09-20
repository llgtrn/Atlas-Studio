import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { computeBenchmarkRatio, loadRegistry, loadShard, matchPortfolioEntries } from './query-benchmark-ratio.mjs'

const SCRIPT_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), 'query-benchmark-ratio.mjs')

// Runs the script as a REAL child process (not an import) so the module's own
// `if (... === ...) main()` entrypoint guard actually executes -- this is the only way to catch a
// regression like the one found on Windows/PowerShell controller audit of PR #2153, where
// `import.meta.url === \`file://${process.argv[1]}\`` was always false on Windows (argv[1] is a
// native `C:\...` path, not a `file:///C:/...` URL) and the CLI silently no-op'd with exit 0 and no
// output -- a bug that in-process unit tests calling computeBenchmarkRatio() directly could never see.
function runCli(args) {
  return spawnSync(process.execPath, [SCRIPT_PATH, ...args], { encoding: 'utf8' })
}

const EVIDENCE_VOCABULARY = new Set([
  'DOC_ONLY',
  'TRACKING_ONLY',
  'PRIMITIVE',
  'CODE_PRESENT',
  'RUNTIME_CLOSED',
  'CONFORMANCE_GREEN',
  'PRODUCTION_PROVEN',
  'LOCAL_AUDIT_REQUIRED',
])

test('registry carries exactly the 11 canonical platform.replicate_* families with no duplicates', () => {
  const registry = loadRegistry()
  const keys = registry.families.map((f) => f.capability_key)
  assert.equal(keys.length, 11)
  assert.equal(new Set(keys).size, 11)
  for (const key of keys) {
    assert.match(key, /^platform\.replicate_[a-z_]+$/)
  }
})

test('every registry family resolves against the live chronica-platform-plane shard', () => {
  const registry = loadRegistry()
  const shard = loadShard()
  for (const family of registry.families) {
    assert.ok(
      shard.capabilityByKey.has(family.capability_key),
      `${family.capability_key} missing from crates/chronica-platform-plane/.chronica/sub-cap-arch.jsonl`,
    )
  }
})

test('computeBenchmarkRatio never claims root capabilities.db or architecture.db authority', () => {
  const result = computeBenchmarkRatio()
  assert.equal(result.truth_label, 'LOCAL_AUDIT_REQUIRED')
  assert.ok(result.non_claims.some((c) => c.includes('docs/capabilities.db')))
  for (const family of result.families) {
    assert.equal(family.dimensions.capability_tracking.root_capabilities_db_present, false)
  }
})

test('computeBenchmarkRatio ratios are internally consistent (count <= denominator, denominator = total_families)', () => {
  const result = computeBenchmarkRatio()
  const total = result.ratios.total_families
  assert.equal(total, result.families.length)
  for (const [key, val] of Object.entries(result.ratios)) {
    if (key === 'total_families') continue
    assert.equal(val.denominator, total, `${key}.denominator should equal total_families`)
    assert.ok(val.count >= 0 && val.count <= total, `${key}.count out of range`)
  }
})

test('no family is claimed claim_ready without a filed claim', () => {
  const result = computeBenchmarkRatio()
  for (const family of result.families) {
    if (family.claim_ready) {
      assert.ok(family.claim_filed, `${family.family_key} is claim_ready but has no filed claim`)
    }
  }
})

test('has_shard_evidence status is always the contract-only variant, never a bare CONNECTED', () => {
  const result = computeBenchmarkRatio()
  for (const family of result.families) {
    assert.notEqual(family.dimensions.has_shard_evidence.status, 'CONNECTED')
  }
})

test('matchPortfolioEntries does not false-positive on a shared leading word (regression: "Open Design" vs "OpenAI Agents SDK")', () => {
  const family = { external_benchmarks: ['Open Design', 'AI Website Cloner'] }
  const portfolioEntries = [
    { id: 'openai-agents-sdk-model-abstraction', name: 'OpenAI Agents SDK model abstraction', repository: 'openai/openai-agents-python' },
    { id: 'vllm-openai-compatible-inference-server', name: 'vLLM OpenAI-compatible inference server', repository: 'vllm-project/vllm' },
  ]
  assert.deepEqual(matchPortfolioEntries(family, portfolioEntries), [])
})

test('matchPortfolioEntries matches an explicit family_keys pin without loosening single-token brand matching', () => {
  const family = { family_key: 'commerce_platform', external_benchmarks: ['Medusa'] }
  const portfolioEntries = [
    { id: 'medusa-engine', name: 'Medusa commerce engine', repository: 'medusajs/medusa', family_keys: ['commerce_platform'] },
    { id: 'unrelated-medusa-blog', name: 'A blog that mentions Medusa once', repository: 'example/blog' },
  ]
  const matched = matchPortfolioEntries(family, portfolioEntries)
  assert.equal(matched.length, 1)
  assert.equal(matched[0].id, 'medusa-engine')
})

test('matchPortfolioEntries does match a real, exact-phrase pinned entry', () => {
  const family = { external_benchmarks: ['LiteLLM provider router'] }
  const portfolioEntries = [
    { id: 'litellm-provider-router', name: 'LiteLLM provider router', repository: 'BerriAI/litellm' },
    { id: 'unrelated', name: 'Something else entirely', repository: 'foo/bar' },
  ]
  const matched = matchPortfolioEntries(family, portfolioEntries)
  assert.equal(matched.length, 1)
  assert.equal(matched[0].id, 'litellm-provider-router')
})

test('every family carries a valid docs/116 evidence_vocabulary_family_claim, and none reach RUNTIME_CLOSED-or-better family-wide', () => {
  const result = computeBenchmarkRatio()
  for (const family of result.families) {
    const label = family.world_class_gate.evidence_vocabulary_family_claim
    assert.ok(EVIDENCE_VOCABULARY.has(label), `${family.family_key} has invalid evidence_vocabulary_family_claim: ${label}`)
    assert.ok(
      !['RUNTIME_CLOSED', 'CONFORMANCE_GREEN', 'PRODUCTION_PROVEN'].includes(label),
      `${family.family_key} claims ${label} family-wide -- not evidenced this session`,
    )
  }
  assert.equal(result.ratios.evidence_vocabulary_runtime_closed_or_better.count, 0)
})

test('no family claims family-wide WAVE_ADMISSION_READY production evidence', () => {
  const result = computeBenchmarkRatio()
  assert.equal(result.wave_admission_ready, false)
  assert.equal(result.ratios.production_evidence_proven.count, 0)
  assert.equal(result.ratios.runtime_closure_live.count, 0)
})

test('required_matrix never fabricates a pin that is not a 40-char commit', () => {
  const result = computeBenchmarkRatio()
  for (const family of result.families) {
    const matrix = family.world_class_gate.required_matrix
    if (matrix.pinned_version_or_sha.startsWith('NOT_YET_PINNED')) {
      assert.match(matrix.verdict, /BENCHMARK_BLOCKED_WITH_EVIDENCE/)
    } else {
      assert.match(matrix.pinned_version_or_sha, /@[0-9a-f]{40}/i)
      assert.doesNotMatch(matrix.pinned_version_or_sha, /@(main|master|develop)\b/)
    }
  }
})

test('CLI smoke: `summary` subcommand, run as a real child process, prints the total_families line', () => {
  const { status, stdout, stderr } = runCli(['summary'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  assert.match(stdout, /benchmark-ratio: total_families=11/)
  assert.match(stdout, /named_and_mapped: 11\/11/)
})

test('CLI smoke: `full` subcommand, run as a real child process, prints parseable JSON with 11 families', () => {
  const { status, stdout, stderr } = runCli(['full'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  assert.ok(stdout.trim().length > 0, 'expected non-empty stdout')
  const parsed = JSON.parse(stdout)
  assert.equal(parsed.families.length, 11)
  assert.equal(parsed.ratios.total_families, 11)
})

test('CLI smoke: `family <key>` subcommand, run as a real child process, prints that family only', () => {
  const { status, stdout, stderr } = runCli(['family', 'trust_service'])
  assert.equal(status, 0, `expected exit 0, got ${status}; stderr: ${stderr}`)
  const parsed = JSON.parse(stdout)
  assert.equal(parsed.family_key, 'trust_service')
})

test('CLI smoke: with no subcommand, defaults to `summary` and still prints output', () => {
  const { status, stdout } = runCli([])
  assert.equal(status, 0)
  assert.match(stdout, /benchmark-ratio: total_families=11/)
})

test('has_code_substrate never counts a CONTRACT_ONLY family (regression: controller audit found CODE_SUBSTRATE_PRESENT and CONTRACT_ONLY merged into one CONNECTED bucket)', () => {
  const result = computeBenchmarkRatio()
  const contractOnlyFamilies = result.families.filter((f) => f.dimensions.has_code_path.status === 'CONNECTED_CONTRACT_ONLY')
  const codeSubstrateFamilies = result.families.filter((f) => f.dimensions.has_code_path.status === 'CONNECTED')
  assert.ok(contractOnlyFamilies.length >= 1, 'expected at least one CONTRACT_ONLY family (data_platform) to exercise this path')
  for (const f of contractOnlyFamilies) {
    assert.notEqual(f.dimensions.has_code_path.status, 'CONNECTED')
  }
  assert.equal(result.ratios.has_code_substrate.count, codeSubstrateFamilies.length)
  assert.equal(result.ratios.has_contract_only_substrate.count, contractOnlyFamilies.length)
  // has_code_substrate + has_contract_only_substrate must never double-count or omit any family
  // whose code_substrate is CODE_SUBSTRATE_PRESENT or CONTRACT_ONLY.
  assert.equal(
    result.ratios.has_code_substrate.count + result.ratios.has_contract_only_substrate.count,
    result.families.filter((f) => ['CONNECTED', 'CONNECTED_CONTRACT_ONLY'].includes(f.dimensions.has_code_path.status)).length,
  )
})
