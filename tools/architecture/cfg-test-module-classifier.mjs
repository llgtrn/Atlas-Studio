// ── cfg(test) module truth classification ──────────────────────────────────────────────────────
//
// listRustFiles (architecture-lib.mjs) walks every physical .rs file under src/, but a file only
// ever declared behind `#[cfg(test)]` (a same-crate test submodule, e.g. `#[cfg(test)] mod foo;`
// or `#[cfg(test)] #[path = "foo_tests.rs"] mod foo;`) never exists in a release build — it is not
// part of the crate's PRODUCTION module graph. Reporting it as a `status: 'generated'` architecture
// node claims it ships, which is false.
//
// A file is classified test-only iff it is reachable from a real crate root only through a
// `#[cfg(test)]`-implying edge. That state propagates through external child modules — NOT merely
// through declarations carrying a direct cfg attribute. A file independently reachable from
// production (e.g. `mod real;`) and once from a `#[cfg(test)]` block elsewhere (e.g. a shared
// `#[path]` target reused for a test fixture) genuinely ships in a release build via its
// production reference, so it stays a real product node.
//
// `#[cfg(test)]`-implication is evaluated structurally, not by exact string match: `#[cfg(all(test))]`
// and `#[cfg(all(test, feature = "x"))]` both imply test-only (an `all(...)` conjunction is false
// whenever ANY conjunct is false, so ANDing anything else with `test` can never be true outside a
// test build). `#[cfg(any(test, feature = "x"))]` does NOT imply test-only (it can be true in a
// non-test build via the other disjunct) and stays classified as production — the conservative,
// non-hiding direction. `#[cfg_attr(test, ...)]` is deliberately NOT treated as `#[cfg(test)]`: it
// conditionally applies an attribute to an item that still compiles in production, it does not
// conditionally compile the item itself. `#[cfg(not(test))]` (and any other `not(...)`) is
// conservatively never treated as test-only (see `cfgImpliesTestOnly`).
//
// Scope/limitations, disclosed rather than silently assumed away: this is a lexical scanner, not a
// real Rust parser. It does not expand macros (a macro literally emitting `mod x;` text would not
// be seen), and it evaluates only `cfg`'s own boolean-combinator vocabulary (`all`/`any`/`not`) over
// a `test` atom — any other predicate name is an opaque atom that never implies test-only on its
// own.

import { existsSync, readFileSync } from 'node:fs'
import { basename, dirname, join, normalize, relative, sep } from 'node:path'

function readText(path) {
  return readFileSync(path, 'utf8')
}

// Replace every comment and string/char-literal body with the SAME NUMBER of filler characters so
// indices stay aligned with the original source (callers slice the original text at these indices
// to recover exact attribute text, e.g. a `#[path = "..."]` value) while brace/bracket/keyword
// scanning below never mistakes text inside a comment or string for real code.
function maskStringsAndComments(src) {
  const out = []
  let i = 0
  const n = src.length
  const fillTo = (end) => {
    while (i < end) {
      out.push(' ')
      i++
    }
  }
  while (i < n) {
    const c = src[i]
    const c2 = src[i + 1]
    if (c === '/' && c2 === '/') {
      let end = src.indexOf('\n', i)
      if (end === -1) end = n
      fillTo(end)
      continue
    }
    if (c === '/' && c2 === '*') {
      let depth = 1
      let j = i + 2
      while (j < n && depth > 0) {
        if (src[j] === '/' && src[j + 1] === '*') {
          depth++
          j += 2
          continue
        }
        if (src[j] === '*' && src[j + 1] === '/') {
          depth--
          j += 2
          continue
        }
        j++
      }
      fillTo(j)
      continue
    }
    // raw string: optional leading 'b', then 'r', then zero-or-more '#', then '"'
    {
      let j = i
      if (src[j] === 'b') j++
      if (src[j] === 'r') {
        let k = j + 1
        let hashes = 0
        while (src[k] === '#') {
          hashes++
          k++
        }
        if (src[k] === '"') {
          const closer = '"' + '#'.repeat(hashes)
          let end = src.indexOf(closer, k + 1)
          end = end === -1 ? n : end + closer.length
          fillTo(end)
          continue
        }
      }
    }
    if (c === '"') {
      let j = i + 1
      while (j < n && src[j] !== '"') j += src[j] === '\\' ? 2 : 1
      fillTo(Math.min(j + 1, n))
      continue
    }
    if (c === "'") {
      // char literal: 'x'  '\n'  '\\'  '\''  '\xNN'  '\u{...}' — anything else is a lifetime, leave it
      if (c2 === '\\') {
        const j = i + 2
        if (src[j] === 'u' && src[j + 1] === '{') {
          let k = src.indexOf('}', j)
          k = k === -1 ? j : k + 1
          if (src[k] === "'") {
            fillTo(k + 1)
            continue
          }
        } else {
          const k = src[j] === 'x' ? j + 3 : j + 1
          if (src[k] === "'") {
            fillTo(k + 1)
            continue
          }
        }
      } else if (c2 !== undefined && c2 !== "'" && src[i + 2] === "'") {
        fillTo(i + 3)
        continue
      }
      out.push(c)
      i++
      continue
    }
    out.push(c)
    i++
  }
  return out.join('')
}

// Split a `cfg(...)`-argument-list string on top-level commas (respecting nested parens), e.g.
// `test, feature = "x"` -> ['test', 'feature = "x"']; `any(a,b), c` -> ['any(a,b)', 'c'].
const LIMITS = Object.freeze({ maxTextLength: 4096, maxDepth: 32, maxNodes: 256 })
const TRUTH = Object.freeze({ TRUE: 'TRUE', FALSE: 'FALSE', UNKNOWN: 'UNKNOWN' })

function boundedText(text, state) {
  const trimmed = String(text).trim()
  if (!trimmed || trimmed.length > LIMITS.maxTextLength) {
    state.exceeded = true
    return null
  }
  return trimmed
}

function splitTopLevelArgs(text, state = { exceeded: false }) {
  text = boundedText(text, state)
  if (text == null) return null
  const parts = []
  let depth = 0
  let current = ''
  let quote = false
  let escaped = false
  for (const ch of text) {
    if (quote) {
      current += ch
      if (escaped) escaped = false
      else if (ch === '\\') escaped = true
      else if (ch === '"') quote = false
      continue
    }
    if (ch === '"') {
      quote = true
      current += ch
      continue
    }
    if (ch === '(') {
      depth++
      if (depth > LIMITS.maxDepth) state.exceeded = true
      current += ch
      continue
    }
    if (ch === ')') {
      depth--
      if (depth < 0) state.exceeded = true
      current += ch
      continue
    }
    if (ch === ',' && depth === 0) {
      parts.push(current.trim())
      current = ''
      continue
    }
    current += ch
  }
  if (current.trim() !== '') parts.push(current.trim())
  if (quote || depth !== 0 || parts.some((part) => !part)) state.exceeded = true
  return state.exceeded ? null : parts
}

// Parse the inside of a `cfg(...)` predicate into a tiny boolean AST over `all`/`any`/`not` and an
// opaque-atom fallback (`{ kind: 'atom', isTest }`). Not a full Rust-meta-item grammar — just enough
// structure to answer "does this predicate imply test-only" (see `cfgImpliesTestOnly`).
function parseCfgNode(text, state = { exceeded: false, nodes: 0 }, depth = 0) {
  text = boundedText(text, state)
  if (text == null || depth > LIMITS.maxDepth || ++state.nodes > LIMITS.maxNodes) {
    state.exceeded = true
    return { kind: 'unknown' }
  }
  const call = text.match(/^([A-Za-z_][A-Za-z0-9_]*)\s*\(([\s\S]*)\)$/)
  if (call && ['all', 'any', 'not'].includes(call[1])) {
    const args = splitTopLevelArgs(call[2], state)
    if (!args || (call[1] === 'not' && args.length !== 1)) return { kind: 'unknown' }
    return { kind: call[1], args: args.map((arg) => parseCfgNode(arg, state, depth + 1)) }
  }
  if (/[()]/.test(text) || /^test\s*=/.test(text)) return { kind: 'unknown' }
  return { kind: 'atom', isTest: text === 'test' }
}

// Does this predicate being satisfied REQUIRE `test` to be on — i.e. is the item it gates
// impossible to compile into a non-test (release) build? `all(...)` is a conjunction: false
// whenever any conjunct is false, so if even ONE conjunct implies test-only, the whole conjunction
// does too. `any(...)` is a disjunction: it can be satisfied by any ONE disjunct alone, so it only
// implies test-only if EVERY disjunct does. `not(...)` is conservatively never treated as
// test-only (see module doc comment) — a bare unrecognized atom is never test-only on its own.
function cfgImpliesTestOnly(node) {
  if (!node) return false
  if (node.kind === 'atom') return node.isTest === true
  if (node.kind === 'all') return node.args.some(cfgImpliesTestOnly)
  if (node.kind === 'any') return node.args.length > 0 && node.args.every(cfgImpliesTestOnly)
  return false // 'not' and anything unrecognized: conservative, non-hiding default
}

function evaluateWithoutTest(node) {
  if (!node || node.kind === 'unknown') return TRUTH.UNKNOWN
  if (node.kind === 'atom') return node.isTest ? TRUTH.FALSE : TRUTH.UNKNOWN
  const values = node.args.map(evaluateWithoutTest)
  if (node.kind === 'all') {
    if (values.includes(TRUTH.FALSE)) return TRUTH.FALSE
    return values.every((value) => value === TRUTH.TRUE) ? TRUTH.TRUE : TRUTH.UNKNOWN
  }
  if (node.kind === 'any') {
    if (values.includes(TRUTH.TRUE)) return TRUTH.TRUE
    return values.every((value) => value === TRUTH.FALSE) && values.length ? TRUTH.FALSE : TRUTH.UNKNOWN
  }
  if (node.kind === 'not') {
    return values[0] === TRUTH.TRUE ? TRUTH.FALSE : values[0] === TRUTH.FALSE ? TRUTH.TRUE : TRUTH.UNKNOWN
  }
  return TRUTH.UNKNOWN
}

function parseAttribute(attrText, state, depth = 0) {
  attrText = boundedText(attrText, state)
  if (attrText == null || depth > LIMITS.maxDepth || ++state.nodes > LIMITS.maxNodes) {
    state.exceeded = true
    return TRUTH.UNKNOWN
  }
  const outer = attrText.match(/^#\s*\[([\s\S]*)\]$/)
  if (!outer) return TRUTH.UNKNOWN
  const meta = boundedText(outer[1], state)
  if (meta == null) return TRUTH.UNKNOWN
  const cfg = meta.match(/^cfg\s*\(([\s\S]*)\)$/)
  if (cfg) return evaluateWithoutTest(parseCfgNode(cfg[1], state, depth + 1))
  const cfgAttr = meta.match(/^cfg_attr\s*\(([\s\S]*)\)$/)
  if (!cfgAttr) return TRUTH.TRUE
  const args = splitTopLevelArgs(cfgAttr[1], state)
  if (!args || args.length < 2) return TRUTH.UNKNOWN
  const condition = evaluateWithoutTest(parseCfgNode(args[0], state, depth + 1))
  let applied = TRUTH.TRUE
  for (const arg of args.slice(1)) {
    const value = parseAttribute(`#[${arg}]`, state, depth + 1)
    if (value === TRUTH.FALSE) applied = TRUTH.FALSE
    else if (value === TRUTH.UNKNOWN && applied !== TRUTH.FALSE) applied = TRUTH.UNKNOWN
  }
  if (condition === TRUTH.FALSE) return TRUTH.TRUE
  if (condition === TRUTH.TRUE) return applied
  return applied === TRUTH.TRUE ? TRUTH.TRUE : TRUTH.UNKNOWN
}

// `attrText` is one full attribute span, e.g. `'#[cfg(test)]'` or `'#[cfg(all(test, feature = "x"))]'`.
function isCfgTestPredicate(attrText) {
  const state = { exceeded: false, nodes: 0 }
  return parseAttribute(attrText, state) === TRUTH.FALSE && !state.exceeded
}

// Scan a masked/raw source pair for `mod` item declarations at every nesting depth. Returns entries
// for EXTERNALLY-declared modules only (`mod name;`), each carrying whether it is reachable solely
// through a `#[cfg(test)]`-implying path (its own attribute, or an enclosing inline `mod block { }`
// that is itself `#[cfg(test)]`-implying) and any `#[path = "..."]` override text.
function findModDeclarations(masked, raw) {
  const results = []
  const blockStack = [] // { name, openDepthBeforeIncrement, testOnly }
  let depth = 0
  let pendingAttrs = [] // raw attribute text spans, e.g. '#[cfg(test)]', '#[path = "foo.rs"]'
  let i = 0
  const n = masked.length
  const isIdentStart = (ch) => /[A-Za-z_]/.test(ch)
  const isIdentChar = (ch) => /[A-Za-z0-9_]/.test(ch)
  const isSpace = (ch) => ch === ' ' || ch === '\n' || ch === '\t' || ch === '\r'

  while (i < n) {
    const c = masked[i]
    if (isSpace(c)) {
      i++
      continue
    }
    if (c === '#' && masked[i + 1] === '[') {
      let j = i + 2
      let bdepth = 1
      while (j < n && bdepth > 0) {
        if (masked[j] === '[') bdepth++
        else if (masked[j] === ']') bdepth--
        j++
      }
      pendingAttrs.push(raw.slice(i, j))
      i = j
      continue
    }
    if (isIdentStart(c)) {
      let j = i + 1
      while (j < n && isIdentChar(masked[j])) j++
      const word = masked.slice(i, j)
      if (word === 'pub') {
        // 'pub'/'pub(crate)'/'pub(super)'/'pub(in path)' stays transparent to a following 'mod'
        let k = j
        while (k < n && isSpace(masked[k])) k++
        if (masked[k] === '(') {
          let pdepth = 1
          let m = k + 1
          while (m < n && pdepth > 0) {
            if (masked[m] === '(') pdepth++
            else if (masked[m] === ')') pdepth--
            m++
          }
          i = m
        } else {
          i = j
        }
        continue
      }
      if (word === 'mod') {
        let k = j
        while (k < n && isSpace(masked[k])) k++
        let m = k
        while (m < n && isIdentChar(masked[m])) m++
        const name = masked.slice(k, m)
        let p = m
        while (p < n && isSpace(masked[p])) p++
        const attrsForThis = pendingAttrs
        pendingAttrs = []
        if (!name) {
          i = p
          continue
        }
        const cfgTestSelf = attrsForThis.some(isCfgTestPredicate)
        const pathAttr = attrsForThis.find((a) => /^#\s*\[\s*path\s*=/.test(a))
        const pathMatch = pathAttr ? pathAttr.match(/path\s*=\s*"([^"]*)"/) : null
        const pathOverride = pathMatch ? pathMatch[1] : null
        const testOnly = cfgTestSelf || blockStack.some((b) => b.testOnly)
        if (masked[p] === ';') {
          results.push({
            name,
            testOnly,
            pathOverride,
            enclosingBlocks: blockStack.map((b) => b.name),
          })
          i = p + 1
          continue
        }
        if (masked[p] === '{') {
          blockStack.push({ name, openDepthBeforeIncrement: depth, testOnly })
          depth++
          i = p + 1
          continue
        }
        i = p
        continue
      }
      // any other keyword/identifier terminates attribute association with a later 'mod'
      pendingAttrs = []
      i = j
      continue
    }
    if (c === '{') {
      depth++
      i++
      continue
    }
    if (c === '}') {
      depth--
      while (blockStack.length && blockStack[blockStack.length - 1].openDepthBeforeIncrement === depth) {
        blockStack.pop()
      }
      i++
      continue
    }
    if (pendingAttrs.length && c !== '#') pendingAttrs = []
    i++
  }
  return results
}

// Resolve a `mod` declaration (from `findModDeclarations`) to the absolute file path Rust itself
// would compile it from, mirroring rustc's own directory-ownership rules: `mod.rs`/`lib.rs`/
// `main.rs` own their containing directory directly; any other file owns a same-named sibling
// directory. A `#[path]` override is relative to the module directory at the declaration site;
// inline modules therefore contribute their directory components too.
function resolveModTargetPath(declaringFile, entry) {
  const dir = dirname(declaringFile)
  const fileName = basename(declaringFile)
  const isModuleRoot = fileName === 'mod.rs' || fileName === 'lib.rs' || fileName === 'main.rs'
  const stem = fileName.replace(/\.rs$/, '')
  let moduleDir = isModuleRoot ? dir : join(dir, stem)
  for (const blockName of entry.enclosingBlocks) moduleDir = join(moduleDir, blockName)

  if (entry.pathOverride) {
    const target = join(entry.enclosingBlocks.length ? moduleDir : dir, entry.pathOverride)
    return existsSync(target) ? target : null
  }
  const asFile = join(moduleDir, `${entry.name}.rs`)
  const asDir = join(moduleDir, entry.name, 'mod.rs')
  if (existsSync(asFile)) return asFile
  if (existsSync(asDir)) return asDir
  return null
}

// Every physical .rs file that is reachable from crate roots ONLY through `#[cfg(test)]`-implying
// module paths — i.e. it has test reachability and no independent production reachability —
// never compiled into a production build, so never a product architecture node/edge. A file with
// BOTH a production reference and a test-cfg reference (e.g. a shared `#[path]` target declared
// once plainly and once behind `#[cfg(test)]` elsewhere) genuinely ships via its production
// reference and is NOT included here. Deterministic: pure function of file contents on disk.
export function discoverReachableModules(files) {
  const normalizedFiles = new Set(files.map(normalize))
  const roots = files.filter((file) => {
    const path = normalize(file).split(sep).join('/')
    return /\/src\/(?:lib|main)\.rs$/.test(path) || /\/src\/bin\/[^/]+\.rs$/.test(path)
  })
  const declarations = new Map()
  const getDeclarations = (file) => {
    if (!declarations.has(file)) {
      const raw = readText(file)
      declarations.set(file, findModDeclarations(maskStringsAndComments(raw), raw))
    }
    return declarations.get(file)
  }
  const production = []
  const tests = []
  const queue = roots.map((file) => {
    const name = basename(file, '.rs')
    const modulePath = dirname(file).endsWith(`${sep}bin`) ? `bin::${name}` : name
    return { file: normalize(file), modulePath, testOnly: false }
  })
  const visited = new Set()
  while (queue.length) {
    const current = queue.shift()
    const visitKey = `${current.testOnly ? 'test' : 'prod'}\0${current.file}\0${current.modulePath}`
    if (visited.has(visitKey)) continue
    visited.add(visitKey)
    ;(current.testOnly ? tests : production).push([current.modulePath, current.file])
    for (const entry of getDeclarations(current.file)) {
      const target = resolveModTargetPath(current.file, entry)
      if (!target || !normalizedFiles.has(normalize(target))) continue
      const parent = current.modulePath === 'lib' || current.modulePath === 'main' ? '' : current.modulePath
      const modulePath = [...(parent ? [parent] : []), ...entry.enclosingBlocks, entry.name].join('::')
      queue.push({ file: normalize(target), modulePath, testOnly: current.testOnly || entry.testOnly })
    }
  }
  const unique = (entries) => [...new Map(entries.map(([modulePath, file]) => [`${modulePath}\0${file}`, [modulePath, file]])).values()]
    .sort(([aPath, aFile], [bPath, bFile]) => aPath.localeCompare(bPath) || aFile.localeCompare(bFile))
  return { production: unique(production), tests: unique(tests) }
}

export function findTestOnlyModuleFiles(files) {
  const { production, tests } = discoverReachableModules(files)
  const productionFiles = new Set(production.map(([, file]) => file))
  return new Set(tests.map(([, file]) => file).filter((file) => !productionFiles.has(file)))
}

// Exported for focused unit testing of the cfg-predicate boolean logic in isolation, independent of
// file I/O.
export const _internal = { LIMITS, TRUTH, parseCfgNode, cfgImpliesTestOnly, isCfgTestPredicate, splitTopLevelArgs, parseAttribute }
