#!/usr/bin/env node
// build-db.mjs — build docs/capabilities.db, the queryable capability EXECUTION-SPEC store.
// System of record for Chronica's true capability scope (donor-grounded). Docs are VIEWS over this.
//
// Sources (paths centralized in tools/_paths.mjs; live under docs/_machine/):
//   - docs/_machine/parity/capabilities.registry.json     (the 195 current slices + their verified test ids)
//   - .clean-counts.json                                (the 74-donor TRUE capability counts, consistent granularity)
//   - docs/_machine/donor-inventory/donor-inventory.generated.json (donor -> target crates)
//
// Run:  node tools/capabilities/build-db.mjs
import Database from 'better-sqlite3'
import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { CAPABILITIES_DB, REGISTRY, DONOR_INVENTORY } from '../_paths.mjs'

const ROOT = process.cwd()
const DB_PATH = CAPABILITIES_DB
const reg = JSON.parse(readFileSync(REGISTRY, 'utf8'))
const clean = existsSync(join(ROOT, '.clean-counts.json'))
  ? JSON.parse(readFileSync(join(ROOT, '.clean-counts.json'), 'utf8'))
  : {}
const inv = JSON.parse(readFileSync(DONOR_INVENTORY, 'utf8'))
const invArr = Array.isArray(inv) ? inv : (inv.donors || Object.values(inv).find(Array.isArray))

const db = new Database(DB_PATH)
db.pragma('journal_mode = WAL')

db.exec(`
DROP TABLE IF EXISTS capability;
DROP TABLE IF EXISTS donor_scope;
DROP TABLE IF EXISTS meta;

CREATE TABLE capability (
  id            INTEGER PRIMARY KEY,
  key           TEXT UNIQUE,            -- e.g. "erp.purchase_invoice.post" or "slice.19"
  title         TEXT,
  donor         TEXT,                   -- donor provenance
  donor_source  TEXT,                   -- donor source module/path (where the capability lives)
  target_crate  TEXT,                   -- chronica-* crate that owns/will own it
  target_module TEXT,                   -- crate module
  inputs_json   TEXT,                   -- typed input contract (JSON)
  outputs_json  TEXT,                   -- typed output contract (JSON)
  gate_class    TEXT,                   -- read_only | internal_write | artifact_write | money | side_effect
  db_objects    TEXT,                   -- tables/types read+written
  calls_json    TEXT,                   -- ordered call chain (the execution recipe)
  acceptance_test TEXT,                 -- proving test id (--test)
  fin_test      TEXT,                   -- financial-control test id (--fin) for money caps
  status        TEXT,                   -- spec | red | green | verified
  percent       INTEGER,               -- 0..100
  next_action   TEXT,                   -- exact next step to build it
  blocker       TEXT,                   -- blocker evidence
  risk_class    TEXT,
  moves_money   INTEGER,               -- 0/1
  wave          INTEGER,               -- build wave
  priority      TEXT,                   -- P0/P1/P2
  slice         INTEGER,               -- linked registry slice (if any)
  kind          TEXT                    -- 'slice' (a current roadmap slice) | 'donor_capability' (a true-scope stub)
);

CREATE INDEX idx_cap_status ON capability(status);
CREATE INDEX idx_cap_crate  ON capability(target_crate);
CREATE INDEX idx_cap_donor  ON capability(donor);
CREATE INDEX idx_cap_money  ON capability(moves_money);
CREATE INDEX idx_cap_kind   ON capability(kind);

CREATE TABLE donor_scope (
  donor         TEXT PRIMARY KEY,
  true_capabilities INTEGER,            -- the donor's TRUE capability count (strict, consistent granularity)
  target_crates TEXT,
  priority      TEXT,
  language      TEXT,
  disposition   TEXT
);

CREATE TABLE meta (k TEXT PRIMARY KEY, v TEXT);
`)

// --- meta ---
const trueDenom = Object.values(clean).reduce((a, b) => a + b, 0)
const insMeta = db.prepare('INSERT INTO meta(k,v) VALUES(?,?)')
insMeta.run('built_at', '2026-06-02')
insMeta.run('true_ideal_denominator', String(trueDenom))
insMeta.run('current_slices', '195')
insMeta.run('verified_slices', '37')
insMeta.run('roadmap_pct_of_ideal', (100 * 195 / trueDenom).toFixed(2))
insMeta.run('verified_pct_of_ideal', (100 * 37 / trueDenom).toFixed(2))
insMeta.run('note', 'true denominator = consistent verb-noun-action granularity across all 74 donors (40 giants strict-normalized). Slices are the current roadmap; donor_capability rows are the true-scope expansion stubs.')

// --- donor_scope (74 donors, true counts) ---
const dmap = {}
for (const d of invArr) {
  dmap[d.donorRepo] = d
}
const insDonor = db.prepare('INSERT OR REPLACE INTO donor_scope(donor,true_capabilities,target_crates,priority,language,disposition) VALUES(?,?,?,?,?,?)')
const donorTx = db.transaction(() => {
  for (const [donor, count] of Object.entries(clean)) {
    // match clean key (dir or donor name) back to inventory if possible
    const meta = dmap[donor] || invArr.find(x => (x.donorRepo || '').toLowerCase().includes(donor.toLowerCase().slice(0, 8)) || donor.toLowerCase().includes((x.donorRepo || '').toLowerCase().slice(0, 8))) || {}
    insDonor.run(donor, count, (meta.chronicaTargetCrates || []).join(', ') || '', meta.priority || '', meta.language || '', meta.languageDisposition || '')
  }
})
donorTx()

// --- capability rows: the 195 current slices (full execution metadata where verified) ---
const slices = (reg.rows || []).filter(x => x.kind === 'slice')
const insCap = db.prepare(`INSERT INTO capability
 (key,title,donor,donor_source,target_crate,target_module,inputs_json,outputs_json,gate_class,db_objects,calls_json,acceptance_test,fin_test,status,percent,next_action,blocker,risk_class,moves_money,wave,priority,slice,kind)
 VALUES (@key,@title,@donor,@donor_source,@target_crate,@target_module,@inputs_json,@outputs_json,@gate_class,@db_objects,@calls_json,@acceptance_test,@fin_test,@status,@percent,@next_action,@blocker,@risk_class,@moves_money,@wave,@priority,@slice,@kind)`)
const pctOf = { spec: 0, red: 25, green: 70, verified: 100 }
const sliceTx = db.transaction(() => {
  for (const s of slices) {
    insCap.run({
      key: `slice.${s.slice}`,
      title: s.title || '',
      donor: (s.donorRefs || []).join(', '),
      donor_source: '',
      target_crate: (s.owningCrates || []).join(', '),
      target_module: '',
      inputs_json: '',
      outputs_json: '',
      gate_class: s.movesMoney ? 'money' : (s.sideEffectClass || 'read_only'),
      db_objects: s.dbModels || '',
      calls_json: '',
      acceptance_test: (s.testIds && s.testIds[0]) || '',
      fin_test: s.financialControlTestId || '',
      status: s.status,
      percent: pctOf[s.status] ?? 0,
      next_action: s.status === 'verified' ? '' : 'read slice in 14-vertical-slice-plan; RED->GREEN->verify',
      blocker: (s.dependsOn || []).join(', '),
      risk_class: s.sideEffectClass || '',
      moves_money: s.movesMoney ? 1 : 0,
      wave: null,
      priority: '',
      slice: s.slice,
      kind: 'slice',
    })
  }
})
sliceTx()

// --- donor_capability STUB rows: make the TRUE scope addressable (toward 6,102) ---
// Each donor contributes (true_capabilities - slicesAlreadyCoveringIt) addressable stub rows so the
// full ideal scope is queryable, not just the 195 current slices. Stubs are status='spec', percent=0,
// carrying donor + target_crate + a generated key; they are placeholders to be expanded into full
// execution specs as waves advance (the honest "what remains" surface).
const slicesByDonor = {}
for (const s of slices) for (const dr of (s.donorRefs || [])) slicesByDonor[dr] = (slicesByDonor[dr] || 0) + 1
const insStub = db.prepare(`INSERT INTO capability
 (key,title,donor,target_crate,gate_class,status,percent,next_action,kind)
 VALUES (?,?,?,?,?,?,?,?,?)`)
const stubTx = db.transaction(() => {
  for (const [donor, count] of Object.entries(clean)) {
    const meta = dmap[donor] || invArr.find(x => (x.donorRepo || '').toLowerCase().includes(donor.toLowerCase().slice(0, 8))) || {}
    const crate = (meta.chronicaTargetCrates || [])[0] || ''
    const covered = slicesByDonor[donor] || 0
    const remaining = Math.max(0, count - covered)
    for (let i = 1; i <= remaining; i++) {
      insStub.run(
        `cap.${donor.replace(/[^a-z0-9]+/gi, '-').toLowerCase()}.${i}`,
        `${donor} capability #${i} (unmapped — expand to a real verb-noun action)`,
        donor, crate, 'unknown', 'spec', 0,
        `enumerate from donor source; assign verb-noun key + target module + contract; then RED->GREEN->verify`,
        'donor_capability'
      )
    }
  }
})
stubTx()

const counts = db.prepare(`SELECT kind, count(*) n FROM capability GROUP BY kind`).all()
console.log('built docs/capabilities.db')
console.log('  true ideal denominator:', trueDenom)
console.log('  slices loaded:', slices.length)
console.log('  donor_scope rows:', Object.keys(clean).length)
console.log('  capability rows by kind:', JSON.stringify(counts))
db.close()
