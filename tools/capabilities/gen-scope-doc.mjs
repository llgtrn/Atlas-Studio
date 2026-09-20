import Database from 'better-sqlite3'
import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, TRUE_SCOPE_DOC } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB, { readonly: true })
const m = Object.fromEntries(db.prepare('SELECT k,v FROM meta').all().map(r => [r.k, r.v]))
const donors = db.prepare('SELECT donor,true_capabilities,priority FROM donor_scope ORDER BY true_capabilities DESC').all()
const totals = db.prepare("SELECT count(*) n, sum(CASE WHEN status='verified' THEN 1 ELSE 0 END) v FROM capability").get()

let md = `# Chronica TRUE Ideal Scope — Donor-Grounded Capability Denominator

> Rebuilt **2026-06-02** by deep-reading all 74 donor repos (real source, not READMEs) and enumerating each donor's capability surface with a **consistent strict rubric** (a capability = a distinct verb-noun ACTION, NOT a model/field/endpoint/provider-variant). This corrects the prior roadmap, which was extrapolated from an ~11-file-per-donor skim and was **undersized by ~30×**.

> **System of record:** \`docs/capabilities.db\` (SQLite). This doc is a generated PROJECTION over it. Query the store directly:
> \`node tools/capabilities/query.mjs scope | next-money | by-crate <crate> | get <key> | donors\`

## The honest numbers

| Metric | Value |
|---|---|
| **TRUE ideal capability denominator** | **${m.true_ideal_denominator}** capabilities (74 donors, consistent strict granularity) |
| Current roadmap (slices) | 195 = **${m.roadmap_pct_of_ideal}%** of the true ideal |
| Verified today | 37 = **${m.verified_pct_of_ideal}%** of the true ideal |
| Addressable capability rows in the DB | ${totals.n} (195 slices + ${totals.n - 195} donor-capability stubs); ${totals.v} verified |

**Blunt answer to "is the roadmap 100% or a small part?": it is ~3.2% of the true ideal.** The 195-slice plan covers a small fraction of what absorbing all 74 donors actually requires. The earlier "19% verified" was 19% of that 3.2% target — so true absorption is **~0.6%** of the real scope.

## Method + integrity (why this number is trustworthy)

- **Pass 1:** 74 donors deep-read, one agent each — first-pass enumeration.
- **Pass 2:** the 40 "giant" donors (≥80 first-pass capabilities) re-read **exhaustively to convergence** (full module tree).
- **Pass 3+4:** ALL 40 giants re-counted with a **strict, consistent rubric** after detecting granularity inflation. Example: erpnext first returned **6,915** (it counted ~600 DocTypes/models); under the strict "verb-noun action" rubric it normalized to **278** real capabilities. This is why the final **6,102** is defensible and not the inflated ~17K an inconsistent sum would have produced.
- **Non-giant 34 donors** (<80 capabilities each) use first-pass counts — low inflation risk, bounded impact.

## True capability scope per donor (top 25)

| Donor | True capabilities | Priority |
|---|---|---|
`
for (const d of donors.slice(0, 25)) md += `| ${d.donor} | ${d.true_capabilities} | ${d.priority || ''} |\n`
md += `
_(Full 74 via \`node tools/capabilities/query.mjs donors\`.)_

## What this means for the roadmap

- **6,102** is the real target. The current 195 slices are the *spine*; the **${totals.n - 195} donor-capability stub rows** in the DB are the addressable remainder.
- Each stub carries donor + target crate + next-action ("enumerate from donor source; assign verb-noun key + contract; RED→GREEN→verify"). Expand stubs into full execution specs wave by wave.
- An agent builds by **querying the DB**, not reading prose: \`node tools/capabilities/query.mjs next-money\` returns the next buildable money capability with its blockers + contract; \`get <key>\` returns one capability's full execution spec.

## Honest caveats

- **6,102 is a capability-granularity count.** Reasonable people could group slightly differently (±~15%). What matters is that it is *consistent across donors*, so the ratio is sound.
- The donor-capability stub rows are **placeholders** — they make the true scope *addressable + countable*, but each still needs its real verb-noun key, contract, and call-chain filled in from donor source. They are honest "not-yet-specified" rows, not fabricated capabilities.
- Non-giant donor counts are first-pass (not strict-normalized); a future pass could tighten them.
- The store is the source of truth; if these numbers and the DB disagree, the DB wins (regenerate this doc with \`node tools/capabilities/gen-scope-doc.mjs\`).
`
writeFileSync(TRUE_SCOPE_DOC, md)
db.close()
console.log('wrote ' + TRUE_SCOPE_DOC + ' (' + md.length + ' bytes)')
