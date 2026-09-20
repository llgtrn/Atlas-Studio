#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { compileSchemas, validate, formatErrors } from "../schema-validate.mjs";
import { loadFleetRegistry } from "./donor-allocator.mjs";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..", "..");
const SCHEMA_DIR = resolve(ROOT, "tools/system-atlas/schema");

function validateInstance(instance, schemaId) {
  const result = validate(instance, schemaId, compileSchemas(SCHEMA_DIR));
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}
function slug(value) { return String(value).replace(/[^A-Za-z0-9_.-]+/g, "-"); }

export function buildChronicaCandidateTask({ candidate, currentChronicaSha } = {}) {
  validateInstance(candidate, "chronica.system-atlas.chronica-candidate.v1");
  if (!/^[0-9a-f]{40}$/.test(currentChronicaSha ?? "")) throw new Error("currentChronicaSha must be exact");
  return {
    id: `chronica-candidate:${slug(candidate.candidate_id)}`,
    kind: "CHRONICA_CANDIDATE",
    target_repo: "llgtrn/Chronica",
    target_cell_id: null,
    target_package_id: null,
    target_package_kind: null,
    refoundation_generation: null,
    target_ref: "main",
    base_sha: currentChronicaSha,
    donor_id: candidate.donor_ids[0] ?? null,
    donor_repo: null,
    donor_revision: null,
    donor_source_path: null,
    candidate_sha: null,
    chronica_candidate: candidate,
    state: "READY",
    depends_on: [],
    owned_scope: [candidate.semantic_owner],
    deletion_scope: [],
    chronica_reference_sha: currentChronicaSha,
    instructions:
      `Implement the proven reusable semantic from Development Cell ${candidate.source_cell_id}, package ${candidate.source_package_id}, repo ${candidate.source_repo}@${candidate.source_repo_sha} into the current Chronica owner ${candidate.semantic_owner}. Re-read current Chronica at the assigned base SHA; preserve authority/evidence/recovery invariants; treat package evidence as proposal/proof input, not canonical truth. Candidate: ${candidate.summary}`,
  };
}

export function buildChronicaCandidatePlan({
  candidate,
  currentChronicaSha,
  now = new Date().toISOString(),
  registry = loadFleetRegistry(),
} = {}) {
  const task = buildChronicaCandidateTask({ candidate, currentChronicaSha });
  const plan = {
    schema: "chronica.system-atlas.fleet-work-plan.v1",
    generated_at: now,
    canonical_repo: registry.canonical_repo,
    tasks_total: 1,
    ready_wave_total: 1,
    deferred_total: 0,
    tasks: [task],
    ready_wave: [task.id],
  };
  validateInstance(plan, "chronica.system-atlas.fleet-work-plan.v1");
  return plan;
}

export function buildReconvergePlan({
  candidate,
  mergedChronicaSha,
  reports = [],
  registry = loadFleetRegistry(),
  now = new Date().toISOString(),
} = {}) {
  validateInstance(candidate, "chronica.system-atlas.chronica-candidate.v1");
  if (!/^[0-9a-f]{40}$/.test(mergedChronicaSha ?? "")) throw new Error("mergedChronicaSha must be exact");
  const registered = new Map(registry.development_cells.map((row) => [row.repo, row]));
  const reportByRepo = new Map(
    reports
      .map((row) => [row.cell_repo ?? row.ops_repo, row])
      .filter(([repo]) => Boolean(repo)),
  );
  const targetPackages = [
    { repo: candidate.source_repo, cell_id: candidate.source_cell_id, package_id: candidate.source_package_id },
    ...candidate.affected_packages,
  ];
  const deduped = new Map(targetPackages.map((row) => [row.repo, row]));
  const tasks = [];

  for (const target of [...deduped.values()].sort((a, b) => a.repo.localeCompare(b.repo))) {
    const repo = target.repo;
    const cell = registered.get(repo);
    if (!cell) throw new Error(`candidate references unregistered Development Cell ${repo}`);
    if (cell.cell_id !== target.cell_id || cell.package_id !== target.package_id) {
      throw new Error(`candidate package identity mismatch for ${repo}`);
    }
    const report = reportByRepo.get(repo);
    const baseSha = report?.cell_sha ?? report?.ops_sha ?? null;
    tasks.push({
      id: `package-reconverge:${slug(candidate.candidate_id)}:${slug(repo)}`,
      kind: "PACKAGE_RECONVERGE",
      target_repo: repo,
      target_cell_id: cell.cell_id,
      target_package_id: cell.package_id,
      target_package_kind: cell.package_kind,
      refoundation_generation: Number(cell.refoundation_generation ?? 1),
      target_ref: cell.default_ref ?? "main",
      base_sha,
      donor_id: candidate.donor_ids[0] ?? null,
      donor_repo: null,
      donor_revision: null,
      donor_source_path: null,
      candidate_sha: null,
      state: baseSha ? "READY" : "BLOCKED",
      depends_on: [],
      owned_scope: [
        "chronica-package.json","chronica-mirror.json",
        "core/","runtime/","adapter/","organism/","graph/","bindings/","apps/","docs/",
      ],
      deletion_scope: [],
      chronica_reference_sha: mergedChronicaSha,
      instructions:
        `Reconverge Development Cell ${cell.cell_id} / package ${cell.package_id} onto Chronica ${mergedChronicaSha} after candidate ${candidate.candidate_id} was integrated. Update the pinned Chronica reference, remove local duplicate canonical semantics where applicable, preserve package-local adapters/projections/provider mechanics, keep implementation in Chronica-isomorphic responsibility roots only, keep UI under apps/ui TypeScript, run local tests and regenerate the Mirror report. The package remains independently developable but not independently canonical.`,
    });
  }

  const ready = tasks.filter((task) => task.state === "READY").map((task) => task.id);
  const plan = {
    schema: "chronica.system-atlas.fleet-work-plan.v1",
    generated_at: now,
    canonical_repo: registry.canonical_repo,
    tasks_total: tasks.length,
    ready_wave_total: ready.length,
    deferred_total: tasks.length - ready.length,
    tasks,
    ready_wave: ready,
  };
  validateInstance(plan, "chronica.system-atlas.fleet-work-plan.v1");
  return plan;
}

function argValue(args, name) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : null;
}

function main(args = process.argv.slice(2)) {
  const candidatePath = argValue(args, "--candidate");
  const base = argValue(args, "--chronica-base-sha");
  const merged = argValue(args, "--chronica-sha");
  const out = argValue(args, "--out");
  if (!candidatePath) throw new Error("--candidate is required");
  if (Boolean(base) === Boolean(merged)) {
    throw new Error("provide exactly one of --chronica-base-sha (candidate integration plan) or --chronica-sha (package reconvergence plan)");
  }
  const candidate = JSON.parse(readFileSync(resolve(candidatePath), "utf8"));
  const plan = base
    ? buildChronicaCandidatePlan({ candidate, currentChronicaSha: base })
    : buildReconvergePlan({ candidate, mergedChronicaSha: merged, reports: [] });
  const text = `${JSON.stringify(plan, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
}

if (process.argv[1] && process.argv[1].endsWith("fleet-reconverge.mjs")) {
  try { main(); } catch (error) { console.error(`fleet package reconverge failed: ${error.message}`); process.exit(2); }
}
