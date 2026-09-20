import test from 'node:test'
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import {
  parseContractsFromText,
  validateContract,
  validateContractSet,
  affectsContract,
  trackedArchitectureDocs,
} from './architecture-contract-lib.mjs'
import { canonicalArchitectureOwners } from '../docs/architecture-registry.mjs'

test('parses embedded architecture contract JSON', () => {
  const [contract] = parseContractsFromText(`
\`\`\`chronica-contract
{"id":"INV-TEST-001","kind":"invariant","owner":"test","severity":"hard","predicate":"x == x","runtime_owner":["crates/core/"],"verification":["property test"]}
\`\`\`
`, 'fixture.md')
  assert.equal(contract.id, 'INV-TEST-001')
  assert.equal(contract.__source, 'fixture.md')
})

test('rejects measured performance without executable evidence', () => {
  const errors = validateContract({
    id: 'ARCH-PERF-TEST-001', kind: 'performance', owner: 'test', severity: 'guardrail', gate: 'MEASURED',
    metric: 'latency', direction: 'max', runtime_owner: ['crates/runtime/'], paths: ['crates/runtime/'],
    hard_invariants: ['INV-TEST-001'], verification: ['benchmark'],
  })
  assert.ok(errors.some((error) => error.includes('threshold')))
  assert.ok(errors.some((error) => error.includes('ci.command')))
  assert.ok(errors.some((error) => error.includes('fixture')))
  assert.ok(errors.some((error) => error.includes('baseline')))
})

test('requires performance hard-invariant references to resolve', () => {
  const errors = validateContractSet([{
    id: 'ARCH-PERF-TEST-001', kind: 'performance', owner: 'test', severity: 'guardrail', gate: 'STRUCTURAL',
    metric: 'latency', direction: 'max', runtime_owner: ['crates/runtime/'], paths: ['crates/runtime/'],
    hard_invariants: ['INV-TEST-MISSING'], verification: ['benchmark later'], __source: 'fixture.md',
  }])
  assert.ok(errors.some((error) => error.includes('is not declared')))
})

test('requires hard_invariants to reference severity=hard invariants', () => {
  const errors = validateContractSet([
    {
      id: 'INV-TEST-GUARDRAIL-001', kind: 'invariant', owner: 'test', severity: 'guardrail',
      predicate: 'fixture invariant', runtime_owner: ['crates/runtime/'], verification: ['fixture'], __source: 'fixture.md',
    },
    {
      id: 'ARCH-PERF-TEST-001', kind: 'performance', owner: 'test', severity: 'guardrail', gate: 'STRUCTURAL',
      metric: 'latency', direction: 'max', runtime_owner: ['crates/runtime/'], paths: ['crates/runtime/'],
      hard_invariants: ['INV-TEST-GUARDRAIL-001'], verification: ['benchmark later'], __source: 'fixture.md',
    },
  ])
  assert.ok(errors.some((error) => error.includes('must reference severity=hard')))
})

test('selects performance contracts by affected repository responsibility', () => {
  const contract = { kind: 'performance', paths: ['crates/runtime/chronica-runtime-example/', 'tools/benchmark/'] }
  assert.equal(affectsContract(contract, ['crates/runtime/chronica-runtime-example/src/lib.rs']), true)
  assert.equal(affectsContract(contract, ['docs/README.md']), false)
})

test('canonical architecture registry exactly owns tracked architecture documents', () => {
  const tracked = trackedArchitectureDocs()
    .filter((path) => path !== 'docs/architecture/README.md')
    .sort()
  const registered = [...canonicalArchitectureOwners].sort()
  assert.deepEqual(tracked, registered, 'tracked architecture owner docs must exactly match the canonical registry')
})

test('canonical architecture owners exist and every declared contract is owner-bound and valid', () => {
  const contracts = []
  for (const file of canonicalArchitectureOwners) {
    assert.equal(existsSync(file), true, `missing canonical owner: ${file}`)
    const text = readFileSync(file, 'utf8')
    const parsed = parseContractsFromText(text, file)
    contracts.push(...parsed)
    for (const contract of parsed) {
      assert.deepEqual(validateContract(contract), [], `${file}: invalid contract ${contract.id}`)
    }
  }
  assert.deepEqual(validateContractSet(contracts), [])
})

test('contract owner cannot claim a different canonical owner document', () => {
  const errors = validateContractSet([{
    id: 'INV-TEST-OWNER-001',
    kind: 'invariant',
    owner: 'execution',
    severity: 'hard',
    predicate: 'fixture',
    runtime_owner: ['crates/runtime/'],
    verification: ['fixture'],
    __source: 'docs/architecture/governance/authority.md',
  }])
  assert.ok(errors.some((error) => error.includes("must match canonical document owner 'authority'")))
})

test('documentation quality is not defined by boilerplate quotas', () => {
  for (const file of canonicalArchitectureOwners) {
    const text = readFileSync(file, 'utf8')
    assert.ok(text.trim().length > 0, `${file}: must not be empty`)
  }
  // Intentionally no minimum character count, heading count, contract count,
  // Mermaid requirement, equation quota, frontmatter requirement, or fixed section list.
})

test('capability remains derived semantics, not a resurrected Capability layer', () => {
  assert.equal(existsSync('docs/architecture/foundation/capability.md'), false, 'legacy capability.md must stay retired')
  assert.equal(existsSync('docs/architecture/capability-resolution.md'), false, 'flat architecture owner path must stay retired')

  const path = 'docs/architecture/foundation/capability-resolution.md'
  const contracts = parseContractsFromText(readFileSync(path, 'utf8'), path)
  const byId = new Map(contracts.map((contract) => [contract.id, contract]))
  assert.ok(byId.has('ARCH-EQ-CAPRES-001'))
  assert.ok(byId.has('INV-CAPRES-001'))
  assert.ok(byId.has('ARCH-PERF-CAPRES-001'))

  const invariant = byId.get('INV-CAPRES-001')
  assert.match(invariant.predicate, /derived semantic resolution/i)
  assert.match(invariant.predicate, /never an independent canonical store/i)
  assert.match(invariant.predicate, /operational-authority grant/i)
  assert.match(invariant.predicate, /normative permission/i)
  assert.match(invariant.predicate, /execution-admission result/i)

  for (const contract of contracts) {
    assert.equal((contract.runtime_owner ?? []).includes('crates/cap/'), false, `${contract.id}: runtime_owner must not normalize crates/cap as permanent capability owner`)
  }
})

test('authority and execution machine contracts preserve semantic separation', () => {
  const authorityPath = 'docs/architecture/governance/authority.md'
  const executionPath = 'docs/architecture/governance/execution.md'
  const authority = parseContractsFromText(readFileSync(authorityPath, 'utf8'), authorityPath)
  const execution = parseContractsFromText(readFileSync(executionPath, 'utf8'), executionPath)
  const byId = new Map([...authority, ...execution].map((contract) => [contract.id, contract]))

  assert.ok(byId.has('INV-AUTH-001'))
  assert.ok(byId.has('INV-EXEC-001'))
  assert.ok(byId.has('INV-EXEC-ADMISSION-001'))
  assert.match(byId.get('INV-AUTH-001').predicate, /operational authority remains distinct/i)
  assert.match(byId.get('INV-AUTH-001').predicate, /final ExecutionAdmission/i)
  assert.match(byId.get('INV-EXEC-ADMISSION-001').predicate, /distinguishable inputs to one ExecutionAdmission boundary/i)
  assert.match(byId.get('INV-EXEC-001').predicate, /canonical World changes only through candidate-event canonicalization/i)
})

test('architecture root contains no duplicate semantic owners or compatibility copies', () => {
  for (const path of [
    'docs/architecture/world.md',
    'docs/architecture/authority.md',
    'docs/architecture/execution.md',
    'docs/architecture/organism.md',
    'docs/architecture/machine.md',
    'docs/architecture/NORTH-STAR.md',
    'docs/architecture/ENGINEERING-CONTRACT.md',
    'docs/architecture/compatibility',
  ]) assert.equal(existsSync(path), false, `duplicate/compatibility architecture path must stay retired: ${path}`)
})
