#!/usr/bin/env node
// backend-truth-audit.mjs — Backend Truth Reconstruction Program, deterministic metrics engine.
//
// Phase 0 (scaffold) + Phase 1 (quantitative truth) + Phase 6 (architecture-debt inputs).
// "Planning Brain leads, grep follows": this script is the GREP half — cheap, deterministic,
// build-free metrics (line/file/god-file/test/TODO/unwrap counts, dependency graph, circular
// deps, hidden deps, debt score). The PLANNING-BRAIN half (purpose, justification, donor
// challenge, verdict, business value) is produced by reasoning agents and merged in separately;
// this engine never invents a verdict it cannot measure.
//
// No `cargo build` — reads Cargo.toml + src/**/*.rs only. Safe on a full disk.
//
// Outputs:
//   docs/backend-truth-audit/02-aggregate-scores.json   (array, sorted by debt_score desc)
//   docs/backend-truth-audit/03-coupling-graph.json      (depends_on / depended_on_by / cycles)
//   docs/backend-truth-audit/01-per-crate/<crate>.json   (skeleton truth record per crate)
//   backend-crates.json                                  (machine-readable full array, repo root)
//
// Usage: node tools/capabilities/backend-truth-audit.mjs

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = path.join(ROOT, 'docs', 'backend-truth-audit');
const PER_CRATE = path.join(OUT, '01-per-crate');

// Foundational crates: legitimately depended-on by many; depending on them is NOT domain leakage.
const FOUNDATIONAL = new Set([
  'chronica-core', 'chronica-policy', 'chronica-security', 'chronica-events',
  'chronica-observability', 'chronica-approvals',
]);

function read(p) { try { return fs.readFileSync(p, 'utf8'); } catch { return ''; } }
function walk(dir, acc = []) {
  let ents;
  try { ents = fs.readdirSync(dir, { withFileTypes: true }); } catch { return acc; }
  for (const e of ents) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walk(full, acc);
    else if (e.isFile() && e.name.endsWith('.rs')) acc.push(full);
  }
  return acc;
}

// Workspace members from root Cargo.toml.
function members() {
  const toml = read(path.join(ROOT, 'Cargo.toml'));
  const block = toml.slice(toml.indexOf('members'), toml.indexOf(']', toml.indexOf('members')) + 1);
  return [...block.matchAll(/"crates\/([^"]+)"/g)].map((m) => m[1]);
}

// chronica-* deps declared in a crate's Cargo.toml (any section).
function declaredDeps(crateDir) {
  const toml = read(path.join(crateDir, 'Cargo.toml'));
  const deps = new Set();
  for (const m of toml.matchAll(/^\s*(chronica-[a-z0-9-]+)\s*=/gm)) deps.add(m[1]);
  return deps;
}

// Count occurrences excluding the inline test module (heuristic: everything after the FIRST
// `#[cfg(test)]` is test code). Returns [nonTest, total].
function countSplit(src, re) {
  const cut = src.indexOf('#[cfg(test)]');
  const head = cut === -1 ? src : src.slice(0, cut);
  const all = (src.match(re) || []).length;
  const non = (head.match(re) || []).length;
  return [non, all];
}

function debtScore(q, arch) {
  let s = 0;
  s += q.god_files.length * 15;
  s += Math.min(q.todo_fixme_hack_count * 5, 30);
  s += q.language_bridge_violations.length * 10;
  s += arch.circular_dependencies.length * 10;
  s += q.dead_code_allows * 5;
  if (q.test_coverage_files === 0) s += 20;
  else if (q.total_source_files > 0 && q.test_coverage_files / q.total_source_files < 0.2) s += 10;
  s += q.unwrap_nontest * 10; // hidden-panic proxy (unwrap in non-test code)
  return s;
}

function archDebtScore(q, arch) {
  let s = 0;
  s += arch.circular_dependencies.length * 30;
  s += arch.hidden_dependencies.length * 10;
  if (arch.depends_on.length > 8) s += (arch.depends_on.length - 8) * 5;
  s += q.unwrap_nontest * 3;
  s += q.todo_fixme_hack_count * 2;
  return s;
}

function main() {
  fs.mkdirSync(PER_CRATE, { recursive: true });
  const list = members();
  const records = {};

  // Pass 1: per-crate metrics + declared deps + code references.
  for (const crate of list) {
    const crateDir = path.join(ROOT, 'crates', crate);
    const srcDir = path.join(crateDir, 'src');
    const files = walk(srcDir);
    let totalLines = 0;
    const godFiles = [];
    let testFiles = 0, deadAllows = 0, todo = 0, unwrapNon = 0, unwrapAll = 0,
        expectNon = 0, expectAll = 0, bridges = 0;
    const codeRefs = new Set();
    for (const f of files) {
      const src = read(f);
      const lines = src.split('\n').length;
      totalLines += lines;
      if (lines > 500) godFiles.push({ file: path.relative(ROOT, f).replace(/\\/g, '/'), lines });
      if (/#\[test\]|#\[cfg\(test\)\]/.test(src)) testFiles++;
      if (/allow\(dead_code\)|allow\(unused/.test(src)) deadAllows++;
      todo += (src.match(/TODO|FIXME|HACK|unimplemented!|todo!\(\)/g) || []).length;
      const [un, ua] = countSplit(src, /\.unwrap\(\)/g); unwrapNon += un; unwrapAll += ua;
      const [en, ea] = countSplit(src, /\.expect\(/g); expectNon += en; expectAll += ea;
      // Language bridge: embedded non-Rust source files would not be .rs; flag inline python!/js sentinels.
      if (/PyModule|pyo3::|napi::|wasm_bindgen/.test(src)) bridges++;
      // Cross-crate references — CODE only. Strip line comments (`//`, `//!`, `///`) and block-comment
      // continuation lines (`*`, `/*`) so a doc-comment mention is NOT mistaken for a real edge. (Verified
      // 2026-06-21: all 7 first-pass "hidden deps" were doc-comment refs; 0 real hidden edges.)
      for (const raw of src.split('\n')) {
        const t = raw.trim();
        if (t.startsWith('*') || t.startsWith('/*')) continue;
        const ci = raw.indexOf('//');
        const code = ci === -1 ? raw : raw.slice(0, ci);
        for (const m of code.matchAll(/chronica_([a-z0-9_]+)/g)) codeRefs.add('chronica-' + m[1].replace(/_/g, '-'));
      }
    }
    const declared = declaredDeps(crateDir);
    const quality = {
      total_lines: totalLines,
      god_files: godFiles,
      test_coverage_files: testFiles,
      total_source_files: files.length,
      todo_fixme_hack_count: todo,
      dead_code_allows: deadAllows,
      language_bridge_violations: bridges ? ['native-binding sentinel found'] : [],
      unwrap_nontest: unwrapNon,
      unwrap_total: unwrapAll,
      expect_nontest: expectNon,
      expect_total: expectAll,
    };
    records[crate] = { crate, quality, declared, codeRefs, files: files.length };
  }

  // Pass 2: reverse dep map, cycles, hidden deps, domain leakage.
  const dependsOn = {}, dependedBy = {};
  for (const c of list) { dependsOn[c] = []; dependedBy[c] = []; }
  for (const c of list) {
    for (const d of records[c].declared) {
      if (list.includes(d)) { dependsOn[c].push(d); dependedBy[d].push(c); }
    }
  }
  const cyclesOf = (c) => dependsOn[c].filter((d) => dependsOn[d] && dependsOn[d].includes(c));

  const aggregate = [];
  const couplingGraph = {};
  for (const c of list) {
    const r = records[c];
    // Hidden deps: chronica-* referenced in code but NOT declared in Cargo.toml (and not self).
    const hidden = [...r.codeRefs].filter(
      (ref) => ref !== c && list.includes(ref) && !r.declared.has(ref),
    );
    // Domain leakage: code references a NON-foundational sibling crate's domain.
    const leakage = [...r.codeRefs].filter(
      (ref) => ref !== c && list.includes(ref) && !FOUNDATIONAL.has(ref),
    );
    const arch = {
      depends_on: dependsOn[c].sort(),
      depended_on_by: dependedBy[c].sort(),
      circular_dependencies: cyclesOf(c).map((d) => `${c} <-> ${d}`),
      hidden_dependencies: hidden.sort(),
      domain_references_non_foundational: leakage.sort(),
      owns_domain: c.replace('chronica-', ''),
    };
    const debt = debtScore(r.quality, arch);
    const archDebt = archDebtScore(r.quality, arch);
    couplingGraph[c] = {
      depends_on: arch.depends_on,
      depended_on_by: arch.depended_on_by,
      dep_count: arch.depends_on.length,
      fan_in: arch.depended_on_by.length,
      circular: arch.circular_dependencies,
    };

    // Preserve any reasoning already merged from the Planning-Brain pass (idempotent re-runs: the
    // engine owns mechanical fields ONLY and must never clobber a verdict — RULE B re-runs in Phase 7).
    const skelFile = path.join(PER_CRATE, `${c}.json`);
    let prior = null;
    if (fs.existsSync(skelFile)) { try { prior = JSON.parse(read(skelFile)); } catch { prior = null; } }
    const reasoned = prior && prior.verdict && prior.verdict !== 'UNAUDITED';

    const skeleton = {
      crate: c,
      manifest_path: `crates/${c}/Cargo.toml`,
      // Reasoning fields (Planning-Brain-owned) — preserved across re-runs; never auto-guessed.
      purpose: reasoned ? prior.purpose : {
        business_purpose: 'PENDING_PLANNING_BRAIN',
        technical_purpose: 'PENDING_PLANNING_BRAIN',
        is_justified: 'REQUIRES_INVESTIGATION',
        justification_source: 'PENDING_PLANNING_BRAIN',
      },
      ...(reasoned && prior.donor_origins ? { donor_origins: prior.donor_origins } : {}),
      architecture: {
        ...arch,
        domain_leakage: reasoned && prior.architecture ? (prior.architecture.domain_leakage || []) : [],
      },
      quality: r.quality,
      scores: {
        debt_score: debt,
        architecture_debt_score: archDebt,
        reality_score: reasoned ? prior.scores.reality_score : 'PENDING_PLANNING_BRAIN',
        doctrine_compliance_score: reasoned ? prior.scores.doctrine_compliance_score : 'PENDING_PLANNING_BRAIN',
        business_value_score: reasoned ? prior.scores.business_value_score : 'PENDING_PLANNING_BRAIN',
        ...(reasoned && prior.scores.business_value_reason ? { business_value_reason: prior.scores.business_value_reason } : {}),
      },
      verdict: reasoned ? prior.verdict : 'UNAUDITED',
      verdict_reason: reasoned ? prior.verdict_reason : 'mechanical metrics only; reasoning pass pending',
      ...(reasoned && prior.owner_pillars ? { owner_pillars: prior.owner_pillars } : {}),
    };
    fs.writeFileSync(skelFile, JSON.stringify(skeleton, null, 2) + '\n');
    aggregate.push({
      crate: c,
      total_lines: r.quality.total_lines,
      source_files: r.quality.total_source_files,
      god_files: r.quality.god_files.length,
      test_files: r.quality.test_coverage_files,
      todo_fixme_hack: r.quality.todo_fixme_hack_count,
      unwrap_nontest: r.quality.unwrap_nontest,
      expect_nontest: r.quality.expect_nontest,
      dep_count: arch.depends_on.length,
      fan_in: arch.depended_on_by.length,
      circular: arch.circular_dependencies.length,
      hidden_deps: arch.hidden_dependencies.length,
      debt_score: debt,
      architecture_debt_score: archDebt,
    });
  }

  aggregate.sort((a, b) => b.debt_score - a.debt_score);
  fs.writeFileSync(path.join(OUT, '02-aggregate-scores.json'), JSON.stringify(aggregate, null, 2) + '\n');
  fs.writeFileSync(
    path.join(OUT, '03-coupling-graph.json'),
    JSON.stringify(
      {
        generated_by: 'backend-truth-audit.mjs',
        crate_count: list.length,
        foundational: [...FOUNDATIONAL],
        god_dependencies: list
          .filter((c) => dependedBy[c].length > list.length * 0.5)
          .map((c) => ({ crate: c, fan_in: dependedBy[c].length })),
        crates: couplingGraph,
      },
      null,
      2,
    ) + '\n',
  );
  fs.writeFileSync(path.join(ROOT, 'backend-crates.json'), JSON.stringify(aggregate, null, 2) + '\n');

  // Console summary.
  const totLines = aggregate.reduce((s, a) => s + a.total_lines, 0);
  const totGod = aggregate.reduce((s, a) => s + a.god_files, 0);
  const noTests = aggregate.filter((a) => a.test_files === 0).length;
  const cycles = aggregate.filter((a) => a.circular > 0).length;
  console.log(`crates=${list.length} total_src_lines=${totLines} god_files=${totGod} ` +
    `crates_without_tests=${noTests} crates_in_cycles=${cycles}`);
  console.log('Top 10 debt:');
  for (const a of aggregate.slice(0, 10)) {
    console.log(`  ${a.debt_score.toString().padStart(4)}  ${a.crate.padEnd(28)} ` +
      `lines=${a.total_lines} god=${a.god_files} tests=${a.test_files} ` +
      `unwrap_nt=${a.unwrap_nontest} todo=${a.todo_fixme_hack} cyc=${a.circular}`);
  }
}

main();
