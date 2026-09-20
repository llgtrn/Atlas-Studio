import test from 'node:test'
import assert from 'node:assert/strict'
import { classifyDonorFile, normalizeDonorPath } from './donor-file-census-lib.mjs'

test('normalizes Windows paths to donor-relative slash paths', () => {
  assert.equal(
    normalizeDonorPath('Temporary\\erpnext-main\\erpnext\\accounts\\doctype\\payment_entry.py', 'erpnext-main'),
    'erpnext/accounts/doctype/payment_entry.py'
  )
})

test('classifies generated, vendored, build, binary, support, and source files conservatively', () => {
  assert.deepEqual(classifyDonorFile('node_modules/pkg/index.js'), {
    kind: 'vendor',
    classification: 'generated_vendor_build_artifact',
    readStatus: 'classified_not_read',
    reason: 'vendor directory',
  })
  assert.deepEqual(classifyDonorFile('dist/app.bundle.js'), {
    kind: 'build',
    classification: 'generated_vendor_build_artifact',
    readStatus: 'classified_not_read',
    reason: 'build/output directory',
  })
  assert.deepEqual(classifyDonorFile('assets/logo.png'), {
    kind: 'binary_asset',
    classification: 'non_behavioral_support',
    readStatus: 'classified_not_read',
    reason: 'binary/media asset',
  })
  assert.deepEqual(classifyDonorFile('README.md'), {
    kind: 'documentation',
    classification: 'behavior_review_pending',
    readStatus: 'unread_pending',
    reason: 'documentation may contain behavior contracts',
  })
  assert.deepEqual(classifyDonorFile('src/server.ts'), {
    kind: 'source',
    classification: 'capability_review_pending',
    readStatus: 'unread_pending',
    reason: 'source or behavior-bearing file requires capability review',
  })
})
