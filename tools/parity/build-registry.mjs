#!/usr/bin/env node
/**
 * build-registry.mjs — Generate capabilities.registry.json from the spec system.
 *
 * Sources of truth (paths centralized in tools/_paths.mjs; live under docs/_machine/):
 *   - docs/_machine/implementation/14-vertical-slice-plan.md  (slices = capability groups; Verify = acceptance)
 *   - docs/.archive/**\/*.md §15 Verification Criteria      (per-spec acceptance commands)
 *   - docs/_machine/donor-absorption/absorption-map.generated.json (donor parity rows -> donorRefs)
 *   - docs/_machine/donor-inventory/donor-capability-map.generated.json (capability classes)
 *
 * Output: docs/_machine/parity/capabilities.registry.json
 *   One canonical row per capability that MUST ship. The coverage gate
 *   (check-coverage.mjs) refuses to call a slice "done" until every row in
 *   its scope is `verified`, and refuses to pass if a spec declares a
 *   capability with no registry row (doc-to-registry miss).
 *
 * The registry is REGENERATED additively: existing rows keep their `status`
 * and `testIds` (so progress is never lost); new rows are appended as `spec`.
 */
import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { DOCS, REGISTRY, DONOR_CAPABILITY_MAP, ABSORPTION_MAP, RETIREMENT_MAP, SLICE_PLAN } from '../_paths.mjs';

const ROOT = process.cwd();
const OUT = REGISTRY;

// ---- helpers ----------------------------------------------------------------
function walk(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, acc);
    else if (name.endsWith('.md')) acc.push(p);
  }
  return acc;
}
const read = (p) => readFileSync(p, 'utf8');
const rel = (p) => relative(ROOT, p).replace(/\\/g, '/');

// Money / side-effect detection from free text (conservative: err toward gating).
const MONEY_RE = /\b(money|spend|spends|payment|payout|order|orders|purchase|invoic|price|pricing|ad spend|budget|supplier order|charge|refund|payable|ledger|accounting|finance|finances|reconcil|trade|trades)\b/i;
const SIDEEFFECT_RE = /\b(side[- ]effect|approval[- ]gate|approval-gated|gated|post|posts|posting|submit|send|publish|create_order|browser_submit|write_external|mutat)\b/i;
const APPROVAL_RE = /\b(approval|approve|board|gate|requires_approval|HITL|human[- ]in[- ]the[- ]loop)\b/i;

// Extract the concrete verify command(s) from a chunk of text.
function extractVerifyCommands(text) {
  const cmds = new Set();
  // fenced code blocks
  for (const m of text.matchAll(/```[a-z]*\n([\s\S]*?)```/g)) {
    for (const line of m[1].split('\n')) {
      const t = line.trim();
      if (/^(pnpm|cargo|npx|node|docker|Get-ChildItem|Select-String)\b/.test(t)) cmds.add(t);
    }
  }
  // inline `pnpm ...` / `cargo ...`
  for (const m of text.matchAll(/`((?:pnpm|cargo|npx|node)[^`]+)`/g)) cmds.add(m[1].trim());
  return [...cmds];
}

// ---- load donor parity for donorRefs ---------------------------------------
const capMapPath = DONOR_CAPABILITY_MAP;
const capMap = existsSync(capMapPath) ? JSON.parse(read(capMapPath)) : { byCrate: {}, byCapabilityClass: {} };
const crateDonors = capMap.byCrate || {};

// ---- load the DONOR-ABSORPTION map (the super-hard rule: every Temporary donor
// is absorbed natively into the Chronica monolith). Produced by the absorption
// classification workflow → docs/donor-absorption/absorption-map.generated.json.
// Each entry: { donor, disposition, targetModule, capabilityAbsorbed,
// optionalBridge, priority, status }. status lifecycle mirrors slices:
// spec → red → green → verified. This makes "all 74 donors usable + parity"
// a first-class, gateable metric (per user directive).
const absorptionPath = ABSORPTION_MAP;
const absorptionMap = existsSync(absorptionPath) ? JSON.parse(read(absorptionPath)) : { donors: [] };
const donorNames = (absorptionMap.donors || []).map((d) => d.donor).filter(Boolean);
const donorAbsorption = (absorptionMap.donors || []).map((d) => ({
  donor: d.donor,
  kind: 'donor-absorption',
  disposition: d.disposition || 'native_absorption',
  targetModule: d.targetModule || '',
  capabilityAbsorbed: d.capabilityAbsorbed || '',
  optionalBridge: !!d.optionalBridge,
  priority: d.priority || 'P1',
  // discard_or_reference_only donors are "absorbed" by definition (nothing to port).
  status: d.status || (d.disposition === 'discard_or_reference_only' ? 'verified' : 'spec'),
  testIds: d.testIds || [],
}));

// ---- load the PACKAGE-RETIREMENT map (the strangler: every legacy TS backend
// package is DELETED as its chronica-* crate reaches route-parity, for a fully
// Rust backend). docs/architecture/package-retirement-map.generated.json. A
// package is `retired` only after its covering slices verify + route cutover +
// an integration test (the gate raises PACKAGE-NOT-RETIRED until then).
const retirementPath = RETIREMENT_MAP;
const retirementMap = existsSync(retirementPath) ? JSON.parse(read(retirementPath)) : { packages: [] };
const packageRetirement = (retirementMap.packages || []).map((p) => ({
  package: p.package,
  kind: 'package-retirement',
  replacementCrate: p.replacementCrate || '',
  coveredBySlices: p.coveredBySlices || [],
  // 'transitional' (db/shared) are not blocked by the gate until schema/UI cutover.
  transitional: p.status === 'transitional',
  status: p.status || 'active', // active -> strangling -> retired (transitional kept)
  note: p.note || '',
  // Carry the source-map evidence through the rollup so partial-strangle progress
  // (a Rust route serving SOME of a package's routes, proven by testIds) is not lost.
  ...(p.testIds ? { testIds: p.testIds } : {}),
  ...(p.rustRouteAvailable !== undefined ? { rustRouteAvailable: p.rustRouteAvailable } : {}),
}));

// ---- 1) SLICE-DERIVED CAPABILITY ROWS --------------------------------------
const slicePlanPath = SLICE_PLAN;
const slicePlan = read(slicePlanPath);
const sliceBlocks = slicePlan.split(/\n## (?=Slice \d+)/).slice(1);

function donorKey(s) {
  return String(s)
    .toLowerCase()
    .replace(/`|\*\*/g, '')
    .replace(/\([^)]*\)/g, '')
    .replace(/github\.com\//g, '')
    .replace(/https?:\/\/github\.com\//g, '')
    .replace(/[^a-z0-9]+/g, '');
}
function explicitDonorRefs(text) {
  const keys = donorNames.map((name) => ({
    name,
    key: donorKey(name),
    short: donorKey(String(name).replace(/-(main|master|dev)$/i, '')),
  }));
  const hay = donorKey(text);
  return keys
    .filter(({ key, short }) => (short && hay.includes(short)) || (key && hay.includes(key)))
    .map(({ name }) => name);
}

// All area spec files (for §15 fallback + coverage linking)
const AREA_DIRS_ALL = ['architecture','company-os','departments','runtime','internet-hand','workflows','approvals-security','artifacts','observability','osint-research','knowledge-memory','integrations','project-templates','ui'];
// Area specs are the detailed/legacy reference, preserved under docs/.archive/ (the canonical human
// docs are the numbered tree docs/000-NNN). Scan live area dirs AND the archived copies so
// spec-coverage linking still resolves after the docs were thinned to the numbered tree.
const ALL_SPECS = AREA_DIRS_ALL.flatMap((d) =>
  [join(DOCS, d), join(DOCS, '.archive', d)]
    .filter((base) => existsSync(base))
    .flatMap((base) => walk(base).map((p) => ({ area: d, path: p })))
);

const rows = [];
const seen = new Set();
function addRow(row) {
  if (seen.has(row.capabilityId)) return;
  seen.add(row.capabilityId);
  rows.push(row);
}
function slug(s) {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 60);
}

// ---- DEPENDENCY GRAPH (machine-enforced execution order) --------------------
// Phase-0 kernel skeleton is a HARD barrier every slice depends on.
const KERNEL = 'phase0.kernel-skeleton';
// Explicit per-slice dependency edges (slice numbers), derived from the slice-plan
// "Slice sequencing & parallelism" section + each slice's target crates + composition.
// A slice may not START/verify until all its deps are `verified`. Edited here on purpose
// (reviewed once) rather than fuzzily inferred — one wrong edge breaks the whole order.
const DEPS = {
  1: [], 2: [1], 3: [1, 2],            // company core: model -> dept/role -> project/template
  4: [], 5: [], 6: [],                 // approvals, artifacts, runtime: leaf foundations (kernel only)
  7: [1, 8], 8: [4],                   // model-gen draft needs company + review; review needs approvals
  // S9 strategy canvas is GREENFIELD ("informed by company-model-generator" is a doc
  // reference, not a code dependency) — it scores offers/options with chronica-company
  // economics + chronica-approvals; it does NOT consume S7's draft output. Corrected
  // 9:[4,7] → 9:[4] per the S37-dependency diagnosis (wf_cf888b12).
  9: [4],                              // strategy canvas needs the approval gate (S4) only
  10: [6],                             // internet hand needs runtime/tools
  11: [6], 12: [11],                   // osint needs runtime; evidence bundle needs investigation
  13: [6], 14: [13],                   // workflows need runtime; durable needs builder
  15: [6], 16: [6],                    // observability + knowledge need runtime
  17: [6, 4], 18: [17], 19: [17], 20: [17], 21: [17], // integration registry needs runtime+approvals; connectors need registry
  22: [6, 5], 23: [6],                 // media needs runtime+artifacts; sandbox needs runtime
  24: [3, 18, 19, 17], 25: [3, 11], 26: [3, 22], 27: [3, 21], 28: [3, 11, 5],
  29: [3, 22], 30: [3, 11, 18], 31: [3, 11], 32: [3, 9, 17], // templates need template-engine(3) + domain slices
  33: [17, 6], 34: [15], 35: [13, 16, 9, 33], 36: [6],       // new directions
  // S37 Business Replication is built on the NEW native foundation, NOT the legacy S7/S8
  // TS model-generator graph (those are LegacyReplaced, see slice plan). Its real deps:
  // OSINT(11) + RAG(38) + simulation(41) + debate(42) + learning loop(39) + M&A/PM lenses
  // (177/179) + TemplateProgram assembler(183) + Template Store instantiate(190) + the
  // strategy/template engine (3,9). (Diagnosis wf_cf888b12 + the lens-based S37 directive.)
  37: [3, 9, 11, 38, 41, 42, 39, 177, 179, 183, 190],
  // S40 self-evolving workflow authoring. Its REAL code deps are all verified: the learning
  // loop(39) feeds proposals, workflow definitions/versioning(13) gives lineage, approvals(4)
  // board-gate money/side-effect changes, simulation(41) compares old vs new, debate(42)
  // reviews. S14 (durable retry/cancel state) and S35 (the legacy TS AI-authored automation)
  // are conceptual lineage only — S40 is the NATIVE replacement for the S35 vision and never
  // touches S14's durable execution machine, so they are not hard deps. It never auto-mutates
  // an active workflow; every change is a new approved version.
  40: [39, 13, 4, 41, 42],
  // Workspace / Group hierarchy (S121–S142). S133 (Workspace identity) is the anchor;
  // it needs company core (1). The rest build on the Workspace/Group/contract tiers.
  133: [1],
  121: [133, 1], 122: [121, 1], 123: [121, 3], 124: [121, 15],
  125: [121, 4, 13], 126: [121, 5, 16], 127: [121, 4], 128: [121],
  129: [128, 15, 4], 130: [127, 4], 131: [121, 16], 132: [128, 41],
  134: [123, 13], 135: [121, 4, 15], 136: [121, 6],
  137: [37, 128, 132], 138: [38, 131], 139: [39, 131], 140: [40, 131, 134],
  141: [41, 42, 132], 142: [121, 4],
  // Unified ERP-centered model (S143–S195): allocation→governance→world-model/field
  // →ERP cores→lenses+template-store→commerce. ERP-central: chronica-erp binding (167)
  // built early; allocation (144) IS the ERP asset/cost-center layer. Self-contained
  // dependency block (deps only within S143-195) from the 7-cluster design pass.
  143: [], 144: [143], 145: [144], 146: [145], 147: [146], 148: [145], 149: [147, 148],
  150: [143], 151: [150], 152: [151], 153: [151], 154: [149, 153], 155: [154],
  156: [143], 157: [156], 158: [157, 150], 159: [158], 160: [159, 153], 161: [160, 153],
  162: [143], 163: [162], 164: [163], 165: [164, 149],
  166: [144], 167: [166, 145], 168: [167], 169: [168], 170: [169, 148], 171: [170], 172: [171],
  173: [167], 174: [173], 175: [174], 176: [175, 149, 159],
  177: [159, 160], 178: [177, 154], 179: [178], 180: [179], 181: [157, 177], 182: [178, 180, 160],
  183: [181, 182, 167], 184: [183], 185: [184], 186: [185], 187: [186, 143], 188: [187], 189: [188], 190: [189, 149],
  191: [143], 192: [191, 156], 193: [192, 167], 194: [192, 174, 175], 195: [194, 159, 160, 184],
};
// External-infrastructure prerequisites (not slices) a slice's GREEN/verify needs.
const INFRA_DEPS = {
  34: ['infra.clickhouse'],            // ClickHouse container must be running
  36: ['infra.coolify', 'infra.clickhouse', 'infra.postgres'], // deploy provisions the stack
  129: ['infra.clickhouse'],           // consolidated/group KPI rollups read OLAP
  135: ['infra.clickhouse'],           // inter-company spend attribution reads OLAP rollups
  175: ['infra.clickhouse'],           // OLAP warehouse (ClickHouse behind analytics.*)
  176: ['infra.clickhouse', 'infra.postgres'], // 4-store data flow Postgres→OLAP
  194: ['infra.clickhouse'],           // commerce funnels/cohorts over OLAP
  195: ['infra.clickhouse'],           // commerce scenario analytics over OLAP
  187: ['infra.postgres'],             // template registry 16-table Postgres schema
};
function depCapId(n) { const t = SLICE_TITLE[n]; return t ? `slice.${String(n).padStart(2,'0')}.${slug(t)}` : null; }
// Pre-scan slice titles so dep edges can resolve to capabilityIds.
const SLICE_TITLE = {};
for (const b of sliceBlocks) { const m = b.split('\n')[0].match(/Slice (\d+)\s*[—-]\s*(.+)/); if (m) SLICE_TITLE[Number(m[1])] = m[2].trim(); }

for (const block of sliceBlocks) {
  const head = block.split('\n')[0];               // "Slice 1 — CompanyTemplate + CompanyModel core"
  const sm = head.match(/Slice (\d+)\s*[—-]\s*(.+)/);
  if (!sm) continue;
  const sliceNum = Number(sm[1]);
  const sliceTitle = sm[2].trim();
  const field = (name) => {
    const re = new RegExp(`\\*\\*${name}[^*]*\\*\\*[:：]?\\s*([^\\n]+)`, 'i');
    const m = block.match(re);
    return m ? m[1].trim() : '';
  };
  const behavior = field('Behavior');
  const target = field('Target');
  const dbModels = field('DB models');
  const apiRoutes = field('API routes');
  const approval = field('Approval/Security');
  const artifacts = field('Artifacts');
  const observability = field('Observability');
  const verify = field('Verify');
  const parity = field('Parity');
  const donors = field('Donors');

  const crates = [...target.matchAll(/`(chronica-[a-z-]+)`/g)].map((m) => m[1]);
  const blob = [behavior, approval, artifacts].join(' ');
  // Money classification: an explicit `**MoneyClass:** money|none` field is AUTHORITATIVE
  // when present (lets a slice opt out of incidental keyword matches like "budget"/"order"
  // that appear in non-money prose). Absent the field, fall back to keyword-sniffing over
  // the blob (preserves behavior for slices that don't declare a class). The literal
  // `**MONEY**` token always forces money=true.
  // The MoneyClass value is the FIRST word ('money' | 'none'); a parenthetical note may
  // follow (e.g. "none (read-only ... false-positive)") — match the leading token only.
  const moneyClass = (field('MoneyClass').toLowerCase().match(/^\s*(money|none)\b/) || [, ''])[1];
  const explicitMoneyToken = /\*\*MONEY\*\*/.test(block);
  let movesMoney;
  if (moneyClass === 'money') movesMoney = true;
  else if (moneyClass === 'none') movesMoney = explicitMoneyToken; // 'none' unless the MONEY token is present
  else movesMoney = MONEY_RE.test(blob) || explicitMoneyToken;
  // An explicit `MoneyClass: none` asserts the slice's BUILT capability is read-only /
  // observational — the side-effect language in the prose (e.g. "any submit requires
  // approval") describes a gate contract for FUTURE callers, not what this slice executes.
  // So `none` suppresses the side-effect classification (no financial-control test demanded),
  // unless the explicit `**MONEY**` token is present. `money` forces side-effect=true.
  const sideEffect = moneyClass === 'none'
    ? movesMoney // (only true if the **MONEY** token is present)
    : (SIDEEFFECT_RE.test(blob) || movesMoney);
  // Verify commands: prefer the slice block; fall back to the linked spec's §15.
  let verifyCmds = extractVerifyCommands(block);
  if (!verifyCmds.length) verifyCmds = (verify.match(/`([^`]+)`/g) || []).map((s) => s.replace(/`/g, '')).filter((s) => /\b(pnpm|cargo|npx|node)\b/.test(s));
  if (!verifyCmds.length) {
    // pull from the spec file(s) this slice targets, by §15 of same-area specs
    const areaHint = crates.map((c) => c.replace('chronica-', '')).join('|');
    for (const sp of ALL_SPECS) {
      if (areaHint && new RegExp(areaHint).test(sp.path)) {
        const v = extractVerifyCommands((read(sp.path).split(/## 15\. Verification/i)[1] || '').split(/## 16\./)[0] || '');
        if (v.length) { verifyCmds = v.slice(0, 2); break; }
      }
    }
  }
  if (!verifyCmds.length) verifyCmds = ['pnpm -r typecheck']; // always have a floor command

  // Donor absorption must be explicit at the slice level. The old crate-based
  // fallback linked every donor mapped to a crate to every slice touching that
  // crate, which made unrelated workspace/governance slices appear to absorb
  // donors such as blackbox_exporter. Crate-to-donor maps remain inventory
  // metadata; slice coverage is only evidence-backed when the slice names a donor.
  const donorRefs = [...new Set(donors ? explicitDonorRefs(donors) : [])];

  addRow({
    capabilityId: `slice.${String(sliceNum).padStart(2, '0')}.${slug(sliceTitle)}`,
    kind: 'slice',
    slice: sliceNum,
    title: sliceTitle,
    owningCrates: crates,
    sourceSpec: rel(slicePlanPath) + `#slice-${sliceNum}`,
    behavior,
    dbModels: dbModels.replace(/`/g, ''),
    apiRoutes: apiRoutes.replace(/`/g, ''),
    artifacts: artifacts.replace(/`/g, ''),
    observability: observability.replace(/`/g, ''),
    parityLedger: parity.replace(/`/g, ''),
    donorRefs,
    movesMoney,
    sideEffectClass: movesMoney ? 'money' : (sideEffect ? 'write_external' : 'read_only'),
    requiresApproval: APPROVAL_RE.test(approval) || sideEffect,
    verifyCommands: verifyCmds,
    requiresFinancialControlTest: movesMoney || sideEffect,
    // A LegacyReplaced slice is a compatibility/retirement target whose capability is
    // delivered by a newer native slice — it is NOT work-to-do and does NOT block the gate
    // (the coverage gate skips it from UNVERIFIED). The replacement note is in the slice body.
    legacyReplaced: /\*\*LegacyReplaced:\*\*\s*true/i.test(block),
    // Machine-enforced ordering: every slice depends on the Phase-0 kernel barrier,
    // plus its explicit slice deps (resolved to capabilityIds) + external infra.
    dependsOn: [KERNEL, ...(DEPS[sliceNum] || []).map(depCapId).filter(Boolean)],
    infraDeps: INFRA_DEPS[sliceNum] || [],
    status: 'spec',          // spec -> red -> green -> verified
    testIds: [],
    notes: '',
  });
}

// ---- 2) SPEC-COVERAGE ROWS (every area spec must be covered) ----------------
// One coverage row per area spec file, so the gate can flag a spec that no
// slice/registry capability references (a doc-to-registry miss).
const AREA_DIRS = ['architecture','company-os','departments','runtime','internet-hand','workflows','approvals-security','artifacts','observability','osint-research','knowledge-memory','integrations','project-templates','ui'];
const specCoverage = [];
for (const d of AREA_DIRS) {
  const dir = join(DOCS, d);
  if (!existsSync(dir)) continue;
  for (const f of walk(dir)) {
    const text = read(f);
    const title = (text.match(/^#\s+(.+)/m) || [, rel(f)])[1].trim();
    const verifySection = (text.split(/## 15\. Verification/i)[1] || '').split(/## 16\./)[0] || '';
    const cmds = extractVerifyCommands(verifySection);
    const blob = text.slice(0, 4000);
    specCoverage.push({
      capabilityId: `spec.${d}.${slug(title)}`,
      kind: 'spec-coverage',
      area: d,
      title,
      sourceSpec: rel(f),
      hasVerificationSection: /## 15\. Verification/i.test(text),
      verifyCommands: cmds,
      movesMoney: MONEY_RE.test(blob),
      sideEffectClass: MONEY_RE.test(blob) ? 'money' : (SIDEEFFECT_RE.test(blob) ? 'write_external' : 'read_only'),
      requiresFinancialControlTest: MONEY_RE.test(blob) || SIDEEFFECT_RE.test(blob),
      coveredBySlices: [],   // filled below: which slice rows cite this area
      status: 'spec',
      testIds: [],
    });
  }
}
// Link spec-coverage rows to the slices that deliver them.
// Strategy: (a) explicit area->slice map for whole-area coverage, then
// (b) significant-word overlap between spec title and slice title/behavior.
const STOP = new Set(['the','and','a','an','of','for','to','in','on','core','system','model','spec','chronica','company','project','template','templates','and/or','with']);
const sig = (s) => new Set(s.toLowerCase().replace(/[^a-z0-9 ]/g, ' ').split(/\s+/).filter((w) => w.length > 3 && !STOP.has(w)));
// Area -> slice numbers that cover that whole area (every spec in it).
const AREA_TO_SLICES = {
  // Workspace/Group hierarchy slices (S121–S142) extend the area coverage so the new
  // model-correction specs link to the slices that deliver them.
  // Unified-model (S143-195) slices added to area coverage so the model-correction
  // specs link to their delivering slices. Allocation/governance/ERP-bindings/PM/
  // field-execution live in company-os; templates in project-templates; etc.
  'company-os': [1, 2, 3, 7, 8, 9, 121, 122, 123, 124, 128, 133, 134, 136, 137,
    143, 144, 145, 146, 148, 149, 150, 151, 152, 153, 154, 155, 162, 163, 164, 165, 179, 180],
  'departments': [2],
  'runtime': [6, 13, 14, 23, 136],
  'workflows': [13, 14, 125, 134, 140],
  'approvals-security': [4, 125, 127, 130, 142, 154, 155, 165, 168, 170],
  'artifacts': [5, 22, 126],
  'observability': [15, 124, 129, 135, 146, 156, 173, 174, 175, 176],
  'osint-research': [11, 12, 28, 30, 31, 163],
  'internet-hand': [10],
  'knowledge-memory': [16, 126, 131, 138, 139, 161],
  'integrations': [17, 18, 19, 20, 21, 166, 167, 168, 169, 170, 171, 172, 191, 192, 193],
  'project-templates': [24, 25, 26, 27, 28, 29, 30, 31, 32, 123, 137,
    177, 178, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190],
  'architecture': [34, 36, 147, 157, 158, 159, 160, 176, 194, 195],
  'ui': [], // UI surfaces are delivered alongside every slice's UI line; covered by design
};
const sliceRows = rows.filter((r) => r.kind === 'slice');
for (const sc of specCoverage) {
  const fromArea = AREA_TO_SLICES[sc.area] || [];
  const scWords = sig(sc.title);
  const fromTitle = sliceRows
    .filter((r) => { const rw = sig(r.title + ' ' + r.behavior); let n = 0; for (const w of scWords) if (rw.has(w)) n++; return n >= 1; })
    .map((r) => r.slice);
  sc.coveredBySlices = [...new Set([...fromArea, ...fromTitle])].sort((a, b) => a - b);
}

// ---- 2b) PREREQ NODES (kernel barrier + external infra) ---------------------
// These are status-tracked dependency targets that are NOT slices. A slice whose
// dependsOn/infraDeps include one of these cannot start/verify until it is `ready`.
const usedInfra = [...new Set(rows.flatMap((r) => r.infraDeps || []))];
const prereqs = [
  { capabilityId: KERNEL, kind: 'prereq', title: 'Phase-0 Rust kernel skeleton (chronica-core + chronica-api strangler + shared Postgres)', status: 'spec', readyWhen: 'cargo build --workspace green and one endpoint served through Rust', testIds: [] },
  ...usedInfra.map((id) => ({ capabilityId: id, kind: 'prereq', title: `External infrastructure: ${id.replace('infra.', '')}`, status: 'spec', readyWhen: `${id.replace('infra.', '')} container provisioned and reachable (via Coolify)`, testIds: [] })),
];
// prereq status uses: spec -> ready (instead of verified) for infra/kernel.

// ---- 3) MERGE WITH EXISTING (preserve status + all progress fields) --------
let prev = { rows: [], specCoverage: [], prereqs: [], donorAbsorption: [] };
if (existsSync(OUT)) {
  try { prev = JSON.parse(read(OUT)); } catch { /* ignore */ }
}
const prevById = new Map([...(prev.rows || []), ...(prev.specCoverage || []), ...(prev.prereqs || [])].map((r) => [r.capabilityId, r]));
function preserve(r) {
  const old = prevById.get(r.capabilityId);
  if (old) {
    r.status = old.status || r.status;
    r.testIds = old.testIds || r.testIds;
    r.notes = old.notes ?? r.notes;
    // CRITICAL: preserve the money-safety link so regen never drops a recorded
    // financial-control test (would silently re-open the MONEY gate).
    if (old.financialControlTestId) r.financialControlTestId = old.financialControlTestId;
  }
  return r;
}
rows.forEach(preserve);
specCoverage.forEach(preserve);
prereqs.forEach(preserve);
// Donor-absorption rows preserve their own status/testIds across regen.
const prevDonor = new Map((prev.donorAbsorption || []).map((d) => [d.donor, d]));
donorAbsorption.forEach((d) => {
  const old = prevDonor.get(d.donor);
  if (old) { d.status = old.status || d.status; d.testIds = old.testIds || d.testIds; }
});
// Link each donor to the SLICES that absorb it (slice.donorRefs include it), and
// roll its status up: a donor is absorbed when ALL its covering slices verify.
// This makes the per-slice work advance donor absorption automatically — no
// duplicate per-donor effort — while still letting `parity:set donor` override.
const STATUS_RANK = { spec: 0, red: 1, green: 2, verified: 3 };
const RANK_STATUS = ['spec', 'red', 'green', 'verified'];
const norm = (s) => String(s).replace(/-(main|master|dev|video-main|4\.x|6)$/i, '').toLowerCase();
for (const d of donorAbsorption) {
  const dn = norm(d.donor);
  const covering = rows.filter((r) => (r.donorRefs || []).some((ref) => norm(ref) === dn || norm(ref).includes(dn) || dn.includes(norm(ref))));
  d.coveredBySlices = covering.map((r) => r.slice).filter((n) => n != null);
  if (d.disposition === 'discard_or_reference_only') { d.status = 'verified'; continue; }
  if (covering.length) {
    // rollup = min status across covering slices (all must verify to absorb)
    const roll = Math.min(...covering.map((r) => STATUS_RANK[r.status] ?? 0));
    const own = STATUS_RANK[d.status] ?? 0;
    // effective status is the better of own-recorded and slice-rollup, but cannot
    // exceed the rollup (a donor isn't absorbed until its covering slices are).
    d.status = RANK_STATUS[Math.min(Math.max(own, roll), roll)] ?? d.status;
    // if no covering slice yet, keep own status (allows manual absorption tracking)
  }
}
// Package-retirement preserve + slice rollup: a backend package strangles/retires
// only as its covering slices verify. status: active (no covering slice verified)
// -> strangling (some verified) -> retired (ALL covering slices verified AND
// route cutover proven — the final flip to 'retired' is MANUAL via parity:set
// package, after the integration test + actual deletion).
const prevPkg = new Map((prev.packageRetirement || []).map((p) => [p.package, p]));
for (const p of packageRetirement) {
  const old = prevPkg.get(p.package);
  // The SOURCE map's `retired` is authoritative: it is only ever written by
  // `parity:set package <name> retired` — the MANUAL, process-gated flip that
  // happens AFTER route cutover + a DB-backed integration test proves parity +
  // the package dir is deleted. The slice-rollup below must not clobber it back
  // to 'strangling'. (The rollup auto-advances active->strangling as covering
  // slices verify, but the final ->retired is always a deliberate human flip.)
  if (p.status === 'retired') continue; // explicit retirement in the source map
  if (old && old.status === 'retired') { p.status = 'retired'; continue; } // never un-retire
  if (p.transitional) continue; // db/shared: not gated until cutover
  // An explicit source-map `strangling` carrying evidence (testIds / rustRouteAvailable)
  // records a DELIBERATE partial-strangle flip — a Rust route already serves SOME of the
  // package's routes (proven), even if the covering slice isn't `verified` yet. Honor it;
  // do not let the slice-rollup revert it to 'active'. (Still blocks PACKAGE-NOT-RETIRED.)
  if (p.status === 'strangling' && (p.testIds?.length || p.rustRouteAvailable !== undefined)) continue;
  const covering = (p.coveredBySlices || []).map((n) => rows.find((r) => r.slice === n)).filter(Boolean);
  if (covering.length) {
    const allVerified = covering.every((r) => r.status === 'verified');
    const anyProgress = covering.some((r) => (STATUS_RANK[r.status] ?? 0) >= STATUS_RANK.green);
    // Slices verified makes it ELIGIBLE for retirement (strangling-complete); the
    // actual 'retired' flip stays manual (requires route cutover + integration
    // test + deletion). So rollup tops out at 'strangling', never auto-'retired'.
    if (allVerified) p.status = old && old.status === 'retired' ? 'retired' : 'strangling';
    else if (anyProgress) p.status = 'strangling';
    else p.status = old?.status || 'active';
  }
}

// ---- 3b) DEPENDENCY GRAPH SELF-CHECK (missing refs + cycles) ----------------
const allIds = new Set([...rows.map((r) => r.capabilityId), ...prereqs.map((p) => p.capabilityId)]);
const depErrors = [];
for (const r of rows) {
  for (const d of [...(r.dependsOn || []), ...(r.infraDeps || [])]) {
    if (!allIds.has(d)) depErrors.push(`${r.capabilityId} -> unknown dependency '${d}'`);
  }
}
// cycle detection (DFS) over slice dependsOn (ignore prereqs — they are sinks)
const sliceById = new Map(rows.map((r) => [r.capabilityId, r]));
const WHITE = 0, GRAY = 1, BLACK = 2; const color = new Map();
function dfs(id, stack) {
  color.set(id, GRAY);
  const node = sliceById.get(id);
  for (const d of (node?.dependsOn || [])) {
    if (!sliceById.has(d)) continue; // prereq sink
    if (color.get(d) === GRAY) { depErrors.push(`dependency CYCLE: ${[...stack, id, d].join(' -> ')}`); continue; }
    if ((color.get(d) || WHITE) === WHITE) dfs(d, [...stack, id]);
  }
  color.set(id, BLACK);
}
for (const r of rows) if ((color.get(r.capabilityId) || WHITE) === WHITE) dfs(r.capabilityId, []);
if (depErrors.length) { console.error('DEPENDENCY GRAPH ERRORS:'); depErrors.forEach((e) => console.error('  ' + e)); }

// ---- 4) WRITE --------------------------------------------------------------
const registry = {
  generated: 'tools/parity/build-registry.mjs',
  description: 'Canonical capability registry for spec-test-driven, zero-miss parity. Coverage gate: tools/parity/check-coverage.mjs. See docs/runbooks/spec-test-driven-parity.md.',
  statusLegend: { spec: 'declared in docs, no test yet', red: 'failing test written (TDD red)', green: 'implemented, unit test passes', verified: 'verify command + (if side-effect/money) financial-control test pass; parity ledger flipped' },
  statusLegendPrereq: { spec: 'not started', ready: 'kernel built / infra provisioned and reachable' },
  counts: {
    capabilities: rows.length,
    specCoverageRows: specCoverage.length,
    prereqs: prereqs.length,
    moneyOrSideEffect: rows.filter((r) => r.requiresFinancialControlTest).length,
    donorAbsorption: donorAbsorption.length,
    packageRetirement: packageRetirement.length,
    dependencyErrors: depErrors.length,
  },
  prereqs,
  rows,
  specCoverage,
  donorAbsorption,
  packageRetirement,
};
writeFileSync(OUT, JSON.stringify(registry, null, 2) + '\n', 'utf8');
console.log(`Wrote ${rel(OUT)}`);
console.log(`  capability rows (slices): ${rows.length}`);
console.log(`  spec-coverage rows:       ${specCoverage.length}`);
console.log(`  prereq nodes:             ${prereqs.length} (${prereqs.map((p) => p.capabilityId).join(', ')})`);
console.log(`  money/side-effect rows:   ${registry.counts.moneyOrSideEffect}`);
console.log(`  donor-absorption rows:    ${donorAbsorption.length}${donorAbsorption.length ? '' : ' (no absorption-map.generated.json yet)'}`);
console.log(`  package-retirement rows:  ${packageRetirement.length} (${packageRetirement.filter((p) => p.status === 'retired').length} retired, ${packageRetirement.filter((p) => p.transitional).length} transitional)`);
console.log(`  dependency errors:        ${depErrors.length}${depErrors.length ? ' (see above)' : ' (graph OK)'}`);
