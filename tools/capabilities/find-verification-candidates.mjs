#!/usr/bin/env node
// find-verification-candidates.mjs — a READ-ONLY bridge to tools/capabilities/record-verified.mjs.
//
// record-verified.mjs is deliberately not a bulk auto-promoter: it requires a real, specific
// --key/--test/--impl/--symbols per call and HARD-GUARDS by grepping the impl/test file to confirm
// the cited test symbol actually exists before allowing a "verified" stamp. That safety design is
// correct and this tool does not weaken it. What's missing is upstream of it: nothing systematically
// finds candidates to feed it, so it sits underused while docs/capabilities-canonical's
// verified_count stays stuck (28/5075 at the time this tool was written) despite real merged work.
//
// This tool finds CANDIDATES ONLY. It NEVER calls record-verified.mjs and NEVER writes to
// docs/capabilities-canonical or any DB — it is a heuristic candidate-finder, not an oracle, and its
// output must be individually confirmed by a human/Lane E session before calling record-verified.mjs
// for real. Confidence is reported honestly (HIGH/MEDIUM/LOW) based on how strong the keyword-token
// match is, never inflated to look more certain than a grep-based heuristic can be.
//
// Scope: capabilities with status IN ('implemented_unverified', 'implemented') — i.e. rows someone
// already recorded as implemented but not yet verified. This is deliberately narrower than "every
// unimplemented capability" (5047 rows in this repo): those are unbuilt-by-record, and searching test
// files for them would either find nothing (honest, but a huge slow no-op sweep) or risk false
// "candidate" noise implying work exists that doesn't. implemented_unverified is the exact set
// record-verified.mjs exists to promote.
//
// Usage:
//   node tools/capabilities/find-verification-candidates.mjs [summary|full] [--domain <name>] [--root <path>]
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(__dirname, '..', '..')

const CANDIDATE_STATUSES = new Set(['implemented_unverified', 'implemented'])
const STOPWORDS = new Set([
  'a', 'an', 'the', 'of', 'and', 'or', 'to', 'in', 'on', 'by', 'for', 'is', 'it', 'as', 'at', 'with',
  'from', 'via', 'per', 'be', 'are', 'was', 'were', 'this', 'that', 'not', 'no', 'if', 'so', 'do',
])

function loadCanonicalCapabilities(root) {
  const domainsDir = path.join(root, 'docs', 'capabilities-canonical', 'domains')
  if (!existsSync(domainsDir)) {
    throw new Error(`docs/capabilities-canonical/domains not found at ${domainsDir}`)
  }
  const files = readdirSync(domainsDir).filter((f) => f.endsWith('.jsonl'))
  const rows = []
  for (const file of files) {
    const lines = readFileSync(path.join(domainsDir, file), 'utf8').split('\n').filter((l) => l.trim().length > 0)
    for (const line of lines) rows.push(JSON.parse(line))
  }
  return rows
}

// Tokenizes a capability_key ("policy.consent_propagation_outbox") into meaningful search terms,
// dropping the domain prefix (before the first ".") since it names a TRACKING category, not
// something a Rust test/fn name would spell out, plus short/stopword tokens.
export function tokenizeCapabilityKey(key) {
  const afterDomain = key.includes('.') ? key.slice(key.indexOf('.') + 1) : key
  return afterDomain
    .split(/[._-]+/)
    .map((t) => t.toLowerCase())
    .filter((t) => t.length >= 3 && !STOPWORDS.has(t))
}

export function tokenizeName(name) {
  return String(name || '')
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((t) => t.length >= 4 && !STOPWORDS.has(t))
}

function listRustFiles(dir) {
  const out = []
  const stack = [dir]
  while (stack.length) {
    const current = stack.pop()
    let entries
    try {
      entries = readdirSync(current, { withFileTypes: true })
    } catch {
      continue
    }
    for (const entry of entries) {
      const full = path.join(current, entry.name)
      if (entry.isDirectory()) {
        if (entry.name === 'target' || entry.name === 'node_modules') continue
        stack.push(full)
      } else if (entry.isFile() && entry.name.endsWith('.rs')) {
        out.push(full)
      }
    }
  }
  return out.sort()
}

// Finds every `#[test]`-family function in a Rust source file: the attribute may be `#[test]`,
// `#[tokio::test]`, `#[actix_rt::test]`, `#[test_log::test]`, etc. — matched by "test" appearing in
// an attribute line within a few lines above `fn NAME`, the same backward-attribute-scan shape
// audit-truth.mjs already uses for #[ignore] detection, not a bare string search of the whole file.
function findTestFunctions(source) {
  const lines = source.split(/\r?\n/)
  const found = []
  const fnRe = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)/
  for (let i = 0; i < lines.length; i += 1) {
    const m = fnRe.exec(lines[i])
    if (!m) continue
    let isTest = false
    for (let j = i - 1; j >= Math.max(0, i - 6); j -= 1) {
      const line = lines[j].trim()
      if (!line) continue
      if (line.startsWith('#[') || line.startsWith('//')) {
        if (/#\[\s*[\w:]*test[\w:]*/i.test(line)) isTest = true
        continue
      }
      break
    }
    if (isTest) found.push({ name: m[1], line: i + 1 })
  }
  return found
}

function scoreTokenMatch(name, primaryTokens, secondaryTokens) {
  const normalized = name.toLowerCase()
  const primaryHits = primaryTokens.filter((t) => normalized.includes(t))
  const secondaryHits = secondaryTokens.filter((t) => !primaryTokens.includes(t) && normalized.includes(t))
  return { primaryHits, secondaryHits, score: primaryHits.length * 2 + secondaryHits.length }
}

function resolveModuleHintPaths(crateDir, targetModule) {
  if (!targetModule) return []
  const parts = String(targetModule).split('/')
  const candidates = [
    path.join(crateDir, 'src', ...parts) + '.rs',
    path.join(crateDir, 'src', ...parts, 'mod.rs'),
    path.join(crateDir, 'src', ...parts, 'lib.rs'),
  ]
  return candidates.filter((p) => existsSync(p))
}

function findCandidateImplFile(crateDir, testFilePath, targetModule) {
  const dir = path.dirname(testFilePath)
  const base = path.basename(testFilePath, '.rs')
  // "propagation_tests.rs" -> "propagation.rs"; "foo_test.rs" -> "foo.rs"
  const stripped = base.replace(/_tests?$/, '')
  if (stripped !== base) {
    const sibling = path.join(dir, `${stripped}.rs`)
    if (existsSync(sibling)) return { path: sibling, basis: 'sibling file matching test-file naming convention' }
  }
  const moduleHints = resolveModuleHintPaths(crateDir, targetModule)
  const nonTest = moduleHints.find((p) => p !== testFilePath && !/_tests?\.rs$/.test(p))
  if (nonTest) return { path: nonTest, basis: 'target_module resolves to a real file' }
  if (testFilePath !== dir) return { path: testFilePath, basis: 'no distinct impl file found; test file itself is the only match (inline #[cfg(test)] pattern is common — confirm manually)' }
  return null
}

export function findCandidatesForCrate({ root, crate, capabilities }) {
  const crateDir = path.join(root, 'crates', crate)
  if (!existsSync(crateDir)) {
    return capabilities.map((cap) => ({
      capability_key: cap.capability_key,
      target_crate: crate,
      classification: 'NO_CANDIDATE',
      reason: `crates/${crate}/ does not exist on disk`,
    }))
  }
  const rustFiles = listRustFiles(crateDir)
  const results = []
  for (const cap of capabilities) {
    const primaryTokens = tokenizeCapabilityKey(cap.capability_key)
    const secondaryTokens = tokenizeName(cap.canonical_name)
    if (primaryTokens.length === 0) {
      results.push({
        capability_key: cap.capability_key,
        target_crate: crate,
        classification: 'NO_CANDIDATE',
        reason: 'capability_key tokenized to zero usable search terms (too short/stopwords only)',
      })
      continue
    }

    // Prefer files under the target_module hint path when one resolves, but always search the whole
    // crate too -- a module hint that's slightly off (renamed dir, moved file) should not hide a real
    // match elsewhere in the same crate.
    const moduleHintFiles = new Set(resolveModuleHintPaths(crateDir, cap.target_module))

    let best = null
    for (const file of rustFiles) {
      let source
      try {
        source = readFileSync(file, 'utf8')
      } catch {
        continue
      }
      for (const testFn of findTestFunctions(source)) {
        const { primaryHits, secondaryHits, score } = scoreTokenMatch(testFn.name, primaryTokens, secondaryTokens)
        if (score === 0) continue
        const underModuleHint = moduleHintFiles.has(file)
        const candidate = { file, testFn, primaryHits, secondaryHits, score, underModuleHint }
        if (!best || score > best.score || (score === best.score && underModuleHint && !best.underModuleHint)) {
          best = candidate
        }
      }
    }

    if (!best) {
      results.push({
        capability_key: cap.capability_key,
        target_crate: crate,
        target_module: cap.target_module,
        classification: 'NO_CANDIDATE',
        reason: `no test function in crates/${crate}/**/*.rs matched any token from "${cap.capability_key}" or "${cap.canonical_name}"`,
        searched_tokens: { primary: primaryTokens, secondary: secondaryTokens },
      })
      continue
    }

    let confidence
    if (best.primaryHits.length >= 2) confidence = 'HIGH'
    else if (best.primaryHits.length >= 1 && best.underModuleHint) confidence = 'MEDIUM'
    else if (best.primaryHits.length >= 1) confidence = 'MEDIUM'
    else confidence = 'LOW' // secondary-token-only match

    const implCandidate = findCandidateImplFile(crateDir, best.file, cap.target_module)

    results.push({
      capability_key: cap.capability_key,
      target_crate: crate,
      target_module: cap.target_module,
      canonical_name: cap.canonical_name,
      status: cap.status,
      classification: 'CANDIDATE',
      confidence,
      candidate_test_file: path.relative(root, best.file).split(path.sep).join('/'),
      candidate_test_symbol: best.testFn.name,
      candidate_test_line: best.testFn.line,
      candidate_impl_file: implCandidate ? path.relative(root, implCandidate.path).split(path.sep).join('/') : null,
      candidate_impl_basis: implCandidate ? implCandidate.basis : 'no candidate impl file found',
      matched_tokens: { primary: best.primaryHits, secondary: best.secondaryHits },
      under_target_module_hint: best.underModuleHint,
      note:
        `Heuristic keyword match only (${confidence} confidence) -- NOT verified. Before calling record-verified.mjs: ` +
        `open ${path.relative(root, best.file).split(path.sep).join('/')}, confirm \`${best.testFn.name}\` actually proves ` +
        `"${cap.canonical_name}", identify the REAL --impl file and --symbols (this tool does not enumerate exact impl symbols), ` +
        `and if moves_money=1 confirm a real --fin financial-control test exists.`,
    })
  }
  return results
}

export function findVerificationCandidates({ root = ROOT, domain = null } = {}) {
  const all = loadCanonicalCapabilities(root)
  const targets = all.filter((c) => CANDIDATE_STATUSES.has(c.status) && (!domain || c.domain === domain))
  const byCrate = new Map()
  for (const cap of targets) {
    if (!cap.target_crate) continue
    if (!byCrate.has(cap.target_crate)) byCrate.set(cap.target_crate, [])
    byCrate.get(cap.target_crate).push(cap)
  }
  const noTargetCrate = targets.filter((c) => !c.target_crate).map((c) => ({
    capability_key: c.capability_key,
    target_crate: null,
    classification: 'NO_CANDIDATE',
    reason: 'no target_crate assigned in docs/capabilities-canonical -- nothing to search',
  }))

  const results = [...noTargetCrate]
  for (const [crate, capabilities] of [...byCrate.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
    results.push(...findCandidatesForCrate({ root, crate, capabilities }))
  }
  results.sort((a, b) => a.capability_key.localeCompare(b.capability_key))

  const candidates = results.filter((r) => r.classification === 'CANDIDATE')
  const byConfidence = { HIGH: 0, MEDIUM: 0, LOW: 0 }
  for (const c of candidates) byConfidence[c.confidence] += 1

  return {
    schema_version: 1,
    generated_at: null, // stamped by main()/caller
    scope: {
      statuses_searched: [...CANDIDATE_STATUSES],
      domain_filter: domain,
      total_capabilities_in_scope: targets.length,
    },
    truth_label: 'HEURISTIC_CANDIDATE_ONLY',
    authority:
      'Read-only. Reads docs/capabilities-canonical/**/*.jsonl and crates/**/*.rs only -- never ' +
      'docs/capabilities.db, never docs/architecture.db, never writes anything, never calls ' +
      'record-verified.mjs. Every CANDIDATE row is a keyword-match heuristic, not a verified fact.',
    counts: {
      total: results.length,
      candidates: candidates.length,
      no_candidate: results.length - candidates.length,
      by_confidence: byConfidence,
    },
    results,
    non_claims: [
      'This tool never calls tools/capabilities/record-verified.mjs and never writes to docs/capabilities-canonical, docs/capabilities.db, or any other tracking surface.',
      'A CANDIDATE row is a keyword-token match between a capability_key/canonical_name and a #[test]-family function name found by grep-style scanning -- it is not proof the test actually proves the capability.',
      'candidate_impl_file is a best-guess sibling/module-hint file, not a confirmed --impl argument -- record-verified.mjs itself will refuse an incorrect one when the real call is made.',
      'This tool does not enumerate exact impl symbols (--symbols); a human must read the candidate impl file and confirm them before calling record-verified.mjs.',
      'Scope is intentionally limited to status IN (implemented_unverified, implemented) -- it does not search the 5047 unimplemented capabilities for accidentally-already-built code (a different, larger problem: sync-anchor-v2 / SHARD_CLAIMS_UNIMPLEMENTED_BUT_CODE_IS_REAL, out of scope here).',
    ],
  }
}

function printSummary(result) {
  console.log(`find-verification-candidates: ${result.scope.total_capabilities_in_scope} capability(ies) in scope (status IN ${JSON.stringify(result.scope.statuses_searched)})`)
  console.log(`  candidates found: ${result.counts.candidates}/${result.counts.total}`)
  console.log(`    HIGH: ${result.counts.by_confidence.HIGH}  MEDIUM: ${result.counts.by_confidence.MEDIUM}  LOW: ${result.counts.by_confidence.LOW}`)
  console.log(`  no candidate: ${result.counts.no_candidate}`)
  for (const r of result.results) {
    if (r.classification === 'CANDIDATE') {
      console.log(`  [${r.confidence}] ${r.capability_key} (${r.target_crate}) -> test ${r.candidate_test_symbol} @ ${r.candidate_test_file}:${r.candidate_test_line}`)
    }
  }
}

function usage() {
  return 'Usage: node tools/capabilities/find-verification-candidates.mjs [summary|full] [--domain <name>] [--root <path>]'
}

function main() {
  const argv = process.argv.slice(2)
  const cmd = argv.find((a) => !a.startsWith('--')) || 'summary'
  const domainFlagIndex = argv.indexOf('--domain')
  const domain = domainFlagIndex >= 0 ? argv[domainFlagIndex + 1] : null
  const rootFlagIndex = argv.indexOf('--root')
  const root = rootFlagIndex >= 0 ? path.resolve(argv[rootFlagIndex + 1]) : ROOT

  let result
  try {
    result = findVerificationCandidates({ root, domain })
  } catch (error) {
    console.error(`find-verification-candidates failed: ${error.message}`)
    process.exitCode = 1
    return
  }
  result.generated_at = new Date().toISOString()

  if (cmd === 'summary') {
    printSummary(result)
  } else if (cmd === 'full') {
    console.log(JSON.stringify(result, null, 2))
  } else {
    console.error(usage())
    process.exitCode = 2
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
