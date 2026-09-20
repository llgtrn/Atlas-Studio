// sync-anchor-v2-doc-status.test.mjs — loadCrateDocCapabilityStatus: unrecognized status text is
// kept separate, never normalized; bounded/symlink-safe doc reads and capped directory enumeration
// (repair round 5, items 2 and 4); plus (repair round 7, item 1) verifyCrate-level fail-closed
// semantics for missing/unverifiable doc coverage on a "claim" shard status.
//
// Split out of sync-anchor-v2-shard.test.mjs purely to keep both files under this repo's
// ~500-line file-size convention; the round-7 item-1 tests were moved here from
// sync-anchor-v2-lib.test.mjs for the same reason.
import assert from 'node:assert/strict'
import { mkdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { BOUNDS, loadCrateDocCapabilityStatus } from './sync-anchor-v2-shard.mjs'
import { discoverWorkspaceCrates, REASON, verifyCrate, verifyCrates } from './sync-anchor-v2-lib.mjs'
import { cap, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

test('loadCrateDocCapabilityStatus parses the generated crate-doc table', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    writeFileSync(
      join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'),
      `# 900

## 16. capabilities.db row(s)

| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |
| --- |--- |--- |--- |--- |--- |--- |
| fixture.real |unimplemented |0 |internal_write |chronica-fixture |real_mod | |
| fixture.done |implemented |0 |internal_write |chronica-fixture |real_mod | |
`,
    )
    const doc = loadCrateDocCapabilityStatus(root, 'chronica-fixture')
    assert.equal(doc.statuses.get('fixture.real'), 'unimplemented')
    assert.equal(doc.statuses.get('fixture.done'), 'implemented')
    assert.equal(doc.invalidStatuses.size, 0)
    assert.equal(doc.docFinding, null)
    rmSync(cratePath, { recursive: true, force: true })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateDocCapabilityStatus: an unrecognized status cell goes into invalidStatuses, never normalized into statuses', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-invalid-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    writeFileSync(
      join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'),
      `# 900

## 16. capabilities.db row(s)

| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |
| --- |--- |--- |--- |--- |--- |--- |
| fixture.garbage |kinda-done-ish |0 |internal_write |chronica-fixture |real_mod | |
| fixture.blank | |0 |internal_write |chronica-fixture |real_mod | |
`,
    )
    const doc = loadCrateDocCapabilityStatus(root, 'chronica-fixture')
    assert.equal(doc.statuses.has('fixture.garbage'), false)
    assert.equal(doc.statuses.has('fixture.blank'), false)
    assert.equal(doc.invalidStatuses.get('fixture.garbage'), 'kinda-done-ish')
    assert.equal(doc.invalidStatuses.get('fixture.blank'), '')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateDocCapabilityStatus: an over-length status cell is bounded, not stored raw/unbounded', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-bound-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    const longText = 'z'.repeat(1000)
    writeFileSync(
      join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'),
      `# 900

## 16. capabilities.db row(s)

| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |
| --- |--- |--- |--- |--- |--- |--- |
| fixture.garbage |${longText} |0 |internal_write |chronica-fixture |real_mod | |
`,
    )
    const doc = loadCrateDocCapabilityStatus(root, 'chronica-fixture')
    assert.ok(doc.invalidStatuses.get('fixture.garbage').length < 1000)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateDocCapabilityStatus: an oversized doc file is refused and reported, never partially read (repair round 5, item 4)', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-oversized-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    writeFileSync(join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'), `# 900\n${'x'.repeat(BOUNDS.MAX_DOC_BYTES + 10)}\n`)
    const doc = loadCrateDocCapabilityStatus(root, 'chronica-fixture')
    assert.equal(doc.statuses.size, 0)
    assert.ok(doc.docFinding)
    assert.equal(doc.docFinding.reason, 'DOC_OVERSIZED')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('loadCrateDocCapabilityStatus: a doc file that is a SYMLINK escaping root is refused, never read (repair round 5, item 2)', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-symlink-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-doc-symlink-outside')
  try {
    mkdirSync(outside, { recursive: true })
    const marker = 'OUTSIDE_ROOT_DOC_CONTENT'
    writeFileSync(join(outside, 'evil.md'), `# ${marker}\n`)
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    symlinkSync(join(outside, 'evil.md'), join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'))

    const doc = loadCrateDocCapabilityStatus(root, 'chronica-fixture')
    assert.ok(doc.docFinding)
    assert.equal(doc.docFinding.reason, 'DOC_UNREADABLE')
    assert.equal(doc.statuses.size, 0)
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('loadCrateDocCapabilityStatus: a docs/ directory listing truncated by its own entry cap is reported when the target file is not found within the cap', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-dir-truncated-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    // Every generated filename sorts BEFORE "900-crate-chronica-fixture.md" (numeric prefixes
    // 000-899), so a small injected bound truncates the listing before the real target is ever
    // reached -- proving the cap is enforced, not merely present.
    for (let i = 0; i < 20; i += 1) writeFileSync(join(root, 'docs', 'crates', `${String(i).padStart(3, '0')}-other.md`), '# other\n')
    writeFileSync(join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'), '# 900\n')

    const originalMax = BOUNDS.MAX_DOC_DIR_ENTRIES
    BOUNDS.MAX_DOC_DIR_ENTRIES = 5
    try {
      const doc = loadCrateDocCapabilityStatus(root, 'chronica-fixture')
      assert.equal(doc.path, null)
      assert.ok(doc.docFinding)
      assert.equal(doc.docFinding.reason, 'DOC_DIR_TRUNCATED')
    } finally {
      BOUNDS.MAX_DOC_DIR_ENTRIES = originalMax
    }
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 7, item 1: missing/unverifiable doc coverage on a "claim" status is never ──────
// ── silently normalized into AGREE -- absence of doc evidence is not evidence of doc agreement ──

test('verifyCrate: an "implemented" shard status with a doc that loaded successfully but has no row for this key is DOC_STATUS_MISSING_FOR_CLAIM, never silent AGREE', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-missing-for-claim-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    const shardRows = [cap('fixture.undocumented_claim', 'real_mod', 'implemented')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    // The doc file exists and loads fine -- it just has no row at all for fixture.undocumented_claim.
    writeFileSync(
      join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'),
      `# 900\n\n## 16. capabilities.db row(s)\n\n| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |\n| --- |--- |--- |--- |--- |--- |--- |\n| fixture.some_other_key |implemented |0 |internal_write |chronica-fixture |real_mod | |\n`,
    )

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.DOC_STATUS_MISSING_FOR_CLAIM)
    assert.equal(result.evidence.doc_available, true, 'the doc itself loaded fine -- only this specific key was absent from it')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: a "verified" shard status in a crate with NO docs/ directory at all is DOC_STATUS_UNVERIFIABLE_FOR_CLAIM, never silent AGREE', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-unverifiable-no-dir-')
  try {
    const shardRows = [cap('fixture.no_docs_dir_claim', 'real_mod', 'implemented')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    // No docs/ directory created at all -- doc coverage could not be attempted, not just absent.

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.DOC_STATUS_UNVERIFIABLE_FOR_CLAIM)
    assert.equal(result.evidence.doc_available, false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: a doc that exists but FAILED TO LOAD (oversized) with an "implemented_unverified" claim is DOC_STATUS_UNVERIFIABLE_FOR_CLAIM, not silent AGREE', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-unverifiable-oversized-')
  try {
    mkdirSync(join(root, 'docs', 'crates'), { recursive: true })
    writeFileSync(join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'), `# 900\n${'x'.repeat(BOUNDS.MAX_DOC_BYTES + 10)}\n`)
    const shardRows = [cap('fixture.doc_oversized_claim', 'real_mod', 'implemented_unverified')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'DISAGREE')
    assert.equal(result.reason, REASON.DOC_STATUS_UNVERIFIABLE_FOR_CLAIM)
    assert.equal(result.evidence.doc_available, false, 'the doc EXISTS but failed to load -- still unverifiable, not just absent')
    assert.ok(report.operational_findings.some((f) => f.reason === REASON.DOC_OVERSIZED), 'the doc-read failure is still ALSO surfaced as its own operational finding')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrate: an "unimplemented" shard status with no doc coverage at all stays AGREE -- nothing is claimed anywhere, so there is nothing a doc could contradict', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-missing-benign-')
  try {
    const shardRows = [cap('fixture.honestly_unimplemented', 'nonexistent_target', 'unimplemented')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    // No docs/ directory at all -- this must NOT be escalated, unlike the claim-status cases above.

    const crates = discoverWorkspaceCrates(root)
    const report = verifyCrate(root, 'chronica-fixture', crates)
    const result = report.results[0]
    assert.equal(result.verdict, 'AGREE', 'unimplemented + no doc claim anywhere is the honest baseline, not a gap to escalate')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('verifyCrates: DOC_STATUS_MISSING_FOR_CLAIM and DOC_STATUS_UNVERIFIABLE_FOR_CLAIM are both counted in disagree_by_reason', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-doc-missing-aggregate-')
  try {
    const shardRows = [cap('fixture.claim_one', 'real_mod', 'implemented'), cap('fixture.claim_two', 'real_mod', 'implemented_unverified')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    // No docs/ directory -- both claim-status rows are DOC_STATUS_UNVERIFIABLE_FOR_CLAIM (no doc
    // exists at all for the crate, so every claim row hits the same "unavailable" branch, never
    // MISSING_FOR_CLAIM in this scenario since there is no successfully-loaded doc to be silent).

    const summary = verifyCrates(root, ['chronica-fixture'])
    assert.equal(summary.disagree_by_reason[REASON.DOC_STATUS_UNVERIFIABLE_FOR_CLAIM], 2)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
