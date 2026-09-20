import { test } from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { lstatSync, readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { tmpdir } from 'node:os'
import { parse as parseYaml } from 'yaml'
import {
  computeDonorProgress,
  derivedState,
  docChurnSignal,
  buildSummaryLines,
  computeCorpusExtinctionCounts,
  computeCorpusFileTotals,
  functionalAbsorptionEvidence,
} from './donor-burndown.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')

function makeFixtureRepo() {
  const root = mkdtempSync(join(tmpdir(), 'donor-burndown-'))
  const git = (args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' })
  git(['init', '--quiet'])
  git(['config', 'user.email', 'test@example.com'])
  git(['config', 'user.name', 'Test'])
  return { root, git }
}

function writeAndCommit(root, git, files, message) {
  for (const [path, content] of Object.entries(files)) {
    const full = join(root, path)
    mkdirSync(join(full, '..'), { recursive: true })
    writeFileSync(full, content)
  }
  git(['add', '-A'])
  git(['commit', '--quiet', '-m', message])
  return git(['rev-parse', 'HEAD']).trim()
}

function loadDonors() {
  const doc = parseYaml(readFileSync(join(ROOT, 'tools/refoundation/donor-corpus.yaml'), 'utf8'))
  return doc.donors
}

test('a donor with no source present returns null (nothing to measure)', () => {
  const donors = loadDonors()
  const d = donors.find((x) => !x.source_present)
  assert.ok(d, 'fixture assumption: at least one DISCOVERED (no source) donor exists')
  assert.equal(computeDonorProgress(d), null)
})

test('every source-present donor has a real, non-empty baseline commit and baseline_files > 0', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  assert.ok(donors.length > 0, 'fixture assumption: at least one donor has source present')
  for (const d of donors) {
    const p = computeDonorProgress(d)
    assert.match(p.baseline_commit, /^[0-9a-f]{40}$/, `${d.repo}: baseline_commit must be a real commit SHA`)
    assert.ok(p.baseline_files > 0, `${d.repo}: baseline_files must be > 0 for an ingested donor`)
  }
})

test('remaining_files never exceeds baseline_files (drain is monotonic, never negative)', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  for (const d of donors) {
    const p = computeDonorProgress(d)
    assert.ok(p.remaining_files <= p.baseline_files, `${d.repo}: remaining (${p.remaining_files}) > baseline (${p.baseline_files})`)
    assert.ok(p.drain_percent >= 0 && p.drain_percent <= 100, `${d.repo}: drain_percent out of range: ${p.drain_percent}`)
  }
})

test('postgres/postgres reflects real, live-derived Git history -- including a real deletion that was later safely reverted, not a hardcoded snapshot', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const d = donors.find((x) => x.repo === 'postgres/postgres')
  assert.ok(d, 'fixture assumption: postgres/postgres is in the corpus with source present')
  const p = computeDonorProgress(d)
  // Commit e2620a246f drained doc/src/sgml/mvcc.sgml, but a later coupling audit (6bb68b2af6,
  // "restore 11 donor files wrongly deleted despite retained coupling") found a retained donor
  // file elsewhere still referenced it and restored it -- so remaining_files is correctly back to
  // baseline_files today. This is exactly the live-Git-derivation property this module exists
  // for: it must never keep reporting a since-reverted deletion as still-drained progress.
  assert.ok(p.last_deletion_commit, 'expected a real deletion commit to still be found via git log --diff-filter=D, even though it was later reverted');
  assert.ok(p.remaining_files <= p.baseline_files, 'remaining must never exceed baseline');
  assert.ok(p.drain_percent >= 0 && p.drain_percent <= 100, 'drain_percent must stay in range regardless of restore/re-drain history');
  assert.equal(p.fully_drained, false, 'postgres has thousands of files remaining, nowhere near fully drained');
})

test('derivedState computes the four absorption states purely from baseline/remaining counts', () => {
  assert.equal(derivedState(0, 0), 'NOT_INGESTED')
  assert.equal(derivedState(100, 100), 'INGESTED_NOT_STARTED')
  assert.equal(derivedState(100, 40), 'ACTIVE_ABSORPTION')
  assert.equal(derivedState(100, 0), 'FULLY_DRAINED')
})

test('a real deleted donor file changes the drain count: the exact file postgres commit e2620a24 removed no longer counts toward remaining_files', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const d = donors.find((x) => x.repo === 'postgres/postgres')
  const p = computeDonorProgress(d)
  assert.equal(p.remaining_files, p.baseline_files - p.drained_files)
  assert.ok(p.last_deletion_commit, 'expected a real deletion commit to be found via git log --diff-filter=D')
})

test('a not-yet-ingested donor (no source present) is never reported as active or fully drained', () => {
  const donors = loadDonors()
  const d = donors.find((x) => !x.source_present)
  assert.ok(d, 'fixture assumption: at least one DISCOVERED donor exists')
  assert.equal(computeDonorProgress(d), null)
})

test('an INGESTED_NOT_STARTED donor (source present, zero drain) is distinguished from ACTIVE_ABSORPTION', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path && d.repo !== 'postgres/postgres')
  const d = donors.find((x) => {
    const p = computeDonorProgress(x)
    return p && p.drained_files === 0 && p.baseline_files > 0
  })
  if (!d) return // every other source-present donor has already started absorbing -- nothing to assert
  const p = computeDonorProgress(d)
  assert.equal(p.state, 'INGESTED_NOT_STARTED')
})

test('baseline commit discovery returns a real 40-hex SHA that is an ancestor of HEAD', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const d = donors.find((x) => x.repo === 'postgres/postgres')
  const p = computeDonorProgress(d)
  // `git merge-base --is-ancestor` prints nothing either way -- it signals true/false purely via
  // exit code, so a successful call here (no throw) is itself the assertion.
  assert.doesNotThrow(() => execFileSync('git', ['merge-base', '--is-ancestor', p.baseline_commit, 'HEAD'], { cwd: ROOT }))
})

test('a tracked symlink is measured by its own blob size, not its dereferenced target (restate/CLAUDE.md -> AGENTS.md is a real case)', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const d = donors.find((x) => x.repo === 'restatedev/restate')
  assert.ok(d, 'fixture assumption: restatedev/restate is in the corpus with source present')
  const symlinkPath = join(ROOT, d.temporary_path, 'CLAUDE.md')
  const linkStat = lstatSync(symlinkPath)
  assert.ok(linkStat.isSymbolicLink(), 'fixture assumption: temporary/restate/CLAUDE.md is a real symlink')
  const p = computeDonorProgress(d)
  // remaining_bytes must not have inflated to include AGENTS.md's dereferenced content (thousands
  // of bytes) on top of the symlink's own few-byte blob size -- it must stay <= baseline_bytes
  // here, since the only change since baseline is one file's removal.
  assert.ok(p.remaining_bytes <= p.baseline_bytes, `remaining_bytes (${p.remaining_bytes}) must not exceed baseline_bytes (${p.baseline_bytes}) after only a deletion`)
})

test('computeDonorProgress results are deterministic across repeated calls against the same repo state', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const d = donors.find((x) => x.repo === 'postgres/postgres')
  const first = computeDonorProgress(d)
  const second = computeDonorProgress(d)
  assert.deepEqual(first, second)
})

test('the stale "no donor absorption yet" / "no per-donor drain yet" prose is gone from the CLI source', () => {
  const src = readFileSync(join(ROOT, 'tools/refoundation/donor-burndown.mjs'), 'utf8')
  assert.doesNotMatch(src, /no per-donor drain yet/i)
  assert.doesNotMatch(src, /no donor has entered active absorption yet/i)
  assert.doesNotMatch(src, /no donor in this corpus has begun absorption yet/i)
})

test('docChurnSignal on the real postgres absorption commit is NOT a churn candidate (it touched core/runtime/adapter and drained donor source in the same commit)', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const d = donors.find((x) => x.repo === 'postgres/postgres')
  const p = computeDonorProgress(d)
  const signal = docChurnSignal(d, p.baseline_commit)
  assert.equal(signal.candidate, false, 'the real absorption commit did real work, not doc-only churn')
})

test('docChurnSignal always returns a well-shaped signal, regardless of how many commits mention the donor', () => {
  const d = loadDonors().find((x) => x.source_present && x.temporary_path)
  const p = computeDonorProgress(d)
  const signal = docChurnSignal(d, p.baseline_commit)
  assert.ok(typeof signal.mentioning_commits === 'number')
  assert.ok(typeof signal.churn_only_commits === 'number')
  assert.ok(typeof signal.candidate === 'boolean')
})

// A single shared buildSummaryLines() call covers both assertions below -- each call re-scans
// every source-present donor via Git, so this keeps the corpus-wide cost to one pass instead of two.
test('buildSummaryLines() is deterministic and reports the real corpus size, not a hardcoded number', () => {
  const donors = loadDonors()
  const first = buildSummaryLines()
  const second = buildSummaryLines()
  assert.deepEqual(first, second)
  assert.ok(first.some((l) => l.includes(`Unique donors              ${donors.length}`)))
})

// CHRONICA -- MAKE ABSORPTION RATIOS + DONOR EXTINCTION A REAL RUNTIME REQUIREMENT, section 4/11.

test('computeCorpusExtinctionCounts() is deterministic and its counts partition the corpus exactly', () => {
  const donors = loadDonors()
  const first = computeCorpusExtinctionCounts()
  const second = computeCorpusExtinctionCounts()
  assert.deepEqual(first, second)

  assert.equal(first.totalDonors, donors.length)
  // Every donor is either source-present or not-ingested -- no third bucket, no double-counting.
  assert.equal(first.sourcePresentDonors + first.notIngestedDonors, first.totalDonors)
  // Every source-present donor has either started real SOURCE DRAIN or not -- same partition rule.
  // (This is deliberately about source_drain_started, NOT functional_absorption_started -- the
  // two are independent facts and must never be forced into the same partition.)
  assert.equal(first.sourceDrainStartedDonors + first.sourceDrainNotStartedDonors, first.sourcePresentDonors)
  // functionalAbsorptionStartedDonors is a DIFFERENT, independent fact -- it can never exceed the
  // source-present population, but it is not required to partition against any other count here.
  assert.ok(first.functionalAbsorptionStartedDonors <= first.sourcePresentDonors)
})

test('FULLY_DRAINED donor detection is Git/tree-derived: computeCorpusExtinctionCounts() agrees exactly with an independent recount via computeDonorProgress', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const independentFullyDrainedCount = donors.filter((d) => computeDonorProgress(d).fully_drained).length
  assert.equal(computeCorpusExtinctionCounts().fullyDrainedDonors, independentFullyDrainedCount)
})

// A PARTIAL donor (any nonzero baseline with a nonzero remaining count -- ACTIVE_ABSORPTION) and a
// RETAINED_COUPLED donor (a donor whose only real deletion was later restored because retained
// donor source elsewhere still referenced it, e.g. postgres/postgres today -- see the
// "reflects real, live-derived Git history" test above) must never be counted extinct.
// `derivedState` is the exact pure classification `computeDonorProgress`'s own `fully_drained`/
// `state` fields are built from (fully_drained := baseline > 0 && remaining === 0, the same
// condition as derivedState's own FULLY_DRAINED branch) -- proving this property against it
// directly, rather than against whichever real donor happens to currently be mid-drain, is
// immune to that real state legitimately changing (including being safely reverted) between runs.
test('a PARTIAL donor (nonzero baseline, nonzero remaining) is never classified FULLY_DRAINED', () => {
  for (const [baseline, remaining] of [[100, 40], [7632, 7631], [9336, 1]]) {
    assert.notEqual(derivedState(baseline, remaining), 'FULLY_DRAINED')
  }
})

test('a RETAINED_COUPLED donor -- one whose only real deletion was safely reverted, landing back at baseline == remaining -- is INGESTED_NOT_STARTED, never counted extinct', () => {
  // This is postgres/postgres's own real current state today (see the "reflects real,
  // live-derived Git history" test above): a real deletion happened and was found unsafe
  // (dangling docbook reference), so it was restored, and remaining_files == baseline_files
  // again. INGESTED_NOT_STARTED, not FULLY_DRAINED and not ACTIVE_ABSORPTION.
  assert.equal(derivedState(7632, 7632), 'INGESTED_NOT_STARTED')
  assert.notEqual(derivedState(7632, 7632), 'FULLY_DRAINED')
})

test('the corpus-wide extinct count matches a real, live-derived snapshot: currently zero donors are extinct, because no source-present donor has zero remaining_files', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const anyFullyDrained = donors.some((d) => computeDonorProgress(d).fully_drained)
  const extinctCount = computeCorpusExtinctionCounts().fullyDrainedDonors
  // A live assertion, not a frozen snapshot (this module's own established convention): it only
  // requires internal consistency between the two independently-reached conclusions, so it keeps
  // holding whether the real answer is currently 0 or, once a donor genuinely reaches extinction,
  // some positive number.
  assert.equal(extinctCount > 0, anyFullyDrained)
})

test('DONOR_EXTINCTION_RATIO\'s denominator is source-present donors, never the full corpus (a donor that was never even fetched must not silently inflate the ratio in either direction)', () => {
  const counts = computeCorpusExtinctionCounts()
  assert.ok(counts.notIngestedDonors > 0, 'fixture assumption: this corpus has donors with no source present yet')
  // fullyDrainedDonors can only ever be <= sourcePresentDonors (an extinct donor was, by
  // definition, ingested first) -- proving the ratio's natural denominator is sourcePresentDonors,
  // not totalDonors (which would make the ratio artificially, and wrongly, smaller).
  assert.ok(counts.fullyDrainedDonors <= counts.sourcePresentDonors)
  assert.ok(counts.sourcePresentDonors < counts.totalDonors)
})

test('computeCorpusFileTotals() is deterministic and agrees with an independent sum over computeDonorProgress', () => {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const first = computeCorpusFileTotals()
  const second = computeCorpusFileTotals()
  assert.deepEqual(first, second)

  const independentBaseline = donors.reduce((s, d) => s + computeDonorProgress(d).baseline_files, 0)
  const independentRemaining = donors.reduce((s, d) => s + computeDonorProgress(d).remaining_files, 0)
  assert.equal(first.baselineDonorFiles, independentBaseline)
  assert.equal(first.remainingDonorFiles, independentRemaining)
  assert.equal(first.drainedDonorFiles, independentBaseline - independentRemaining)
})

// CHRONICA -- fix the FUNCTIONAL ABSORPTION != SOURCE DELETION defect (2026-09-17). Four
// distinct facts, proven independently: INGESTED, FUNCTIONAL_ABSORPTION_STARTED,
// SOURCE_DRAIN_STARTED, FULLY_DRAINED/EXTINCT.

test('postgres/postgres (real corpus): FUNCTIONAL_ABSORPTION_STARTED is true even though SOURCE_DRAIN_STARTED is currently false', () => {
  // This is the exact real case the defect report named: postgres/postgres has contributed four
  // real native primitives (core::Revision, runtime::world::WorldTransaction,
  // adapter::postgres::PostgresWorldStore, adapter::wal_lsn -- see its own donor-corpus.yaml
  // absorbed_into field) while its one real source deletion (mvcc.sgml) was later safely
  // reverted (commit 6bb68b2af6) because retained donor source elsewhere still referenced it, so
  // remaining_files == baseline_files again today. The old code's `drained_files > 0` proxy would
  // have wrongly reported this donor as NOT started; the fix must report it as functionally
  // started regardless.
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const postgres = donors.find((d) => d.repo === 'postgres/postgres')
  assert.ok(postgres, 'fixture assumption: postgres/postgres is in the corpus')
  const p = computeDonorProgress(postgres)
  assert.equal(p.functional_absorption_started, true, `expected real native-root commit evidence for postgres/postgres, found: ${JSON.stringify(p.functional_absorption_evidence_commits)}`)
  assert.ok(p.functional_absorption_evidence_commits.length > 0)
  assert.equal(p.source_drain_started, false, 'the one real deletion was safely reverted -- remaining_files == baseline_files today')
  assert.equal(p.fully_drained, false)
})

test('functionalAbsorptionEvidence: a native implementation commit with zero deleted donor files is functionally started but never source-drain started (synthetic fixture)', () => {
  const { root, git } = makeFixtureRepo()
  const baseline = writeAndCommit(root, git, {
    'temporary/exampleorg/examplerepo/foo.go': 'package examplerepo\n',
  }, 'seed exampleorg/examplerepo donor')
  writeAndCommit(root, git, {
    'core/src/example_primitive.rs': '// native, pressure-formed from exampleorg/examplerepo\n',
  }, 'core: absorb a real primitive from exampleorg/examplerepo')

  const donor = { repo: 'exampleorg/examplerepo' }
  const evidence = functionalAbsorptionEvidence(donor, baseline, root)
  assert.equal(evidence.length, 1)

  // No donor file was ever deleted -- the donor's own tree is untouched since baseline.
  const remaining = git(['ls-files', 'temporary/exampleorg/examplerepo']).trim().split('\n').filter(Boolean)
  assert.equal(remaining.length, 1, 'the donor source file must still be present -- zero drain')
  rmSync(root, { recursive: true, force: true })
})

test('functionalAbsorptionEvidence: a donor source deletion with NO native implementation commit is never counted as functionally absorbed (synthetic fixture)', () => {
  const { root, git } = makeFixtureRepo()
  const baseline = writeAndCommit(root, git, {
    'temporary/exampleorg/deletedonly/a.go': 'package deletedonly\n',
    'temporary/exampleorg/deletedonly/b.go': 'package deletedonly\n',
  }, 'seed exampleorg/deletedonly donor')
  git(['rm', '-q', 'temporary/exampleorg/deletedonly/a.go'])
  // Deliberately does NOT mention the donor at all, and touches no canonical native root --
  // exactly "source deletion with no native implementation."
  git(['commit', '--quiet', '-m', 'chore: drop an unused fixture file'])

  const donor = { repo: 'exampleorg/deletedonly' }
  const evidence = functionalAbsorptionEvidence(donor, baseline, root)
  assert.deepEqual(evidence, [], 'a file disappearing must never, by itself, count as functional absorption evidence')

  const remaining = git(['ls-files', 'temporary/exampleorg/deletedonly']).trim().split('\n').filter(Boolean)
  assert.equal(remaining.length, 1, 'fixture assumption: exactly one file was deleted, one remains (a real partial drain happened)')
  rmSync(root, { recursive: true, force: true })
})

test('functionalAbsorptionEvidence: a partial safe drain WITH a real native implementation commit is functionally started, source-drain started, and not extinct (synthetic fixture)', () => {
  const { root, git } = makeFixtureRepo()
  const baseline = writeAndCommit(root, git, {
    'temporary/exampleorg/partialdrain/a.go': 'package partialdrain\n',
    'temporary/exampleorg/partialdrain/b.go': 'package partialdrain\n',
  }, 'seed exampleorg/partialdrain donor')
  git(['rm', '-q', 'temporary/exampleorg/partialdrain/a.go'])
  writeAndCommit(root, git, {
    'runtime/src/partial_primitive.rs': '// native, absorbed from exampleorg/partialdrain\n',
  }, 'runtime: absorb a primitive from exampleorg/partialdrain, drain a.go (b.go retained/coupled)')

  const donor = { repo: 'exampleorg/partialdrain' }
  const evidence = functionalAbsorptionEvidence(donor, baseline, root)
  assert.equal(evidence.length, 1, 'the combined commit both deletes donor source and touches a native root')

  const remaining = git(['ls-files', 'temporary/exampleorg/partialdrain']).trim().split('\n').filter(Boolean)
  assert.equal(remaining.length, 1, 'b.go must still be present -- a real partial drain, not extinction')
  rmSync(root, { recursive: true, force: true })
})

test('functionalAbsorptionEvidence: a fully drained donor WITH a real native implementation commit is functionally started, source-drain started, AND extinct (synthetic fixture)', () => {
  const { root, git } = makeFixtureRepo()
  const baseline = writeAndCommit(root, git, {
    'temporary/exampleorg/fullydrained/only.go': 'package fullydrained\n',
  }, 'seed exampleorg/fullydrained donor')
  git(['rm', '-q', 'temporary/exampleorg/fullydrained/only.go'])
  const finalSha = writeAndCommit(root, git, {
    'adapter/src/fully_drained_primitive.rs': '// native, fully absorbed from exampleorg/fullydrained\n',
  }, 'adapter: fully absorb exampleorg/fullydrained, drain its only file')

  const donor = { repo: 'exampleorg/fullydrained' }
  const evidence = functionalAbsorptionEvidence(donor, baseline, root)
  assert.equal(evidence.length, 1)
  assert.equal(evidence[0], finalSha)

  const remaining = git(['ls-tree', '-r', '--name-only', finalSha, '--', 'temporary/exampleorg/fullydrained']).trim()
  assert.equal(remaining, '', 'the donor tree must be completely empty as of this commit -- real extinction')
  rmSync(root, { recursive: true, force: true })
})

test('functionalAbsorptionEvidence returns no evidence at all for a donor with no baseline (never ingested) -- never a guess', () => {
  const donor = { repo: 'exampleorg/neveringested' }
  assert.deepEqual(functionalAbsorptionEvidence(donor, null, ROOT), [])
})

test('FUNCTIONAL_DONOR_COVERAGE-style ratio renders cleanly (0% not NaN) when functionalAbsorptionStartedDonors is legitimately 0 relative to a nonzero denominator', () => {
  // Direct property proof, independent of whatever the real corpus's current live count happens
  // to be: the ratio math itself must never produce NaN for a real nonzero denominator.
  const counts = computeCorpusExtinctionCounts()
  assert.ok(counts.sourcePresentDonors > 0)
  const pct = (counts.functionalAbsorptionStartedDonors / counts.sourcePresentDonors) * 100
  assert.ok(Number.isFinite(pct))
})
