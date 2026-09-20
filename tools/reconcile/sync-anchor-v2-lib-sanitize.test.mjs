// sync-anchor-v2-lib-sanitize.test.mjs — verifyCrate/verifyCrates output-sanitization integration
// tests: long/secret/control-character VALID field content is sanitized on every emitted record
// (never a raw echo), and crate identity, source-tree symlink escapes, and doc/manifest failures
// surface as real, structured, non-suppressible operational findings end to end (repair round 5).
//
// Split out of sync-anchor-v2-lib.test.mjs purely to keep both files under this repo's ~500-line
// file-size convention.
import assert from 'node:assert/strict'
import { mkdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { BOUNDS, discoverWorkspaceCrates, REASON, verifyCrate, verifyCrates } from './sync-anchor-v2-lib.mjs'
import { REDACTED_TOKEN } from './sync-anchor-v2-sanitize.mjs'
import { buildAdversarialSecretShapes, cap, makeFixtureWorkspace, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

const secrets = buildAdversarialSecretShapes()

// ── target_module is always redacted without equality/order correlation ────────────────────

test('verifyCrate: target_module is always redacted, even for a real, legitimate module name', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-target-module-redacted-')
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.real', 'real_mod', 'unimplemented')) + '\n')
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.results[0].target_module, REDACTED_TOKEN)
    assert.ok(!JSON.stringify(report).includes('"real_mod"'), 'the raw target_module text must never appear anywhere in the serialized report')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: target_module output has no equality/order correlation reference', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-target-module-ref-')
  try {
    const rows = [
      cap('fixture.a', 'real_mod', 'unimplemented'),
      cap('fixture.b', 'other_target', 'unimplemented'),
      cap('fixture.c', 'real_mod', 'unimplemented'),
    ]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.results.length, 3)
    for (const r of report.results) {
      assert.equal(r.target_module, REDACTED_TOKEN)
      assert.equal('target_module_ref' in r, false)
      assert.equal('matched_module_refs' in r.evidence, false)
      assert.equal('real_match_refs' in r.evidence, false)
    }
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── item 5: long/secret/control-character VALID field content is sanitized on every surface ────

test('verifyCrate: a valid but abnormally long capability_key is sanitized in the result record (never a raw echo)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-sanitize-key-')
  try {
    const secretShaped = secrets.genericSkLive
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap(secretShaped, 'real_mod', 'unimplemented')) + '\n')
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.results.length, 1)
    assert.ok(!report.results[0].capability_key.includes(secretShaped))
    assert.equal(report.results[0].capability_key, REDACTED_TOKEN)
    // The serialized JSON (what --json/--out actually emit) also never contains the raw secret.
    assert.ok(!JSON.stringify(report).includes(secretShaped))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: a control-character-laden shard status is sanitized in the result record', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-sanitize-status-')
  try {
    const weirdStatus = 'unimplemented\x1b[31m; DROP TABLE\x07'
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.weird', 'real_mod', weirdStatus)) + '\n')
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.results[0].shard_status.includes('\x1b'), false)
    assert.equal(report.results[0].shard_status, REDACTED_TOKEN)
    // classification still correctly used the RAW status internally: an out-of-enum status is
    // still SHARD_STATUS_VALUE_INVALID, proving sanitization happens only at output construction,
    // never before the enum check that needs the real value.
    assert.equal(report.results[0].reason, REASON.SHARD_STATUS_VALUE_INVALID)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: sanitization never breaks doc-status cross-referencing (raw capability_key is still used for the Map lookup, even when the OUTPUT capability_key field is redacted)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-sanitize-lookup-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    // Round 6: capability_key now passes through OUTPUT when it matches its own strict grammar
    // (CAPABILITY_KEY_ALLOW_RE), so a single grotesquely-long segment (round 5's `longKey`) is now
    // ALSO excluded from the doc-table's Map at PARSE time by that same shared regex -- it would
    // never reach the lookup this test means to exercise. To keep testing "raw shard value drives
    // the Map lookup, independent of what the OUTPUT sanitizer later does with capability_key",
    // this key is built from MANY short (2-char) dot-separated segments: each segment individually
    // satisfies CAPABILITY_KEY_ALLOW_RE (so shard.mjs's doc-table filter -- which has no OVERALL
    // length cap of its own, only the shared per-segment regex -- still recognizes and indexes this
    // row), but the assembled string is well over sanitizeCapabilityKey's 96-character total cap,
    // so the OUTPUT capability_key field is still redacted. This reproduces the exact asymmetry the
    // test needs: found in the Map, redacted in the rendered result.
    const longKey = Array.from({ length: 40 }, (_, i) => `s${i}`).join('.')
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap(longKey, 'real_mod', 'unimplemented')) + '\n')
    writeFileSync(
      join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'),
      `# 900\n\n## 16. capabilities.db row(s)\n\n| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |\n| --- |--- |--- |--- |--- |--- |--- |\n| ${longKey} |implemented |0 |internal_write |chronica-fixture |real_mod | |\n`,
    )
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    // shard=unimplemented + real code -> SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL fires first
    // (higher-priority than the doc-status check), which itself proves the long key was correctly
    // looked up against the doc table (doc_status is non-null only if the Map lookup succeeded).
    assert.equal(report.results[0].doc_status, 'implemented')
    assert.equal(report.results[0].capability_key, REDACTED_TOKEN, 'the OUTPUT field is still redacted -- it exceeds the total-length cap even though every segment is individually valid')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrates: a duplicate-capability-key finding also sanitizes an abnormally long capability_key', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-sanitize-dup-')
  try {
    const longKey = `fixture.${'w'.repeat(400)}`
    const rows = [cap(longKey, 'real_mod', 'unimplemented'), cap(longKey, 'real_mod', 'verified')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    const summary = verifyCrates(root, ['chronica-fixture'])
    const dupFinding = summary.shard_findings.find((f) => f.reason === REASON.SHARD_CAPABILITY_KEY_DUPLICATE)
    assert.ok(dupFinding)
    assert.equal(dupFinding.capability_key, REDACTED_TOKEN)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 5: crate identity, source-tree symlink escapes, and doc/manifest failures are ──
// ── real, structured, non-suppressible operational findings end to end through verifyCrate(s) ───

test('verifyCrate: a genuinely LIVE crate name always passes sanitizeCrateName unchanged in every emitted record', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.crate, 'chronica-fixture')
    assert.ok(report.results.every((r) => r.crate === 'chronica-fixture'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: an unknown --crate argument shaped like a secret is redacted, never echoed, in the NOT_A_LIVE_WORKSPACE_CRATE result', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const secretShaped = `${secrets.genericBearer} ${secrets.stripeLive}`
    const report = verifyCrate(root, secretShaped, crates)
    assert.equal(report.applicable, false)
    assert.equal(report.reason, 'NOT_A_LIVE_WORKSPACE_CRATE')
    assert.notEqual(report.crate, secretShaped)
    assert.equal(report.crate, REDACTED_TOKEN)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: a source file that is a symlink escaping the crate source tree is a SOURCE_PATH_ESCAPES_ROOT operational finding, content never read (repair round 5, item 2)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-lib-source-symlink-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-lib-source-symlink-outside')
  try {
    mkdirSync(outside, { recursive: true })
    const marker = 'OUTSIDE_SOURCE_TREE_LIB_MARKER'
    writeFileSync(join(outside, 'evil.rs'), `pub fn run() -> u32 { /* ${marker} */ 1 }\n`)
    symlinkSync(join(outside, 'evil.rs'), join(cratePath, 'src', 'linked.rs'))
    writeFileSync(join(cratePath, 'src', 'lib.rs'), 'pub mod real_mod;\nmod linked;\n\npub fn dispatch() {\n    real_mod::run();\n    linked::run();\n}\n')
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.linked', 'linked', 'unimplemented')) + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.ok(report.operational_findings.some((f) => f.reason === REASON.SOURCE_PATH_ESCAPES_ROOT))
    assert.ok(!JSON.stringify(report).includes(marker), 'the outside-root source content must never be read/exposed')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('verifyCrates: an oversized member Cargo.toml surfaces as a MANIFEST_OVERSIZED operational finding, never silently drops the crate with no trace (repair round 5, item 4)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-lib-manifest-oversized-')
  try {
    mkdirSync(join(root, 'crates', 'chronica-huge'), { recursive: true })
    writeFileSync(join(root, 'crates', 'chronica-huge', 'Cargo.toml'), `[package]\nname = "chronica-huge"\n# ${'x'.repeat(BOUNDS.MAX_MANIFEST_BYTES + 10)}\n`)
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture", "crates/chronica-huge"]\n')

    const summary = verifyCrates(root, ['chronica-fixture'])
    assert.ok(summary.operational_findings.some((f) => f.reason === REASON.MANIFEST_OVERSIZED))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: a crate whose src/ path is a FILE, not a directory, surfaces as a DIR_ENUMERATION_UNREADABLE operational finding end to end, never a thrown exception (repair round 6, item b)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-lib-src-not-a-dir-')
  try {
    rmSync(join(cratePath, 'src'), { recursive: true, force: true })
    writeFileSync(join(cratePath, 'src'), 'not actually a directory\n')
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.applicable, false, 'the crate is refused entirely -- an unenumerable source tree can hide a real caller')
    assert.equal(report.reason, 'DIR_ENUMERATION_UNREADABLE')
    assert.equal(report.results.length, 0)
    assert.equal(report.operational_findings.length, 1)
    assert.equal(report.operational_findings[0].reason, REASON.DIR_ENUMERATION_UNREADABLE)
    assert.equal(report.operational_findings[0].code, 'ENOTDIR')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: an oversized crate doc surfaces as a DOC_OVERSIZED operational finding; capabilities still classify against CODE+SHARD alone (repair round 5, item 4)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-lib-doc-oversized-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    writeFileSync(join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'), `# 900\n${'x'.repeat(BOUNDS.MAX_DOC_BYTES + 10)}\n`)
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.real', 'real_mod', 'unimplemented')) + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.applicable, true)
    assert.equal(report.results.length, 1)
    assert.equal(report.results[0].reason, REASON.SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL, 'code+shard classification still runs even though the doc could not be consulted')
    assert.ok(report.operational_findings.some((f) => f.reason === REASON.DOC_OVERSIZED))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
