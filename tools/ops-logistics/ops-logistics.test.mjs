import test from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync, appendFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { tmpdir } from 'node:os'
import { stringify as yamlStringify } from 'yaml'
import {
  buildAbsorptionPlan,
  buildDocsSyncProposal,
  collectOpsReality,
  deploymentReality,
  logisticsScorecard,
  validateSemanticConvergence,
  writeDeploymentEvidence,
} from './lib.mjs'
import { validateChronicaEvidence } from './chronica-evidence.mjs'

const chronicaRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..')

function run(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim()
}

function write(root, path, content) {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, content, 'utf8')
}

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-ops-logistics-'))
  run(root, ['init', '-b', 'main'])
  run(root, ['config', 'user.email', 'ops-fixture@example.test'])
  run(root, ['config', 'user.name', 'Ops Fixture'])
  write(root, '.gitignore', '.chronica/\n')
  write(root, 'src/adapters/provider.ts', 'export const providerAction = "scan_state"\n')
  write(root, 'LICENSE', 'fixture\n')
  run(root, ['add', '.'])
  run(root, ['commit', '-m', 'donor baseline'])
  const baselineSha = run(root, ['rev-parse', 'HEAD'])
  const chronicaSha = run(chronicaRoot, ['rev-parse', 'HEAD'])

  write(root, 'src/core/service.ts', 'export const semantic = "implementation.observe"\n')
  write(root, 'src/core/service.test.ts', 'export const expectedSemantic = "implementation.observe"\n')
  for (const path of [
    'docs/README.md',
    'docs/INDEX.md',
    'docs/TEMPLATE.md',
    'docs/architecture/README.md',
    'docs/architecture/foundation/README.md',
    'docs/blueprints/README.md',
    'docs/decisions/README.md',
    'docs/guides/README.md',
    'docs/references/README.md',
  ]) write(root, path, `# ${path}\n`)

  const manifest = {
    manifest_version: 5,
    ops: { id: 'fixture-ops', name: 'Chronica Fixture Ops', version: '1.0.0', maturity: 'STANDALONE_PROVEN', product_identity: 'chronica_branded' },
    donor: {
      required: true,
      complete_tree_accounted: true,
      repositories: [{ url: 'https://example.invalid/donor.git', commit: baselineSha, license: 'fixture' }],
      ops_git_baseline: { imported: true, baseline_commit: baselineSha, baseline_tag_or_branch: 'donor-baseline/fixture', remote_pushed: true, baseline_build_status: 'passed' },
    },
    legacy_burndown: {
      status: 'ZERO_DONOR_LEGACY_IMPLEMENTATION',
      donor_legacy_files_or_modules_remaining: 0,
      donor_public_callers_remaining: 0,
      donor_db_or_migration_ownership_remaining: 0,
      donor_ui_routes_remaining: 0,
      donor_background_jobs_remaining: 0,
      donor_policy_authority_paths_remaining: 0,
      donor_docs_brand_surfaces_remaining: 0,
      duplicate_semantics_remaining: 0,
      compatibility_aliases_remaining: 0,
      terminal_target: 'ZERO_DONOR_LEGACY_IMPLEMENTATION_AND_ZERO_UNEXPLAINED_DUPLICATE_SEMANTICS',
    },
    chronica_reference: {
      repository: 'llgtrn/Chronica',
      ref: 'fixture',
      sha: chronicaSha,
      owners_read: ['docs/architecture/operations/fabric.md'],
      code_paths_read: ['tools/reality-atlas/lib.mjs'],
      test_paths_read: ['tools/reality-atlas/reality-atlas.test.mjs'],
      conflict_audit_status: 'proven',
      conflicts_found: [],
      decisions: [],
    },
    semantic_convergence: {
      status: 'proven',
      contract: 'docs/ops_production/contracts/SEMANTIC-CONVERGENCE.md',
      chronica_sha: chronicaSha,
      mappings: [{
        semantic_id: 'implementation.observe',
        donor_terms: ['scan_state'],
        ops_term_before: 'scan_state',
        fingerprint: { resource: 'implementation', transition: 'unobserved_to_observed', effect: 'observe_implementation', authority: 'read', evidence: 'scan_result', idempotency: 'safe', unknown_outcome: 'retry' },
        match_status: 'EQUIVALENT_WITH_DIFFERENT_NAME',
        disposition: 'ALIAS_AT_BOUNDARY',
        canonical_term_after: 'implementation.observe',
        chronica_owner: 'docs/architecture/operations/fabric.md',
        chronica_code_refs: ['tools/reality-atlas/lib.mjs'],
        chronica_test_refs: ['tools/reality-atlas/reality-atlas.test.mjs'],
        temporary_aliases: ['scan_state'],
        extension_gap: null,
        retirement_target: 'keep_scan_state_only_in_provider_adapter',
      }],
      unmapped_semantics: [],
      unexplained_duplicate_semantics: [],
      extension_candidates: [],
      compatibility_aliases: ['scan_state'],
    },
    documentation: { governance_status: 'proven' },
    logistics: {
      contract_version: 1,
      reality: { mode: 'observed_working_tree' },
      docs_sync: { mode: 'proposal_only' },
      deployment_evidence: { path: '.chronica/deployment-evidence.json', required_environments: ['staging', 'production'] },
    },
  }
  write(root, 'ops.manifest.yaml', yamlStringify(manifest))
  run(root, ['add', '.'])
  run(root, ['commit', '-m', 'refound fixture ops'])
  const headSha = run(root, ['rev-parse', 'HEAD'])

  for (const environment of ['staging', 'production']) {
    writeDeploymentEvidence(root, manifest, {
      environment,
      gitSha: headSha,
      deploymentId: `${environment}-deploy-1`,
      buildId: `${environment}-build-1`,
      runtimeVersion: 'fixture-1',
      artifactDigest: `sha256:${environment}`,
      healthStatus: 'PASS',
      evidenceRefs: [`fixture://${environment}`],
      observedAt: new Date().toISOString(),
    })
  }
  return { root, manifest, baselineSha, headSha, chronicaSha }
}

test('Ops Reality Mirror observes donor baseline, docs, semantics and deployment evidence', () => {
  const fx = fixture()
  const reality = collectOpsReality(fx.root)
  assert.equal(reality.baseline.exists, true)
  assert.equal(reality.docs.complete, true)
  assert.equal(reality.legacy.total, 0)
  assert.equal(reality.semantics.length, 1)
  assert.equal(reality.semantics[0].internalDonorReferences.length, 0)
  assert.equal(reality.deployment.environments.filter((entry) => entry.status === 'CURRENT_HEAD').length, 2)
  assert.equal(reality.drift.filter((entry) => entry.severity === 'error').length, 0)
})

test('Semantic Convergence Checker proves exact Chronica SHA paths and rejects internal donor alias leakage', () => {
  const fx = fixture()
  let reality = collectOpsReality(fx.root)
  const semantic = validateSemanticConvergence(fx.manifest, reality)
  const chronica = validateChronicaEvidence(chronicaRoot, fx.manifest)
  assert.deepEqual(semantic.errors, [])
  assert.deepEqual(chronica.errors, [])
  assert.ok(chronica.checkedPaths >= 3)

  appendFileSync(join(fx.root, 'src/core/service.ts'), 'export const badLegacyAlias = "scan_state"\n')
  reality = collectOpsReality(fx.root)
  const bad = validateSemanticConvergence(fx.manifest, reality)
  assert.ok(bad.errors.some((error) => error.includes('leaks into non-boundary production source')))
})

test('Absorption Planner, Docs Sync Proposal and scorecard compose from the same observed state', () => {
  const fx = fixture()
  const reality = collectOpsReality(fx.root)
  const semantic = validateSemanticConvergence(fx.manifest, reality)
  const chronica = validateChronicaEvidence(chronicaRoot, fx.manifest)
  const combined = { ...semantic, errors: [...semantic.errors, ...chronica.errors], warnings: [...semantic.warnings, ...chronica.warnings] }
  const plan = buildAbsorptionPlan(fx.manifest, reality, combined)
  assert.equal(plan.readiness, 'READY_FOR_BOUNDED_ABSORPTION')
  assert.equal(plan.actions[0].action, 'TRANSLATE_ALIAS_AND_RETIRE_INTERNAL_DUPLICATE')

  const docs = buildDocsSyncProposal(fx.manifest, reality, plan, null)
  assert.equal(docs.mutationPolicy, 'PROPOSAL_ONLY_NEVER_AUTO_REWRITE_CANONICAL_DOCS')
  assert.equal(docs.docsSyncRequired, false)

  const score = logisticsScorecard({ manifest: fx.manifest, reality, semanticCheck: combined, absorptionPlan: plan, docsProposal: docs })
  assert.equal(score.score, score.total)
  assert.equal(score.percent, 100)
})

test('Docs Sync Proposal requests canonical review for a richer reusable Ops semantic', () => {
  const fx = fixture()
  const richer = structuredClone(fx.manifest)
  richer.semantic_convergence.mappings[0].match_status = 'CHRONICA_NARROWER'
  richer.semantic_convergence.mappings[0].disposition = 'EXTEND_CHRONICA_SEMANTIC'
  richer.semantic_convergence.mappings[0].extension_gap = 'Ops proves a reusable distinction not represented in the current canonical owner.'
  const reality = collectOpsReality(fx.root)
  const semantic = validateSemanticConvergence(richer, reality)
  const plan = buildAbsorptionPlan(richer, reality, semantic)
  const proposal = buildDocsSyncProposal(richer, reality, plan, null)
  assert.equal(proposal.docsSyncRequired, true)
  assert.equal(proposal.proposals[0].kind, 'CANONICAL_SEMANTIC_EXTENSION')
})

test('Deployment evidence marks older runtime SHA as behind the current Ops head', () => {
  const fx = fixture()
  const record = {
    environment: 'staging',
    gitSha: fx.baselineSha,
    deploymentId: 'staging-old',
    buildId: 'old',
    runtimeVersion: 'old',
    artifactDigest: 'sha256:old',
    healthStatus: 'PASS',
    evidenceRefs: ['fixture://old'],
    observedAt: new Date(Date.now() + 1000).toISOString(),
  }
  writeDeploymentEvidence(fx.root, fx.manifest, record)
  const deployment = deploymentReality(fx.root, fx.manifest, fx.headSha)
  assert.equal(deployment.environments.find((entry) => entry.environment === 'staging').status, 'OLDER_REPO_COMMIT')
})
