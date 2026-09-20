// sync-anchor-v2-parser.test.mjs — black-box CLI tests: operational-failure exit codes,
// unknown-arg/missing-flag-value handling, --report honesty (DATA findings vs OPERATIONAL
// findings), unreadable/unwritable output paths, and that secret-shaped/malformed shard content
// never leaks into any CLI output surface (stdout, --json, --summary-out).
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import test from 'node:test'
import { buildAdversarialSecretShapes, cap, makeCrateWithManyFiles, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const parserPath = join(here, 'sync-anchor-v2-parser.mjs')

test('CLI: --expect-commit refuses a mismatched provenance commit before scanning', () => {
  const result = spawnSync(process.execPath, [parserPath, '--all', '--expect-commit', '0000000000000000000000000000000000000000'], { cwd: here, encoding: 'utf8' })
  assert.equal(result.status, 2)
  assert.match(result.stderr, /EXPECTED_COMMIT_MISMATCH/)
})
const repoRoot = join(here, '..', '..')

function runCli(cwd, args) {
  return spawnSync(process.execPath, [parserPath, ...args], { cwd, encoding: 'utf8' })
}

function noStackTrace(text) {
  // A raw Node uncaught-exception dump always includes a "at " stack frame line and/or the
  // literal "Error:"/"TypeError:" prefix straight from V8; a clean structured message never does.
  return !/^\s*at .+\(.*:\d+:\d+\)/m.test(text)
}

test('CLI: an unknown argument is a structured, bounded exit 2 -- never an uncaught stack trace, never suppressed by --report', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-unknown-')
  try {
    const result = runCli(root, ['--totally-not-a-real-flag'])
    assert.equal(result.status, 2)
    assert.match(result.stderr, /UNKNOWN_ARGUMENT@0/)
    assert.doesNotMatch(result.stderr, /--totally-not-a-real-flag/, 'the raw argument text must never be echoed, only a value-free category')
    assert.ok(noStackTrace(result.stderr))

    const reported = runCli(root, ['--totally-not-a-real-flag', '--report'])
    assert.equal(reported.status, 2, '--report must not suppress a configuration/usage error')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: a flag missing its required value is a structured exit 2, not undefined-crate nonsense or a crash', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-missing-value-')
  try {
    const result = runCli(root, ['--crate'])
    assert.equal(result.status, 2)
    assert.match(result.stderr, /requires a value/)
    assert.match(result.stderr, /MISSING_FLAG_VALUE@1/)
    assert.ok(noStackTrace(result.stderr))

    const followedByFlag = runCli(root, ['--crate', '--report'])
    assert.equal(followedByFlag.status, 2, '"--crate" immediately followed by another flag must not silently consume it as a value')
    assert.match(followedByFlag.stderr, /MISSING_FLAG_VALUE@1/)
    assert.doesNotMatch(followedByFlag.stderr, /--report/, 'the raw flag value ("--report") must never be echoed, only a value-free category')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: --out to a path whose parent is a FILE (not a directory) is a structured exit 3, never a stack trace', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-unwritable-')
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    const blocker = join(root, 'blocker-file')
    writeFileSync(blocker, 'not a directory')
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--out', join(blocker, 'report.json')])
    assert.equal(result.status, 3)
    assert.match(result.stderr, /could not write report/)
    assert.ok(noStackTrace(result.stderr))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: --summary-out to an unwritable path is also a structured exit 3', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-unwritable-summary-')
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    const blocker = join(root, 'blocker-file-2')
    writeFileSync(blocker, 'not a directory')
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--summary-out', join(blocker, 'summary.md')])
    assert.equal(result.status, 3)
    assert.ok(noStackTrace(result.stderr))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: --report makes a DATA finding (malformed shard line) exit 0 while still fully surfacing it', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-report-data-')
  try {
    const lines = [JSON.stringify(cap('fixture.real', 'real_mod', 'unimplemented')), 'not json {{{']
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), lines.join('\n') + '\n')

    const reported = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.equal(reported.status, 0)
    const parsed = JSON.parse(reported.stdout)
    assert.equal(parsed.shard_finding_count, 1)
    assert.equal(parsed.shard_findings[0].reason, 'SHARD_PARSE_ERROR')

    const notReported = runCli(root, ['--crate', 'chronica-fixture', '--json'])
    assert.equal(notReported.status, 1, 'without --report the same DATA finding must still cause a nonzero exit')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: a workspace-member path traversal is an OPERATIONAL finding -- nonzero even under --report', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-traversal-')
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    mkdirSync(join(root, '..', 'chronica-sync-anchor-v2-cli-outside'), { recursive: true })
    writeFileSync(join(root, '..', 'chronica-sync-anchor-v2-cli-outside', 'Cargo.toml'), '[package]\nname = "should-not-be-read"\nversion = "0.1.0"\n')
    writeFileSync(
      join(root, 'Cargo.toml'),
      '[workspace]\nmembers = ["crates/chronica-fixture", "../chronica-sync-anchor-v2-cli-outside"]\n',
    )

    const result = runCli(root, ['--all', '--report', '--json'])
    assert.equal(result.status, 1, 'a workspace-path escape must remain a nonzero, blocking finding even with --report')
    const parsed = JSON.parse(result.stdout)
    assert.equal(parsed.workspace_finding_count, 1)
    assert.equal(parsed.workspace_findings[0].reason, 'WORKSPACE_MEMBER_PATH_ESCAPES_ROOT')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(join(root, '..', 'chronica-sync-anchor-v2-cli-outside'), { recursive: true, force: true })
  }
})

test('CLI: secret-shaped content inside a malformed shard line never appears in stdout, --json, or --summary-out', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-secret-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-secret-out-'))
  try {
    const secret = buildAdversarialSecretShapes().genericSkLiveMedium
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), `{"capability_key": "${secret}, broken\n`)
    const outPath = join(outDir, 'report.json')
    const summaryPath = join(outDir, 'summary.md')

    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json', '--out', outPath, '--summary-out', summaryPath])
    assert.equal(result.status, 0)
    assert.ok(!result.stdout.includes(secret))
    assert.ok(!result.stderr.includes(secret))

    const jsonReport = readFileSync(outPath, 'utf8')
    const mdSummary = readFileSync(summaryPath, 'utf8')
    assert.ok(!jsonReport.includes(secret), 'the JSON report must never contain the secret-shaped content')
    assert.ok(!mdSummary.includes(secret), 'the Markdown summary must never contain the secret-shaped content')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})

test('CLI: identical input produces byte-identical --json output across two runs (determinism)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-determinism-')
  try {
    const rows = [cap('fixture.a', 'real_mod', 'unimplemented'), cap('fixture.b', 'real_mod', 'verified')]
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    const first = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    const second = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.equal(first.stdout, second.stdout)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: --out carries a truthful provenance block (generation time, repo commit, schema version); bare --json stdout stays deterministic without one', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-provenance-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-provenance-out-'))
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    const outPath = join(outDir, 'report.json')

    // Run 1: bare --json stdout only (no --out), so stdout is pure JSON with nothing appended.
    const jsonOnly = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.equal(jsonOnly.status, 0)
    const stdoutParsed = JSON.parse(jsonOnly.stdout)
    assert.equal('provenance' in stdoutParsed, false, 'bare --json stdout must stay free of a timestamp so repeated runs are byte-identical')

    // Run 2: --out writes the provenance-bearing artifact to disk; stdout also carries a trailing
    // human-readable confirmation line, so only the FILE (not stdout) is parsed as JSON here.
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--out', outPath])
    assert.equal(result.status, 0)
    const fileParsed = JSON.parse(readFileSync(outPath, 'utf8'))
    assert.equal(typeof fileParsed.provenance.generated_at, 'string')
    assert.ok(!Number.isNaN(Date.parse(fileParsed.provenance.generated_at)))
    // The fixture workspace is a plain temp dir, not a git repo, so repo_commit legitimately
    // falls back to null here (never throws, never a fabricated commit-shaped string) -- the
    // "runs inside the real repo" test below confirms a real commit hash is captured when one
    // actually exists.
    assert.ok(fileParsed.provenance.repo_commit === null || typeof fileParsed.provenance.repo_commit === 'string')
    assert.match(fileParsed.provenance.tool_schema_version, /^sync-anchor-v2-report\//)
    // Every non-provenance field is otherwise identical to the undecorated summary.
    assert.equal(fileParsed.capabilities_checked, stdoutParsed.capabilities_checked)
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})

test('CLI: --out captures the real repo_commit when actually run inside a git repository', () => {
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-provenance-real-'))
  try {
    const outPath = join(outDir, 'report.json')
    const result = runCli(repoRoot, ['--crate', 'chronica-cli', '--report', '--out', outPath])
    assert.equal(result.status, 0)
    const fileParsed = JSON.parse(readFileSync(outPath, 'utf8'))
    assert.match(fileParsed.provenance.repo_commit, /^[0-9a-f]{40}$/)
  } finally {
    rmSync(outDir, { recursive: true, force: true })
  }
})

// ── item 3: an unresolvable diff base is ALWAYS nonzero, even under --report (no-scan failure) ──

test('CLI: an unresolvable diff base (not a git repo, no --all/--crate/--base scope) is exit 2 even under --report -- never converted into a false success', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-nobase-')
  try {
    // makeMinimalCrateWorkspace creates a plain temp dir, not a git repository, so `git diff`
    // against the default base fails deterministically -- exactly the "diff base unavailable,
    // zero crates scanned" condition item 3 targets.
    const plain = runCli(root, [])
    assert.equal(plain.status, 2)
    assert.ok(noStackTrace(plain.stderr))

    const reported = runCli(root, ['--report'])
    assert.equal(reported.status, 2, '--report must not convert a no-scan operational failure into exit 0')
    assert.match(reported.stderr, /base ref unavailable/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: an unresolvable diff base with --out still writes an honest, explicitly-failed report artifact instead of nothing', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-nobase-out-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-nobase-out-artifact-'))
  try {
    const outPath = join(outDir, 'report.json')
    const summaryPath = join(outDir, 'summary.md')
    const result = runCli(root, ['--report', '--out', outPath, '--summary-out', summaryPath])
    assert.equal(result.status, 2)
    const parsed = JSON.parse(readFileSync(outPath, 'utf8'))
    assert.equal(parsed.ok, false)
    assert.equal(parsed.error, 'BASE_REF_UNAVAILABLE')
    assert.equal(parsed.scanned, false)
    const md = readFileSync(summaryPath, 'utf8')
    assert.match(md, /FAILED/)
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})

// ── new operational-finding types are also nonzero even under --report ─────────────────────────

test('CLI: a truncated source-file listing (>MAX_SOURCE_FILES_PER_CRATE) is nonzero even under --report', () => {
  const { root, cratePath, modNames } = makeCrateWithManyFiles('chronica-sync-anchor-v2-cli-truncated-', 2005)
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.many', modNames[0], 'unimplemented')) + '\n')
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.equal(result.status, 1, 'SOURCE_FILES_TRUNCATED must remain a nonzero operational finding even under --report')
    const parsed = JSON.parse(result.stdout)
    assert.ok(parsed.operational_finding_count >= 1)
    assert.equal(parsed.crates_skipped[0].reason, 'SOURCE_FILES_TRUNCATED')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, { timeout: 30_000 })

test('CLI: an oversized individual source file is nonzero even under --report', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-oversized-file-')
  try {
    writeFileSync(join(cratePath, 'src', 'huge.rs'), 'X'.repeat(3 * 1024 * 1024))
    writeFileSync(join(cratePath, 'src', 'lib.rs'), 'pub mod real_mod;\nmod huge;\n\npub fn dispatch() {\n    real_mod::run();\n}\n')
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.huge', 'huge', 'unimplemented')) + '\n')
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.equal(result.status, 1, 'SOURCE_FILE_OVERSIZED must remain a nonzero operational finding even under --report')
    const parsed = JSON.parse(result.stdout)
    assert.ok(parsed.operational_finding_count >= 1)
    assert.ok(parsed.operational_findings.some((f) => f.reason === 'SOURCE_FILE_OVERSIZED'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
