import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { compileUiReality, validateUiReality } from './ui-audit.mjs'

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-ui-audit-'))
  execFileSync('git', ['init', '-q'], { cwd: root })
  mkdirSync(join(root, 'apps/ui/src/components'), { recursive: true })
  mkdirSync(join(root, 'apps/ui/public'), { recursive: true })
  mkdirSync(join(root, 'apps/ui/packages/example/src'), { recursive: true })
  writeFileSync(join(root, 'apps/ui/src/main.tsx'), `import { App } from './App'; App();\n`)
  writeFileSync(join(root, 'apps/ui/src/App.tsx'), `import { Active } from '@/components/Active'; export function App(){ return Active(); }\n`)
  writeFileSync(join(root, 'apps/ui/src/components/Active.tsx'), `export function Active(){ return null }\n`)
  writeFileSync(join(root, 'apps/ui/src/components/Orphan.tsx'), `export function Orphan(){ return null }\n`)
  writeFileSync(join(root, 'apps/ui/src/components/Active.test.tsx'), `import { Active } from './Active'; void Active;\n`)
  writeFileSync(join(root, 'apps/ui/index.html'), '<link rel="icon" href="/favicon.svg"><title>Paperclip</title>')
  writeFileSync(join(root, 'apps/ui/public/favicon.svg'), '<svg/>')
  writeFileSync(join(root, 'apps/ui/public/orphan.svg'), '<svg/>')
  writeFileSync(join(root, 'apps/ui/feed-preview.html'), '<html></html>')
  writeFileSync(join(root, 'apps/ui/packages/example/package.json'), JSON.stringify({ name: '@chronica/example' }))
  writeFileSync(join(root, 'apps/ui/packages/example/src/index.ts'), 'export const example = true;\n')
  writeFileSync(join(root, 'apps/ui/package.json'), JSON.stringify({
    name: '@chronica/ui',
    description: 'Paperclip board UI',
    homepage: 'https://github.com/paperclipai/paperclip',
    dependencies: { '@chronica/example': 'workspace:*' },
  }, null, 2))
  execFileSync('git', ['add', '.'], { cwd: root })
  return root
}

test('classifies production-reachable and orphan UI files', () => {
  const root = fixture()
  const ui = compileUiReality(root)
  const byPath = new Map(ui.files.map((file) => [file.path, file]))
  assert.equal(byPath.get('apps/ui/src/main.tsx')?.status, 'ACTIVE_PRODUCTION')
  assert.equal(byPath.get('apps/ui/src/App.tsx')?.status, 'ACTIVE_PRODUCTION')
  assert.equal(byPath.get('apps/ui/src/components/Active.tsx')?.status, 'ACTIVE_PRODUCTION')
  assert.equal(byPath.get('apps/ui/src/components/Active.test.tsx')?.status, 'TEST_ONLY')
  assert.equal(byPath.get('apps/ui/src/components/Orphan.tsx')?.status, 'ORPHAN_CANDIDATE')
})

test('audits public asset and workspace package liveness', () => {
  const root = fixture()
  const ui = compileUiReality(root)
  const assets = new Map(ui.publicAssets.map((asset) => [asset.path, asset]))
  assert.equal(assets.get('apps/ui/public/favicon.svg')?.status, 'ACTIVE_REFERENCED')
  assert.equal(assets.get('apps/ui/public/orphan.svg')?.status, 'UNREFERENCED_ASSET_CANDIDATE')
  assert.equal(ui.packageReviewCandidates.some((pkg) => pkg.name === '@chronica/example' && pkg.status === 'MANIFEST_ONLY'), true)
})

test('hard-fails retired preview artifacts and donor UI identity', () => {
  const root = fixture()
  const errors = validateUiReality(compileUiReality(root))
  assert.ok(errors.some((error) => error.includes('feed-preview.html')))
  assert.ok(errors.some((error) => error.includes('donor UI identity remains')))
})
