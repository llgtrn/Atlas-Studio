// sync-anchor-v2-sanitize.test.mjs — output-sanitization unit tests.
//
// Repair round 5 replaced the round-4 denylist (`looksSecretShaped`/`SECRET_SHAPE_PATTERNS`,
// which missed sk_live_/sk_test_/JWT/Basic-auth/DB-URL shapes) with strict allowlist/always-redact
// primitives: `sanitizeEnumField` (fixed-vocabulary passthrough), `sanitizeCrateName`/
// `sanitizePathField` (shape-allowlist passthrough for identifiers this tool computed/verified
// itself), and `redactValue`/`sanitizeErrorForDisplay` (ALWAYS value-free redact, no passthrough of
// any kind, for genuinely free-form/untrusted content). This file also carries the full C0/DEL/C1
// control-char coverage tests from round 4 (boundedText).
//
// Repair round 6 correction: round 5's redaction format (`<redacted:length=N:sha256=H>`) was found
// to NOT be value-free -- length and a hash are both derived from the actual secret, which is
// dictionary-attackable for short/low-entropy values. `redactValue`/`sanitizeErrorForDisplay` now
// emit a single fixed static token (`REDACTED_TOKEN`) carrying zero bits derived from the input; the
// tests below assert byte-identical output across MANY different secret shapes (not just "does not
// contain the substring"), which is the only way to prove zero information leakage. The adversarial
// secret-shaped literals below are built at runtime from harmless fragments (see
// buildAdversarialSecretShapes in sync-anchor-v2-fixtures.mjs) so this file's own source bytes never
// contain a contiguous secret-shaped string a scanner would flag.
import assert from 'node:assert/strict'
import test from 'node:test'
import {
  boundedText,
  comparePlain,
  redactValue,
  REDACTED_TOKEN,
  sanitizeCapabilityKey,
  sanitizeCrateName,
  sanitizeEnumField,
  sanitizeErrorForDisplay,
  sanitizePathField,
  sanitizeTargetModule,
} from './sync-anchor-v2-sanitize.mjs'
import { buildAdversarialSecretShapes } from './sync-anchor-v2-fixtures.mjs'

const secrets = buildAdversarialSecretShapes()

// ── comparePlain: locale-INDEPENDENT sort order (repair round 7, item 6) ────────────────────────

test('comparePlain: orders plain ASCII strings as expected', () => {
  assert.deepEqual(['b', 'a', 'c'].sort(comparePlain), ['a', 'b', 'c'])
})

test('comparePlain: returns 0 for equal strings, never a nonzero "close enough" collation result', () => {
  assert.equal(comparePlain('same', 'same'), 0)
})

test('comparePlain: uppercase sorts strictly before lowercase (plain UTF-16 code-unit order, never locale case-folding)', () => {
  // Under a case-insensitive/locale-aware collation (what localeCompare does by default in many
  // locales), 'a' and 'A' can compare as equal or order differently depending on the running
  // process's ICU data. Plain code-unit comparison is unambiguous: 'A' (U+0041) < 'a' (U+0061).
  assert.equal(comparePlain('A', 'a'), -1)
  assert.equal(comparePlain('a', 'A'), 1)
})

test('comparePlain: digits sort before letters, matching raw code-unit order (never a locale-aware "natural sort")', () => {
  assert.deepEqual(['b1', 'a1', '1a'].sort(comparePlain), ['1a', 'a1', 'b1'])
})

test('comparePlain: accented characters sort by raw code point, not locale collation (a real difference from localeCompare)', () => {
  // 'é' (U+00E9) has a strictly higher code unit than 'z' (U+007A) -- under many locale
  // collations, 'é' would sort adjacent to 'e', not after 'z'. This is the exact class of
  // platform/ICU-dependent behavior comparePlain exists to avoid.
  assert.equal(comparePlain('z', 'é'), -1)
})

// ── boundedText: full C0/DEL/C1 control-char coverage ───────────────────────────────────────────

test('boundedText: short strings pass through unchanged', () => {
  assert.equal(boundedText('unimplemented'), 'unimplemented')
})

test('boundedText: long strings are truncated with a bounded, disclosed marker', () => {
  const long = 'x'.repeat(1000)
  const result = boundedText(long, 50)
  assert.ok(result.length < 1000)
  assert.ok(result.startsWith('x'.repeat(50)))
  assert.match(result, /\[\+950 chars truncated\]$/)
})

test('boundedText: never throws on null/undefined/non-string input', () => {
  assert.doesNotThrow(() => boundedText(null))
  assert.doesNotThrow(() => boundedText(undefined))
  assert.doesNotThrow(() => boundedText(42))
  assert.doesNotThrow(() => boundedText(['array']))
})

test('boundedText: CR, LF, and TAB are stripped, not passed through', () => {
  assert.equal(boundedText('a\rb\nc\td'), 'abcd')
})

test('boundedText: DEL (U+007F) is stripped', () => {
  assert.equal(boundedText('a\x7Fb'), 'ab')
})

test('boundedText: the full C1 control range (U+0080-U+009F) is stripped', () => {
  const c1 = Array.from({ length: 0x9f - 0x80 + 1 }, (_, i) => String.fromCharCode(0x80 + i)).join('')
  assert.equal(boundedText(`x${c1}y`), 'xy')
})

test('boundedText: every C0 control code point (U+0000-U+001F) is stripped', () => {
  const c0 = Array.from({ length: 0x20 }, (_, i) => String.fromCharCode(i)).join('')
  assert.equal(boundedText(`x${c0}y`), 'xy')
})

// ── redactValue: unconditional, value-free (zero-derived-bits) redaction ────────────────────────

test('redactValue: never returns the original value verbatim, even a short/printable one', () => {
  const short = 'other.repo_scripts_registry'
  const result = redactValue(short)
  assert.notEqual(result, short)
  assert.equal(result, REDACTED_TOKEN)
})

test('redactValue: never returns a prefix of a long value', () => {
  const result = redactValue(secrets.genericSkLive)
  assert.ok(!result.includes('sk-live'))
  assert.equal(result, REDACTED_TOKEN)
})

test('redactValue: output is stable across repeated calls for the same input', () => {
  const value = `x${'y'.repeat(500)}`
  assert.equal(redactValue(value), redactValue(value))
})

// Round 6 correction: round 5 asserted "two different values redact to different hashes" -- the
// EXACT property that made the format dictionary-attackable. The correct, value-free property is
// the opposite: every distinct input -- short, long, structured, random -- redacts to the SAME
// static output. This is what "carries zero bits derived from the input" means operationally.
test('redactValue: many distinct secret shapes (short and long, every provider format) all redact to the exact same byte-identical static token', () => {
  const allShapes = [
    'a'.repeat(500),
    'b'.repeat(500),
    'short',
    '',
    secrets.stripeLive,
    secrets.stripeTest,
    secrets.jwt,
    secrets.basicAuth,
    secrets.postgresUrl,
    secrets.providerToken,
    secrets.providerTokenShort,
    secrets.genericBearer,
    secrets.genericPassword,
    secrets.genericApiKey,
    secrets.pemHeader,
    secrets.githubToken,
  ]
  const outputs = new Set(allShapes.map((value) => redactValue(value)))
  assert.equal(outputs.size, 1, 'every distinct secret shape must redact to exactly one shared static output')
  assert.equal([...outputs][0], REDACTED_TOKEN)
})

test('redactValue: never throws on null/undefined/non-string input', () => {
  assert.doesNotThrow(() => redactValue(null))
  assert.doesNotThrow(() => redactValue(undefined))
  assert.doesNotThrow(() => redactValue(42))
})

// item 1's explicitly-named adversarial shapes: every one of these must redact, none may pass
// through unchanged, regardless of length or "looking printable", and all collapse to the same
// static token (proving no length/hash oracle survives for any of them).
for (const [label, value] of [
  ['sk_live_ Stripe secret key', secrets.stripeLive],
  ['sk_test_ Stripe secret key', secrets.stripeTest],
  ['JWT', secrets.jwt],
  ['Basic auth header', secrets.basicAuth],
  ['postgres connection URL', secrets.postgresUrl],
  ['a short, otherwise-unnamed provider token', secrets.providerTokenShort],
]) {
  test(`redactValue: ${label} is never echoed and redacts to the static token`, () => {
    const result = redactValue(value)
    assert.ok(!result.includes(value))
    assert.equal(result, REDACTED_TOKEN)
  })
}

// ── sanitizeEnumField: strict allowlist for fixed-vocabulary fields ─────────────────────────────

const STATUS_ENUM = new Set(['unimplemented', 'implemented', 'implemented_unverified', 'verified'])

test('sanitizeEnumField: a real enum member passes through unchanged', () => {
  assert.equal(sanitizeEnumField('verified', STATUS_ENUM), 'verified')
})

test('sanitizeEnumField: a non-member value is redacted, never passed through', () => {
  const result = sanitizeEnumField('kinda-done-ish', STATUS_ENUM)
  assert.equal(result, REDACTED_TOKEN)
})

test('sanitizeEnumField: a secret-shaped value masquerading as a status is redacted, not treated as a member', () => {
  const result = sanitizeEnumField(secrets.basicAuth, STATUS_ENUM)
  assert.equal(result, REDACTED_TOKEN)
  assert.ok(!result.includes(secrets.basicAuth))
})

// ── sanitizeCrateName: Cargo's own package-name grammar as the allowlist ────────────────────────

test('sanitizeCrateName: a real crate name passes through unchanged', () => {
  assert.equal(sanitizeCrateName('chronica-cli'), 'chronica-cli')
})

test('sanitizeCrateName: an underscore-only name (also valid Cargo grammar) passes through', () => {
  assert.equal(sanitizeCrateName('chronica_tool'), 'chronica_tool')
})

test('sanitizeCrateName: a name containing a space is redacted', () => {
  assert.equal(sanitizeCrateName(secrets.genericBearer), REDACTED_TOKEN)
})

test('sanitizeCrateName: a name over 64 characters is redacted regardless of charset', () => {
  const long = 'a'.repeat(65)
  assert.equal(sanitizeCrateName(long), REDACTED_TOKEN)
})

test('sanitizeCrateName: a name not starting with a letter is redacted (Cargo requires a leading letter)', () => {
  assert.equal(sanitizeCrateName('123-not-a-valid-crate'), REDACTED_TOKEN)
})

test('sanitizeCrateName: a JWT-shaped value passed as a "crate name" is redacted', () => {
  assert.equal(sanitizeCrateName(secrets.jwt), REDACTED_TOKEN)
})

// ── sanitizePathField: a conservative charset allowlist for paths this tool computed itself ─────

test('sanitizePathField: a normal relative path passes through unchanged', () => {
  assert.equal(sanitizePathField('crates/chronica-cli/src/lib.rs'), 'crates/chronica-cli/src/lib.rs')
})

test('sanitizePathField: a path containing a ".." segment is redacted (defense-in-depth alongside isPathWithinRoot)', () => {
  assert.equal(sanitizePathField('crates/../../../etc/passwd'), REDACTED_TOKEN)
})

test('sanitizePathField: a path containing a space is redacted', () => {
  assert.equal(sanitizePathField('crates/evil path/lib.rs'), REDACTED_TOKEN)
})

test('sanitizePathField: a postgres URL passed as a "path" is redacted', () => {
  assert.equal(sanitizePathField(secrets.postgresUrl), REDACTED_TOKEN)
  assert.ok(!sanitizePathField(secrets.postgresUrl).includes(secrets.postgresUrl))
})

test('sanitizePathField: an over-length path is redacted', () => {
  const long = `crates/${'x'.repeat(500)}/lib.rs`
  assert.equal(sanitizePathField(long), REDACTED_TOKEN)
})

// ── sanitizeCapabilityKey: this tool's own dotted-identifier grammar (repair round 6) ───────────

test('sanitizeCapabilityKey: a real two-segment key passes through unchanged', () => {
  assert.equal(sanitizeCapabilityKey('fixture.real'), 'fixture.real')
})

test('sanitizeCapabilityKey: a real three-segment key passes through unchanged (5 of 2158 live keys use 2 dots)', () => {
  assert.equal(sanitizeCapabilityKey('policy.access.check_permission'), 'policy.access.check_permission')
})

test('sanitizeCapabilityKey: a key with no dot at all is redacted -- every real capability_key has at least one', () => {
  assert.equal(sanitizeCapabilityKey('no_dot_here'), REDACTED_TOKEN)
})

test('sanitizeCapabilityKey: a JWT is redacted even though it contains dots -- its oversized signature segment fails the per-segment length cap', () => {
  assert.equal(sanitizeCapabilityKey(secrets.jwt), REDACTED_TOKEN)
  assert.ok(!sanitizeCapabilityKey(secrets.jwt).includes(secrets.jwt))
})

test('sanitizeCapabilityKey: a Basic-auth/postgres-URL/password/api-key value used directly as a key is redacted (no dot, or forbidden characters)', () => {
  for (const value of [secrets.basicAuth, secrets.postgresUrl, secrets.genericPassword, secrets.genericApiKey, secrets.genericBearer]) {
    assert.equal(sanitizeCapabilityKey(value), REDACTED_TOKEN)
  }
})

test('sanitizeCapabilityKey: a key over the length cap is redacted regardless of otherwise-valid shape', () => {
  const long = `fixture.${'a'.repeat(100)}`
  assert.equal(sanitizeCapabilityKey(long), REDACTED_TOKEN)
})

// KNOWN, ACCEPTED LIMITATION -- documented here so it is never mistaken for an unfixed defect: a
// short alnum+underscore-only secret (e.g. a classic GitHub PAT body) embedded AFTER a
// legitimate-looking dotted prefix an attacker also controls satisfies this same grammar and is
// shown as-is, exactly like a real capability_key would be. This is structurally identical to the
// pre-existing, accepted risk in sanitizeCrateName (a bare 40-character GH PAT already satisfies
// Cargo's own package-name grammar) -- no allowlist-shape grammar can distinguish "a real
// short identifier" from "a short high-entropy secret with an identical shape" using shape alone;
// only unconditional redaction (round 5's model, rejected by round 6 for destroying report
// navigability) closes this completely. The per-segment length cap still closes the JWT case
// (test above) because real JWT segments are long; this residual gap is specifically for SHORT
// alnum-only secrets, which none of this round's named adversarial shapes (sk_live_/sk_test_/JWT/
// Basic/postgres-URL/generic bearer/password/api-key/PEM) naturally are once charset+dot
// requirements are applied to their OWN unmodified value (see the redaction tests above).
test('sanitizeCapabilityKey: KNOWN LIMITATION -- a short alnum-only secret fragment embedded after a plausible dotted prefix satisfies the grammar and passes through, exactly like sanitizeCrateName already accepts for crate names', () => {
  const composite = `fixture.${secrets.githubToken}`
  assert.equal(sanitizeCapabilityKey(composite), composite, 'documented, accepted residual risk -- not a regression to "fix" here')
})

// ── sanitizeTargetModule: ALWAYS redacted, no grammar passthrough (repair round 7 correction) ───
//
// Round 6's Rust-identifier-segment grammar was found (independent post-push audit against the
// exact-head commit) to simultaneously (a) still admit short alnum+underscore secret shapes
// (an AWS-key/GitHub-PAT-style token easily satisfies "<=40 chars, starts with a letter, only
// alnum+underscore" -- exactly as well as a real module name) and (b) redact ~30 genuinely real
// target_module values in this repo's own shard data that legitimately contain hyphens or dots
// (e.g. "abuse-prevention", "nodes.knowledge") -- no single regex can admit real repo usage
// without also admitting secret shapes of a similar size, so target_module is now unconditionally
// redacted with zero grammar-based passthrough of any kind; equality across rows is intentionally
// not preserved in output (there is no `target_module_ref` correlation index).

test('sanitizeTargetModule: a real module name is STILL redacted -- there is no passthrough at all anymore', () => {
  assert.equal(sanitizeTargetModule('real_mod'), REDACTED_TOKEN)
})

test('sanitizeTargetModule: a real ::-separated module path is redacted', () => {
  assert.equal(sanitizeTargetModule('billing::invoice_processor'), REDACTED_TOKEN)
})

test('sanitizeTargetModule: a real hyphenated/dotted repo target_module value (round-6\'s grammar gap) is also redacted, never echoed', () => {
  for (const value of ['abuse-prevention', 'nodes.knowledge', 'usage-metering']) {
    assert.equal(sanitizeTargetModule(value), REDACTED_TOKEN)
  }
})

test('sanitizeTargetModule: a short alnum-only secret-shaped value is redacted exactly like any other value -- no shape gets special treatment', () => {
  assert.equal(sanitizeTargetModule(secrets.githubToken), REDACTED_TOKEN)
})

test('sanitizeTargetModule: many distinct values (short, long, real-shaped, secret-shaped) all redact to the exact same byte-identical static token', () => {
  const values = ['real_mod', 'billing::invoice_processor', 'abuse-prevention', secrets.jwt, secrets.githubToken, '', 'x'.repeat(500)]
  const outputs = new Set(values.map((v) => sanitizeTargetModule(v)))
  assert.equal(outputs.size, 1)
  assert.equal([...outputs][0], REDACTED_TOKEN)
})

// ── sanitizeErrorForDisplay: fixed category + the same static redaction token, never an echo ────

test('sanitizeErrorForDisplay: never echoes any fragment of the original text', () => {
  const result = sanitizeErrorForDisplay('GIT_DIFF_FAILED', 'fatal: bad revision origin/nonexistent-branch')
  assert.ok(!result.includes('fatal'))
  assert.ok(!result.includes('bad revision'))
  assert.ok(!result.includes('nonexistent-branch'))
})

test('sanitizeErrorForDisplay: never echoes secret-shaped free-form text', () => {
  const result = sanitizeErrorForDisplay('UNEXPECTED_INTERNAL_ERROR', `${secrets.genericPassword} leaked in a stack frame`)
  assert.ok(!result.includes(secrets.genericPassword))
})

test('sanitizeErrorForDisplay: never echoes control characters/ANSI from the original text', () => {
  const result = sanitizeErrorForDisplay('GIT_DIFF_FAILED', 'line1\r\nline2\x1b[31mFAKE\x1b[0m')
  assert.ok(!/[\x00-\x1F\x7F-\x9F]/.test(result))
})

test('sanitizeErrorForDisplay: carries the given category verbatim plus the fixed static redaction token, never a length/hash derived from rawText', () => {
  const result = sanitizeErrorForDisplay('GIT_DIFF_FAILED', 'some stderr text')
  assert.equal(result, `GIT_DIFF_FAILED ${REDACTED_TOKEN}`)
  assert.equal(result, sanitizeErrorForDisplay('GIT_DIFF_FAILED', 'some stderr text'))
})

test('sanitizeErrorForDisplay: many distinct rawText values under the same category all produce byte-identical output (no length/hash oracle)', () => {
  const rawTexts = ['', 'x', 'a'.repeat(5000), secrets.stripeLive, secrets.jwt, secrets.postgresUrl, 'fatal: something went wrong']
  const outputs = new Set(rawTexts.map((text) => sanitizeErrorForDisplay('GIT_DIFF_FAILED', text)))
  assert.equal(outputs.size, 1, 'every distinct rawText under one category must produce exactly one shared output')
})

test('sanitizeErrorForDisplay: never throws on null/undefined/non-string input', () => {
  assert.doesNotThrow(() => sanitizeErrorForDisplay('CATEGORY', null))
  assert.doesNotThrow(() => sanitizeErrorForDisplay('CATEGORY', undefined))
  assert.doesNotThrow(() => sanitizeErrorForDisplay('CATEGORY', 42))
})
