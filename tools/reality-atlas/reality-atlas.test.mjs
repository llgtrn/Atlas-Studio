import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { execFileSync } from 'node:child_process'
import {
  CANONICAL_BACKEND_SOURCE_ROOTS,
  REALITY_SOURCE_ROOTS,
  classifyImplementationState,
  compileReality,
  extractArchitectureRefs,
  isTestSource,
  matchesOwnerPath,
  normalizeOwnerPath,
  validateReality,
} from './lib.mjs'

test('owner paths normalize and match deterministically', () => {
  assert.equal(normalizeOwnerPath('./runtime/'), 'runtime')
  assert.equal(matchesOwnerPath('runtime/src/foo.rs', 'runtime/'), true)
  assert.equal(matchesOwnerPath('runtime-else/foo.rs', 'runtime/'), false)
})

test('architecture evidence ids are extracted without prose guessing', () => {
  assert.deepEqual(
    extractArchitectureRefs('// INV-AUTH-001 then ARCH-EQ-WORLD-001 and INV-AUTH-001 again'),
    ['ARCH-EQ-WORLD-001', 'INV-AUTH-001'],
  )
})

test('test-source classifier recognizes Rust and TypeScript evidence', () => {
  assert.equal(isTestSource('runtime/tests/replay.rs', ''), true)
  assert.equal(isTestSource('apps/ui/src/foo.test.tsx', ''), true)
  assert.equal(isTestSource('core/src/lib.rs', '#[test]\nfn works() {}'), true)
  assert.equal(isTestSource('core/src/lib.rs', 'pub fn works() {}'), false)
})

test('implementation state is conservative and evidence-ordered', () => {
  assert.equal(classifyImplementationState({ runtimeFiles: 0, codeEvidence: 0, testEvidence: 0, externalReferences: null }), 'TARGET_ONLY')
  assert.equal(classifyImplementationState({ runtimeFiles: 4, codeEvidence: 0, testEvidence: 0, externalReferences: null }), 'STRUCTURAL')
  assert.equal(classifyImplementationState({ runtimeFiles: 4, codeEvidence: 1, testEvidence: 0, externalReferences: null }), 'CODE_EVIDENCED')
  assert.equal(classifyImplementationState({ runtimeFiles: 4, codeEvidence: 2, testEvidence: 1, externalReferences: null }), 'TESTED')
  assert.equal(classifyImplementationState({ runtimeFiles: 4, codeEvidence: 2, testEvidence: 1, externalReferences: 3 }), 'INTEGRATED')
})

test('reality source scope covers the canonical backend and excludes retired backend universes', () => {
  assert.deepEqual(CANONICAL_BACKEND_SOURCE_ROOTS, ['core/', 'runtime/', 'adapter/', 'organism/'])
  for (const root of CANONICAL_BACKEND_SOURCE_ROOTS) assert.ok(REALITY_SOURCE_ROOTS.includes(root))
  assert.equal(REALITY_SOURCE_ROOTS.includes('crates/'), false)
  assert.equal(REALITY_SOURCE_ROOTS.includes('ops/'), false)
})

test('Vite Atlas watcher tracks every canonical backend root and drops retired crates/', () => {
  const source = readFileSync(new URL('../../apps/ui/vite-system-atlas.ts', import.meta.url), 'utf8')
  for (const root of CANONICAL_BACKEND_SOURCE_ROOTS) assert.ok(source.includes(`'${root}'`), `watcher missing ${root}`)
  assert.equal(source.includes("'crates/'"), false)
})

test('reality compiler observes tracked implementation and test evidence without claiming production callers', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-reality-'))
  execFileSync('git', ['init'], { cwd: root })
  execFileSync('git', ['config', 'user.email', 'test@example.com'], { cwd: root })
  execFileSync('git', ['config', 'user.name', 'Reality Test'], { cwd: root })

  mkdirSync(join(root, 'runtime/src'), { recursive: true })
  mkdirSync(join(root, 'runtime/tests'), { recursive: true })
  writeFileSync(join(root, 'runtime/Cargo.toml'), '[package]\nname = "chronica-core-world"\nversion = "0.1.0"\n')
  writeFileSync(join(root, 'runtime/src/lib.rs'), '// INV-WORLD-001\npub fn replay() {}\n')
  writeFileSync(join(root, 'runtime/tests/replay.rs'), '// INV-WORLD-001\n#[test]\nfn replay_works() {}\n')
  execFileSync('git', ['add', '.'], { cwd: root })
  execFileSync('git', ['commit', '-m', 'baseline'], { cwd: root })

  const atlas = {
    nodes: [{
      id: 'doc:architecture/foundation/world',
      title: 'World',
      kind: 'architecture-owner',
      path: 'docs/architecture/foundation/world.md',
      contractIds: ['INV-WORLD-001'],
      runtimeOwners: ['runtime/'],
    }],
  }

  const reality = compileReality(root, atlas)
  assert.equal(reality.owners.length, 1)
  assert.equal(reality.owners[0].implementationState, 'TESTED')
  assert.equal(reality.owners[0].directCodeEvidenceFiles.length, 2)
  assert.equal(reality.owners[0].directTestEvidenceFiles.length, 1)
  assert.equal(reality.owners[0].externalReferenceFiles, 0)
  assert.equal(reality.stats.architectureOwners, 1)
  assert.deepEqual(validateReality(reality, atlas), [])
})
