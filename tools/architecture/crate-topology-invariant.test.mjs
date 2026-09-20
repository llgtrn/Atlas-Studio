import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

import { CANONICAL_CRATE_ROOTS, verifyCanonicalCrateTopology } from './crate-topology-invariant.mjs'

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-crate-topology-'))
  for (const name of CANONICAL_CRATE_ROOTS) mkdirSync(join(root, 'crates', name), { recursive: true })
  return root
}

test('accepts the one canonical core/cap/adapter/runtime substrate', () => {
  const root = fixture()
  try {
    writeFileSync(join(root, 'crates', 'README.md'), 'files at crates/ root are harmless\n')
    const result = verifyCanonicalCrateTopology({ root })
    assert.equal(result.ok, true)
    assert.deepEqual(result.errors, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('accepts crates/infra as the one non-product firewalled exception', () => {
  const root = fixture()
  try {
    mkdirSync(join(root, 'crates', 'infra', 'chronica-infra-build-fabric-core'), { recursive: true })
    const result = verifyCanonicalCrateTopology({ root })
    assert.equal(result.ok, true)
    assert.deepEqual(result.errors, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects a domain-specific parallel substrate such as crates/fintech', () => {
  const root = fixture()
  try {
    mkdirSync(join(root, 'crates', 'fintech', 'core'), { recursive: true })
    const result = verifyCanonicalCrateTopology({ root })
    assert.equal(result.ok, false)
    assert.ok(result.errors.some((error) => error.startsWith('non-canonical crate root: crates/fintech')))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('rejects a missing canonical substrate layer', () => {
  const root = fixture()
  try {
    rmSync(join(root, 'crates', 'runtime'), { recursive: true, force: true })
    const result = verifyCanonicalCrateTopology({ root })
    assert.equal(result.ok, false)
    assert.ok(result.errors.includes('missing canonical crate root: crates/runtime'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
