import { test } from 'node:test'
import assert from 'node:assert/strict'
import { findTemporaryDependencyViolations, stripLineComments } from './no-temporary-dependency-gate.mjs'

test('the live repo has zero production dependencies on temporary/', () => {
  const violations = findTemporaryDependencyViolations()
  assert.deepEqual(violations, [], `expected no violations, got: ${JSON.stringify(violations)}`)
})

test('findTemporaryDependencyViolations returns an array of human-readable strings, not booleans/objects', () => {
  const violations = findTemporaryDependencyViolations()
  for (const v of violations) assert.equal(typeof v, 'string')
})

test('stripLineComments removes everything from // to end of line, covering Rust doc comments (///, //!) and JS/TS comments alike', () => {
  assert.equal(stripLineComments('let x = 1; // a comment'), 'let x = 1; ')
  assert.equal(stripLineComments('//! a module doc comment'), '')
  assert.equal(stripLineComments('/// a citation `temporary/foo/bar.rs`'), '')
  assert.equal(stripLineComments('no comment here'), 'no comment here')
})

test('a doc comment citing a donor provenance path in backticks (the established style in this codebase) is never a violation, even though the raw text matches the literal-detection pattern', () => {
  // Regression: core::retention_policy's own doc comment -- "the donor file (`temporary/loki/
  // pkg/compactor/retention/expiration.go`) is left untouched" -- was a real false positive
  // this gate produced before stripLineComments() existed, rejecting a good absorption candidate.
  const docComment = [
    '//! Absorbed from grafana/loki (temporary/loki/pkg/compactor/retention/expiration.go: the',
    '//! selection logic).',
    '//!',
    '//! DRAIN: the donor file (`temporary/loki/pkg/compactor/retention/expiration.go`) is left',
    '//! untouched because retained code in the same file calls it directly.',
  ].join('\n')
  const stripped = stripLineComments(docComment)
  assert.doesNotMatch(stripped, /["'`](\.\.\/)*temporary\//)
})
