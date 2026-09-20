#!/usr/bin/env node
// Scans every `verified` capability row's `.rs` code_refs for a real shipped caller
// (docs/doctrines/2026-07-12-runtime-reachability-doctrine.md's ISLAND classification).
//
// This is the corrected version of the ad-hoc script behind
// docs/_machine/repo-wide-verified-island-audit-2026-08-18.md. That first pass had two bugs,
// both fixed here (see that doc's own §6 for the root-cause writeup):
//
//   1. Same-crate check used ONE combined alternation regex per crate over every tracked module
//      name, then picked whichever alternative `re.search` matched leftmost in a hit line. When a
//      nested module's intermediate path segment collided with another tracked module name in the
//      same crate (e.g. `contracts::media` vs. a separately-tracked sibling `contracts/mod.rs`),
//      the hit got attributed to the wrong module. Fixed here by grepping each module individually
//      instead of batching into one alternation.
//   2. Cross-crate check searched for the literal contiguous string `crate_pkg::modname`, which
//      never matches real qualified paths with an intermediate module segment
//      (`chronica_esign::slice2::pdf_fields`). Fixed here with `crate_pkg::(\w+::)*modname\b`.
//
// A third gap (not a bug in the original, just a limitation) is also addressed: a symbol
// re-exported through several module levels and then imported+used by its short flattened name
// leaves no literal `modname::` text at the call site at all. See `checkSymbolCalled` below.
//
// Usage: node tools/capabilities/scan-verified-callers.mjs [--domain=<name>] [--key=<capability_key>]
//   --domain filters to one domain shard (e.g. --domain=media-generation).
//   --key filters to one capability_key (for re-checking a single row).
// Output: JSON to stdout, one object per scanned row: { key, domain, classification, hits }.
// classification is one of HAS_REAL_CALLER | PUB_USE_ONLY | ISLAND | AMBIGUOUS.
//
// This tool does NOT change any status -- it only reports. Per the audit doc's own conclusion,
// every row this flags must be hand-verified before its status is touched; false positives are
// expected and confirmed to occur even with these fixes (dynamic dispatch, macro-generated call
// sites, and other patterns no static grep can see).

import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { loadCapabilityShards, parseArgs, rel } from '../canonical-shards/jsonl-lib.mjs'

const args = parseArgs(process.argv)
const domainFilter = args.domain ?? null
const keyFilter = args.key ?? null

const MIN_MODULE_NAME_LENGTH = 4

// A code_ref's `#` fragment is sometimes a comma-separated symbol list
// (`foo.rs#bar,baz`), but many rows use it for free-text provenance prose
// (`foo.rs#main (real, non-test caller: shells out to curl ...)`). Only real
// Rust identifiers can be module/symbol names; anything else (prose, paths,
// punctuation) must never reach `grep -E`, or it builds an invalid pattern
// (`[.:]main (real\s*\(` -> "grep: Unmatched ( or \(") and crashes the scan.
// Validating identifier shape both fixes that crash and correctly ignores prose.
function isRustIdent(name) {
  return typeof name === 'string' && /^[A-Za-z_][A-Za-z0-9_]*$/.test(name)
}

function isTestFile(path) {
  const p = path.toLowerCase()
  return (
    p.includes('/tests/') ||
    p.endsWith('_tests.rs') ||
    p.endsWith('_test.rs') ||
    p.includes('/test_') ||
    p.endsWith('/tests.rs')
  )
}

function isDocOrCommentLine(text) {
  const t = text.trim()
  return t.startsWith('///') || t.startsWith('//!') || t.startsWith('//') || t.startsWith('*') || t.startsWith('/*')
}

function grepLines(pattern, target) {
  try {
    const out = execFileSync('grep', ['-rn', '-E', pattern, target], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
    return out.split('\n').filter(Boolean)
  } catch (err) {
    // grep exits 1 when there are no matches -- not an error for us.
    if (err.status === 1) return []
    throw err
  }
}

function moduleNameFor(path) {
  const base = path.split('/').pop().replace(/\.rs$/, '')
  if (base === 'mod' || base === 'lib') {
    const parts = path.split('/')
    return parts[parts.length - 2] ?? base
  }
  return base
}

function checkOneModule(path, modName) {
  // Defensive: never embed a non-identifier module name in a grep -E pattern.
  if (!isRustIdent(modName)) return { realHits: [], pubUseHits: [] }
  const crateRoot = path.split('/src/')[0]
  const cratePkg = crateRoot.split('/').pop().replaceAll('-', '_')

  // Same-crate: grep for this module's name alone (no batching -- avoids the leftmost-match
  // misattribution bug when another tracked module in the same crate shares a substring).
  const sameCratePattern = String.raw`\b${modName}::`
  const sameCrateHits = grepLines(sameCratePattern, join(crateRoot, 'src'))

  let realHits = []
  let pubUseHits = []
  for (const line of sameCrateHits) {
    const idx1 = line.indexOf(':')
    const idx2 = line.indexOf(':', idx1 + 1)
    if (idx1 < 0 || idx2 < 0) continue
    const fpath = line.slice(0, idx1)
    const content = line.slice(idx2 + 1)
    if (fpath === path) continue
    if (isTestFile(fpath)) continue
    if (isDocOrCommentLine(content)) continue
    if (new RegExp(String.raw`^\s*pub\s+use\s+${modName}::`).test(content)) {
      pubUseHits.push({ file: fpath, content: content.trim() })
      continue
    }
    if (new RegExp(String.raw`^\s*(pub\s+)?mod\s+${modName}\s*;`).test(content)) continue
    realHits.push({ file: fpath, content: content.trim() })
  }

  // Cross-crate: allow any number of intermediate module segments between the crate name and the
  // target module (this is the fix for bug #2).
  // grep -E is POSIX ERE, not PCRE -- no (?:...) non-capturing groups, use a plain group instead.
  const crossCratePattern = String.raw`${cratePkg}::(\w+::)*${modName}\b`
  const crossHitsRaw = grepLines(crossCratePattern, 'crates/')
  for (const line of crossHitsRaw) {
    const idx1 = line.indexOf(':')
    const idx2 = line.indexOf(':', idx1 + 1)
    if (idx1 < 0 || idx2 < 0) continue
    const fpath = line.slice(0, idx1)
    const content = line.slice(idx2 + 1)
    if (fpath.startsWith(crateRoot + '/')) continue // same-crate hits already covered above
    if (isTestFile(fpath)) continue
    if (isDocOrCommentLine(content)) continue
    realHits.push({ file: fpath, content: content.trim() })
  }

  return { realHits, pubUseHits }
}

// Supplementary check for re-export-chain cases the module-path check can't see: a symbol
// (function/method/struct) re-exported through several levels (module -> parent mod -> crate
// root) and then imported+used by its short flattened name in another crate, with no literal
// `modname::` text anywhere at the call site. Checks workspace-wide for the bare symbol name used
// in a call-like position (`symbol(` or `.symbol(`), which a mere re-export or type reference
// would not produce.
function checkSymbolCalled(symbolName, ownFilePath, searchRoot = 'crates/') {
  if (!isRustIdent(symbolName)) return []
  if (symbolName.length < MIN_MODULE_NAME_LENGTH) return []
  // Match any call-like position: bare `symbol(` (a free fn/tuple-struct imported by its flattened
  // name and called with no path prefix), `.symbol(` (method), and `::symbol(` (path). The earlier
  // `[.:]symbol(` form REQUIRED a `.`/`:` immediately before the name, so it silently missed the
  // single most common flattened-caller shape -- `use crate::foo; ... foo(args)` -- and produced
  // systematic false PUB_USE_ONLY/ISLAND classifications (e.g. `apply_stealth(...)`,
  // `natural_balance_on(...)`, `commerce_growth_loop_report(...)`). `\b` still prevents matching a
  // longer identifier that merely ends in the symbol (`other_foo(`); the definition line and
  // `use`/comment/test lines are filtered out below.
  const pattern = String.raw`\b${symbolName}\s*\(`
  const hits = grepLines(pattern, searchRoot)
  const real = []
  for (const line of hits) {
    const idx1 = line.indexOf(':')
    const idx2 = line.indexOf(':', idx1 + 1)
    if (idx1 < 0 || idx2 < 0) continue
    const fpath = line.slice(0, idx1)
    const content = line.slice(idx2 + 1)
    if (fpath === ownFilePath) continue
    if (isTestFile(fpath)) continue
    if (isDocOrCommentLine(content)) continue
    if (new RegExp(String.raw`^\s*(pub\s+)?fn\s+${symbolName}\b`).test(content)) continue // the fn definition itself
    if (new RegExp(String.raw`^\s*(pub\s+)?(struct|enum)\s+${symbolName}\b`).test(content)) continue // tuple-struct/enum def, not a call
    real.push({ file: fpath, content: content.trim() })
  }
  return real
}

// A code_ref is `path[#symbols]` but a substantial minority of rows use the
// `path.rs:symbol` colon form instead of `#` (e.g. osint's
// `security_tool_wrappers.rs:classify_wafw00f_response`). Both forms must be
// parsed identically, or the colon-form rows silently drop all their symbols and
// mis-classify as ISLAND. The symbol segment is always comma-separated; only
// identifier-shaped tokens survive (prose/line-numbers/`main (real ...)` do not).
function parseRef(ref) {
  const hashIdx = ref.indexOf('#')
  let path
  let frag
  if (hashIdx >= 0) {
    path = ref.slice(0, hashIdx).trim()
    frag = ref.slice(hashIdx + 1)
  } else {
    // `foo.rs:bar` -- extension immediately followed by a colon and an identifier.
    const m = ref.match(/^(.*\.(?:rs|ts|tsx|mjs|js|py|sql|toml|md|json)):(.+)$/)
    if (m) {
      path = m[1].trim()
      frag = m[2]
    } else {
      path = ref.trim()
      frag = ''
    }
  }
  const symbols = frag
    .split(',')
    .map((s) => s.trim())
    .filter((s) => isRustIdent(s))
  return { path, symbols }
}

// A `crates/<crate>/src/bin/<name>.rs` entry is a Cargo binary target: a shipped,
// non-test entry point (`cargo build` produces it). Per the runtime-reachability
// doctrine (§2.2 "caller is part of a built binary or deployed product path"), a
// capability cited from such a bin has, structurally, a shipped caller. The
// module-path grep can never see this -- bin modules are entry points, never
// imported by `binname::` anywhere -- so without this short-circuit every
// bin-backed row (e.g. osint's `probe_*` reachability bins) falsely reads as ISLAND.
function shippedBinRef(codeRefs) {
  for (const ref of codeRefs) {
    const { path } = parseRef(ref)
    if (/\/src\/bin\/[^/]+\.rs$/.test(path) && existsSync(path)) return path
  }
  return null
}

function classifyRow(record) {
  const codeRefs = record.code_refs ?? []

  const binPath = shippedBinRef(codeRefs)
  if (binPath) {
    return { classification: 'HAS_REAL_CALLER', hits: [{ file: binPath, content: 'shipped bin target (cargo bin entry point)', module: 'src/bin' }] }
  }

  const moduleChecks = []
  for (const ref of codeRefs) {
    const { path, symbols } = parseRef(ref)
    if (!path.endsWith('.rs')) continue
    if (!existsSync(path)) continue
    if (!path.includes('/src/')) continue
    const modName = moduleNameFor(path)
    moduleChecks.push({ path, modName, symbols })
  }
  if (moduleChecks.length === 0) return null

  let anyReal = false
  let anyPubUse = false
  let anyChecked = false
  const allHits = []
  for (const { path, modName, symbols } of moduleChecks) {
    if (modName.length < MIN_MODULE_NAME_LENGTH) continue
    anyChecked = true
    const { realHits, pubUseHits } = checkOneModule(path, modName)
    if (realHits.length) {
      anyReal = true
      allHits.push(...realHits.slice(0, 3).map((h) => ({ ...h, module: modName })))
      continue
    }
    // Module-path check found nothing conclusive (pure island or re-export-only) -- before
    // accepting that, check whether any individual declared symbol is actually CALLED elsewhere
    // (the re-export-chain case a module-path-only check can't see: a symbol re-exported through
    // several levels, e.g. `module -> parent_mod -> crate_root`, then imported by its short
    // flattened name at the call site, with no literal `modname::` text anywhere).
    let foundViaSymbol = false
    for (const sym of symbols.slice(0, 5)) {
      const symHits = checkSymbolCalled(sym, path)
      if (symHits.length) {
        anyReal = true
        foundViaSymbol = true
        allHits.push(...symHits.slice(0, 2).map((h) => ({ ...h, module: `${modName}::${sym}` })))
        break
      }
    }
    if (foundViaSymbol) continue
    if (pubUseHits.length) {
      anyPubUse = true
      allHits.push(...pubUseHits.slice(0, 1).map((h) => ({ ...h, module: modName })))
    }
  }

  let classification
  if (!anyChecked) classification = 'AMBIGUOUS'
  else if (anyReal) classification = 'HAS_REAL_CALLER'
  else if (anyPubUse) classification = 'PUB_USE_ONLY'
  else classification = 'ISLAND'

  return { classification, hits: allHits }
}

// Exported for tools/capabilities/scan-verified-callers.test.mjs -- the pure,
// filesystem-independent parsing/classification helpers are unit-tested there so
// the crash fix and the shipped-bin / colon-form rules stay a repeatable gate.
export { isRustIdent, parseRef, shippedBinRef, moduleNameFor, classifyRow, checkSymbolCalled }

function runMain() {
  const { entries } = loadCapabilityShards(process.cwd())
  const results = []

  for (const entry of entries) {
    const record = entry.record
    if (!record || record.status !== 'verified') continue
    if (keyFilter && record.capability_key !== keyFilter) continue
    const domain = record.domain ?? rel(process.cwd(), entry.file)
    if (domainFilter && domain !== domainFilter) continue

    const result = classifyRow(record)
    if (!result) continue
    results.push({ key: record.capability_key, domain, ...result })
  }

  console.log(JSON.stringify(results, null, 2))
  console.error(`scanned ${results.length} verified rows with checkable .rs code_refs`)
  const counts = results.reduce((acc, r) => {
    acc[r.classification] = (acc[r.classification] ?? 0) + 1
    return acc
  }, {})
  console.error(JSON.stringify(counts))
}

// Only sweep the repo when invoked as a script; importing for tests must not run it.
if (process.argv[1] && import.meta.url === `file://${process.argv[1]}`) {
  runMain()
}
