#!/usr/bin/env node
// CHRONICA -- DISPATCH-FIRST MASS ABSORPTION ENGINE: the smallest real controller/tooling the
// directive asks for, not an agent framework. It covers:
//   - candidate discovery / target declaration -- the slice shape below (validateSlice)
//   - collision detection -- detectConflicts (shared target path, or shared hotspot owner)
//   - wave construction -- planWave (greedy independent-set selection over the conflict graph)
//   - isolated dispatch contract generation -- buildScoutPrompt/buildBuilderPrompt/
//     buildVerifierChecklist (the directive's own templates, filled from a slice)
// Donor-drain recomputation is NOT reimplemented here -- reuse tools/refoundation/
// donor-burndown.mjs, which already derives it from Git. Serial canonical integration is a
// separate script (integrate-candidates.mjs) since it performs real git/cargo operations rather
// than pure planning.
//
// No progress database: a slice here is a planning input the controller discards once dispatched
// (or keeps only as an ephemeral JSON file the operator hands to `--plan`, never a durable ledger
// this tool reads back as truth). Absorption progress itself stays exclusively Git-derived.

import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// Canonical areas the directive singles out for stricter concurrency (section 25): "core root
// semantics, runtime/world, runtime/execution, runtime/authority, runtime/canonicalization,
// graph type system, binding resolution." Adapters are deliberately NOT listed -- they may run
// much wider parallelism, per the same section.
export const HOTSPOT_PREFIXES = [
  'core/',
  'runtime/src/world',
  'runtime/src/execution',
  'runtime/src/authority',
  'runtime/src/canonicalization',
  'graph/',
  'bindings/',
]

const REQUIRED_SLICE_FIELDS = [
  'id',
  'donor',
  'source_area',
  'behavior',
  'target_responsibility',
  'target_paths',
  'proof_expectation',
  'deletion_scope',
]

const VALID_RESPONSIBILITIES = new Set(['CORE', 'RUNTIME', 'ADAPTER', 'ORGANISM', 'APP'])

export function validateSlice(slice) {
  const errors = []
  for (const field of REQUIRED_SLICE_FIELDS) {
    if (slice?.[field] === undefined) errors.push(`missing required field "${field}"`)
  }
  if (slice?.target_responsibility && !VALID_RESPONSIBILITIES.has(slice.target_responsibility)) {
    errors.push(`target_responsibility must be one of ${[...VALID_RESPONSIBILITIES].join(', ')}, got "${slice.target_responsibility}"`)
  }
  if (slice?.target_paths && (!Array.isArray(slice.target_paths) || slice.target_paths.length === 0)) {
    errors.push('target_paths must be a non-empty array')
  }
  if (!Array.isArray(slice?.deletion_scope)) {
    errors.push('deletion_scope must be an array; use [] when donor source must be preserved until the native promotion/extinction gate is satisfied')
  }
  return errors
}

function hotspotsOf(slice) {
  const hits = new Set()
  for (const path of slice.target_paths ?? []) {
    for (const prefix of HOTSPOT_PREFIXES) {
      if (path.startsWith(prefix)) hits.add(prefix)
    }
  }
  return hits
}

/** Conflict edges between two slices: a shared literal target path, or both landing in the same
 * hotspot area. Adapter-only slices with disjoint paths never conflict, by design (section 8:
 * "Adapters may allow much wider parallelism"). */
export function detectConflicts(slices) {
  const conflicts = []
  for (let i = 0; i < slices.length; i++) {
    for (let j = i + 1; j < slices.length; j++) {
      const a = slices[i]
      const b = slices[j]
      const sharedPaths = (a.target_paths ?? []).filter((p) => (b.target_paths ?? []).includes(p))
      if (sharedPaths.length) {
        conflicts.push({ a: a.id, b: b.id, reason: `shared target path: ${sharedPaths.join(', ')}` })
        continue
      }
      const aHotspots = hotspotsOf(a)
      const sharedHotspot = [...hotspotsOf(b)].find((h) => aHotspots.has(h))
      if (sharedHotspot) {
        conflicts.push({ a: a.id, b: b.id, reason: `shared hotspot owner: ${sharedHotspot}` })
      }
    }
  }
  return conflicts
}

/** Greedy independent-set wave planner over the conflict graph (section 8: "does not require
 * mathematically optimal graph coloring; a deterministic greedy scheduler is acceptable
 * initially"). Slices are considered in input order; a slice joins the wave only if it conflicts
 * with nothing already chosen. Deferred slices carry the conflict reason so the next wave (after
 * the chosen set integrates and HEAD advances) can re-evaluate them against new reality. */
export function planWave(slices) {
  const conflicts = detectConflicts(slices)
  const conflictsById = new Map()
  const addConflict = (id, c) => {
    if (!conflictsById.has(id)) conflictsById.set(id, [])
    conflictsById.get(id).push(c)
  }
  for (const c of conflicts) {
    addConflict(c.a, c)
    addConflict(c.b, c)
  }

  const chosen = []
  const chosenIds = new Set()
  const deferred = []
  for (const slice of slices) {
    const blockedBy = (conflictsById.get(slice.id) ?? []).find(
      (c) => chosenIds.has(c.a === slice.id ? c.b : c.a),
    )
    if (blockedBy) {
      deferred.push({ id: slice.id, reason: blockedBy.reason })
    } else {
      chosen.push(slice)
      chosenIds.add(slice.id)
    }
  }
  return { wave: chosen, deferred, conflicts }
}

export function buildScoutPrompt(baseSha, donor, area) {
  return [
    'You are a temporary Chronica absorption Scout.',
    '',
    `BASE_SHA: ${baseSha}`,
    `DONOR: ${donor}`,
    `AREA: ${area ?? '(unscoped -- find one bounded behavior/invariant worth absorbing)'}`,
    '',
    'READ MINIMUM. Read only the donor source needed to identify one bounded technology behavior/invariant and its dependency closure.',
    'Find exactly one bounded behavior/invariant worth reconstructing as Chronica-native technology.',
    'Do NOT require donor deletion for the slice to be executable. For STRATEGIC/FOUNDATIONAL technology, preservation is the default until the Native Technology Strategy extinction gate is satisfied.',
    'If deletion is proposed, it must be a bounded scope already justified by census/lineage/replacement proof; otherwise return an empty deletion scope and continue native reconstruction.',
    'Do not scout the same area again after emitting an executable slice unless a concrete build blocker proves the dependency closure was incomplete.',
    '',
    'Return only:',
    'SOURCE:',
    'BEHAVIOR:',
    'WHY CHRONICA NEEDS IT:',
    'TARGET RESPONSIBILITY:',
    'LIKELY TARGET PATHS:',
    'PROOF STRATEGY:',
    'DONOR DELETE CANDIDATE:',
    'COLLISION RISK:',
    '',
    'Do not code. Do not redesign architecture. Do not generate a report. Do not create metadata.',
  ].join('\n')
}

export function buildBuilderPrompt(baseSha, slice) {
  return [
    'You are a temporary Chronica absorption Builder.',
    '',
    `BASE_SHA: ${baseSha}`,
    '',
    'SLICE:',
    `  id: ${slice.id}`,
    `  donor: ${slice.donor}`,
    `  source_area: ${slice.source_area}`,
    `  behavior: ${slice.behavior}`,
    `  target_responsibility: ${slice.target_responsibility}`,
    `  target_paths: ${JSON.stringify(slice.target_paths)}`,
    `  proof_expectation: ${slice.proof_expectation}`,
    `  deletion_scope: ${JSON.stringify(slice.deletion_scope)}`,
    '',
    'Implement exactly this bounded dependency closure.',
    '',
    'Rules:',
    'ABSORB = CENSUS + NATIVE REPLACEMENT + PROOF. DELETE is a later extinction action, not an automatic property of every slice.',
    'READ MINIMUM -> implement Chronica-native code -> migrate caller when appropriate -> tests/proof -> preserve donor lineage/source by default -> delete only an explicitly assigned and already-justified deletion_scope -> commit candidate.',
    'A candidate that adds/proves a real native slice with zero donor deletion is VALID when source preservation is still required by sequencing or the native promotion gate.',
    'If deletion is not yet justified, keep deletion_scope empty; do not manufacture deletion merely to make progress look complete.',
    '',
    'Never delete a donor file merely because Chronica no longer builds the donor tree. If any',
    'retained donor source (elsewhere in the same temporary/<donor>/) imports/includes/calls/',
    'requires the file(s) you want to delete, RETAIN them -- unless the entire dependency closure',
    'is drained atomically in this same candidate. A native reimplementation stands on its own',
    'whether or not the donor file stays; deleting a donor file the donor tree itself still depends',
    'on is a real correctness bug in the drain, not a cosmetic one, even though Chronica never',
    'builds that tree. This is checked mechanically by donor-coupling-gate.mjs before your',
    'candidate can be integrated -- an unsafe deletion will be rejected regardless of how the rest',
    'of the candidate looks.',
    '',
    'Backend must be Rust. Frontend only may be TypeScript.',
    'Do not create new root architecture. Do not create donor-shaped permanent packages.',
    'Do not depend on temporary/. Do not update manual progress percentages.',
    'Do not merge canonical HEAD.',
    '',
    'Finish with:',
    'CANDIDATE_SHA:',
    'NATIVE_PATHS:',
    'TESTS:',
    'DELETION_SCOPE:',
    'DRAINED_DONOR_PATHS:',
    'DONOR_FILES_REMOVED:',
    'DONOR_FILES_REMAINING_IN_SCOPE:',
    'BLOCKERS:',
  ].join('\n')
}

export function buildVerifierChecklist(canonicalHead, candidateSha, slice) {
  return [
    'You are a temporary Chronica candidate Verifier.',
    '',
    `CANONICAL_HEAD: ${canonicalHead}`,
    `CANDIDATE: ${candidateSha}`,
    `SLICE: ${slice.id} (${slice.donor})`,
    '',
    'Verify:',
    '1. scope matched the assigned slice',
    '2. architecture placement (core/runtime/adapter/organism/apps) is correct',
    '3. Rust backend rule respected (no backend TypeScript)',
    '4. no competing truth universe / duplicate architecture created',
    '5. no runtime dependency on temporary/',
    '6. donor source preservation/extinction is correct for the assigned slice. Zero deletion is',
    '   valid while census, lineage, native reconstruction, parity, sequencing, or FULLY_NATIVE proof',
    '   is incomplete. If deletion_scope is non-empty, deletion must be explicitly justified, match',
    '   the absorbed behavior (not bulk/unrelated deletion), and no retained donor source elsewhere',
    '   in the same temporary/<donor>/ may still import/include/call/require anything deleted unless',
    '   the whole dependency closure is drained atomically. donor-coupling-gate.mjs checks coupling;',
    '   the Native Technology Strategy decides whether extinction itself is eligible.',
    '7. tests prove retained behavior (positive and negative paths)',
    '8. candidate still applies cleanly to current HEAD',
    '9. no duplicate implementation already landed since BASE_SHA',
    '',
    'Return exactly one: ACCEPT | REBASE_REQUIRED | CONFLICT | INSUFFICIENT_PROOF | ARCHITECTURE_VIOLATION | SCOPE_VIOLATION',
    'Then concrete reasons only -- no vague "looks good".',
  ].join('\n')
}

function main(argv = process.argv.slice(2)) {
  const planIdx = argv.indexOf('--plan')
  if (planIdx === -1) {
    console.error('usage: absorption-controller.mjs --plan <slices.json>')
    return 2
  }
  const slicesPath = argv[planIdx + 1]
  if (!slicesPath) {
    console.error('usage: absorption-controller.mjs --plan <slices.json>')
    return 2
  }
  const slices = JSON.parse(readFileSync(slicesPath, 'utf8'))
  const allErrors = slices.flatMap((s) => validateSlice(s).map((e) => `${s.id ?? '?'}: ${e}`))
  if (allErrors.length) {
    console.error('invalid slice(s):')
    for (const e of allErrors) console.error(`  - ${e}`)
    return 1
  }
  const { wave, deferred, conflicts } = planWave(slices)
  console.log(`WAVE (${wave.length} independent slice(s)):`)
  for (const s of wave) console.log(`  ${s.id}  [${s.target_responsibility}]  ${s.donor}  ${JSON.stringify(s.target_paths)}`)
  console.log('')
  console.log(`DEFERRED (${deferred.length}):`)
  for (const d of deferred) console.log(`  ${d.id}  blocked by: ${d.reason}`)
  console.log('')
  console.log(`CONFLICT EDGES: ${conflicts.length}`)
  for (const c of conflicts) console.log(`  ${c.a} <-> ${c.b}: ${c.reason}`)
  return 0
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exit(main())
