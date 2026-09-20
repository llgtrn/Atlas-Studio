import test from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(here, '..', '..')
const scriptPath = join(here, 'query.mjs')

// Regression guard for the 2571-vs-3038 architecture-linked-capabilities dispute
// (docs/_machine/reconcile-reports/cloud/5d-ratio-benchmark-sync-cycle2-*.json):
// docs/architecture.db is a real, git-tracked, present-in-every-clone DB, so
// `coverage` can and should keep computing real numbers from it -- but per
// docs/023 section 1/3 and docs/116 section 8 it is a local aggregate audit
// target, never a cloud-promotable source of final truth. This test proves the
// CLI can never silently drop that disclaimer and present a bare number as final.
test('CLI: query.mjs coverage always labels its numbers LOCAL_AUDIT_REQUIRED', () => {
  const result = spawnSync('node', [scriptPath, 'coverage'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stderr)

  const output = JSON.parse(result.stdout)
  assert.equal(output.truth_label, 'LOCAL_AUDIT_REQUIRED')
  assert.match(output.local_audit_note, /local aggregate audit/i)
  assert.match(output.local_audit_note, /docs\/023/)

  // The label must accompany the real numbers, not replace them: this DB is
  // present and readable in cloud checkouts (unlike docs/capabilities.db), so
  // the fix here is disclosure, not nulling out a computable figure.
  assert.equal(typeof output.live.linked_distinct_capabilities, 'number')
  assert.ok(output.live.linked_distinct_capabilities > 0)
  assert.equal(typeof output.meta.canonical_capability_count, 'string')
})

test('CLI: query.mjs summary is unaffected (no coverage command, no label expected)', () => {
  const result = spawnSync('node', [scriptPath, 'summary'], { cwd: repoRoot, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stderr)
  const output = JSON.parse(result.stdout)
  assert.equal(output.truth_label, undefined)
  assert.ok(output.meta)
})
