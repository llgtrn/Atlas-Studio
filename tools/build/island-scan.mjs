#!/usr/bin/env node
// island-scan.mjs — CI gate CLI for the reverse-Cargo-dependency / zero-caller
// "island" check (issue #1524). Read-only: runs `cargo metadata --no-deps`
// (no compilation, no network/registry access, no writes to Cargo.toml,
// docs/architecture.db, or docs/capabilities.db) and reports every workspace
// crate that has zero shipped reverse Cargo dependents and is not itself a
// shipped binary entry point.
//
// Usage:
//   node tools/build/island-scan.mjs                 # gate mode: exit 1 on any BLOCKED/stale crate
//   node tools/build/island-scan.mjs --report         # never exit 1; print full report only
//   node tools/build/island-scan.mjs --scan-only      # print the raw scan (no allowlist policy applied); deterministic, no wall-clock input
//   node tools/build/island-scan.mjs --today 2026-07-16   # pin "today" for allowlist expiry evaluation (tests / reproducibility)
//   node tools/build/island-scan.mjs --out <path>     # also write the full report JSON to <path>
//   node tools/build/island-scan.mjs --print-fingerprint <crate>   # print the canonical evidence_fingerprint for a currently-ISLAND crate, for authoring/renewing an allowlist entry
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  applyAllowlistPolicy,
  classifyWorkspace,
  computeIslandEvidenceFingerprint,
  ISLAND_SCAN_POLICY_VERSION,
  stableStringify,
  validateAllowlistDoc,
} from './island-lib.mjs'

// Resolved relative to this script's own location, NOT process.cwd() — the
// allowlist must load correctly regardless of the invoking shell's working
// directory (e.g. a pre-commit hook or a future wrapper run from a
// subdirectory), not only when invoked from the repo root as CI happens to do.
const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const ALLOWLIST_PATH = join(SCRIPT_DIR, 'island-scan-allowlist.json')
// Repo root is two levels up from tools/build/.
const REPO_ROOT = join(SCRIPT_DIR, '..', '..')

function parseArgs(argv) {
  const args = { report: false, scanOnly: false, today: null, out: null, printFingerprint: null }
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i]
    if (a === '--report') args.report = true
    else if (a === '--scan-only') args.scanOnly = true
    else if (a === '--today') args.today = argv[++i]
    else if (a === '--out') args.out = argv[++i]
    else if (a === '--print-fingerprint') args.printFingerprint = argv[++i]
    else {
      console.error(`island-scan: unknown argument "${a}"`)
      process.exit(2)
    }
  }
  return args
}

function todayIso(override) {
  if (override) return override
  return new Date().toISOString().slice(0, 10)
}

export function runCargoMetadata({ cwd = REPO_ROOT } = {}) {
  const result = spawnSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
    cwd,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  })
  if (result.status !== 0) {
    throw new Error(`cargo metadata failed (exit ${result.status}): ${result.stderr}`)
  }
  return JSON.parse(result.stdout)
}

function loadAllowlist(path) {
  if (!existsSync(path)) {
    return { schema_version: 1, grace_period_days_default: 14, exceptions: [] }
  }
  return JSON.parse(readFileSync(path, 'utf8'))
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  const metadata = runCargoMetadata()
  const scan = classifyWorkspace(metadata)

  if (scan.errors.length) {
    console.error('island-scan: scan-level errors (fail-closed):')
    for (const e of scan.errors) console.error(`  - ${e}`)
    process.exit(2)
  }

  if (args.printFingerprint) {
    const c = scan.crates.find((c) => c.crate === args.printFingerprint)
    if (!c) {
      console.error(`island-scan: unknown crate "${args.printFingerprint}" — not a current workspace member.`)
      process.exit(2)
    }
    if (c.status !== 'ISLAND') {
      console.error(`island-scan: "${args.printFingerprint}" is currently ${c.status}, not ISLAND — no allowlist exception is needed or should be granted.`)
      process.exit(2)
    }
    process.stdout.write(computeIslandEvidenceFingerprint(c, ISLAND_SCAN_POLICY_VERSION) + '\n')
    process.exit(0)
  }

  if (args.scanOnly) {
    const out = stableStringify(scan)
    process.stdout.write(out)
    if (args.out) {
      mkdirSync(dirname(args.out), { recursive: true })
      writeFileSync(args.out, out)
    }
    process.exit(0)
  }

  const allowlistDoc = loadAllowlist(ALLOWLIST_PATH)
  const knownCrates = scan.crates.map((c) => c.crate)
  const { errors: allowlistErrors, entries } = validateAllowlistDoc(allowlistDoc, { knownCrates })

  if (allowlistErrors.length) {
    console.error(`island-scan: ${relative(REPO_ROOT, ALLOWLIST_PATH)} failed schema validation (fail-closed — no exception is granted from a malformed allowlist):`)
    for (const e of allowlistErrors) console.error(`  - ${e}`)
    process.exit(2)
  }

  const today = todayIso(args.today)
  const policy = applyAllowlistPolicy(scan, entries, { today, policyVersion: ISLAND_SCAN_POLICY_VERSION })
  const report = {
    schema_version: 1,
    policy_version: policy.policy_version,
    scan: { entrypoints: scan.entrypoints, summary: scan.summary },
    today: policy.today,
    crates: policy.crates,
    summary: policy.summary,
    blocked_crates: policy.blocked_crates,
    allowlisted_crates: policy.allowlisted_crates,
    stale_exceptions: policy.stale_exceptions,
    pass: policy.pass,
  }
  const out = stableStringify(report)
  process.stdout.write(out)
  if (args.out) {
    mkdirSync(dirname(args.out), { recursive: true })
    writeFileSync(args.out, out)
  }

  if (!policy.pass) {
    if (policy.blocked_crates.length) {
      console.error(`island-scan: ${policy.blocked_crates.length} crate(s) blocked (zero shipped caller, no valid unexpired fingerprint-matching allowlist exception): ${policy.blocked_crates.join(', ')}`)
      console.error('island-scan: promote a real caller, retire the crate, or add/renew a schema-valid, fingerprint-matching, time-bounded exception to tools/build/island-scan-allowlist.json with a linked issue.')
    }
    if (policy.stale_exceptions.length) {
      console.error(`island-scan: ${policy.stale_exceptions.length} stale allowlist exception(s) must be removed (never a silent pass):`)
      for (const s of policy.stale_exceptions) console.error(`  - ${s.crate} [${s.reason}]: ${s.detail}`)
    }
  }
  process.exit(args.report ? 0 : policy.pass ? 0 : 1)
}

main()
