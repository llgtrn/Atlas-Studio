import { execFileSync } from 'node:child_process'
import { runAndRecord } from './local-verification.mjs'
import { checksForPaths } from './local-verification-plan.mjs'

function changedPaths(root) {
  const output = execFileSync('git', ['diff-tree', '--no-commit-id', '--name-only', '-r', 'HEAD'], {
    cwd: root,
    encoding: 'utf8',
  })
  return output.split('\n').map((line) => line.trim()).filter(Boolean)
}

const root = process.cwd()
const paths = changedPaths(root)
const checks = checksForPaths(root, paths, { mode: 'post_commit' })

if (checks.length === 0) {
  console.log('[chronica] post-commit verification: no relevant checks for this commit')
  process.exit(0)
}

console.log(`[chronica] post-commit verification: ${checks.length} check(s) for ${paths.length} changed path(s)`)
let failed = false
for (const spec of checks) {
  console.log(`[chronica] ${spec.check}`)
  const { item } = runAndRecord(root, { ...spec, executor: 'git-post-commit' })
  console.log(`[chronica] ${item.status} ${item.check} assurance=${item.workingTreeDirty ? 'WORKTREE_OBSERVATION_ONLY' : 'EXACT_CLEAN_SHA'}`)
  if (item.status !== 'PASS') failed = true
}

if (failed) process.exitCode = 1
