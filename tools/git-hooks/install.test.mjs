import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import test from 'node:test'
import { installChronicaGitHooks } from './install.mjs'

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-hook-install-'))
  execFileSync('git', ['init', '-q'], { cwd: root })
  return root
}

function hookPath(root) {
  const relative = execFileSync('git', ['rev-parse', '--git-path', 'hooks/post-commit'], { cwd: root, encoding: 'utf8' }).trim()
  return resolve(root, relative)
}

test('installs a managed post-commit hook into an empty repository', () => {
  const root = fixture()
  const result = installChronicaGitHooks(root)
  assert.equal(result.installed, true)
  const content = readFileSync(hookPath(root), 'utf8')
  assert.match(content, /CHRONICA_MANAGED_POST_COMMIT_V1/)
  assert.match(content, /local-verification-post-commit\.mjs/)
})

test('does not overwrite an unmanaged post-commit hook', () => {
  const root = fixture()
  const hook = hookPath(root)
  mkdirSync(dirname(hook), { recursive: true })
  writeFileSync(hook, '#!/bin/sh\necho custom\n', 'utf8')
  const result = installChronicaGitHooks(root)
  assert.equal(result.installed, false)
  assert.equal(result.reason, 'UNMANAGED_POST_COMMIT_HOOK_EXISTS')
  assert.equal(readFileSync(hook, 'utf8'), '#!/bin/sh\necho custom\n')
})
