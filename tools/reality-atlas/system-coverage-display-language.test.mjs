import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import test from 'node:test'
import { enrichDisplayLanguage, validateDisplayLanguage } from './system-coverage-display-language.mjs'

function write(root, file, content) {
  mkdirSync(dirname(join(root, file)), { recursive: true })
  writeFileSync(join(root, file), content)
}

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-display-language-'))
  execFileSync('git', ['init', '-q'], { cwd: root })
  execFileSync('git', ['config', 'user.email', 'atlas@test.invalid'], { cwd: root })
  execFileSync('git', ['config', 'user.name', 'Atlas Test'], { cwd: root })

  write(root, 'apps/ui/src/feed/PostCard.tsx', `
import { useTranslation } from 'react-i18next'
export function ActivityPost(){ const { t } = useTranslation(); return <article>{t('activity.order.filled')}</article> }
`)
  write(root, 'apps/ui/src/control-plane/CapabilitiesPage.tsx', `
export function CapabilitiesPage(){ return <section title="Capability registry">Capability registry is not available yet</section> }
`)
  write(root, 'apps/ui/src/components/MixedTranslationButton.tsx', `
import { useTranslation } from 'react-i18next'
export function MixedTranslationButton(){ const { t } = useTranslation(); void t; return <button aria-label="Open evidence">Open evidence</button> }
`)
  write(root, 'apps/ui/src/components/InternalTypeLeak.tsx', `
export function InternalTypeLeak(){ return <span>WorkRun</span> }
`)
  write(root, 'apps/ui/src/components/CommentOnly.tsx', `
// Capability registry and WorkRun are developer-only words in this comment.
/* Binance CapabilityDefinition reference for maintainers only. */
export function CommentOnly(){ return null }
`)

  for (const locale of ['en', 'ja', 'vi', 'zh-CN', 'zh-TW']) {
    write(root, `apps/ui/src/i18n/locales/${locale}.json`, JSON.stringify({ activity: { order: { filled: 'x' } } }))
  }

  execFileSync('git', ['add', '.'], { cwd: root })
  execFileSync('git', ['commit', '-qm', 'fixture'], { cwd: root })
  return root
}

function coverage() {
  const paths = [
    'apps/ui/src/feed/PostCard.tsx',
    'apps/ui/src/control-plane/CapabilitiesPage.tsx',
    'apps/ui/src/components/MixedTranslationButton.tsx',
    'apps/ui/src/components/InternalTypeLeak.tsx',
    'apps/ui/src/components/CommentOnly.tsx',
  ]
  return {
    families: {
      ui: {
        name: 'ui',
        nodes: paths.map((path, index) => ({
          id: `ui:${index}`,
          family: 'ui',
          kind: 'source_file',
          label: path.split('/').at(-1),
          path,
        })),
        edges: [],
        artifacts: [],
        gaps: [],
      },
    },
    stats: {
      familyStats: { ui: { nodes: paths.length, edges: 0, artifacts: 0, gaps: 0 } },
      totalGaps: 0,
    },
  }
}

test('measures rendered display language while ignoring developer-comment vocabulary', () => {
  const root = fixture()
  const result = enrichDisplayLanguage(root, coverage())

  assert.deepEqual(validateDisplayLanguage(result), [])
  assert.equal(result.displayLanguage.schemaVersion, 1)
  assert.equal(result.displayLanguage.activityProjectionModel, 'ActivityProjection')
  assert.equal(result.displayLanguage.stats.localeCatalogs, 5)
  assert.equal(result.displayLanguage.stats.primaryLocaleFamiliesPresent, 5)
  assert.equal(result.displayLanguage.localeFamilies.english, true)
  assert.equal(result.displayLanguage.localeFamilies.japanese, true)
  assert.equal(result.displayLanguage.localeFamilies.vietnamese, true)
  assert.equal(result.displayLanguage.localeFamilies.chinese_simplified, true)
  assert.equal(result.displayLanguage.localeFamilies.chinese_traditional, true)

  assert.equal(result.displayLanguage.stats.activityProjectionFiles, 1)
  assert.equal(result.displayLanguage.stats.staleCanonicalTermFiles, 1)
  assert.ok(result.displayLanguage.stats.hardcodedCopyFiles >= 3)
  assert.equal(result.displayLanguage.stats.internalTermFiles, 1)

  const post = result.families.ui.nodes.find((node) => node.path.endsWith('PostCard.tsx'))
  assert.equal(post.displayLanguage.translationAware, true)
  assert.equal(post.displayLanguage.activityProjectionSurface, true)
  assert.equal(post.displayLanguage.hardcodedUserCopy, false)

  const stale = result.families.ui.nodes.find((node) => node.path.endsWith('CapabilitiesPage.tsx'))
  assert.equal(stale.displayLanguage.state, 'STALE_CANONICAL_TERM')

  const mixed = result.families.ui.nodes.find((node) => node.path.endsWith('MixedTranslationButton.tsx'))
  assert.equal(mixed.displayLanguage.translationAware, true)
  assert.equal(mixed.displayLanguage.state, 'HARDCODED_USER_COPY')

  const commentOnly = result.families.ui.nodes.find((node) => node.path.endsWith('CommentOnly.tsx'))
  assert.equal(commentOnly.displayLanguage.state, 'NO_USER_COPY_DETECTED')
  assert.equal(commentOnly.displayLanguage.staleCanonicalTermCount, 0)
  assert.equal(commentOnly.displayLanguage.internalTermCount, 0)
  assert.equal(commentOnly.displayLanguage.providerTermCount, 0)

  const internal = result.families.ui.nodes.find((node) => node.path.endsWith('InternalTypeLeak.tsx'))
  assert.equal(internal.displayLanguage.internalTermCount, 1)

  assert.ok(result.families.ui.gaps.some((gap) => gap.kind === 'DISPLAY_LANGUAGE_HARDCODED_COPY_DEBT'))
  assert.ok(result.families.ui.gaps.some((gap) => gap.kind === 'DISPLAY_LANGUAGE_STALE_CANONICAL_TERM'))
  assert.ok(result.families.ui.gaps.some((gap) => gap.kind === 'DISPLAY_LANGUAGE_INTERNAL_TYPE_LEAK'))
})
