import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { maskRustNonCode } from './sync-anchor-v2-rust-lexer.mjs'
import { buildCrateModuleIndex, hasTestEvidence, isModuleReferencedElsewhere } from './sync-anchor-v2-module-index.mjs'
import { makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

test('lexer masks nested comments and every Rust string family but preserves lifetimes', () => {
  const input = `/* outer /* fake::call() */ */ "normal::call" b"byte::call" c"c::call" r##"raw::call"## br#"braw::call"# cr"craw::call" 'x' b'y' &'life str`
  const masked = maskRustNonCode(input)
  for (const spoof of ['fake::call', 'normal::call', 'byte::call', 'c::call', 'raw::call', 'braw::call', 'craw::call']) assert.doesNotMatch(masked, new RegExp(spoof.replace('::', '::')))
  assert.match(masked, /'life/)
})

test('cfg(any(test, feature)) and cfg_attr remain production-reachable', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-compound-cfg-')
  try {
    writeFileSync(join(cratePath, 'src', 'lib.rs'), `pub mod real_mod;\n#[cfg(any(test, feature = "x"))]\nfn spoof(){ real_mod::run(); }\n#[cfg_attr(test, cfg(unix))]\nfn also_spoof(){ real_mod::run(); }\n`)
    const index = buildCrateModuleIndex(cratePath); const target = index.modules.find((m) => m.modulePath === 'real_mod')
    assert.equal(isModuleReferencedElsewhere(index, target), true)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('raw and normal strings cannot spoof callers or test evidence', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-string-spoof-')
  try {
    writeFileSync(join(cratePath, 'src', 'lib.rs'), `pub mod real_mod; fn text(){ let _ = r#"real_mod::run(); #[test]"#; let _ = "real_mod::run()"; }`)
    const index = buildCrateModuleIndex(cratePath); const target = index.modules.find((m) => m.modulePath === 'real_mod')
    assert.equal(isModuleReferencedElsewhere(index, target), false)
    assert.equal(hasTestEvidence(index, target), false)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('src/bin and explicit Cargo bin roots are discovered with separate canonical identities', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-bins-')
  try {
    mkdirSync(join(cratePath, 'src', 'bin'), { recursive: true })
    writeFileSync(join(cratePath, 'src', 'bin', 'worker.rs'), 'mod duplicate; fn main(){ duplicate::run(); }')
    mkdirSync(join(cratePath, 'src', 'bin', 'duplicate'), { recursive: true })
    writeFileSync(join(cratePath, 'src', 'bin', 'duplicate.rs'), 'pub fn run(){ let mut x=0; for i in 0..10 { x += i; } assert!(x>0); }')
    writeFileSync(join(cratePath, 'custom.rs'), 'fn main(){}')
    writeFileSync(join(cratePath, 'Cargo.toml'), '[package]\nname="chronica-fixture"\nversion="0.1.0"\n[[bin]]\nname="custom"\npath="custom.rs"\n')
    const manifest = '[package]\nname="chronica-fixture"\nversion="0.1.0"\n[[bin]]\nname="custom"\npath="custom.rs"\n'
    const index = buildCrateModuleIndex(cratePath, undefined, manifest)
    assert.ok(index.roots.some((r) => r.id === 'bin:worker'))
    assert.ok(index.roots.some((r) => r.id === 'bin:custom'))
    assert.equal(new Set(index.roots.map((r) => r.id)).size, index.roots.length)
  } finally { rmSync(root, { recursive: true, force: true }) }
})
