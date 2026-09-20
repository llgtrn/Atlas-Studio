import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import test from 'node:test'
import { GRAPH_FAMILIES, validateSystemCoverage } from './system-coverage.mjs'
import { compileFullSystemCoverage } from './system-coverage-pipeline.mjs'
import { validateUnifiedSystemCoverage } from './system-coverage-unified.mjs'

function write(root, file, content) {
  mkdirSync(dirname(join(root, file)), { recursive: true })
  writeFileSync(join(root, file), content)
}

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-system-atlas-'))
  execFileSync('git', ['init', '-q'], { cwd: root })
  execFileSync('git', ['config', 'user.email', 'atlas@test.invalid'], { cwd: root })
  execFileSync('git', ['config', 'user.name', 'Atlas Test'], { cwd: root })

  write(root, 'Cargo.toml', '[workspace]\nmembers=["crates/runtime/test-runtime"]\n')
  write(root, 'crates/runtime/test-runtime/Cargo.toml', '[package]\nname="test-runtime"\nversion="0.1.0"\n[dependencies]\nserde="1"\n')
  write(root, 'crates/runtime/test-runtime/src/lib.rs', `// INV-AUTH-001\n// World canonicalization authority execution evidence relation binding\npub struct WorkRequest; pub struct WorkRun; fn authorize(){} fn evidence(){} fn append_event(){}\n`)
  write(root, 'crates/adapter/modbus/src/lib.rs', `// INV-SAFE-001\nfn modbus_control_plc(){ /* safety interlock */ }\n`)
  write(root, 'crates/infra/db/migrations/001.sql', 'CREATE TABLE events(id bigint); CREATE INDEX events_id_idx ON events(id);')
  write(root, 'apps/ui/package.json', JSON.stringify({ name: '@chronica/ui', scripts: { check: 'node ../../tools/check.mjs' }, dependencies: { react: '^19.0.0' } }))
  write(root, 'apps/ui/src/main.tsx', `import { App } from './App'; App();`)
  write(root, 'apps/ui/src/App.tsx', `import { Page } from './pages/Home'; export function App(){ return Page(); }`)
  write(root, 'apps/ui/src/pages/Home.tsx', `// authority\nexport function Page(){ fetch('/api/world'); return null }`)
  write(root, 'bindings/mcp-example.ts', `// mcp provider binding\nconst tool = "world.read"; const event_type = "world.changed"; const queue = "world.events"; void tool; void event_type; void queue;`)
  write(root, 'bindings/world.proto', `syntax = "proto3"; message WorldRequest {} message WorldReply {} service WorldService {\n  rpc ReadWorld(WorldRequest) returns (WorldReply);\n}\n`)
  write(root, 'tools/check.mjs', `console.log('check')`)
  write(root, '.github/workflows/check.yml', `jobs:\n  check:\n    steps:\n      - run: node tools/check.mjs\n      - run: cargo test\n`)
  write(root, 'deploy/production/docker-compose.yml', `services:\n  kernel:\n    image: chronica/kernel\n    healthcheck:\n      test: ["CMD", "true"]\n`)
  write(root, 'docs/ops_production/contracts/OPS-MANIFEST.md', 'DONOR_BASELINE semantic_convergence legacy_burndown absorption chronica_owner')
  write(root, 'bindings/provider.md', 'Claude provider binding maps to execution evidence authority')
  write(root, 'crates/runtime/test-runtime/tests/execution_test.rs', '// INV-AUTH-001\n#[test] fn execution_test(){}')
  write(root, 'crates/runtime/test-runtime/tests/integration/effect.rs', '// INV-AUTH-001\n#[test] fn integrated_effect(){}')
  write(root, 'crates/runtime/test-runtime/benches/admission_benchmark.rs', '// INV-AUTH-001\nfn benchmark(){}')
  write(root, 'README.md', 'Chronica World Resource State Capability Resolution Authority Execution Evidence Memory')
  write(root, '.gitignore', '.chronica/\n')

  execFileSync('git', ['add', '.'], { cwd: root })
  execFileSync('git', ['commit', '-qm', 'fixture'], { cwd: root })
  const fixtureSha = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim()

  write(root, '.chronica/deployment-evidence.json', JSON.stringify({ records: [{ environment: 'production', service: 'kernel', gitSha: 'fixture-sha', deploymentId: 'dep-1', healthStatus: 'PASS', observedAt: '2026-09-15T00:00:00Z' }] }))
  write(root, '.chronica/ops-evidence.json', JSON.stringify({ ops: [{ repo: 'ExampleOps', donorBaseline: 'donor@abc', semanticMappings: [{ canonicalTerm: 'conversation.close', matchStatus: 'EQUIVALENT_WITH_DIFFERENT_NAME', disposition: 'ALIAS_AT_BOUNDARY' }], legacyBurndown: { remaining: 2 }, absorptionCandidates: [{ id: 'conversation.close' }], chronicaDestinations: [{ owner: 'docs/architecture/governance/execution.md' }] }] }))
  write(root, '.chronica/verification-evidence.json', JSON.stringify({ schemaVersion: 1, source: 'local_execution', records: [{ id: `${fixtureSha}:atlas:system:test:fixture`, sha: fixtureSha, check: 'atlas:system:test', command: 'pnpm --filter @chronica/ui atlas:system:test', status: 'PASS', executor: 'fixture', environment: 'test', executedAt: '2026-09-15T00:00:00Z', durationMs: 10, exitCode: 0, workingTreeDirty: false, workingTreeStatusCount: 0, evidenceRefs: [] }] }))
  return root
}

function compiled(root = fixture()) {
  return compileFullSystemCoverage(root)
}

test('covers all required System Atlas graph families and all tracked artifacts', () => {
  const coverage = compiled()
  assert.deepEqual(coverage.graphFamilies, GRAPH_FAMILIES)
  assert.equal(coverage.stats.domainCoveragePercent, 100)
  assert.equal(coverage.stats.trackedArtifactCoveragePercent, 100)
  assert.equal(coverage.unclassifiedArtifacts.length, 0)
  assert.deepEqual(validateSystemCoverage(coverage), [])
  assert.deepEqual(validateUnifiedSystemCoverage(coverage), [])
})

test('discovers repository, HTTP/MCP/protobuf/event/queue, data, execution, deployment, Ops and machine evidence', () => {
  const coverage = compiled()
  const api = coverage.families.api_protocol
  assert.ok(coverage.families.repository.nodes.some((node) => node.kind === 'crate' && node.label === 'test-runtime'))
  assert.ok(api.nodes.some((node) => node.kind === 'http_route' && node.label === '/api/world'))
  assert.ok(api.nodes.some((node) => node.kind === 'mcp_tool' && node.label === 'world.read'))
  assert.ok(api.nodes.some((node) => node.kind === 'event' && node.label === 'world.changed'))
  assert.ok(api.nodes.some((node) => node.kind === 'queue_or_topic' && node.label === 'world.events'))
  assert.ok(api.nodes.some((node) => node.kind === 'proto_service' && node.label === 'WorldService'))
  assert.ok(api.nodes.some((node) => node.kind === 'proto_rpc' && node.label === 'WorldService.ReadWorld'))
  assert.ok(api.nodes.some((node) => node.kind === 'proto_message' && node.label === 'WorldRequest'))
  assert.ok(coverage.families.data.nodes.some((node) => node.kind === 'table' && node.label === 'events'))
  assert.ok(coverage.families.execution.edges.some((edge) => edge.type === 'IMPLEMENTS'))
  assert.ok(coverage.families.test_evidence.edges.some((edge) => edge.type === 'VERIFIED_BY'))
  assert.ok(coverage.families.deployment.nodes.some((node) => node.kind === 'service' && node.label === 'kernel'))
  assert.ok(coverage.families.ops.nodes.some((node) => node.kind === 'ops_instance' && node.label === 'ExampleOps'))
  assert.ok(coverage.families.machine_physical.nodes.some((node) => node.kind === 'machine_protocol' && node.label === 'modbus'))
  assert.ok(coverage.families.external_integration.nodes.some((node) => node.kind === 'provider_or_service' && node.label === 'claude'))
})

test('deepens World semantics, adapter execution, data ownership, runtime health and live Ops convergence', () => {
  const coverage = compiled()
  assert.ok(coverage.families.semantic.nodes.some((node) => node.kind === 'semantic_primitive' && node.label === 'world'))
  assert.ok(coverage.families.semantic.nodes.some((node) => node.kind === 'semantic_primitive' && node.label === 'relation_binding'))
  assert.ok(coverage.families.semantic.nodes.some((node) => node.kind === 'semantic_primitive' && node.label === 'canonicalization'))
  assert.ok(coverage.families.execution.nodes.some((node) => node.kind === 'execution_stage' && node.label === 'adapter'))
  assert.ok(coverage.families.execution.edges.some((edge) => edge.type === 'NEXT' && edge.source.endsWith('stage:work_run') && edge.target.endsWith('stage:adapter')))
  assert.ok(coverage.families.data.edges.some((edge) => edge.type === 'OWNS_DATA_OBJECT'))
  assert.ok(coverage.families.deployment.edges.some((edge) => edge.type === 'CONFIGURED_HEALTHCHECK'))
  assert.ok(coverage.families.deployment.edges.some((edge) => edge.type === 'OBSERVED_RUNTIME_HEALTH'))
  assert.ok(coverage.families.ops.nodes.some((node) => node.kind === 'donor_baseline'))
  assert.ok(coverage.families.ops.nodes.some((node) => node.kind === 'semantic_mapping'))
  assert.ok(coverage.families.ops.nodes.some((node) => node.kind === 'legacy_burndown'))
  assert.ok(coverage.families.ops.nodes.some((node) => node.kind === 'absorption_candidate'))
  assert.ok(coverage.families.ops.nodes.some((node) => node.kind === 'chronica_destination'))
})

test('enriches callers, UI chains, evidence chains, deployment promotion, Ops lifecycle and physical safety', () => {
  const coverage = compiled()
  assert.ok(coverage.families.repository.edges.some((edge) => edge.type === 'IMPORTED_BY'))
  assert.ok(coverage.families.ui.edges.some((edge) => edge.type === 'USES_COMPONENT'))
  assert.ok(coverage.families.ui.edges.some((edge) => edge.type === 'CALLS_API'))
  assert.ok(coverage.families.ui.edges.some((edge) => edge.type === 'BOUND_TO_SEMANTIC' || edge.type === 'INVOKES_SEMANTIC'))
  assert.ok(coverage.families.api_protocol.edges.some((edge) => edge.type === 'CALLS'))
  assert.ok(coverage.families.test_evidence.edges.some((edge) => edge.type === 'IMPLEMENTED_BY'))
  assert.ok(coverage.families.test_evidence.edges.some((edge) => edge.type === 'RUNS_TEST_SUITE'))
  assert.ok(coverage.families.deployment.edges.some((edge) => edge.type === 'PROMOTED_TO'))
  assert.ok(coverage.families.deployment.edges.some((edge) => edge.type === 'RUNS'))
  assert.ok(coverage.families.ops.edges.some((edge) => edge.type === 'NEXT'))
  assert.ok(coverage.families.machine_physical.edges.some((edge) => edge.type === 'USES_PROTOCOL'))
  assert.ok(coverage.families.machine_physical.edges.some((edge) => edge.type === 'SUBJECT_TO'))
  assert.ok(coverage.families.machine_physical.edges.some((edge) => edge.type === 'PROTECTS'))
})

test('separates unit, integration, benchmark and CI evidence stages without asserting a false total ordering', () => {
  const coverage = compiled()
  const family = coverage.families.test_evidence
  assert.ok(family.nodes.some((node) => node.kind === 'unit_test'))
  assert.ok(family.nodes.some((node) => node.kind === 'integration_test'))
  assert.ok(family.nodes.some((node) => node.kind === 'benchmark'))
  assert.ok(family.nodes.some((node) => node.kind === 'ci_workflow'))
  assert.ok(family.edges.some((edge) => edge.type === 'EVIDENCE_PIPELINE_NEXT'))
  assert.ok(family.edges.some((edge) => edge.type === 'COMPLEMENTED_BY'))
  assert.ok(family.edges.some((edge) => edge.type === 'EVIDENCE_LEVEL'))
  assert.ok(family.edges.some((edge) => edge.type === 'EXECUTES_AT'))
  assert.ok(!family.edges.some((edge) => edge.type === 'STRONGER_EVIDENCE'))
})

test('projects exact-SHA local execution without pretending hosted CI ran', () => {
  const coverage = compiled()
  const family = coverage.families.test_evidence
  assert.equal(coverage.verification.localExecution.status, 'PASS')
  assert.equal(coverage.verification.localExecution.checks[0].check, 'atlas:system:test')
  assert.equal(coverage.verification.hostedCi.status, 'NOT_OBSERVED_BY_LOCAL_LEDGER')
  assert.equal(coverage.verification.productionRuntime.status, 'SEPARATE_EVIDENCE_SOURCE')
  assert.ok(family.nodes.some((node) => node.kind === 'local_execution_evidence' && node.status === 'PASS'))
  assert.ok(family.nodes.some((node) => node.kind === 'evidence_provider' && node.label === 'local_execution'))
  assert.ok(family.edges.some((edge) => edge.type === 'ASSURANCE_NEXT'))
})

test('creates a unified cross-family graph with artifact, invariant, endpoint and semantic bridges', () => {
  const coverage = compiled()
  assert.ok(coverage.unified.stats.crossFamilyEdges > 0)
  assert.ok(coverage.unified.crossFamilyEdges.some((edge) => edge.type === 'SAME_ARTIFACT' || edge.type === 'PROJECTED_AS'))
  assert.ok(coverage.unified.crossFamilyEdges.some((edge) => edge.type === 'SAME_INVARIANT'))
  assert.ok(coverage.unified.crossFamilyEdges.some((edge) => edge.type === 'SAME_ENDPOINT'))
  assert.ok(coverage.unified.crossFamilyEdges.some((edge) => edge.type === 'SAME_SEMANTIC'))
  const nodes = new Map(coverage.unified.nodes.map((node) => [node.id, node]))
  assert.ok(coverage.unified.crossFamilyEdges.every((edge) => nodes.get(edge.source)?.family !== nodes.get(edge.target)?.family))
})