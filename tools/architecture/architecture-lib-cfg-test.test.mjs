import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { discoverWorkspaceArchitecture, findTestOnlyModuleFiles } from './architecture-lib.mjs'
import { _internal } from './cfg-test-module-classifier.mjs'

const { LIMITS, TRUTH, cfgImpliesTestOnly, parseCfgNode, isCfgTestPredicate, parseAttribute } = _internal

// Focused coverage for the cfg(test) module-truth classifier: a file only ever reachable through a
// `#[cfg(test)]`-gated `mod` declaration must never become a product architecture node/edge, while
// every other case (plain sibling, `#[cfg_attr(test, ...)]`, nested test module) is classified
// correctly and deterministically.

function writeFixture(root) {
  mkdirSync(join(root, 'src', 'codegen', 'nested_wrap'), { recursive: true })

  // lib.rs owns the crate's own top-level modules: one plain production module ('prod'), and the
  // 'codegen' module (itself a directory owning further nested modules below).
  writeFileSync(
    join(root, 'src', 'lib.rs'),
    `mod prod;\nmod codegen;\n`,
  )
  writeFileSync(join(root, 'src', 'prod.rs'), `pub fn real_production_fn() -> bool { true }\n`)

  // codegen.rs exercises every classification case in one file:
  //   - inline single-line #[cfg(test)] mod
  //   - multiline #[cfg(\n test\n)] mod, combined with a #[path] override
  //   - #[cfg_attr(test, ...)] (NOT cfg(test) itself) on a real production sibling -- must NOT be excluded
  //   - a #[cfg(test)] inline mod BLOCK containing a further `mod nested_child;` declaration with no
  //     attribute of its own -- inherits test-only-ness from the enclosing block
  writeFileSync(
    join(root, 'src', 'codegen.rs'),
    [
      'pub fn real_codegen_fn() -> bool { true }',
      '',
      '#[cfg(test)]',
      'mod inline_tests;',
      '',
      '#[cfg(',
      '    test',
      ')]',
      '#[path = "codegen_other_tests.rs"]',
      'mod other_tests;',
      '',
      '#[cfg_attr(test, allow(dead_code))]',
      'mod cfg_attr_prod;',
      '',
      '#[cfg(test)]',
      'mod nested_wrap {',
      '    mod nested_child;',
      '}',
      '',
    ].join('\n'),
  )
  writeFileSync(join(root, 'src', 'codegen', 'inline_tests.rs'), `#[test]\nfn t() { assert!(true); }\n`)
  writeFileSync(join(root, 'src', 'codegen_other_tests.rs'), `#[test]\nfn t2() { assert!(true); }\n`)
  writeFileSync(join(root, 'src', 'codegen', 'cfg_attr_prod.rs'), `pub fn still_shipped() -> bool { true }\n`)
  writeFileSync(join(root, 'src', 'codegen', 'nested_wrap', 'nested_child.rs'), `#[test]\nfn t3() { assert!(true); }\n`)

  return {
    prodLib: join(root, 'src', 'lib.rs'),
    prodSibling: join(root, 'src', 'prod.rs'),
    codegen: join(root, 'src', 'codegen.rs'),
    inlineTests: join(root, 'src', 'codegen', 'inline_tests.rs'),
    otherTests: join(root, 'src', 'codegen_other_tests.rs'),
    cfgAttrProd: join(root, 'src', 'codegen', 'cfg_attr_prod.rs'),
    nestedChild: join(root, 'src', 'codegen', 'nested_wrap', 'nested_child.rs'),
  }
}

function makeCrateRoot() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-cfgtest-'))
  const files = writeFixture(root)
  return { root, files }
}

test('findTestOnlyModuleFiles: plain sibling module is never classified test-only', () => {
  const { files } = makeCrateRoot()
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.prodSibling, files.codegen])
  assert.equal(testOnly.has(files.prodSibling), false)
})

test('findTestOnlyModuleFiles: inline single-line #[cfg(test)] mod is classified test-only', () => {
  const { files } = makeCrateRoot()
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.codegen, files.inlineTests])
  assert.equal(testOnly.has(files.inlineTests), true)
})

test('findTestOnlyModuleFiles: multiline #[cfg(\\n test\\n)] with #[path] override is classified test-only', () => {
  const { files } = makeCrateRoot()
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.codegen, files.otherTests])
  assert.equal(testOnly.has(files.otherTests), true)
})

test('findTestOnlyModuleFiles: #[cfg_attr(test, ...)] is NOT treated as #[cfg(test)] (negative case)', () => {
  const { files } = makeCrateRoot()
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.codegen, files.cfgAttrProd])
  assert.equal(testOnly.has(files.cfgAttrProd), false)
})

test('findTestOnlyModuleFiles: a mod nested inside a #[cfg(test)] inline block inherits test-only-ness', () => {
  const { files } = makeCrateRoot()
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.codegen, files.nestedChild])
  assert.equal(testOnly.has(files.nestedChild), true)
})

test('findTestOnlyModuleFiles is deterministic across repeated runs on the same files', () => {
  const { files } = makeCrateRoot()
  const all = [files.prodLib, files.prodSibling, files.codegen, files.inlineTests, files.otherTests, files.cfgAttrProd, files.nestedChild]
  const first = [...findTestOnlyModuleFiles(all)].sort()
  const second = [...findTestOnlyModuleFiles(all)].sort()
  const third = [...findTestOnlyModuleFiles(all)].sort()
  assert.deepEqual(first, second)
  assert.deepEqual(second, third)
  assert.deepEqual(first, [files.inlineTests, files.nestedChild, files.otherTests].sort())
})

test('discoverWorkspaceArchitecture: no product node/edge exists for any #[cfg(test)]-only module, and production siblings are untouched', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-cfgtest-ws-'))
  writeFileSync(join(root, 'Cargo.toml'), `[workspace]\nmembers = [\n  "crates/chronica-cfgtest-fixture",\n]\n`)
  mkdirSync(join(root, 'crates', 'chronica-cfgtest-fixture'), { recursive: true })
  writeFileSync(
    join(root, 'crates', 'chronica-cfgtest-fixture', 'Cargo.toml'),
    `[package]\nname = "chronica-cfgtest-fixture"\n`,
  )
  writeFixture(join(root, 'crates', 'chronica-cfgtest-fixture'))

  const arch = discoverWorkspaceArchitecture(root)
  const nodeIds = new Set(arch.nodes.map((n) => n.id))
  const crate = 'chronica-cfgtest-fixture'

  // production modules: present, status 'generated' (truthfully shipped)
  for (const modulePath of ['lib', 'prod', 'codegen', 'codegen::cfg_attr_prod']) {
    assert.equal(nodeIds.has(`module:${crate}:${modulePath}`), true, `expected production node for ${modulePath}`)
  }
  assert.equal(
    arch.edges.some((e) => e.fromNode === `crate:${crate}` && e.toNode === `module:${crate}:codegen::cfg_attr_prod` && e.kind === 'owns'),
    true,
  )

  // #[cfg(test)]-only modules: absent entirely -- no node, no edge, under any plausible module path
  for (const modulePath of ['codegen::inline_tests', 'codegen_other_tests', 'codegen::nested_wrap::nested_child', 'codegen::nested_wrap']) {
    assert.equal(nodeIds.has(`module:${crate}:${modulePath}`), false, `expected NO node for test-only ${modulePath}`)
  }
  assert.equal(
    arch.edges.some((e) => e.toNode.startsWith(`module:${crate}:codegen::inline_tests`) || e.toNode.startsWith(`module:${crate}:codegen_other_tests`) || e.toNode.includes('nested_child')),
    false,
  )

  // whole-graph determinism: rerunning discovery on the same fixture yields an identical node/edge set
  const again = discoverWorkspaceArchitecture(root)
  const sortById = (rows) => [...rows].sort((a, b) => a.id.localeCompare(b.id))
  const sortEdges = (rows) => [...rows].sort((a, b) => `${a.fromNode}|${a.toNode}|${a.kind}`.localeCompare(`${b.fromNode}|${b.toNode}|${b.kind}`))
  assert.deepEqual(sortById(arch.nodes), sortById(again.nodes))
  assert.deepEqual(sortEdges(arch.edges), sortEdges(again.edges))
})

// ── cfg predicate boolean logic: unit-level, no file I/O ────────────────────────────────────────
// Does satisfying this predicate REQUIRE `test` to be on (cfgImpliesTestOnly)? `all(...)` is a
// conjunction (false if ANY conjunct is false, so one test-implying conjunct suffices); `any(...)`
// is a disjunction (only test-only if EVERY disjunct is); `not(...)` is conservatively never
// test-only.

test('cfgImpliesTestOnly: bare test is test-only', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('test')), true)
})

test('cfgImpliesTestOnly: all(test) is test-only (a conjunction with one test-implying conjunct)', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('all(test)')), true)
})

test('cfgImpliesTestOnly: all(test, feature = "x") is still test-only -- ANDing anything else with test cannot be true outside a test build', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('all(test, feature = "x")')), true)
})

test('cfgImpliesTestOnly: any(test, feature = "x") is NOT test-only -- the feature disjunct alone can be true outside test', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('any(test, feature = "x")')), false)
})

test('cfgImpliesTestOnly: any(test, test) IS test-only -- every disjunct independently implies test-only', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('any(test, test)')), true)
})

test('cfgImpliesTestOnly: not(test) is conservatively NOT test-only (it is the opposite: production-only)', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('not(test)')), false)
})

test('cfgImpliesTestOnly: an opaque feature predicate alone is never test-only', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('feature = "x"')), false)
})

test('cfgImpliesTestOnly: nested all(any(test, test), test) is test-only', () => {
  assert.equal(cfgImpliesTestOnly(parseCfgNode('all(any(test, test), test)')), true)
})

test('isCfgTestPredicate: recognizes #[cfg(all(test))] as test-implying (the reported regression)', () => {
  assert.equal(isCfgTestPredicate('#[cfg(all(test))]'), true)
})

test('isCfgTestPredicate: recognizes #[cfg(all(test, feature = "x"))] as test-implying', () => {
  assert.equal(isCfgTestPredicate('#[cfg(all(test, feature = "x"))]'), true)
})

test('isCfgTestPredicate: does not treat #[cfg(any(test, feature = "x"))] as test-implying', () => {
  assert.equal(isCfgTestPredicate('#[cfg(any(test, feature = "x"))]'), false)
})

test('isCfgTestPredicate: does not treat #[cfg(not(test))] as test-implying', () => {
  assert.equal(isCfgTestPredicate('#[cfg(not(test))]'), false)
})

test('isCfgTestPredicate: a non-cfg attribute is never test-implying', () => {
  assert.equal(isCfgTestPredicate('#[allow(dead_code)]'), false)
})

test('bounded evaluator accepts 4096 bytes and rejects 4097 before recursive parsing', () => {
  const exact = `#[cfg(${` `.repeat(LIMITS.maxTextLength - 12)}test)]`
  assert.equal(exact.length, LIMITS.maxTextLength)
  assert.equal(isCfgTestPredicate(exact), true)
  assert.equal(isCfgTestPredicate(`${exact} `), true, 'trim happens once before the bound')
  assert.equal(isCfgTestPredicate(`${exact}x`), false)
})

test('oversized top-level and recursively applied cfg_attr attributes are UNKNOWN/production-preserving', () => {
  const state = { exceeded: false, nodes: 0 }
  assert.equal(parseAttribute(`#[cfg(${`x`.repeat(4097)})]`, state), TRUTH.UNKNOWN)
  assert.equal(state.exceeded, true)
  assert.equal(isCfgTestPredicate(`#[cfg_attr(unix, cfg(${`x`.repeat(4097)}))]`), false)
})

test('malformed quote/paren, depth, node, empty, and opaque inputs remain production-preserving', () => {
  const cases = [
    '#[cfg(all(test)]', '#[cfg(feature = "unterminated)]', '#[cfg()]', '#[cfg(opaque(call))]',
    `#[cfg(${'all('.repeat(LIMITS.maxDepth + 1)}test${')'.repeat(LIMITS.maxDepth + 1)})]`,
    `#[cfg(any(${Array.from({ length: LIMITS.maxNodes + 1 }, () => 'test').join(',')}))]`,
  ]
  for (const value of cases) assert.equal(isCfgTestPredicate(value), false, value)
})

test('nested cfg_attr compile gates use bounded three-valued implication semantics', () => {
  assert.equal(isCfgTestPredicate('#[cfg_attr(unix, cfg(test))]'), false)
  assert.equal(isCfgTestPredicate('#[cfg_attr(test, cfg(test))]'), false)
  assert.equal(isCfgTestPredicate('#[cfg_attr(not(test), cfg(test))]'), true)
  assert.equal(isCfgTestPredicate('#[cfg_attr(not(test), cfg_attr(not(test), cfg(test)))]'), true)
})

// ── production-reachability override: a file with BOTH a production and a test-cfg reference ───
// must stay a real product node -- it genuinely ships via its production reference regardless of
// any separate #[cfg(test)] reference elsewhere.

function writeDualReferenceFixture(root) {
  mkdirSync(join(root, 'src'), { recursive: true })
  // 'shared.rs' is referenced TWICE: once plainly from lib.rs (production) and once behind
  // #[cfg(test)] from a sibling module (fixtures.rs) via a #[path] override pointing at the SAME
  // file. It must be classified as a real product node -- it ships in a release build.
  writeFileSync(
    join(root, 'src', 'lib.rs'),
    ['mod shared;', 'mod fixtures;', 'mod compound_cfg;', ''].join('\n'),
  )
  writeFileSync(join(root, 'src', 'shared.rs'), `pub fn shared_prod_fn() -> bool { true }\n`)
  writeFileSync(
    join(root, 'src', 'fixtures.rs'),
    ['#[cfg(test)]', '#[path = "shared.rs"]', 'mod shared_test_alias;', ''].join('\n'),
  )
  // compound_cfg.rs exercises #[cfg(all(test))] (must be excluded, the reported regression) and
  // #[cfg(any(test, feature = "x"))] (must stay included -- conservative, non-hiding)
  writeFileSync(
    join(root, 'src', 'compound_cfg.rs'),
    [
      '#[cfg(all(test))]',
      'mod all_test_only;',
      '',
      '#[cfg(any(test, feature = "x"))]',
      'mod any_test_or_feature;',
      '',
    ].join('\n'),
  )
  mkdirSync(join(root, 'src', 'compound_cfg'), { recursive: true })
  writeFileSync(join(root, 'src', 'compound_cfg', 'all_test_only.rs'), `#[test]\nfn t() { assert!(true); }\n`)
  writeFileSync(join(root, 'src', 'compound_cfg', 'any_test_or_feature.rs'), `pub fn maybe_shipped() -> bool { true }\n`)

  return {
    prodLib: join(root, 'src', 'lib.rs'),
    shared: join(root, 'src', 'shared.rs'),
    fixtures: join(root, 'src', 'fixtures.rs'),
    compoundCfg: join(root, 'src', 'compound_cfg.rs'),
    allTestOnly: join(root, 'src', 'compound_cfg', 'all_test_only.rs'),
    anyTestOrFeature: join(root, 'src', 'compound_cfg', 'any_test_or_feature.rs'),
  }
}

test('findTestOnlyModuleFiles: a file referenced from BOTH production and #[cfg(test)] stays a product file', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-dualref-'))
  const files = writeDualReferenceFixture(root)
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.shared, files.fixtures, files.compoundCfg])
  assert.equal(testOnly.has(files.shared), false, 'shared.rs has a real production reference and must not be excluded')
})

test('findTestOnlyModuleFiles: #[cfg(all(test))] is classified test-only (the reported regression)', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-dualref-'))
  const files = writeDualReferenceFixture(root)
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.compoundCfg, files.allTestOnly, files.anyTestOrFeature])
  assert.equal(testOnly.has(files.allTestOnly), true)
})

test('findTestOnlyModuleFiles: #[cfg(any(test, feature = "x"))] stays classified as production (conservative)', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-dualref-'))
  const files = writeDualReferenceFixture(root)
  const testOnly = findTestOnlyModuleFiles([files.prodLib, files.compoundCfg, files.allTestOnly, files.anyTestOrFeature])
  assert.equal(testOnly.has(files.anyTestOrFeature), false)
})

test('discoverWorkspaceArchitecture: a dual-referenced (production + cfg(test)) file keeps its product node', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-dualref-ws-'))
  writeFileSync(join(root, 'Cargo.toml'), `[workspace]\nmembers = [\n  "crates/chronica-dualref-fixture",\n]\n`)
  mkdirSync(join(root, 'crates', 'chronica-dualref-fixture'), { recursive: true })
  writeFileSync(
    join(root, 'crates', 'chronica-dualref-fixture', 'Cargo.toml'),
    `[package]\nname = "chronica-dualref-fixture"\n`,
  )
  writeDualReferenceFixture(join(root, 'crates', 'chronica-dualref-fixture'))

  const arch = discoverWorkspaceArchitecture(root)
  const nodeIds = new Set(arch.nodes.map((n) => n.id))
  const crate = 'chronica-dualref-fixture'

  assert.equal(nodeIds.has(`module:${crate}:shared`), true, 'dual-referenced shared.rs must keep its product node')
  assert.equal(nodeIds.has(`module:${crate}:compound_cfg::any_test_or_feature`), true, 'any(test, feature) stays conservative/production')
  assert.equal(nodeIds.has(`module:${crate}:compound_cfg::all_test_only`), false, 'all(test) is genuinely test-only')
})

test('transitive reachability: descendants of an external test module remain test-only, while a shared production descendant wins', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-transitive-'))
  mkdirSync(join(root, 'src', 'tests', 'child'), { recursive: true })
  writeFileSync(join(root, 'src', 'lib.rs'), 'mod production;\n#[cfg(test)] mod tests;\n')
  writeFileSync(join(root, 'src', 'production.rs'), '#[path = "tests/shared.rs"] mod shared;\n')
  writeFileSync(join(root, 'src', 'tests.rs'), 'mod child;\nmod shared;\n')
  writeFileSync(join(root, 'src', 'tests', 'child.rs'), 'mod nested;\n')
  writeFileSync(join(root, 'src', 'tests', 'child', 'nested.rs'), '#[test]\nfn nested() {}\n')
  writeFileSync(join(root, 'src', 'tests', 'shared.rs'), 'pub fn shipped_through_production() {}\n')
  const files = [
    join(root, 'src', 'lib.rs'), join(root, 'src', 'production.rs'), join(root, 'src', 'tests.rs'),
    join(root, 'src', 'tests', 'child.rs'), join(root, 'src', 'tests', 'child', 'nested.rs'),
    join(root, 'src', 'tests', 'shared.rs'),
  ]
  const testOnly = findTestOnlyModuleFiles(files)
  assert.equal(testOnly.has(join(root, 'src', 'tests', 'child.rs')), true)
  assert.equal(testOnly.has(join(root, 'src', 'tests', 'child', 'nested.rs')), true)
  assert.equal(testOnly.has(join(root, 'src', 'tests', 'shared.rs')), false, 'independent production reachability takes precedence')
})

test('inline nested #[path] uses the inline module directory context', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-inline-path-'))
  mkdirSync(join(root, 'src', 'outer', 'inner'), { recursive: true })
  writeFileSync(join(root, 'src', 'lib.rs'), [
    '#[cfg(test)]', 'mod outer {', '  mod inner {', '    #[path = "fixture.rs"]', '    mod leaf;', '  }', '}', '',
  ].join('\n'))
  const leaf = join(root, 'src', 'outer', 'inner', 'fixture.rs')
  writeFileSync(leaf, '#[test]\nfn inline_path() {}\n')
  assert.equal(findTestOnlyModuleFiles([join(root, 'src', 'lib.rs'), leaf]).has(leaf), true)
})

test('workspace module identity follows declaration aliases rather than physical directories', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-identity-'))
  const crateRoot = join(root, 'crates', 'chronica-identity-fixture')
  mkdirSync(join(crateRoot, 'src', 'router'), { recursive: true })
  mkdirSync(join(crateRoot, 'src', 'routes'), { recursive: true })
  writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-identity-fixture"]\n')
  writeFileSync(join(crateRoot, 'Cargo.toml'), '[package]\nname = "chronica-identity-fixture"\n')
  writeFileSync(join(crateRoot, 'src', 'lib.rs'), 'mod router;\n')
  writeFileSync(join(crateRoot, 'src', 'router', 'mod.rs'), '#[path = "../routes/erp_reports.rs"]\nmod erp_reports;\n')
  writeFileSync(join(crateRoot, 'src', 'routes', 'erp_reports.rs'), 'pub fn report() {}\n')
  const nodes = discoverWorkspaceArchitecture(root).nodes.filter((node) => node.crate === 'chronica-identity-fixture' && node.kind === 'module')
  assert.deepEqual(nodes.map((node) => node.modulePath).sort(), ['lib', 'router', 'router::erp_reports'])
  const report = nodes.find((node) => node.modulePath === 'router::erp_reports')
  assert.equal(report.filePath, 'crates/chronica-identity-fixture/src/routes/erp_reports.rs')
  assert.equal(nodes.some((node) => node.modulePath === 'routes::erp_reports'), false)
})

test('cfg-dependent canonical collisions preserve every production physical target and nested target', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-collision-'))
  writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers=["crates/collision"]\n')
  const crateRoot = join(root, 'crates', 'collision')
  mkdirSync(join(crateRoot, 'src', 'unix', 'platform'), { recursive: true })
  mkdirSync(join(crateRoot, 'src', 'windows', 'platform'), { recursive: true })
  writeFileSync(join(crateRoot, 'Cargo.toml'), '[package]\nname="collision"\nversion="0.0.0"\n')
  writeFileSync(join(crateRoot, 'src', 'lib.rs'), [
    '#[cfg(unix)] #[path="unix/platform.rs"] mod platform;',
    '#[cfg(windows)] #[path="windows/platform.rs"] mod platform;',
  ].join('\n'))
  writeFileSync(join(crateRoot, 'src', 'unix', 'platform.rs'), 'mod nested;\n')
  writeFileSync(join(crateRoot, 'src', 'unix', 'platform', 'nested.rs'), '')
  writeFileSync(join(crateRoot, 'src', 'windows', 'platform.rs'), 'mod nested;\n')
  writeFileSync(join(crateRoot, 'src', 'windows', 'platform', 'nested.rs'), '')
  const modules = discoverWorkspaceArchitecture(root).nodes.filter((node) => node.crate === 'collision' && node.kind === 'module')
  for (const canonical of ['platform', 'platform::nested']) {
    const variants = modules.filter((node) => node.modulePath === canonical)
    assert.equal(variants.length, 2)
    assert.equal(new Set(variants.map((node) => node.filePath)).size, 2)
    assert.equal(new Set(variants.map((node) => node.id)).size, 2)
  }
})

test('test-only canonical collision cannot erase independently reachable production evidence', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-arch-test-collision-'))
  writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers=["crates/collision"]\n')
  const crateRoot = join(root, 'crates', 'collision')
  mkdirSync(join(crateRoot, 'src', 'prod'), { recursive: true })
  mkdirSync(join(crateRoot, 'src', 'fixtures'), { recursive: true })
  writeFileSync(join(crateRoot, 'Cargo.toml'), '[package]\nname="collision"\nversion="0.0.0"\n')
  writeFileSync(join(crateRoot, 'src', 'lib.rs'), [
    '#[path="prod/service.rs"] mod service;',
    '#[cfg(test)] #[path="fixtures/service.rs"] mod service;',
  ].join('\n'))
  writeFileSync(join(crateRoot, 'src', 'prod', 'service.rs'), '')
  writeFileSync(join(crateRoot, 'src', 'fixtures', 'service.rs'), '')
  const service = discoverWorkspaceArchitecture(root).nodes.filter((node) => node.crate === 'collision' && node.modulePath === 'service')
  assert.deepEqual(service.map((node) => node.filePath), ['crates/collision/src/prod/service.rs'])
})
