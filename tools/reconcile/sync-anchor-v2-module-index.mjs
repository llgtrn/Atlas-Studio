// sync-anchor-v2-module-index.mjs — builds the set of source files actually wired into a crate's
// compiled module tree (walking `mod`/`pub mod` declarations from src/lib.rs and/or src/main.rs),
// call/test evidence for a matched module, and name-based target_module -> real-module matching.
//
// Split out of sync-anchor-v2-lib.mjs (repair round 5) purely to keep both files under this repo's
// ~500-line file-size convention -- this module answers "what real modules exist and are they
// reachable/tested", while sync-anchor-v2-lib.mjs answers "does a capability's claim agree with
// that", using this module's output as one of its inputs.
//
// Module-target matching is name-based (no `cargo` invocation, no Rust AST), so it is a
// heuristic, not a compiler: "exact" confidence normalizes and compares full module paths;
// "prefix" confidence only fires for single-segment target_module tokens of >=5 normalized
// characters and only compares the LAST path segment, to keep short generic words (cli, core,
// api, db) from matching everything. Prefix-confidence hits are still reported (this is exactly
// the mechanism that would have caught `other.repo_scripts_registry` against
// `scripts_registry.rs` by machine instead of by hand) but every finding carries its
// `match_confidence` so a human/CI reviewer can weight it.
import { existsSync } from 'node:fs'
import { basename, dirname, join, relative, sep } from 'node:path'
import { listRustFiles, SourceCache } from './sync-anchor-v2-shard.mjs'
import { maskRustNonCode } from './sync-anchor-v2-rust-lexer.mjs'
import { discoverRustRoots } from './sync-anchor-v2-roots.mjs'

const slash = (path) => path.split(sep).join('/')

// ── rust source scanning ────────────────────────────────────────────────────────────────────

const stripComments = maskRustNonCode

// Removes every `#[cfg(test)] <item> { ... }` block ENTIRELY (brace-depth matched, not a
// regex-only match, since Rust blocks nest arbitrarily) -- repair round 6/7: a reference that
// exists ONLY inside a `#[cfg(test)]`-gated item is conditionally compiled just for `cargo test`,
// never part of the shipped/production binary, so it must not count as proof of a real caller for
// isModuleReferencedElsewhere. `[^{;]*` after the attribute(s) matches ANY item signature up to
// its opening brace -- `mod tests { ... }` (round 6's original target), but also `fn foo(...) {
// ... }`, `pub fn foo(...) -> T { ... }`, `impl Foo { ... }`, etc. (round 7, item 3: a BARE
// `#[cfg(test)] fn` with no enclosing mod block was not previously stripped at all). A
// semicolon-terminated item (`#[cfg(test)] mod tests;`, an EXTERNAL file) has no inline body to
// strip here at all -- that case is handled separately, at the file-walk level, by
// parseModDeclarations excluding it from the PRODUCTION reachable set entirely (see
// buildReachableModuleFiles/buildCfgTestModuleFiles below). This is a bounded heuristic, not a
// Rust parser, but `[^{;]*` is a plain negated-class repetition (linear time, no backtracking
// blowup) so it stays safe against pathological input.
// Returns true only when a cfg predicate cannot be true with `test = false`. Unknown target and
// feature predicates remain potentially true. This deliberately distinguishes all(test, ...)
// from any(test, ...), and cfg_attr is never treated as an item-compilation gate.
function cfgRequiresTest(attrs) {
  const predicates = [...attrs.matchAll(/#\[\s*cfg\s*\(([^\]]*)\)\s*\]/g)].map((m) => m[1])
  const possibleWithoutTest = (raw) => {
    const value = raw.trim()
    if (value === 'test') return false
    const call = /^(all|any|not)\s*\((.*)\)$/.exec(value)
    if (!call) return true
    const parts = []; let start = 0; let depth = 0
    for (let i = 0; i <= call[2].length; i += 1) {
      const ch = call[2][i]
      if (ch === '(') depth += 1
      else if (ch === ')') depth -= 1
      else if ((ch === ',' || i === call[2].length) && depth === 0) {
        parts.push(call[2].slice(start, i)); start = i + 1
      }
    }
    if (call[1] === 'all') return parts.every(possibleWithoutTest)
    if (call[1] === 'any') return parts.some(possibleWithoutTest)
    // An unknown/test-dependent inner predicate may be false, so `not(...)` may be true.
    return true
  }
  return predicates.some((predicate) => !possibleWithoutTest(predicate))
}

function stripCfgTestBlocks(text) {
  const startRe = /(?:#\[(?:[^\[\]]|\[[^\]]*\])*\]\s*)+[^#\[{;]*\{/g
  let result = ''
  let lastIndex = 0
  let match
  while ((match = startRe.exec(text))) {
    const attrs = match[0].slice(0, match[0].lastIndexOf(']') + 1)
    if (!cfgRequiresTest(attrs)) continue
    result += text.slice(lastIndex, match.index)
    let depth = 1
    let i = match.index + match[0].length
    while (i < text.length && depth > 0) {
      if (text[i] === '{') depth += 1
      else if (text[i] === '}') depth -= 1
      i += 1
    }
    lastIndex = i
    startRe.lastIndex = i
  }
  result += text.slice(lastIndex)
  return result
}

function moduleDirFor(filePath) {
  const base = basename(filePath, '.rs')
  if (base === 'mod' || base === 'lib' || base === 'main') return dirname(filePath)
  return join(dirname(filePath), base)
}

// repair round 7, item 3: an EXTERNAL-file module declaration gated by `#[cfg(test)]`
// (`#[cfg(test)]\nmod tests;`, resolving to tests.rs/tests/mod.rs) was previously walked into the
// PRODUCTION reachable set exactly like an ungated `mod foo;` -- `stripCfgTestBlocks` only ever
// strips INLINE `{ ... }` bodies, and a semicolon-terminated external declaration has no inline
// body to strip. Returns `{ names, cfgTestNames }`: `names` excludes every cfg(test)-gated
// declaration (the correct input for the PRODUCTION walk); `cfgTestNames` is exactly the
// cfg(test)-gated ones (the correct input for the SEPARATE test-only walk in
// buildCfgTestModuleFiles, so a `#[cfg(test)]`-gated integration-style module is still legitimate
// test evidence, just never a production caller).
function parseModDeclarations(text) {
  const stripped = stripComments(text)
  const modRe = /\b(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;/g
  const allNames = new Set()
  let match
  while ((match = modRe.exec(stripped))) allNames.add(match[1])

  const cfgTestModRe = /(?:#\[(?:[^\[\]]|\[[^\]]*\])*\]\s*)+(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;/g
  const cfgTestNames = new Set()
  while ((match = cfgTestModRe.exec(stripped))) {
    const attrs = match[0].slice(0, match[0].lastIndexOf(']') + 1)
    if (cfgRequiresTest(attrs)) cfgTestNames.add(match[1])
  }

  const names = new Set([...allNames].filter((n) => !cfgTestNames.has(n)))
  return { names, cfgTestNames }
}

function resolveChildFile(dir, name) {
  const asFile = join(dir, `${name}.rs`)
  if (existsSync(asFile)) return asFile
  const asDir = join(dir, name, 'mod.rs')
  if (existsSync(asDir)) return asDir
  return null
}

/**
 * The set of source files actually wired into the crate's compiled module tree, computed by
 * walking `mod`/`pub mod` declarations from src/lib.rs and/or src/main.rs using Rust's standard
 * file-resolution rule (`foo.rs` or `foo/mod.rs` owns children under `foo/`). A file that exists
 * on disk but is never reached by this walk is not part of the shipped crate at all — the
 * strongest form of "no caller" (doc 180: "code without a shipped caller is ISLAND").
 */
export function buildReachableModuleFiles(srcDir, cache = new SourceCache({ root: srcDir }), suppliedRoots = null) {
  const roots = suppliedRoots ?? ['lib.rs', 'main.rs'].map((f) => join(srcDir, f)).filter(existsSync)
  const reachable = new Set(roots)
  const queue = [...roots]
  while (queue.length) {
    const file = queue.shift()
    const { names } = parseModDeclarations(cache.read(file))
    const dir = moduleDirFor(file)
    for (const name of names) {
      const child = resolveChildFile(dir, name)
      if (child && !reachable.has(child)) {
        reachable.add(child)
        queue.push(child)
      }
    }
  }
  return reachable
}

function walkRootModules(root, cache) {
  const production = new Map([[root.path, root.kind === 'lib' ? 'lib' : root.name]])
  const testOnly = new Map()
  const queue = [{ file: root.path, modulePath: '', inheritedTest: false }]
  while (queue.length) {
    const { file, modulePath, inheritedTest } = queue.shift()
    const { names, cfgTestNames } = parseModDeclarations(cache.read(file))
    const children = inheritedTest ? new Set([...names, ...cfgTestNames]) : names
    for (const name of children) {
      const child = resolveChildFile(moduleDirFor(file), name)
      if (!child) continue
      const childPath = modulePath ? `${modulePath}::${name}` : name
      const target = inheritedTest ? testOnly : production
      if (!target.has(child)) {
        target.set(child, childPath)
        queue.push({ file: child, modulePath: childPath, inheritedTest })
      }
    }
    if (inheritedTest) continue
    for (const name of cfgTestNames) {
      const child = resolveChildFile(moduleDirFor(file), name)
      if (!child || testOnly.has(child)) continue
      const childPath = modulePath ? `${modulePath}::${name}` : name
      testOnly.set(child, childPath)
      queue.push({ file: child, modulePath: childPath, inheritedTest: true })
    }
  }
  return { production, testOnly }
}

/**
 * Files reachable ONLY through a `#[cfg(test)]`-gated external module declaration (`mod tests;`
 * gated by `#[cfg(test)]`, resolving to a real file), starting from lib.rs/main.rs, plus
 * everything transitively reachable from each such file -- once inside a `#[cfg(test)]`-gated
 * module, further NESTED `mod` declarations compile under the SAME inherited gate, so they are
 * walked normally (via `names`, not re-requiring their own `#[cfg(test)]`), not excluded a second
 * time. This is the legitimate "compiled only under `cargo test`" surface beyond the production
 * reachable set: `hasTestEvidence` scans this UNION `reachable` UNION `integrationTestFiles`, so
 * a genuinely test-only module still counts as real test evidence, but an orphan file that
 * nothing ever `mod`-declares under ANY configuration still never does (repair round 7, item 3).
 */
export function buildCfgTestModuleFiles(srcDir, cache = new SourceCache({ root: srcDir })) {
  const roots = ['lib.rs', 'main.rs'].map((f) => join(srcDir, f)).filter(existsSync)
  const testFiles = new Set()
  const queue = []
  for (const file of roots) {
    const { cfgTestNames } = parseModDeclarations(cache.read(file))
    const dir = moduleDirFor(file)
    for (const name of cfgTestNames) {
      const child = resolveChildFile(dir, name)
      if (child && !testFiles.has(child)) {
        testFiles.add(child)
        queue.push(child)
      }
    }
  }
  while (queue.length) {
    const file = queue.shift()
    const { names, cfgTestNames } = parseModDeclarations(cache.read(file))
    const dir = moduleDirFor(file)
    for (const name of new Set([...names, ...cfgTestNames])) {
      const child = resolveChildFile(dir, name)
      if (child && !testFiles.has(child)) {
        testFiles.add(child)
        queue.push(child)
      }
    }
  }
  return testFiles
}

/** `listRustFiles`'s own count/entry bound is enforced FIRST -- if the source-file listing is
 * truncated (a directory level within it was too, or -- repair round 6 -- an `opendir`/`readdir`
 * call itself failed partway through, which `listRustFiles` now also folds into `truncated`),
 * `buildReachableModuleFiles` is never called at all: the module index is already known to be
 * incomplete/discarded by the caller (verifyCrate refuses classification entirely on
 * `sourceFilesTruncated`, or on the more specific `unreadableDirs` when the cause was a real I/O
 * failure rather than the size cap), so walking `mod` declarations and reading lib.rs/main.rs
 * against a listing that cannot be trusted would only spend real reads on a result nobody uses
 * (repair round 5, item 3: "enforce traversal/entry count before reads", not after). `reachable`
 * is an empty Set in that case -- never populated from a partial/unsafe listing. */
export function buildCrateModuleIndex(crateAbsPath, cache = new SourceCache({ root: crateAbsPath }), manifestText = '') {
  const srcDir = join(crateAbsPath, 'src')
  const { files: allFiles, truncated, unsafeEntries, unreadableDirs } = listRustFiles(srcDir)
  const rootInfo = discoverRustRoots(crateAbsPath, manifestText)
  const reachableByRoot = new Map()
  const testReachableByRoot = new Map()
  const pathsByRoot = new Map()
  const rootEnumerationFailed = rootInfo.truncated || rootInfo.unsafeEntries.length || rootInfo.unreadableDirs.length || rootInfo.duplicateRootIds.length
  if (!truncated && !rootEnumerationFailed) {
    for (const root of rootInfo.roots) {
      const walked = walkRootModules(root, cache)
      reachableByRoot.set(root.id, new Set(walked.production.keys()))
      testReachableByRoot.set(root.id, new Set(walked.testOnly.keys()))
      pathsByRoot.set(root.id, walked.production)
    }
  }
  const indexedFiles = [...new Set([...allFiles, ...[...reachableByRoot.values()].flatMap((set) => [...set])])].sort()
  const modules = indexedFiles.flatMap((absPath) => {
    const rel = slash(relative(srcDir, absPath))
    const fallbackPath = rel.replace(/\.rs$/, '').replace(/\/mod$/, '').replace(/\//g, '::') || 'lib'
    const owners = rootInfo.roots.filter((root) => reachableByRoot.get(root.id)?.has(absPath))
    if (!owners.length) return [{ modulePath: fallbackPath, canonicalId: `orphan::${fallbackPath}`, rootId: null, filePath: slash(relative(crateAbsPath, absPath)), absPath }]
    return owners.map((root) => {
      const modulePath = pathsByRoot.get(root.id).get(absPath)
      return { modulePath, canonicalId: `${root.id}::${modulePath}`, rootId: root.id, filePath: slash(relative(crateAbsPath, absPath)), absPath }
    })
  }).map((module, index) => ({ ...module, moduleRef: index + 1 }))
  const reachable = truncated ? new Set() : new Set([...reachableByRoot.values()].flatMap((s) => [...s]))
  // repair round 7, item 3: the set of files reachable ONLY via a #[cfg(test)]-gated external mod
  // declaration -- genuinely compiled under `cargo test`, so legitimate test evidence, but NEVER
  // folded into `reachable` (which must stay production-only).
  const testReachable = truncated || rootEnumerationFailed ? new Set() : new Set([...testReachableByRoot.values()].flatMap((set) => [...set]))
  // repair round 6, item (1): Rust's own convention for crate-level integration tests -- a
  // `tests/` directory SIBLING to `src/`, each `.rs` file there compiled as its own separate test
  // binary -- was previously invisible to hasTestEvidence entirely (it only ever scanned
  // `allFiles`, which is `src/`-only), so a capability tested EXCLUSIVELY via an integration test
  // incorrectly reported SHARD_CLAIMS_TESTED_BUT_NO_TEST_FILE_REFERENCES_MODULE. `listRustFiles`
  // already returns `{files: [], ...}` for a nonexistent `tests/` (most crates have none), so this
  // is safe to call unconditionally; an unreadable tests/ directory only risks a MISSED (false
  // DISAGREE, safely surfaced) test-evidence match, never a false AGREE, so unlike `src/` this does
  // NOT refuse the whole crate -- it is instead folded into `operational_findings` as a lower-
  // stakes, non-blocking finding by the caller (see verifyCrate's `testDirFindings`).
  const testsDir = join(crateAbsPath, 'tests')
  const { files: integrationTestFiles, unreadableDirs: testsUnreadableDirs } = listRustFiles(testsDir)
  return {
    srcDir,
    crateAbsPath,
    allFiles,
    modules,
    reachable,
    reachableByRoot,
    testReachableByRoot,
    roots: rootInfo.roots,
    rootEnumeration: rootInfo,
    testReachable,
    sourceFilesTruncated: truncated,
    unsafeEntries: unsafeEntries ?? [],
    unreadableDirs: unreadableDirs ?? [],
    integrationTestFiles,
    integrationTestsUnreadableDirs: testsUnreadableDirs ?? [],
  }
}

// ── module reachability / call evidence ─────────────────────────────────────────────────────

/**
 * True if the module's leaf name is used as `<leaf>::` somewhere outside its own file, with the
 * bare `mod <leaf>;` / `pub mod <leaf>;` declaration line stripped first so a declaration alone
 * never counts as a caller.
 *
 * Repair round 6, item (1) correction: this previously scanned `moduleIndex.allFiles` (EVERY `.rs`
 * file found under src/, including orphan files never reached by any `mod` declaration from
 * lib.rs/main.rs) and never stripped `#[cfg(test)]` blocks -- so a module could be marked
 * "referenced elsewhere" (proof of a real, shipped caller) by a reference that only exists in dead
 * code that is not even compiled into the crate, or inside a `#[cfg(test)]`-gated test module that
 * is not part of the production binary. Neither is a real caller. This now scans only
 * `moduleIndex.reachable` (the set `buildReachableModuleFiles` proved is actually wired into
 * lib.rs/main.rs's compiled module tree) and strips `#[cfg(test)]` blocks before searching, so a
 * reference only reachable via dead code or only visible under `cargo test` no longer counts.
 */
export function isModuleReferencedElsewhere(moduleIndex, matchedModule, cache = new SourceCache({ root: moduleIndex.crateAbsPath ?? moduleIndex.srcDir })) {
  const leaf = matchedModule.modulePath.split('::').pop()
  if (!leaf || leaf === 'lib' || leaf === 'main') return true
  const declRe = new RegExp(`\\b(?:pub(?:\\([^)]*\\))?\\s+)?mod\\s+${leaf}\\s*;`, 'g')
  const reference = `${matchedModule.modulePath === 'lib' || matchedModule.modulePath === 'main' ? '' : matchedModule.modulePath}::`
  const useRe = new RegExp(`\\b${reference.replace(/::/g, '\\s*::\\s*')}`)
  const candidates = matchedModule.rootId ? moduleIndex.reachableByRoot.get(matchedModule.rootId) ?? [] : []
  for (const file of candidates) {
    if (file === matchedModule.absPath) continue
    const text = stripCfgTestBlocks(stripComments(cache.read(file))).replace(declRe, '')
    if (useRe.test(text)) return true
  }
  return false
}

/** True if some file with a `#[test]`-family attribute also CALLS the module (a `leaf::` module-
 * path reference, not just an incidental word match), or the module's own file carries its own
 * `#[test]`s (the common same-file `mod tests` pattern).
 *
 * Repair round 6, item (1): also scans `moduleIndex.integrationTestFiles` (Rust's `tests/`
 * directory, a sibling of `src/`, each file compiled as its own separately-run integration test
 * binary) -- previously invisible here entirely, so a capability tested ONLY via an integration
 * test incorrectly reported no test evidence at all.
 *
 * Repair round 7, item 3 correction: round 6 scanned `moduleIndex.allFiles` -- EVERY `.rs` file
 * under src/, including files nothing ever `mod`-declares under ANY configuration -- so a genuine
 * orphan file that happens to contain both a `#[test]` attribute (for something else entirely) AND
 * an incidental, unrelated mention of the leaf name (a comment, a string, an unrelated identifier)
 * could wrongly count as test evidence. This now scans `reachable` UNION `testReachable` (the
 * `#[cfg(test)]`-gated external-module surface) UNION `integrationTestFiles` -- every file
 * genuinely compiled under SOME configuration (production or test), never an orphan the crate
 * would not even build. It also requires a `leaf::` module-path reference (matching
 * isModuleReferencedElsewhere's own bar for "is this a real reference"), not a bare `\bleaf\b`
 * word match, EXCEPT for the module's own file (self-file `#[test]`s count regardless of naming,
 * the standard same-file `mod tests { #[test] fn ... { super::run() } }` pattern). */
export function hasTestEvidence(moduleIndex, matchedModule, cache = new SourceCache({ root: moduleIndex.crateAbsPath ?? moduleIndex.srcDir })) {
  const root = moduleIndex.roots?.find((item) => item.id === matchedModule.rootId)
  const reference = `${matchedModule.modulePath === 'lib' || matchedModule.modulePath === 'main' ? '' : matchedModule.modulePath}::`
  const useRe = new RegExp(`\\b${reference.replace(/::/g, '\\s*::\\s*')}`)
  const candidateFiles = new Set([
    ...(moduleIndex.reachableByRoot.get(matchedModule.rootId) ?? []),
    ...(moduleIndex.testReachableByRoot?.get(matchedModule.rootId) ?? []),
    ...(root?.kind === 'lib' ? moduleIndex.integrationTestFiles ?? [] : []),
  ])
  for (const file of candidateFiles) {
    const text = stripComments(cache.read(file))
    if (!/#\[\s*(?:tokio::)?(?:async_std::)?test\s*\]/.test(text)) continue
    if (file === matchedModule.absPath || useRe.test(text)) return true
  }
  return false
}

export function classifyFileBody(text) {
  const stripped = stripComments(text)
  const hasPlaceholder = /\b(?:todo|unimplemented)!\s*\(/.test(stripped)
  const codeLines = stripped
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !/^(use\s|pub\s+mod\s|mod\s|#\[|\}$|\{$)/.test(line))
  if (codeLines.length === 0) return 'empty'
  if (hasPlaceholder && codeLines.length < 20) return 'stub'
  if (codeLines.length < 6) return 'stub'
  return 'real'
}

// ── capability target_module -> real module matching ────────────────────────────────────────

function segmentsOf(pathLike) {
  return String(pathLike ?? '')
    .split(/::|\//)
    .map((s) => s.trim())
    .filter(Boolean)
}

function normSeg(s) {
  return s.toLowerCase().replace(/[^a-z0-9]/g, '')
}

/** `modules` is always iterated/returned in the deterministic (sorted-by-path) order
 * buildCrateModuleIndex produces, so a target with multiple candidate matches always sees them
 * in the same order across runs/platforms. */
export function matchCapabilityModule(targetModule, modules) {
  const targetSegs = segmentsOf(targetModule).map(normSeg)
  if (!targetSegs.length) return { matches: [], confidence: null }
  const targetJoined = targetSegs.join('')
  const targetLast = targetSegs[targetSegs.length - 1]
  const exact = []
  const prefix = []
  for (const module of modules) {
    const realSegs = segmentsOf(module.modulePath).map(normSeg)
    const realJoined = realSegs.join('')
    const realLast = realSegs[realSegs.length - 1] ?? ''
    if (targetJoined && realJoined && targetJoined === realJoined) {
      exact.push(module)
      continue
    }
    if (
      targetSegs.length === 1 &&
      targetLast.length >= 5 &&
      realLast.length >= 5 &&
      (realLast.startsWith(targetLast) || targetLast.startsWith(realLast))
    ) {
      prefix.push(module)
    }
  }
  if (exact.length) return { matches: exact, confidence: 'exact' }
  if (prefix.length) return { matches: prefix, confidence: 'prefix' }
  return { matches: [], confidence: null }
}
