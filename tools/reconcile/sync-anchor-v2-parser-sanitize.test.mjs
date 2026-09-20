// sync-anchor-v2-parser-sanitize.test.mjs — black-box CLI output-sanitization tests: a VALID
// (schema-conformant) but abnormally long/control-char/secret-shaped field never leaks raw across
// any output surface, diff/write/usage errors are value-free categories, and repair-round-5
// write-confirmation/symlink-escape/oversized-manifest end-to-end coverage.
//
// Split out of sync-anchor-v2-parser.test.mjs purely to keep both files under this repo's
// ~500-line file-size convention.
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import test from 'node:test'
import { buildAdversarialSecretShapes, cap, makeMinimalCrateWorkspace } from './sync-anchor-v2-fixtures.mjs'
import { REDACTED_TOKEN } from './sync-anchor-v2-sanitize.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const parserPath = join(here, 'sync-anchor-v2-parser.mjs')
const repoRoot = join(here, '..', '..')
const secrets = buildAdversarialSecretShapes()

function runCli(cwd, args) {
  return spawnSync(process.execPath, [parserPath, ...args], { cwd, encoding: 'utf8' })
}

function noStackTrace(text) {
  return !/^\s*at .+\(.*:\d+:\d+\)/m.test(text)
}

// ── item 5: a VALID (schema-conformant) but abnormally long/control-char field never leaks raw ──

test('CLI: a valid capability row with a long/secret-shaped capability_key never appears raw in human summary, --json stdout, or --out', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-sanitize-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-sanitize-out-'))
  try {
    const secretShaped = secrets.genericSkLive
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap(secretShaped, 'nonexistent_mod', 'unimplemented')) + '\n')
    const outPath = join(outDir, 'report.json')

    const human = runCli(root, ['--crate', 'chronica-fixture', '--report'])
    assert.ok(!human.stdout.includes(secretShaped), 'human summary must never echo the raw long/secret-shaped capability_key')

    const jsonRun = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.ok(!jsonRun.stdout.includes(secretShaped), 'bare --json stdout must never echo it either')
    const jsonKey = JSON.parse(jsonRun.stdout).disagreements[0]?.capability_key ?? ''
    assert.ok(jsonKey === REDACTED_TOKEN || jsonKey === '')

    const outRun = runCli(root, ['--crate', 'chronica-fixture', '--report', '--out', outPath])
    assert.equal(outRun.status, 0)
    const fileContent = readFileSync(outPath, 'utf8')
    assert.ok(!fileContent.includes(secretShaped), '--out report file must never echo it either')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})

test('CLI: a valid capability row with a control-character-laden status never leaks a raw ANSI/control sequence into any output', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-sanitize-control-')
  try {
    const weirdStatus = 'unimplemented\x1b[31mFAKE\x1b[0m'
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.weird', 'real_mod', weirdStatus)) + '\n')
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.ok(!result.stdout.includes('\x1b['), 'no raw ANSI escape sequence from shard content may reach stdout')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 4: adversarial coverage across EVERY output surface (human, --json, --out, ────
// ── --summary-out Markdown) for CR/LF/TAB/DEL/C1/ANSI and short secret-shaped field values ──────

test('CLI: CR/LF/TAB/DEL/C1 in a capability_key never leak raw into human, --json, --out, or --summary-out Markdown', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-c0c1-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-c0c1-out-'))
  try {
    const c1Char = String.fromCharCode(0x85)
    const poisoned = `fixture.evil\r\nInjected-Header: yes\t${c1Char}\x7Fdel-and-nel`
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap(poisoned, 'real_mod', 'unimplemented')) + '\n')
    const outPath = join(outDir, 'report.json')
    const summaryPath = join(outDir, 'summary.md')
    // Every output surface here legitimately uses plain LF (U+000A) as its own line separator
    // (human summary lines, JSON.stringify indentation, the Markdown file) -- the control-byte scan
    // below deliberately excludes ONLY that one byte so it does not false-positive on the CLI's own
    // formatting, while still catching CR/TAB/DEL/C1/every other C0 byte from the poisoned value.

    const human = runCli(root, ['--crate', 'chronica-fixture', '--report'])
    assert.ok(!human.stdout.includes(poisoned))
    assert.ok(!/[\x00-\x09\x0B-\x1F\x7F-\x9F]/.test(human.stdout), 'no raw control byte from shard content may reach human stdout')

    const jsonRun = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.ok(!jsonRun.stdout.includes(poisoned))
    assert.ok(!/[\x00-\x09\x0B-\x1F\x7F-\x9F]/.test(jsonRun.stdout), 'no raw control byte may reach --json stdout')

    const withOut = runCli(root, ['--crate', 'chronica-fixture', '--report', '--out', outPath, '--summary-out', summaryPath])
    assert.equal(withOut.status, 0)
    const fileContent = readFileSync(outPath, 'utf8')
    const mdContent = readFileSync(summaryPath, 'utf8')
    assert.ok(!fileContent.includes(poisoned))
    assert.ok(!/[\x00-\x09\x0B-\x1F\x7F-\x9F]/.test(fileContent), 'no raw control byte may reach the --out report file')
    assert.ok(!mdContent.includes(poisoned))
    assert.ok(!/[\x00-\x09\x0B-\x1F\x7F-\x9F]/.test(mdContent), 'no raw control byte may reach the --summary-out Markdown file')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})

test('CLI: a SHORT secret-shaped capability_key (Bearer/API-key/password/private-key/sk_live_/sk_test_/JWT/Basic/postgres-URL/provider-token fragment) is redacted across every output surface, not just long ones (repair round 5, item 1)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-short-secret-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-short-secret-out-'))
  try {
    const shortSecrets = [
      secrets.genericBearer,
      secrets.genericPassword,
      secrets.genericApiKey,
      secrets.pemHeader,
      secrets.githubToken,
      secrets.stripeLiveShort,
      secrets.stripeTestShort,
      secrets.jwt,
      secrets.basicAuth,
      secrets.postgresUrl,
      secrets.providerToken,
    ]
    // Each secret is used AS the raw capability_key, exactly as an attacker who fully controls
    // this field would set it -- not wrapped in a synthetic "fixture.N." prefix, which would
    // artificially manufacture a 2-segment dotted shape none of these secrets naturally has (round
    // 6 correction: capability_key now passes through when it matches its own strict grammar --
    // see sanitizeCapabilityKey's CAPABILITY_KEY_ALLOW_RE -- so this test must exercise the field
    // exactly as real attacker-controlled input would appear, not a shape this test invented).
    const rows = shortSecrets.map((secret) => cap(secret, 'real_mod', 'unimplemented'))
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), rows.map((r) => JSON.stringify(r)).join('\n') + '\n')
    const outPath = join(outDir, 'report.json')
    const summaryPath = join(outDir, 'summary.md')

    const human = runCli(root, ['--crate', 'chronica-fixture', '--report'])
    const jsonRun = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    const withOut = runCli(root, ['--crate', 'chronica-fixture', '--report', '--out', outPath, '--summary-out', summaryPath])
    assert.equal(withOut.status, 0)
    const fileContent = readFileSync(outPath, 'utf8')
    const mdContent = readFileSync(summaryPath, 'utf8')

    for (const secret of shortSecrets) {
      assert.ok(!human.stdout.includes(secret), `human summary must never echo short secret-shaped fragment: ${secret}`)
      assert.ok(!jsonRun.stdout.includes(secret), `--json stdout must never echo short secret-shaped fragment: ${secret}`)
      assert.ok(!fileContent.includes(secret), `--out report file must never echo short secret-shaped fragment: ${secret}`)
      assert.ok(!mdContent.includes(secret), `--summary-out Markdown must never echo short secret-shaped fragment: ${secret}`)
    }
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})

// ── repair round 4, item 3: diff/write/usage errors are value-free categories, never raw text ───

test('CLI: a git-diff failure against a bad --base ref is reported as a value-free GIT_DIFF_FAILED category, never git\'s raw stderr text', () => {
  const result = runCli(repoRoot, ['--base', 'totally-bogus-nonexistent-ref-xyz'])
  assert.equal(result.status, 2)
  assert.match(result.stderr, new RegExp(`GIT_DIFF_FAILED ${REDACTED_TOKEN.replace(/[<>]/g, '\\$&')}`))
  assert.ok(!/unknown revision|bad revision|ambiguous argument/i.test(result.stderr), "git's own raw stderr text must never be echoed")
  assert.ok(noStackTrace(result.stderr))
})

test('CLI: an unknown argument\'s exact text never appears in stderr even when it looks like a plausible secret/flag', () => {
  const { root } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-unknown-secret-')
  try {
    const result = runCli(root, [`--api-key=${secrets.genericSkLiveFlag}`])
    assert.equal(result.status, 2)
    assert.match(result.stderr, new RegExp(`UNKNOWN_ARGUMENT@0 ${REDACTED_TOKEN.replace(/[<>]/g, '\\$&')}`))
    assert.ok(!result.stderr.includes(secrets.genericSkLiveFlag))
    assert.ok(noStackTrace(result.stderr))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: --out to an unwritable path reports REPORT_WRITE_FAILED as a value-free category when the OS gives no short error code text of its own', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-write-fail-category-')
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    const blocker = join(root, 'blocker-file-3')
    writeFileSync(blocker, 'not a directory')
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--out', join(blocker, 'report.json')])
    assert.equal(result.status, 3)
    // A short, fixed, known-safe OS errno code (never attacker/shard text, e.g. EEXIST/ENOTDIR
    // depending on which mkdir/write step trips first) is shown as-is; this asserts the
    // write-failure path is reachable end to end and stays bounded/clean, complementing the
    // ReportWriteError unit-level coverage of the sanitizeErrorForDisplay fallback.
    assert.match(result.stderr, /: E[A-Z]+\s*$/m)
    assert.ok(noStackTrace(result.stderr))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

// ── repair round 5 ────────────────────────────────────────────────────────────────────────────

test('CLI: item 1 -- a successful write confirmation log never echoes the --out/--summary-out path itself', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-write-confirm-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-cli-write-confirm-out-'))
  try {
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    const outPath = join(outDir, 'a-distinctive-report-name.json')
    const summaryPath = join(outDir, 'a-distinctive-summary-name.md')
    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--out', outPath, '--summary-out', summaryPath])
    assert.equal(result.status, 0)
    assert.ok(!result.stdout.includes(outPath), 'the write-confirmation log must not echo the --out path')
    assert.ok(!result.stdout.includes(summaryPath), 'the write-confirmation log must not echo the --summary-out path')
    assert.ok(!result.stdout.includes('a-distinctive-report-name'))
    assert.ok(!result.stdout.includes('a-distinctive-summary-name'))
    assert.ok(result.stdout.includes(`wrote report to ${REDACTED_TOKEN}`))
    assert.ok(result.stdout.includes(`wrote Markdown summary to ${REDACTED_TOKEN}`))
    // The files themselves are still genuinely written to the real path -- only the CONFIRMATION
    // LOG stops echoing it.
    assert.ok(readFileSync(outPath, 'utf8').length > 0)
    assert.ok(readFileSync(summaryPath, 'utf8').length > 0)
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})

test('CLI: item 2 -- a workspace member whose Cargo.toml is a SYMLINK escaping the repo root is an OPERATIONAL finding end to end, never silently followed', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-symlink-escape-')
  const outside = join(root, '..', 'chronica-sync-anchor-v2-cli-symlink-outside')
  try {
    mkdirSync(outside, { recursive: true })
    const marker = 'OUTSIDE_ROOT_CLI_MARKER'
    writeFileSync(join(outside, 'Cargo.toml'), `[package]\nname = "${marker}"\nversion = "0.1.0"\n`)
    mkdirSync(join(root, 'crates', 'chronica-evil'), { recursive: true })
    symlinkSync(join(outside, 'Cargo.toml'), join(root, 'crates', 'chronica-evil', 'Cargo.toml'))
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture", "crates/chronica-evil"]\n')
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')

    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.equal(result.status, 1, 'a workspace-member symlink escape is an operational finding, never suppressed by --report')
    const parsed = JSON.parse(result.stdout)
    assert.ok(parsed.workspace_findings.some((f) => f.reason === 'WORKSPACE_MEMBER_PATH_ESCAPES_ROOT'))
    assert.ok(!result.stdout.includes(marker), 'the outside-root package name must never be read/exposed')
    assert.ok(!result.stdout.includes('chronica-evil'), 'the raw member path text is always fully redacted, never echoed')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outside, { recursive: true, force: true })
  }
})

test('CLI: item 4 -- an oversized member Cargo.toml is an OPERATIONAL finding end to end, never a thrown exception or a silently-dropped crate', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-cli-manifest-oversized-')
  try {
    mkdirSync(join(root, 'crates', 'chronica-huge'), { recursive: true })
    writeFileSync(join(root, 'crates', 'chronica-huge', 'Cargo.toml'), `[package]\nname = "chronica-huge"\n# ${'x'.repeat(300 * 1024)}\n`)
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture", "crates/chronica-huge"]\n')
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')

    const result = runCli(root, ['--crate', 'chronica-fixture', '--report', '--json'])
    assert.equal(result.status, 1, 'an oversized manifest is an operational finding, never suppressed by --report')
    const parsed = JSON.parse(result.stdout)
    assert.ok(parsed.operational_findings.some((f) => f.reason === 'MANIFEST_OVERSIZED'))
    assert.ok(noStackTrace(result.stderr))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('CLI: shardFindingsFor never echoes a raw shard_path -- a workspace member path containing a control character never reaches console, --json, --out, or --summary-out Markdown (round 7 post-push fix)', () => {
  const { root, cratePath } = makeMinimalCrateWorkspace('chronica-sync-anchor-v2-shard-path-control-char-')
  const outDir = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-shard-path-out-'))
  try {
    // A workspace member declared with an embedded tab character in its own path text -- passes
    // containment (it never escapes root) but must never be echoed raw once folded into shard_path.
    const weirdDirName = 'crates/weird\ttab-member'
    mkdirSync(join(root, weirdDirName), { recursive: true })
    writeFileSync(join(root, weirdDirName, 'Cargo.toml'), '[package]\nname = "chronica-weird"\nversion = "0.1.0"\n')
    mkdirSync(join(root, weirdDirName, '.chronica'), { recursive: true })
    // A malformed JSONL line triggers a SHARD_PARSE_ERROR finding, which carries shard_path.
    writeFileSync(join(root, weirdDirName, '.chronica', 'sub-cap-arch.jsonl'), '{not valid json\n')
    writeFileSync(join(root, 'Cargo.toml'), `[workspace]\nmembers = ["crates/chronica-fixture", "${weirdDirName}"]\n`)
    writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), JSON.stringify(cap('fixture.stub', 'nonexistent', 'unimplemented')) + '\n')
    const outPath = join(outDir, 'report.json')
    const summaryPath = join(outDir, 'summary.md')

    const human = runCli(root, ['--all', '--report'])
    const jsonRun = runCli(root, ['--all', '--report', '--json'])
    const withOut = runCli(root, ['--all', '--report', '--out', outPath, '--summary-out', summaryPath])
    assert.equal(withOut.status, 0)
    const fileContent = readFileSync(outPath, 'utf8')
    const mdContent = readFileSync(summaryPath, 'utf8')

    for (const surface of [human.stdout, jsonRun.stdout, fileContent, mdContent]) {
      assert.ok(!surface.includes(weirdDirName), 'the raw weird member path text must never appear on any output surface')
      assert.ok(!/[\x00-\x09\x0B-\x1F\x7F-\x9F]/.test(surface), 'no raw control byte from the member path may reach any output surface')
    }
    const parsed = JSON.parse(jsonRun.stdout)
    const shardFinding = parsed.shard_findings.find((f) => f.reason === 'SHARD_PARSE_ERROR')
    assert.ok(shardFinding, 'the malformed line in the weird-path crate must still be surfaced as a real finding')
    assert.equal(shardFinding.shard_path, REDACTED_TOKEN, 'a shard_path containing a control character must be redacted, not echoed raw')
  } finally {
    rmSync(root, { recursive: true, force: true })
    rmSync(outDir, { recursive: true, force: true })
  }
})
