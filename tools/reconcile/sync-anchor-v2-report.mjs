// sync-anchor-v2-report.mjs — pure rendering of a verifyCrates() summary into human-readable
// text and a bounded, escaped Markdown job-summary block for CI.
//
// Every dynamic value that reaches Markdown output is passed through escapeMarkdownCell (escapes
// `|`/backtick/newline and bounds length) so a crafted capability_key, shard status, or other
// shard-derived string cannot break a table's structure or inject arbitrary Markdown/formatting
// into a CI-rendered surface. Every list is rendered with an explicit, disclosed cap (never a
// silent truncation) via BOUNDS.MAX_RENDERED_LIST_ITEMS.
import { BOUNDS, escapeMarkdownCell } from './sync-anchor-v2-shard.mjs'

function capped(list, limit = BOUNDS.MAX_RENDERED_LIST_ITEMS) {
  return { shown: list.slice(0, limit), remaining: Math.max(0, list.length - limit) }
}

export function renderHumanSummary(summary) {
  const lines = []
  lines.push(`sync-anchor-v2 atom parser: ${summary.scope_note}`)
  lines.push(
    `  ${summary.capabilities_checked} capabilities checked across ${summary.crates_checked} crate(s) ` +
      `· ${summary.agree_count} AGREE · ${summary.disagree_count} DISAGREE`,
  )
  if (summary.crates_skipped.length) {
    lines.push(`  skipped ${summary.crates_skipped.length} crate(s) (no shard / not a workspace member / oversized):`)
    const { shown, remaining } = capped(summary.crates_skipped, 10)
    for (const s of shown) lines.push(`    ${s.crate}: ${s.reason}`)
    if (remaining) lines.push(`    …and ${remaining} more`)
  }
  if (summary.workspace_finding_count) {
    lines.push(`  ${summary.workspace_finding_count} workspace-level finding(s) (never silently ignored):`)
    for (const f of summary.workspace_findings) lines.push(`    ${f.reason}: ${f.member}`)
  }
  if (summary.shard_finding_count) {
    lines.push(`  ${summary.shard_finding_count} shard structural finding(s) (malformed/invalid/duplicate — never silently ignored):`)
    const { shown, remaining } = capped(summary.shard_findings)
    for (const f of shown) {
      const loc = f.line != null ? `:${f.line}` : ''
      const extra = f.capability_key ? ` capability_key=${f.capability_key}` : f.detail ? ` (${f.detail})` : ''
      lines.push(`    ${f.crate} ${f.shard_path}${loc} -> ${f.reason}${extra}`)
    }
    if (remaining) lines.push(`    …and ${remaining} more`)
  }
  // Oversized-individual-source-file findings don't show up in crates_skipped (the crate itself
  // stays applicable) or workspace_findings, so they get their own line here -- otherwise a human
  // reading only the text summary would never learn a file's content was refused, only that the
  // process exited nonzero.
  const oversizedSourceFindings = summary.operational_findings.filter((f) => f.reason === 'SOURCE_FILE_OVERSIZED')
  if (oversizedSourceFindings.length) {
    lines.push(`  ${oversizedSourceFindings.length} oversized source file(s) refused (content never read, never silently ignored):`)
    const { shown, remaining } = capped(oversizedSourceFindings)
    for (const f of shown) lines.push(`    ${f.crate} ${f.path} (${f.size_bytes} bytes > ${f.limit_bytes}-byte limit)`)
    if (remaining) lines.push(`    …and ${remaining} more`)
  }
  if (summary.disagree_count) {
    lines.push('  DISAGREE by reason:')
    for (const [reason, count] of Object.entries(summary.disagree_by_reason)) lines.push(`    ${reason}: ${count}`)
    lines.push('  DISAGREE detail (first ' + BOUNDS.MAX_RENDERED_LIST_ITEMS + '):')
    const { shown, remaining } = capped(summary.disagreements)
    for (const d of shown) {
      lines.push(
        `    ${d.crate}/${d.capability_key} [shard=${d.shard_status}] -> ${d.reason} ` +
          `(match=${d.evidence.match_confidence ?? 'n/a'})`,
      )
    }
    if (remaining) lines.push(`    …and ${remaining} more`)
  } else {
    lines.push('  no per-capability disagreements found in this scope.')
  }
  return lines
}

/** Renders a bounded, Markdown-escaped job-summary block. Structural findings (shard/workspace)
 * are always rendered ABOVE the AGREE/DISAGREE headline specifically so "0 DISAGREE" can never
 * read as an all-clear when a malformed/invalid/duplicate record is hiding in a different bucket
 * (repair-round-2 item 4's CI truth requirement). */
export function renderMarkdownSummary(summary) {
  const lines = [
    '### sync-anchor-v2 atom parser (report-only, non-blocking)',
    '',
    summary.scope_note,
    '',
  ]

  // Every non-DATA finding (shard structural content, a workspace-path escape, a truncated/
  // oversized/unreadable shard, a truncated per-crate file listing, or an oversized individual
  // source file) is rendered in ONE table, above the AGREE/DISAGREE headline, so a clean
  // disagree_count can never read as an all-clear when any of these is nonzero.
  const otherOperationalFindings = summary.operational_findings.filter(
    (f) => f.reason !== 'WORKSPACE_MEMBER_PATH_ESCAPES_ROOT',
  )
  const structuralCount = summary.shard_finding_count + summary.workspace_finding_count + otherOperationalFindings.length
  if (structuralCount > 0) {
    lines.push(
      `**⚠ ${structuralCount} structural/operational finding(s)** (malformed JSON, invalid record, duplicate capability key, workspace-path escape, or a refused oversized/truncated/unreadable input) — these are separate from AGREE/DISAGREE and are never hidden by a clean disagree count.`,
      '',
    )
    lines.push('| crate | location | reason |', '| --- | --- | --- |')
    const rows = [
      ...summary.workspace_findings.map((f) => ({ crate: '(workspace)', loc: f.member, reason: f.reason })),
      ...summary.shard_findings.map((f) => ({ crate: f.crate, loc: `${f.shard_path}${f.line != null ? `:${f.line}` : ''}`, reason: f.reason })),
      ...otherOperationalFindings.map((f) => ({ crate: f.crate, loc: f.path ?? f.shard_path ?? '(crate-level)', reason: f.reason })),
    ]
    const { shown, remaining } = capped(rows)
    for (const f of shown) {
      lines.push(`| ${escapeMarkdownCell(f.crate)} | ${escapeMarkdownCell(f.loc)} | \`${escapeMarkdownCell(f.reason)}\` |`)
    }
    if (remaining) lines.push(`| … | ${remaining} more (not shown) | |`)
    lines.push('')
  }

  lines.push(
    `**${summary.capabilities_checked}** capabilities checked across **${summary.crates_checked}** crate(s) ` +
      `-> **${summary.agree_count}** AGREE, **${summary.disagree_count}** DISAGREE.`,
  )

  if (summary.disagree_count > 0) {
    lines.push('', '| reason | count |', '| --- | --- |')
    for (const [reason, count] of Object.entries(summary.disagree_by_reason)) {
      lines.push(`| \`${escapeMarkdownCell(reason)}\` | ${count} |`)
    }
    lines.push(
      '',
      '<details><summary>Disagreements (bounded)</summary>',
      '',
      '| crate | capability | shard status | reason | match confidence |',
      '| --- | --- | --- | --- | --- |',
    )
    const { shown, remaining } = capped(summary.disagreements, 30)
    for (const d of shown) {
      lines.push(
        `| ${escapeMarkdownCell(d.crate)} | \`${escapeMarkdownCell(d.capability_key)}\` | ${escapeMarkdownCell(d.shard_status)} | ` +
          `${escapeMarkdownCell(d.reason)} | ${escapeMarkdownCell(d.evidence?.match_confidence ?? 'n/a')} |`,
      )
    }
    if (remaining) lines.push(`| … | ${remaining} more (not shown) | | | |`)
    lines.push('', '</details>')
  }

  lines.push(
    '',
    '_A per-capability DISAGREE or a shard-content structural finding is report-only under --report ' +
      '(a mechanical Tier-1/2/3 checker only; see docs/023 for scope). An OPERATIONAL failure -- a ' +
      'workspace/manifest/source/doc path escaping the repo root, an oversized/unreadable/truncated ' +
      'input, or an unresolvable diff base -- is NEVER suppressed by --report and still fails the build._',
  )
  return lines.join('\n') + '\n'
}
