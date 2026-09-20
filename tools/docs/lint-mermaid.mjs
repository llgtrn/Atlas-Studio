#!/usr/bin/env node
// tools/docs/lint-mermaid.mjs
// ---------------------------------------------------------------------------
// AUTHORITATIVE Mermaid linter for the docs tree.
//
// It parses every ```mermaid block in docs/**/*.md with the REAL Mermaid
// grammar (mermaid.parse), so a block that fails HERE is exactly a block that
// GitHub red-boxes. This is not a regex heuristic — it is the same parser
// GitHub renders with, run headless via a jsdom DOM shim.
//
// Zero new dependencies: mermaid@11 (declared in apps/ui/package.json) and jsdom
// (already in the pnpm store, transitive) are used straight off disk. No
// puppeteer / chromium / network — all egress-blocked here anyway.
//
// Usage:
//   node tools/docs/lint-mermaid.mjs            # lint LIVE docs (excludes docs/_archive)
//   node tools/docs/lint-mermaid.mjs --all      # also lint docs/_archive
//   node tools/docs/lint-mermaid.mjs --json     # machine-readable output
//   node tools/docs/lint-mermaid.mjs <file.md>  # lint a single file (or .mmd)
//
// Exit code: 0 = all blocks parse, 1 = at least one invalid block, 2 = setup error.
//
// The authoring RULES this enforces live in docs/doctrines/201-operating-mermaid-diagram-authoring-rules.md.
// ---------------------------------------------------------------------------

import { readFileSync, globSync } from 'node:fs';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import path from 'node:path';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '..', '..');

const rawArgs = process.argv.slice(2);
const flags = new Set(rawArgs.filter((a) => a.startsWith('--')));
const fileArgs = rawArgs.filter((a) => !a.startsWith('--'));
const INCLUDE_ARCHIVE = flags.has('--all');
const JSON_OUT = flags.has('--json');

// --- 1. DOM shim -----------------------------------------------------------
// jsdom is a transitive dep (not hoisted, not resolvable by bare name), so we
// locate it in the pnpm store by glob and import it via a file:// URL.
const jsdomHit = globSync('node_modules/.pnpm/jsdom@*/node_modules/jsdom/lib/api.js', { cwd: REPO })[0];
if (!jsdomHit) {
  console.error('lint-mermaid: jsdom not found in the pnpm store — run `pnpm install` first.');
  process.exit(2);
}
const { JSDOM } = await import(pathToFileURL(path.join(REPO, jsdomHit)).href);
const dom = new JSDOM('<!DOCTYPE html><html><body></body></html>', { pretendToBeVisual: true });
// Set the DOM globals BEFORE importing mermaid — its bundled DOMPurify binds at
// import time. (Do NOT reassign globalThis.navigator on Node 22 — read-only.)
globalThis.window = dom.window;
globalThis.document = dom.window.document;
globalThis.DOMParser = dom.window.DOMParser;
globalThis.Element = dom.window.Element;
globalThis.HTMLElement = dom.window.HTMLElement;
globalThis.Node = dom.window.Node;

// --- 2. mermaid (resolves from the apps/ui/ workspace) ---------------------
async function loadMermaid() {
  // Fast path: bare import (works when cwd is apps/ui/ or mermaid is hoisted).
  try {
    const m = await import('mermaid');
    if (m?.default) return m.default;
  } catch { /* fall through */ }
  // Robust path: resolve the package from the apps/ui/ workspace and import its ESM entry.
  const req = createRequire(path.join(REPO, 'apps', 'ui', 'package.json'));
  let pkgPath;
  try {
    pkgPath = req.resolve('mermaid/package.json');
  } catch (e) {
    const hit = globSync('node_modules/.pnpm/mermaid@*/node_modules/mermaid/package.json', { cwd: REPO })[0];
    if (!hit) throw e;
    pkgPath = path.join(REPO, hit);
  }
  const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
  const dot = pkg.exports?.['.'];
  const imp = dot?.import ?? dot;
  let entry =
    (typeof imp === 'string' ? imp : imp?.default ?? imp?.node ?? imp?.browser) ||
    pkg.module ||
    pkg.main;
  const entryAbs = path.join(path.dirname(pkgPath), entry);
  const m = await import(pathToFileURL(entryAbs).href);
  return m.default ?? m;
}

let mermaid;
try {
  mermaid = await loadMermaid();
  mermaid.initialize({ startOnLoad: false, securityLevel: 'loose' });
} catch (e) {
  console.error('lint-mermaid: could not load mermaid from apps/ui/ workspace:', String(e?.message || e));
  process.exit(2);
}

// --- 3. extract ```mermaid blocks from a markdown file ---------------------
function extractBlocks(absFile) {
  const lines = readFileSync(absFile, 'utf8').split(/\r?\n/);
  const blocks = [];
  for (let i = 0; i < lines.length; i++) {
    if (/^\s*```mermaid\s*$/.test(lines[i])) {
      const startLine = i + 1; // 1-indexed fence line
      const body = [];
      i++;
      while (i < lines.length && !/^\s*```\s*$/.test(lines[i])) {
        body.push(lines[i]);
        i++;
      }
      blocks.push({ startLine, src: body.join('\n') });
    }
  }
  return blocks;
}

// --- 4. gather target files ------------------------------------------------
let files;
if (fileArgs.length) {
  files = fileArgs.map((f) => path.relative(REPO, path.resolve(f)).split(path.sep).join('/'));
} else {
  files = globSync('docs/**/*.md', { cwd: REPO }).map((f) => f.split(path.sep).join('/'));
  if (!INCLUDE_ARCHIVE) files = files.filter((f) => !f.includes('/_archive/'));
}
files.sort();

// --- 5. parse every block --------------------------------------------------
const failures = [];
let total = 0;
for (const rel of files) {
  const abs = path.join(REPO, rel);
  for (const b of extractBlocks(abs)) {
    total++;
    try {
      await mermaid.parse(b.src);
    } catch (e) {
      failures.push({
        file: rel,
        line: b.startLine,
        error: String(e?.message || e).split('\n')[0].trim(),
      });
    }
  }
}

// --- 6. report -------------------------------------------------------------
if (JSON_OUT) {
  console.log(JSON.stringify({ total, invalid: failures.length, failures }, null, 2));
} else {
  for (const f of failures) console.error(`FAIL ${f.file}:${f.line}  ${f.error}`);
  const scope = fileArgs.length ? `${files.length} file(s)` : INCLUDE_ARCHIVE ? 'all docs (incl. _archive)' : 'live docs';
  const verdict = failures.length ? `${failures.length} INVALID` : 'all valid';
  console.error(`\nlint-mermaid: ${total} block(s) checked across ${scope} — ${verdict}.`);
}
process.exit(failures.length ? 1 : 0);
