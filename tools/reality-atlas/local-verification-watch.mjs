import { existsSync, watch } from 'node:fs'
import { resolve } from 'node:path'
import { checksForPaths, normalizeChangedPath } from './local-verification-plan.mjs'
import { runAndRecord } from './local-verification.mjs'
import { installChronicaGitHooks } from '../git-hooks/install.mjs'

const WATCH_ROOTS = ['crates', 'apps', 'graph', 'bindings', 'deploy', 'tools', 'docs', '.github', 'scripts']
const ROOT_FILES = new Set(['README.md', 'AGENTS.md', 'Cargo.toml', 'Cargo.lock', 'package.json', 'pnpm-workspace.yaml', 'pnpm-lock.yaml', 'Dockerfile', 'deny.toml'])
const IGNORE_PARTS = ['/.git/', '/.chronica/', '/node_modules/', '/target/', '/dist/', '/storybook-static/']
const GENERATED = new Set(['apps/ui/public/docs-atlas.json'])
const DEBOUNCE_MS = 2500

function ignored(path) {
  const framed = `/${path.replaceAll('\\', '/')}/`
  return GENERATED.has(path) || IGNORE_PARTS.some((part) => framed.includes(part))
}

const root = process.cwd()
const hook = installChronicaGitHooks(root)
console.log(`[chronica] verify:watch started; post-commit hook=${hook.installed ? 'installed' : hook.reason}`)
console.log(`[chronica] debounce=${DEBOUNCE_MS}ms; .chronica/ is ignored by the verifier to prevent evidence feedback loops`)

let timer = null
let running = false
const pendingPaths = new Set()

function schedule(path) {
  if (!path || ignored(path)) return
  pendingPaths.add(path)
  if (timer) clearTimeout(timer)
  timer = setTimeout(flush, DEBOUNCE_MS)
}

async function flush() {
  timer = null
  if (running || pendingPaths.size === 0) return
  running = true
  const paths = [...pendingPaths]
  pendingPaths.clear()
  const checks = checksForPaths(root, paths, { mode: 'watch' })
  if (checks.length) console.log(`\n[chronica] verify:watch ${paths.length} path(s) → ${checks.map((item) => item.check).join(', ')}`)
  for (const spec of checks) {
    const { item } = runAndRecord(root, { ...spec, executor: 'verify-watch' })
    console.log(`[chronica] ${item.status} ${item.check} assurance=${item.workingTreeDirty ? 'WORKTREE_OBSERVATION_ONLY' : 'EXACT_CLEAN_SHA'}`)
  }
  running = false
  if (pendingPaths.size) {
    if (timer) clearTimeout(timer)
    timer = setTimeout(flush, DEBOUNCE_MS)
  }
}

for (const prefix of WATCH_ROOTS) {
  const absolute = resolve(root, prefix)
  if (!existsSync(absolute)) continue
  const watcher = watch(absolute, { recursive: true }, (_event, filename) => {
    if (!filename) return
    schedule(normalizeChangedPath(root, resolve(absolute, String(filename))))
  })
  watcher.on('error', (error) => console.error(`[chronica] watcher error ${prefix}: ${error.message}`))
}

for (const file of ROOT_FILES) {
  const absolute = resolve(root, file)
  if (!existsSync(absolute)) continue
  const watcher = watch(absolute, () => schedule(file))
  watcher.on('error', (error) => console.error(`[chronica] watcher error ${file}: ${error.message}`))
}

process.on('SIGINT', () => {
  console.log('\n[chronica] verify:watch stopped')
  process.exit(0)
})
