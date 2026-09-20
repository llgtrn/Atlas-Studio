import test from 'node:test'
import assert from 'node:assert/strict'
import {
  assertNoInventedVerified,
  findDuplicateIds,
  findDuplicateKeys,
  mergeRecoverySources,
  missingColumns,
  rowsFromArchitectureLinks,
  rowsFromCloudCore,
} from './canonical-recovery-lib.mjs'

test('findDuplicateKeys returns keys appearing more than once', () => {
  assert.deepEqual(findDuplicateKeys([{ key: 'a' }, { key: 'b' }, { key: 'a' }]), ['a'])
  assert.deepEqual(findDuplicateKeys([{ key: 'a' }, { key: 'b' }]), [])
})

test('findDuplicateIds returns positive integer IDs appearing more than once', () => {
  assert.deepEqual(findDuplicateIds([{ id: 10 }, { id: 42 }, { id: 10 }]), [10])
  assert.deepEqual(findDuplicateIds([{ id: 10 }, { id: 42 }]), [])
})

test('missingColumns reports required columns absent from a source', () => {
  assert.deepEqual(missingColumns(['key', 'status'], ['key', 'status', 'moves_money']), ['moves_money'])
  assert.deepEqual(missingColumns(['key', 'status', 'moves_money'], ['key', 'status']), [])
})

test('rowsFromCloudCore passes through cap-core.db rows including a pre-existing verified status', () => {
  const rows = rowsFromCloudCore([
    { id: 42, key: 'erp.invoice.post', canonical_name: 'Post invoice', domain: 'erp', target_crate: 'chronica-erp', moves_money: 1, requires_approval: 1, status: 'verified', financial_control_test: 'money_flow_test' },
  ])
  assert.equal(rows.length, 1)
  assert.equal(rows[0].status, 'verified')
  assert.equal(rows[0].recovery_source, 'docs/capabilities-cloud/cap-core.db')
  assert.equal(rows[0].moves_money, 1)
  assert.equal(rows[0].id, 42)
})

test('rowsFromArchitectureLinks never emits verified and conservatively flags money domains', () => {
  const rows = rowsFromArchitectureLinks([
    { capability_key: 'erp.ledger.post', target_crate: 'chronica-erp', target_module: 'ledger', status: 'verified' },
    { capability_key: 'osint.trend.scan', target_crate: 'chronica-osint', target_module: 'scan', status: 'unimplemented' },
  ])
  const byKey = Object.fromEntries(rows.map((r) => [r.key, r]))
  assert.equal(byKey['erp.ledger.post'].status, 'unimplemented', 'verified from arch.db must be downgraded')
  assert.equal(byKey['erp.ledger.post'].moves_money, 1, 'erp domain is conservatively money-flagged')
  assert.ok(byKey['erp.ledger.post'].financial_control_test.startsWith('REQUIRED'))
  assert.equal(byKey['osint.trend.scan'].moves_money, 0)
  assert.equal(byKey['osint.trend.scan'].financial_control_test, null)
})

test('rowsFromArchitectureLinks collapses duplicate keys to one representative row, preferring the higher-rank status', () => {
  const rows = rowsFromArchitectureLinks([
    { capability_key: 'x.y', target_crate: null, target_module: null, status: 'unimplemented' },
    { capability_key: 'x.y', target_crate: 'chronica-x', target_module: 'mod', status: 'verified' },
  ])
  assert.equal(rows.length, 1)
  assert.equal(rows[0].target_crate, 'chronica-x')
  assert.equal(rows[0].status, 'unimplemented', 'still downgraded even though the winning source row was verified')
})

test('mergeRecoverySources: cap-core.db is primary; architecture.db only backfills keys cap-core.db lacks', () => {
  const merged = mergeRecoverySources({
    cloudCoreRows: [{ id: 10, key: 'a.one', canonical_name: 'A One', status: 'unimplemented', moves_money: 0, requires_approval: 0 }],
    archLinkRows: [
      { capability_key: 'a.one', target_crate: 'should-be-ignored', target_module: null, status: 'verified' },
      { capability_key: 'b.two', target_crate: 'chronica-b', target_module: null, status: 'unimplemented' },
    ],
  })
  assert.equal(merged.issues.length, 0)
  const byKey = Object.fromEntries(merged.rows.map((r) => [r.key, r]))
  assert.equal(byKey['a.one'].recovery_source, 'docs/capabilities-cloud/cap-core.db', 'cap-core.db wins for an overlapping key')
  assert.equal(byKey['b.two'].recovery_source, 'docs/architecture.db', 'architecture.db backfills a key cap-core.db lacks')
  assert.equal(byKey['a.one'].id, 10)
  assert.equal(byKey['b.two'].id, 11, 'architecture-only rows receive deterministic IDs above the cloud maximum')
})

test('mergeRecoverySources fails closed on a malformed snapshot with duplicate keys (no partial merge)', () => {
  const merged = mergeRecoverySources({
    cloudCoreRows: [
      { id: 1, key: 'dup.key', canonical_name: 'First', status: 'unimplemented', moves_money: 0, requires_approval: 0 },
      { id: 2, key: 'dup.key', canonical_name: 'Second (corrupt duplicate)', status: 'unimplemented', moves_money: 0, requires_approval: 0 },
    ],
    archLinkRows: [],
  })
  assert.equal(merged.rows.length, 0, 'no rows are returned when the snapshot is malformed')
  assert.equal(merged.issues.length, 1)
  assert.equal(merged.issues[0].type, 'MALFORMED_SNAPSHOT_DUPLICATE_KEY')
  assert.deepEqual(merged.issues[0].keys, ['dup.key'])
})

test('mergeRecoverySources fails closed on a row with an empty/missing key', () => {
  const merged = mergeRecoverySources({
    cloudCoreRows: [{ id: 1, key: '', canonical_name: 'No key', status: 'unimplemented', moves_money: 0, requires_approval: 0 }],
    archLinkRows: [],
  })
  assert.equal(merged.rows.length, 0)
  assert.equal(merged.issues[0].type, 'MALFORMED_SNAPSHOT_EMPTY_KEY')
})

test('mergeRecoverySources fails closed on duplicate or invalid canonical IDs', () => {
  const duplicate = mergeRecoverySources({
    cloudCoreRows: [
      { id: 7, key: 'a', canonical_name: 'A', status: 'unimplemented', moves_money: 0, requires_approval: 0 },
      { id: 7, key: 'b', canonical_name: 'B', status: 'unimplemented', moves_money: 0, requires_approval: 0 },
    ],
  })
  assert.equal(duplicate.rows.length, 0)
  assert.equal(duplicate.issues[0].type, 'MALFORMED_SNAPSHOT_DUPLICATE_ID')

  const invalid = mergeRecoverySources({
    cloudCoreRows: [{ id: 0, key: 'a', canonical_name: 'A', status: 'unimplemented', moves_money: 0, requires_approval: 0 }],
  })
  assert.equal(invalid.rows.length, 0)
  assert.equal(invalid.issues[0].type, 'MALFORMED_SNAPSHOT_INVALID_ID')
})

test('assertNoInventedVerified throws only for architecture.db-sourced verified rows, never for cap-core.db rows', () => {
  assert.doesNotThrow(() => assertNoInventedVerified([{ key: 'a', status: 'verified', recovery_source: 'docs/capabilities-cloud/cap-core.db' }]))
  assert.throws(() => assertNoInventedVerified([{ key: 'b', status: 'verified', recovery_source: 'docs/architecture.db' }]), /verified.*architecture\.db/i)
})
