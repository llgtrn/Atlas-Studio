#!/usr/bin/env node
// gen-index.mjs — regenerate docs/000-INDEX.md by enumerating every numbered canonical doc + its H1
// title, grouped into the canonical band scheme. Leads with the CURRENT TRUTH (the spec-vs-reality
// audit + the roadmap + the spec STATUS), then the capability-census numbers (DB projections).
//
// Canonical scheme (2026-06-30 refactor): one uniform 3-digit banded tree. The numbering bands below
// are the single source of the taxonomy; docs are auto-discovered by filename number. Hand-authored
// docs (doctrine, invariants, runbooks, crate atlas) and generated docs (capability roadmaps, strategic
// brain, truth/reality) share one tree. Legacy reset-history + the old index live in docs/_archive/.
import Database from 'better-sqlite3'
import { existsSync, readdirSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { attachCanonicalCapabilityFallback } from './canonical-fallback.mjs'
import { DOCS, DOCTRINES, GENERATED, CRATE_DOCS, BENCHMARKS } from '../_paths.mjs'

mkdirSync(GENERATED, { recursive: true })
mkdirSync(CRATE_DOCS, { recursive: true })
mkdirSync(BENCHMARKS, { recursive: true })
const db = new Database(join(DOCS, 'capabilities.db'), { readonly: true })
attachCanonicalCapabilityFallback(db)
const meta = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map(r => [r.k, r.v]))
const verifiedCanon = db.prepare("SELECT count(*) n FROM canonical_capability WHERE status='verified'").get().n
const verifiedSlices = db.prepare("SELECT count(*) n FROM canonical_capability WHERE key LIKE 'slice.%' AND status='verified'").get().n

// 000-INDEX.md itself lives in docs/_generated/ (it's a DB projection) alongside the
// other generated/hybrid numbered docs, alongside the crate atlas docs (300-399) in their
// own docs/crates/ folder; hand-authored numbered docs live in docs/ or, increasingly,
// docs/doctrines/ (the doctrine/invariant/operating "why/what" corpus consolidated out of
// docs/ root — see docs/doctrines/README.md); benchmark methodology/doctrine docs (hybrid
// or hand-authored) live in docs/benchmarks/ (see docs/benchmarks/README.md). Scan all five
// so the index still covers the whole numbered tree regardless of which location a given
// doc currently lives in.
const listNumbered = (dir) => existsSync(dir) ? readdirSync(dir).filter(f => /^\d{3}-.*\.md$/.test(f) && f !== '000-INDEX.md').map(f => ({ f, dir })) : []
const files = [...listNumbered(DOCS), ...listNumbered(DOCTRINES), ...listNumbered(GENERATED), ...listNumbered(CRATE_DOCS), ...listNumbered(BENCHMARKS)].sort((a, b) => a.f.localeCompare(b.f))
function title(f, dir) {
  const first = readFileSync(join(dir, f), 'utf8').split('\n').find(l => /^#\s/.test(l)) || ''
  return first.replace(/^#\s+\d+\s*[—-]\s*/, '').replace(/^#\s+/, '').trim()
}
// href relative to docs/_generated/000-INDEX.md: same-dir (docs/_generated/) files need no
// prefix; docs/-root files need to go up one level (../); docs/crates/, docs/doctrines/, and
// docs/benchmarks/ are GENERATED-sibling directories, so each needs to go up one level then
// back down.
const href = (f, dir) => dir === GENERATED ? f : dir === CRATE_DOCS ? `../crates/${f}` : dir === DOCTRINES ? `../doctrines/${f}` : dir === BENCHMARKS ? `../benchmarks/${f}` : `../${f}`
const rows = files.map(({ f, dir }) => ({ f, dir, num: +f.slice(0, 3), title: title(f, dir) }))

// ── the canonical band taxonomy (single source of the scheme) ─────────────────
const BANDS = [
  [1, 9, 'Doctrine — the operating philosophy'],
  [10, 19, 'Invariants & Money-Safety — the must-hold'],
  [20, 23, 'The Right Path — roadmap & current truth'],
  [24, 29, 'Strategic Brain (generated from capabilities.db)'],
  [30, 43, 'Capability Roadmaps — core domains (generated)'],
  [44, 49, 'Company Brain — assumptions · decisions · value · market · debt · evolution (generated)'],
  [50, 53, 'Truth & Reality Engine (generated)'],
  [60, 69, 'Capability Roadmaps — overflow domains (generated)'],
  [70, 89, 'Architecture & System Map — the as-built subsystems (reflects the engineering specs)'],
  [90, 99, 'Operating Model — how the Brain plans & the Cloud builds (the build loop)'],
  [150, 159, 'Operations Runbooks'],
  [300, 399, 'Crate Atlas — one per crate'],
]

let md = `# 000 — Chronica Canonical Docs Index\n\n`
md += `> **THE ENTRY POINT.** Chronica is a money-safe, game-theoretic **Company-OS / 8-Layer Business Network** — a Rust kernel (\`chronica-*\` crates) + a Paperclip-native React UI + one sanctioned browser boundary. Money is **Layer 7**, gated; the moat is **Layer 8** cross-account knowledge + the cross-company trust graph. Docs are one uniform 3-digit canonical tree (banded below); legacy reset-history is in \`_archive/\`.\n\n`

// ── CURRENT TRUTH (the right path) — read these first ─────────────────────────
md += `## ⬇️ Current ground truth — read these first\n`
md += `These three are the live source of truth; they supersede any census number below.\n`
md += `- 🔍 **Audit:** [doctrines/audit/spec-vs-reality-audit-2026-06-30.md](../doctrines/audit/spec-vs-reality-audit-2026-06-30.md) — every spec → 5-state, all 9 money/security invariants verified first-hand, what is real vs. paper.\n`
md += `- 🗺️ **Roadmap:** [doctrines/020-roadmap.md](../doctrines/020-roadmap.md) — the right path forward (build order by leverage × money-safety).\n`
md += `- 📁 **Spec reality-index:** [doctrines/specs/STATUS.md](../doctrines/specs/STATUS.md) — the 35-doc spec corpus filed by \`built/\` · \`partial/\` · \`backlog/\` · \`process/\`.\n\n`

md += `## The honest numbers (capability census)\n`
md += `> These are \`capabilities.db\` **projections** — a different, broader denominator than the spec-vs-reality audit. Where they disagree, the audit is the truth for "does the code exist and is it money-safe."\n\n`
md += `| Metric | Value |\n|---|---|\n`
md += `| Literal source capabilities (donors) | **${meta.source_capability_count}** |\n| Canonical Chronica capabilities | **${meta.canonical_capability_count}** |\n| Money-moving canonical | **${meta.money_canonical_count}** |\n| Verified (code+test) | **${verifiedCanon}** canonical + ${verifiedSlices} registry slices |\n| This numbered tree | **${files.length}** canonical docs |\n`
if (meta.file_census_rows) {
  md += `| File-level donor census rows | **${meta.file_census_rows}** |\n`
  md += `| File-level rows still review-pending | **${meta.file_census_unread_pending || 'unknown'}** |\n`
}
md += `\n`

md += `## How an agent uses this tree (the two-surface round-trip)\n`
md += `The numbered docs AND \`docs/capabilities.db\` are the two synchronized tracking surfaces. Each doc's hand-written prose is doc-owned; its \`## Execution roadmap\` fenced block is DB-owned and round-trips.\n`
md += `1. Pick a capability-roadmap doc (the \`030–043\` / \`060–062\` \`cap-*.md\` domain docs).\n`
md += `2. \`pnpm caps:query next-unverified-canonical\` — the build queue; \`pnpm caps:query get-canonical <key>\` — one capability's full spec.\n`
md += `3. Build it natively in the target \`chronica-*\` crate with a real test (money caps need a financial-control test).\n`
md += `4. Flip a doc line \`- [ ] ⬜ spec\` → \`- [x] ✅ verified\`; then \`pnpm docs:sync\` writes it back to the DB, \`pnpm docs:gen\` re-projects, \`pnpm docs:verify\` confirms both sides agree.\n\n`

md += `## The canonical tree\n`
for (const [lo, hi, name] of BANDS) {
  const inBand = rows.filter(r => r.num >= lo && r.num <= hi)
  if (!inBand.length) continue
  md += `\n### ${String(lo).padStart(3, '0')}–${String(hi).padStart(3, '0')} · ${name}\n`
  for (const r of inBand) md += `- [${r.f}](${href(r.f, r.dir)}) — ${r.title}\n`
}

md += `\n## Archive & machine files\n`
md += `- \`_archive/\` — retired reset-history (\`90/91/99-*\`) and the superseded \`xx0-INDEX\`. Kept out of the canonical tree.\n`
md += `- \`_machine/\` — machine tracking inputs the tooling reads (NOT a human surface): the parity registry, the slice plan, donor inventory/absorption/retirement maps. Paths centralized in \`tools/_paths.mjs\`.\n`
md += `- \`capabilities.db\` — the capability system of record; this index + the per-domain cap docs are projections of it. The \`specs/\` folder holds the engineering-spec corpus (filed built/partial/backlog/process — see \`specs/STATUS.md\`).\n`
writeFileSync(join(GENERATED, '000-INDEX.md'), md)
console.log(`regenerated 000-INDEX.md: ${files.length} numbered docs across ${BANDS.filter(([lo, hi]) => rows.some(r => r.num >= lo && r.num <= hi)).length} bands`)
db.close()
