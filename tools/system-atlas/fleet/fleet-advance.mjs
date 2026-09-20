#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compileSchemas, validate, formatErrors } from "../schema-validate.mjs";
import { validateWorkerResult } from "./fleet-dispatcher.mjs";
import { buildChronicaCandidateTask, buildReconvergePlan } from "./fleet-reconverge.mjs";
import { loadFleetRegistry } from "./donor-allocator.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(HERE, "..", "schema");

function validateInstance(instance, schemaId) {
  const result = validate(instance, schemaId, compileSchemas(SCHEMA_DIR));
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}

function slug(value) {
  return String(value).replace(/[^A-Za-z0-9_.-]+/g, "-");
}

function selectReadyWave(tasks) {
  const ready = [];
  const repos = new Set();
  for (const task of tasks) {
    if (task.state !== "READY") continue;
    if (repos.has(task.target_repo)) continue;
    repos.add(task.target_repo);
    ready.push(task.id);
  }
  return ready;
}

function buildPlan(tasks, canonicalRepo, now) {
  const readyWave = selectReadyWave(tasks);
  const plan = {
    schema: "chronica.system-atlas.fleet-work-plan.v1",
    generated_at: now,
    canonical_repo: canonicalRepo,
    tasks_total: tasks.length,
    ready_wave_total: readyWave.length,
    deferred_total: tasks.filter((task) => task.state !== "READY").length,
    tasks,
    ready_wave: readyWave,
  };
  validateInstance(plan, "chronica.system-atlas.fleet-work-plan.v1");
  return plan;
}

function buildTasksFromScout(task, result) {
  return result.discovered_slices.map((slice) => ({
    id: `donor-build:${task.donor_id}:${slug(slice.id)}`,
    kind: "PACKAGE_BUILD",
    target_repo: task.target_repo,
    target_cell_id: task.target_cell_id,
    target_package_id: task.target_package_id,
    target_package_kind: task.target_package_kind ?? null,
    refoundation_generation: task.refoundation_generation ?? null,
    target_ref: task.target_ref,
    base_sha: result.base_sha,
    donor_id: task.donor_id,
    donor_repo: task.donor_repo,
    donor_revision: task.donor_revision ?? null,
    donor_source_path: task.donor_source_path ?? null,
    candidate_sha: null,
    state: "READY",
    depends_on: [task.id],
    owned_scope: slice.owned_scope,
    deletion_scope: slice.deletion_scope,
    chronica_reference_sha: task.chronica_reference_sha,
    instructions:
      `ABSORB = REPLACE + DELETE. Implement exactly one bounded donor behavior/dependency closure inside Development Cell ${task.target_cell_id}, package ${task.target_package_id}. Behavior: ${slice.behavior}. Target responsibility: ${slice.target_responsibility}. Proof: ${slice.proof_expectation.join("; ")}. REQUIRED DONOR DELETION SCOPE: ${slice.deletion_scope.join("; ")}. In the SAME candidate commit: implement backend Rust only in core/runtime/adapter/organism as appropriate; put UI/frontend TypeScript only under apps/ui; use graph/bindings for declarations; do not create root src/backend/frontend/services/packages folders; migrate the caller, run proof, then delete donor source in deletion_scope. COMPLETE is forbidden if donor source did not shrink. If safe deletion is impossible, return BLOCKED with the exact retained dependency closure; do not start another research/scout loop. The package may be independently developed/tested but must not create standalone canonical World/Identity/Authority/Execution/Evidence/Memory ownership.`,
  }));
}

function buildPackageVerifyTask(task, result) {
  const deletionScope = task.deletion_scope?.length
    ? task.deletion_scope
    : result.extinction.deleted_paths;
  if (!deletionScope?.length) {
    throw new Error(`${task.id} cannot advance without concrete donor deletion evidence`);
  }
  return {
    id: `package-verify-integrate:${slug(task.id)}`,
    kind: "PACKAGE_VERIFY_INTEGRATE",
    target_repo: task.target_repo,
    target_cell_id: task.target_cell_id,
    target_package_id: task.target_package_id,
    target_package_kind: task.target_package_kind ?? null,
    refoundation_generation: task.refoundation_generation ?? null,
    target_ref: task.target_ref,
    // Integrator resolves the then-current target head. The candidate commit remains pinned separately.
    base_sha: null,
    donor_id: task.donor_id,
    donor_repo: task.donor_repo,
    donor_revision: task.donor_revision ?? null,
    donor_source_path: task.donor_source_path ?? null,
    candidate_sha: result.candidate_sha,
    state: "READY",
    depends_on: [task.id],
    owned_scope: task.owned_scope,
    deletion_scope: deletionScope,
    chronica_reference_sha: task.chronica_reference_sha,
    instructions:
      `Resolve the exact current Development Cell repository head before integration. Independently verify package candidate ${result.candidate_sha} against that head. Check scope, Rust/TypeScript law, package contract, standalone-sovereignty signals, tests and Mirror Atlas. EXTINCTION IS A COMMIT INVARIANT: verify the candidate actually deletes the concrete donor paths recorded in deletion_scope and does not leave dangling retained-donor dependencies. A candidate that adds native code but leaves the donor slice unchanged is invalid. Integrate serially only if replace+delete proof passes. Return the resolved pre-integration head as base_sha and the resulting integrated cell/package SHA as candidate_sha. Emit chronica_candidates only for reusable universal semantics proven by the integrated package result; package-local adapters/projections/provider mechanics stay local.`,
  };
}

function buildChronicaVerifyTask(task, result, currentChronicaSha) {
  if (!task.chronica_candidate) throw new Error(`${task.id} is missing chronica_candidate context`);
  return {
    id: `chronica-verify-integrate:${slug(task.chronica_candidate.candidate_id)}`,
    kind: "CHRONICA_VERIFY_INTEGRATE",
    target_repo: "llgtrn/Chronica",
    target_cell_id: null,
    target_package_id: null,
    target_package_kind: null,
    refoundation_generation: null,
    target_ref: "main",
    base_sha: currentChronicaSha,
    donor_id: task.donor_id,
    donor_repo: task.donor_repo,
    donor_revision: task.donor_revision ?? null,
    donor_source_path: null,
    candidate_sha: result.candidate_sha,
    chronica_candidate: task.chronica_candidate,
    state: "READY",
    depends_on: [task.id],
    owned_scope: task.owned_scope,
    deletion_scope: [],
    chronica_reference_sha: currentChronicaSha,
    instructions:
      `Independently verify Chronica candidate commit ${result.candidate_sha} against CURRENT canonical main ${currentChronicaSha}. Use Chronica architecture/test/authority/evidence/recovery gates and serial integration. Return the resulting canonical main SHA as candidate_sha only after successful integration.`,
  };
}

export function advanceFleet({
  plan,
  results,
  reports = [],
  currentChronicaSha,
  registry = loadFleetRegistry(),
  now = new Date().toISOString(),
} = {}) {
  validateInstance(plan, "chronica.system-atlas.fleet-work-plan.v1");
  if (!Array.isArray(results)) throw new Error("results must be an array");
  const taskById = new Map(plan.tasks.map((task) => [task.id, task]));
  const nextTasks = [];
  const refreshRequired = new Set();
  const blockers = [];

  for (const rawResult of results) {
    const task = taskById.get(rawResult.task_id);
    if (!task) throw new Error(`worker result references unknown task ${rawResult.task_id}`);
    const result = validateWorkerResult(task, rawResult);

    if (result.status !== "COMPLETE") {
      blockers.push({ task_id: task.id, target_repo: task.target_repo, status: result.status, evidence: result.evidence });
      continue;
    }

    switch (task.kind) {
      case "DONOR_ABSORB_SCOUT":
        nextTasks.push(...buildTasksFromScout(task, result));
        break;

      case "PACKAGE_BUILD":
      case "DONOR_ABSORB_BUILD":
        nextTasks.push(buildPackageVerifyTask(task, result));
        break;

      case "PACKAGE_VERIFY_INTEGRATE":
      case "OPS_VERIFY_INTEGRATE": {
        for (const candidate of result.chronica_candidates) {
          if (candidate.source_repo !== task.target_repo) {
            throw new Error(`${task.id} emitted candidate for wrong source_repo ${candidate.source_repo}`);
          }
          if (candidate.source_cell_id !== task.target_cell_id) {
            throw new Error(`${task.id} emitted candidate for wrong source_cell_id ${candidate.source_cell_id}`);
          }
          if (candidate.source_package_id !== task.target_package_id) {
            throw new Error(`${task.id} emitted candidate for wrong source_package_id ${candidate.source_package_id}`);
          }
          if (candidate.source_repo_sha !== result.candidate_sha) {
            throw new Error(`${task.id} candidate source_repo_sha must equal integrated package SHA ${result.candidate_sha}`);
          }
          nextTasks.push(buildChronicaCandidateTask({ candidate, currentChronicaSha }));
        }
        refreshRequired.add(task.target_repo);
        break;
      }

      case "CHRONICA_CANDIDATE":
        nextTasks.push(buildChronicaVerifyTask(task, result, currentChronicaSha));
        break;

      case "CHRONICA_VERIFY_INTEGRATE": {
        if (!task.chronica_candidate) throw new Error(`${task.id} missing candidate context`);
        const reconverge = buildReconvergePlan({
          candidate: task.chronica_candidate,
          mergedChronicaSha: result.candidate_sha,
          reports,
          registry,
          now,
        });
        nextTasks.push(...reconverge.tasks);
        refreshRequired.add("llgtrn/Chronica");
        break;
      }

      case "CELL_RESET_BASELINE":
      case "CELL_ENROLL":
      case "OPS_ENROLL":
      case "DONOR_INTAKE":
      case "DONOR_ACCOUNT":
      case "DONOR_RELOCATE":
      case "DONOR_CENSUS":
      case "PACKAGE_RECONVERGE":
      case "OPS_RECONVERGE":
        refreshRequired.add(task.target_repo);
        break;

      default:
        throw new Error(`no fleet advance rule for task kind ${task.kind}`);
    }
  }

  const nextPlan = buildPlan(nextTasks, registry.canonical_repo, now);
  const advanced = {
    schema: "chronica.system-atlas.fleet-advance.v1",
    generated_at: now,
    consumed_results_total: results.length,
    blockers,
    refresh_required: [...refreshRequired].sort(),
    next_plan: nextPlan,
  };
  validateInstance(advanced, "chronica.system-atlas.fleet-advance.v1");
  return advanced;
}

function argValues(args, name) {
  const values = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i] === name && args[i + 1]) values.push(args[++i]);
  }
  return values;
}

function argValue(args, name) {
  const values = argValues(args, name);
  return values.length ? values[values.length - 1] : null;
}

function main(args = process.argv.slice(2)) {
  const planPath = argValue(args, "--plan");
  const resultPaths = argValues(args, "--result");
  const reportPaths = argValues(args, "--report");
  const currentChronicaSha = argValue(args, "--chronica-sha");
  const out = argValue(args, "--out");
  if (!planPath) throw new Error("--plan is required");
  if (!resultPaths.length) throw new Error("at least one --result is required");

  const plan = JSON.parse(readFileSync(resolve(planPath), "utf8"));
  const results = resultPaths.map((path) => JSON.parse(readFileSync(resolve(path), "utf8")));
  const reports = reportPaths.map((path) => JSON.parse(readFileSync(resolve(path), "utf8")));
  const advanced = advanceFleet({ plan, results, reports, currentChronicaSha });
  const text = `${JSON.stringify(advanced, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
}

if (process.argv[1] && process.argv[1].endsWith("fleet-advance.mjs")) {
  try { main(); } catch (error) { console.error(`fleet advance failed: ${error.message}`); process.exit(2); }
}
