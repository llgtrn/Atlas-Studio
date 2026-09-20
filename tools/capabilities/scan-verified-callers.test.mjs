import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { isRustIdent, parseRef, shippedBinRef, moduleNameFor, classifyRow, checkSymbolCalled } from './scan-verified-callers.mjs'

// Regression coverage for the 2026-08 island-scan hardening round. The scan
// (tools/capabilities/scan-verified-callers.mjs) is the mechanized ISLAND gate;
// three defects made it non-repeatable and produced false ISLAND positives:
//   1. A code_ref's `#` fragment is often provenance prose
//      (`probe_x.rs#main (real, non-test caller: shells out to curl ...)`), not a
//      symbol list. The old code split it on `,` and fed `main (real` straight into
//      `grep -E`, which crashed the whole scan ("grep: Unmatched ( or \(").
//   2. Many rows use the `path.rs:symbol` colon form instead of `path.rs#symbol`;
//      the old parser dropped every colon-form symbol, so those rows mis-classified.
//   3. A `src/bin/<name>.rs` caller is a shipped Cargo bin entry point (doctrine
//      §2.2) but is never imported by `binname::`, so bin-backed rows read as ISLAND.
// These tests pin all three fixes.

test('isRustIdent accepts identifiers and rejects prose / punctuation / line numbers', () => {
  assert.equal(isRustIdent('build_delivery'), true)
  assert.equal(isRustIdent('DatabaseManager'), true)
  assert.equal(isRustIdent('_private'), true)
  assert.equal(isRustIdent('main (real'), false)
  assert.equal(isRustIdent('42'), false)
  assert.equal(isRustIdent('a::b'), false)
  assert.equal(isRustIdent(''), false)
  assert.equal(isRustIdent(null), false)
})

test('parseRef: hash form extracts only identifier symbols, prose is discarded', () => {
  assert.deepEqual(parseRef('crates/c/src/x.rs#foo,bar'), {
    path: 'crates/c/src/x.rs',
    symbols: ['foo', 'bar'],
  })
  // The crash case: a prose annotation must yield zero symbols, never a grep-breaking token.
  const prose = 'crates/c/src/bin/probe_x.rs#main (real, non-test caller: shells out to curl, then calls foo)'
  const parsed = parseRef(prose)
  assert.equal(parsed.path, 'crates/c/src/bin/probe_x.rs')
  assert.deepEqual(parsed.symbols, [])
})

test('parseRef: colon form (path.rs:symbol) is parsed the same as the hash form', () => {
  assert.deepEqual(parseRef('crates/c/src/security_tool_wrappers.rs:classify_wafw00f_response'), {
    path: 'crates/c/src/security_tool_wrappers.rs',
    symbols: ['classify_wafw00f_response'],
  })
  // A bare `path.rs:42` line number is not an identifier and must not become a symbol.
  assert.deepEqual(parseRef('crates/c/src/x.rs:42'), { path: 'crates/c/src/x.rs', symbols: [] })
})

test('parseRef: a plain path with no fragment yields no symbols and does not crash', () => {
  assert.deepEqual(parseRef('crates/c/src/x.rs'), { path: 'crates/c/src/x.rs', symbols: [] })
})

test('moduleNameFor resolves mod.rs / lib.rs to their directory name', () => {
  assert.equal(moduleNameFor('crates/c/src/foo/bar.rs'), 'bar')
  assert.equal(moduleNameFor('crates/c/src/foo/mod.rs'), 'foo')
  assert.equal(moduleNameFor('crates/c/src/lib.rs'), 'src')
})

function makeCrateWithBin() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-scan-callers-'))
  mkdirSync(join(root, 'crates', 'chronica-osint', 'src', 'bin'), { recursive: true })
  const binPath = join(root, 'crates', 'chronica-osint', 'src', 'bin', 'probe_x.rs')
  writeFileSync(binPath, 'fn main() {}\n', 'utf8')
  return { root, binPath }
}

test('shippedBinRef finds an existing src/bin caller across both ref forms', () => {
  const { binPath } = makeCrateWithBin()
  // hash-form ref with prose
  assert.equal(shippedBinRef([`${binPath}#main (real, non-test caller: ...)`]), binPath)
  // colon-form ref
  assert.equal(shippedBinRef([`${binPath}:main`]), binPath)
  // a non-existent bin path is not counted
  assert.equal(shippedBinRef([`${binPath}.does-not-exist.rs#main`]), null)
  // a plain library file is not a bin
  assert.equal(shippedBinRef(['crates/chronica-osint/src/lib.rs#foo']), null)
})

test('classifyRow returns HAS_REAL_CALLER when a shipped bin caller is cited', () => {
  const { binPath } = makeCrateWithBin()
  const record = {
    status: 'verified',
    code_refs: [
      // colon-form library symbols the old parser dropped entirely
      'crates/chronica-osint/src/security_tool_wrappers.rs:classify_wafw00f_response',
      // the shipped bin that proves reachability
      `${binPath}#main (real, non-test caller: shells out to the real wafw00f CLI ...)`,
    ],
  }
  const result = classifyRow(record)
  assert.equal(result.classification, 'HAS_REAL_CALLER')
  assert.equal(result.hits[0].file, binPath)
})

test('classifyRow does not crash on a pure-prose hash fragment (the original bug)', () => {
  const record = {
    status: 'verified',
    code_refs: ['crates/chronica-osint/src/bin/probe_missing.rs#main (real, non-test caller: calls foo(real))'],
  }
  // File does not exist -> no checkable module -> null, but crucially: no throw.
  assert.doesNotThrow(() => classifyRow(record))
})

test('checkSymbolCalled detects a BARE flattened-name call, not just `.`/`::` calls', () => {
  // Regression for the 2026-08 round-37 fix: the caller shape `use crate::foo; ... foo(args)`
  // (a free fn imported by its flattened name and called with no path prefix) was missed by the
  // old `[.:]symbol(` pattern, causing systematic false PUB_USE_ONLY/ISLAND classifications.
  const root = mkdtempSync(join(tmpdir(), 'chronica-scan-barecall-'))
  mkdirSync(join(root, 'a'), { recursive: true })
  const def = join(root, 'a', 'def.rs')
  const caller = join(root, 'a', 'caller.rs')
  writeFileSync(def, 'pub fn apply_stealth(x: u32) -> u32 { x }\n', 'utf8')
  writeFileSync(
    caller,
    'use crate::a::def::apply_stealth;\nfn run() {\n    let _ = apply_stealth(42);\n}\n',
    'utf8',
  )
  const hits = checkSymbolCalled('apply_stealth', def, root)
  assert.equal(hits.length, 1)
  assert.equal(hits[0].file, caller)
})

test('checkSymbolCalled ignores the definition line and a longer identifier that ends in the symbol', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-scan-barecall2-'))
  mkdirSync(join(root, 'a'), { recursive: true })
  const def = join(root, 'a', 'def.rs')
  const other = join(root, 'a', 'other.rs')
  writeFileSync(def, 'pub fn balance_on(d: u32) -> u32 { d }\n', 'utf8')
  // `natural_balance_on(` must NOT count as a call of `balance_on` (word boundary), and the
  // fn definition line in def.rs must not count either.
  writeFileSync(other, 'fn natural_balance_on(d: u32) -> u32 { d }\n', 'utf8')
  const hits = checkSymbolCalled('balance_on', def, root)
  assert.equal(hits.length, 0)
})
