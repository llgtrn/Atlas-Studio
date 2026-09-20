#!/usr/bin/env node
// Serial canonical integration (CHRONICA -- DISPATCH-FIRST MASS ABSORPTION ENGINE, sections
// 11/33): Builders may work in parallel isolated worktrees/branches, but canonical HEAD only ever
// advances one candidate at a time. For each candidate branch, in the given order:
//   1. attempt a merge against the CURRENT head (not the stale BASE_SHA the branch started from)
//   2. on a clean merge, run the workspace's real verification (cargo test + the JS absorption
//      gates) against the merged tree
//   3. only on success does the merge commit land and HEAD advance; the next candidate is then
//      evaluated against that NEW head
// A conflicted or test-failing candidate is aborted (never left half-merged) and reported; it does
// not block later candidates in the list.
//
// Usage: node tools/refoundation/integrate-candidates.mjs <candidate1> [candidate2 ...]
//
// Each <candidate> is anything `git merge` accepts as a <commit> -- a branch name, a tag, or a
// bare commit SHA (e.g. from a detached worktree candidate produced without ever creating a named
// branch). This script never required a named branch specifically; `git merge`/`git diff` are
// ref-agnostic, so "main@BASE + several detached-worktree candidate SHAs, verified and integrated
// serially against current main" already works with zero changes to this file.
//
// This script performs real git/cargo operations -- it is proven by actual use (the first proof
// wave), not by a mocked unit-test harness, matching the directive's own "prefer simple
// deterministic tooling" instruction over building a simulated integration engine.
//
// CHRONICA -- MAKE ABSORPTION RATIOS + DONOR EXTINCTION A REAL RUNTIME REQUIREMENT (2026-09-17):
// this is now the DISPATCH -> BUILD -> VERIFY -> SERIAL INTEGRATE -> DONOR SAFETY GATES ->
// RATIO REPORT -> LOOP VERDICT -> NEXT LOOP loop's own SERIAL INTEGRATE + RATIO REPORT stages.
// `absorption-ratio-report.mjs` already existed and called itself "MANDATORY PER-LOOP RATIO
// REPORT," but nothing in this real loop mechanically invoked it -- mandatory in contract was not
// mandatory in runtime. `main` below now ALWAYS calls it exactly once after the serial-integration
// loop finishes (win or lose), building its LiveCounts purely from this invocation's own real
// candidate outcomes (`buildLiveCounts`) -- never a persisted or guessed value. A report that
// fails to generate does not roll back already-integrated candidates (their commits are already
// valid canonical history), but it DOES make `main` return non-zero and print
// `RATIO_REPORT_FAILURE`, so an orchestrator driving repeated loops must not silently start the
// next one until reporting works again.

import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { buildRatioReport } from './absorption-ratio-report.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..', '..')

// The four canonical roots this gate's own report also checks -- a real native-code touch is
// what LiveCounts.acceptedWithRealNativeCode measures, Git-derived from the actual merge commit,
// never self-reported by the candidate.
const NATIVE_PREFIXES = ['core/', 'runtime/', 'adapter/', 'organism/']

function git(args, opts = {}) {
  return execFileSync('git', args, { cwd: ROOT, encoding: 'utf8', ...opts })
}

function run(cmd, args) {
  execFileSync(cmd, args, { cwd: ROOT, stdio: 'inherit' })
}

function currentBranch() {
  return git(['rev-parse', '--abbrev-ref', 'HEAD']).trim()
}

function headSha() {
  return git(['rev-parse', 'HEAD']).trim()
}

function isWorkingTreeClean() {
  return git(['status', '--porcelain']).trim() === ''
}

/** Merges `branch` into the currently checked-out branch, then runs verification. On any
 * failure, aborts the merge (or, for a test failure after a clean merge, resets back to the
 * pre-merge head) so the working tree is never left in a half-integrated state. */
function integrateOne(branch) {
  const before = headSha()
  try {
    git(['merge', '--no-ff', '--no-commit', branch])
  } catch (err) {
    git(['merge', '--abort'])
    return { branch, before, result: 'CONFLICT', detail: err.message.split('\n')[0] }
  }

  const checks = [
    ['cargo', ['fmt', '--check']],
    ['cargo', ['check', '--workspace', '--all-targets', '--locked']],
    ['cargo', ['test', '--workspace', '--locked']],
    ['cargo', ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']],
    ['node', ['tools/system-atlas/validate-v2.mjs']],
    ['node', ['tools/refoundation/no-temporary-dependency-gate.mjs']],
    ['node', ['tools/refoundation/donor-coupling-gate.mjs', '--since', before]],
  ]
  for (const [cmd, args] of checks) {
    try {
      run(cmd, args)
    } catch {
      git(['reset', '--hard', before])
      return { branch, before, result: 'TEST_FAILURE', detail: `${cmd} ${args.join(' ')} failed against the merged tree` }
    }
  }

  git(['commit', '--no-edit', '-m', `merge: integrate absorption candidate ${branch}`])
  return { branch, before, result: 'INTEGRATED', detail: headSha() }
}

/** Whether `sha`'s own commit (not a whole range) touched a canonical native-code root -- the
 * Git-derived signal behind LiveCounts.acceptedWithRealNativeCode. Never a guess: a candidate
 * that only touched tests/docs/tooling is not "real native code," regardless of what its own
 * commit message claims. */
export function commitTouchedNativeCode(sha, root = ROOT) {
  const out = execFileSync('git', ['show', '--name-only', '--format=', sha], { cwd: root, encoding: 'utf8' }).trim()
  const paths = out ? out.split('\n').filter(Boolean) : []
  return paths.some((p) => NATIVE_PREFIXES.some((prefix) => p.startsWith(prefix)))
}

/** Classifies one INTEGRATED candidate's interaction with donor source, purely from its own merge
 * commit's diff against its pre-merge head -- the safeDrainClassification bucket
 * `absorption-ratio-report.mjs`'s own doc says "cannot be reconstructed from Git alone once a
 * rejected candidate's branch is deleted," but for an ACCEPTED candidate it very much can:
 *   - REFERENCE_ONLY: the commit deleted no file under temporary/ at all (donor source was read
 *     as reference material only, nothing was drained).
 *   - DRAINED: the commit deleted >=1 file under some temporary/<donor>/, and as of this commit
 *     that donor's ENTIRE remaining tree is empty -- this candidate's own deletions, on top of
 *     whatever drained before it, achieved that donor's full extinction.
 *   - RETAINED_COUPLED: the commit deleted >=1 donor file but left other files behind in that
 *     donor's tree -- a real, gate-checked partial drain, not a full extinction.
 * Never a guess or a candidate self-report: both the deletion and the resulting tree state are
 * read straight from the commit `integrateOne` itself just produced. */
export function classifyDonorDrain(before, sha, root = ROOT) {
  const out = execFileSync('git', ['diff', '--name-status', `${before}..${sha}`, '--', 'temporary/'], { cwd: root, encoding: 'utf8' }).trim()
  if (!out) return 'REFERENCE_ONLY'
  const donors = new Set()
  for (const line of out.split('\n')) {
    const [status, path] = line.split('\t')
    if (status !== 'D') continue
    const m = path.match(/^temporary\/([^/]+)\//)
    if (m) donors.add(m[1])
  }
  if (donors.size === 0) return 'REFERENCE_ONLY'
  for (const donor of donors) {
    const remaining = execFileSync('git', ['ls-tree', '-r', '--name-only', sha, '--', `temporary/${donor}`], { cwd: root, encoding: 'utf8' }).trim()
    if (!remaining) return 'DRAINED'
  }
  return 'RETAINED_COUPLED'
}

/** Builds `absorption-ratio-report.mjs`'s own LiveCounts shape purely from THIS invocation's real
 * candidate outcomes (`results`, one entry per branch `integrateOne` was given) -- never a
 * persisted or guessed value, matching the loop directive's own "Do not guess these values.
 * Derive them from actual candidate/integration outcomes" instruction. */
export function buildLiveCounts(results, root = ROOT) {
  const accepted = results.filter((r) => r.result === 'INTEGRATED')
  const rejected = results.filter((r) => r.result === 'TEST_FAILURE').length
  const conflicted = results.filter((r) => r.result === 'CONFLICT').length
  // A CONFLICT never reaches the check phase at all (the merge itself failed); every other
  // outcome (TEST_FAILURE, INTEGRATED) means a clean merge was actually submitted to verification.
  const verificationSubmitted = results.filter((r) => r.result !== 'CONFLICT').length

  const safeDrainClassification = { DRAINED: 0, RETAINED_COUPLED: 0, REFERENCE_ONLY: 0 }
  let acceptedWithRealNativeCode = 0
  for (const r of accepted) {
    if (commitTouchedNativeCode(r.detail, root)) acceptedWithRealNativeCode += 1
    const classification = classifyDonorDrain(r.before, r.detail, root)
    safeDrainClassification[classification] += 1
  }

  return {
    candidatesPlanned: results.length,
    buildersCompleted: results.length,
    accepted: accepted.length,
    rejected,
    conflicted,
    verificationSubmitted,
    verificationPassed: accepted.length,
    acceptedWithRealNativeCode,
    safeDrainClassification,
  }
}

/** The mandatory step this file's own module doc names: no serial-integration run is a complete
 * loop until this emits. Reuses `absorption-ratio-report.mjs::buildRatioReport` -- the SAME
 * reporter every other caller uses, never a duplicate ratio engine -- fed with LiveCounts derived
 * purely from `results` (see `buildLiveCounts`). Returns `{ ok: true, lines }` on success or
 * `{ ok: false, error }` on failure; never throws, so `main` can always decide what to print and
 * how to exit. */
export function emitRatioReportForLoop({ baseSha, finalSha, results, root = ROOT }) {
  try {
    const live = buildLiveCounts(results, root)
    const lines = buildRatioReport({ baseSha, finalSha, live })
    return { ok: true, lines }
  } catch (error) {
    return { ok: false, error }
  }
}

function main(argv = process.argv.slice(2)) {
  if (argv.length === 0) {
    console.error('usage: integrate-candidates.mjs <candidate1> [candidate2 ...]  (branch name, tag, or bare commit SHA)')
    return 2
  }
  const startBranch = currentBranch()
  if (startBranch === 'HEAD') {
    console.error('integrate-candidates: refusing to run from a detached HEAD -- check out the integration branch first')
    return 2
  }
  if (!isWorkingTreeClean()) {
    console.error('integrate-candidates: refusing to run -- the integration branch has uncommitted changes (git status is not clean).')
    console.error('A candidate failure resets with `git reset --hard`, which would silently discard that in-progress work. Commit or stash it first.')
    return 2
  }
  const baseSha = headSha()
  console.log(`Serial integration onto "${startBranch}", starting at ${baseSha}`)
  const results = []
  for (const branch of argv) {
    console.log(`\n--- integrating ${branch} against current HEAD (${headSha()}) ---`)
    const result = integrateOne(branch)
    results.push(result)
    console.log(`${result.result}: ${branch}${result.detail ? ` (${result.detail})` : ''}`)
  }
  console.log('\nSUMMARY:')
  for (const r of results) console.log(`  ${r.branch}: ${r.result}`)

  // MANDATORY: this loop is not complete until the ratio report has successfully emitted, win or
  // lose on the candidates themselves. Already-integrated commits are never rolled back merely
  // because report generation fails -- they are already valid canonical history -- but a failure
  // here still makes this process exit non-zero, so an orchestrator driving repeated loops must
  // not silently start the next one until reporting works again.
  const finalSha = headSha()
  console.log('\n--- CHRONICA ABSORPTION LOOP RATIO REPORT (mandatory) ---')
  const report = emitRatioReportForLoop({ baseSha, finalSha, results })
  if (!report.ok) {
    console.error(`RATIO_REPORT_FAILURE: ${report.error?.stack ?? report.error}`)
    console.error('Canonical HEAD already reflects every successfully integrated candidate above (not rolled back), but this loop is NOT complete: do not automatically start the next absorption loop until reporting works.')
    return 1
  }
  for (const line of report.lines) console.log(line)

  return results.some((r) => r.result !== 'INTEGRATED') ? 1 : 0
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exit(main())
