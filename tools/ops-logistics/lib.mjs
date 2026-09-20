import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, extname, join, resolve, sep } from 'node:path'
import { parse as parseYaml } from 'yaml'

export const OPS_LOGISTICS_SCHEMA_VERSION = 1
export const REALITY_SOURCE = 'observed Ops Git/working-tree evidence; generated projection, never canonical truth'

const SOURCE_EXTENSIONS = new Set(['.rs', '.ts', '.tsx', '.js', '.mjs', '.cjs', '.sql', '.proto', '.py', '.go', '.java', '.kt', '.rb', '.php'])
const MANIFEST_NAMES = ['ops.manifest.yaml', 'ops.manifest.yml']
const REQUIRED_DOCS = [
  'docs/README.md',
  'docs/INDEX.md',
  'docs/TEMPLATE.md',
  'docs/architecture/README.md',
]
const REQUIRED_DOC_DIRS = [
  'docs/architecture',
  'docs/blueprints',
  'docs/decisions',
  'docs/guides',
  'docs/references',
]
const BOUNDARY_SEGMENTS = new Set([
  'adapter', 'adapters', 'provider', 'providers', 'integration', 'integrations',
  'compat', 'compatibility', 'transport', 'transports', 'vendor', 'vendors',
  'provenance', 'reference', 'references', 'fixtures', 'fixture', 'migrations',
])
const LOCAL_DISPOSITIONS = new Set(['KEEP_PRODUCT_LOCAL', 'KEEP_PROVIDER_MECHANIC'])
const CHRONICA_MAPPED_DISPOSITIONS = new Set([
  'REUSE_CHRONICA_SEMANTIC',
  'ALIAS_AT_BOUNDARY',
  'MAP_TO_CHRONICA_SEMANTIC',
  'EXTEND_CHRONICA_SEMANTIC',
  'TEMPORARY_COMPATIBILITY_ALIAS',
  'RETIRE_DUPLICATE_SEMANTIC',
])
const MATCH_STATUSES = new Set([
  'EXACT_EQUIVALENT',
  'EQUIVALENT_WITH_DIFFERENT_NAME',
  'CHRONICA_NARROWER',
  'CHRONICA_BROADER',
  'PROVIDER_MECHANIC_ONLY',
  'PRODUCT_LOCAL',
  'NO_EQUIVALENT_FOUND',
  'UNRESOLVED',
])
const DISPOSITIONS = new Set([
  ...CHRONICA_MAPPED_DISPOSITIONS,
  ...LOCAL_DISPOSITIONS,
  'DEFER_UNRESOLVED_WITH_EVIDENCE',
])

function slash(value) {
  return String(value).split(sep).join('/')
}

export function parseCli(argv) {
  const out = { _: [] }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (!arg.startsWith('--')) {
      out._.push(arg)
      continue
    }
    const key = arg.slice(2)
    const next = argv[i + 1]
    if (next && !next.startsWith('--')) {
      out[key] = next
      i += 1
    } else {
      out[key] = true
    }
  }
  return out
}

export function git(root, args, { allowFailure = false } = {}) {
  try {
    return execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', allowFailure ? 'ignore' : 'pipe'] }).trim()
  } catch (error) {
    if (allowFailure) return ''
    throw error
  }
}

export function gitLines(root, args, options = {}) {
  const value = git(root, args, options)
  return value ? value.split('\n').map((line) => line.trim()).filter(Boolean) : []
}

export function repoSha(root) {
  return git(root, ['rev-parse', 'HEAD'])
}

export function workingTreeStatus(root) {
  return gitLines(root, ['status', '--porcelain=v1'])
}

export function findManifest(root) {
  for (const name of MANIFEST_NAMES) {
    const file = join(root, name)
    if (existsSync(file)) return file
  }
  throw new Error(`Ops manifest not found under ${root}; expected ${MANIFEST_NAMES.join(' or ')}`)
}

export function loadOpsManifest(root) {
  const file = findManifest(root)
  const raw = readFileSync(file, 'utf8')
  const manifest = parseYaml(raw)
  if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest)) throw new Error(`${file}: manifest must be a YAML object`)
  return { file, raw, manifest }
}

export function sourceFiles(root) {
  const files = gitLines(root, ['ls-files', '--cached', '--others', '--exclude-standard'])
  return files
    .map(slash)
    .filter((file) => existsSync(join(root, file)))
    .filter((file) => SOURCE_EXTENSIONS.has(extname(file)))
    .sort()
}

export function isTestFile(file, text = '') {
  return /(?:^|\/)(?:tests?|__tests__)(?:\/|$)/.test(file)
    || /\.(?:test|spec)\.[cm]?[jt]sx?$/.test(file)
    || (file.endsWith('.rs') && /#\s*\[\s*(?:tokio::)?test\s*\]/.test(text))
}

export function isBoundaryPath(file) {
  const parts = slash(file).toLowerCase().split('/')
  return parts.some((part) => BOUNDARY_SEGMENTS.has(part))
}

function safeRead(file) {
  try {
    return readFileSync(file, 'utf8')
  } catch {
    return ''
  }
}

function unique(values) {
  return [...new Set(values.filter(Boolean))]
}

function termOccurrences(text, term) {
  if (typeof term !== 'string' || !term.trim()) return 0
  const needle = term.trim()
  let index = 0
  let count = 0
  while ((index = text.indexOf(needle, index)) >= 0) {
    count += 1
    index += Math.max(needle.length, 1)
  }
  return count
}

function semanticMappings(manifest) {
  const mappings = manifest?.semantic_convergence?.mappings
  return Array.isArray(mappings) ? mappings : []
}

function docsKernel(root) {
  const files = REQUIRED_DOCS.map((path) => ({ path, exists: existsSync(join(root, path)) }))
  const directories = REQUIRED_DOC_DIRS.map((path) => ({ path, exists: existsSync(join(root, path)) }))
  return {
    files,
    directories,
    complete: files.every((entry) => entry.exists) && directories.every((entry) => entry.exists),
    missing: [...files, ...directories].filter((entry) => !entry.exists).map((entry) => entry.path),
  }
}

function commitExists(root, sha) {
  if (!sha || typeof sha !== 'string') return false
  return Boolean(git(root, ['cat-file', '-e', `${sha}^{commit}`], { allowFailure: true }) || git(root, ['rev-parse', '--verify', `${sha}^{commit}`], { allowFailure: true }))
}

function baselineEvidence(root, manifest) {
  const baseline = manifest?.donor?.ops_git_baseline ?? {}
  const commit = baseline.baseline_commit
  const exists = commitExists(root, commit)
  const remoteBranches = exists ? gitLines(root, ['branch', '-r', '--contains', commit], { allowFailure: true }) : []
  const remoteClaim = baseline.remote_pushed === true
  return {
    commit: typeof commit === 'string' ? commit : null,
    exists,
    manifestClaimsImported: baseline.imported === true,
    manifestClaimsRemotePushed: remoteClaim,
    observedRemoteBranches: remoteBranches,
    remotePushEvidence: !remoteClaim ? 'NOT_CLAIMED' : remoteBranches.length ? 'OBSERVED_REMOTE_REF_CONTAINS_BASELINE' : 'UNVERIFIED_IN_LOCAL_CLONE',
    tagOrBranch: baseline.baseline_tag_or_branch ?? null,
    buildStatus: baseline.baseline_build_status ?? null,
  }
}

function legacyCounters(manifest) {
  const legacy = manifest?.legacy_burndown ?? {}
  const numeric = {}
  for (const [key, value] of Object.entries(legacy)) {
    if (typeof value === 'number' && Number.isFinite(value) && key.endsWith('_remaining')) numeric[key] = value
  }
  return {
    status: legacy.status ?? 'UNKNOWN',
    terminalTarget: legacy.terminal_target ?? null,
    counters: numeric,
    total: Object.values(numeric).reduce((sum, value) => sum + value, 0),
  }
}

export function deploymentEvidencePath(root, manifest) {
  const configured = manifest?.logistics?.deployment_evidence?.path
  return resolve(root, typeof configured === 'string' && configured.trim() ? configured : '.chronica/deployment-evidence.json')
}

export function readDeploymentEvidence(root, manifest) {
  const file = deploymentEvidencePath(root, manifest)
  if (!existsSync(file)) return { file, schemaVersion: 1, records: [], missing: true }
  try {
    const parsed = JSON.parse(readFileSync(file, 'utf8'))
    return {
      file,
      schemaVersion: parsed.schemaVersion ?? 1,
      records: Array.isArray(parsed.records) ? parsed.records : [],
      missing: false,
    }
  } catch (error) {
    return { file, schemaVersion: 1, records: [], missing: false, parseError: error.message }
  }
}

export function writeDeploymentEvidence(root, manifest, record) {
  const current = readDeploymentEvidence(root, manifest)
  if (current.parseError) throw new Error(`cannot append deployment evidence: ${current.parseError}`)
  const file = current.file
  mkdirSync(dirname(file), { recursive: true })
  const records = [...current.records, record]
  const payload = { schemaVersion: 1, generatedProjection: true, records }
  writeFileSync(file, `${JSON.stringify(payload, null, 2)}\n`, 'utf8')
  return { file, payload }
}

function latestByEnvironment(records) {
  const map = new Map()
  for (const record of records) {
    if (!record?.environment) continue
    const previous = map.get(record.environment)
    if (!previous || String(previous.observedAt ?? '') < String(record.observedAt ?? '')) map.set(record.environment, record)
  }
  return [...map.values()].sort((a, b) => String(a.environment).localeCompare(String(b.environment)))
}

export function deploymentReality(root, manifest, headSha = repoSha(root)) {
  const evidence = readDeploymentEvidence(root, manifest)
  const required = Array.isArray(manifest?.logistics?.deployment_evidence?.required_environments)
    ? manifest.logistics.deployment_evidence.required_environments
    : []
  const latest = latestByEnvironment(evidence.records)
  const byEnv = new Map(latest.map((record) => [record.environment, record]))
  const environments = unique([...required, ...latest.map((record) => record.environment)]).map((environment) => {
    const record = byEnv.get(environment) ?? null
    return {
      environment,
      required: required.includes(environment),
      record,
      status: !record ? 'MISSING' : record.gitSha === headSha ? 'CURRENT_HEAD' : commitExists(root, record.gitSha) ? 'OLDER_REPO_COMMIT' : 'UNKNOWN_COMMIT',
    }
  })
  return { ...evidence, requiredEnvironments: required, environments }
}

export function collectOpsReality(root) {
  const opsRoot = resolve(root)
  const { manifest } = loadOpsManifest(opsRoot)
  const headSha = repoSha(opsRoot)
  const status = workingTreeStatus(opsRoot)
  const files = sourceFiles(opsRoot)
  const records = files.map((path) => {
    const text = safeRead(join(opsRoot, path))
    return { path, text, isTest: isTestFile(path, text), boundary: isBoundaryPath(path) }
  })
  const mappings = semanticMappings(manifest)
  const semantics = mappings.map((mapping) => {
    const donorTerms = Array.isArray(mapping?.donor_terms) ? mapping.donor_terms.filter((value) => typeof value === 'string') : []
    const canonical = typeof mapping?.canonical_term_after === 'string' ? mapping.canonical_term_after : null
    const donorRefs = []
    const canonicalRefs = []
    for (const record of records) {
      const donorCount = donorTerms.reduce((sum, term) => sum + termOccurrences(record.text, term), 0)
      const canonicalCount = canonical ? termOccurrences(record.text, canonical) : 0
      if (donorCount) donorRefs.push({ path: record.path, occurrences: donorCount, boundary: record.boundary, test: record.isTest })
      if (canonicalCount) canonicalRefs.push({ path: record.path, occurrences: canonicalCount, boundary: record.boundary, test: record.isTest })
    }
    const internalDonorRefs = donorRefs.filter((entry) => !entry.boundary && !entry.test)
    return {
      semanticId: mapping?.semantic_id ?? null,
      matchStatus: mapping?.match_status ?? null,
      disposition: mapping?.disposition ?? null,
      canonicalTerm: canonical,
      donorTerms,
      donorReferences: donorRefs,
      internalDonorReferences: internalDonorRefs,
      canonicalReferences: canonicalRefs,
      canonicalProductionReferences: canonicalRefs.filter((entry) => !entry.test),
      temporaryAliases: Array.isArray(mapping?.temporary_aliases) ? mapping.temporary_aliases : [],
      retirementTarget: mapping?.retirement_target ?? null,
    }
  })

  const baseline = baselineEvidence(opsRoot, manifest)
  const docs = docsKernel(opsRoot)
  const legacy = legacyCounters(manifest)
  const deployment = deploymentReality(opsRoot, manifest, headSha)
  const drift = []

  if (!baseline.exists) drift.push({ severity: 'error', kind: 'DONOR_BASELINE_MISSING', message: 'Declared donor baseline commit is not present in Ops Git history.' })
  if (!docs.complete) drift.push({ severity: 'error', kind: 'DOCS_KERNEL_INCOMPLETE', message: `Missing: ${docs.missing.join(', ')}` })
  for (const semantic of semantics) {
    if (semantic.internalDonorReferences.length && ['ALIAS_AT_BOUNDARY', 'REUSE_CHRONICA_SEMANTIC', 'MAP_TO_CHRONICA_SEMANTIC', 'RETIRE_DUPLICATE_SEMANTIC'].includes(semantic.disposition)) {
      drift.push({
        severity: 'error',
        kind: 'DONOR_SEMANTIC_LEAKS_INTERNALLY',
        semanticId: semantic.semanticId,
        message: `${semantic.semanticId ?? '<unknown>'}: donor/provider terminology still appears in non-boundary production source (${semantic.internalDonorReferences.length} files).`,
      })
    }
    if (CHRONICA_MAPPED_DISPOSITIONS.has(semantic.disposition) && semantic.canonicalTerm && semantic.canonicalProductionReferences.length === 0) {
      drift.push({ severity: 'warning', kind: 'CANONICAL_TERM_NOT_OBSERVED', semanticId: semantic.semanticId, message: `${semantic.semanticId ?? '<unknown>'}: canonical term ${semantic.canonicalTerm} is not observed in production source.` })
    }
  }
  for (const env of deployment.environments) {
    if (env.required && env.status === 'MISSING') drift.push({ severity: 'error', kind: 'DEPLOYMENT_EVIDENCE_MISSING', environment: env.environment, message: `Required ${env.environment} deployment evidence is missing.` })
    if (env.required && env.status !== 'MISSING' && env.status !== 'CURRENT_HEAD') drift.push({ severity: 'warning', kind: 'DEPLOYMENT_BEHIND_HEAD', environment: env.environment, message: `${env.environment} is evidenced at ${env.record?.gitSha ?? 'unknown'}, not current head ${headSha}.` })
  }

  return {
    schemaVersion: OPS_LOGISTICS_SCHEMA_VERSION,
    source: REALITY_SOURCE,
    opsRoot: slash(opsRoot),
    headSha,
    workingTreeDirty: status.length > 0,
    workingTreeChanges: status.length,
    manifestVersion: manifest?.manifest_version ?? null,
    baseline,
    docs,
    legacy,
    semantics,
    deployment,
    drift,
    stats: {
      sourceFiles: records.length,
      testFiles: records.filter((record) => record.isTest).length,
      semanticMappings: semantics.length,
      internalDonorSemanticLeakFiles: unique(semantics.flatMap((entry) => entry.internalDonorReferences.map((ref) => ref.path))).length,
      legacyCounterTotal: legacy.total,
      driftFindings: drift.length,
      requiredDeploymentEnvironments: deployment.requiredEnvironments.length,
      currentDeployments: deployment.environments.filter((entry) => entry.status === 'CURRENT_HEAD').length,
    },
  }
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0
}

function isSha(value) {
  return typeof value === 'string' && /^[0-9a-f]{40}$/i.test(value)
}

export function validateSemanticConvergence(manifest, reality = null) {
  const errors = []
  const warnings = []
  const semantic = manifest?.semantic_convergence
  const mappings = semanticMappings(manifest)
  if ((manifest?.manifest_version ?? 0) < 5) errors.push('manifest_version must be >= 5 for the executable Ops Logistics contract')
  if (semantic?.status !== 'proven') errors.push('semantic_convergence.status must be proven')
  if (!isSha(semantic?.chronica_sha ?? manifest?.chronica_reference?.sha)) errors.push('semantic convergence requires an exact 40-character Chronica SHA')
  if ((semantic?.unmapped_semantics ?? []).length) errors.push('semantic_convergence.unmapped_semantics must be empty')
  if ((semantic?.unexplained_duplicate_semantics ?? []).length) errors.push('semantic_convergence.unexplained_duplicate_semantics must be empty')

  const ids = new Set()
  const semanticToCanonical = new Map()
  for (const [index, mapping] of mappings.entries()) {
    const label = mapping?.semantic_id ?? `mapping[${index}]`
    if (!nonEmptyString(mapping?.semantic_id)) errors.push(`${label}: semantic_id is required`)
    else if (ids.has(mapping.semantic_id)) errors.push(`${label}: duplicate semantic_id`)
    else ids.add(mapping.semantic_id)
    if (!MATCH_STATUSES.has(mapping?.match_status)) errors.push(`${label}: unsupported match_status ${String(mapping?.match_status)}`)
    if (!DISPOSITIONS.has(mapping?.disposition)) errors.push(`${label}: unsupported disposition ${String(mapping?.disposition)}`)
    if (!mapping?.fingerprint || typeof mapping.fingerprint !== 'object') errors.push(`${label}: semantic fingerprint is required`)
    else {
      if (!nonEmptyString(mapping.fingerprint.resource)) errors.push(`${label}: fingerprint.resource is required`)
      if (!nonEmptyString(mapping.fingerprint.transition) && !nonEmptyString(mapping.fingerprint.effect)) errors.push(`${label}: fingerprint requires transition or effect`)
    }

    if (CHRONICA_MAPPED_DISPOSITIONS.has(mapping?.disposition)) {
      if (!nonEmptyString(mapping?.canonical_term_after)) errors.push(`${label}: canonical_term_after is required for Chronica-mapped semantics`)
      if (!nonEmptyString(mapping?.chronica_owner)) errors.push(`${label}: chronica_owner is required for Chronica-mapped semantics`)
      if (!Array.isArray(mapping?.chronica_code_refs) || mapping.chronica_code_refs.length === 0) errors.push(`${label}: chronica_code_refs must contain real implementation paths`)
      if (!Array.isArray(mapping?.chronica_test_refs) || mapping.chronica_test_refs.length === 0) warnings.push(`${label}: no Chronica test refs recorded; semantic implementation parity is weaker`)
    }

    if (mapping?.match_status === 'NO_EQUIVALENT_FOUND') {
      const search = mapping?.search_evidence
      if (!search || !Array.isArray(search.owners) || !search.owners.length || !Array.isArray(search.code_paths) || !search.code_paths.length) {
        errors.push(`${label}: NO_EQUIVALENT_FOUND requires search_evidence.owners and search_evidence.code_paths`)
      }
    }
    if (mapping?.match_status === 'CHRONICA_NARROWER' && mapping?.disposition !== 'EXTEND_CHRONICA_SEMANTIC') errors.push(`${label}: CHRONICA_NARROWER must use EXTEND_CHRONICA_SEMANTIC`)
    if (mapping?.disposition === 'EXTEND_CHRONICA_SEMANTIC' && !nonEmptyString(mapping?.extension_gap)) errors.push(`${label}: EXTEND_CHRONICA_SEMANTIC requires extension_gap`)
    if ((mapping?.temporary_aliases ?? []).length && !nonEmptyString(mapping?.retirement_target)) errors.push(`${label}: temporary aliases require retirement_target`)

    if (nonEmptyString(mapping?.semantic_id) && nonEmptyString(mapping?.canonical_term_after)) {
      const previous = semanticToCanonical.get(mapping.semantic_id)
      if (previous && previous !== mapping.canonical_term_after) errors.push(`${label}: same semantic_id maps to multiple canonical terms (${previous}, ${mapping.canonical_term_after})`)
      semanticToCanonical.set(mapping.semantic_id, mapping.canonical_term_after)
    }
  }

  if (reality) {
    for (const semanticReality of reality.semantics ?? []) {
      if (semanticReality.internalDonorReferences.length && ['ALIAS_AT_BOUNDARY', 'REUSE_CHRONICA_SEMANTIC', 'MAP_TO_CHRONICA_SEMANTIC', 'RETIRE_DUPLICATE_SEMANTIC'].includes(semanticReality.disposition)) {
        errors.push(`${semanticReality.semanticId}: donor/provider alias leaks into non-boundary production source`)
      }
    }
  }

  return { errors, warnings, mappings: mappings.length }
}

export function buildAbsorptionPlan(manifest, reality, semanticCheck) {
  const actions = []
  for (const mapping of semanticMappings(manifest)) {
    const base = {
      semanticId: mapping.semantic_id,
      canonicalTerm: mapping.canonical_term_after ?? null,
      chronicaOwner: mapping.chronica_owner ?? null,
      disposition: mapping.disposition,
    }
    switch (mapping.disposition) {
      case 'REUSE_CHRONICA_SEMANTIC':
        actions.push({ ...base, action: 'REUSE_EXISTING_CHRONICA_IMPLEMENTATION', terminalPlacement: 'EXISTING_OWNER' })
        break
      case 'ALIAS_AT_BOUNDARY':
      case 'TEMPORARY_COMPATIBILITY_ALIAS':
        actions.push({ ...base, action: 'TRANSLATE_ALIAS_AND_RETIRE_INTERNAL_DUPLICATE', terminalPlacement: 'ADAPTER_OR_COMPAT_BOUNDARY', retirementTarget: mapping.retirement_target ?? null })
        break
      case 'MAP_TO_CHRONICA_SEMANTIC':
        actions.push({ ...base, action: 'MIGRATE_CALLERS_TO_EXISTING_CHRONICA_SEMANTIC', terminalPlacement: 'EXISTING_OWNER' })
        break
      case 'EXTEND_CHRONICA_SEMANTIC':
        actions.push({ ...base, action: 'EXTEND_CANONICAL_CHRONICA_OWNER_THEN_CONVERGE_OPS', terminalPlacement: 'CANONICAL_OWNER', extensionGap: mapping.extension_gap ?? null })
        break
      case 'KEEP_PRODUCT_LOCAL':
        actions.push({ ...base, action: 'KEEP_PRODUCT_LOCAL_WITH_EXPLICIT_CANONICAL_BOUNDARY', terminalPlacement: 'APP_SURFACE' })
        break
      case 'KEEP_PROVIDER_MECHANIC':
        actions.push({ ...base, action: 'KEEP_PROVIDER_MECHANIC_BEHIND_ADAPTER', terminalPlacement: 'ADAPTER' })
        break
      case 'RETIRE_DUPLICATE_SEMANTIC':
        actions.push({ ...base, action: 'MIGRATE_CALLERS_THEN_DELETE_DUPLICATE_SEMANTIC', terminalPlacement: 'RETIRE' })
        break
      default:
        actions.push({ ...base, action: 'BLOCKED_PENDING_SEMANTIC_DECISION', terminalPlacement: 'UNRESOLVED' })
    }
  }

  const blockers = []
  for (const error of semanticCheck.errors ?? []) blockers.push({ kind: 'SEMANTIC_CONVERGENCE', message: error })
  if (!reality.baseline.exists) blockers.push({ kind: 'DONOR_BASELINE', message: 'Declared donor baseline commit is not present.' })
  if (!reality.docs.complete) blockers.push({ kind: 'DOCS_KERNEL', message: `Missing Ops docs surfaces: ${reality.docs.missing.join(', ')}` })
  if (reality.legacy.total > 0) blockers.push({ kind: 'LEGACY_REMAINS', message: `Legacy counters total ${reality.legacy.total}; absorption may proceed only for explicitly bounded slices.` })
  for (const item of reality.drift.filter((entry) => entry.severity === 'error')) blockers.push({ kind: item.kind, message: item.message })

  return {
    schemaVersion: 1,
    generatedProjection: true,
    opsHeadSha: reality.headSha,
    chronicaReferenceSha: manifest?.semantic_convergence?.chronica_sha ?? manifest?.chronica_reference?.sha ?? null,
    readiness: blockers.length ? 'BLOCKED' : 'READY_FOR_BOUNDED_ABSORPTION',
    blockers,
    actions,
    legacy: reality.legacy,
    deployment: reality.deployment.environments,
  }
}

export function buildDocsSyncProposal(manifest, reality, absorptionPlan, chronicaReality = null) {
  const proposals = []
  for (const mapping of semanticMappings(manifest)) {
    if (mapping.disposition === 'EXTEND_CHRONICA_SEMANTIC') {
      proposals.push({
        kind: 'CANONICAL_SEMANTIC_EXTENSION',
        owner: mapping.chronica_owner ?? null,
        semanticId: mapping.semantic_id,
        canonicalTerm: mapping.canonical_term_after ?? null,
        reason: mapping.extension_gap ?? 'Ops proved a reusable semantic not fully expressed by Chronica.',
        evidence: {
          chronicaSha: manifest?.semantic_convergence?.chronica_sha ?? null,
          opsHeadSha: reality.headSha,
          chronicaCodeRefs: mapping.chronica_code_refs ?? [],
          chronicaTestRefs: mapping.chronica_test_refs ?? [],
        },
        action: 'REVIEW_AND_PATCH_CANONICAL_OWNER',
      })
    }
  }
  for (const mapping of semanticMappings(manifest)) {
    if (mapping.match_status === 'NO_EQUIVALENT_FOUND' && mapping.disposition !== 'KEEP_PRODUCT_LOCAL' && mapping.disposition !== 'KEEP_PROVIDER_MECHANIC') {
      proposals.push({ kind: 'NEW_SEMANTIC_REVIEW', semanticId: mapping.semantic_id, owner: mapping.chronica_owner ?? null, reason: 'Ops search found no equivalent Chronica semantic; admission decision required.', action: 'REVIEW_BEFORE_CREATING_CANONICAL_SEMANTIC' })
    }
  }
  for (const drift of chronicaReality?.drift ?? []) {
    if (drift.kind === 'ORPHAN_IMPLEMENTATION_PACKAGE') proposals.push({ kind: 'ORPHAN_IMPLEMENTATION', path: drift.path, reason: drift.message, action: 'MAP_TO_EXISTING_OWNER_OR_PROPOSE_DOCUMENTED_RESPONSIBILITY' })
  }

  return {
    schemaVersion: 1,
    generatedProjection: true,
    mutationPolicy: 'PROPOSAL_ONLY_NEVER_AUTO_REWRITE_CANONICAL_DOCS',
    docsSyncRequired: proposals.length > 0,
    proposals,
    absorptionReadiness: absorptionPlan.readiness,
  }
}

export function validateDeploymentRecord(record) {
  const errors = []
  if (!nonEmptyString(record?.environment)) errors.push('environment is required')
  if (!isSha(record?.gitSha)) errors.push('gitSha must be an exact 40-character commit SHA')
  if (!nonEmptyString(record?.observedAt) || Number.isNaN(Date.parse(record.observedAt))) errors.push('observedAt must be an ISO timestamp')
  if (!nonEmptyString(record?.deploymentId)) errors.push('deploymentId is required')
  if (!nonEmptyString(record?.artifactDigest)) errors.push('artifactDigest is required')
  if (!['PASS', 'FAIL', 'UNKNOWN'].includes(record?.healthStatus)) errors.push('healthStatus must be PASS, FAIL or UNKNOWN')
  return errors
}

export function logisticsScorecard({ manifest, reality, semanticCheck, absorptionPlan, docsProposal }) {
  const deploymentRequired = reality.deployment.requiredEnvironments
  const deploymentCovered = deploymentRequired.every((env) => reality.deployment.environments.some((entry) => entry.environment === env && entry.status !== 'MISSING'))
  const checks = [
    { id: 'OPS_REALITY_MIRROR', pass: reality.source === REALITY_SOURCE && reality.baseline && reality.docs && Array.isArray(reality.semantics) },
    { id: 'SEMANTIC_CONVERGENCE_CHECKER', pass: semanticCheck.errors.length === 0 },
    { id: 'ABSORPTION_PLANNER', pass: Array.isArray(absorptionPlan.actions) && absorptionPlan.actions.length === semanticMappings(manifest).length },
    { id: 'DOCS_SYNC_PROPOSAL', pass: docsProposal.mutationPolicy === 'PROPOSAL_ONLY_NEVER_AUTO_REWRITE_CANONICAL_DOCS' },
    { id: 'DEPLOYMENT_RUNTIME_EVIDENCE', pass: deploymentRequired.length === 0 || deploymentCovered },
    { id: 'LOGISTICS_CONTRACT_VERSION', pass: (manifest?.manifest_version ?? 0) >= 5 && manifest?.logistics?.contract_version === 1 },
  ]
  return {
    score: checks.filter((entry) => entry.pass).length,
    total: checks.length,
    percent: Number(((checks.filter((entry) => entry.pass).length / checks.length) * 100).toFixed(1)),
    checks,
  }
}
