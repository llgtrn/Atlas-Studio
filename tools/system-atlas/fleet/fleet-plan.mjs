#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { allocateDonors, loadFleetRegistry } from "./donor-allocator.mjs";
import { buildFleetCensus } from "./fleet-census.mjs";
import { compileSchemas, validate, formatErrors } from "../schema-validate.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..", "..");
const SCHEMA_DIR = resolve(ROOT, "tools/system-atlas/schema");

function validateInstance(instance, schemaId) {
  const result = validate(instance, schemaId, compileSchemas(SCHEMA_DIR));
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}

function canonicalHead() {
  for (const args of [["rev-parse","--verify","origin/main"],["rev-parse","HEAD"]]) {
    try {
      const value = execFileSync("git", args, { cwd: ROOT, encoding: "utf8" }).trim();
      if (/^[0-9a-f]{40}$/.test(value)) return value;
    } catch {}
  }
  return null;
}

function slug(value) {
  return String(value).replace(/[^A-Za-z0-9_.-]+/g, "-");
}

const READY_PRIORITY = new Map([
  ["CELL_RESET_BASELINE", 0],
  ["CHRONICA_VERIFY_INTEGRATE", 1],
  ["PACKAGE_VERIFY_INTEGRATE", 1],
  ["CHRONICA_CANDIDATE", 2],
  ["PACKAGE_RECONVERGE", 2],
  ["DONOR_INTAKE", 3],
  ["DONOR_CENSUS", 4],
  ["DONOR_ABSORB_SCOUT", 5],
  ["DONOR_ACCOUNT", 9],
]);

function readyPriority(task) {
  return READY_PRIORITY.get(task.kind) ?? 6;
}

export function buildFleetPlan({
  allocation = allocateDonors(),
  registry = loadFleetRegistry(),
  reports = [],
  now = new Date().toISOString(),
  chronicaSha = canonicalHead(),
} = {}) {
  const census = buildFleetCensus({ allocation, registry, reports, now });
  const reportsByRepo = new Map(
    reports
      .map((r) => [r.cell_repo ?? r.ops_repo, r])
      .filter(([repo]) => Boolean(repo)),
  );
  const cellsByRepo = new Map(registry.development_cells.map((row) => [row.repo, row]));
  const tasks = [];

  for (const cell of registry.development_cells.filter((row) => row.active !== false)) {
    const report = reportsByRepo.get(cell.repo);
    const requiredGeneration = Number(cell.refoundation_generation ?? 1);
    const observedGeneration = Number(report?.refoundation_generation ?? 0);
    const resetReady = Boolean(
      report
      && observedGeneration >= requiredGeneration
      && report.reset_mode === "CLEAN_PACKAGE_BASELINE"
      && report.pre_reset_head_sha
      && report.preservation_ref
      && report.package_contract_present
      && report.standalone_sovereignty === false
      && report.canonical_runtime_mode === "CHRONICA_REQUIRED"
    );

    if (!resetReady) {
      tasks.push({
        id: `cell-reset:${slug(cell.repo)}:g${requiredGeneration}`,
        kind: "CELL_RESET_BASELINE",
        target_repo: cell.repo,
        target_cell_id: cell.cell_id,
        target_package_id: cell.package_id,
        target_package_kind: cell.package_kind,
        refoundation_generation: requiredGeneration,
        target_ref: cell.default_ref ?? "main",
        base_sha: report?.cell_sha ?? report?.ops_sha ?? null,
        donor_id: null,
        donor_repo: null,
        donor_revision: null,
        donor_source_path: null,
        candidate_sha: null,
        state: "READY",
        depends_on: [],
        owned_scope: ["./"],
        deletion_scope: ["./"],
        chronica_reference_sha: chronicaSha,
        instructions:
          `RESET THIS DEVELOPMENT CELL TO CLEAN GENERATION ${requiredGeneration}. Preserve the exact pre-reset head under an auditable archive ref, then replace the active tracked tree with a minimal Chronica-isomorphic Package baseline using root grammar core/runtime/adapter/organism/graph/bindings/apps/deploy/tools/docs/license/temporary plus provenance donor evidence. UI root is apps/ui. chronica-package.json and chronica-mirror.json define the contract. Do not carry forward old application/backend/frontend/DB/authority/execution code. Git history/archive is the recovery mechanism. Package ${cell.package_id} (${cell.package_kind}) remains independently developable/testable but canonical_runtime=CHRONICA_REQUIRED and standalone_sovereignty=false. After reset, donor source must be reintroduced only through allocated donor intake tasks.`,
      });
    }
  }

  for (const donor of allocation.allocations) {
    const target = donor.primary_absorber;
    const isChronica = donor.primary_kind === "CHRONICA_NATIVE";
    const targetCell = isChronica ? null : cellsByRepo.get(target);
    const report = isChronica ? null : reportsByRepo.get(target);
    const baseSha = isChronica ? chronicaSha : (report?.cell_sha ?? report?.ops_sha ?? null);
    const targetRef = isChronica ? "main" : targetCell?.default_ref ?? "main";
    const targetCellId = targetCell?.cell_id ?? null;
    const targetPackageId = targetCell?.package_id ?? null;
    const targetPackageKind = targetCell?.package_kind ?? null;
    const requiredGeneration = Number(targetCell?.refoundation_generation ?? 1);
    const reportGeneration = Number(report?.refoundation_generation ?? 0);
    const resetTask = isChronica ? null : `cell-reset:${slug(target)}:g${requiredGeneration}`;
    const cellResetReady = isChronica || Boolean(
      report
      && reportGeneration >= requiredGeneration
      && report.reset_mode === "CLEAN_PACKAGE_BASELINE"
      && report.pre_reset_head_sha
      && report.preservation_ref
      && report.package_contract_present
    );
    const enrollmentTask = resetTask;
    const hasReport = cellResetReady;
    const donorSourcePath = isChronica
      ? (donor.source_path ?? null)
      : `temporary/donors/${donor.donor_id}/source`;
    const donorRevision = donor.source_revision_hint ?? null;
    const terminalAccountOnly = ["DUPLICATE","REFERENCE_ONLY","NOT_NEEDED","REDUNDANT","RETIRED"].includes(donor.status);
    const deepAdmission = !terminalAccountOnly && (
      donor.needed === "YES"
      || ["NEEDED","CENSUSED","ABSORBING","PARTIALLY_ABSORBED"].includes(donor.status)
    );

    // Allocation is closed-world; execution is sparse. UNKNOWN/NO/terminal donors remain
    // cold inventory until a concrete capability gap changes admission to needed=YES.
    if (!deepAdmission) continue;
    const mirrorDonor = report?.donor_census?.find(
      (row) => row.id === donor.donor_id && row.allocation_role === "PRIMARY_ABSORBER",
    );
    const donorIntakeReady = isChronica
      ? Boolean(donor.source_present && donorSourcePath)
      : Boolean(
          mirrorDonor
          && mirrorDonor.present
          && mirrorDonor.root === donorSourcePath
          && mirrorDonor.ops_baseline_commit
        );
    const censusComplete = isChronica
      ? Boolean(donor.census_complete)
      : Boolean(mirrorDonor?.census_complete);

    const intakeTask = !isChronica && !donorIntakeReady
      ? `donor-intake:${donor.donor_id}`
      : null;

    if (intakeTask) {
      tasks.push({
        id: intakeTask,
        kind: "DONOR_INTAKE",
        target_repo: target,
        target_cell_id: targetCellId,
        target_package_id: targetPackageId,
        target_package_kind: targetPackageKind,
        refoundation_generation: requiredGeneration,
        target_ref: targetRef,
        base_sha: baseSha,
        donor_id: donor.donor_id,
        donor_repo: donor.repo,
        donor_revision: donorRevision,
        donor_source_path: donorSourcePath,
        candidate_sha: null,
        state: cellResetReady ? "READY" : "DEFERRED",
        depends_on: !cellResetReady && resetTask ? [resetTask] : [],
        owned_scope: [
          donorSourcePath,
          `temporary/donors/${donor.donor_id}/intake.json`,
          `provenance/donors/${donor.donor_id}.json`,
          `license/donors/${donor.donor_id}/`,
          "chronica-mirror.json",
        ],
        deletion_scope: [],
        chronica_reference_sha: chronicaSha,
        instructions:
          `Deterministically intake donor ${donor.donor_id} (${donor.repo}) before any feature code. Store source only at ${donorSourcePath}. Preserve exact upstream revision, provenance and license evidence; if a corpus revision hint exists, use exactly that revision. Update chronica-mirror.json with PRIMARY_ABSORBER donor metadata and census_complete=false. Production code must not import/link from donor source.`,
      });
    }

    tasks.push({
      id: `donor-census:${donor.donor_id}`,
      kind: "DONOR_CENSUS",
      target_repo: target,
      target_cell_id: targetCellId,
      target_package_id: targetPackageId,
      target_package_kind: targetPackageKind,
      refoundation_generation: isChronica ? null : requiredGeneration,
      target_ref: targetRef,
      base_sha: baseSha,
      donor_id: donor.donor_id,
      donor_repo: donor.repo,
      donor_revision: donorRevision,
      donor_source_path: donorSourcePath,
      candidate_sha: null,
      state: censusComplete ? "DEFERRED" : donorIntakeReady ? "READY" : "DEFERRED",
      depends_on: intakeTask ? [intakeTask] : !hasReport && enrollmentTask ? [enrollmentTask] : [],
      owned_scope: isChronica
        ? ["tools/refoundation/","temporary/","license/","docs/ops_production/donor/"]
        : [donorSourcePath,"chronica-mirror.json","provenance/","docs/references/"],
      deletion_scope: [],
      chronica_reference_sha: chronicaSha,
      instructions:
        `Census donor ${donor.donor_id} only after source exists at ${donorSourcePath ?? "<chronica donor path>"}. Verify exact revision/provenance/license, identify only the concrete behavior surface needed for this package, and set this donor entry census_complete=true in chronica-mirror.json only when that evidence is real. Do not implement feature code in the census task.`,
    });

    const donorState = mirrorDonor?.state ?? null;
    const donorFinished = ["FULLY_DRAINED","REFERENCE_ONLY"].includes(donorState);

    if (censusComplete && !isChronica && cellResetReady && donorIntakeReady && !donorFinished) {
      const remainingBaselineFiles = Number(mirrorDonor?.remaining_baseline_files ?? 0);
      const progressToken = remainingBaselineFiles > 0 ? `remaining-${remainingBaselineFiles}` : "next";
      tasks.push({
        id: `donor-build-next:${donor.donor_id}:${progressToken}`,
        kind: "PACKAGE_BUILD",
        target_repo: target,
        target_cell_id: targetCellId,
        target_package_id: targetPackageId,
        target_package_kind: targetPackageKind,
        refoundation_generation: requiredGeneration,
        target_ref: targetRef,
        base_sha: baseSha,
        donor_id: donor.donor_id,
        donor_repo: donor.repo,
        donor_revision: mirrorDonor?.revision ?? donorRevision,
        donor_source_path: donorSourcePath,
        candidate_sha: null,
        state: "READY",
        depends_on: [],
        owned_scope: [
          "core/",
          "runtime/",
          "adapter/",
          "organism/",
          "graph/",
          "bindings/",
          "apps/ui/",
          "chronica-mirror.json",
          donorSourcePath,
        ],
        deletion_scope: [],
        chronica_reference_sha: chronicaSha,
        instructions:
          `FAST PATH — NO SEPARATE SCOUT ROUND. Read the minimum code necessary from ${donorSourcePath}, the current Cell/package and pinned Chronica. In THIS SAME task choose exactly one bounded executable behavior/dependency closure, implement it natively, migrate its caller, prove it, and delete the consumed donor files before commit. Backend is Rust only under core/runtime/adapter/organism; UI is TypeScript/TSX only under apps/ui; graph/bindings stay declarative. Do not scan unrelated donor areas, do not redesign the package, and do not open another research loop. COMPLETE requires real donor contraction with deleted_paths under ${donorSourcePath}. If the closure cannot be safely determined or deleted, return BLOCKED with the exact dependency closure instead of scouting broadly. Continue the same donor on the next plan until its Mirror state becomes FULLY_DRAINED; only then move on.`,
      });
    }
  }

  // Ready wave: one mutation task per repository. This is intentionally cross-repo parallelism,
  // while work within the same repository remains serialized unless a later path-level planner proves independence.
  const readyWave = [];
  const claimedRepos = new Set();
  const readyTasks = tasks
    .filter((task) => task.state === "READY")
    .sort((a, b) => readyPriority(a) - readyPriority(b) || a.id.localeCompare(b.id));
  for (const task of readyTasks) {
    if (claimedRepos.has(task.target_repo)) continue;
    claimedRepos.add(task.target_repo);
    readyWave.push(task.id);
  }

  const plan = {
    schema: "chronica.system-atlas.fleet-work-plan.v1",
    generated_at: now,
    canonical_repo: registry.canonical_repo,
    tasks_total: tasks.length,
    ready_wave_total: readyWave.length,
    deferred_total: tasks.filter((task) => task.state !== "READY").length,
    tasks,
    ready_wave: readyWave,
  };
  validateInstance(plan, "chronica.system-atlas.fleet-work-plan.v1");
  return plan;
}

function main(args = process.argv.slice(2)) {
  const outIndex = args.indexOf("--out");
  const out = outIndex >= 0 ? args[outIndex + 1] : null;
  const reportPaths = args.filter((arg, index) => arg !== "--out" && index !== outIndex + 1);
  const reports = reportPaths.map((path) => JSON.parse(readFileSync(resolve(path), "utf8")));
  const plan = buildFleetPlan({ reports });
  const text = `${JSON.stringify(plan, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { console.error(`fleet planner failed: ${error.message}`); process.exit(1); }
}
