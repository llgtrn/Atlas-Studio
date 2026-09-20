#!/usr/bin/env node
// sync-anchor-v2-parser.mjs — CLI for the atom-level sync-anchor-v2 parser
// (docs/doctrines/023-five-dimension-cloud-shard-contract.md, marked NOT_BUILT there).
//
// Usage:
//   node tools/reconcile/sync-anchor-v2-parser.mjs                 # diff mode: crates touched vs base
//   node tools/reconcile/sync-anchor-v2-parser.mjs --all           # full-repo sweep (baseline runs)
//   node tools/reconcile/sync-anchor-v2-parser.mjs --crate chronica-cli [--crate ...]
//   node tools/reconcile/sync-anchor-v2-parser.mjs --base origin/main
//   node tools/reconcile/sync-anchor-v2-parser.mjs --json          # machine-readable stdout
//   node tools/reconcile/sync-anchor-v2-parser.mjs --out path.json # write the JSON report to disk
//                                                                   #   (with a provenance block:
//                                                                   #   generated_at, repo_commit,
//                                                                   #   tool_schema_version)
//   node tools/reconcile/sync-anchor-v2-parser.mjs --summary-out path.md  # bounded Markdown summary
//   node tools/reconcile/sync-anchor-v2-parser.mjs --report        # never exit non-zero for DATA findings
//
// --report is honest about the difference between a DATA finding and an OPERATIONAL failure:
//   - DATA findings about a crate's own shard CONTENT -- a per-capability DISAGREE, a malformed
//     JSONL line (SHARD_PARSE_ERROR), a structurally invalid record (SHARD_RECORD_INVALID), or a
//     duplicate capability_key (SHARD_CAPABILITY_KEY_DUPLICATE) -- are exactly what --report
//     exists to make non-blocking: this process never throws on them, and --report makes them
//     exit 0 while still printing/writing every one of them in full.
//   - OPERATIONAL failures -- an unknown CLI argument, a flag missing its required value, an
//     unresolvable diff base (NO crate was scanned at all), a workspace-member Cargo.toml path
//     that would escape the repo root (traversal), a shard/source-file listing the tool refused
//     to read/process in full (unreadable, exceeds a size bound, or a per-crate source-file count
//     was truncated), or a failure to WRITE the --out/--summary-out report itself -- mean the
//     tool could not run/scan as configured, not "here is a disagreement to note and move past".
//     These remain a structured, bounded, non-zero exit REGARDLESS of --report, and never surface
//     as an uncaught stack trace.
//
// Exit codes: 0 success (or --report absorbing DATA findings only); 1 DATA findings present and
// not --report, OR any operational finding present (summary.operational_finding_count > 0)
// REGARDLESS of --report; 2 CLI usage/configuration error (unknown argument, missing flag value,
// unresolvable diff base with no --all/--crate scope -- ALWAYS this code, never suppressed by
// --report, since zero crates were scanned); 3 could not write --out/--summary-out; 4 unexpected
// internal error (defense-in-depth backstop -- everything reachable from real input is expected
// to be handled by one of the above, never by this catch-all).
//
// Every diagnostic string this CLI prints or writes falls into one of two sanitization tiers
// (sync-anchor-v2-sanitize.mjs, re-exported via sync-anchor-v2-lib.mjs): operator-supplied CLI
// values (--base/--out/--summary-out) have no shape this tool can prove safe in advance and go
// through `redactValue`, which ALWAYS replaces them with the same fixed static token (no length,
// no hash, no fingerprint -- round 5's length+sha256 format was found to be dictionary-attackable
// for short/low-entropy values and replaced in round 6), no conditional passthrough (repair round
// 5, item 1: a successful-write confirmation log must not echo the path it just wrote, same as any
// other free-form/untrusted field). Genuinely free-form diagnostic text with no provable-safe
// shape (a git error, a filesystem error, an unexpected exception's message) goes through
// `sanitizeErrorForDisplay`, which NEVER echoes any of the original text -- only the caller's
// fixed category plus that same static token. Shard-derived text (capability_key, target_module,
// status, crate identity) is sanitized once, centrally, in sync-anchor-v2-lib.mjs's verifyCrate so
// every consumer (human summary, --json, --out, --summary-out) inherits the same guarantee.
import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname } from 'node:path'
import { discoverWorkspaceCrates, redactValue, sanitizeCrateName, sanitizeErrorForDisplay, verifyCrates } from './sync-anchor-v2-lib.mjs'
import { renderHumanSummary, renderMarkdownSummary } from './sync-anchor-v2-report.mjs'

// Bumped whenever the summary/report JSON *shape* changes (field renames, new top-level keys) --
// not on every classification-logic tweak. v2 renamed shard_parse_errors/shard_parse_error_count
// to shard_findings/shard_finding_count (now covering SHARD_RECORD_INVALID and
// SHARD_CAPABILITY_KEY_DUPLICATE too, not just SHARD_PARSE_ERROR) and added
// workspace_findings/workspace_finding_count. v3 added operational_findings/
// operational_finding_count (unifying workspace escapes, unreadable/oversized/truncated shard or
// source-file input) and sanitized capability_key/target_module/status in every result record.
// Repair round 5 widened the operational-finding reason vocabulary and the sanitization model
// (redactValue/sanitizeEnumField/sanitizeCrateName/sanitizePathField replacing sanitizeDisplayField)
// but changed no field's SHAPE, so the version stays at /3 per this policy.
const REPORT_SCHEMA_VERSION = 'sync-anchor-v2-report/3'

// stdio: ['ignore', 'pipe', 'pipe'] is required on every execFileSync('git', ...) call in this file:
// Node's execFileSync, left to its own default stdio, still WRITES the child's stderr straight to
// this process' own stderr (in addition to capturing it on error.stderr for the catch block) --
// entirely bypassing sanitizeErrorForDisplay/redactValue no matter how carefully the catch
// branch is written. Explicitly piping stderr is what actually stops git's raw text from reaching
// the terminal/CI log unsanitized (item 3, repair round 4).
const GIT_STDIO = ['ignore', 'pipe', 'pipe']

function currentCommit(root) {
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8', stdio: GIT_STDIO }).trim()
  } catch {
    return null
  }
}

/** Provenance is attached only to a persisted `--out` artifact (a baseline/cloud-report file
 * meant to be read later, out of its generating process's context) -- never to bare `--json`
 * stdout, which stays byte-for-byte deterministic across repeated invocations of the same input
 * for programmatic/CI consumption and test assertions. */
function withProvenance(root, summary) {
  return {
    ...summary,
    provenance: {
      generated_at: new Date().toISOString(),
      repo_commit: currentCommit(root),
      tool_schema_version: REPORT_SCHEMA_VERSION,
    },
  }
}

class UsageError extends Error {}
class ReportWriteError extends Error {
  constructor(path, cause) {
    // cause.code is a short, fixed OS error code (e.g. "ENOTDIR", "EACCES") -- a small, known-safe
    // vocabulary from Node's errno list, never attacker/shard text -- so it is shown as-is when
    // present. The cause.message fallback is a value-free category: an arbitrary fs error message
    // cannot be proven safe by any length/control-character/secret-shape check, so it is NEVER
    // echoed, not even a prefix (item 3, repair round 4). `path` is operator-supplied (a --out/
    // --summary-out CLI flag) and is ALWAYS fully redacted via redactValue, never echoed even when
    // short/printable-looking (repair round 5, item 1).
    super(`could not write report to "${redactValue(path)}": ${cause.code ?? sanitizeErrorForDisplay('REPORT_WRITE_FAILED', cause.message)}`)
    this.path = path
  }
}

const FLAGS_WITH_VALUE = new Set(['--crate', '--base', '--out', '--summary-out', '--expect-commit'])

function parseArgs(argv) {
  const opts = { all: false, crates: [], base: null, json: false, report: false, out: null, summaryOut: null, expectCommit: null }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--all') {
      opts.all = true
      continue
    }
    if (arg === '--json') {
      opts.json = true
      continue
    }
    if (arg === '--report') {
      opts.report = true
      continue
    }
    if (FLAGS_WITH_VALUE.has(arg)) {
      const value = argv[i + 1]
      if (value === undefined || value.startsWith('--')) {
        // The flag name itself (`--crate`, `--base`, ...) is one of this CLI's own fixed,
        // known-safe strings and is shown as-is; argv[i+1] (or its absence) is caller-supplied
        // and is never echoed -- a value-free category plus position/length/hash only (item 3).
        throw new UsageError(`"${arg}" requires a value: ${sanitizeErrorForDisplay(`MISSING_FLAG_VALUE@${i + 1}`, value ?? '')}`)
      }
      i += 1
      if (arg === '--crate') opts.crates.push(value)
      else if (arg === '--base') opts.base = value
      else if (arg === '--out') opts.out = value
      else if (arg === '--summary-out') opts.summaryOut = value
      else if (arg === '--expect-commit') opts.expectCommit = value
      continue
    }
    // The unrecognized argument itself is caller-supplied and never echoed -- a value-free
    // category plus its argv position/length/hash only (item 3, repair round 4).
    throw new UsageError(sanitizeErrorForDisplay(`UNKNOWN_ARGUMENT@${i}`, arg))
  }
  return opts
}

function resolveBaseRef(explicit) {
  if (explicit) return explicit
  if (process.env.GITHUB_BASE_REF) return `origin/${process.env.GITHUB_BASE_REF}`
  return 'origin/main'
}

function gitDiffFiles(root, base) {
  try {
    return execFileSync('git', ['diff', '--name-only', `${base}...HEAD`], { cwd: root, encoding: 'utf8', stdio: GIT_STDIO })
      .split('\n')
      .filter(Boolean)
  } catch (error) {
    // error.message here is git's own stderr text -- free-form and never provably safe -- so it is
    // value-free-categorized (never echoed, not even a prefix) per item 3, repair round 4. `base` is
    // operator-supplied (a --base flag or resolved ref) and sanitized the same way as any other
    // high-risk display field.
    console.error(`sync:verify-atoms: could not diff against "${redactValue(base)}": ${sanitizeErrorForDisplay('GIT_DIFF_FAILED', error.message)}`)
    return null
  }
}

function touchedCrateNames(files, crates) {
  const liveNames = new Set(crates.map((c) => c.name))
  const touched = new Set()
  for (const file of files) {
    // 2026-09-16 hard refoundation: crates/<name>/ no longer exists (crates/ is retired -- see
    // docs/decisions/0016-legacy-backend-retired-to-git-history.md). Each live crate now owns its
    // own top-level directory directly (crate.crate_path, e.g. "core", "runtime"), not a
    // crates/<name>/ wrapper, so match against the crate's real discovered path instead of a
    // hardcoded prefix.
    const crate = crates.find((c) => file === c.crate_path || file.startsWith(`${c.crate_path}/`))
    if (crate) touched.add(crate.name)
    // crate atlas docs live under docs/crates/ (see tools/_paths.mjs); the docs/_generated/
    // and unprefixed docs/ forms are still matched for diffs that span either migration
    // commit (docs/_generated/ -> docs/crates/, and the original docs/ -> docs/_generated/).
    const docMatch = file.match(/^docs\/(?:crates\/|_generated\/)?\d{3}-crate-(chronica-[a-z0-9-]+)\.md$/)
    if (docMatch && liveNames.has(docMatch[1])) touched.add(docMatch[1])
  }
  return [...touched].sort()
}

/** Writes `content` to `path`, creating parent directories as needed. Any failure (unwritable
 * directory, permission error, an invalid/traversal-shaped path the filesystem rejects) is
 * converted to a ReportWriteError -- the caller turns this into a structured, bounded, non-zero
 * exit, never an uncaught stack trace. */
function writeReportFile(path, content) {
  try {
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, content)
  } catch (error) {
    throw new ReportWriteError(path, error)
  }
}

/** On a no-scan operational failure (the diff base could not be resolved, so NO crate was ever
 * scanned), still write a minimal, honestly-labeled failure artifact to --out/--summary-out if
 * either was requested -- a CI consumer that unconditionally reads the report path should see a
 * truthful "this did not run" record, not a missing file and not a stale/misleading prior report. */
function writeBaseRefFailureReports(opts, base) {
  if (opts.out) {
    const failure = { ok: false, error: 'BASE_REF_UNAVAILABLE', base_ref: redactValue(base), scanned: false }
    writeReportFile(opts.out, `${JSON.stringify(failure, null, 2)}\n`)
  }
  if (opts.summaryOut) {
    writeReportFile(
      opts.summaryOut,
      `### sync-anchor-v2 atom parser: FAILED (no scan)\n\nCould not resolve diff base \`${redactValue(base)}\`; ` +
        `no crate was scanned. Pass --all, --crate <name>, or --base <ref>.\n`,
    )
  }
}

function resolveTargetCrates(root, opts, crates) {
  if (opts.all) return crates.map((c) => c.name)
  if (opts.crates.length) return opts.crates

  const base = resolveBaseRef(opts.base)
  const files = gitDiffFiles(root, base)
  if (files === null) {
    // This is a NO-SCAN operational failure, never a DATA finding: zero crates were examined, so
    // there is nothing for --report to legitimately absorb. Converting this into exit 0 under
    // --report would let a broken/misconfigured scan look identical to "0 crates touched, all
    // clear" -- silently converting "the tool did not run" into "success". Always non-zero,
    // regardless of --report (item 3 of the repair-round-3 audit).
    console.error(`sync:verify-atoms: base ref unavailable; pass --all, --crate <name>, or --base <ref>. No crate was scanned.`)
    writeBaseRefFailureReports(opts, base)
    process.exitCode = 2
    return null
  }
  const targetCrates = touchedCrateNames(files, crates)
  // `base` reaches this success-path log the same way it reaches the failure paths above (operator
  // --base flag, or GITHUB_BASE_REF/origin/main) -- sanitized for defense-in-depth consistency
  // across every human-readable surface, not just the error ones (item 3/4, repair round 4).
  const safeBase = redactValue(base)
  if (!targetCrates.length) {
    console.log(`sync:verify-atoms: no crates/** or crate-doc changes vs ${safeBase}; nothing to verify.`)
  } else {
    // targetCrates here are already cross-validated against the real discovered workspace crate
    // list (touchedCrateNames only returns names present in `liveNames`), but sanitizeCrateName is
    // still applied for defense-in-depth consistency with every other crate-identity display site
    // (repair round 5, item 1: "Centrally sanitize crate/targetCrates/... fields").
    console.log(`sync:verify-atoms: diffing vs ${safeBase} -> ${targetCrates.length} crate(s) touched: ${targetCrates.map(sanitizeCrateName).join(', ')}`)
  }
  return targetCrates
}

function run() {
  const root = process.cwd()
  const opts = parseArgs(process.argv.slice(2))
  const crates = discoverWorkspaceCrates(root)
  if (opts.expectCommit && currentCommit(root) !== opts.expectCommit) throw new UsageError('EXPECTED_COMMIT_MISMATCH')

  const targetCrates = resolveTargetCrates(root, opts, crates)
  if (targetCrates === null) return // resolveTargetCrates already set process.exitCode

  const summary = verifyCrates(root, targetCrates)

  if (opts.json) console.log(JSON.stringify(summary, null, 2))
  else for (const line of renderHumanSummary(summary)) console.log(line)

  // A successful-write confirmation log must not echo the --out/--summary-out path it just wrote
  // (repair round 5, item 1): the path is operator-supplied CLI input with no shape this tool can
  // prove safe, so it is always fully redacted here too, exactly like every failure-path message
  // that already redacted it -- "it worked" is not a reason to relax the same guarantee.
  if (opts.out) {
    writeReportFile(opts.out, `${JSON.stringify(withProvenance(root, summary), null, 2)}\n`)
    console.log(`sync:verify-atoms: wrote report to ${redactValue(opts.out)}`)
  }
  if (opts.summaryOut) {
    writeReportFile(opts.summaryOut, renderMarkdownSummary(summary))
    console.log(`sync:verify-atoms: wrote Markdown summary to ${redactValue(opts.summaryOut)}`)
  }

  // Operational findings (a workspace-member path that would escape the repo root, a shard this
  // tool refused to read/process because it was unreadable or exceeded a size bound, a truncated
  // per-crate source-file listing, or an individual oversized source file) are never suppressed
  // by --report -- they mean the scan itself could not fully/safely run as configured over that
  // input, not "here is a disagreement to note and move past" (see the module doc comment).
  // summary.operational_finding_count is computed once, centrally, in verifyCrates -- this CLI no
  // longer hand-filters crates_skipped by reason string, which previously silently missed any new
  // operational reason a future change added (exactly the gap SOURCE_FILES_TRUNCATED/
  // SOURCE_FILE_OVERSIZED closed in this round).
  const operationalFindingCount = summary.operational_finding_count
  // DATA findings (per-capability disagreements, and shard-content structural findings: malformed
  // JSON / invalid record / duplicate key) are exactly what --report exists to make non-blocking.
  const dataFindingCount = summary.disagree_count + summary.shard_finding_count

  if (operationalFindingCount > 0) process.exitCode = 1
  else if (dataFindingCount > 0 && !opts.report) process.exitCode = 1
}

function main() {
  try {
    run()
  } catch (error) {
    if (error instanceof UsageError) {
      console.error(`sync:verify-atoms: ${error.message}`)
      process.exitCode = 2
      return
    }
    if (error instanceof ReportWriteError) {
      console.error(`sync:verify-atoms: ${error.message}`)
      process.exitCode = 3
      return
    }
    // Defense-in-depth backstop: every reachable failure mode above is expected to be handled by
    // one of the structured branches; this never re-throws a raw stack trace to the terminal/CI
    // log for an unforeseen failure, and always exits non-zero regardless of --report (an
    // unexpected internal error is not a DATA finding). An unforeseen error's message could in
    // principle wrap/echo arbitrary shard or secret-shaped content and cannot be proven safe by any
    // shape check, so it is value-free-categorized -- never echoed, not even a prefix (item 3,
    // repair round 4).
    console.error(`sync:verify-atoms: unexpected internal error: ${sanitizeErrorForDisplay('UNEXPECTED_INTERNAL_ERROR', error?.message ?? String(error))}`)
    process.exitCode = 4
  }
}

main()
