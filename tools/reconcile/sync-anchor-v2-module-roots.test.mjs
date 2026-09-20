import assert from 'node:assert/strict'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { buildCrateModuleIndex, hasTestEvidence, isModuleReferencedElsewhere } from './sync-anchor-v2-module-index.mjs'
import { makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

const write = (path, body) => {
  mkdirSync(join(path, '..'), { recursive: true })
  writeFileSync(path, body)
}

test('canonical sibling paths prevent a::leaf callers and tests from crediting b::leaf', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-module-sibling-leaves-')
  try {
    write(join(cratePath, 'src/a.rs'), 'pub mod leaf;\n')
    write(join(cratePath, 'src/a/leaf.rs'), 'pub fn run() {}\n')
    write(join(cratePath, 'src/b.rs'), 'pub mod leaf;\n')
    write(join(cratePath, 'src/b/leaf.rs'), 'pub fn run() {}\n')
    write(join(cratePath, 'src/lib.rs'), [
      'pub mod a;',
      'pub mod b;',
      'pub fn dispatch() { a::leaf::run(); }',
      '#[cfg(test)] mod tests { #[test] fn covers_a() { crate::a::leaf::run(); } }',
    ].join('\n'))
    const index = buildCrateModuleIndex(cratePath)
    const a = index.modules.find((module) => module.modulePath === 'a::leaf')
    const b = index.modules.find((module) => module.modulePath === 'b::leaf')
    assert.equal(isModuleReferencedElsewhere(index, a), true)
    assert.equal(isModuleReferencedElsewhere(index, b), false)
    assert.equal(hasTestEvidence(index, a), true)
    assert.equal(hasTestEvidence(index, b), false)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('caller and test evidence stays within the owning conventional bin root', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-module-bin-leaves-')
  try {
    for (const name of ['one', 'two']) {
      write(join(cratePath, `src/bin/${name}.rs`), `mod leaf;\n${name === 'one' ? 'fn main() { leaf::run(); }\n#[cfg(test)] mod tests { #[test] fn covers_leaf() { super::leaf::run(); } }' : 'fn main() {}'}\n`)
      write(join(cratePath, `src/bin/${name}/leaf.rs`), 'pub fn run() {}\n')
    }
    const index = buildCrateModuleIndex(cratePath)
    const one = index.modules.find((module) => module.rootId === 'bin:one' && module.modulePath === 'leaf')
    const two = index.modules.find((module) => module.rootId === 'bin:two' && module.modulePath === 'leaf')
    assert.ok(one && two)
    assert.equal(isModuleReferencedElsewhere(index, one), true)
    assert.equal(isModuleReferencedElsewhere(index, two), false)
    assert.equal(hasTestEvidence(index, one), true)
    assert.equal(hasTestEvidence(index, two), false)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('conventional src/bin directory root contributes children, callers, and cfg(test) tree', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-module-bin-tree-')
  try {
    write(join(cratePath, 'src/bin/worker/main.rs'), 'mod child;\n#[cfg(test)] mod tests;\nfn main() { child::run(); }\n')
    write(join(cratePath, 'src/bin/worker/child.rs'), 'pub fn run() {}\n')
    write(join(cratePath, 'src/bin/worker/tests.rs'), '#[test]\nfn covers_child() { super::child::run(); }\n')
    const index = buildCrateModuleIndex(cratePath)
    const child = index.modules.find((module) => module.rootId === 'bin:worker' && module.modulePath === 'child')
    assert.ok(child)
    assert.equal(isModuleReferencedElsewhere(index, child), true)
    assert.equal(index.testReachableByRoot.get('bin:worker').has(join(cratePath, 'src/bin/worker/tests.rs')), true)
    assert.equal(hasTestEvidence(index, child), true)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('explicit custom bin outside src contributes canonical modules and evidence', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-module-custom-bin-')
  try {
    write(join(cratePath, 'commands/launch.rs'), 'mod child;\nfn main() { child::run(); }\n')
    write(join(cratePath, 'commands/launch/child.rs'), 'pub fn run() {}\n#[cfg(test)] mod tests { #[test] fn works() { super::run(); } }\n')
    const manifest = '[[bin]]\nname = "launch"\npath = "commands/launch.rs"\n'
    const index = buildCrateModuleIndex(cratePath, undefined, manifest)
    const child = index.modules.find((module) => module.rootId === 'bin:launch' && module.modulePath === 'child')
    assert.ok(child)
    assert.equal(child.filePath, 'commands/launch/child.rs')
    assert.equal(isModuleReferencedElsewhere(index, child), true)
    assert.equal(hasTestEvidence(index, child), true)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('integration tests credit library ownership but never an unrelated bin root', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-module-integration-owner-')
  try {
    write(join(cratePath, 'src/lib.rs'), 'pub mod leaf;\n')
    write(join(cratePath, 'src/leaf.rs'), 'pub fn run() {}\n')
    write(join(cratePath, 'src/bin/tool.rs'), 'mod leaf;\nfn main() {}\n')
    write(join(cratePath, 'src/bin/tool/leaf.rs'), 'pub fn run() {}\n')
    write(join(cratePath, 'tests/leaf.rs'), '#[test]\nfn library_leaf() { chronica_fixture::leaf::run(); }\n')
    const index = buildCrateModuleIndex(cratePath)
    const library = index.modules.find((module) => module.rootId === 'lib:lib' && module.modulePath === 'leaf')
    const binary = index.modules.find((module) => module.rootId === 'bin:tool' && module.modulePath === 'leaf')
    assert.equal(hasTestEvidence(index, library), true)
    assert.equal(hasTestEvidence(index, binary), false)
  } finally { rmSync(root, { recursive: true, force: true }) }
})