#!/usr/bin/env node
// Donor absorption burn-down (CHRONICA CORPUS MASTER CHECKLIST directive, section 17A/17B,
// 2026-09-16). Progress is derived from Git/filesystem reality, never manually persisted:
//
//   node tools/refoundation/donor-burndown.mjs --summary          # global corpus + drain report
//   node tools/refoundation/donor-burndown.mjs --donor <slug>     # one donor's burn-down
//   node tools/refoundation/donor-burndown.mjs --stalled          # ABSORPTION_STALL detector
//   node tools/refoundation/donor-burndown.mjs --doc-churn        # DOCUMENTATION_CHURN detector
//
// tools/refoundation/donor-corpus.yaml stores STABLE identity/classification only (repo, family,
// needed, temporary_path, status, absorption_state). It never stores remaining_files/drain_percent
// -- those are computed here, every run, from `git ls-tree`/`git ls-files`/`git log` against the
// real repository. There is no per-file manual tracking database (no donor-files.json /
// absorption-files.json / progress.json) -- Git already knows what existed, what exists, and which
// commit changed it.
//
// NATIVE_FILES_CHANGED / TESTS_CHANGED are NOT retroactively mined from history here -- even now
// that real absorption commits exist (e.g. e2620a246f), heuristically correlating a core/runtime/
// adapter/organism commit to a specific donor slug after the fact stays unreliable in general.
// Per section M, each absorption commit self-reports those fields in its own commit message; this
// tool's job is only the objective, mechanically-derivable donor-source-tree measurement.
import { execFileSync } from 'node:child_process'
import { lstatSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { parse as parseYaml } from 'yaml'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')
const CHECKLIST_PATH = join(ROOT, 'tools/refoundation/donor-corpus.yaml')

function git(args, root = ROOT) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 })
}

function loadDonors() {
  const doc = parseYaml(readFileSync(CHECKLIST_PATH, 'utf8'))
  return doc.donors
}

/** The commit that first introduced this donor's source path -- the acquisition baseline. */
function baselineCommit(path) {
  const out = git(['log', '--format=%H', '--follow', '--', path]).trim().split('\n').filter(Boolean)
  return out.length ? out[out.length - 1] : null
}

function treeFileCountAndBytes(commit, path) {
  let out
  try {
    out = git(['ls-tree', '-r', '-l', commit, '--', path])
  } catch {
    return { files: 0, bytes: 0 }
  }
  let files = 0
  let bytes = 0
  for (const line of out.split('\n')) {
    if (!line.trim()) continue
    const parts = line.split(/\s+/)
    const size = Number(parts[3])
    if (!Number.isNaN(size)) bytes += size
    files++
  }
  return { files, bytes }
}

function workingTreeFileCountAndBytes(path) {
  let files
  try {
    files = git(['ls-files', '--', path]).split('\n').filter(Boolean)
  } catch {
    return { files: 0, bytes: 0 }
  }
  let bytes = 0
  for (const f of files) {
    try {
      // lstatSync, not statSync: a tracked symlink's own blob content is the target path string
      // (what `git ls-tree -l` reports for the baseline side too), not the dereferenced target
      // file's size. Following the link here would inflate remaining_bytes by the size of
      // whatever the symlink happens to point to -- real donor trees contain these (e.g. a
      // `CLAUDE.md -> AGENTS.md` convention), and statSync's dereferenced size can make
      // remaining_bytes exceed baseline_bytes even after a real file was deleted.
      bytes += lstatSync(join(ROOT, f)).size
    } catch {
      // file listed by git but unreadable on disk (rare edge case) -- excluded from the byte sum,
      // not from the file count, so drain% still reflects real file removal.
    }
  }
  return { files: files.length, bytes }
}

function lastDeletionCommit(path) {
  const out = git(['log', '--diff-filter=D', '--format=%H %s', '-1', '--', path]).trim()
  return out || null
}

function commitsSince(commit, path) {
  if (!commit) return 0
  const out = git(['log', '--format=%H', `${commit}..HEAD`, '--', path]).trim()
  return out ? out.split('\n').length : 0
}

const NATIVE_PREFIXES = ['core/', 'runtime/', 'adapter/', 'organism/']

/** Commits since baseline whose message mentions this donor, found repo-wide (not scoped to the
 * donor's own temporary_path -- a real absorption commit touches core/runtime/adapter/organism
 * *and* deletes donor source in the same commit, so it must be findable by donor-name mention
 * alone, not by path). */
function donorMentioningCommitsSince(baseline, donor) {
  if (!baseline) return []
  const needle = donor.repo.split('/').pop().replace(/[.*[\]^$\\]/g, '\\$&')
  const out = git(['log', '--format=%H', `--grep=${needle}`, '-i', `${baseline}..HEAD`]).trim()
  return out ? out.split('\n').filter(Boolean) : []
}

function commitChangedPaths(commit, root = ROOT) {
  const out = git(['show', '--name-only', '--format=', commit], root).trim()
  return out ? out.split('\n').filter(Boolean) : []
}

/** Real, Git-derived evidence that `donor` has contributed to canonical native Chronica
 * implementation (core/runtime/adapter/organism) since its own baseline -- the
 * FUNCTIONAL_ABSORPTION_STARTED signal (CHRONICA -- fix the FUNCTIONAL ABSORPTION != SOURCE
 * DELETION defect, 2026-09-17). This is a DIFFERENT fact from source drain: a donor can
 * contribute several real native primitives while its own source tree stays fully intact (e.g.
 * postgres/postgres today -- core::Revision/runtime::world::WorldTransaction/
 * adapter::postgres::PostgresWorldStore/adapter::wal_lsn all came from it, yet its one real
 * source deletion was later safely reverted because retained donor source elsewhere still
 * referenced it, so remaining_files == baseline_files again). Conflating "drained_files > 0" with
 * "produced real native behavior" was exactly the bug this function exists to fix.
 *
 * Deliberately matches the donor's FULL "owner/repo" slug, not `donorMentioningCommitsSince`'s
 * bare trailing-segment match (that function's own, looser heuristic, kept unchanged for
 * `docChurnSignal`'s different purpose): a bare repo name can collide with an unrelated, generic
 * technical term this codebase now uses pervasively as infrastructure vocabulary. "postgres" is
 * the standout real case -- it appears in dozens of commits merely because `PostgresWorldStore`
 * is the durable-storage backend every kernel proof in this workspace already uses, not because
 * each of those commits absorbed anything FROM the postgres donor. Requiring the full slug (43
 * bare "postgres" matches narrow to 8 full-slug matches in this real corpus) and a genuine
 * native-root touch together cut that noise dramatically, though not to zero -- this is
 * deliberately the CONSERVATIVE side of the tradeoff the loop directive asks for: it may
 * occasionally attribute a commit to the wrong donor when two donors' absorption work touches the
 * same shared infrastructure in the same window, but it can never report `true` from zero real
 * native-code evidence, and a commit that only deletes donor source, or only touches
 * tools/docs/metadata, or is the donor's own acquisition commit, never counts. A donor with no
 * baseline (never ingested) returns no evidence at all -- never a guessed answer.
 *
 * `root` defaults to the real Chronica checkout; tests pass an isolated synthetic git repo (the
 * same pattern `donor-coupling-gate.mjs::findDonorCouplingViolations` already uses) so the four
 * real distinct cases (functional-only, drain-only, both-partial, both-extinct) can each be
 * proven precisely rather than hunted for in real corpus history. */
export function functionalAbsorptionEvidence(donor, baseline, root = ROOT) {
  if (!baseline) return []
  const slug = donor.repo.replace(/[.*[\]^$\\]/g, '\\$&')
  const out = git(['log', '--format=%H', `--grep=${slug}`, '-i', `${baseline}..HEAD`], root).trim()
  const mentioning = out ? out.split('\n').filter(Boolean) : []
  return mentioning.filter((commit) => {
    const paths = commitChangedPaths(commit, root)
    return paths.some((p) => NATIVE_PREFIXES.some((prefix) => p.startsWith(prefix)))
  })
}

/** Real, Git-derived DOCUMENTATION_CHURN_CANDIDATE signal (section 12): among the commits that
 * mention this donor since its baseline, how many touched neither native code (core/runtime/
 * adapter/organism) nor the donor's own source tree at all -- i.e. commits about the donor that
 * moved no real work. A warning signal, not a persisted state. */
export function docChurnSignal(donor, baseline) {
  const mentioning = donorMentioningCommitsSince(baseline, donor)
  if (mentioning.length === 0) return { mentioning_commits: 0, churn_only_commits: 0, candidate: false }
  let churnOnly = 0
  for (const commit of mentioning) {
    const paths = commitChangedPaths(commit)
    const touchesRealWork = paths.some((p) => NATIVE_PREFIXES.some((prefix) => p.startsWith(prefix)) || p.startsWith(donor.temporary_path))
    if (!touchesRealWork) churnOnly += 1
  }
  // Conservative: only flag when churn-only commits actually dominate (>= 2 and all of them),
  // never from a single commit or a minority pattern.
  const candidate = mentioning.length >= 2 && churnOnly === mentioning.length
  return { mentioning_commits: mentioning.length, churn_only_commits: churnOnly, candidate }
}

// Derived, never persisted (CHRONICA -- GIT IS THE ABSORPTION LEDGER, section 9):
//   baseline == 0                    -> NOT_INGESTED       (no baseline tree could be found at all)
//   0 < current == baseline          -> INGESTED_NOT_STARTED
//   0 < current < baseline           -> ACTIVE_ABSORPTION
//   baseline > 0 && current == 0     -> FULLY_DRAINED
export function derivedState(baselineFiles, remainingFiles) {
  if (baselineFiles === 0) return 'NOT_INGESTED'
  if (remainingFiles === 0) return 'FULLY_DRAINED'
  if (remainingFiles === baselineFiles) return 'INGESTED_NOT_STARTED'
  return 'ACTIVE_ABSORPTION'
}

export function computeDonorProgress(donor) {
  if (!donor.source_present || !donor.temporary_path) return null
  const path = donor.temporary_path
  const baseline = baselineCommit(path)
  const baselineStat = baseline ? treeFileCountAndBytes(baseline, path) : { files: 0, bytes: 0 }
  const current = workingTreeFileCountAndBytes(path)
  const drainedFiles = Math.max(0, baselineStat.files - current.files)
  const drainedBytes = Math.max(0, baselineStat.bytes - current.bytes)
  const lastDeletion = lastDeletionCommit(path)
  const functionalEvidence = functionalAbsorptionEvidence(donor, baseline)
  return {
    repo: donor.repo,
    temporary_path: path,
    baseline_commit: baseline,
    baseline_files: baselineStat.files,
    remaining_files: current.files,
    drained_files: drainedFiles,
    drain_percent: baselineStat.files ? (drainedFiles / baselineStat.files) * 100 : 0,
    baseline_bytes: baselineStat.bytes,
    remaining_bytes: current.bytes,
    drained_bytes: drainedBytes,
    byte_drain_percent: baselineStat.bytes ? (drainedBytes / baselineStat.bytes) * 100 : 0,
    last_deletion_commit: lastDeletion,
    commits_since_baseline: commitsSince(baseline, path),
    fully_drained: baselineStat.files > 0 && current.files === 0,
    state: derivedState(baselineStat.files, current.files),
    // FUNCTIONAL_ABSORPTION_STARTED: a fact independent of source_drain_started/fully_drained --
    // see functionalAbsorptionEvidence's own doc. Never derived from drained_files.
    functional_absorption_started: functionalEvidence.length > 0,
    functional_absorption_evidence_commits: functionalEvidence,
    // SOURCE_DRAIN_STARTED: the fact the old, now-fixed `absorptionStartedDonors` wrongly reused
    // as a proxy for functional absorption. Kept, named honestly, and used for its own real
    // purpose only.
    source_drain_started: drainedFiles > 0,
  }
}

function fmtPct(n) {
  return `${n.toFixed(2)}%`
}

function printDonorReport(p) {
  console.log(`DONOR: ${p.repo}  (${p.temporary_path})`)
  console.log(`  Baseline files:       ${p.baseline_files}`)
  console.log(`  Remaining files:      ${p.remaining_files}`)
  console.log(`  Files drained:        ${p.drained_files}`)
  console.log(`  Drain:                ${fmtPct(p.drain_percent)}`)
  console.log('')
  console.log(`  Baseline bytes:       ${p.baseline_bytes}`)
  console.log(`  Remaining bytes:      ${p.remaining_bytes}`)
  console.log(`  Byte drain:           ${fmtPct(p.byte_drain_percent)}`)
  console.log('')
  console.log(`  Last deletion commit: ${p.last_deletion_commit || '(none yet)'}`)
  console.log(`  Commits since baseline touching this path: ${p.commits_since_baseline}`)
}

// Scanning the full corpus is genuinely expensive (`computeDonorProgress` runs several real `git`
// subprocesses per donor, and some donor trees -- clickhouse, infisical, tooljet -- are gigabytes
// with deep history), so a single process invocation memoizes the one full pass rather than
// re-walking the same immutable-for-this-process Git state every time a caller asks for it. This
// is an in-memory cache for the lifetime of ONE CLI invocation only -- never written to disk,
// never read back by a later invocation, so it introduces no second persisted truth (section 2:
// "do not persist dynamic absorption state").
let cachedSourcePresentProgresses = null

/** Every source-present donor's real, Git-derived progress -- the one place `buildSummaryLines`
 * and the corpus-level extinction/file-total metrics below all read from, so none of them
 * reimplements or re-derives donor state on its own, and none of them re-scans the corpus if a
 * caller already asked for it once in this same process. */
function sourcePresentProgresses() {
  if (cachedSourcePresentProgresses) return cachedSourcePresentProgresses
  const donors = loadDonors()
  const sourcePresent = donors.filter((d) => d.source_present && d.temporary_path)
  cachedSourcePresentProgresses = { donors, sourcePresent, progresses: sourcePresent.map((d) => computeDonorProgress(d)) }
  return cachedSourcePresentProgresses
}

/** Corpus-wide donor counts across FOUR distinct facts (CHRONICA -- fix the FUNCTIONAL ABSORPTION
 * != SOURCE DELETION defect, 2026-09-17; originally CHRONICA -- MAKE ABSORPTION RATIOS + DONOR
 * EXTINCTION A REAL RUNTIME REQUIREMENT, section 4). Every count here is derived from
 * `computeDonorProgress` (itself Git/tree-derived, never a manually-maintained `status` field) --
 * no new model, no second truth. These four facts are NEVER conflated with one another:
 *
 *   1. INGESTED                     donor source exists in temporary/ (sourcePresentDonors)
 *   2. FUNCTIONAL_ABSORPTION_STARTED  real donor-derived behavior materialized into
 *                                      core/runtime/adapter/organism (functionalAbsorptionStartedDonors)
 *   3. SOURCE_DRAIN_STARTED         at least one donor source file has safely disappeared
 *                                      (sourceDrainStartedDonors)
 *   4. FULLY_DRAINED / EXTINCT      baseline_files > 0 && remaining_files == 0 (fullyDrainedDonors)
 *
 * A donor can be functionally-started with zero source drain (postgres/postgres today -- see
 * `functionalAbsorptionEvidence`'s own doc) or, in principle, source-drain-started before any
 * commit ever produced real native code (a deletion alone proves nothing functional) -- this
 * function keeps both possibilities representable rather than collapsing them into one count, the
 * exact defect this fixes. `fullyDrainedDonors` reuses `computeDonorProgress`'s own
 * `fully_drained` field verbatim (baseline_files > 0 && remaining_files === 0), so a donor with
 * ANY retained/coupled source, however small, is never counted extinct.
 */
export function computeCorpusExtinctionCounts() {
  const { donors, sourcePresent, progresses } = sourcePresentProgresses()
  const functionalAbsorptionStartedDonors = progresses.filter((p) => p.functional_absorption_started).length
  const sourceDrainStartedDonors = progresses.filter((p) => p.source_drain_started).length
  const sourceDrainNotStartedDonors = progresses.filter((p) => !p.source_drain_started).length
  const fullyDrainedDonors = progresses.filter((p) => p.fully_drained).length
  return {
    totalDonors: donors.length,
    sourcePresentDonors: sourcePresent.length,
    notIngestedDonors: donors.length - sourcePresent.length,
    functionalAbsorptionStartedDonors,
    sourceDrainStartedDonors,
    sourceDrainNotStartedDonors,
    fullyDrainedDonors,
  }
}

/** Corpus-wide donor SOURCE FILE totals (section 5) -- the same `baseline_files`/`remaining_files`
 * `buildSummaryLines` already prints, exposed as plain numbers rather than parsed back out of
 * rendered text, for callers (e.g. `absorption-ratio-report.mjs`) that need the raw values. */
export function computeCorpusFileTotals() {
  const { progresses } = sourcePresentProgresses()
  const baselineDonorFiles = progresses.reduce((s, p) => s + p.baseline_files, 0)
  const remainingDonorFiles = progresses.reduce((s, p) => s + p.remaining_files, 0)
  return {
    baselineDonorFiles,
    remainingDonorFiles,
    drainedDonorFiles: baselineDonorFiles - remainingDonorFiles,
  }
}

export function buildSummaryLines() {
  const { donors, sourcePresent, progresses } = sourcePresentProgresses()

  const totalBaselineFiles = progresses.reduce((s, p) => s + p.baseline_files, 0)
  const totalRemainingFiles = progresses.reduce((s, p) => s + p.remaining_files, 0)
  const totalDrainedFiles = totalBaselineFiles - totalRemainingFiles

  // Source-drain activity only -- kept distinct from FUNCTIONAL absorption below (never conflate
  // "a file disappeared" with "real native behavior was produced").
  const sourceDrainActive = progresses.filter((p) => p.source_drain_started && !p.fully_drained)
  const sourceDrainNotStarted = progresses.filter((p) => !p.source_drain_started)
  const fullyDrained = progresses.filter((p) => p.fully_drained)
  const functionallyStarted = progresses.filter((p) => p.functional_absorption_started)

  const evaluated = donors.filter((d) => d.status !== 'DISCOVERED').length

  const lines = []
  lines.push('CHRONICA DONOR CORPUS')
  lines.push('')
  lines.push(`Unique donors              ${donors.length}`)
  lines.push('')
  lines.push(`Corpus classified          ${donors.length} / ${donors.length}     100.00%`)
  lines.push(`Evaluated (status set)     ${evaluated} / ${donors.length}     ${fmtPct((evaluated / donors.length) * 100)}`)
  lines.push('')
  lines.push(`Source present             ${sourcePresent.length}`)
  lines.push(`Functional absorption started  ${functionallyStarted.length}`)
  lines.push(`Source drain not started    ${sourceDrainNotStarted.length}`)
  lines.push(`Source drain active         ${sourceDrainActive.length}`)
  lines.push(`Fully drained               ${fullyDrained.length}`)
  lines.push(`Retired (status)            ${donors.filter((d) => d.status === 'RETIRED').length}`)
  lines.push('')
  lines.push(`Source baseline files      ${totalBaselineFiles}`)
  lines.push(`Source remaining files     ${totalRemainingFiles}`)
  lines.push(`Source files drained       ${totalDrainedFiles}`)
  lines.push('')
  lines.push(`SOURCE FILE DRAIN PROGRESS   ${fmtPct(totalBaselineFiles ? (totalDrainedFiles / totalBaselineFiles) * 100 : 0)}`)
  return lines
}

function summary() {
  for (const line of buildSummaryLines()) console.log(line)
}

function stalled() {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const flagged = []
  for (const d of donors) {
    const p = computeDonorProgress(d)
    // "Active" per the directive means the donor has received commits since baseline but the
    // source tree has not shrunk at all -- a real stall signal, not merely "not started yet"
    // (commits_since_baseline includes the baseline commit itself, hence > 1).
    if (p.commits_since_baseline > 1 && p.drained_files === 0) {
      flagged.push({ donor: d.repo, commits_since_baseline: p.commits_since_baseline, drained_files: p.drained_files })
    }
  }
  if (!flagged.length) {
    console.log('ABSORPTION_STALL: none detected.')
    return
  }
  for (const f of flagged) {
    console.log(`DONOR: ${f.donor}`)
    console.log(`  COMMITS SINCE BASELINE: ${f.commits_since_baseline}`)
    console.log(`  DONOR FILE DELTA: 0`)
    console.log('  RESULT: ABSORPTION_STALL')
    console.log('')
  }
}

function docChurn() {
  const donors = loadDonors().filter((d) => d.source_present && d.temporary_path)
  const candidates = []
  for (const d of donors) {
    const baseline = baselineCommit(d.temporary_path)
    const signal = docChurnSignal(d, baseline)
    if (signal.mentioning_commits > 0) {
      console.log(`DONOR: ${d.repo}`)
      console.log(`  Commits mentioning donor since baseline: ${signal.mentioning_commits}`)
      console.log(`  Of those, touching no native/donor path:  ${signal.churn_only_commits}`)
      console.log(`  RESULT: ${signal.candidate ? 'DOCUMENTATION_CHURN_CANDIDATE' : 'ok'}`)
      console.log('')
      if (signal.candidate) candidates.push(d.repo)
    }
  }
  if (!candidates.length) {
    console.log('DOCUMENTATION_CHURN: none detected across donors with commit activity since baseline.')
  }
}

function main() {
  const args = process.argv.slice(2)
  if (args.includes('--summary') || args.length === 0) {
    summary()
    return
  }
  const donorIdx = args.indexOf('--donor')
  if (donorIdx !== -1) {
    const slug = args[donorIdx + 1]
    const donors = loadDonors()
    const d = donors.find((x) => x.repo === slug || x.temporary_path === `temporary/${slug}` || x.id === slug)
    if (!d) {
      console.error(`donor-burndown: no donor matching "${slug}" (match by repo, temporary_path, or id)`)
      process.exitCode = 2
      return
    }
    const p = computeDonorProgress(d)
    if (!p) {
      console.log(`DONOR: ${d.repo} -- no source present (status: ${d.status}), nothing to measure.`)
      return
    }
    printDonorReport(p)
    return
  }
  if (args.includes('--stalled')) {
    stalled()
    return
  }
  if (args.includes('--doc-churn')) {
    docChurn()
    return
  }
  console.error('usage: donor-burndown.mjs [--summary | --donor <slug> | --stalled | --doc-churn]')
  process.exitCode = 2
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main()
