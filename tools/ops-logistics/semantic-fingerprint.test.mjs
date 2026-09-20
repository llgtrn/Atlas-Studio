import test from 'node:test'
import assert from 'node:assert/strict'
import { detectSemanticFingerprintCollisions, fingerprintKey } from './semantic-fingerprint.mjs'

const fingerprint = {
  resource: 'conversation',
  transition: 'open_to_closed',
  effect: 'close_conversation',
  authority: 'required',
  evidence: 'close_result',
  idempotency: 'required',
  unknown_outcome: 'reconcile',
}

test('fingerprint normalization is stable across object key order and casing', () => {
  const a = fingerprintKey(fingerprint)
  const b = fingerprintKey({
    unknown_outcome: ' RECONCILE ',
    idempotency: 'REQUIRED',
    evidence: 'CLOSE_RESULT',
    authority: 'Required',
    effect: 'close_conversation',
    transition: 'open_to_closed',
    resource: 'Conversation',
  })
  assert.equal(a, b)
})

test('same semantic fingerprint cannot map to two canonical internal names', () => {
  const manifest = {
    semantic_convergence: {
      mappings: [
        {
          semantic_id: 'conversation.close',
          fingerprint,
          canonical_term_after: 'conversation.close',
          disposition: 'REUSE_CHRONICA_SEMANTIC',
        },
        {
          semantic_id: 'ticket.resolve',
          fingerprint: { ...fingerprint },
          canonical_term_after: 'ticket.resolve',
          disposition: 'MAP_TO_CHRONICA_SEMANTIC',
        },
      ],
    },
  }
  const result = detectSemanticFingerprintCollisions(manifest)
  assert.equal(result.collisions.length, 1)
  assert.equal(result.errors.length, 1)
  assert.match(result.errors[0], /multiple canonical terms/)
})

test('provider aliases may share a fingerprint when they converge on one canonical term', () => {
  const manifest = {
    semantic_convergence: {
      mappings: [
        {
          semantic_id: 'conversation.close.provider-a',
          fingerprint,
          canonical_term_after: 'conversation.close',
          disposition: 'ALIAS_AT_BOUNDARY',
        },
        {
          semantic_id: 'conversation.close.provider-b',
          fingerprint: { ...fingerprint },
          canonical_term_after: 'conversation.close',
          disposition: 'ALIAS_AT_BOUNDARY',
        },
      ],
    },
  }
  const result = detectSemanticFingerprintCollisions(manifest)
  assert.equal(result.collisions.length, 1)
  assert.deepEqual(result.errors, [])
})
