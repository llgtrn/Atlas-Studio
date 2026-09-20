// sync-anchor-v2-sanitize.mjs — every bound and every output-sanitization primitive the
// sync-anchor-v2 atom parser uses to keep PR-controlled/operator-supplied content out of any
// diagnostic surface (human summary, --json, --out, --summary-out Markdown, CI job summary).
//
// Split out of sync-anchor-v2-shard.mjs (repair round 5) purely to keep both files under this
// repo's ~500-line file-size convention -- this module has no filesystem/traversal concerns of its
// own (see sync-anchor-v2-fs-safety.mjs for those); it is pure string transforms.

// repair round 7, item 6: a locale-INDEPENDENT string comparator, used everywhere this tool sorts
// findings/entries for deterministic output. `String.prototype.localeCompare` was used throughout
// rounds 2-6 -- its collation order depends on the running process's ICU data and default locale,
// which can differ across Node.js builds/versions/OS locales, silently breaking the "identical
// input always produces byte-identical output regardless of platform" guarantee this tool relies
// on elsewhere (listRustFiles/listDirEntriesBounded already sort explicitly FOR this reason).
// Plain `<`/`>` on strings compares raw UTF-16 code units -- no locale, no ICU, no collation rules,
// the same result on every machine this ever runs on.
export function comparePlain(a, b) {
  if (a < b) return -1
  if (a > b) return 1
  return 0
}

// ── bounds (defense-in-depth against pathological/adversarial shard or source input) ───────────
// Every bound below carries at least 5x headroom over the largest real value measured in this
// repository at the time this module was written (largest shard: ~600KB/1100 lines/828-char
// line; largest .rs file: ~120KB; most .rs files in one crate: 316; largest manifest ~28KB;
// largest doc ~299KB; docs/ directory ~321 entries; largest single-level src/ directory 280
// entries) so no real crate is ever affected — these exist to turn a pathological/adversarial
// input into a deterministic, bounded, fail-closed finding instead of unbounded memory/CPU/output
// growth.
export const BOUNDS = {
  MAX_SHARD_BYTES: 8 * 1024 * 1024,
  MAX_SHARD_LINES: 20_000,
  MAX_LINE_LENGTH: 8_192,
  MAX_SOURCE_FILE_BYTES: 2 * 1024 * 1024,
  MAX_SOURCE_FILES_PER_CRATE: 2_000,
  MAX_TEXT_FIELD_LENGTH: 200,
  MAX_RENDERED_LIST_ITEMS: 40,
  // Repair round 5 (item 4): the root/member Cargo.toml manifests and crate docs this tool reads
  // were previously loaded with no size bound at all (a bare readFileSync).
  MAX_MANIFEST_BYTES: 256 * 1024,
  MAX_DOC_BYTES: 2 * 1024 * 1024,
  MAX_DOC_DIR_ENTRIES: 4_000,
  // Bounds ANY single directory level's raw entry enumeration (source-tree walk) independent of
  // the separate MAX_SOURCE_FILES_PER_CRATE total.
  MAX_DIR_ENTRIES_PER_LEVEL: 5_000,
  // repair round 7, item 4: bounds the source-tree walk's own recursion depth and total directory
  // count, independent of file count -- a directory-symlink cycle or a pathologically deep/wide
  // (but file-sparse) tree could previously recurse unboundedly (stack overflow) or loop forever
  // without ever tripping MAX_SOURCE_FILES_PER_CRATE. Measured: this repo's deepest crate src/ tree
  // is 3 levels, largest crate has under 100 total directories under src/ -- both bounds carry
  // >20x headroom over any real value.
  MAX_DIR_DEPTH: 64,
  MAX_DIR_COUNT: 10_000,
}

// A single pattern used to build two SEPARATE RegExp instances below: `.test()` on a `/g` regex
// is stateful (mutates `lastIndex` across calls, a classic footgun when the same instance is
// reused), so the test-only and replace-only regexes are deliberately distinct objects.
//
// Covers the FULL C0 control range (U+0000-U+001F -- including TAB U+0009, LF U+000A, and CR
// U+000D), DEL (U+007F), and the C1 control range (U+0080-U+009F, contiguous with DEL so one
// range expresses both: \x7F-\x9F). CR/LF/TAB are exactly as unsafe as any other control byte
// here: each can forge extra lines, fake table rows, or manipulate a terminal/log renderer just
// as effectively as an ESC-prefixed ANSI sequence.
const CONTROL_CHAR_PATTERN = '[\\x00-\\x1F\\x7F-\\x9F]'
const CONTROL_CHAR_STRIP_RE = new RegExp(CONTROL_CHAR_PATTERN, 'g')

function stripControlChars(text) {
  return text.replace(CONTROL_CHAR_STRIP_RE, '')
}

/** Coerces to a bounded-length, control-character-free string for safe embedding in any
 * diagnostic surface. Never throws; never returns more than maxLen characters; strips C0/DEL/C1
 * control bytes (including raw ANSI escape sequences and CR/LF/TAB) before bounding so a crafted
 * value cannot manipulate a terminal/log renderer or forge extra lines/rows. Still shows a
 * (bounded) content PREFIX -- this is a low-level primitive for genuinely low-risk/internal text
 * (e.g. feeding escapeMarkdownCell); it must never be used as the sole guard on shard-derived or
 * operator-supplied content -- use `redactValue`/`sanitizeEnumField`/`sanitizeCrateName`/
 * `sanitizePathField` for that (repair round 5), or `sanitizeErrorForDisplay` for genuinely
 * free-form/unexpected diagnostic text that must never echo ANY of the original. */
export function boundedText(value, maxLen = BOUNDS.MAX_TEXT_FIELD_LENGTH) {
  const text = stripControlChars(typeof value === 'string' ? value : String(value ?? ''))
  if (text.length <= maxLen) return text
  return `${text.slice(0, maxLen)}…[+${text.length - maxLen} chars truncated]`
}

// ── strict allowlist sanitization (repair round 5, corrected round 6) ───────────────────────────
//
// A round-4 audit found that shape-based secret DETECTION (a denylist of recognizable credential
// patterns: Bearer/PEM/password=/sk-/ghp_/...) still let sk_live_/sk_test_ (underscore, not the
// denylist's hyphen), JWTs, `Basic <base64>`, database connection URLs, and any other
// not-yet-enumerated provider-key shape straight through -- a denylist can only ever cover shapes
// someone thought to name in advance, so it degrades to "protection against known secrets", never
// to "protection against unexpected content". The functions below invert that: rather than trying
// to recognize what is UNSAFE, they recognize what is SAFE (a small, explicit, independently
// justified allowlist per field) and value-free-redact everything else, with no partial/shape-based
// passthrough. A field with no defensible fixed shape is ALWAYS redacted, unconditionally.
//
// Round 5 redacted with `length + sha256(value).slice(0,12)`. A round-6 audit correctly identified
// this as NOT value-free: length and a hash are both derived from the actual secret, and for any
// short/low-entropy value (a 4-8 char password, a short token) an attacker can enumerate candidate
// plaintexts, hash each one the same way, and match against the emitted hash -- a dictionary
// attack the "value-free" claim implicitly promised was impossible. There is no length/hash
// encoding that is simultaneously informative and safe against this: even truncating the hash or
// bucketing the length narrows the search space but does not close it. The only genuinely
// value-free redaction is a SINGLE FIXED STATIC TOKEN carrying no bit of information derived from
// the input at all -- not length, not a hash, not a fingerprint, not a prefix, not a suffix. That
// necessarily gives up cross-finding correlation of repeated abnormal values (a real capability
// this format used to offer); the trade is required, not optional, once any derived-from-content
// output is admitted to be an oracle.
export const REDACTED_TOKEN = '<redacted>'

/** Replaces `value` ENTIRELY with a single fixed static token -- the SAME token for every input,
 * carrying zero bits derived from `value` (no length, no hash, no fingerprint, no prefix, no
 * suffix, no content of any kind). This is the correct treatment for every genuinely
 * free-form/untrusted field (capability_key, target_module, an operator-supplied
 * --out/--summary-out/--base value, a workspace-member path escape attempt, any invalid/unparsed
 * status text): none of these have a shape this tool can prove is safe in advance, so none of them
 * are ever echoed, not even a short/printable-looking one, and none of them ever influence the
 * redaction's own output -- two different secrets (of any length, any shape) redact to
 * byte-identical text, which is the only way to guarantee zero information leakage, including
 * against an offline dictionary/brute-force attack over short or low-entropy inputs. */
export function redactValue(_value) {
  return REDACTED_TOKEN
}

/** For a field that is genuinely fixed-vocabulary (a small, code-controlled enum -- a status word,
 * a normalized doc-status word): passes through ONLY when `value` is an exact member of
 * `allowedValues`; any other value (by definition already known-invalid/unexpected, since a valid
 * enum member always matches) is treated as free-form/untrusted and value-free redacted via
 * `redactValue`. Membership in a Set is a strict allowlist check, not a shape guess. */
export function sanitizeEnumField(value, allowedValues) {
  const text = typeof value === 'string' ? value : String(value ?? '')
  return allowedValues.has(text) ? text : redactValue(text)
}

// Cargo's own published package-name grammar (a real, external, independently-authoritative
// spec -- not a shape this tool invented): must start with an ASCII letter, followed by any run of
// ASCII letters/digits/underscore/hyphen. crates.io additionally caps package names at 64
// characters; reused verbatim here rather than a repo-specific guess.
const CRATE_NAME_ALLOW_RE = /^[A-Za-z][A-Za-z0-9_-]*$/
const CRATE_NAME_MAX_LEN = 64

/** For a crate-identity field: passes through ONLY when `value` matches Cargo's real package-name
 * grammar and stays within crates.io's own 64-character cap; anything else (whitespace, a
 * protocol/credential shape, control characters, an over-length value) is value-free redacted.
 * Crate names in this tool are parsed from a PR-controlled Cargo.toml `name = "..."` line with no
 * shape validation of their own (repair round 5 finding: an attacker-chosen package name is not
 * otherwise constrained before reaching a report), so this is the field's own dedicated guard. */
export function sanitizeCrateName(value) {
  const text = typeof value === 'string' ? value : String(value ?? '')
  if (text.length > 0 && text.length <= CRATE_NAME_MAX_LEN && CRATE_NAME_ALLOW_RE.test(text)) return text
  return redactValue(text)
}

// A conservative charset for a normal relative filesystem path in this repo (ASCII
// letters/digits/`._-`/`/` only -- no whitespace, no `:`/`@`/protocol-scheme punctuation), plus an
// explicit `..` path-segment refusal (defense-in-depth alongside isPathWithinRoot's own
// resolve()-based containment check, never a substitute for it).
const PATH_CHARSET_RE = /^[A-Za-z0-9._/-]+$/
const PATH_MAX_LEN = 400

/** For a path field that this tool computed itself (via `path.relative()` from an
 * already-containment-checked root -- shardPath, docPath, a matched source-module path): passes
 * through ONLY when `value` matches a conservative path charset, contains no `..` segment, and
 * stays within a generous bound; anything else is value-free redacted. Never used for operator
 * CLI-flag values (--out/--summary-out/--base) or pre-verification workspace-member text -- those
 * are always fully redacted via `redactValue` regardless of shape (repair round 5, item 1: a
 * successful-write confirmation log must not echo the path it just wrote). */
export function sanitizePathField(value) {
  const text = typeof value === 'string' ? value : String(value ?? '')
  const looksSafe = text.length > 0 && text.length <= PATH_MAX_LEN && PATH_CHARSET_RE.test(text) && !text.split('/').includes('..')
  return looksSafe ? text : redactValue(text)
}

// This tool's own, pre-existing, documented capability_key grammar (repair round 2's doc-table
// parser already required exactly this shape to treat a `| Key | ... |` cell as a real capability
// key at all -- see sync-anchor-v2-shard.mjs's doc-status matcher): one or more dot-separated
// segments, each segment starting with an alphanumeric and continuing with
// alphanumeric/`_`/`-`, case-insensitive. This is the one field-specific grammar this tool itself
// defines and has already relied on structurally (not just for output display) since round 2, so
// it is exactly the kind of "documented strict grammar" a field needs to earn a passthrough rather
// than a static-token redaction (round 6 correction): a capability_key that matches is shown as-is
// (it is exactly what the shard/doc format requires a real key to look like); anything else --
// including a genuinely malicious/secret-shaped value smuggled into a JSONL field with no shape
// enforcement of its own at parse time -- is value-free redacted, never partially shown.
//
// PER-SEGMENT length is capped (not just the whole string) because a naive "alnum/`.`/`_`/`-`,
// bounded overall length" grammar still admits a JWT: three dot-separated base64url segments are
// themselves each pure alnum+`_` (base64url's own alphabet), so a whole-string charset+length
// check alone cannot tell "policy.access.check_permission" (a real 3-segment capability key) apart
// from "<jwt-header>.<jwt-payload>.<jwt-signature>". The real repo's 2158 live capability_key
// values (measured while writing this fix) top out at 36 characters for any single segment and 48
// for the whole key; a minimal HS256 JWT signature segment alone is 43 base64url characters, and
// RS256/ES256 signatures are far longer -- so a 40-character per-segment cap sits cleanly between
// "the longest real capability_key segment ever observed" and "the shortest possible real JWT
// signature segment", rejecting every JWT shape tested without narrowing real usage at all.
// One segment is `[a-z0-9][a-z0-9_-]{0,39}` (alnum-start, <=40 chars total via the {0,39}
// continuation); the full key is 2+ such segments joined by literal dots -- `(?:\.segment)+`
// requires at least one dot, matching every real capability_key ever observed (100% have >=1).
export const CAPABILITY_KEY_ALLOW_RE = /^[a-z0-9][a-z0-9_-]{0,39}(?:\.[a-z0-9][a-z0-9_-]{0,39})+$/i
const CAPABILITY_KEY_MAX_LEN = 96

export function sanitizeCapabilityKey(value) {
  const text = typeof value === 'string' ? value : String(value ?? '')
  if (text.length > 0 && text.length <= CAPABILITY_KEY_MAX_LEN && CAPABILITY_KEY_ALLOW_RE.test(text)) return text
  return redactValue(text)
}

// repair round 7 correction (post-push independent audit): the round-6 grammar above (Rust
// identifier segments, <=40 chars each) was found to still admit plausible AWS-key/GitHub-PAT-
// shaped secrets (a bare alnum+underscore token under 40 chars satisfies it just as easily as a
// real module name does -- the exact "short alnum-only secret" gap already disclosed for
// capability_key, but here with no offsetting dotted-prefix requirement to even narrow it) WHILE
// SIMULTANEOUSLY redacting ~30 genuinely real target_module values in this repo's own shard data
// (measured: 12 distinct values in one sweep, e.g. "abuse-prevention", "nodes.knowledge" --
// target_module in practice is not always a strict Rust identifier path; real values legitimately
// use hyphens and dots this grammar's charset never allowed). No regex can fix both problems at
// once -- widening the charset to admit hyphens/dots only widens what secret shapes also pass.
// target_module is therefore ALWAYS redacted to the shared static token, with no grammar-based
// passthrough or equality/order reference. This intentionally sacrifices correlation utility so
// attacker-controlled module values cannot leak through output structure.
export function sanitizeTargetModule(_value) {
  return REDACTED_TOKEN
}

/** For genuinely free-form, UNEXPECTED diagnostic text -- a native error's `.message` (git
 * stderr, a filesystem error, an uncaught exception, an operator-supplied CLI argument that
 * failed to parse) -- where no amount of length/control-character/shape checking can prove the
 * text is safe, because its shape is not known in advance (repair round 5's own CLI test proved
 * this surface carries operator-controlled, potentially secret-shaped text: an unrecognized
 * `--flag=value` argument). Like `redactValue`, this NEVER echoes any part of the original text
 * and now (round 6 correction) never derives ANY part of its output from `rawText` either -- no
 * length, no hash, no fingerprint. It always returns the caller-supplied, FIXED (never derived
 * from `rawText`) `category` vocabulary word (e.g. `GIT_DIFF_FAILED`) followed by the same static
 * `REDACTED_TOKEN` `redactValue` uses, so the message stays actionable (the category alone says
 * what kind of failure occurred) without the round-5 format's length+hash oracle -- two different
 * `rawText` values under the same category now redact to byte-identical output, closing the
 * dictionary-attack surface a derived length/hash left open for short/low-entropy content. */
export function sanitizeErrorForDisplay(category, _rawText) {
  return `${category} ${REDACTED_TOKEN}`
}

/** Escapes a value for safe embedding in a single Markdown table cell: neutralizes `|` (breaks
 * table structure), backticks and control/newline characters (formatting/markdown-injection
 * vectors), then bounds the length. Used only for rendering already-known, already-classified
 * field values (e.g. a shard's own status text) into the CI job summary — never for arbitrary
 * shard bytes. */
export function escapeMarkdownCell(value, maxLen = BOUNDS.MAX_TEXT_FIELD_LENGTH) {
  const text = boundedText(value, maxLen)
  return text
    .replace(/\\/g, '\\\\')
    .replace(/\|/g, '\\|')
    .replace(/`/g, "'")
    .replace(/[\r\n\t]/g, ' ')
}
