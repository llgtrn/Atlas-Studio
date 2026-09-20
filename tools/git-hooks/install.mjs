import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { execFileSync } from 'node:child_process'

const MARKER = '# CHRONICA_MANAGED_POST_COMMIT_V1'

function git(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim()
}

export function installChronicaGitHooks(root = process.cwd()) {
  let repoRoot
  try {
    repoRoot = git(root, ['rev-parse', '--show-toplevel'])
  } catch {
    return { installed: false, reason: 'NOT_A_GIT_REPOSITORY' }
  }

  const hookPath = resolve(repoRoot, git(repoRoot, ['rev-parse', '--git-path', 'hooks/post-commit']))
  if (existsSync(hookPath)) {
    const existing = readFileSync(hookPath, 'utf8')
    if (!existing.includes(MARKER)) {
      return { installed: false, reason: 'UNMANAGED_POST_COMMIT_HOOK_EXISTS', hookPath }
    }
  }

  mkdirSync(dirname(hookPath), { recursive: true })
  const body = `#!/bin/sh\n${MARKER}\nROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0\ncd "$ROOT" || exit 0\nnode tools/reality-atlas/local-verification-post-commit.mjs\nSTATUS=$?\nif [ "$STATUS" -ne 0 ]; then\n  echo "[chronica] post-commit verification reported failures; commit is preserved and evidence was recorded." >&2\nfi\nexit 0\n`
  writeFileSync(hookPath, body, 'utf8')
  chmodSync(hookPath, 0o755)
  return { installed: true, reason: 'INSTALLED', hookPath }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const result = installChronicaGitHooks(process.cwd())
  console.log(JSON.stringify(result, null, 2))
  if (!result.installed && result.reason === 'UNMANAGED_POST_COMMIT_HOOK_EXISTS') {
    console.error('[chronica] Existing unmanaged post-commit hook was not overwritten.')
  }
}
