import assert from 'node:assert/strict'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { BOUNDS, discoverWorkspaceCrates, REASON, SourceCache, verifyCrate, verifyCrates } from './sync-anchor-v2-lib.mjs'
import { cap, makeCrateWithManyFiles, makeFixtureWorkspace, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

// Round 6 correction: capability_key/target_module now redact to ONE shared static token (no
// length/hash), so `.find(r => r.capability_key === redactValue(key))` can no longer distinguish
// between rows -- every row's capability_key is now byte-identical. `verifyCrate` preserves shard
// declaration order into `report.results` (sync-anchor-v2-shard.mjs's capabilityRows filter/map
// keeps input order; verifyCrate's `shard.capabilities.map(...)` does too), so tests instead index
// results positionally, matching each fixture's own declared row order below.
const FIXTURE_INDEX = {
  'fixture.real': 0,
  'fixture.stub': 1,
  'fixture.island_declared_unused': 2,
  'fixture.island_orphan': 3,
  'fixture.implemented_claim': 4,
  'fixture.verified_no_tests': 5,
  'fixture.verified_with_tests': 6,
  'fixture.doc_mismatch': 7,
}

// ── fixture-crate integration tests ─────────────────────────────────────────────────────────

test('verifyCrate: real reachable code with unimplemented shard status disagrees', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.real']]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: stub body with unimplemented shard status agrees', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.stub']]
    assert.equal(result.verdict, 'AGREE')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: declared-but-never-called real code is flagged ISLAND', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.island_declared_unused']]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: orphan file never declared via mod is flagged ISLAND', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.island_orphan']]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: shard claims implemented but only a stub body exists', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.implemented_claim']]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.SHARD_CLAIMS_IMPLEMENTED_BUT_NO_REAL_CODE_FOUND)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: shard claims verified but no test references the module', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.verified_no_tests']]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.SHARD_CLAIMS_TESTED_BUT_NO_TEST_FILE_REFERENCES_MODULE)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: shard claims verified and an inline #[test] exists -> agree', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.verified_with_tests']]
    assert.equal(result.verdict, 'AGREE')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: doc status contradicts shard status when code cannot resolve either claim', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.doc_mismatch']]
    // shard says unimplemented + no code found -> that alone would AGREE, but the crate doc claims
    // "implemented" for the same key, which the doc-status check must still catch.
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.DOC_STATUS_CONTRADICTS_SHARD_STATUS)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrates: aggregates counts and skips crates with no shard', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const summary = verifyCrates(root, ['chronica-fixture', 'chronica-does-not-exist'])
    assert.equal(summary.capabilities_checked, 8)
    assert.equal(summary.crates_checked, 1)
    assert.equal(summary.crates_skipped.length, 1)
    assert.equal(summary.crates_skipped[0].reason, 'NOT_A_LIVE_WORKSPACE_CRATE')
    assert.ok(summary.disagree_count >= 5)
    assert.ok(summary.agree_count >= 2)
    assert.equal(summary.shard_finding_count, 0)
    assert.deepEqual(summary.shard_findings, [])
    assert.equal(summary.workspace_finding_count, 0)
    assert.deepEqual(summary.workspace_findings, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrates: disagree_by_reason keys are sorted for deterministic output', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const summary = verifyCrates(root, ['chronica-fixture'])
    const keys = Object.keys(summary.disagree_by_reason)
    assert.deepEqual(keys, [...keys].sort())
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── shard-status enum validation: never a silent AGREE for an out-of-enum value ────────────────

test('classifyCapability: an out-of-enum shard status is explicit DISAGREE with SHARD_STATUS_VALUE_INVALID', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-invalid-status-')
  try {
    const shardRows = [
      { record_type: 'meta', crate: 'chronica-fixture' },
      cap('fixture.garbage_status', 'real_mod', 'in_progress'),
    ]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.SHARD_STATUS_VALUE_INVALID)
    assert.notEqual(result.verdict, 'AGREE')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('classifyCapability: an empty-string shard status is also explicit DISAGREE, never AGREE', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-invalid-status-')
  try {
    const shardRows = [
      { record_type: 'meta', crate: 'chronica-fixture' },
      cap('fixture.empty_status', 'real_mod', ''),
    ]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.SHARD_STATUS_VALUE_INVALID)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrates: SHARD_STATUS_VALUE_INVALID is counted in disagree_by_reason', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-invalid-status-')
  try {
    const shardRows = [cap('fixture.garbage_status', 'real_mod', 'not_a_real_status')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')

    const summary = verifyCrates(root, ['chronica-fixture'])
    assert.equal(summary.disagree_by_reason[REASON.SHARD_STATUS_VALUE_INVALID], 1)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── doc-status enum validation: never silently normalized to unimplemented/AGREE ───────────────

test('verifyCrate: an unrecognized doc-table status cell fires DOC_STATUS_VALUE_INVALID, never silently normalized to unimplemented/AGREE', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-status-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    // real_mod is a real, reachable file; give it a DIFFERENT unimplemented-target capability so
    // the shard status itself is internally consistent, isolating the doc-status check.
    writeFileSync(join(cratePath, 'src', 'stub_target.rs'), 'pub fn run() {\n    todo!()\n}\n')
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\nmod stub_target;\n\npub fn dispatch() {\n    real_mod::run();\n}\n',
    )
    const shardRows = [cap('fixture.garbage_doc', 'stub_target', 'unimplemented')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    writeFileSync(
      join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'),
      `# 900 - crate chronica-fixture

## 16. capabilities.db row(s)

| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |
| --- |--- |--- |--- |--- |--- |--- |
| fixture.garbage_doc |sort-of-done-maybe |0 |internal_write |chronica-fixture |stub_target | |
`,
    )

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.DOC_STATUS_VALUE_INVALID)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// repair round 7, item 1's fail-closed missing/unverifiable-doc-coverage tests live in
// sync-anchor-v2-doc-status.test.mjs now (moved there to keep this file under the ~500-line
// convention; thematically it already owned doc-status semantics).

// ── ambiguous module match: never a silent first-match pick among several real candidates ──────

test('classifyCapability: two real files matching the same single-segment target_module is AMBIGUOUS_MODULE_MATCH, not a silent first-pick', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-ambiguous-')
  try {
    // Two distinct real (>=6 substantial code lines, no todo!/unimplemented!) modules whose leaf
    // names both prefix-match a short single-segment target.
    writeFileSync(
      join(cratePath, 'src', 'billingcore.rs'),
      `
pub fn run() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        if i % 2 == 0 {
            total += i;
        } else {
            total -= i;
        }
    }
    total
}
`,
    )
    writeFileSync(
      join(cratePath, 'src', 'billingedge.rs'),
      `
pub fn run() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        if i % 3 == 0 {
            total += i * 2;
        } else {
            total += i;
        }
    }
    total
}
`,
    )
    writeFileSync(
      join(cratePath, 'src', 'lib.rs'),
      'pub mod real_mod;\nmod billingcore;\nmod billingedge;\n\npub fn dispatch() {\n    real_mod::run();\n    billingcore::run();\n    billingedge::run();\n}\n',
    )
    const shardRows = [cap('fixture.ambiguous', 'billing', 'unimplemented')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.AMBIGUOUS_MODULE_MATCH)
    assert.equal('real_match_refs' in result.evidence, false)
    assert.equal('real_matches' in result.evidence, false)
    assert.equal('matched_modules' in result.evidence, false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('classifyCapability: one real match among several candidates (others are stubs) is not ambiguous', () => {
  const { root } = makeFixtureWorkspace()
  try {
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[FIXTURE_INDEX['fixture.real']]
    assert.notEqual(result.reason, REASON.AMBIGUOUS_MODULE_MATCH)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── source-read caching: a shared cache is read from disk once per file, not once per capability ─

test('SourceCache: repeated reads of the same file return cached content without re-reading', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cache-')
  try {
    const cache = new SourceCache()
    const filePath = join(cratePath, 'src', 'real_mod.rs')
    const first = cache.read(filePath)
    writeFileSync(filePath, 'pub fn run() { /* mutated after first read */ }\n')
    const second = cache.read(filePath)
    assert.equal(second, first, 'second read must return the cached value, not re-read the mutated file')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: classifying many capabilities against the same crate reuses one SourceCache (no behavior change, just proves the plumbing)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cache-')
  try {
    const rows = Array.from({ length: 5 }, (_, i) => cap(`fixture.many_${i}`, 'real_mod', 'unimplemented'))
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.results.length, 5)
    assert.ok(report.results.every((r) => r.reason === REASON.SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── SOURCE_FILES_TRUNCATED: a truncated source-file listing refuses the whole crate, never AGREE ──

test('verifyCrate: a source-file listing exceeding BOUNDS.MAX_SOURCE_FILES_PER_CRATE refuses the whole crate (never a partial-scan AGREE/success)', () => {
  const fileCount = BOUNDS.MAX_SOURCE_FILES_PER_CRATE + 5
  const { root, cratePath, modNames } = makeCrateWithManyFiles('chronica-sync-anchor-v2-many-files-', fileCount)
  try {
    // A capability whose real implementation happens to be one of the (likely-truncated) files --
    // if the truncation bug were still present, this could silently resolve to a false AGREE.
    const targetModule = modNames[modNames.length - 1]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.many', targetModule, 'unimplemented')) + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.applicable, false)
    assert.equal(report.reason, 'SOURCE_FILES_TRUNCATED')
    assert.equal(report.results.length, 0, 'no capability may be classified (AGREE or DISAGREE) against an incomplete file listing')
    assert.equal(report.operational_findings.length, 1)
    assert.equal(report.operational_findings[0].reason, REASON.SOURCE_FILES_TRUNCATED)
    assert.equal(report.operational_findings[0].file_count_limit, BOUNDS.MAX_SOURCE_FILES_PER_CRATE)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, { timeout: 30_000 })

test('verifyCrates: SOURCE_FILES_TRUNCATED is counted in operational_finding_count, capabilities_checked stays 0 for that crate', () => {
  const fileCount = BOUNDS.MAX_SOURCE_FILES_PER_CRATE + 5
  const { root, cratePath, modNames } = makeCrateWithManyFiles('chronica-sync-anchor-v2-many-files-agg-', fileCount)
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.many', modNames[0], 'unimplemented')) + '\n')
    const summary = verifyCrates(root, ['chronica-fixture'])
    assert.equal(summary.capabilities_checked, 0)
    assert.equal(summary.crates_skipped[0].reason, 'SOURCE_FILES_TRUNCATED')
    assert.ok(summary.operational_finding_count >= 1)
    assert.ok(summary.operational_findings.some((f) => f.reason === REASON.SOURCE_FILES_TRUNCATED))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, { timeout: 30_000 })

// ── SOURCE_FILE_OVERSIZED: an individual oversized source file is a per-file operational finding ──

test('verifyCrate: an oversized individual source file is a SOURCE_FILE_OVERSIZED operational finding; other capabilities in the same crate still classify normally', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-oversized-integration-')
  try {
    const bigPath = join(cratePath, 'src', 'huge_mod.rs')
    writeFileSync(bigPath, 'X'.repeat(3 * 1024 * 1024)) // 3MB > BOUNDS.MAX_SOURCE_FILE_BYTES (2MB)
    writeFileSync(join(cratePath, 'src', 'lib.rs'), 'pub mod real_mod;\nmod huge_mod;\n\npub fn dispatch() {\n    real_mod::run();\n}\n')
    const rows = [cap('fixture.real', 'real_mod', 'unimplemented'), cap('fixture.huge', 'huge_mod', 'unimplemented')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    assert.equal(report.applicable, true, 'the crate itself is still applicable -- only the one oversized file is refused')
    assert.equal(report.results.length, 2, 'both capabilities are still classified')
    const realResult = report.results[FIXTURE_INDEX['fixture.real']]
    assert.equal(realResult.reason, REASON.SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL, 'a capability matching a NORMAL file is unaffected')
    assert.equal(report.operational_findings.length, 1)
    assert.equal(report.operational_findings[0].reason, REASON.SOURCE_FILE_OVERSIZED)
    assert.match(report.operational_findings[0].path, /huge_mod\.rs$/)
    assert.equal(report.operational_findings[0].size_bytes, 3 * 1024 * 1024)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
