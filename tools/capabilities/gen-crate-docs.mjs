#!/usr/bin/env node
// gen-crate-docs.mjs
//
// Generate one 24-section crate atom doc per live chronica-* crate from the
// real code tree plus capabilities.db and architecture.db. These DBs are
// build-time/audit-time tracking inputs only; generated crate docs must never
// imply product runtime code reads them. The generated layer is deterministic
// and preserves each hand-written rationale block.
import Database from 'better-sqlite3'
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { join, relative, sep } from 'node:path'
import { CAPABILITIES_DB, ARCHITECTURE_DB, CRATE_DOCS } from '../_paths.mjs'

const ROOT = process.cwd()
const CRATES = join(ROOT, 'crates')
const DOCS = join(ROOT, 'docs')
const REPORTS = join(DOCS, '_machine', 'reconcile-reports')
const ARCHIVE = join(DOCS, '_archive', 'crate-docs-pre-v97-2026-07-07')
const CHECK = process.argv.includes('--check')
mkdirSync(ARCHIVE, { recursive: true })
mkdirSync(REPORTS, { recursive: true })
mkdirSync(CRATE_DOCS, { recursive: true })

const slash = (p) => p.split(sep).join('/')
const rel = (p) => slash(relative(ROOT, p))
const esc = (s) => String(s ?? '').replaceAll('|', '\\|').replaceAll('\n', ' ')
const code = (s) => `\`${s}\``
const list = (items, empty = '_none_') => items.length ? items.map((x) => `- ${x}`).join('\n') : empty
const csvCode = (items, empty = '_none_') => items.length ? items.map(code).join(', ') : empty
const first = (items, n) => items.slice(0, n)

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms)
}

function writeText(path, body) {
  let lastError = null
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      writeFileSync(path, body)
      return
    } catch (error) {
      lastError = error
      if (!['UNKNOWN', 'EBUSY', 'EPERM', 'EACCES'].includes(error.code)) throw error
      sleep(50 * (attempt + 1))
    }
  }
  throw lastError
}

function readCargo(crateName) {
  const path = join(CRATES, crateName, 'Cargo.toml')
  const text = readFileSync(path, 'utf8')
  const name = text.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1] || crateName
  const deps = [...text.matchAll(/^\s*(chronica-[a-z0-9-]+)\s*[=.]/gm)]
    .map((m) => m[1])
    .filter((d) => d !== name)
  const features = [...text.matchAll(/^\s*([a-zA-Z0-9_-]+)\s*=\s*\[/gm)]
    .map((m) => m[1])
    .filter((f) => !['package', 'dependencies', 'dev-dependencies'].includes(f))
  return { path, text, name, deps: [...new Set(deps)].sort(), features: [...new Set(features)].sort() }
}

function rustFiles(crateName) {
  const src = join(CRATES, crateName, 'src')
  const out = []
  const walk = (dir) => {
    if (!existsSync(dir)) return
    for (const ent of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, ent.name)
      if (ent.isDirectory()) walk(full)
      else if (ent.isFile() && ent.name.endsWith('.rs')) out.push(full)
    }
  }
  walk(src)
  return out.sort((a, b) => rel(a).localeCompare(rel(b)))
}

function readLib(crateName) {
  const path = join(CRATES, crateName, 'src', 'lib.rs')
  if (!existsSync(path)) return { path, role: '', api: [] }
  const lines = readFileSync(path, 'utf8').split(/\r?\n/)
  const role = lines
    .slice(0, 24)
    .map((ln) => ln.match(/^\s*\/\/!\s?(.*)/)?.[1]?.trim())
    .find(Boolean) || ''
  const api = lines
    .filter((l) => /^\s*pub (use|mod|fn|struct|enum|trait|type|const) /.test(l))
    .map((l) => l.trim().replace(/\s*\{.*$/, '').replace(/;$/, ''))
    .slice(0, 80)
  return { path, role, api }
}

function topLevelModules(crateName) {
  const src = join(CRATES, crateName, 'src')
  if (!existsSync(src)) return []
  const entries = readdirSync(src, { withFileTypes: true })
  const files = entries
    .filter((e) => e.isFile() && e.name.endsWith('.rs') && !['lib.rs', 'main.rs'].includes(e.name))
    .map((e) => e.name.replace(/\.rs$/, ''))
  const dirs = entries.filter((e) => e.isDirectory()).map((e) => `${e.name}/`)
  return [...files, ...dirs].sort()
}

function testSymbols(files) {
  const tests = []
  for (const file of files) {
    const src = readFileSync(file, 'utf8')
    for (const m of src.matchAll(/#\s*\[\s*(?:tokio::)?test(?:\s*\([^)]*\))?\s*\][\s\S]{0,240}?\bfn\s+([a-zA-Z0-9_]+)/g)) {
      tests.push({ file: rel(file), symbol: m[1], async: m[0].includes('tokio::test') })
    }
  }
  return tests.sort((a, b) => `${a.file}:${a.symbol}`.localeCompare(`${b.file}:${b.symbol}`))
}

function sourceRows(crateName, files) {
  const key = [
    join(CRATES, crateName, 'Cargo.toml'),
    join(CRATES, crateName, 'src', 'lib.rs'),
    join(CRATES, crateName, 'src', 'main.rs'),
    ...files,
  ].filter((p, i, arr) => existsSync(p) && arr.indexOf(p) === i)
  return first(key, 30).map((p) => ({
    path: rel(p),
    size: statSync(p).size,
    role: p.endsWith('Cargo.toml') ? 'manifest' : p.endsWith('lib.rs') ? 'library root' : p.endsWith('main.rs') ? 'binary root' : 'module',
  }))
}

function preserveRationale(file) {
  if (!existsSync(file)) return ''
  const text = readFileSync(file, 'utf8')
  return text.match(/<!-- rationale:start -->([\s\S]*?)<!-- rationale:end -->/)?.[1]?.trim() || ''
}

function table(headers, rows) {
  if (!rows.length) return '_none_'
  return [
    `| ${headers.join(' |')} |`,
    `| ${headers.map(() => '---').join(' |')} |`,
    ...rows.map((r) => `| ${r.map((c) => esc(c)).join(' |')} |`),
  ].join('\n')
}

function existingCrateDocs() {
  const byCrate = {}
  const all = []
  for (const f of readdirSync(CRATE_DOCS)) {
    const m = f.match(/^(\d{3})-crate-(chronica-[a-z0-9-]+)\.md$/)
    if (!m) continue
    byCrate[m[2]] = { num: m[1], file: f }
    all.push({ num: m[1], crate: m[2], file: f })
  }
  return { byCrate, all }
}

const crates = readdirSync(CRATES)
  .filter((d) => existsSync(join(CRATES, d, 'Cargo.toml')))
  .sort()
const { byCrate: docByCrate, all: allCrateDocs } = existingCrateDocs()
let next = 300
const used = new Set(allCrateDocs.map((d) => Number(d.num)))
function assignNumber(crateName) {
  if (docByCrate[crateName]) return docByCrate[crateName].num
  while (used.has(next)) next += 1
  used.add(next)
  return String(next).padStart(3, '0')
}

const caps = new Database(CAPABILITIES_DB, { readonly: true })
const arch = new Database(ARCHITECTURE_DB, { readonly: true })
const q = (db, sql) => { try { return db.prepare(sql) } catch { return null } }
const hasCapsTable = (name) => caps.prepare("SELECT count(*) n FROM sqlite_master WHERE type='table' AND name=?").get(name).n > 0
const hasCanonicalCaps = hasCapsTable('canonical_capability')
const capRowsQ = hasCanonicalCaps ? q(caps, `
  SELECT key, canonical_name, domain, moves_money, side_effect_class, status,
         target_module, acceptance_test, financial_control_test
  FROM canonical_capability
  WHERE target_crate=?
  ORDER BY (status='verified') DESC, moves_money DESC, domain, canonical_name
`) : q(caps, `
  SELECT sc.canonical_name AS key,
         sc.canonical_name AS canonical_name,
         COALESCE(NULLIF(substr(sc.canonical_name, 1, instr(sc.canonical_name || '.', '.') - 1), ''), 'other') AS domain,
         COALESCE(co.moves_money, max(sc.moves_money), 0) AS moves_money,
         COALESCE(max(sc.side_effect_class), '') AS side_effect_class,
         COALESCE(co.status, 'unimplemented') AS status,
         COALESCE(max(sc.target_module), '') AS target_module,
         co.acceptance_test AS acceptance_test,
         co.financial_control_test AS financial_control_test
  FROM source_capability sc
  LEFT JOIN canonical_status_override co ON co.canonical_key=sc.canonical_name
  WHERE sc.target_crate=?
  GROUP BY sc.canonical_name
  ORDER BY (COALESCE(co.status, 'unimplemented')='verified') DESC,
           COALESCE(co.moves_money, max(sc.moves_money), 0) DESC,
           domain,
           sc.canonical_name
`)
const capCountQ = hasCanonicalCaps ? q(caps, `
  SELECT count(*) n,
         COALESCE(sum(moves_money),0) money,
         COALESCE(sum(status='verified'),0) verified
  FROM canonical_capability WHERE target_crate=?
`) : q(caps, `
  SELECT count(*) n,
         COALESCE(sum(moves_money),0) money,
         COALESCE(sum(status='verified'),0) verified
  FROM (
    SELECT sc.canonical_name,
           COALESCE(co.moves_money, max(sc.moves_money), 0) AS moves_money,
           COALESCE(co.status, 'unimplemented') AS status
    FROM source_capability sc
    LEFT JOIN canonical_status_override co ON co.canonical_key=sc.canonical_name
    WHERE sc.target_crate=?
    GROUP BY sc.canonical_name
  )
`)
const capDomainsQ = hasCanonicalCaps ? q(caps, `
  SELECT domain, count(*) n, COALESCE(sum(status='verified'),0) verified
  FROM canonical_capability WHERE target_crate=?
  GROUP BY domain ORDER BY n DESC, domain
`) : q(caps, `
  SELECT domain, count(*) n, COALESCE(sum(status='verified'),0) verified
  FROM (
    SELECT sc.canonical_name,
           COALESCE(NULLIF(substr(sc.canonical_name, 1, instr(sc.canonical_name || '.', '.') - 1), ''), 'other') AS domain,
           COALESCE(co.status, 'unimplemented') AS status
    FROM source_capability sc
    LEFT JOIN canonical_status_override co ON co.canonical_key=sc.canonical_name
    WHERE sc.target_crate=?
    GROUP BY sc.canonical_name
  )
  GROUP BY domain ORDER BY n DESC, domain
`)
const archNodeQ = q(arch, 'SELECT * FROM architecture_node WHERE id=?')
const outEdgesQ = q(arch, "SELECT DISTINCT to_node FROM architecture_edge WHERE from_node=? AND to_node LIKE 'crate:%' ORDER BY to_node")
const inEdgesQ = q(arch, "SELECT DISTINCT from_node FROM architecture_edge WHERE to_node=? AND from_node LIKE 'crate:%' ORDER BY from_node")
const moduleNQ = q(arch, "SELECT count(*) n FROM architecture_node WHERE kind='module' AND crate=?")
const capLinksQ = q(arch, `
  SELECT capability_key, architecture_node, relationship, status, target_module
  FROM capability_architecture_link
  WHERE target_crate=?
  ORDER BY (status='verified') DESC, capability_key, architecture_node
`)

function renderDoc({ num, crateName, cargo, lib, files, tests, mods, capsRows, capsCount, capDomains, archNode, outEdges, inEdges, moduleN, capLinks, rationale }) {
  const docPath = `${num}-crate-${crateName}.md`
  const rustCount = files.length
  const syncCount = tests.filter((t) => !t.async).length
  const asyncCount = tests.filter((t) => t.async).length
  const status = rustCount ? 'HONEST-CODE-PRESENT' : 'NO-RUST-SOURCE'
  const money = capsCount.money > 0 ? 'money-adjacent by capability census; behavior audit only in this pass' : 'money-free by current capability census'
  const sourceTable = table(['Path', 'Bytes', 'Role'], sourceRows(crateName, files).map((r) => [r.path, String(r.size), r.role]))
  const capTable = table(
    ['Capability', 'Domain', 'Status', 'Money', 'Module', 'Acceptance test'],
    first(capsRows, 80).map((c) => [c.key, c.domain, c.status || '', c.moves_money ? '1' : '0', c.target_module || '', c.acceptance_test || ''])
  )
  const archTable = table(
    ['Capability', 'Node', 'Relation', 'Status', 'Target module'],
    first(capLinks, 80).map((l) => [l.capability_key, l.architecture_node, l.relationship, l.status || '', l.target_module || ''])
  )
  const testTable = table(
    ['Test', 'File', 'Kind'],
    first(tests, 80).map((t) => [t.symbol, t.file, t.async ? 'tokio::test' : 'test'])
  )
  const invariantRows = [
    ['Crate node exists when code exists', `docs/architecture.db:crate:${crateName}`, 'npm run arch:verify'],
    ['Generated doc matches code/db projection', docPath, 'npm run docs:crate-docs:check'],
    ['Verified capability rows must have real test evidence', 'docs/capabilities.db:impl_evidence', 'npm run caps:verify-impl'],
  ]
  // Cap at 300, not 60: verify-docs.mjs cross-checks every canonical capability against this
  // exact table (its `## N. capabilities.db row(s)` scan), and the highest real per-crate
  // canonical-capability count on disk today is chronica-osint at 265 -- a 60-row cap silently
  // dropped 136 of chronica-analytics's 196 mapped capabilities from this table (while the
  // untruncated header line above still listed all of them), making them invisible to
  // verify-docs.mjs's coverage check even though the header claimed them. Found via that real
  // check once crate docs were wired back into its scan (see docs/crates/ migration).
  const capStatusRows = capsRows.length
    ? first(capsRows, 300).map((c) => [c.key, c.status || '', c.moves_money ? '1' : '0', c.side_effect_class || '', cargo.name, c.target_module || '', c.acceptance_test || ''])
    : []
  const capStatusTable = table(['Key', 'Status', 'Money', 'Side effect', 'Target crate', 'Target module', 'Acceptance test'], capStatusRows)
  const dependencyRows = [
    ...cargo.deps.map((d) => [d, 'workspace dependency']),
    ...inEdges.map((d) => [d.replace('crate:', ''), 'reverse dependency']),
  ]
  const knownGaps = []
  if (!lib.role) knownGaps.push('No crate-level `//!` role comment was found in `src/lib.rs`.')
  if (!capsRows.length) knownGaps.push('No canonical capabilities currently target this crate in `docs/capabilities.db`.')
  if (capsRows.some((c) => c.status !== 'verified')) knownGaps.push('Some mapped capabilities remain unverified because no accepted implementation evidence row exists yet.')
  if (!tests.length) knownGaps.push('No Rust `#[test]` or `#[tokio::test]` functions were detected under this crate.')
  const gapList = knownGaps.length ? knownGaps.map((g, i) => `${i + 1}. ${g}`).join('\n') : '1. None proven by this generated pass; the code/db/doc projection is internally consistent.'

  return `# ${num} - crate ${code(crateName)}

> GENERATED 24-section crate atom doc. Do not hand-edit outside the rationale block; run \`npm run docs:crate-docs\` to refresh.

**Atom:** ${crateName}
**Status:** ${status}
**Owner crate / module:** ${code(`crates/${crateName}`)}
**capabilities.db key(s):** ${capsRows.length ? capsRows.map((c) => code(c.key)).join(', ') : '_none_'}; **architecture.db node(s):** ${code(`crate:${crateName}`)}

## 1. Identity & one-line truth

${crateName} is ${lib.role || 'a Chronica Rust crate without a crate-level role comment'}.

## 2. Status & evidence

| Evidence | Value |
|---|---:|
| Rust source files | ${rustCount} |
| Top-level modules | ${mods.length} |
| Synchronous tests | ${syncCount} |
| Async tests | ${asyncCount} |
| Canonical capabilities targeting crate | ${capsCount.n} |
| Verified capabilities targeting crate | ${capsCount.verified} |
| Architecture module nodes | ${moduleN} |

Primary verification commands: \`npm run docs:crate-docs:check\`, \`npm run arch:verify\`, and \`npm run caps:verify-impl\`.

## 3. Intent (business reason)

Generated tracking rows do not infer business intent. The crate-specific intent must come from the preserved rationale, source-level role comments, and tests. If the preserved rationale is absent or generic, that is a documentation gap rather than an implementation claim.

## 4. Doctrine & invariant linkage

- Rust ownership: crate lives under \`crates/chronica-*\`.
- Tracking ownership: capability/architecture DBs are build-time/audit-time surfaces only.
- Money classification from the current capability census: ${money}.
- Runtime authority must be proven in product code and tests, not by DB rows.

## 5. Runtime state machine claim

No runtime state machine is generated here. The generator refuses to draw a generic lifecycle because it would not describe ${crateName}. If this crate has a real state machine, document it in the preserved rationale with code symbols and proving tests; otherwise this section is intentionally a non-claim.

## 6. Inputs / preconditions

- \`crates/${crateName}/Cargo.toml\` must exist.
- For the generator only, \`docs/capabilities.db\` must expose either \`canonical_capability\` or the current \`source_capability\` + \`canonical_status_override\` side-table schema.
- For the generator only, \`docs/architecture.db\` must expose \`architecture_node\` and \`capability_architecture_link\`.
- The crate doc generator must run from the repository root.
- Chronica product runtime must not read these tracking DBs.

## 7. Outputs / postconditions

- \`docs/crates/${docPath}\` is a deterministic projection of the code and DB surfaces.
- \`docs/_machine/reconcile-reports/${crateName}.json\` exists for machine reconciliation.
- Existing rationale prose between sentinels is preserved.

## 8. Invariants enforced

${table(['Invariant', 'Enforced by', 'Verifier'], invariantRows)}

## 9. Authority & scope

This crate is owned by the Chronica monolith. Status changes to capabilities must be made through committed reconcile reports or guarded capability tooling, not by manually claiming completion in prose.

## 10. Money / side-effect classification

| Field | Value |
|---|---|
| Money classification | ${esc(money)} |
| Money capability rows | ${capsCount.money} |
| Required behavior for money-adjacent rows | Audit and document only unless a dedicated money-spine task authorizes behavior changes. |

## 11. Runtime data flow claim

No runtime data-flow diagram is generated here. DB-to-doc projection flow is tooling behavior, not a crate behavior, so it is deliberately omitted from the crate report. Real data flow must be documented in the preserved rationale and backed by source/test references.

## 12. Runtime happy-path claim

No runtime happy-path sequence is generated here. A sequence diagram is meaningful only when it names real actors, functions, persistence boundaries, and tests for ${crateName}. Generic docs/caps/arch sync is not reported as crate behavior.

## 13. Runtime failure modes & fail-safe behavior claim

No runtime failure-mode list is generated here. Missing crate source, missing tracking rows, or missing implementation evidence are audit findings, not product failure modes. Real fail-safe behavior must be grounded in code paths and proving tests; money-adjacent behavior must never be changed from docs or tracking rows.

## 14. Source of truth (files + symbols)

${sourceTable}

## 15. Proving tests

${testTable}

## 16. capabilities.db row(s)

${capStatusTable}

## 17. architecture.db node(s) / links

| Field | Value |
|---|---|
| Crate node | ${archNode ? code(`crate:${crateName}`) : '_missing_'} |
| Depends on | ${csvCode(outEdges.map((e) => e.replace('crate:', '')))} |
| Depended on by | ${csvCode(inEdges.map((e) => e.replace('crate:', '')))} |
| Module nodes | ${moduleN} |

${archTable}

## 18. Dependencies

${table(['Crate', 'Relation'], dependencyRows)}

## 19. Determinism / reproducibility

The generated doc is deterministic for a fixed code tree, \`docs/capabilities.db\`, and \`docs/architecture.db\`. Those DBs are build/audit inputs, not runtime inputs. Runtime determinism of crate behavior is not inferred from DB rows; it must be proven by product code paths, crate tests, and local audit of accepted implementation evidence.

## 20. Redaction / secret-safety

This generated doc includes paths, module names, capability keys, and test symbols only. It does not read or print secrets, environment variables, database rows beyond tracking metadata, or donor source contents.

## 21. Audit / evidence trail

- Architecture tracking evidence: \`docs/architecture.db\` node and link rows.
- Capability tracking evidence: \`docs/capabilities.db\` status and \`impl_evidence\` rows.
- Reconcile report: \`docs/_machine/reconcile-reports/${crateName}.json\`.

## 22. Known gaps / deferred

${gapList}

## 23. Anti-claims (what it does NOT do)

- This doc does not claim unverified capabilities are implemented.
- This doc does not prove runtime behavior from DB rows; behavior must be proven by product code paths and tests.
- This doc does not authorize money-spine or money-adjacent behavior changes.
- This doc and the tracking DBs do not provide product runtime configuration, authorization, routing, tenancy, or execution behavior.

## 24. Change-control / governance-readiness

Changes to this crate's capability status require a real code/test change plus a committed reconcile report or guarded capability command. Changes to money behavior require separate money-spine review and are outside this audit/doc/DB sync.

## Rationale (preserved)
<!-- rationale:start -->
${rationale || '_Design rationale not yet written. Add crate-specific intent, safety posture, and DB reconciliation notes here._'}
<!-- rationale:end -->
`
}

function defaultReport(crateName, docFile, capsRows, capLinks) {
  const capSummary = capsRows.length
    ? `${capsRows.length} canonical capabilities mapped, ${capsRows.filter((c) => c.status === 'verified').length} verified`
    : 'no canonical capabilities mapped'
  const capStatuses = capsRows.map((c) => c.status || 'unimplemented')
  const linkStatuses = capLinks.map((l) => l.status || 'unimplemented')
  const completeStatuses = new Set(['verified', 'implemented_unverified'])
  const allTrackedRowsAccounted =
    (capStatuses.length || linkStatuses.length)
    && [...capStatuses, ...linkStatuses].every((status) => completeStatuses.has(status))
  return {
    crate: crateName,
    judgment: allTrackedRowsAccounted ? 'all_agree' : 'mixed',
    doc_action: `Backfilled missing reconcile report while generating ${docFile}; ${capSummary}.`,
    tracking_changes: [],
    rationale_pointer: `docs/crates/${docFile}#rationale`,
  }
}

let written = 0
let archived = 0
let orphaned = 0
let reportsCreated = 0
const report = []

for (const crateName of crates) {
  const num = assignNumber(crateName)
  const docFile = `${num}-crate-${crateName}.md`
  const docPath = join(CRATE_DOCS, docFile)
  const cargo = readCargo(crateName)
  const lib = readLib(crateName)
  const files = rustFiles(crateName)
  const tests = testSymbols(files)
  const mods = topLevelModules(crateName)
  const capsRows = capRowsQ ? capRowsQ.all(cargo.name) : []
  const capsCount = capCountQ ? capCountQ.get(cargo.name) : { n: 0, money: 0, verified: 0 }
  const capDomains = capDomainsQ ? capDomainsQ.all(cargo.name) : []
  const archNode = archNodeQ ? archNodeQ.get(`crate:${cargo.name}`) : null
  const outEdges = outEdgesQ ? outEdgesQ.all(`crate:${cargo.name}`).map((e) => e.to_node) : []
  const inEdges = inEdgesQ ? inEdgesQ.all(`crate:${cargo.name}`).map((e) => e.from_node) : []
  const moduleN = moduleNQ ? moduleNQ.get(cargo.name).n : 0
  const capLinks = capLinksQ ? capLinksQ.all(cargo.name) : []
  const rationale = preserveRationale(docPath)
  const body = renderDoc({
    num,
    crateName,
    cargo,
    lib,
    files,
    tests,
    mods,
    capsRows,
    capsCount,
    capDomains,
    archNode,
    outEdges,
    inEdges,
    moduleN,
    capLinks,
    rationale,
  })

  if (CHECK) {
    if (!existsSync(docPath) || readFileSync(docPath, 'utf8') !== body) report.push(`DRIFT: ${docFile}`)
    const reportPath = join(REPORTS, `${crateName}.json`)
    if (!existsSync(reportPath)) report.push(`DRIFT: missing reconcile report ${slash(relative(ROOT, reportPath))}`)
    continue
  }

  const archivePath = join(ARCHIVE, docFile)
  if (existsSync(docPath) && readFileSync(docPath, 'utf8') !== body && !existsSync(archivePath)) {
    copyFileSync(docPath, archivePath)
    archived += 1
  }
  writeText(docPath, body)
  written += 1

  const reportPath = join(REPORTS, `${crateName}.json`)
  if (!existsSync(reportPath)) {
    writeText(reportPath, `${JSON.stringify(defaultReport(crateName, docFile, capsRows, capLinks), null, 2)}\n`)
    reportsCreated += 1
  }

  report.push(`${docFile} - files ${files.length} - tests ${tests.length} - caps ${capsCount.n} - verified ${capsCount.verified}`)
}

for (const d of allCrateDocs) {
  if (crates.includes(d.crate)) continue
  const docPath = join(CRATE_DOCS, d.file)
  if (CHECK) {
    report.push(`ORPHAN: ${d.file} (crate ${d.crate} gone)`)
  } else if (existsSync(docPath)) {
    copyFileSync(docPath, join(ARCHIVE, d.file))
    unlinkSync(docPath)
    orphaned += 1
    report.push(`ORPHAN archived+removed: ${d.file}`)
  }
}

caps.close()
arch.close()

console.log(report.join('\n'))
console.log(`\ngen-crate-docs: ${crates.length} crates - ${written} written - ${archived} archived - ${orphaned} orphans removed - ${reportsCreated} reports created -> ${slash(relative(ROOT, ARCHIVE))}`)
if (CHECK && report.length) {
  console.error('DRIFT DETECTED (run without --check to regenerate)')
  process.exit(1)
}
