import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import test from 'node:test'
import { checksForPaths, defaultLocalVerificationChecks } from './local-verification-plan.mjs'

function write(root, file, content) {
  mkdirSync(dirname(join(root, file)), { recursive: true })
  writeFileSync(join(root, file), content)
}

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-verification-plan-'))
  execFileSync('git', ['init', '-q'], { cwd: root })
  write(root, 'runtime/Cargo.toml', '[package]\nname = "chronica-example"\nversion = "0.1.0"\n')
  write(root, 'runtime/src/lib.rs', 'pub fn x() {}\n')
  return root
}

function names(checks) {
  return checks.map((item) => item.check)
}

test('UI edits trigger Atlas and typecheck without unrelated docs checks', () => {
  const root = fixture()
  const result = names(checksForPaths(root, ['apps/ui/src/App.tsx']))
  assert.deepEqual(result, ['atlas:system:check', 'ui:typecheck', 'atlas:i18n:test'])
})

test('architecture docs trigger Atlas, docs and architecture contracts', () => {
  const root = fixture()
  const result = names(checksForPaths(root, ['docs/architecture/governance/execution.md']))
  assert.deepEqual(result, ['atlas:system:check', 'docs:check', 'arch:contracts'])
})

test('Rust edits target the owning Cargo package', () => {
  const root = fixture()
  const result = names(checksForPaths(root, ['runtime/src/lib.rs']))
  assert.deepEqual(result, ['atlas:system:check', 'cargo:check:chronica-example'])
})

test('edits under any of the four top-level backend roots (core/runtime/adapter/organism) trigger Atlas -- not just crates/, which no longer exists', () => {
  const root = fixture()
  for (const top of ['core', 'runtime', 'adapter', 'organism']) {
    const result = names(checksForPaths(root, [`${top}/src/lib.rs`]))
    assert.ok(result.includes('atlas:system:check'), `${top}/ edits must trigger atlas:system:check`)
  }
})

test('post-commit mode adds executable System Atlas regression proof', () => {
  const root = fixture()
  const result = names(checksForPaths(root, ['apps/ui/src/App.tsx'], { mode: 'post_commit' }))
  assert.deepEqual(result, ['atlas:system:test', 'atlas:system:check', 'ui:typecheck', 'atlas:i18n:test'])
})

test('full local suite adds workspace Cargo verification only when requested', () => {
  assert.ok(!names(defaultLocalVerificationChecks()).includes('cargo:check:workspace'))
  assert.ok(names(defaultLocalVerificationChecks({ full: true })).includes('cargo:check:workspace'))
})
