import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

const CONTRACT = 'docs/architecture/foundation/product-language.md'
const UI_SOURCE_RE = /\.(?:ts|tsx|js|jsx)$/i
const TEST_RE = /(?:^|\/)(?:tests?|__tests__)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/i
const LOCALE_RE = /^apps\/ui\/src\/i18n\/locales\/([^/]+)\.json$/i

const STALE_CANONICAL_TERMS = [
  /capability registry/i,
  /capability definition(?:\/binding)? registry/i,
  /registered capabilities?/i,
  /runtime tool-capability store/i,
]

const INTERNAL_TERMS = [
  /\bExecutionAdmission\b/,
  /\bCandidateBinding\b/,
  /\bWorkRequest\b/,
  /\bWorkRun\b/,
  /\bCapabilityDefinition\b/,
]

const PROVIDER_TERMS = [
  /\bBinance\b/i,
  /\bCCXT\b/i,
  /\bOpenAI\b/i,
  /\bClaude\b/i,
  /\bChatwoot\b/i,
  /\bERPNext\b/i,
  /\bChannex\b/i,
]

const ACTIVITY_SURFACE_RE = /\b(?:ActivityPost|PostCard|ActivityRow|Timeline|Feed|ArtifactCard|EvidenceCard|ReplyThread)\b/
const TRANSLATION_AWARE_RE = /react-i18next|from\s+["']i18next["']|useTranslation\s*\(|\bi18n\.t\s*\(/
const COPY_PROP = '(?:title|label|placeholder|description|detail|heading|helperText|emptyMessage|aria-label)'
const JSX_TEXT_RE = />\s*([^<>{}\n][^<>{}\n]{1,200}?)\s*</g
const JSX_COPY_PROP_RE = new RegExp(`\\b${COPY_PROP}\\s*=\\s*(?:\\{\\s*)?["'\`]([^"'\`\\n]{2,240})["'\`](?:\\s*\\})?`, 'g')
const OBJECT_COPY_PROP_RE = new RegExp(`\\b${COPY_PROP}\\s*:\\s*["'\`]([^"'\`\\n]{2,240})["'\`]`, 'g')

function gitFiles(root) {
  return execFileSync('git', ['ls-files', 'apps/ui/src'], { cwd: root, encoding: 'utf8' })
    .split('\n')
    .map((value) => value.trim().replaceAll('\\', '/'))
    .filter(Boolean)
}

function safeRead(root, file) {
  try {
    const absolute = join(root, file)
    return existsSync(absolute) ? readFileSync(absolute, 'utf8') : ''
  } catch {
    return ''
  }
}

function localeFamily(locale) {
  const value = String(locale).toLowerCase()
  if (value === 'en' || value.startsWith('en-')) return 'english'
  if (value === 'ja' || value.startsWith('ja-')) return 'japanese'
  if (value === 'vi' || value.startsWith('vi-')) return 'vietnamese'
  if (value === 'zh-cn' || value === 'zh-hans' || value.startsWith('zh-hans-')) return 'chinese_simplified'
  if (['zh-tw', 'zh-hk', 'zh-hant'].includes(value) || value.startsWith('zh-hant-')) return 'chinese_traditional'
  return null
}

function stripDeveloperComments(text) {
  return String(text)
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|\n)\s*\/\/[^\n]*/g, '$1')
}

function pushCopy(values, raw) {
  const value = String(raw ?? '').replace(/\s+/g, ' ').trim()
  if (value.length < 2) return
  if (/^[\W_]+$/u.test(value)) return
  values.push(value)
}

function extractUserFacingCopy(text) {
  const source = stripDeveloperComments(text)
  const values = []
  for (const re of [JSX_TEXT_RE, JSX_COPY_PROP_RE, OBJECT_COPY_PROP_RE]) {
    re.lastIndex = 0
    let match
    while ((match = re.exec(source))) pushCopy(values, match[1])
  }
  return [...new Set(values)]
}

function matchedPatterns(patterns, text) {
  return patterns.filter((pattern) => pattern.test(text)).map((pattern) => pattern.source)
}

function classifyFile(file, text) {
  const translationAware = TRANSLATION_AWARE_RE.test(text) || file.includes('/i18n/')
  const userFacingCopy = extractUserFacingCopy(text)
  const userFacingText = userFacingCopy.join('\n')
  const hardcodedUserCopy = userFacingCopy.length > 0
  const staleCanonicalTerms = matchedPatterns(STALE_CANONICAL_TERMS, userFacingText)
  const internalTerms = matchedPatterns(INTERNAL_TERMS, userFacingText)
  const providerTerms = matchedPatterns(PROVIDER_TERMS, userFacingText)
  const activityProjectionSurface = ACTIVITY_SURFACE_RE.test(text) || /(?:^|\/)(?:feed|timeline)(?:\/|$)/i.test(file)

  let state = 'NO_USER_COPY_DETECTED'
  if (staleCanonicalTerms.length) state = 'STALE_CANONICAL_TERM'
  else if (hardcodedUserCopy) state = 'HARDCODED_USER_COPY'
  else if (translationAware) state = 'TRANSLATION_AWARE'

  return {
    file,
    state,
    translationAware,
    hardcodedUserCopy,
    userFacingCopyCount: userFacingCopy.length,
    staleCanonicalTerms,
    internalTerms,
    providerTerms,
    activityProjectionSurface,
  }
}

function addAggregateGap(family, kind, message, count) {
  if (!count) return
  const artifact = CONTRACT
  if (family.gaps.some((gap) => gap.kind === kind && gap.artifact === artifact)) return
  family.gaps.push({ kind, artifact, message: `${message} (${count} file${count === 1 ? '' : 's'} observed).` })
}

function attachUiMetadata(coverage, records) {
  const family = coverage?.families?.ui
  if (!family) return
  const byPath = new Map(records.map((record) => [record.file, record]))
  for (const node of family.nodes ?? []) {
    if (typeof node.path !== 'string') continue
    const record = byPath.get(node.path)
    if (!record) continue
    node.displayLanguage = {
      state: record.state,
      translationAware: record.translationAware,
      activityProjectionSurface: record.activityProjectionSurface,
      hardcodedUserCopy: record.hardcodedUserCopy,
      userFacingCopyCount: record.userFacingCopyCount,
      staleCanonicalTermCount: record.staleCanonicalTerms.length,
      internalTermCount: record.internalTerms.length,
      providerTermCount: record.providerTerms.length,
    }
  }

  addAggregateGap(
    family,
    'DISPLAY_LANGUAGE_HARDCODED_COPY_DEBT',
    'Reusable UI source still contains likely user-facing literal copy instead of only stable locale keys plus structured values',
    records.filter((record) => record.hardcodedUserCopy).length,
  )
  addAggregateGap(
    family,
    'DISPLAY_LANGUAGE_STALE_CANONICAL_TERM',
    'Rendered UI copy still exposes terminology associated with a superseded canonical model',
    records.filter((record) => record.staleCanonicalTerms.length > 0).length,
  )
  addAggregateGap(
    family,
    'DISPLAY_LANGUAGE_INTERNAL_TYPE_LEAK',
    'Rendered UI copy still exposes internal architecture/type vocabulary that should normally be projected into product language',
    records.filter((record) => record.internalTerms.length > 0).length,
  )

  coverage.stats.familyStats.ui = {
    nodes: family.nodes.length,
    edges: family.edges.length,
    artifacts: family.artifacts.length,
    gaps: family.gaps.length,
  }
}

export function enrichDisplayLanguage(root, coverage) {
  const files = gitFiles(root)
  const localeFiles = files.filter((file) => LOCALE_RE.test(file))
  const locales = localeFiles
    .map((file) => file.match(LOCALE_RE)?.[1])
    .filter(Boolean)
    .sort()
  const localeFamilies = Object.fromEntries([
    'english',
    'japanese',
    'vietnamese',
    'chinese_simplified',
    'chinese_traditional',
  ].map((family) => [family, locales.some((locale) => localeFamily(locale) === family)]))

  const records = files
    .filter((file) => UI_SOURCE_RE.test(file) && !TEST_RE.test(file))
    .map((file) => classifyFile(file, safeRead(root, file)))

  attachUiMetadata(coverage, records)

  const states = Object.fromEntries([
    'TRANSLATION_AWARE',
    'HARDCODED_USER_COPY',
    'STALE_CANONICAL_TERM',
    'NO_USER_COPY_DETECTED',
  ].map((state) => [state, records.filter((record) => record.state === state).length]))

  coverage.displayLanguage = {
    schemaVersion: 1,
    contract: CONTRACT,
    opsContract: 'docs/ops_production/contracts/DISPLAY-LANGUAGE.md',
    referenceStyle: 'minimalist-white-sns-like',
    activityProjectionModel: 'ActivityProjection',
    states,
    locales,
    localeFamilies,
    stats: {
      uiSourceFiles: records.length,
      translationAwareFiles: records.filter((record) => record.translationAware).length,
      hardcodedCopyFiles: records.filter((record) => record.hardcodedUserCopy).length,
      staleCanonicalTermFiles: records.filter((record) => record.staleCanonicalTerms.length > 0).length,
      internalTermFiles: records.filter((record) => record.internalTerms.length > 0).length,
      providerTermFiles: records.filter((record) => record.providerTerms.length > 0).length,
      activityProjectionFiles: records.filter((record) => record.activityProjectionSurface).length,
      localeCatalogs: locales.length,
      primaryLocaleFamiliesPresent: Object.values(localeFamilies).filter(Boolean).length,
    },
    debt: records
      .filter((record) => record.hardcodedUserCopy || record.staleCanonicalTerms.length || record.internalTerms.length)
      .map((record) => ({
        file: record.file,
        state: record.state,
        userFacingCopyCount: record.userFacingCopyCount,
        staleCanonicalTermCount: record.staleCanonicalTerms.length,
        internalTermCount: record.internalTerms.length,
      })),
  }

  coverage.stats.displayLanguage = coverage.displayLanguage.stats
  coverage.stats.totalGaps = Object.values(coverage.families ?? {}).reduce((sum, family) => sum + (family.gaps?.length ?? 0), 0)
  return coverage
}

export function validateDisplayLanguage(coverage) {
  const errors = []
  if (!coverage?.displayLanguage) return ['System Atlas display-language projection is missing']
  if (coverage.displayLanguage.schemaVersion !== 1) errors.push('System Atlas display-language schemaVersion must be 1')
  if (coverage.displayLanguage.contract !== CONTRACT) errors.push(`System Atlas display-language contract must be ${CONTRACT}`)
  if (coverage.displayLanguage.activityProjectionModel !== 'ActivityProjection') errors.push('System Atlas must identify ActivityProjection as the reusable display projection model')
  return errors
}
