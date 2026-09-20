import { test } from 'node:test'
import assert from 'node:assert/strict'
import { validateSlice, detectConflicts, planWave, buildScoutPrompt, buildBuilderPrompt, buildVerifierChecklist } from './absorption-controller.mjs'

function slice(overrides) {
  return {
    id: 'S1',
    donor: 'example/donor',
    source_area: 'src/foo.rs',
    behavior: 'example behavior',
    target_responsibility: 'ADAPTER',
    target_paths: ['adapter/src/foo.rs'],
    proof_expectation: 'unit tests',
    deletion_scope: ['temporary/example/src/foo.rs'],
    ...overrides,
  }
}

test('validateSlice accepts a well-formed slice', () => {
  assert.deepEqual(validateSlice(slice()), [])
})

test('validateSlice reports every missing required field, not just the first', () => {
  const errors = validateSlice({ id: 'S1' })
  assert.ok(errors.length >= 6, `expected several missing-field errors, got ${errors.length}: ${JSON.stringify(errors)}`)
})

test('validateSlice rejects an unknown target_responsibility', () => {
  const errors = validateSlice(slice({ target_responsibility: 'ORCHESTRATOR' }))
  assert.ok(errors.some((e) => e.includes('target_responsibility')))
})

test('validateSlice rejects an empty target_paths array', () => {
  const errors = validateSlice(slice({ target_paths: [] }))
  assert.ok(errors.some((e) => e.includes('target_paths')))
})

test('validateSlice accepts an empty deletion_scope while donor technology must be preserved', () => {
  const errors = validateSlice(slice({ deletion_scope: [] }))
  assert.deepEqual(errors, [])
})

test('detectConflicts finds no edges between disjoint adapter slices from different donors', () => {
  const a = slice({ id: 'A', donor: 'nats-io/nats-server', target_paths: ['adapter/src/nats.rs'] })
  const b = slice({ id: 'B', donor: 'open-telemetry/opentelemetry-collector', target_paths: ['adapter/src/otel.rs'] })
  assert.deepEqual(detectConflicts([a, b]), [])
})

test('detectConflicts flags a shared literal target path', () => {
  const a = slice({ id: 'A', target_paths: ['adapter/src/shared.rs'] })
  const b = slice({ id: 'B', target_paths: ['adapter/src/shared.rs'] })
  const conflicts = detectConflicts([a, b])
  assert.equal(conflicts.length, 1)
  assert.match(conflicts[0].reason, /shared target path/)
})

test('detectConflicts flags two slices landing in the same hotspot owner even with different paths', () => {
  const a = slice({ id: 'A', target_responsibility: 'RUNTIME', target_paths: ['runtime/src/world.rs'] })
  const b = slice({ id: 'B', target_responsibility: 'RUNTIME', target_paths: ['runtime/src/world_extra.rs'] })
  const conflicts = detectConflicts([a, b])
  assert.equal(conflicts.length, 1)
  assert.match(conflicts[0].reason, /shared hotspot owner: runtime\/src\/world/)
})

test('detectConflicts does not flag two adapter slices merely for both touching adapter/ (not a hotspot prefix)', () => {
  const a = slice({ id: 'A', target_paths: ['adapter/src/one.rs'] })
  const b = slice({ id: 'B', target_paths: ['adapter/src/two.rs'] })
  assert.deepEqual(detectConflicts([a, b]), [])
})

test('planWave selects all slices into one wave when none conflict', () => {
  const slices = [
    slice({ id: 'A', target_paths: ['adapter/src/a.rs'] }),
    slice({ id: 'B', target_paths: ['adapter/src/b.rs'] }),
    slice({ id: 'C', target_paths: ['adapter/src/c.rs'] }),
  ]
  const { wave, deferred } = planWave(slices)
  assert.deepEqual(wave.map((s) => s.id), ['A', 'B', 'C'])
  assert.deepEqual(deferred, [])
})

test('planWave defers a slice that conflicts with an earlier-chosen slice, in input order', () => {
  const slices = [
    slice({ id: 'A', target_responsibility: 'CORE', target_paths: ['core/src/lib.rs'] }),
    slice({ id: 'B', target_responsibility: 'CORE', target_paths: ['core/src/other.rs'] }), // hotspot clash with A
    slice({ id: 'C', target_paths: ['adapter/src/c.rs'] }), // independent
  ]
  const { wave, deferred } = planWave(slices)
  assert.deepEqual(wave.map((s) => s.id), ['A', 'C'])
  assert.equal(deferred.length, 1)
  assert.equal(deferred[0].id, 'B')
})

test('planWave allows at most one active slice per hotspot but unlimited independent adapter slices', () => {
  const slices = [
    slice({ id: 'RT1', target_responsibility: 'RUNTIME', target_paths: ['runtime/src/authority.rs'] }),
    slice({ id: 'RT2', target_responsibility: 'RUNTIME', target_paths: ['runtime/src/authority_admission.rs'] }),
    slice({ id: 'AD1', target_paths: ['adapter/src/one.rs'] }),
    slice({ id: 'AD2', target_paths: ['adapter/src/two.rs'] }),
    slice({ id: 'AD3', target_paths: ['adapter/src/three.rs'] }),
  ]
  const { wave, deferred } = planWave(slices)
  assert.deepEqual(wave.map((s) => s.id), ['RT1', 'AD1', 'AD2', 'AD3'])
  assert.deepEqual(deferred.map((d) => d.id), ['RT2'])
})

test('buildScoutPrompt embeds base sha, donor, and area, and forbids coding', () => {
  const prompt = buildScoutPrompt('abc123', 'nats-io/nats-server', 'server/reconnect.go')
  assert.match(prompt, /BASE_SHA: abc123/)
  assert.match(prompt, /DONOR: nats-io\/nats-server/)
  assert.match(prompt, /AREA: server\/reconnect\.go/)
  assert.match(prompt, /Do not code\./)
})

test('buildBuilderPrompt embeds the full slice and forbids merging canonical HEAD', () => {
  const s = slice({ id: 'S9' })
  const prompt = buildBuilderPrompt('deadbeef', s)
  assert.match(prompt, /BASE_SHA: deadbeef/)
  assert.match(prompt, /id: S9/)
  assert.match(prompt, /Do not merge canonical HEAD\./)
})

test('buildBuilderPrompt preserves donor source by default and treats deletion as a gated extinction action', () => {
  const prompt = buildBuilderPrompt('deadbeef', slice({ id: 'S9', deletion_scope: [] }))
  assert.match(prompt, /DELETE is a later extinction action/)
  assert.match(prompt, /zero donor deletion is VALID/)
  assert.match(prompt, /keep deletion_scope empty/)
  assert.match(prompt, /Never delete a donor file merely because Chronica no longer builds the donor tree/)
  assert.match(prompt, /donor-coupling-gate\.mjs/)
})

test('buildVerifierChecklist embeds the canonical head, candidate sha, and slice id, listing all nine checks', () => {
  const s = slice({ id: 'S9' })
  const checklist = buildVerifierChecklist('h0', 'c1', s)
  assert.match(checklist, /CANONICAL_HEAD: h0/)
  assert.match(checklist, /CANDIDATE: c1/)
  assert.match(checklist, /SLICE: S9/)
  for (let i = 1; i <= 9; i++) {
    assert.match(checklist, new RegExp(`^${i}\\. `, 'm'), `missing check ${i}`)
  }
  assert.match(checklist, /ACCEPT \| REBASE_REQUIRED \| CONFLICT \| INSUFFICIENT_PROOF \| ARCHITECTURE_VIOLATION \| SCOPE_VIOLATION/)
})

test('buildVerifierChecklist check 6 allows preservation and gates any actual extinction', () => {
  const checklist = buildVerifierChecklist('h0', 'c1', slice({ id: 'S9', deletion_scope: [] }))
  assert.match(checklist, /Zero deletion is\s+valid/)
  assert.match(checklist, /FULLY_NATIVE proof/)
  assert.match(checklist, /no retained donor source elsewhere/)
  assert.match(checklist, /Native Technology Strategy decides whether extinction itself is eligible/)
})
