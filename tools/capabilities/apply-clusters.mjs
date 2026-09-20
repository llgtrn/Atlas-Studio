#!/usr/bin/env node
// apply-clusters.mjs — rebuild canonical_capability from semantic-dedupe agent clusters.
// Reads the semantic-dedupe workflow journal, applies canonical_clusters (key+name+source_ids).
// Usage: node tools/capabilities/apply-clusters.mjs <workflow_run_dir>
import Database from 'better-sqlite3'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

const [dir] = process.argv.slice(2)
const journal = readFileSync(join(dir, 'journal.jsonl'), 'utf8')
const domainResults = []
for (const line of journal.split('\n')) {
  if (!line.trim()) continue
  let e; try { e = JSON.parse(line) } catch { continue }
  if (e.type === 'result' && e.result && Array.isArray(e.result.canonical_clusters)) domainResults.push(e.result)
}

const db = new Database(join(process.cwd(), 'docs', 'capabilities.db'))
db.pragma('journal_mode = WAL')
db.exec('DELETE FROM canonical_capability; DELETE FROM provenance; UPDATE source_capability SET canonical_id=NULL;')

const srcMeta = new Map(db.prepare('SELECT id, donor, target_crate, moves_money, requires_approval FROM source_capability').all().map(r => [r.id, r]))

const insCanon = db.prepare(`INSERT INTO canonical_capability
 (key,canonical_name,domain,target_crate,side_effect_class,moves_money,requires_approval,acceptance_criteria,financial_control_test,status,donor_count)
 VALUES (@key,@canonical_name,@domain,@target_crate,@side_effect_class,@moves_money,@requires_approval,@acceptance_criteria,@financial_control_test,@status,@donor_count)`)
const insProv = db.prepare('INSERT OR IGNORE INTO provenance (source_id,canonical_id) VALUES (?,?)')
const setCanon = db.prepare('UPDATE source_capability SET canonical_id=? WHERE id=?')

let canonN = 0, mappedSrc = new Set(), dupKey = new Map()
const tx = db.transaction(() => {
  for (const dr of domainResults) {
    const domain = dr.domain
    for (const cl of dr.canonical_clusters) {
      const ids = (cl.source_ids || []).filter(id => srcMeta.has(id))
      if (!ids.length) continue
      // money if any member moves money (cross-check source truth, not just agent flag)
      const money = cl.moves_money || ids.some(id => srcMeta.get(id).moves_money) ? 1 : 0
      const approval = ids.some(id => srcMeta.get(id).requires_approval) ? 1 : 0
      const crates = new Set(ids.map(id => srcMeta.get(id).target_crate).filter(Boolean))
      const donors = new Set(ids.map(id => srcMeta.get(id).donor))
      // dedupe key collision across domains -> suffix
      let key = cl.canonical_key || (domain + '.' + (cl.canonical_name || 'cap').toLowerCase().replace(/[^a-z0-9]+/g, '_').slice(0, 40))
      if (dupKey.has(key)) { dupKey.set(key, dupKey.get(key) + 1); key = key + '__' + dupKey.get(key) } else dupKey.set(key, 1)
      const info = insCanon.run({
        key, canonical_name: cl.canonical_name || key, domain,
        target_crate: [...crates][0] || '',
        side_effect_class: money ? 'money' : '',
        moves_money: money, requires_approval: approval,
        acceptance_criteria: money
          ? `Test proves: ${cl.canonical_name} runs policy -> board approval -> CostRecord -> audit chain; blocked path leaves no side effect; agent cannot self-approve.`
          : `Test proves: ${cl.canonical_name} executes its documented behavior natively in Rust and returns expected output for representative inputs.`,
        financial_control_test: money ? 'REQUIRED (not yet written)' : null,
        status: 'unimplemented',
        donor_count: donors.size,
      })
      const cid = info.lastInsertRowid
      for (const id of ids) { insProv.run(id, cid); setCanon.run(cid, id); mappedSrc.add(id) }
      canonN++
    }
  }
})
tx()

// Any source rows the agents didn't cluster -> create a 1:1 canonical (so nothing is lost / unmapped=0).
const unmapped = db.prepare('SELECT id, canonical_name, donor, target_crate, moves_money, requires_approval FROM source_capability WHERE canonical_id IS NULL').all()
const tx2 = db.transaction(() => {
  for (const r of unmapped) {
    const money = r.moves_money ? 1 : 0
    let key = 'unclustered.' + (r.canonical_name || 'cap').toLowerCase().replace(/[^a-z0-9]+/g, '_').slice(0, 40) + '_' + r.id
    const info = insCanon.run({
      key, canonical_name: r.canonical_name, domain: 'unclustered', target_crate: r.target_crate || '',
      side_effect_class: money ? 'money' : '', moves_money: money, requires_approval: r.requires_approval ? 1 : 0,
      acceptance_criteria: money
        ? `Test proves: ${r.canonical_name} runs policy -> board approval -> CostRecord -> audit chain; no self-approval.`
        : `Test proves: ${r.canonical_name} executes its documented behavior natively and returns expected output.`,
      financial_control_test: money ? 'REQUIRED (not yet written)' : null,
      status: 'unimplemented', donor_count: 1,
    })
    insProv.run(r.id, info.lastInsertRowid); setCanon.run(info.lastInsertRowid, r.id)
  }
})
tx2()

const src = db.prepare('SELECT count(*) n FROM source_capability').get().n
const canon = db.prepare('SELECT count(*) n FROM canonical_capability').get().n
const money = db.prepare('SELECT count(*) n FROM canonical_capability WHERE moves_money=1').get().n
const orphan = db.prepare('SELECT count(*) n FROM source_capability WHERE canonical_id IS NULL').get().n
const byDomain = db.prepare('SELECT domain, count(*) n FROM canonical_capability GROUP BY domain ORDER BY n DESC').all()
console.log(`APPLIED semantic clusters:`)
console.log(`  source: ${src} -> canonical: ${canon} (${(100 * (1 - canon / src)).toFixed(0)}% collapse), money canonical: ${money}, orphans: ${orphan}`)
console.log(`  clustered canonicals: ${canonN} | 1:1 unclustered fallbacks: ${unmapped.length}`)
console.log('  by domain:', JSON.stringify(byDomain))
db.close()
