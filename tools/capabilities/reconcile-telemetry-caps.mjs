#!/usr/bin/env node
// Reconcile stale composite telemetry rows with the verified local telemetry cap.

import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const db = new Database(CAPABILITIES_DB)
const now = new Date().toISOString()

const key = 'obs.traces.capture_telemetry_events'
const exclusionNote =
  'Duplicate/composite stale row. Local product/usage telemetry capture is verified as obs.emit_telemetry_events in chronica-observability::telemetry; external telemetry export/OTLP/vendor sending should remain separate unimplemented observability capabilities.'
const blocker =
  'No unique Chronica implementation under this key; donor provenance mixes local capture, external analytics sending, OTLP tracing/export, and opt-out policy.'

db.transaction(() => {
  db.prepare(`
    UPDATE canonical_capability
       SET status='excluded',
           target_crate='chronica-observability',
           target_module=NULL,
           side_effect_class='external_write',
           moves_money=0,
           requires_approval=1,
           acceptance_test=NULL,
           required_tests=NULL,
           financial_control_test=NULL,
           blocker=@blocker,
           exclusion_note=@exclusion_note
     WHERE key=@key
  `).run({ key, blocker, exclusion_note: exclusionNote })

  db.prepare(`
    INSERT INTO canonical_status_override (
      canonical_key,status,acceptance_test,financial_control_test,blocker,set_at,set_by,moves_money
    ) VALUES (
      @key,'excluded',NULL,NULL,@blocker,@set_at,'codex',0
    )
    ON CONFLICT(canonical_key) DO UPDATE SET
      status=excluded.status,
      acceptance_test=excluded.acceptance_test,
      financial_control_test=excluded.financial_control_test,
      blocker=excluded.blocker,
      set_at=excluded.set_at,
      set_by=excluded.set_by,
      moves_money=excluded.moves_money
  `).run({ key, blocker, set_at: now })

  db.prepare('DELETE FROM impl_evidence WHERE canonical_key=@key').run({ key })
})()

const row = db.prepare(`
  SELECT key,status,target_crate,side_effect_class,requires_approval,exclusion_note
    FROM canonical_capability WHERE key=?
`).get(key)
console.log(`reconcile-telemetry-caps: ${row.key} -> ${row.status} (${row.target_crate}, ${row.side_effect_class})`)
db.close()
