#!/usr/bin/env node
// gen-numbered-docs.mjs — project docs/capabilities.db (the system of record) into the
// numbered canonical docs as EXECUTION-ROADMAP fenced blocks.
//
// TWO-SURFACE MODEL: the numbered docs (docs/NNN-*.md) and capabilities.db are the two
// synchronized tracking surfaces. This generator writes the DB-OWNED zones only —
// fenced `<!-- chronica:header NNN -->` (counts) and `<!-- chronica:caps NNN … -->`
// (the checkable capability lines). Everything OUTSIDE those fences is hand-authored
// prose (business logic, UX/UI logic, object models) and is NEVER touched.
//
// Round-trip: an agent flips a `- [ ] ⬜ spec` line to `- [x] ✅ verified` in a doc;
// sync-docs.mjs parses it back into canonical_status_override; this generator re-emits.
//
// Per-domain cap-roadmap docs are GENERATED whole on first creation (title + blurb
// scaffold), then fence-injected on every later run. The original domain docs stay
// at 030-043; later domains use generated overflow numbers. 001-029 / non-domain
// 044+ docs are hand-written prose; we ONLY inject/replace their fences. Selector
// resolution decides which capability set each doc shows; unknown docs get NO
// fence (safe default).
//
// Run: node tools/capabilities/gen-numbered-docs.mjs   (then gen-index.mjs for 000-INDEX)
import Database from 'better-sqlite3'
import { writeFileSync, existsSync, readFileSync, readdirSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, DOCS, DOCTRINES, GENERATED, BENCHMARKS } from '../_paths.mjs'
import { attachCanonicalCapabilityFallback } from './canonical-fallback.mjs'
import { numberedDocCapabilityScope, shouldCompactNumberedDocCaps } from './numbered-doc-selector.mjs'

mkdirSync(GENERATED, { recursive: true })
mkdirSync(BENCHMARKS, { recursive: true })

const db = new Database(CAPABILITIES_DB, { readonly: true })
attachCanonicalCapabilityFallback(db, { includeOverrideView: true })
const meta = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map(r => [r.k, r.v]))

// ── domain docs + display titles ──────────────────────────────────────────────
// The optional 4th tuple item pins the numbered doc. Keep the original 030-043
// stable; new/generated domains live in the overflow band so they do not collide
// with older hand-authored doc slots.
const DOMAIN_ORDER = [
  ['erp-finance', 'ERP & Finance', 'The central operating core: ledger, invoices, payments, stock valuation. Money-dense.'],
  ['commerce', 'Commerce & Storefront', 'Products, listings, orders, fulfillment, price changes. Money-gated.'],
  ['trading-markets', 'Trading & Markets', 'Brokers, quotes, portfolio, backtest. Money-dense; trade execution is board-gated.'],
  ['workflow-runtime', 'Workflow & Runtime', 'Durable execution, scheduling, triggers, the invoke_tool gate. Foundation.'],
  ['observability-analytics', 'Observability & Analytics', 'Audit log, metrics, traces, events, funnels, experiments.'],
  ['agent-knowledge', 'Agent & Knowledge', 'LLM providers, memory, RAG, knowledge graph, debate, tool-use.'],
  ['osint', 'OSINT & Research', 'Username/email/phone footprinting, breach search, recon, evidence.'],
  ['browser-internet-hand', 'Browser & Internet-Hand', 'The sanctioned TS/Playwright browser capability behind a Rust boundary.'],
  ['policy-approval-audit', 'Policy, Approval & Audit', 'The money gate, permissions, approvals, the tamper-evident audit chain.'],
  ['infra-execution', 'Infra & Execution', 'Sandboxes, containers, deploy, provisioning, health.'],
  ['media-generation', 'Media Generation', 'Image/video/audio generation, diffusion, render pipelines.'],
  ['crm-support', 'CRM & Support', 'Support inbox, conversations, contacts, tickets.'],
  ['other', 'Cross-Cutting & Unsorted', 'Auth, CLI, cache, reports, comments, and capabilities not yet domain-sorted.'],
  ['unclustered', 'Unclustered', 'Capabilities the dedupe could not cluster; named + source-cited, awaiting domain assignment.'],
  // Overflow domains pinned into the 060–062 capability band (moved out of the 150s
  // runbook band 2026-06-30 to fix the 156- collision and keep cap docs together).
  ['cognition', 'Cognition & Strategic Judgment', 'Run cognition, actor calibration, opportunity briefs, and cognition-aware money gates.', 60],
  ['erp-stock', 'ERP Stock & Item Pricing', 'Stock closing balances, item variants, alternatives, and item price records.', 61],
  ['strategy', 'Strategy & Game Theory', 'Oversight posture, calibration, actor incentives, and world-game decision support.', 62],
  ['legal-esign', 'Legal, E-Sign & Notary', 'Contracts, e-signature envelopes/recipients/fields, recipient-role derivations, evidence lifecycle. Witnessed, never a money/QES authority.', 63],
  ['vault-secrets', 'Vault & Secrets', 'Secret-reference path metadata, folder trees, version lineage, rotation schedules. Value-free — never decrypts or resolves a secret.', 64],
]
// number domains 030, 031, ... unless explicitly pinned; build the docnum->domain + reverse maps.
const DOMAIN_DOCNUM = {}, DOCNUM_DOMAIN = {}
DOMAIN_ORDER.forEach(([d, , , explicitNum], i) => {
  const n = explicitNum || (30 + i)
  DOMAIN_DOCNUM[d] = n
  DOCNUM_DOMAIN[n] = d
})

// ── slice -> domain (the 13 anchor slices that actually have linked canonical caps) ──
const sliceDomainRows = db.prepare(`SELECT slice, domain, count(*) n FROM canonical_capability
  WHERE slice IS NOT NULL GROUP BY slice, domain`).all()
const SLICE_DOMAIN = {}
for (const r of sliceDomainRows) { if (!SLICE_DOMAIN[r.slice] || r.n > SLICE_DOMAIN[r.slice].n) SLICE_DOMAIN[r.slice] = { domain: r.domain, n: r.n } }

// ── explicit doc -> slice map (the ONLY routing; conservative, hand-curated) ──
// Each value is the anchor slice whose linked canonical caps the doc shows. slice:N
// resolves to its anchor domain (the 13 populated slices). 'facet' = the doc describes
// one aspect of that slice (header notes it). A doc NOT listed gets NO fence (safe
// default) — foundation/architecture/parity/UI/dept-org/repo docs are pure narrative.
// We do NOT use the docs' "Related capability keys" rows: that namespace does not
// match canonical_capability.key (0/937 overlap), so a heuristic on it injects wrong
// blocks. Only this reviewed map (and the generated domain auto-map) routes.
const DOC_SLICE = {
  // Money / policy / approval / audit — facets of slice 4 (policy-approval-audit, 47 caps)
  6: { slice: 4, facet: true }, 44: { slice: 4, facet: true }, 45: { slice: 4, facet: true },
  46: { slice: 4, facet: true }, 47: { slice: 4, facet: true }, 48: { slice: 4, facet: true },
  49: { slice: 4, facet: true }, 50: { slice: 4, facet: true }, 91: { slice: 4, facet: true },
  123: { slice: 4, facet: true }, // osint-safety-policy-enforcement -> policy facet
  // Commerce (slice 18, 21 caps)
  64: { slice: 18 }, 73: { slice: 18, facet: true }, 95: { slice: 18, facet: true },
  // ERP (slice 19, 111 caps)
  65: { slice: 19 }, 61: { slice: 19, facet: true }, 86: { slice: 19, facet: true },
  92: { slice: 19, facet: true },
  // Observability / analytics (slice 15, 167 caps)
  66: { slice: 15 }, 112: { slice: 15 }, 113: { slice: 15, facet: true }, 114: { slice: 15, facet: true },
  115: { slice: 15, facet: true }, 116: { slice: 15, facet: true }, 117: { slice: 15, facet: true },
  118: { slice: 15, facet: true }, 119: { slice: 15, facet: true }, 120: { slice: 15, facet: true },
  121: { slice: 15, facet: true }, 122: { slice: 15, facet: true }, 87: { slice: 15, facet: true },
  // Agent & knowledge / memory / RAG (slice 16, 197 caps)
  67: { slice: 16 }, 131: { slice: 16 }, 132: { slice: 16, facet: true }, 133: { slice: 16, facet: true },
  134: { slice: 16, facet: true }, 135: { slice: 16, facet: true }, 136: { slice: 16, facet: true },
  60: { slice: 16, facet: true },
  // OSINT / research (slice 11, 217 caps)
  103: { slice: 11 }, 104: { slice: 11, facet: true }, 105: { slice: 11, facet: true },
  106: { slice: 11, facet: true }, 107: { slice: 11, facet: true }, 93: { slice: 11, facet: true },
  70: { slice: 11, facet: true },
  // Browser / internet-hand (slice 10, 52 caps). The dup-numbered collisions were resolved:
  // the OSINT-intel/money docs moved to 146-150, evidence-capture to 151, so these numbers
  // are now unique and keyed plainly.
  108: { slice: 10 },   // browser-session-lifecycle
  109: { slice: 10 },   // playwright-provider-boundary
  111: { slice: 10 },   // browser-profile-identity-lifecycle
  151: { slice: 10 },   // evidence-capture-and-reconciliation (moved from 112)
  // Workflow & runtime (slice 13, 175 caps)
  80: { slice: 13 }, 81: { slice: 13, facet: true }, 82: { slice: 13, facet: true },
  83: { slice: 13, facet: true }, 84: { slice: 13, facet: true }, 88: { slice: 13 },
  89: { slice: 13, facet: true }, 90: { slice: 13, facet: true }, 138: { slice: 13, facet: true },
  // CRM / support (slice 21, 40 caps)
  79: { slice: 21 }, 94: { slice: 21, facet: true },
  // Media generation (slice 22, 44 caps)
  124: { slice: 22 }, 137: { slice: 22 }, 102: { slice: 22, facet: true },
}

// resolve a doc to a selector (or null = no fence). Generated domain docs auto-route.
// Some keys in DOC_SLICE are FULL filenames (the dup-numbered pairs); the caller passes both.
function resolveSelector(num, fnameNoExt) {
  if (DOCNUM_DOMAIN[num]) return { kind: 'domain', domain: DOCNUM_DOMAIN[num] }
  const entry = DOC_SLICE[fnameNoExt] ?? DOC_SLICE[num]
  if (!entry) return null
  const dom = SLICE_DOMAIN[entry.slice]?.domain
  if (!dom) return null
  return { kind: 'domain', domain: dom, slices: [entry.slice], scope: entry.facet ? 'facet' : undefined, label: `slice ${entry.slice}` }
}

// ── the line emitter (omits empty segments; never prints an orphan 🧪/💰/⛔) ────
function emitLine(c) {
  const box = c.status === 'verified' ? 'x' : c.status === 'implemented_unverified' ? '~' : ' '
  const stoken = c.status === 'verified'
    ? '✅ verified'
    : c.status === 'implemented_unverified'
      ? '🟡 unverified'
      : c.status === 'excluded'
        ? '🚫 excluded'
        : '⬜ spec'
  const targetCrate = c.target_crate || '(crate tbd)'
  const target = c.target_module ? `${targetCrate}::${c.target_module}` : targetCrate
  const gate = c.moves_money ? 'money' : c.requires_approval ? 'approval' : (c.side_effect_class || 'internal-write')
  const segs = [`→ ${target}`, `· ${gate}`]
  if (c.acceptance_test) segs.push(`· 🧪 ${String(c.acceptance_test).split('::').pop()}`)
  if (c.moves_money && c.financial_control_test && !/^REQUIRED/.test(c.financial_control_test)) segs.push(`· 💰 ${String(c.financial_control_test).split('::').pop()}`)
  else if (c.moves_money) segs.push('· 💰 REQUIRED')
  if (c.blocker) segs.push(`· ⛔ ${c.blocker}`)
  return `- [${box}] ${stoken} \`${c.key}\` — ${c.canonical_name} ${segs.join(' ')}`
}

// ── per-selector header counts (honors canonical_status_override via COALESCE) ──
const headerSql = (pred) => `SELECT
  count(*) items,
  sum((CASE WHEN o.status IS NULL THEN c.status ELSE o.status END)='verified') verified,
  sum((CASE WHEN o.status IS NULL THEN c.status ELSE o.status END)='implemented_unverified') unverified,
  sum((CASE WHEN o.status IS NULL THEN c.status ELSE o.status END)='unimplemented') spec,
  sum((CASE WHEN o.status IS NULL THEN c.status ELSE o.status END)='excluded') excluded,
  sum(c.moves_money) money,
  sum(c.moves_money=1 AND COALESCE(c.financial_control_test,'')<>'' AND c.financial_control_test NOT LIKE 'REQUIRED%') dollar_tests
  FROM canonical_capability c LEFT JOIN canonical_status_override o ON o.canonical_key=c.key WHERE ${pred}`

function headerBlock(num, sel) {
  const isDomainDoc = !!DOCNUM_DOMAIN[num]
  const scope = numberedDocCapabilityScope(sel, { domainDoc: isDomainDoc })
  const h = db.prepare(headerSql(scope.pred)).get(scope.params)
  const lbl = sel.label ? ` · ${sel.label}${sel.scope === 'facet' ? ' (facet)' : ''}` : ''
  let md = `<!-- chronica:header ${num} -->\n`
  md += `> Roadmap header — projected from \`capabilities.db\`${lbl}. Regenerate: \`pnpm docs:gen\`. Write back: edit a status token + \`pnpm docs:sync\`.\n\n`
  md += `| Items | Verified | Unverified | Spec | Excluded | Money | $-tests |\n|---|---|---|---|---|---|---|\n`
  md += `| ${h.items} | ${h.verified || 0} | ${h.unverified || 0} | ${h.spec || 0} | ${h.excluded || 0} | ${h.money || 0} | ${h.dollar_tests || 0} / ${h.money || 0} |\n`
  if (sel.domain === 'erp-finance') {
    const noise = db.prepare("SELECT count(*) n FROM canonical_capability WHERE domain='erp-finance' AND key LIKE 'noise.%'").get().n
    if (noise) md += `\n> _(${noise} mis-clustered \`noise.*\` rows are excluded from these ERP totals; they appear flagged in the backlog below pending re-domain.)_\n`
  }
  md += `<!-- /chronica:header -->`
  return md
}

const capsFor = (pred, params) => db.prepare(`SELECT
    c.key,
    c.canonical_name,
    c.target_crate,
    c.target_module,
    c.side_effect_class,
    COALESCE(o.moves_money, c.moves_money) moves_money,
    COALESCE(o.requires_approval, c.requires_approval) requires_approval,
    COALESCE(o.financial_control_test, c.financial_control_test) financial_control_test,
    COALESCE(o.acceptance_test, c.acceptance_test) acceptance_test,
    COALESCE(o.status, c.status) status,
    COALESCE(o.blocker, c.blocker) blocker,
    c.donor_count,
    c.slice
  FROM canonical_capability c
  LEFT JOIN canonical_status_override o ON o.canonical_key=c.key
  WHERE ${pred}
  ORDER BY (COALESCE(o.status, c.status)='verified') DESC,
    (COALESCE(o.status, c.status)='implemented_unverified') DESC,
    COALESCE(o.moves_money, c.moves_money) DESC,
    c.donor_count DESC,
    c.canonical_name`).all(params)

// caps block(s): for domain docs, two blocks (linked + backlog); else one block.
function capsBlocks(num, sel) {
  const isDomainDoc = !!DOCNUM_DOMAIN[num]
  if (isDomainDoc) {
    const linked = capsFor('c.domain=@d AND c.slice IS NOT NULL', { d: sel.domain })
    const backlog = capsFor('c.domain=@d AND c.slice IS NULL', { d: sel.domain })
    let md = `### Slice-linked capabilities\n`
    md += `<!-- chronica:caps ${num} selector=domain:${sel.domain} scope=linked -->\n`
    md += (linked.length ? linked.map(emitLine).join('\n') : '_(none linked to a slice yet)_')
    md += `\n<!-- /chronica:caps -->\n\n`
    md += `### Unassigned — true-scope backlog\n`
    md += `> In scope for this domain but not yet mapped to a registry slice. Listed so nothing is hidden.\n`
    md += `<!-- chronica:caps ${num} selector=domain:${sel.domain} scope=backlog -->\n`
    if (!backlog.length) md += '_(no backlog — all linked)_'
    else {
      const noise = backlog.filter(c => /^noise\./.test(c.key))
      const real = backlog.filter(c => !/^noise\./.test(c.key))
      const parts = []
      if (real.length) parts.push(real.map(emitLine).join('\n'))
      if (noise.length) parts.push(`\n**⚠ Mis-clustered (NOT ${sel.domain} capabilities — pending re-domain by the linker):**\n` + noise.map(emitLine).join('\n'))
      md += parts.join('\n')
    }
    md += `\n<!-- /chronica:caps -->`
    return md
  }
  // non-domain doc: one block over the selector's caps
  const scope = numberedDocCapabilityScope(sel)
  const caps = capsFor(scope.pred, scope.params)
  let md = `<!-- chronica:caps ${num} selector=domain:${sel.domain}${sel.scope ? ' scope=' + sel.scope : ''} -->\n`
  if (shouldCompactNumberedDocCaps(caps.length)) {
    const domainNum = DOMAIN_DOCNUM[sel.domain]
    const domainFile = domainNum ? `${String(domainNum).padStart(3, '0')}-cap-${sel.domain}.md` : null
    const domainRef = domainFile ? `[${domainFile}](${domainFile})` : `the ${sel.domain} domain roadmap`
    md += `_(Projection summary: ${caps.length} capabilities match this curated selector. The full editable capability checklist lives in ${domainRef}; this non-domain document keeps only the counts above so architecture/doctrine files do not duplicate a domain-scale backlog.)_`
  } else {
    md += (caps.length ? caps.map(emitLine).join('\n') : '_(no capabilities resolved)_')
  }
  md += `\n<!-- /chronica:caps -->`
  return md
}

// ── fence regexes (match on doc number; non-greedy so two blocks match independently) ──
const headerRe = (n) => new RegExp(`<!--\\s*chronica:header\\s+${n}\\b[^>]*-->[\\s\\S]*?<!--\\s*/chronica:header\\s*-->`)
const capsReG = (n) => new RegExp(`<!--\\s*chronica:caps\\s+${n}\\b[^>]*-->[\\s\\S]*?<!--\\s*/chronica:caps\\s*-->`, 'g')

// build the full "## Execution roadmap" region (header + caps blocks)
function roadmapRegion(num, sel) {
  return `## Execution roadmap\n\n${headerBlock(num, sel)}\n\n${capsBlocks(num, sel)}`
}

// inject into an existing prose doc, idempotently, preserving everything outside fences.
function injectProse(raw, num, sel) {
  const hasHeader = headerRe(num).test(raw)
  const hasCaps = capsReG(num).test(raw)
  const region = roadmapRegion(num, sel)
  if (hasHeader || hasCaps) {
    // REPLACE the whole "## Execution roadmap …" region. A domain doc has TWO caps blocks
    // (linked + backlog) plus their "### …" sub-headings, so the region runs from the
    // "## Execution roadmap" heading through the LAST `<!-- /chronica:caps -->` in the file.
    // GREEDY [\s\S]* (single file, region is contiguous and always last) to consume both.
    const re = new RegExp(`##\\s*Execution roadmap[\\s\\S]*<!--\\s*/chronica:caps\\s*-->`)
    if (re.test(raw)) return raw.replace(re, region)
    // no "## Execution roadmap" heading (older fence) — replace from header sentinel greedily.
    const re2 = new RegExp(`<!--\\s*chronica:header\\s+${num}\\b[\\s\\S]*<!--\\s*/chronica:caps\\s*-->`)
    if (re2.test(raw)) return raw.replace(re2, region)
    return raw.replace(headerRe(num), headerBlock(num, sel)).replace(capsReG(num), capsBlocks(num, sel))
  }
  // FIRST INSERT: before "## See also"/"## Related", else after the field table, else EOF.
  const insert = `\n${region}\n`
  const seeAlso = raw.search(/\n##\s+(See also|Related)\b/i)
  if (seeAlso >= 0) return raw.slice(0, seeAlso) + '\n' + region + '\n' + raw.slice(seeAlso)
  // after the | Field | Value | table: find the related-keys row, then the next blank line
  const tbl = raw.search(/\|\s*Related capability keys\s*\|[^\n]*\|/i)
  if (tbl >= 0) {
    const after = raw.indexOf('\n', tbl)
    return raw.slice(0, after + 1) + insert + raw.slice(after + 1)
  }
  return raw.replace(/\n*$/, '\n') + insert
}

// ── main: walk every numbered doc, resolve, inject ────────────────────────────
// Domain docs and every doc this script fence-injects are machine-generated/hybrid
// content and live under docs/_generated/ (see tools/_paths.mjs); genuinely
// hand-authored prose (070-099 and everything else with no resolvable selector)
// stays in docs/ or docs/doctrines/. A doc is discovered wherever it currently lives
// so this keeps working whether or not a given file has been moved to
// docs/_generated/ or docs/doctrines/ yet.
const W = (dir, name, body) => { writeFileSync(join(dir, name), body); return name }
const listNumbered = (dir) => existsSync(dir)
  ? readdirSync(dir).filter(f => /^\d{3}-.*\.md$/.test(f) && f !== '000-INDEX.md').map(f => ({ f, dir }))
  : []
const files = [...listNumbered(DOCS), ...listNumbered(DOCTRINES), ...listNumbered(GENERATED), ...listNumbered(BENCHMARKS)]
const domainFnames = new Set(DOMAIN_ORDER.map(([d]) => `${String(DOMAIN_DOCNUM[d]).padStart(3, '0')}-cap-${d}.md`))
let injected = 0, skipped = 0, rewritten = 0

// (1) domain docs are PURE DB projections — written WHOLE each run (scaffold +
// fenced roadmap), so no stale legacy table survives. Any hand-written intro a maintainer
// adds above "## Execution roadmap" would be overwritten — by design these docs are generated.
for (const [domain, title, blurb] of DOMAIN_ORDER) {
  const num = DOMAIN_DOCNUM[domain]
  const fname = `${String(num).padStart(3, '0')}-cap-${domain}.md`
  const sel = { kind: 'domain', domain }
  const pad = String(num).padStart(3, '0')
  const body = `# ${pad} — Capability Roadmap: ${title}\n\n` +
    `> Generated from \`docs/capabilities.db\` (system of record) — this whole doc is a DB projection. ${blurb} Index: [000-INDEX.md](000-INDEX.md). Regenerate: \`pnpm docs:gen\`. Write back a status edit: \`pnpm docs:sync\`.\n\n` +
    `| Field | Value |\n|---|---|\n| Domain | ${domain} |\n| Tracking surface | this doc ⇄ \`canonical_capability WHERE domain='${domain}'\` |\n| File-level donor gate | \`${meta.file_census_unread_pending || 'unknown'}\` review-pending rows globally; query \`pnpm caps:query file-census-gaps\` |\n\n` +
    `A canonical capability row is not literal donor-file exhaustion. Marking this roadmap complete also requires every relevant \`donor_file_census\` source/behavior row to be reviewed and mapped, excluded with evidence, classified as support/artifact, or blocked with evidence.\n\n` +
    `${roadmapRegion(num, sel)}\n`
  if (!existsSync(join(GENERATED, fname)) || readFileSync(join(GENERATED, fname), 'utf8') !== body) { W(GENERATED, fname, body); rewritten++ }
}

// (2) hand-written prose docs (001-029, 044-145): inject/replace ONLY the fenced region,
// preserving every prose line outside the sentinels.
for (const { f, dir } of files) {
  if (domainFnames.has(f)) continue   // handled above
  const num = +f.slice(0, 3)
  // 070–099 = the narrative Architecture & System Map + Operating Model bands (pure prose, no DB
  // roadmap). Never fence-inject them, even if a stale DOC_SLICE entry collides with the number.
  if (num >= 70 && num <= 99) { skipped++; continue }
  const fnameNoExt = f.replace(/\.md$/, '')
  const path = join(dir, f)
  const raw = readFileSync(path, 'utf8')
  const sel = resolveSelector(num, fnameNoExt)
  if (!sel) { skipped++; continue }
  const out = injectProse(raw, num, sel)
  // written back to wherever it was found — this generator only edits fenced content
  // in place, it never relocates files (the docs/_generated/ move is a separate,
  // explicit migration step).
  if (out !== raw) { W(dir, f, out); injected++ }
}

console.log(`gen-numbered-docs: ${rewritten} domain docs rewritten, ${injected} prose docs fence-injected, ${skipped} skipped (pure narrative).`)
db.close()
