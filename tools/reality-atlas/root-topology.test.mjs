import test from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { tmpdir } from 'node:os'
import { compileRootTopology, validateRootTopology } from './root-topology.mjs'

function run(root, args) { return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim() }
function write(root, path, content) {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, content, 'utf8')
}
function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-root-topology-'))
  run(root, ['init', '-b', 'main'])
  run(root, ['config', 'user.email', 'fixture@example.test'])
  run(root, ['config', 'user.name', 'Fixture'])
  write(root, 'README.md', '# Chronica\n')
  write(root, 'Cargo.toml', '[workspace]\nmembers=[]\n')
  write(root, 'package.json', '{"private":true}\n')
  write(root, 'deploy/compose/docker-compose.yml', 'services: {}\n')
  write(root, 'docs/INDEX.md', '# Index\n')
  write(root, 'tools/check.mjs', 'console.log("ok")\n')
  run(root, ['add', '.'])
  run(root, ['commit', '-m', 'fixture'])
  return root
}

test('canonical root topology is accepted', () => {
  const root = fixture()
  const topology = compileRootTopology(root)
  assert.deepEqual(validateRootTopology(topology), [])
  assert.equal(topology.stats.retiredViolations, 0)
  assert.equal(topology.stats.donorResidues, 0)
  assert.equal(topology.stats.stalePathReferences, 0)
  assert.ok(topology.entries.some((entry) => entry.path === 'deploy' && entry.status === 'CANONICAL_DIR'))
})

test('retired crates/ and ops/ roots, stale deployment paths and donor deployment residue are rejected', () => {
  const root = fixture()
  write(root, 'crates/legacy/src/lib.rs', 'pub fn legacy() {}\n')
  write(root, 'ops/compose/docker-compose.yml', 'services: {}\n')
  write(root, 'deploy/production/env.example', 'PAPERCLIP_HOME=/paperclip\n')
  write(root, 'docs/old-runbook.md', 'docker compose -f ops/deploy/docker-compose.deploy.yml up\n')
  run(root, ['add', '.'])
  run(root, ['commit', '-m', 'legacy'])
  const topology = compileRootTopology(root)
  const errors = validateRootTopology(topology)
  assert.ok(errors.some((error) => error.includes('crates') && error.includes('RETIRED_BACKEND_ROOT')))
  assert.ok(errors.some((error) => error.includes('ops')))
  assert.ok(errors.some((error) => error.includes('PAPERCLIP_RESIDUE')))
  assert.ok(errors.some((error) => error.includes('ops/deploy/')))
})
