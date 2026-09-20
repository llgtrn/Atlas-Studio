import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { CliUsageError, parseArgs, writeReviewQueueFiles } from './subdb-conflict-writer.mjs'

test('parseArgs: requires an explicit YYYY-MM-DD date and never reads the system clock', () => {
  assert.deepEqual(parseArgs(['--date', '2026-08-03']), {
    crates: [],
    date: '2026-08-03',
    outDir: join('docs', '_machine', 'subdb-conflicts'),
  })
  assert.throws(() => parseArgs([]), CliUsageError)
  assert.throws(() => parseArgs(['--date', '20260803']), /YYYY-MM-DD/)
})

test('parseArgs: unknown arguments are rejected without echoing operator-controlled text', () => {
  const secret = '--token=sk_live_should_not_echo'
  assert.throws(
    () => parseArgs(['--date', '2026-08-03', secret]),
    (error) => error instanceof CliUsageError && !error.message.includes(secret) && /unknown argument/.test(error.message),
  )
})

test('writeReviewQueueFiles: writes both queue files and refuses overwrite', () => {
  const root = mkdtempSync(join(tmpdir(), 'chronica-subdb-conflict-writer-'))
  try {
    const records = [{ capability_key: 'a.b', crate: 'chronica-x', reason_code: 'CODE_EXISTS_BUT_UNREACHABLE_FROM_ANY_ENTRY_POINT' }]
    const first = writeReviewQueueFiles({
      outDir: root,
      date: '2026-08-03',
      records,
      markdown: '# Review\n',
    })
    assert.equal(readFileSync(first.jsonlPath, 'utf8'), `${JSON.stringify(records[0])}\n`)
    assert.equal(readFileSync(first.mdPath, 'utf8'), '# Review\n')

    assert.throws(
      () => writeReviewQueueFiles({ outDir: root, date: '2026-08-03', records: [], markdown: '# Later\n' }),
      /REFUSING_TO_OVERWRITE_OUTPUT/,
    )
    assert.equal(readFileSync(first.jsonlPath, 'utf8'), `${JSON.stringify(records[0])}\n`)
    assert.deepEqual(readdirSync(root).filter((name) => name.endsWith('.tmp')), [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
