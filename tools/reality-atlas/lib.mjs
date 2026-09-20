import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { extname, join, posix as path } from 'node:path'

export const REALITY_SCHEMA_VERSION = 1
export const REALITY_SOURCE = 'observed working-tree and Git implementation evidence; generated projection, never canonical truth'

export const CANONICAL_BACKEND_SOURCE_ROOTS = ['core/', 'runtime/', 'adapter/', 'organism/']
export const REALITY_SOURCE_ROOTS = [
  ...CANONICAL_BACKEND_SOURCE_ROOTS,
  'apps/',
  'graph/',
  'bindings/',
  'tools/',
  'scripts/',
]
const SOURCE_EXTENSIONS = new Set(['.rs', '.ts', '.tsx', '.js', '.mjs', '.cjs', '.sql', '.proto'])
const ARCH_ID_RE = /\b(?:INV|ARCH-EQ|ARCH-PERF)-[A-Z0-9-]+\b/g

// Retired top-level roots (including crates/) are enforced by root-topology.mjs. These rules
// cover legacy implementation shapes that can still exist inside otherwise canonical roots.
const LEGACY_RULES = [
  {
    id: 'LEGACY_CAPABILITY_REGISTRY_UI',
    description: 'Control-plane UI still presents capability definitions as registry-owned objects instead of derived Capability Resolution.',
    match: (file) => /apps\/ui\/src\/control-plane\/pages\/(?:CapabilitiesPage|CapabilityDefinitionDetailPage)(?:\.test)?\.tsx$/.test(file),
  },
]

function unique(values) {
  return [...new Set(values.filter(Boolean))]
}

function gitLines(root, args) {
  // maxBuffer: default (1MB) is exceeded by `git ls-files --cached` once the tracked donor corpus
  // under temporary/ (736k+ files as of the 2026-09-16 hard refoundation's donor ingestion) is
  // included -- ENOBUFS otherwise. 256MB matches the bound already used elsewhere in this
  // refoundation's own tooling (tools/refoundation/donor-burndown.mjs) for the same reason.
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 })
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
}

function slash(value) {
  return String(value).replaceAll('\\', '/')
}

export function normalizeOwnerPath(value) {
  if (typeof value !== 'string') return ''
  return slash(value.trim()).replace(/^\.\//, '').replace(/\/+$/, '')
}

export function matchesOwnerPath(file, owner) {
  const normalizedFile = slash(file)
  const normalizedOwner = normalizeOwnerPath(owner)
  if (!normalizedOwner) return false
  return normalizedFile === normalizedOwner || normalizedFile.startsWith(`${normalizedOwner}/`)
}

export function extractArchitectureRefs(text) {
  return unique(String(text ?? '').match(ARCH_ID_RE) ?? []).sort()
}

export function isTestSource(file, text = '') {
  const normalized = slash(file)
  return /(?:^|\/)(?:tests?|__tests__)(?:\/|$)/.test(normalized)
    || /\.(?:test|spec)\.[cm]?[jt]sx?$/.test(normalized)
    || (normalized.endsWith('.rs') && /#\s*\[\s*(?:tokio::)?test\s*\]/.test(text))
}

function isSourceFile(file) {
  if (!REALITY_SOURCE_ROOTS.some((root) => file.startsWith(root))) return false
  const base = path.basename(file)
  if (base === 'Cargo.toml' || base === 'package.json') return true
  return SOURCE_EXTENSIONS.has(extname(file))
}

function safeRead(root, file) {
  try {
    return readFileSync(join(root, file), 'utf8')
  } catch {
    return ''
  }
}

function repoSha(root) {
  return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim()
}

function workingTreeStatus(root) {
  return execFileSync('git', ['status', '--porcelain=v1'], { cwd: root, encoding: 'utf8' })
    .split('\n')
    .map((line) => line.trimEnd())
    .filter(Boolean)
}

function packageIdentity(root, owner) {
  const dir = normalizeOwnerPath(owner)
  if (!dir) return null

  const cargoPath = join(root, dir, 'Cargo.toml')
  if (existsSync(cargoPath)) {
    const cargo = readFileSync(cargoPath, 'utf8')
    const pkgSection = cargo.match(/\[package\]([\s\S]*?)(?:\n\[[^\]]+\]|$)/)?.[1] ?? ''
    const name = pkgSection.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1]
    if (name) {
      return {
        kind: 'cargo',
        name,
        tokens: unique([name, name.replaceAll('-', '_')]),
      }
    }
  }

  const packagePath = join(root, dir, 'package.json')
  if (existsSync(packagePath)) {
    try {
      const manifest = JSON.parse(readFileSync(packagePath, 'utf8'))
      if (typeof manifest.name === 'string' && manifest.name.trim()) {
        return { kind: 'npm', name: manifest.name.trim(), tokens: [manifest.name.trim()] }
      }
    } catch {
      return null
    }
  }

  return null
}

function packageRoots(allFiles) {
  const roots = []
  for (const file of allFiles) {
    // 2026-09-16 hard refoundation: crates/{core,runtime,adapter,cap}/*/Cargo.toml no longer
    // exists -- core/, runtime/, adapter/, organism/ are top-level single-package crates now,
    // not multi-package tiers, so their own Cargo.toml (not a nested */Cargo.toml) is the root.
    if (/^(?:core|runtime|adapter|organism)\/Cargo\.toml$/.test(file)) {
      roots.push(path.dirname(file))
      continue
    }
    if (/^apps\/[^/]+\/package\.json$/.test(file)) roots.push(path.dirname(file))
  }
  return unique(roots).sort()
}

function intersects(values, set) {
  return values.some((value) => set.has(value))
}

export function classifyImplementationState({ runtimeFiles = 0, codeEvidence = 0, testEvidence = 0, externalReferences = null }) {
  if (runtimeFiles === 0 && codeEvidence === 0) return 'TARGET_ONLY'
  if (codeEvidence === 0) return 'STRUCTURAL'
  if (testEvidence > 0 && typeof externalReferences === 'number' && externalReferences > 0) return 'INTEGRATED'
  if (testEvidence > 0) return 'TESTED'
  return 'CODE_EVIDENCED'
}

function runtimeEvidenceFor(root, owner, allFiles, sourceRecords) {
  const normalized = normalizeOwnerPath(owner)
  const matchingAll = allFiles.filter((file) => matchesOwnerPath(file, normalized))
  const matchingSource = sourceRecords.filter((record) => matchesOwnerPath(record.path, normalized))
  const identity = packageIdentity(root, normalized)
  let externalReferenceFiles = null
  let externalReferenceTests = null

  if (identity?.tokens?.length) {
    const externalMatches = sourceRecords.filter((record) => {
      if (matchesOwnerPath(record.path, normalized)) return false
      return identity.tokens.some((token) => token.length >= 4 && record.text.includes(token))
    })
    externalReferenceFiles = externalMatches.filter((record) => !record.isTest).length
    externalReferenceTests = externalMatches.filter((record) => record.isTest).length
  }

  return {
    path: owner,
    normalizedPath: normalized,
    exists: existsSync(join(root, normalized)) || matchingAll.length > 0,
    trackedFiles: matchingAll.length,
    sourceFiles: matchingSource.length,
    testFiles: matchingSource.filter((record) => record.isTest).length,
    packageIdentity: identity,
    externalReferenceFiles,
    externalReferenceTests,
    sampleFiles: matchingSource.slice(0, 12).map((record) => record.path),
  }
}

export function compileReality(root, atlas) {
  const tracked = new Set(gitLines(root, ['ls-files', '--cached']).map(slash))
  // Excluding temporary/ (donor source snapshots, acquisition-only -- never an architecture-owner
  // runtime path, package root, or source-of-truth file by construction; see
  // AGENTS.md section 4) before the existsSync pass is a real performance fix, not just a filter
  // moved earlier: after the 2026-09-16 hard refoundation's donor ingestion, temporary/ accounts
  // for 736k+ of this repository's tracked files, so running a synchronous existsSync() against
  // every one of them (this function's actual cost) made compileReality effectively never finish.
  // Nothing downstream (packageRoots, runtimeEvidenceFor, isSourceFile's SOURCE_ROOTS) ever
  // matches a temporary/ path, so excluding it here changes performance only, not output.
  const allFiles = gitLines(root, ['ls-files', '--cached', '--others', '--exclude-standard'])
    .map(slash)
    .filter((file) => !file.startsWith('temporary/'))
    .filter((file) => existsSync(join(root, file)))
  const sourceFiles = allFiles.filter(isSourceFile)
  const sourceRecords = sourceFiles.map((file) => {
    const text = safeRead(root, file)
    return {
      path: file,
      text,
      tracked: tracked.has(file),
      isTest: isTestSource(file, text),
      architectureRefs: extractArchitectureRefs(text),
    }
  })

  const architectureOwners = (atlas?.nodes ?? []).filter((node) => node.kind === 'architecture-owner')
  const declaredRuntimeOwners = unique(architectureOwners.flatMap((node) => node.runtimeOwners ?? [])).sort()
  const runtimeOwners = declaredRuntimeOwners.map((owner) => runtimeEvidenceFor(root, owner, allFiles, sourceRecords))
  const runtimeByPath = new Map(runtimeOwners.map((entry) => [entry.path, entry]))

  const owners = architectureOwners.map((node) => {
    const contracts = new Set(node.contractIds ?? [])
    const declared = node.runtimeOwners ?? []
    const runtimeEvidence = declared.map((owner) => runtimeByPath.get(owner)).filter(Boolean)
    const observedRuntime = sourceRecords.filter((record) => declared.some((owner) => matchesOwnerPath(record.path, owner)))
    const directCode = sourceRecords.filter((record) => intersects(record.architectureRefs, contracts))
    const directTests = directCode.filter((record) => record.isTest)
    const referenceValues = runtimeEvidence.map((entry) => entry.externalReferenceFiles).filter((value) => typeof value === 'number')
    const externalReferences = referenceValues.length ? Math.max(...referenceValues) : null
    const state = classifyImplementationState({
      runtimeFiles: observedRuntime.length,
      codeEvidence: directCode.length,
      testEvidence: directTests.length,
      externalReferences,
    })
    const missingRuntimeOwners = runtimeEvidence.filter((entry) => !entry.exists).map((entry) => entry.path)
    const uncitedContracts = [...contracts].filter((id) => !sourceRecords.some((record) => record.architectureRefs.includes(id)))

    return {
      docId: node.id,
      title: node.title,
      path: node.path,
      implementationState: state,
      declaredRuntimeOwners: declared,
      missingRuntimeOwners,
      runtimeSourceFiles: observedRuntime.length,
      runtimeFileSamples: observedRuntime.slice(0, 12).map((record) => record.path),
      directCodeEvidenceFiles: directCode.map((record) => record.path).slice(0, 20),
      directTestEvidenceFiles: directTests.map((record) => record.path).slice(0, 20),
      externalReferenceFiles: externalReferences,
      contractIds: [...contracts],
      uncitedContracts,
      evidenceStrength: directTests.length > 0 ? 'TEST_LINKED' : directCode.length > 0 ? 'CODE_LINKED' : observedRuntime.length > 0 ? 'STRUCTURAL_ONLY' : 'NO_IMPLEMENTATION_EVIDENCE',
    }
  })

  const legacyByRule = LEGACY_RULES.map((rule) => {
    const matched = sourceRecords.filter((record) => rule.match(record.path))
    return {
      id: rule.id,
      description: rule.description,
      sourceFiles: matched.length,
      productionFiles: matched.filter((record) => !record.isTest).length,
      testFiles: matched.filter((record) => record.isTest).length,
      sampleFiles: matched.slice(0, 20).map((record) => record.path),
    }
  }).filter((entry) => entry.sourceFiles > 0)

  const declaredPaths = declaredRuntimeOwners.map(normalizeOwnerPath).filter(Boolean)
  const allKnownContractIds = new Set(architectureOwners.flatMap((owner) => owner.contractIds ?? []))
  const packages = packageRoots(allFiles)
  const orphanPackages = packages.filter((pkg) => {
    if (declaredPaths.some((owner) => matchesOwnerPath(pkg, owner) || matchesOwnerPath(owner, pkg))) return false
    return !sourceRecords.some((record) => matchesOwnerPath(record.path, pkg) && intersects(record.architectureRefs, allKnownContractIds))
  })

  const drift = []
  for (const owner of owners) {
    if (owner.implementationState === 'TARGET_ONLY') {
      drift.push({ severity: 'warning', kind: 'DECLARED_NOT_IMPLEMENTED', ownerDocId: owner.docId, path: owner.path, message: `${owner.title} has no observed runtime files or direct code evidence.` })
    }
    for (const missing of owner.missingRuntimeOwners) {
      drift.push({ severity: 'warning', kind: 'MISSING_RUNTIME_OWNER', ownerDocId: owner.docId, path: missing, message: `${owner.title} declares runtime owner ${missing}, but that path is absent in the current working tree.` })
    }
  }
  for (const legacy of legacyByRule) {
    drift.push({ severity: 'warning', kind: 'LEGACY_REMAINS', path: legacy.id, message: `${legacy.description} files=${legacy.sourceFiles}` })
  }
  for (const pkg of orphanPackages) {
    drift.push({ severity: 'advisory', kind: 'ORPHAN_IMPLEMENTATION_PACKAGE', path: pkg, message: `${pkg} is not covered by a declared architecture runtime owner and contains no direct architecture-contract citation.` })
  }

  const stateCounts = Object.fromEntries(['TARGET_ONLY', 'STRUCTURAL', 'CODE_EVIDENCED', 'TESTED', 'INTEGRATED'].map((state) => [state, owners.filter((owner) => owner.implementationState === state).length]))
  const status = workingTreeStatus(root)

  return {
    schemaVersion: REALITY_SCHEMA_VERSION,
    source: REALITY_SOURCE,
    sourceRoots: [...REALITY_SOURCE_ROOTS],
    sourceSha: repoSha(root),
    workingTreeDirty: status.length > 0,
    workingTreeStatusCount: status.length,
    untrackedSourceFiles: sourceRecords.filter((record) => !record.tracked).map((record) => record.path),
    owners,
    runtimeOwners,
    legacy: {
      terminalTarget: 'ZERO_DONOR_OR_TRANSITIONAL_LEGACY_IMPLEMENTATION',
      totalSourceFiles: legacyByRule.reduce((sum, entry) => sum + entry.sourceFiles, 0),
      byRule: legacyByRule,
    },
    orphanPackages,
    drift,
    stats: {
      observedSourceFiles: sourceRecords.length,
      trackedSourceFiles: sourceRecords.filter((record) => record.tracked).length,
      untrackedSourceFiles: sourceRecords.filter((record) => !record.tracked).length,
      architectureOwners: owners.length,
      runtimeOwners: runtimeOwners.length,
      contractReferencesInCode: sourceRecords.reduce((sum, record) => sum + record.architectureRefs.length, 0),
      testFilesWithArchitectureRefs: sourceRecords.filter((record) => record.isTest && record.architectureRefs.length > 0).length,
      legacySourceFiles: legacyByRule.reduce((sum, entry) => sum + entry.sourceFiles, 0),
      orphanPackages: orphanPackages.length,
      driftFindings: drift.length,
      ownerStates: stateCounts,
    },
  }
}

export function validateReality(reality, atlas) {
  const errors = []
  if (reality?.schemaVersion !== REALITY_SCHEMA_VERSION) errors.push(`reality schemaVersion must be ${REALITY_SCHEMA_VERSION}`)
  if (reality?.source !== REALITY_SOURCE) errors.push('reality source must identify observed implementation evidence as a generated projection')
  if (!Array.isArray(reality?.sourceRoots)) {
    errors.push('reality sourceRoots must be an array')
  } else {
    for (const root of CANONICAL_BACKEND_SOURCE_ROOTS) {
      if (!reality.sourceRoots.includes(root)) errors.push(`reality sourceRoots must include canonical backend root: ${root}`)
    }
    if (reality.sourceRoots.includes('crates/')) errors.push('reality sourceRoots must not treat retired crates/ as canonical implementation')
    if (reality.sourceRoots.includes('ops/')) errors.push('reality sourceRoots must not treat retired ops/ as canonical implementation')
  }
  if (!reality?.sourceSha || typeof reality.sourceSha !== 'string') errors.push('reality sourceSha is required')
  if (!Array.isArray(reality?.owners)) errors.push('reality owners must be an array')
  if (!Array.isArray(reality?.runtimeOwners)) errors.push('reality runtimeOwners must be an array')
  if (!Array.isArray(reality?.drift)) errors.push('reality drift must be an array')
  if (errors.length) return errors

  const expectedOwners = (atlas?.nodes ?? []).filter((node) => node.kind === 'architecture-owner')
  const ids = new Set()
  for (const owner of reality.owners) {
    if (!owner?.docId) errors.push('reality owner docId is required')
    else if (ids.has(owner.docId)) errors.push(`duplicate reality owner: ${owner.docId}`)
    else ids.add(owner.docId)
  }
  if (ids.size !== expectedOwners.length) errors.push(`reality owner coverage must match architecture owners: ${ids.size}/${expectedOwners.length}`)
  for (const owner of expectedOwners) {
    if (!ids.has(owner.id)) errors.push(`missing reality evidence record for ${owner.id}`)
  }
  return errors
}
