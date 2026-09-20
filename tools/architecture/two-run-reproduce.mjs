#!/usr/bin/env node
// Deterministic, tracked, cross-platform two-run architecture-DB reproduction chain.
//
// Usage:
//   node tools/architecture/two-run-reproduce.mjs [--root <path>] [--out <json-path>]
//
// Rebuilds docs/architecture.db TWICE from the tracked source tree at `--root` (default: the
// current working directory), each into its own fresh temporary file, then computes the same
// raw/normalized SHA-256 pair `two-run-digest.mjs` defines. This is the single command an
// independent auditor re-runs, at the exact commit an evidence record was committed alongside, to
// reproduce an equivalent one from scratch -- no DB file or run1-only artifact needs to be
// tracked, because this script recreates both runs itself from ordinary tracked repo inputs.
//
// Every field this script's `main()` writes comes directly from `reproduceTwoRun`'s return value
// -- nothing is hand-added or edited into the JSON afterward. If a claim about what this evidence
// means needs prose (why it was run, which PR/commit it accompanies, what it does and doesn't
// prove), that prose belongs in the reconcile report that cites this file's output, not inside the
// output itself: mixing tool-generated and hand-authored fields in one JSON file is exactly what
// makes "is this really untouched tool output?" unanswerable from the file alone.
//
// `normalized_identical=true` is the reproducible invariant to check: raw digests legitimately
// differ between the two runs (and between any two independent invocations of this script), since
// `meta.generated_at`/`architecture_evidence.verified_at` are re-stamped on every rebuild -- see
// `two-run-digest.mjs`'s own header for the full canonical-serialization spec that excludes them.

import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { buildArchitectureDb } from './architecture-lib.mjs'
import { digestPair, SPEC_VERSION } from './two-run-digest.mjs'

const REPRODUCE_TOOL = 'tools/architecture/two-run-reproduce.mjs'
const DIGEST_TOOL = 'tools/architecture/two-run-digest.mjs'

/**
 * Rebuild the architecture DB twice, from the same tracked `root`, into fresh temporary files,
 * and return the digest-pair evidence record plus which two tools produced it. The temporary
 * directory (and both DB files in it) is removed before returning -- only the computed record
 * (digests + table counts), never the DB files themselves, needs to survive this call.
 */
export function reproduceTwoRun({ root = process.cwd() } = {}) {
  const resolvedRoot = resolve(root)
  const tmpDir = mkdtempSync(join(tmpdir(), 'two-run-reproduce-'))
  try {
    const run1Path = join(tmpDir, 'run1.db')
    const run2Path = join(tmpDir, 'run2.db')
    buildArchitectureDb({ root: resolvedRoot, dbPath: run1Path })
    buildArchitectureDb({ root: resolvedRoot, dbPath: run2Path })
    // Destructure out `pair`'s own `tool`/`spec_version` (both name the lower-level hash tool)
    // so this script's own `tool`/`spec_version` fields below are the ones that win.
    const { tool: _digestPairTool, spec_version: _digestPairSpecVersion, ...pairRest } = digestPair(
      run1Path,
      run2Path,
    )
    return {
      tool: REPRODUCE_TOOL,
      digest_tool: DIGEST_TOOL,
      spec_version: SPEC_VERSION,
      root: resolvedRoot,
      ...pairRest,
    }
  } finally {
    rmSync(tmpDir, { recursive: true, force: true })
  }
}

function main() {
  const args = process.argv.slice(2)
  const rootIdx = args.indexOf('--root')
  const root = rootIdx >= 0 ? args[rootIdx + 1] : process.cwd()
  const outIdx = args.indexOf('--out')
  const outPath = outIdx >= 0 ? args[outIdx + 1] : null

  const result = reproduceTwoRun({ root })
  console.log(`two-run-reproduce: spec_version=${result.spec_version} root=${result.root}`)
  console.log(`  run1: raw=${result.run1.raw_sha256} normalized=${result.run1.normalized_sha256}`)
  console.log(`  run2: raw=${result.run2.raw_sha256} normalized=${result.run2.normalized_sha256}`)
  console.log(`  raw_identical=${result.raw_identical} normalized_identical=${result.normalized_identical}`)

  if (outPath) {
    writeFileSync(outPath, JSON.stringify(result, null, 2) + '\n')
    console.log(`  wrote ${outPath}`)
  }

  if (!result.normalized_identical) {
    console.error('  ✗ normalized digests differ across two fresh rebuilds -- content is NOT deterministic')
    process.exit(1)
  }
  console.log('  ✓ normalized digests match -- content is deterministic across two fresh rebuilds')
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href
if (isMain) {
  main()
}
