import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import {
  isSymbolToken,
  symbolsFromFragment,
  parseCitation,
  findMissing,
} from './scan-verified-citations.mjs'

// This gate exists because a `#` fragment is a symbol list in most rows but freeform provenance
// prose in others. The prose case is the trap: comma-splitting `main (real, ... reaches enumerate,
// build_probe_request, Prober ...)` yields bare identifiers that are not symbols cited for that
// file, producing false "missing symbol" findings that would drive a false demotion. These tests
// pin the prose-robustness guard and the six real citation forms.

test('isSymbolToken: identifiers and Type::method paths only', () => {
  assert.equal(isSymbolToken('build_delivery'), true)
  assert.equal(isSymbolToken('CancellationRegistry::create_token'), true)
  assert.equal(isSymbolToken('tests::unit_events_sequential_order'), true)
  assert.equal(isSymbolToken('main (real'), false)
  assert.equal(isSymbolToken('reaches'), true) // a bare word IS a valid token in isolation
  assert.equal(isSymbolToken('42'), false)
  assert.equal(isSymbolToken(''), false)
})

test('symbolsFromFragment: a clean list returns tokens; ANY prose token voids the whole list', () => {
  assert.deepEqual(symbolsFromFragment('SiteCheck,classify_site,app_exists'), [
    'SiteCheck',
    'classify_site',
    'app_exists',
  ])
  assert.deepEqual(symbolsFromFragment('CompanyOverview,parse_overview_response'), [
    'CompanyOverview',
    'parse_overview_response',
  ])
  // The false-positive case: one token has a space+paren, so the fragment is prose -> no symbols.
  assert.deepEqual(
    symbolsFromFragment(
      'main (real, non-test caller: reaches enumerate, build_probe_request, Prober, interpolate_username through one program)',
    ),
    [],
  )
  assert.deepEqual(symbolsFromFragment(''), [])
})

test('parseCitation handles all six real-world forms', () => {
  assert.deepEqual(parseCitation('crates/c/src/x.rs'), { path: 'crates/c/src/x.rs', symbols: [] })
  assert.deepEqual(parseCitation('crates/c/src/x.rs#A,B'), { path: 'crates/c/src/x.rs', symbols: ['A', 'B'] })
  assert.deepEqual(parseCitation('crates/c/src/x.rs#T::method,C'), {
    path: 'crates/c/src/x.rs',
    symbols: ['T::method', 'C'],
  })
  // hash + prose -> file kept, zero symbols
  assert.deepEqual(parseCitation('crates/c/src/bin/probe_x.rs#main (real, non-test caller: ...)'), {
    path: 'crates/c/src/bin/probe_x.rs',
    symbols: [],
  })
  // path + parenthetical annotation
  assert.deepEqual(parseCitation('crates/c/src/x.rs (ErrorKind, classify_permanence, decide_retry)'), {
    path: 'crates/c/src/x.rs',
    symbols: ['ErrorKind', 'classify_permanence', 'decide_retry'],
  })
  // colon form
  assert.deepEqual(parseCitation('crates/c/src/x.rs:classify_wafw00f_response'), {
    path: 'crates/c/src/x.rs',
    symbols: ['classify_wafw00f_response'],
  })
})

function makeShardEntry(record) {
  return { record, line: 1, file: 'x.jsonl' }
}

test('findMissing flags a genuinely renamed/moved symbol but not a prose mention', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-citecheck-'))
  mkdirSync(join(root, 'crates', 'c', 'src'), { recursive: true })
  const impl = join(root, 'crates', 'c', 'src', 'x.rs')
  writeFileSync(impl, 'pub fn execute_event() {}\npub struct ActionDefinition;\n', 'utf8')
  const testsFile = join(root, 'crates', 'c', 'src', 'x_tests.rs')
  writeFileSync(testsFile, 'fn unit_ok() {}\n', 'utf8')

  const entries = [
    makeShardEntry({
      capability_key: 'w.good',
      domain: 'w',
      status: 'verified',
      code_refs: [`${impl}#execute_event,ActionDefinition`],
      // prose ref: names execute_event in prose but must NOT be symbol-checked against the bin
      test_refs: [`${impl}#main (real caller: reaches execute_event and ActionDefinition somehow)`],
    }),
    makeShardEntry({
      capability_key: 'w.stale',
      domain: 'w',
      status: 'verified',
      // symbol moved to x_tests.rs -> must be flagged against x.rs
      test_refs: [`${impl}#unit_ok`],
    }),
    makeShardEntry({
      capability_key: 'w.notverified',
      domain: 'w',
      status: 'implemented_unverified',
      code_refs: [`${impl}#does_not_exist`], // not verified -> ignored
    }),
  ]
  const { checkedRows, missingFile, missingSymbol } = findMissing(entries)
  assert.equal(checkedRows, 2) // only the two verified rows
  assert.equal(missingFile.length, 0)
  assert.equal(missingSymbol.length, 1)
  assert.equal(missingSymbol[0].key, 'w.stale')
  assert.equal(missingSymbol[0].sym, 'unit_ok')
})

test('findMissing flags a missing file', () => {
  const entries = [
    makeShardEntry({
      capability_key: 'w.ghost',
      domain: 'w',
      status: 'verified',
      code_refs: ['crates/c/src/does_not_exist.rs#Foo'],
    }),
  ]
  const { missingFile, missingSymbol } = findMissing(entries)
  assert.equal(missingFile.length, 1)
  assert.equal(missingSymbol.length, 0)
  assert.equal(missingFile[0].path, 'crates/c/src/does_not_exist.rs')
})
