#!/usr/bin/env node
// scan-verified-citations.mjs — audit that every `verified` capability row's canonical JSONL
// citations still resolve to REAL code: each cited `code_ref` / `test_ref` FILE must exist, and
// every symbol named in a citation must appear in that file. Complements
// verify-impl-evidence.mjs, which audits the *derived SQLite* `impl_evidence` table; this tool
// audits the canonical JSONL shards directly (meta.json `authority: canonical_jsonl`), which is
// what the tracker-integrity mandate's "grep every cited code_ref symbol / test_ref" step targets.
//
// Read-only. Exits 1 on any dangling citation (renamed/deleted symbol or file), so it is a
// CI-gateable, repeatable gate: `npm run caps:check-citations`.
//
// PROSE-ROBUSTNESS (the reason this is a real tool and not a one-line grep): a `#` fragment is a
// symbol list in most rows (`existence_probes.rs#SiteCheck,classify_site`) but freeform provenance
// prose in others (`probe_x.rs#main (real, non-test caller: ... reaches enumerate, build_probe_request,
// Prober ...)`). Naively comma-splitting the prose yields bare identifiers (`build_probe_request`)
// that are NOT symbols cited for *this* file and produces false "missing symbol" findings — which,
// acted on, would drive a false demotion of a correctly-cited row. This tool only treats a fragment
// as a symbol list when EVERY comma-separated token is a Rust identifier or `Type::method` path;
// anything else is prose and contributes no symbol checks (the file-existence check still applies).
//
// Usage: node tools/capabilities/scan-verified-citations.mjs [--domain=<name>] [--json]
import { readFileSync, existsSync } from 'node:fs'
import { loadCapabilityShards, parseArgs } from '../canonical-shards/jsonl-lib.mjs'

// A source file whose contents we can meaningfully grep for a symbol.
const CHECKABLE_EXT = /\.(rs|ts|tsx|mjs|js|py|sql|toml|md|json)$/
// One symbol token: a Rust/TS identifier, optionally a `Type::method::...` path.
const SYMBOL_TOKEN = /^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*$/

export function isSymbolToken(tok) {
  return typeof tok === 'string' && SYMBOL_TOKEN.test(tok)
}

// Return the array of symbol tokens IFF the fragment is a clean comma-separated symbol list;
// otherwise return [] (prose, mixed, or empty). This is the prose-robustness guard.
export function symbolsFromFragment(fragment) {
  if (!fragment) return []
  const toks = fragment.split(',').map((s) => s.trim()).filter(Boolean)
  if (!toks.length) return []
  if (!toks.every(isSymbolToken)) return []
  return toks
}

// Split a raw citation into { path, symbols }. Handles the six real-world forms:
//   file.rs                          -> path only
//   file.rs#A,B,C                    -> hash + symbol list
//   file.rs#Type::method,C           -> hash + Type::method list
//   file.rs#main (real, ... prose)   -> hash + PROSE (no symbols)
//   file.rs (A, B, fn_c)             -> path + parenthetical symbol-list annotation
//   file.rs:symbol                   -> colon form
export function parseCitation(ref) {
  const s = ref.trim()
  const hash = s.indexOf('#')
  if (hash >= 0) {
    return { path: s.slice(0, hash).trim(), symbols: symbolsFromFragment(s.slice(hash + 1)) }
  }
  // `path.ext (A, B, C)` — trailing parenthetical annotation.
  const paren = s.match(/^(.*\.[A-Za-z0-9]+)\s*\((.+)\)\s*$/)
  if (paren && CHECKABLE_EXT.test(paren[1].trim())) {
    return { path: paren[1].trim(), symbols: symbolsFromFragment(paren[2]) }
  }
  // `path.ext:symbol[,symbol...]` — colon form (extension immediately before the colon).
  const colon = s.match(/^(.*\.(?:rs|ts|tsx|mjs|js|py|sql|toml|md|json)):(.+)$/)
  if (colon) {
    return { path: colon[1].trim(), symbols: symbolsFromFragment(colon[2]) }
  }
  return { path: s, symbols: [] }
}

// The leaf of a `Type::method` path is the most specific real symbol; checking it (rather than
// every path segment) minimizes false positives from generated/re-exported intermediate modules.
function leafSymbol(sym) {
  const parts = sym.split('::')
  return parts[parts.length - 1]
}

export function findMissing(entries, { domainFilter = null } = {}) {
  const missingFile = []
  const missingSymbol = []
  let checkedRows = 0
  const cache = new Map()
  const read = (p) => {
    if (cache.has(p)) return cache.get(p)
    const v = existsSync(p) ? readFileSync(p, 'utf8') : null
    cache.set(p, v)
    return v
  }

  for (const { record: r } of entries) {
    if (!r || r.status !== 'verified') continue
    if (domainFilter && r.domain !== domainFilter) continue
    checkedRows++
    const refs = [
      ...(r.code_refs ?? []).map((x) => ['code', x]),
      ...(r.test_refs ?? []).map((x) => ['test', x]),
    ]
    for (const [kind, ref] of refs) {
      const { path, symbols } = parseCitation(ref)
      if (!path.includes('/') || !CHECKABLE_EXT.test(path)) continue
      const body = read(path)
      if (body == null) {
        missingFile.push({ key: r.capability_key, domain: r.domain, kind, path })
        continue
      }
      for (const sym of symbols) {
        const leaf = leafSymbol(sym)
        if (!new RegExp(`\\b${leaf}\\b`).test(body)) {
          missingSymbol.push({ key: r.capability_key, domain: r.domain, kind, path, sym })
        }
      }
    }
  }
  return { checkedRows, missingFile, missingSymbol }
}

// Only run the repo sweep when invoked as a script; importing for tests must not run it.
if (process.argv[1] && import.meta.url === `file://${process.argv[1]}`) {
  const args = parseArgs(process.argv)
  const { entries } = loadCapabilityShards(process.cwd())
  const result = findMissing(entries, { domainFilter: args.domain ?? null })
  if (args.json) {
    console.log(JSON.stringify(result, null, 2))
  } else {
    console.log(`checked ${result.checkedRows} verified rows`)
    console.log(`MISSING FILES: ${result.missingFile.length}`)
    for (const m of result.missingFile) console.log(`  [${m.domain}] ${m.key} (${m.kind}) -> ${m.path}`)
    console.log(`MISSING SYMBOLS: ${result.missingSymbol.length}`)
    for (const m of result.missingSymbol) console.log(`  [${m.domain}] ${m.key} (${m.kind}) ${m.path} # ${m.sym}`)
  }
  const total = result.missingFile.length + result.missingSymbol.length
  if (total > 0) {
    console.error(`FAIL: ${total} dangling citation(s) on verified rows`)
    process.exit(1)
  }
  console.error('OK: all verified-row citations resolve')
}
