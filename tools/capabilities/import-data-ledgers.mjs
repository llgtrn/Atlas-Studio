#!/usr/bin/env node
// import-data-ledgers.mjs — fold the _data/ status ledgers INTO the capability DB side-tables,
// so the DB (one of the two tracking surfaces) is the queryable home of all tracking data,
// not scattered JSON. The JSONs remain build-registry's generated INPUTS (don't break that),
// but their STATUS now also lives in the DB and is queryable alongside the capabilities.
//
//   donor-absorption/absorption-map.generated.json   -> donor_absorption
//   architecture/package-retirement-map.generated.json -> package_retirement
//   parity/continuous-execution-state.json            -> execution_state
//
// Idempotent (INSERT OR REPLACE on the primary key). Reports counts.
import Database from 'better-sqlite3'
import { readFileSync, existsSync } from 'node:fs'
import { CAPABILITIES_DB, ABSORPTION_MAP, RETIREMENT_MAP, EXECUTION_STATE } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')
const J = (p) => existsSync(p) ? JSON.parse(readFileSync(p, 'utf8')) : null

// ── donor_absorption ──────────────────────────────────────────────────────────
const abs = J(ABSORPTION_MAP)
let nAbs = 0
if (abs?.donors) {
  const ins = db.prepare(`INSERT OR REPLACE INTO donor_absorption
    (donor,disposition,target_module,capability_absorbed,optional_bridge,priority,status,test_ids,note)
    VALUES (@donor,@disposition,@target_module,@capability_absorbed,@optional_bridge,@priority,@status,@test_ids,@note)`)
  const tx = db.transaction(() => {
    for (const d of abs.donors) ins.run({
      donor: d.donor, disposition: d.disposition || 'native_absorption', target_module: d.targetModule || '',
      capability_absorbed: d.capabilityAbsorbed || '', optional_bridge: d.optionalBridge ? 1 : 0,
      priority: d.priority || 'P1', status: d.status || (d.disposition === 'discard_or_reference_only' ? 'verified' : 'spec'),
      test_ids: JSON.stringify(d.testIds || []), note: d.note || '',
    })
    nAbs = abs.donors.length
  })
  tx()
}

// ── package_retirement ────────────────────────────────────────────────────────
const ret = J(RETIREMENT_MAP)
let nRet = 0
if (ret?.packages) {
  const ins = db.prepare(`INSERT OR REPLACE INTO package_retirement
    (package,replacement_crate,covered_by_slices,status,rust_route_available,test_ids,note)
    VALUES (@package,@replacement_crate,@covered_by_slices,@status,@rust_route_available,@test_ids,@note)`)
  const tx = db.transaction(() => {
    for (const p of ret.packages) ins.run({
      package: p.package, replacement_crate: p.replacementCrate || '', covered_by_slices: JSON.stringify(p.coveredBySlices || []),
      status: p.status || 'active', rust_route_available: p.rustRouteAvailable === undefined ? null : (p.rustRouteAvailable ? 1 : 0),
      test_ids: JSON.stringify(p.testIds || []), note: p.note || '',
    })
    nRet = ret.packages.length
  })
  tx()
}

// ── execution_state (flatten the top-level keys) ──────────────────────────────
const ex = J(EXECUTION_STATE)
let nEx = 0
if (ex) {
  const ins = db.prepare('INSERT OR REPLACE INTO execution_state (k,v,detail,set_at) VALUES (?,?,?,?)')
  const at = ex.lastUpdated || ''
  const tx = db.transaction(() => {
    for (const [k, v] of Object.entries(ex)) {
      const val = (typeof v === 'object') ? '' : String(v)
      const detail = (typeof v === 'object') ? JSON.stringify(v) : ''
      ins.run(k, val, detail, at); nEx++
    }
  })
  tx()
}

console.log(`import-data-ledgers: donor_absorption=${nAbs}, package_retirement=${nRet}, execution_state=${nEx} (folded into capabilities.db side-tables)`)
db.close()
