import test from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { tmpdir } from 'node:os'
import { compileToolingReality, validateToolingReality } from './tooling-audit.mjs'

function run(root, args, env = {}) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', env: { ...process.env, ...env } }).trim()
}

function write(root, path, content) {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, content, 'utf8')
}

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-tooling-audit-'))
  run(root, ['init', '-b', 'main'])
  run(root, ['config', 'user.email', 'fixture@example.test'])
  run(root, ['config', 'user.name', 'Fixture'])
  write(root, 'package.json', JSON.stringify({ scripts: { caps: 'node tools/capabilities/run.mjs' } }, null, 2))
  write(root, 'tools/capabilities/run.mjs', 'console.log("capability era")\n')
  write(root, 'tools/active/lib.mjs', 'export const active = true\n')
  write(root, 'src/use-active.mjs', "import { active } from '../tools/active/lib.mjs'\nconsole.log(active)\n")
  write(root, 'tools/manual/README.md', '# Manual tool\nRun manually for operator diagnostics.\n')
  write(root, 'tools/manual/inspect.sh', '#!/bin/sh\necho inspect\n')
  write(root, 'tools/first-l4/strike.sh', '#!/bin/sh\necho old milestone\n')
  run(root, ['add', '.'])
  run(root, ['commit', '-m', 'fixture'])
  return root
}

test('tooling reality separates automated, imported, manual, transitional and retirement candidates', () => {
  const root = fixture()
  const tooling = compileToolingReality(root)
  assert.deepEqual(validateToolingReality(tooling), [])
  const byPath = new Map(tooling.groups.map((group) => [group.path, group]))
  assert.equal(byPath.get('tools/capabilities')?.status, 'ACTIVE_BUT_TRANSITIONAL')
  assert.equal(byPath.get('tools/active')?.status, 'ACTIVE_IMPORTED')
  assert.equal(byPath.get('tools/manual')?.status, 'MANUAL_ENTRYPOINT')
  assert.equal(byPath.get('tools/first-l4')?.status, 'RETIREMENT_CANDIDATE')
  assert.ok(tooling.reviewCandidates.some((entry) => entry.path === 'tools/first-l4'))
  assert.equal(tooling.stats.rootLegacySurfaces, 0)
})

test('retired hidden root tooling surfaces are a hard hygiene violation', () => {
  const root = fixture()
  write(root, '.specify/memory/constitution.md', '# obsolete spec-kit constitution\n')
  write(root, '.claude/skills/speckit-plan/SKILL.md', '# obsolete provider-local skill\n')
  write(root, '.cargo/config.toml', '[alias]\ndevbuild = "run --package old"\n')
  run(root, ['add', '.'])
  run(root, ['commit', '-m', 'add retired hidden roots'])

  const tooling = compileToolingReality(root)
  assert.equal(tooling.stats.rootLegacySurfaces, 3)
  assert.equal(tooling.rootLegacySurfaces.length, 3)
  const errors = validateToolingReality(tooling)
  assert.ok(errors.some((error) => error.includes('.specify')))
  assert.ok(errors.some((error) => error.includes('.claude')))
  assert.ok(errors.some((error) => error.includes('.cargo')))
})
