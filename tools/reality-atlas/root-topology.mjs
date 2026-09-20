import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { extname, join, posix as path } from 'node:path'

// 2026-09-16 hard refoundation: crates/ is retired (see docs/decisions/0016-legacy-backend-
// retired-to-git-history.md); core/, runtime/, adapter/, organism/ are its top-level
// replacements. temporary/ and license/ are also permanent canonical roots (AGENTS.md section 4
// repository-responsibility table) that this set never listed even before the reset.
const CANONICAL_DIRS = new Set([
  '.github', 'apps', 'bindings', 'core', 'runtime', 'adapter', 'organism', 'deploy', 'docs',
  'graph', 'license', 'temporary', 'tools',
])
const CANONICAL_FILES = new Set([
  '.dockerignore', '.env.example', '.gitattributes', '.gitguardian.yaml', '.gitignore', '.npmrc',
  'AGENTS.md', 'Cargo.lock', 'Cargo.toml', 'Dockerfile', 'README.md', 'deny.toml', 'package.json',
  'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'tsconfig.base.json',
])
const RETIRED_ROOTS = new Map([
  ['.cargo', 'HIDDEN_CARGO_CONTROL_SURFACE'],
  ['.claude', 'PROVIDER_LOCAL_PROJECT_SCAFFOLD'],
  ['.specify', 'SPECKIT_PROJECT_SCAFFOLD'],
  ['.mailmap', 'DONOR_IDENTITY_MAILMAP'],
  ['crates', 'RETIRED_BACKEND_ROOT'],
  ['ops', 'AMBIGUOUS_LEGACY_DEPLOYMENT_ROOT'],
  ['legacy', 'LEGACY_PRODUCT_ROOT'],
  ['server', 'LEGACY_SERVER_ROOT'],
  ['cli', 'LEGACY_CLI_ROOT'],
  ['Temporary', 'DONOR_STAGING_ROOT'],
  ['.paperclip', 'DONOR_LOCAL_STATE_ROOT'],
])
const SCAN_ROOT_FILES = new Set(['.env.example', '.dockerignore', '.gitguardian.yaml', 'Dockerfile', 'pnpm-workspace.yaml'])
const DONOR_RESIDUES = [
  { id: 'PAPERCLIP_RESIDUE', re: /paperclip/i },
  { id: 'OPENCLAW_SMOKE_RESIDUE', re: /openclaw-smoke/i },
  { id: 'LEGACY_TS_UPSTREAM_RESIDUE', re: /CHRONICA_TS_UPSTREAM/ },
  { id: 'BETTER_AUTH_RESIDUE', re: /BETTER_AUTH_SECRET/ },
]
const STALE_DEPLOY_PATHS = ['ops/compose/', 'ops/deploy/', 'ops/postgres/', 'ops/container/']
const TEXT_EXTENSIONS = new Set(['.cjs', '.css', '.html', '.js', '.json', '.jsx', '.md', '.mjs', '.ps1', '.py', '.rs', '.sh', '.sql', '.toml', '.ts', '.tsx', '.txt', '.yaml', '.yml'])
const MAX_TEXT_BYTES = 1024 * 1024

function slash(value) { return String(value).replaceAll('\\', '/') }
function gitLines(root, args) {
  // maxBuffer: see tools/reality-atlas/lib.mjs's identical gitLines fix -- the tracked donor
  // corpus under temporary/ (736k+ files) exceeds the default 1MB buffer on a plain `git ls-files`.
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 })
    .split('\n')
    .map((line) => slash(line.trim()))
    .filter(Boolean)
}
function safeRead(root, file) {
  try {
    const absolute = join(root, file)
    if (!existsSync(absolute) || statSync(absolute).size > MAX_TEXT_BYTES) return ''
    return readFileSync(absolute, 'utf8')
  } catch { return '' }
}
function topEntry(file) { return file.split('/')[0] }
function unique(values) { return [...new Set(values.filter(Boolean))].sort() }
function isTextCarrier(file) {
  return TEXT_EXTENSIONS.has(extname(file).toLowerCase()) || ['Cargo.toml', 'package.json', 'Dockerfile', 'Caddyfile', 'Makefile'].includes(path.basename(file))
}
function isDeploymentScanFile(file) {
  if (SCAN_ROOT_FILES.has(file)) return true
  if (!file.startsWith('deploy/')) return false
  return isTextCarrier(file) || extname(file).toLowerCase() === '.example'
}
function isGovernanceDefinition(file) {
  return file === 'tools/reality-atlas/root-topology.mjs' || file === 'tools/reality-atlas/root-topology.test.mjs'
}

export function compileRootTopology(root) {
  const tracked = gitLines(root, ['ls-files'])
  const topEntries = unique(tracked.map(topEntry))
  const entries = topEntries.map((entry) => {
    const files = tracked.filter((file) => topEntry(file) === entry)
    let status = 'REVIEW_ROOT'
    if (RETIRED_ROOTS.has(entry)) status = 'RETIRED_VIOLATION'
    else if (CANONICAL_DIRS.has(entry)) status = 'CANONICAL_DIR'
    else if (CANONICAL_FILES.has(entry)) status = 'CANONICAL_FILE'
    else if (entry.startsWith('.')) status = 'REVIEW_CONFIG'
    return { path: entry, status, trackedFiles: files.length, ruleId: RETIRED_ROOTS.get(entry) ?? null }
  })

  const retiredViolations = entries.filter((entry) => entry.status === 'RETIRED_VIOLATION')
  const reviewEntries = entries.filter((entry) => entry.status === 'REVIEW_ROOT' || entry.status === 'REVIEW_CONFIG')
  const donorResidues = []
  for (const file of tracked.filter(isDeploymentScanFile)) {
    const text = safeRead(root, file)
    if (!text) continue
    for (const rule of DONOR_RESIDUES) {
      if (rule.re.test(text)) donorResidues.push({ id: rule.id, path: file })
    }
  }

  const stalePathReferences = []
  for (const file of tracked.filter(isTextCarrier)) {
    if (isGovernanceDefinition(file)) continue
    const text = safeRead(root, file)
    if (!text) continue
    for (const stalePath of STALE_DEPLOY_PATHS) {
      if (text.includes(stalePath)) stalePathReferences.push({ path: file, stalePath })
    }
  }

  return {
    source: 'observed tracked repository root and deployment configuration; generated hygiene projection, never canonical truth',
    entries,
    retiredViolations,
    reviewEntries,
    donorResidues,
    stalePathReferences,
    stats: {
      rootEntries: entries.length,
      canonicalDirs: entries.filter((entry) => entry.status === 'CANONICAL_DIR').length,
      canonicalFiles: entries.filter((entry) => entry.status === 'CANONICAL_FILE').length,
      reviewEntries: reviewEntries.length,
      retiredViolations: retiredViolations.length,
      donorResidues: donorResidues.length,
      stalePathReferences: stalePathReferences.length,
    },
  }
}

export function validateRootTopology(topology) {
  const errors = []
  if (!topology || !Array.isArray(topology.entries)) errors.push('root topology entries must be an array')
  for (const violation of topology?.retiredViolations ?? []) {
    errors.push(`retired repository root must not exist: ${violation.path} (${violation.ruleId})`)
  }
  for (const residue of topology?.donorResidues ?? []) {
    errors.push(`donor/legacy deployment residue ${residue.id} remains in ${residue.path}`)
  }
  for (const reference of topology?.stalePathReferences ?? []) {
    errors.push(`stale deployment path ${reference.stalePath} remains referenced by ${reference.path}`)
  }
  return errors
}
