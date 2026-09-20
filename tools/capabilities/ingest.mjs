#!/usr/bin/env node
// ingest.mjs — read a census-extraction workflow journal, insert NAMED+SOURCED capability rows
// into source_capability, and record census_coverage. Rejects rows missing source_files (no stubs).
// Usage: node tools/capabilities/ingest.mjs <workflow_run_dir> <pass_label>
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

const [journalDir, passLabel = 'wave'] = process.argv.slice(2)
if (!journalDir) { console.error('usage: ingest.mjs <workflow_run_dir> <pass_label>'); process.exit(2) }

const journal = readFileSync(join(journalDir, 'journal.jsonl'), 'utf8')
const extractions = []
for (const line of journal.split('\n')) {
  if (!line.trim()) continue
  let e
  try { e = JSON.parse(line) } catch { continue }
  if (e.type !== 'result') continue
  const r = e.result
  if (r && Array.isArray(r.capabilities)) extractions.push(r)
}

const db = new Database(join(process.cwd(), 'docs', 'capabilities.db'))
db.pragma('journal_mode = WAL')
const insCap = db.prepare(`INSERT INTO source_capability
 (donor,module,canonical_name,source_files,source_symbols,business_behavior,technical_behavior,inputs,outputs,persistence,surface,side_effect_class,moves_money,requires_approval,external_services,target_crate,created_pass)
 VALUES (@donor,@module,@canonical_name,@source_files,@source_symbols,@business_behavior,@technical_behavior,@inputs,@outputs,@persistence,@surface,@side_effect_class,@moves_money,@requires_approval,@external_services,@target_crate,@created_pass)`)
const insCov = db.prepare(`INSERT OR REPLACE INTO census_coverage (donor,module,status,capabilities,coverage_note,blocker) VALUES (?,?,?,?,?,?)`)

let inserted = 0, rejected = 0
const tx = db.transaction(() => {
  for (const ex of extractions) {
    const donor = String(ex.donor || '').replace(/\\/g, '/').replace(/.*\/Temporary\//, '').split('/')[0] || ex.dir || 'unknown'
    const module = ex.module || ex.mod || 'WHOLE'
    let n = 0
    for (const c of ex.capabilities) {
      if (!c.canonical_name || !c.source_files || !String(c.source_files).trim()) { rejected++; continue } // NO stubs
      insCap.run({
        donor, module,
        canonical_name: String(c.canonical_name).trim(),
        source_files: String(c.source_files).trim(),
        source_symbols: c.source_symbols || '',
        business_behavior: c.business_behavior || '',
        technical_behavior: c.technical_behavior || '',
        inputs: c.inputs || '',
        outputs: c.outputs || '',
        persistence: c.persistence || '',
        surface: c.surface || '',
        side_effect_class: c.side_effect_class || '',
        moves_money: c.moves_money ? 1 : 0,
        requires_approval: c.requires_approval ? 1 : 0,
        external_services: c.external_services || '',
        target_crate: c.target_crate || '',
        created_pass: passLabel,
      })
      n++; inserted++
    }
    insCov.run(donor, module, ex.blocked ? 'blocked' : 'extracted', n, ex.coverageNote || '', ex.blocker || '')
  }
})
tx()

const total = db.prepare('SELECT count(*) n FROM source_capability').get().n
const donors = db.prepare('SELECT count(DISTINCT donor) n FROM source_capability').get().n
const money = db.prepare('SELECT count(*) n FROM source_capability WHERE moves_money=1').get().n
console.log(`ingested pass=${passLabel}: +${inserted} rows (${rejected} rejected for missing source/name)`)
console.log(`  source_capability total: ${total} across ${donors} donors; money rows: ${money}`)
console.log(`  coverage rows: ${db.prepare('SELECT count(*) n FROM census_coverage').get().n}`)
db.close()
