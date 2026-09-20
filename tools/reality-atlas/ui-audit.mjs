import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { extname, join, posix as path } from 'node:path'

const UI_ROOT = 'apps/ui'
const SRC_ROOT = `${UI_ROOT}/src`
const PUBLIC_ROOT = `${UI_ROOT}/public`
const PACKAGES_ROOT = `${UI_ROOT}/packages`
const ENTRYPOINT = `${SRC_ROOT}/main.tsx`
const SOURCE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx']
const TEXT_EXTENSIONS = new Set(['.css', '.html', '.js', '.json', '.jsx', '.md', '.mjs', '.ts', '.tsx', '.txt', '.webmanifest'])
const MAX_TEXT_BYTES = 1024 * 1024
const PREVIEW_RE = /^apps\/ui\/(?!index\.html$).*-preview\.html$/
const RETIRED_PUBLIC_RE = /^apps\/ui\/public\/worktree-favicon(?:[-.].*)?$/
const TEST_RE = /(?:^|\/)(?:tests?|__tests__)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/
const STORY_RE = /\.stories\.[cm]?[jt]sx?$/
const BRAND_SURFACES = [
  `${UI_ROOT}/index.html`,
  `${UI_ROOT}/public/site.webmanifest`,
  `${UI_ROOT}/public/sw.js`,
  `${SRC_ROOT}/context/ThemeContext.tsx`,
]
const TRANSITIONAL_RULES = [
  {
    id: 'CAPABILITY_REGISTRY_PAGE',
    note: 'Capability registry-shaped UI conflicts with graph-derived Capability Resolution and should converge or retire.',
    match: (file) => /apps\/ui\/src\/control-plane\/pages\/(?:CapabilitiesPage|CapabilityDefinitionDetailPage)(?:\.test)?\.tsx$/.test(file),
  },
]

function slash(value) { return String(value).replaceAll('\\', '/') }
function gitLines(root, args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).split('\n').map((line) => slash(line.trim())).filter(Boolean)
}
function safeRead(root, file) {
  try {
    const absolute = join(root, file)
    if (!existsSync(absolute) || statSync(absolute).size > MAX_TEXT_BYTES) return ''
    return readFileSync(absolute, 'utf8')
  } catch { return '' }
}
function unique(values) { return [...new Set(values.filter(Boolean))].sort() }
function isSource(file) { return file.startsWith(`${SRC_ROOT}/`) && SOURCE_EXTENSIONS.includes(extname(file)) }
function isPackageSource(file) { return file.startsWith(`${PACKAGES_ROOT}/`) && SOURCE_EXTENSIONS.includes(extname(file)) }
function isTest(file) { return TEST_RE.test(file) }
function isStory(file) { return STORY_RE.test(file) }
function isFixture(file) { return file.startsWith(`${SRC_ROOT}/fixtures/`) || file.includes('/fixtures/') }
function isTextCarrier(file) {
  return TEXT_EXTENSIONS.has(extname(file).toLowerCase()) || ['Dockerfile', 'package.json', 'Caddyfile'].includes(path.basename(file))
}

export function extractImports(text) {
  const specs = []
  const patterns = [
    /(?:import|export)\s+(?:[^'";]*?\sfrom\s*)?["']([^"']+)["']/g,
    /import\(\s*["']([^"']+)["']\s*\)/g,
  ]
  for (const re of patterns) {
    let match
    while ((match = re.exec(String(text ?? '')))) specs.push(match[1])
  }
  return unique(specs)
}

function resolveImport(fromFile, specifier, fileSet) {
  let base = null
  if (specifier.startsWith('@/')) base = `${SRC_ROOT}/${specifier.slice(2)}`
  else if (specifier.startsWith('.')) base = path.normalize(path.join(path.dirname(fromFile), specifier))
  else return null

  const candidates = [base]
  if (!SOURCE_EXTENSIONS.includes(extname(base))) {
    for (const ext of SOURCE_EXTENSIONS) candidates.push(`${base}${ext}`)
    for (const ext of SOURCE_EXTENSIONS) candidates.push(`${base}/index${ext}`)
  }
  return candidates.find((candidate) => fileSet.has(candidate)) ?? null
}

function donorIdentityResidues(root) {
  const residues = []
  const packageFile = `${UI_ROOT}/package.json`
  const packageText = safeRead(root, packageFile)
  if (packageText) {
    try {
      const pkg = JSON.parse(packageText)
      const fields = [
        ['package.description', pkg.description],
        ['package.homepage', pkg.homepage],
        ['package.bugs.url', pkg.bugs?.url],
        ['package.repository.url', typeof pkg.repository === 'string' ? pkg.repository : pkg.repository?.url],
      ]
      for (const [field, value] of fields) {
        if (typeof value === 'string' && /paperclip/i.test(value)) residues.push({ field, value, path: packageFile })
      }
    } catch {
      residues.push({ field: 'package.json', value: 'INVALID_JSON', path: packageFile })
    }
  }

  for (const file of BRAND_SURFACES) {
    const text = safeRead(root, file)
    if (text && /paperclip/i.test(text)) residues.push({ field: 'brand-surface', value: 'Paperclip token present', path: file })
  }
  return residues
}

function compilePublicAssets(root, tracked) {
  const carriers = tracked.filter(isTextCarrier).map((file) => ({ path: file, text: safeRead(root, file) })).filter((entry) => entry.text)
  const assets = tracked.filter((file) => file.startsWith(`${PUBLIC_ROOT}/`) && file !== `${PUBLIC_ROOT}/docs-atlas.json`)
  return assets.map((file) => {
    const relative = file.slice(`${PUBLIC_ROOT}/`.length)
    const publicUrl = `/${relative}`
    const refs = carriers.filter((carrier) => carrier.path !== file && (carrier.text.includes(publicUrl) || carrier.text.includes(relative))).map((carrier) => carrier.path)
    let status = refs.length ? 'ACTIVE_REFERENCED' : 'UNREFERENCED_ASSET_CANDIDATE'
    if (file === `${PUBLIC_ROOT}/site.webmanifest` || file === `${PUBLIC_ROOT}/sw.js`) status = refs.length ? 'ACTIVE_REFERENCED' : 'RUNTIME_ENTRY_REVIEW'
    return { path: file, publicUrl, status, referenceCount: refs.length, sampleRefs: refs.slice(0, 10) }
  })
}

function compileWorkspacePackages(root, tracked, appSourceRecords) {
  const packageManifests = tracked.filter((file) => file.startsWith(`${PACKAGES_ROOT}/`) && file.endsWith('/package.json'))
  let rootPackage = {}
  try { rootPackage = JSON.parse(safeRead(root, `${UI_ROOT}/package.json`) || '{}') } catch { rootPackage = {} }
  const rootDeclared = new Set([...Object.keys(rootPackage.dependencies ?? {}), ...Object.keys(rootPackage.devDependencies ?? {})])
  const packageSources = tracked.filter(isPackageSource).map((file) => ({ path: file, text: safeRead(root, file) }))

  return packageManifests.map((manifestPath) => {
    let manifest = {}
    try { manifest = JSON.parse(safeRead(root, manifestPath) || '{}') } catch { manifest = {} }
    const name = typeof manifest.name === 'string' ? manifest.name : null
    const rootPath = path.dirname(manifestPath)
    const appRefs = name ? appSourceRecords.filter((record) => record.text.includes(name)).map((record) => record.path) : []
    const transitiveRefs = name ? packageSources.filter((record) => !record.path.startsWith(`${rootPath}/`) && record.text.includes(name)).map((record) => record.path) : []
    const declared = Boolean(name && rootDeclared.has(name))
    let status = 'UNREFERENCED_PACKAGE'
    if (appRefs.length) status = 'ACTIVE_IMPORTED'
    else if (transitiveRefs.length) status = 'ACTIVE_TRANSITIVE'
    else if (declared) status = 'MANIFEST_ONLY'
    return {
      path: rootPath,
      name,
      status,
      rootDeclared: declared,
      appReferenceCount: appRefs.length,
      transitiveReferenceCount: transitiveRefs.length,
      sampleRefs: unique([...appRefs, ...transitiveRefs]).slice(0, 10),
    }
  }).sort((a, b) => a.path.localeCompare(b.path))
}

export function compileUiReality(root) {
  const tracked = gitLines(root, ['ls-files', UI_ROOT])
  const sourceFiles = tracked.filter(isSource)
  const fileSet = new Set(sourceFiles)
  const records = new Map()
  const reverseRefs = new Map(sourceFiles.map((file) => [file, []]))

  for (const file of sourceFiles) {
    const text = safeRead(root, file)
    const imports = extractImports(text)
    const resolved = unique(imports.map((specifier) => resolveImport(file, specifier, fileSet)))
    records.set(file, { file, path: file, text, imports, resolved })
    for (const target of resolved) reverseRefs.get(target)?.push(file)
  }

  const reachable = new Set()
  const queue = fileSet.has(ENTRYPOINT) ? [ENTRYPOINT] : []
  while (queue.length) {
    const file = queue.shift()
    if (!file || reachable.has(file)) continue
    reachable.add(file)
    for (const target of records.get(file)?.resolved ?? []) {
      if (!reachable.has(target) && !isTest(target) && !isStory(target)) queue.push(target)
    }
  }

  const files = sourceFiles.map((file) => {
    const refs = unique(reverseRefs.get(file) ?? [])
    const productionRefs = refs.filter((ref) => reachable.has(ref) && !isTest(ref) && !isStory(ref))
    const testRefs = refs.filter(isTest)
    const storyRefs = refs.filter(isStory)
    let status = 'ORPHAN_CANDIDATE'
    if (reachable.has(file) && !isTest(file) && !isStory(file)) status = 'ACTIVE_PRODUCTION'
    else if (isTest(file)) status = 'TEST_ONLY'
    else if (isStory(file)) status = 'STORY_ONLY'
    else if (isFixture(file)) status = 'FIXTURE_SUPPORT'
    else if (testRefs.length || storyRefs.length) status = 'TEST_STORY_SUPPORT'
    return { path: file, status, inboundRefs: refs.length, productionRefs: productionRefs.length, testRefs: testRefs.length, storyRefs: storyRefs.length, sampleRefs: refs.slice(0, 10) }
  })

  const previewArtifacts = tracked.filter((file) => PREVIEW_RE.test(file))
  const retiredPublicArtifacts = tracked.filter((file) => RETIRED_PUBLIC_RE.test(file))
  const donorIdentity = donorIdentityResidues(root)
  const transitional = TRANSITIONAL_RULES.map((rule) => {
    const matched = tracked.filter((file) => rule.match(file))
    return matched.length ? { id: rule.id, note: rule.note, files: matched } : null
  }).filter(Boolean)
  const orphanCandidates = files.filter((file) => file.status === 'ORPHAN_CANDIDATE')
  const publicAssets = compilePublicAssets(root, tracked)
  const unreferencedPublicAssets = publicAssets.filter((asset) => asset.status === 'UNREFERENCED_ASSET_CANDIDATE')
  const workspacePackages = compileWorkspacePackages(root, tracked, [...records.values()])
  const packageReviewCandidates = workspacePackages.filter((pkg) => pkg.status === 'MANIFEST_ONLY' || pkg.status === 'UNREFERENCED_PACKAGE')
  const statusCounts = Object.fromEntries(unique(files.map((file) => file.status)).map((status) => [status, files.filter((file) => file.status === status).length]))

  return {
    source: 'observed UI import reachability, public-asset references, workspace-package usage and shell/package identity; generated advisory projection, never deletion authority',
    entrypoint: ENTRYPOINT,
    files,
    orphanCandidates,
    previewArtifacts,
    retiredPublicArtifacts,
    donorIdentity,
    transitional,
    publicAssets,
    unreferencedPublicAssets,
    workspacePackages,
    packageReviewCandidates,
    stats: {
      trackedUiFiles: tracked.length,
      sourceFiles: sourceFiles.length,
      productionReachableFiles: files.filter((file) => file.status === 'ACTIVE_PRODUCTION').length,
      orphanCandidates: orphanCandidates.length,
      testOnlyFiles: files.filter((file) => file.status === 'TEST_ONLY').length,
      storyOnlyFiles: files.filter((file) => file.status === 'STORY_ONLY').length,
      supportOnlyFiles: files.filter((file) => ['FIXTURE_SUPPORT', 'TEST_STORY_SUPPORT'].includes(file.status)).length,
      previewArtifacts: previewArtifacts.length,
      retiredPublicArtifacts: retiredPublicArtifacts.length,
      donorIdentityResidues: donorIdentity.length,
      transitionalFiles: transitional.reduce((sum, entry) => sum + entry.files.length, 0),
      publicAssets: publicAssets.length,
      unreferencedPublicAssets: unreferencedPublicAssets.length,
      workspacePackages: workspacePackages.length,
      packageReviewCandidates: packageReviewCandidates.length,
      statusCounts,
    },
  }
}

export function validateUiReality(ui) {
  const errors = []
  if (!ui || !Array.isArray(ui.files)) errors.push('UI reality files must be an array')
  if (!ui?.entrypoint) errors.push('UI reality entrypoint is required')
  for (const file of ui?.previewArtifacts ?? []) errors.push(`retired UI preview artifact must not exist: ${file}`)
  for (const file of ui?.retiredPublicArtifacts ?? []) errors.push(`retired UI public artifact must not exist: ${file}`)
  for (const residue of ui?.donorIdentity ?? []) errors.push(`donor UI identity remains in ${residue.path} (${residue.field}): ${residue.value}`)
  return errors
}
