// export-cloud-snapshot.git-source.test.mjs - integration test for the
// git_canonical_shards fallback path added to export-cloud-snapshot.mjs: when
// the local-only docs/capabilities.db / docs/architecture.db are absent (any
// fresh checkout), the script must still succeed, using only git-tracked
// docs/capabilities-canonical/**/*.jsonl and docs/architecture-canonical/**/*.jsonl,
// and must report (never silently drop) the tables that genuinely aren't
// reproducible from git yet.
//
// This runs against the real repo's canonical shards rather than a synthetic
// fixture, since the thing under test is "does this work in a real fresh
// checkout" - docs/capabilities.db is not git-tracked, so it is guaranteed
// absent here regardless of environment.
import test from 'node:test'
import assert from 'node:assert/strict'
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import Database from 'better-sqlite3'

const ROOT = process.cwd()

test('export-cloud-snapshot.mjs succeeds from git-tracked canonical shards alone and reports genuine gaps', { timeout: 60_000 }, () => {
  assert.ok(!existsSync(join(ROOT, 'docs', 'capabilities.db')), 'precondition: docs/capabilities.db must not be git-tracked')
  assert.ok(!existsSync(join(ROOT, 'docs', 'architecture.db')), 'precondition: docs/architecture.db must not be git-tracked')

  const outDir = mkdtempSync(join(tmpdir(), 'chronica-cloud-snapshot-e2e-'))
  try {
    const result = spawnSync(
      process.execPath,
      [join(ROOT, 'tools', 'capabilities', 'export-cloud-snapshot.mjs'), '--out', outDir],
      { cwd: ROOT, encoding: 'utf8' },
    )
    assert.equal(result.status, 0, `export-cloud-snapshot exited ${result.status}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`)
    assert.match(result.stdout, /git_canonical_shards/)

    const meta = JSON.parse(readFileSync(join(outDir, 'meta.json'), 'utf8'))
    assert.equal(meta.source_mode, 'git_canonical_shards')
    assert.ok(meta.source.canonical_capabilities > 0, 'canonical_capabilities must be real, non-empty data')
    assert.ok(meta.source.money_canonical > 0)
    assert.equal(meta.source.money_without_approval, 0)
    assert.deepEqual(
      Object.keys(meta.git_source_gaps).sort(),
      ['agent_note'],
    )

    const verify = spawnSync(
      process.execPath,
      [join(ROOT, 'tools', 'capabilities', 'verify-cloud-snapshot.mjs'), '--dir', outDir],
      { cwd: ROOT, encoding: 'utf8' },
    )
    assert.equal(verify.status, 0, `verify-cloud-snapshot exited ${verify.status}\nstdout:\n${verify.stdout}\nstderr:\n${verify.stderr}`)

    const core = new Database(join(outDir, 'cap-core.db'), { readonly: true })
    assert.ok(core.prepare('SELECT count(*) n FROM canonical_capability').get().n > 0)
    core.close()

    // donor_file_census / source_file_capability_link are git-tracked
    // (donor-sharded, PR #2612) as of this fallback, so census data must be
    // real - never a faked/empty count in git-source mode. Known-good totals
    // from the #2612 audit: 431,013 donor_file_census rows, 408,169 unread_pending.
    const census = new Database(join(outDir, 'cap-census-summary.db'), { readonly: true })
    const totals = census.prepare('SELECT * FROM donor_file_census_totals').get()
    assert.equal(totals.rows, 431013)
    assert.equal(totals.donors, 94)
    assert.equal(totals.unread_pending, 408169)
    const linkRows = census.prepare('SELECT sum(rows) n FROM source_file_link_summary').get().n
    assert.equal(linkRows, 246539)
    const gapNote = census.prepare("SELECT value FROM cloud_snapshot_info WHERE key='census_gap'").get()
    assert.equal(gapNote, undefined, 'cap-census-summary.db must NOT carry a census_gap note once the source tables are git-tracked')
    census.close()
  } finally {
    rmSync(outDir, { recursive: true, force: true })
  }
})
