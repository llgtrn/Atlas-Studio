#!/usr/bin/env node
/**
 * set-status.mjs — the executor's interface to record progress in the parity registry.
 * Sets the OWN status of a slice, a spec file, or a prereq node (the gate reads it).
 *
 * Usage:
 *   node tools/parity/set-status.mjs slice <N>            <spec|red|green|verified> [--test <id>] [--fin <id>] [--note "..."]
 *   node tools/parity/set-status.mjs spec  <path-substr>  <spec|red|green|verified> [--test <id>] [--note "..."]
 *   node tools/parity/set-status.mjs prereq <id-substr>   <spec|ready>
 *
 * Examples:
 *   node tools/parity/set-status.mjs prereq kernel ready
 *   node tools/parity/set-status.mjs slice 1 verified --test packages/db/.../company.test.ts::roundtrip
 *   node tools/parity/set-status.mjs spec company-os/01-company-model verified --test company-model.test.ts
 *
 * A `spec` row's effective status is still gated by its covering slices (see check-coverage.mjs):
 * marking a spec verified before its slices are verified will be flagged SPEC-NOT-MET by the gate.
 */
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { REGISTRY, ABSORPTION_MAP, RETIREMENT_MAP } from '../_paths.mjs';

const REG = REGISTRY;
if (!existsSync(REG)) { console.error('No registry. Run: node tools/parity/build-registry.mjs'); process.exit(2); }

const [kind, selector, status, ...rest] = process.argv.slice(2);
const SLICE_STATES = ['spec', 'red', 'green', 'verified'];
const PREREQ_STATES = ['spec', 'ready'];
const opt = (flag) => { const i = rest.indexOf(flag); return i >= 0 ? rest[i + 1] : undefined; };
const testId = opt('--test'); const finId = opt('--fin'); const note = opt('--note');

if (!kind || !selector || !status) {
  console.error('Usage: set-status.mjs <slice|spec|prereq> <selector> <status> [--test id] [--fin id] [--note "..."]');
  process.exit(2);
}
const reg = JSON.parse(readFileSync(REG, 'utf8'));

function applyTo(row) {
  row.status = status;
  if (testId) { row.testIds = [...new Set([...(row.testIds || []), testId])]; }
  if (finId) row.financialControlTestId = finId;
  if (note) row.notes = note;
}

let target;
if (kind === 'slice') {
  if (!SLICE_STATES.includes(status)) { console.error(`slice status must be one of ${SLICE_STATES}`); process.exit(2); }
  target = (reg.rows || []).find((r) => String(r.slice) === String(selector));
  if (!target) { console.error(`No slice ${selector}`); process.exit(1); }
} else if (kind === 'spec') {
  if (!SLICE_STATES.includes(status)) { console.error(`spec status must be one of ${SLICE_STATES}`); process.exit(2); }
  const matches = (reg.specCoverage || []).filter((s) => (s.sourceSpec || '').includes(selector) || (s.capabilityId || '').includes(selector));
  if (matches.length === 0) { console.error(`No spec matching "${selector}"`); process.exit(1); }
  if (matches.length > 1) { console.error(`Ambiguous "${selector}" → ${matches.map((m) => m.sourceSpec).join(', ')}. Be more specific.`); process.exit(1); }
  target = matches[0];
} else if (kind === 'prereq') {
  if (!PREREQ_STATES.includes(status)) { console.error(`prereq status must be one of ${PREREQ_STATES}`); process.exit(2); }
  const matches = (reg.prereqs || []).filter((p) => (p.capabilityId || '').includes(selector));
  if (matches.length !== 1) { console.error(`prereq "${selector}" matched ${matches.length}`); process.exit(1); }
  target = matches[0];
} else if (kind === 'donor') {
  // Donor absorption status override. Writes to the SOURCE map so it survives
  // registry regen (registry donor status is otherwise rolled up from slices).
  if (!SLICE_STATES.includes(status)) { console.error(`donor status must be one of ${SLICE_STATES}`); process.exit(2); }
  const apath = ABSORPTION_MAP;
  if (!existsSync(apath)) { console.error(`No absorption map at ${apath}`); process.exit(1); }
  const map = JSON.parse(readFileSync(apath, 'utf8'));
  const matches = (map.donors || []).filter((d) => (d.donor || '').includes(selector));
  if (matches.length !== 1) { console.error(`donor "${selector}" matched ${matches.length}: ${matches.map((m) => m.donor).join(', ')}`); process.exit(1); }
  matches[0].status = status;
  if (testId) matches[0].testIds = [...new Set([...(matches[0].testIds || []), testId])];
  writeFileSync(apath, JSON.stringify(map, null, 2) + '\n', 'utf8');
  console.log(`set donor ${matches[0].donor} → ${status}${testId ? ` (test: ${testId})` : ''} in absorption-map.generated.json`);
  console.log('Run `node tools/parity/build-registry.mjs` then `pnpm parity:check`.');
  process.exit(0);
} else if (kind === 'package') {
  // Package-retirement status override (strangler). Writes the SOURCE map so it
  // survives registry regen. Flip to 'retired' ONLY after route cutover + an
  // integration test proving the Rust path + actually deleting the package dir.
  const PKG_STATES = ['active', 'strangling', 'retired', 'transitional'];
  if (!PKG_STATES.includes(status)) { console.error(`package status must be one of ${PKG_STATES}`); process.exit(2); }
  const mpath = RETIREMENT_MAP;
  if (!existsSync(mpath)) { console.error(`No package-retirement map at ${mpath}`); process.exit(1); }
  const map = JSON.parse(readFileSync(mpath, 'utf8'));
  const matches = (map.packages || []).filter((p) => (p.package || '').includes(selector));
  if (matches.length !== 1) { console.error(`package "${selector}" matched ${matches.length}: ${matches.map((m) => m.package).join(', ')}`); process.exit(1); }
  matches[0].status = status;
  if (testId) matches[0].testIds = [...new Set([...(matches[0].testIds || []), testId])];
  writeFileSync(mpath, JSON.stringify(map, null, 2) + '\n', 'utf8');
  console.log(`set package ${matches[0].package} → ${status}${testId ? ` (test: ${testId})` : ''} in package-retirement-map.generated.json`);
  console.log('Run `node tools/parity/build-registry.mjs` then `pnpm parity:check`.');
  process.exit(0);
} else {
  console.error('kind must be slice | spec | prereq | donor | package'); process.exit(2);
}

applyTo(target);
writeFileSync(REG, JSON.stringify(reg, null, 2) + '\n', 'utf8');
console.log(`set ${kind} ${target.capabilityId || target.sourceSpec} → ${status}${testId ? ` (test: ${testId})` : ''}${finId ? ` (fin: ${finId})` : ''}`);
console.log('Run `pnpm parity:check` / `pnpm parity:progress` to see the effect.');
