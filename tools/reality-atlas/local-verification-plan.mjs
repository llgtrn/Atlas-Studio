import { dirname, isAbsolute, join, relative, resolve } from 'node:path'
import { existsSync, readFileSync } from 'node:fs'

const PNPM = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
const CARGO = process.platform === 'win32' ? 'cargo.exe' : 'cargo'

export const CHECKS = {
  'atlas:system:test': { check: 'atlas:system:test', command: PNPM, args: ['--filter', '@chronica/ui', 'atlas:system:test'] },
  'atlas:system:check': { check: 'atlas:system:check', command: PNPM, args: ['--filter', '@chronica/ui', 'atlas:system:check'] },
  'atlas:check': { check: 'atlas:check', command: PNPM, args: ['--filter', '@chronica/ui', 'atlas:check'] },
  'atlas:i18n:test': { check: 'atlas:i18n:test', command: PNPM, args: ['--filter', '@chronica/ui', 'atlas:i18n:test'] },
  'ui:typecheck': { check: 'ui:typecheck', command: PNPM, args: ['--filter', '@chronica/ui', 'typecheck'] },
  'docs:check': { check: 'docs:check', command: PNPM, args: ['docs:check'] },
  'arch:contracts': { check: 'arch:contracts', command: PNPM, args: ['arch:contracts'] },
  'refoundation:no-temporary-dependency': { check: 'refoundation:no-temporary-dependency', command: PNPM, args: ['refoundation:no-temporary-dependency'] },
  'system-atlas:check': { check: 'system-atlas:check', command: PNPM, args: ['system-atlas:check'] },
  'cargo:check:workspace': { check: 'cargo:check:workspace', command: CARGO, args: ['check', '--workspace'] },
}

function slash(value) {
  return String(value).replaceAll('\\', '/')
}

function uniqueChecks(checks) {
  const seen = new Set()
  return checks.filter((spec) => {
    if (!spec || seen.has(spec.check)) return false
    seen.add(spec.check)
    return true
  })
}

function nearestCargoPackage(root, file) {
  if (!file.endsWith('.rs') && !file.endsWith('Cargo.toml')) return null
  let current = resolve(root, dirname(file))
  const rootResolved = resolve(root)
  while (current.startsWith(rootResolved)) {
    const manifest = join(current, 'Cargo.toml')
    if (existsSync(manifest)) {
      const text = readFileSync(manifest, 'utf8')
      const pkg = text.match(/\[package\][\s\S]*?^name\s*=\s*["']([^"']+)["']/m)?.[1]
      if (pkg) return pkg
    }
    if (current === rootResolved) break
    current = dirname(current)
  }
  return null
}

function cargoChecks(root, paths) {
  if (paths.some((path) => path === 'Cargo.toml' || path === 'Cargo.lock')) return [CHECKS['cargo:check:workspace']]
  const packages = new Set(paths.map((path) => nearestCargoPackage(root, path)).filter(Boolean))
  return [...packages].sort().map((name) => ({
    check: `cargo:check:${name}`,
    command: CARGO,
    args: ['check', '-p', name],
  }))
}

export function checksForPaths(root, inputPaths, { mode = 'watch' } = {}) {
  const paths = [...new Set(inputPaths.map(slash).filter(Boolean))]
  const checks = []
  const any = (predicate) => paths.some(predicate)

  const ui = any((path) => path.startsWith('apps/ui/'))
  const docs = any((path) => path.startsWith('docs/') || path === 'README.md' || path === 'AGENTS.md')
  const architectureDocs = any((path) => path.startsWith('docs/architecture/') || path === 'AGENTS.md' || path === 'docs/INDEX.md')
  const rust = any((path) => path.endsWith('.rs') || path.endsWith('Cargo.toml') || path === 'Cargo.toml' || path === 'Cargo.lock')
  const atlasInputs = any((path) =>
    path.startsWith('core/')
    || path.startsWith('runtime/')
    || path.startsWith('adapter/')
    || path.startsWith('organism/')
    || path.startsWith('apps/')
    || path.startsWith('graph/')
    || path.startsWith('bindings/')
    || path.startsWith('deploy/')
    || path.startsWith('tools/')
    || path.startsWith('docs/')
    || path.startsWith('.github/')
    || ['README.md', 'AGENTS.md', 'Cargo.toml', 'Cargo.lock', 'package.json', 'pnpm-workspace.yaml', 'pnpm-lock.yaml'].includes(path),
  )

  if (atlasInputs) checks.push(CHECKS['atlas:system:check'])
  if (ui) checks.push(CHECKS['ui:typecheck'], CHECKS['atlas:i18n:test'])
  if (docs) checks.push(CHECKS['docs:check'])
  if (architectureDocs) checks.push(CHECKS['arch:contracts'])
  if (rust) checks.push(...cargoChecks(root, paths))

  if (mode === 'post_commit' && atlasInputs) checks.unshift(CHECKS['atlas:system:test'])
  return uniqueChecks(checks)
}

export function defaultLocalVerificationChecks({ full = false } = {}) {
  const base = [
    CHECKS['atlas:system:test'],
    CHECKS['atlas:check'],
    CHECKS['atlas:i18n:test'],
    CHECKS['ui:typecheck'],
    CHECKS['docs:check'],
    CHECKS['arch:contracts'],
    CHECKS['refoundation:no-temporary-dependency'],
    CHECKS['system-atlas:check'],
  ]
  if (full) base.push(CHECKS['cargo:check:workspace'])
  return base
}

export function normalizeChangedPath(root, absoluteOrRelativePath) {
  const original = String(absoluteOrRelativePath)
  if (!isAbsolute(original)) return slash(original).replace(/^\.\//, '')
  return slash(relative(root, original))
}
