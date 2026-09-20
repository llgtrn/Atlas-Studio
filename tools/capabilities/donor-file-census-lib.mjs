import { createHash } from 'node:crypto'
import { readdirSync, statSync, readFileSync } from 'node:fs'
import { join, relative, sep } from 'node:path'

export const SOURCE_EXTENSIONS = new Set([
  '.rs', '.go', '.py', '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs',
  '.java', '.kt', '.kts', '.swift', '.c', '.cc', '.cpp', '.cxx', '.h', '.hh', '.hpp',
  '.cs', '.rb', '.php', '.ex', '.exs', '.erl', '.hrl', '.scala', '.sh', '.bash',
  '.ps1', '.psm1', '.sql', '.graphql', '.proto', '.vue', '.svelte', '.astro',
])

export const CONFIG_SCHEMA_EXTENSIONS = new Set([
  '.json', '.jsonc', '.yaml', '.yml', '.toml', '.xml', '.ini', '.cfg', '.conf',
  '.env', '.properties', '.tf', '.tfvars', '.rego',
])

export const DOC_EXTENSIONS = new Set(['.md', '.mdx', '.rst', '.adoc', '.txt'])

export const BINARY_EXTENSIONS = new Set([
  '.png', '.jpg', '.jpeg', '.gif', '.webp', '.ico', '.icns', '.bmp', '.tiff', '.svgz',
  '.pdf', '.zip', '.tar', '.gz', '.tgz', '.bz2', '.xz', '.7z', '.rar',
  '.mp3', '.mp4', '.mov', '.avi', '.mkv', '.wav', '.flac', '.ogg', '.webm',
  '.woff', '.woff2', '.ttf', '.otf', '.eot', '.wasm', '.dll', '.exe', '.so', '.dylib',
  '.class', '.jar', '.pyc', '.pyo', '.o', '.obj', '.a', '.lib',
])

export const VENDOR_DIRS = new Set([
  '.git', '.hg', '.svn', 'node_modules', 'vendor', 'vendors', 'third_party',
  'third-party', 'extern', 'external', 'deps', 'dependencies', '.venv', 'venv',
  'env', '__pycache__', '.pytest_cache', '.mypy_cache',
])

export const BUILD_DIRS = new Set([
  'dist', 'build', 'out', 'target', '.next', '.nuxt', '.svelte-kit', 'coverage',
  '.cache', '.parcel-cache', '.turbo', 'tmp', 'temp', 'bin', 'obj', 'DerivedData',
])

export const GENERATED_DIRS = new Set([
  'generated', 'gen', '__generated__', '.generated', 'fixtures', 'snapshots',
  '__snapshots__',
])

const LOCKFILE_NAMES = new Set([
  'package-lock.json', 'pnpm-lock.yaml', 'yarn.lock', 'Cargo.lock', 'Gemfile.lock',
  'poetry.lock', 'Pipfile.lock', 'composer.lock', 'go.sum',
])

function extnameLower(path) {
  const base = path.split('/').pop() || ''
  const i = base.lastIndexOf('.')
  return i <= 0 ? '' : base.slice(i).toLowerCase()
}

function pathSegments(path) {
  return path.split('/').filter(Boolean)
}

function hasAnySegment(path, set) {
  return pathSegments(path).some(segment => set.has(segment))
}

export function normalizeDonorPath(path, donor) {
  const slash = path.replaceAll('\\', '/')
  const marker = `Temporary/${donor}/`
  const idx = slash.indexOf(marker)
  if (idx >= 0) return slash.slice(idx + marker.length)
  const donorMarker = `${donor}/`
  const donorIdx = slash.indexOf(donorMarker)
  if (donorIdx >= 0) return slash.slice(donorIdx + donorMarker.length)
  return slash.replace(/^Temporary\//, '')
}

export function classifyDonorDirectory(path) {
  const last = path.replaceAll('\\', '/').split('/').filter(Boolean).pop() || ''
  if (VENDOR_DIRS.has(last)) {
    return {
      kind: 'vendor_dir',
      classification: 'generated_vendor_build_artifact',
      readStatus: 'classified_not_read',
      reason: 'vendor directory',
    }
  }
  if (BUILD_DIRS.has(last)) {
    return {
      kind: 'build_dir',
      classification: 'generated_vendor_build_artifact',
      readStatus: 'classified_not_read',
      reason: 'build/output directory',
    }
  }
  if (GENERATED_DIRS.has(last)) {
    return {
      kind: 'generated_dir',
      classification: 'generated_vendor_build_artifact',
      readStatus: 'classified_not_read',
      reason: 'generated/test fixture directory',
    }
  }
  return null
}

export function classifyDonorFile(path) {
  const slash = path.replaceAll('\\', '/')
  const base = slash.split('/').pop() || ''
  const ext = extnameLower(slash)

  if (hasAnySegment(slash, VENDOR_DIRS)) {
    return {
      kind: 'vendor',
      classification: 'generated_vendor_build_artifact',
      readStatus: 'classified_not_read',
      reason: 'vendor directory',
    }
  }
  if (hasAnySegment(slash, BUILD_DIRS)) {
    return {
      kind: 'build',
      classification: 'generated_vendor_build_artifact',
      readStatus: 'classified_not_read',
      reason: 'build/output directory',
    }
  }
  if (hasAnySegment(slash, GENERATED_DIRS) || LOCKFILE_NAMES.has(base)) {
    return {
      kind: 'generated',
      classification: 'generated_vendor_build_artifact',
      readStatus: 'classified_not_read',
      reason: LOCKFILE_NAMES.has(base) ? 'lockfile/generated dependency graph' : 'generated/test fixture path',
    }
  }
  if (BINARY_EXTENSIONS.has(ext)) {
    return {
      kind: 'binary_asset',
      classification: 'non_behavioral_support',
      readStatus: 'classified_not_read',
      reason: 'binary/media asset',
    }
  }
  if (SOURCE_EXTENSIONS.has(ext)) {
    return {
      kind: 'source',
      classification: 'capability_review_pending',
      readStatus: 'unread_pending',
      reason: 'source or behavior-bearing file requires capability review',
    }
  }
  if (DOC_EXTENSIONS.has(ext)) {
    return {
      kind: 'documentation',
      classification: 'behavior_review_pending',
      readStatus: 'unread_pending',
      reason: 'documentation may contain behavior contracts',
    }
  }
  if (CONFIG_SCHEMA_EXTENSIONS.has(ext) || base.startsWith('.')) {
    return {
      kind: 'config_schema',
      classification: 'behavior_review_pending',
      readStatus: 'unread_pending',
      reason: 'configuration/schema may define behavior',
    }
  }
  return {
    kind: 'support_unknown',
    classification: 'behavior_review_pending',
    readStatus: 'unread_pending',
    reason: 'unknown text/support file needs review',
  }
}

export function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

export function scanDonorFiles(donorRoot, donor, options = {}) {
  const rows = []
  const hashFiles = options.hashFiles === true
  const maxRows = options.maxRows || Infinity

  function pushRow(absPath, relPath, dirent, classification, isDirectory = false) {
    const st = statSync(absPath)
    rows.push({
      donor,
      path: relPath,
      isDirectory: isDirectory ? 1 : 0,
      kind: classification.kind,
      classification: classification.classification,
      readStatus: classification.readStatus,
      mappedSourceIds: '',
      exclusionReason: classification.reason,
      sizeBytes: isDirectory ? null : st.size,
      mtimeMs: Math.trunc(st.mtimeMs),
      sha256: (!isDirectory && hashFiles) ? sha256File(absPath) : null,
    })
  }

  function walk(dir) {
    if (rows.length >= maxRows) return
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (rows.length >= maxRows) return
      const abs = join(dir, entry.name)
      const rel = relative(donorRoot, abs).split(sep).join('/')
      if (entry.isDirectory()) {
        const dirClass = classifyDonorDirectory(rel)
        if (dirClass) {
          pushRow(abs, `${rel}/`, entry, dirClass, true)
          continue
        }
        walk(abs)
      } else if (entry.isFile()) {
        pushRow(abs, rel, entry, classifyDonorFile(rel), false)
      }
    }
  }

  walk(donorRoot)
  return rows
}
