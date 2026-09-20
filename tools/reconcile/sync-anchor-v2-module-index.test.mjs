import assert from 'node:assert/strict'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { buildCrateModuleIndex, buildReachableModuleFiles, classifyFileBody, hasTestEvidence, isModuleReferencedElsewhere, matchCapabilityModule } from './sync-anchor-v2-module-index.mjs'
import { makeFixtureWorkspace, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

// ── pure-helper unit tests ──────────────────────────────────────────────────────────────────

test('classifyFileBody: todo!() macro with little else is a stub', () => {
  assert.equal(classifyFileBody('pub fn run() {\n    todo!()\n}\n'), 'stub')
})

test('classifyFileBody: empty file body is empty', () => {
  assert.equal(classifyFileBody('// just a comment\n'), 'empty')
})

test('classifyFileBody: substantial logic is real', () => {
  const text = `
    pub struct Registry { entries: Vec<Entry> }
    impl Registry {
      pub fn find(&self, name: &str) -> Option<&Entry> {
        for entry in &self.entries {
          if entry.name == name {
            return Some(entry);
          }
        }
        None
      }
      pub fn describe(&self, name: &str) -> String {
        format!("entry: {}", name)
      }
    }
  `
  assert.equal(classifyFileBody(text), 'real')
})

test('matchCapabilityModule: exact normalized match wins over prefix', () => {
  const modules = [
    { modulePath: 'ui::search', filePath: 'src/ui/search.rs', absPath: '/x/src/ui/search.rs' },
    { modulePath: 'search_index', filePath: 'src/search_index.rs', absPath: '/x/src/search_index.rs' },
  ]
  const result = matchCapabilityModule('ui/search', modules)
  assert.equal(result.confidence, 'exact')
  assert.deepEqual(result.matches.map((m) => m.modulePath), ['ui::search'])
})

test('matchCapabilityModule: prefix tier catches scripts -> scripts_registry', () => {
  const modules = [{ modulePath: 'scripts_registry', filePath: 'src/scripts_registry.rs', absPath: '/x/src/scripts_registry.rs' }]
  const result = matchCapabilityModule('scripts', modules)
  assert.equal(result.confidence, 'prefix')
  assert.equal(result.matches.length, 1)
})

test('matchCapabilityModule: short generic tokens do not fuzzy-match', () => {
  const modules = [{ modulePath: 'command_surface', filePath: 'src/command_surface.rs', absPath: '/x/src/command_surface.rs' }]
  const result = matchCapabilityModule('cli', modules)
  assert.equal(result.matches.length, 0)
  assert.equal(result.confidence, null)
})

test('matchCapabilityModule: no match returns empty', () => {
  const result = matchCapabilityModule('completely_unrelated_target', [])
  assert.deepEqual(result, { matches: [], confidence: null })
})

// ── fixture-crate integration tests ─────────────────────────────────────────────────────────

test('buildReachableModuleFiles: only mod-declared files from lib.rs/main.rs are reachable', () => {
  const { root, cratePath } = makeFixtureWorkspace()
  try {
    const reachable = buildReachableModuleFiles(join(cratePath, 'src'))
    const rel = (p) => p.slice(cratePath.length + 1)
    const files = [...reachable].map(rel).sort()
    assert.ok(files.includes('src/real_mod.rs'))
    assert.ok(files.includes('src/declared_but_unused.rs'))
    assert.ok(!files.includes('src/island_mod.rs'), 'orphan file with no mod declaration must not be reachable')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('buildCrateModuleIndex resolves nested module paths with :: and lists files in sorted order', () => {
  const { root, cratePath } = makeFixtureWorkspace()
  try {
    const index = buildCrateModuleIndex(cratePath)
    const paths = index.modules.map((m) => m.modulePath).sort()
    assert.ok(paths.includes('real_mod'))
    assert.ok(paths.includes('lib'))
    const filePaths = index.allFiles
    assert.deepEqual(filePaths, [...filePaths].sort(), 'file walk order must be deterministic/sorted, not filesystem-dependent')
    assert.equal(index.sourceFilesTruncated, false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('production reachability honors cfg boolean semantics and ignores cfg_attr as a gate', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cfg-semantics-')
  try {
    for (const name of ['all_test', 'any_test', 'attr_test']) writeFileSync(join(cratePath, 'src', `${name}.rs`), 'pub fn run() {}\n')
    writeFileSync(join(cratePath, 'src', 'lib.rs'), [
      '#[cfg(all(test, feature = "x"))]\nmod all_test;',
      '#[cfg(any(test, feature = "x"))]\nmod any_test;',
      '#[cfg_attr(test, allow(dead_code))]\nmod attr_test;',
      'pub fn dispatch() { any_test::run(); attr_test::run(); }',
    ].join('\n'))
    const index = buildCrateModuleIndex(cratePath)
    assert.equal(index.reachable.has(join(cratePath, 'src', 'all_test.rs')), false)
    assert.equal(index.testReachable.has(join(cratePath, 'src', 'all_test.rs')), true)
    assert.equal(index.reachable.has(join(cratePath, 'src', 'any_test.rs')), true)
    assert.equal(index.reachable.has(join(cratePath, 'src', 'attr_test.rs')), true)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('astral Unicode before comments and literals cannot shift masking or manufacture modules', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-astral-')
  try {
    writeFileSync(join(cratePath, 'src', 'ghost.rs'), 'pub fn run() {}\n')
    writeFileSync(join(cratePath, 'src', 'lib.rs'), [
      'const A: &str = "😀 mod ghost;";',
      'const B: &str = r#"🚀 mod ghost;"#;',
      '// 🛰 mod ghost;',
      '/* 🌍 mod ghost; */',
      'pub mod real_mod;',
      'pub fn dispatch() { real_mod::run(); }',
    ].join('\n'))
    const index = buildCrateModuleIndex(cratePath)
    assert.equal(index.reachable.has(join(cratePath, 'src', 'ghost.rs')), false)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('duplicate explicit bin root IDs fail closed before reachableByRoot insertion', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-duplicate-bin-')
  try {
    writeFileSync(join(cratePath, 'src', 'one.rs'), 'fn main() {}\n')
    writeFileSync(join(cratePath, 'src', 'two.rs'), 'fn main() {}\n')
    const manifest = '[[bin]]\nname="same"\npath="src/one.rs"\n[[bin]]\nname="same"\npath="src/two.rs"\n'
    const index = buildCrateModuleIndex(cratePath, undefined, manifest)
    assert.deepEqual(index.rootEnumeration.duplicateRootIds, ['bin:same'])
    assert.equal(index.reachableByRoot.size, 0)
  } finally { rmSync(root, { recursive: true, force: true }) }
})

// ── repair round 6, item (1): reachability/test-evidence semantics correction ───────────────────

test('isModuleReferencedElsewhere: a reference from an ORPHAN file (never reached by any mod declaration) does not count as a real caller', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-orphan-ref-')
  try {
    // lib.rs only wires up real_mod -- `orphan.rs` exists on disk but is never `mod`-declared
    // anywhere, so it is dead code, not part of the compiled crate, exactly like island_mod.rs in
    // makeFixtureWorkspace -- except this orphan file ALSO references real_mod::, which (before
    // this round's fix) was enough to wrongly mark real_mod "referenced elsewhere".
    writeFileSync(join(cratePath, 'src', 'orphan.rs'), 'pub fn run() {\n    real_mod::run();\n}\n')
    const index = buildCrateModuleIndex(cratePath)
    const realMod = index.modules.find((m) => m.modulePath === 'real_mod')
    assert.ok(realMod)
    assert.ok(index.reachable.has(realMod.absPath), 'real_mod IS reachable via lib.rs\'s own `pub mod real_mod;`')
    const orphan = index.modules.find((m) => m.modulePath === 'orphan')
    assert.ok(orphan)
    assert.ok(!index.reachable.has(orphan.absPath), 'orphan.rs must not be in the reachable set -- nothing ever `mod`-declares it')
    // real_mod is declared/called from lib.rs's own dispatch(), so it IS genuinely referenced --
    // the point of this test is that this is DECIDED without ever needing the orphan's own
    // (irrelevant) reference; removing the orphan file entirely would not change the verdict.
    assert.equal(isModuleReferencedElsewhere(index, realMod), true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('isModuleReferencedElsewhere: a module referenced ONLY from an orphan file (no production caller at all) is correctly NOT referenced elsewhere', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-orphan-only-ref-')
  try {
    // island_target IS mod-declared (so it is itself reachable/compiled) but dispatch() never
    // calls it -- its ONLY textual reference anywhere is from orphan.rs, a file nothing ever
    // `mod`-declares. Before this round's fix (which scanned ALL files, not just `reachable`
    // ones), this orphan reference would have wrongly marked island_target as having a real
    // caller.
    writeFileSync(join(cratePath, 'src', 'island_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    writeFileSync(join(cratePath, 'src', 'orphan.rs'), 'pub fn run() {\n    island_target::run();\n}\n')
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod island_target;\n\npub fn dispatch() {\n    real_mod::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    const islandTarget = index.modules.find((m) => m.modulePath === 'island_target')
    assert.ok(islandTarget)
    assert.ok(index.reachable.has(islandTarget.absPath), 'island_target IS mod-declared, so it is itself compiled/reachable')
    assert.equal(isModuleReferencedElsewhere(index, islandTarget), false, 'a reference from a file nothing ever mod-declares must not count as a real caller')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('isModuleReferencedElsewhere: a reference that exists ONLY inside a #[cfg(test)] module does not count as a real (production) caller', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cfg-test-only-ref-')
  try {
    writeFileSync(join(cratePath, 'src', 'billing_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    // caller.rs has NO production call to billing_target:: -- the only occurrence is inside a
    // #[cfg(test)] mod tests block, which is conditionally compiled only for `cargo test`.
    writeFileSync(
      join(cratePath, 'src', 'caller.rs'),
      'pub fn run() {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn calls_billing_target() {\n        billing_target::run();\n    }\n}\n',
    )
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod billing_target;\nmod caller;\n\npub fn dispatch() {\n    real_mod::run();\n    caller::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    const billingTarget = index.modules.find((m) => m.modulePath === 'billing_target')
    assert.ok(billingTarget)
    assert.equal(isModuleReferencedElsewhere(index, billingTarget), false, 'a #[cfg(test)]-gated reference is not part of the shipped binary and must not count as a real caller')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('isModuleReferencedElsewhere: a PRODUCTION reference alongside an unrelated #[cfg(test)] block still counts (the fix only strips test blocks, not the whole file)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cfg-test-plus-real-ref-')
  try {
    writeFileSync(join(cratePath, 'src', 'billing_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    writeFileSync(
      join(cratePath, 'src', 'caller.rs'),
      'pub fn run() {\n    billing_target::run();\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn unrelated() {\n        assert!(true);\n    }\n}\n',
    )
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod billing_target;\nmod caller;\n\npub fn dispatch() {\n    real_mod::run();\n    caller::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    const billingTarget = index.modules.find((m) => m.modulePath === 'billing_target')
    assert.equal(isModuleReferencedElsewhere(index, billingTarget), true, 'the real production call in caller::run() must still be found once the unrelated cfg(test) block is stripped')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('hasTestEvidence: a crate-level integration test under tests/ (Rust convention, sibling of src/) is recognized as test evidence', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-integration-test-')
  try {
    writeFileSync(join(cratePath, 'src', 'payments_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod payments_target;\n\npub fn dispatch() {\n    real_mod::run();\n    payments_target::run();\n}\n',
    )
    mkdirSync(join(cratePath, 'tests'), { recursive: true })
    // repair round 7, item 3: the match now requires an actual `leaf::` module-path reference (the
    // same bar isModuleReferencedElsewhere already applies), not a bare word -- a realistic
    // integration test calls the crate's module the same way any other external consumer would.
    writeFileSync(
      join(cratePath, 'tests', 'payments_integration.rs'),
      'use chronica_fixture::payments_target;\n\n#[test]\nfn integration_test_covers_it() {\n    // integration tests exercise the crate as an external consumer would\n    assert_eq!(1 + 1, 2);\n    let _ = payments_target::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    assert.equal(index.integrationTestFiles.length, 1)
    const payments = index.modules.find((m) => m.modulePath === 'payments_target')
    assert.ok(payments)
    assert.equal(hasTestEvidence(index, payments), true, 'a #[test] file under tests/ mentioning the module name must count as test evidence')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 7, item 3: #[cfg(test)] EXTERNAL modules, bare cfg(test) functions, and orphan ──
// ── files that merely happen to mention a name must never count as a real (production) caller ───

test('isModuleReferencedElsewhere: a reference inside a #[cfg(test)]-gated EXTERNAL module (mod tests; resolving to its own file) does not count as a real caller', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cfg-test-external-mod-')
  try {
    writeFileSync(join(cratePath, 'src', 'billing_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    // `tests.rs` is a SEPARATE FILE, only ever compiled because lib.rs's `mod tests;` is gated by
    // #[cfg(test)] -- round 6's stripCfgTestBlocks only stripped INLINE `{ ... }` bodies, so this
    // external-file declaration was still walked into the PRODUCTION reachable set entirely.
    writeFileSync(join(cratePath, 'src', 'tests.rs'), '#[test]\nfn calls_billing_target() {\n    billing_target::run();\n}\n')
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod billing_target;\n\n#[cfg(test)]\nmod tests;\n\npub fn dispatch() {\n    real_mod::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    const testsFile = join(cratePath, 'src', 'tests.rs')
    assert.ok(!index.reachable.has(testsFile), 'a #[cfg(test)]-gated external mod must never be in the PRODUCTION reachable set')
    assert.ok(index.testReachable.has(testsFile), 'but it IS genuinely compiled under cargo test, so it belongs in the test-only reachable set')
    const billingTarget = index.modules.find((m) => m.modulePath === 'billing_target')
    assert.equal(isModuleReferencedElsewhere(index, billingTarget), false, 'a reference only reachable via a #[cfg(test)] external mod is not a production caller')
    assert.equal(hasTestEvidence(index, billingTarget), true, 'but it IS legitimate test evidence -- genuinely compiled and exercised under cargo test')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('isModuleReferencedElsewhere: a reference inside a BARE #[cfg(test)] function (no enclosing mod block) does not count as a real caller', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cfg-test-bare-fn-')
  try {
    writeFileSync(join(cratePath, 'src', 'billing_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    writeFileSync(
      join(cratePath, 'src', 'caller.rs'),
      'pub fn run() {}\n\n#[cfg(test)]\nfn exercises_billing_target() {\n    billing_target::run();\n}\n',
    )
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod billing_target;\nmod caller;\n\npub fn dispatch() {\n    real_mod::run();\n    caller::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    const billingTarget = index.modules.find((m) => m.modulePath === 'billing_target')
    assert.equal(isModuleReferencedElsewhere(index, billingTarget), false, 'a reference inside a bare #[cfg(test)] fn (no mod block) must not count as a real caller')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('hasTestEvidence: an ORPHAN file (never mod-declared under ANY configuration) with a #[test] and an incidental mention of the leaf name is NOT test evidence', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-orphan-test-evidence-')
  try {
    writeFileSync(join(cratePath, 'src', 'billing_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    // orphan_tests.rs is never mod-declared anywhere -- not by lib.rs, not under any #[cfg(test)]
    // gate -- so the crate would not compile it under ANY configuration. Its #[test] function
    // merely NAMES "billing_target" in a comment/string, with no actual `billing_target::` call.
    writeFileSync(
      join(cratePath, 'src', 'orphan_tests.rs'),
      '#[test]\nfn unrelated_dummy_test() {\n    // this test has nothing to do with billing_target, it just mentions the word\n    let billing_target = "not a real reference";\n    assert_eq!(billing_target, "not a real reference");\n}\n',
    )
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod billing_target;\n\npub fn dispatch() {\n    real_mod::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    const orphanFile = join(cratePath, 'src', 'orphan_tests.rs')
    assert.ok(!index.reachable.has(orphanFile) && !index.testReachable.has(orphanFile), 'the orphan file is reachable under NO configuration')
    const billingTarget = index.modules.find((m) => m.modulePath === 'billing_target')
    assert.equal(hasTestEvidence(index, billingTarget), false, 'an orphan file merely mentioning the name must never count as test evidence')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('hasTestEvidence: a REACHABLE (production) file with a #[test] but only an incidental word mention (not a leaf:: call) is NOT test evidence', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-bare-word-not-evidence-')
  try {
    writeFileSync(join(cratePath, 'src', 'billing_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    // caller.rs IS production-reachable (mod-declared, called from dispatch), and its OWN
    // #[cfg(test)] mod tests block mentions "billing_target" only as a local dummy variable name,
    // never as an actual billing_target:: call -- round 6's bare-word `\bleaf\b` scan would have
    // wrongly counted this; round 7 requires an actual `leaf::` reference.
    writeFileSync(
      join(cratePath, 'src', 'caller.rs'),
      'pub fn run() {\n    real_mod_helper();\n}\n\nfn real_mod_helper() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn dummy() {\n        let billing_target = 42;\n        assert_eq!(billing_target, 42);\n    }\n}\n',
    )
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod billing_target;\nmod caller;\n\npub fn dispatch() {\n    real_mod::run();\n    caller::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    const billingTarget = index.modules.find((m) => m.modulePath === 'billing_target')
    assert.equal(hasTestEvidence(index, billingTarget), false, 'a dummy local variable sharing the leaf name is not a leaf:: reference and must not count as test evidence')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('hasTestEvidence: with no inline #[test] and no tests/ directory at all, a module has no test evidence', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-no-test-evidence-')
  try {
    writeFileSync(join(cratePath, 'src', 'untested_target.rs'), 'pub fn run() -> u32 {\n    let mut t = 0;\n    for i in 0..8 { t += i; }\n    t\n}\n')
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\npub mod untested_target;\n\npub fn dispatch() {\n    real_mod::run();\n    untested_target::run();\n}\n',
    )
    const index = buildCrateModuleIndex(cratePath)
    assert.deepEqual(index.integrationTestFiles, [])
    const untested = index.modules.find((m) => m.modulePath === 'untested_target')
    assert.equal(hasTestEvidence(index, untested), false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
