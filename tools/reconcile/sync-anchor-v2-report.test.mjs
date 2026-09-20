import assert from 'node:assert/strict'
import { rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { discoverWorkspaceCrates, verifyCrates } from './sync-anchor-v2-lib.mjs'
import { renderHumanSummary, renderMarkdownSummary } from './sync-anchor-v2-report.mjs'
import { cap, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

function summaryWithFindings() {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-report-')
  const lines = [
    JSON.stringify(cap('fixture.real', 'real_mod', 'unimplemented')),
    // a shard status crafted to try to break a Markdown table if ever echoed unescaped
    JSON.stringify(cap('fixture.injected', 'real_mod', 'bad | status `code` \n line')),
    'not json {{{',
  ]
  writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), lines.join('\n') + '\n')
  const crates = discoverWorkspaceCrates(root)
  const summary = verifyCrates(root, crates.map((c) => c.name))
  return { root, summary }
}

test('renderHumanSummary: discloses structural findings and does not silently omit a nonzero shard_finding_count', () => {
  const { root, summary } = summaryWithFindings()
  try {
    const lines = renderHumanSummary(summary)
    const text = lines.join('\n')
    assert.ok(summary.shard_finding_count > 0)
    assert.match(text, /shard structural finding/)
    assert.match(text, /SHARD_PARSE_ERROR/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('renderHumanSummary: never throws for an all-clear (empty) summary', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-report-clean-')
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    const crates = discoverWorkspaceCrates(root)
    const summary = verifyCrates(root, crates.map((c) => c.name))
    assert.doesNotThrow(() => renderHumanSummary(summary))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('renderMarkdownSummary: a crafted shard status cannot break the Markdown table structure', () => {
  const { root, summary } = summaryWithFindings()
  try {
    const md = renderMarkdownSummary(summary)
    // Count pipe-delimited cells per data row stays consistent -- a raw unescaped `|` in shard
    // content would otherwise inject an extra table column/row.
    const disagreeRow = md.split('\n').find((l) => l.includes('bad'))
    if (disagreeRow) {
      // an escaped pipe renders as the literal sequence \| (backslash-pipe), never a bare |
      assert.ok(!/[^\\]\|.*\|.*\|.*\|.*\|.*bad/.test(disagreeRow) || disagreeRow.includes('\\|'))
    }
    assert.ok(!md.includes('`code`'), 'a raw backtick pair from shard content must never survive into rendered Markdown')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('renderMarkdownSummary: structural findings are rendered ABOVE the AGREE/DISAGREE headline so a clean disagree count cannot look like an all-clear', () => {
  const { root, summary } = summaryWithFindings()
  try {
    const md = renderMarkdownSummary(summary)
    const structuralIdx = md.indexOf('structural/operational finding')
    // Search for the rendered headline sentence itself (not the scope_note's own passing mention
    // of the word "AGREE") so this assertion is not a false negative/positive on prose wording.
    const headlineIdx = md.indexOf('capabilities checked across')
    assert.ok(structuralIdx >= 0)
    assert.ok(headlineIdx >= 0)
    assert.ok(structuralIdx < headlineIdx)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('renderMarkdownSummary: bounded list rendering discloses a remaining count instead of a silent cap', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-report-bound-')
  try {
    const rows = Array.from({ length: 45 }, (_, i) => cap(`fixture.many_${i}`, 'real_mod', 'unimplemented'))
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    const crates = discoverWorkspaceCrates(root)
    const summary = verifyCrates(root, crates.map((c) => c.name))
    assert.equal(summary.disagree_count, 45)
    const md = renderMarkdownSummary(summary)
    assert.match(md, /more \(not shown\)/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('renderMarkdownSummary and renderHumanSummary: deterministic output for identical input', () => {
  const { root, summary } = summaryWithFindings()
  try {
    assert.equal(renderMarkdownSummary(summary), renderMarkdownSummary(summary))
    assert.deepEqual(renderHumanSummary(summary), renderHumanSummary(summary))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── SOURCE_FILE_OVERSIZED is rendered (it is not visible via crates_skipped/workspace_findings) ──

function summaryWithOversizedSourceFile() {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-report-oversized-')
  writeFileSync(join(cratePath, 'src', 'huge.rs'), 'X'.repeat(3 * 1024 * 1024))
  writeFileSync(join(cratePath, 'src', 'lib.rs'), 'pub mod real_mod;\nmod huge;\n\npub fn dispatch() {\n    real_mod::run();\n}\n')
  const rows = [cap('fixture.real', 'real_mod', 'unimplemented'), cap('fixture.huge', 'huge', 'unimplemented')]
  writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
  const crates = discoverWorkspaceCrates(root)
  const summary = verifyCrates(root, crates.map((c) => c.name))
  return { root, summary }
}

test('renderHumanSummary: an oversized individual source file is disclosed even though the crate itself is not skipped', () => {
  const { root, summary } = summaryWithOversizedSourceFile()
  try {
    assert.ok(summary.operational_finding_count >= 1)
    const text = renderHumanSummary(summary).join('\n')
    assert.match(text, /oversized source file/)
    assert.match(text, /huge\.rs/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('renderMarkdownSummary: an oversized individual source file appears in the structural/operational table', () => {
  const { root, summary } = summaryWithOversizedSourceFile()
  try {
    const md = renderMarkdownSummary(summary)
    assert.match(md, /SOURCE_FILE_OVERSIZED/)
    assert.match(md, /huge\.rs/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
