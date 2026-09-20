#!/usr/bin/env node
import { spawn } from "node:child_process";
import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compileSchemas, validate, formatErrors } from "../schema-validate.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..", "..");
const SCHEMA_DIR = resolve(ROOT, "tools/system-atlas/schema");

const PROVIDERS = {
  CLAUDE: "CHRONICA_FLEET_CLAUDE_RUNNER",
  CODEX: "CHRONICA_FLEET_CODEX_RUNNER",
};

const LEGACY_NON_EXECUTABLE_TASKS = new Set([
  "CELL_ENROLL",
  "OPS_ENROLL",
  "DONOR_RELOCATE",
  "DONOR_ABSORB_BUILD",
  "OPS_VERIFY_INTEGRATE",
  "OPS_RECONVERGE",
]);

function resolveProviderRunner(provider) {
  const envName = PROVIDERS[provider];
  const configured = process.env[envName];
  if (configured) return { command: configured, args: [] };
  if (provider === "CLAUDE") {
    return {
      command: process.execPath,
      args: [resolve(HERE, "providers", "claude-code-runner.mjs")],
    };
  }
  throw new Error(
    `${envName} is not configured. Supply a provider runner that accepts one task JSON path and returns one fleet-worker-result JSON object.`,
  );
}

function validateInstance(instance, schemaId) {
  const result = validate(instance, schemaId, compileSchemas(SCHEMA_DIR));
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}

export function validateWorkerResult(task, result) {
  validateInstance(result, "chronica.system-atlas.fleet-worker-result.v1");
  if (result.task_id !== task.id) throw new Error(`worker result task mismatch: ${result.task_id} != ${task.id}`);
  if (result.target_repo !== task.target_repo) throw new Error(`worker result repo mismatch: ${result.target_repo} != ${task.target_repo}`);
  if (result.target_cell_id !== (task.target_cell_id ?? null)) {
    throw new Error(`worker result cell mismatch: ${result.target_cell_id} != ${task.target_cell_id ?? null}`);
  }
  if (result.target_package_id !== (task.target_package_id ?? null)) {
    throw new Error(`worker result package mismatch: ${result.target_package_id} != ${task.target_package_id ?? null}`);
  }
  if (task.base_sha && result.base_sha !== task.base_sha) {
    throw new Error(`worker result base mismatch: ${result.base_sha} != ${task.base_sha}`);
  }
  const mutatingKinds = new Set([
    "CELL_RESET_BASELINE",
    "CELL_ENROLL",
    "OPS_ENROLL",
    "DONOR_INTAKE",
    "DONOR_ACCOUNT",
    "DONOR_RELOCATE",
    "DONOR_CENSUS",
    "PACKAGE_BUILD",
    "DONOR_ABSORB_BUILD",
    "PACKAGE_VERIFY_INTEGRATE",
    "OPS_VERIFY_INTEGRATE",
    "CHRONICA_CANDIDATE",
    "CHRONICA_VERIFY_INTEGRATE",
    "PACKAGE_RECONVERGE",
    "OPS_RECONVERGE",
  ]);
  if (result.status === "COMPLETE" && !result.base_sha) {
    throw new Error(`COMPLETE ${task.kind} worker result requires exact base_sha`);
  }
  if (result.status === "COMPLETE" && mutatingKinds.has(task.kind) && !result.candidate_sha) {
    throw new Error(`COMPLETE ${task.kind} worker result requires candidate_sha`);
  }
  if (task.kind !== "DONOR_ABSORB_SCOUT" && result.discovered_slices.length > 0) {
    throw new Error(`${task.kind} must not emit discovered_slices`);
  }
  if (!["PACKAGE_VERIFY_INTEGRATE","OPS_VERIFY_INTEGRATE"].includes(task.kind) && result.chronica_candidates.length > 0) {
    throw new Error(`${task.kind} must not emit chronica_candidates`);
  }
  if (result.status === "COMPLETE" && task.kind === "DONOR_ABSORB_SCOUT" && result.candidate_sha) {
    throw new Error("read-only DONOR_ABSORB_SCOUT must not return candidate_sha");
  }
  if (result.status === "COMPLETE" && task.kind === "DONOR_ABSORB_SCOUT") {
    if (result.discovered_slices.length < 1 || result.discovered_slices.length > 3) {
      throw new Error("DONOR_ABSORB_SCOUT must return 1-3 executable slices; no open-ended scouting");
    }
    if (!task.donor_source_path) {
      throw new Error("DONOR_ABSORB_SCOUT requires a standardized donor_source_path");
    }
    for (const slice of result.discovered_slices) {
      if (!slice.deletion_scope.length) throw new Error(`slice ${slice.id} has empty deletion_scope`);
      for (const scope of slice.deletion_scope) {
        const normalized = String(scope).replaceAll("\\", "/").replace(/^\.\//, "");
        const donorRoot = String(task.donor_source_path).replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/+$/, "");
        if (!(normalized === donorRoot || normalized.startsWith(`${donorRoot}/`))) {
          throw new Error(`slice ${slice.id} deletion scope escapes donor root: ${scope}`);
        }
      }
    }
  }
  if (result.status === "COMPLETE" && ["PACKAGE_BUILD","DONOR_ABSORB_BUILD"].includes(task.kind)) {
    const terminal = ["SOURCE_EXTINCT","CLASSIFIED_AND_REMOVED"].includes(result.extinction.state);
    if (!result.extinction.applicable) {
      throw new Error("PACKAGE_BUILD must report donor extinction as applicable");
    }
    if (result.extinction.files_removed <= 0 && !terminal) {
      throw new Error("PACKAGE_BUILD cannot COMPLETE without donor source reduction; DELETE SOMETHING or return BLOCKED with a concrete dependency closure");
    }

    // Fused fast-path PACKAGE_BUILD tasks intentionally start without a pre-scoped
    // deletion closure. The worker must discover exactly one bounded closure while
    // reading the minimum donor surface, then prove its actual deletions here.
    if (task.kind === "PACKAGE_BUILD" && !(task.deletion_scope?.length)) {
      if (!task.donor_source_path) {
        throw new Error("fused PACKAGE_BUILD requires donor_source_path");
      }
      if (!result.extinction.deleted_paths.length && !terminal) {
        throw new Error("fused PACKAGE_BUILD cannot COMPLETE without concrete deleted_paths");
      }
      const donorRoot = String(task.donor_source_path).replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/+$/, "");
      for (const path of result.extinction.deleted_paths) {
        const normalized = String(path).replaceAll("\\", "/").replace(/^\.\//, "");
        if (!(normalized === donorRoot || normalized.startsWith(`${donorRoot}/`))) {
          throw new Error(`fused PACKAGE_BUILD deletion escapes donor root: ${path}`);
        }
      }
    }
  }
  if (result.status === "BLOCKED" && ["PACKAGE_BUILD","DONOR_ABSORB_BUILD"].includes(task.kind)) {
    if (!result.extinction.blocker) {
      throw new Error("blocked PACKAGE_BUILD requires a concrete extinction blocker/dependency closure");
    }
  }
  if (result.status === "COMPLETE" && task.kind === "CELL_RESET_BASELINE") {
    if (!result.reset_evidence.applicable) throw new Error("CELL_RESET_BASELINE requires reset evidence");
    if (result.reset_evidence.generation !== task.refoundation_generation) {
      throw new Error(`reset generation mismatch: ${result.reset_evidence.generation} != ${task.refoundation_generation}`);
    }
    if (result.reset_evidence.pre_reset_head_sha !== result.base_sha) {
      throw new Error("CELL_RESET_BASELINE pre_reset_head_sha must equal measured base_sha");
    }
    if (!result.reset_evidence.preservation_ref) {
      throw new Error("CELL_RESET_BASELINE requires preservation_ref");
    }
  }
  return result;
}

function argValue(args, name) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : null;
}

function runProvider(runner, task) {
  return new Promise((resolveResult) => {
    const dir = mkdtempSync(join(tmpdir(), "chronica-fleet-task-"));
    const taskPath = join(dir, "task.json");
    writeFileSync(taskPath, `${JSON.stringify(task, null, 2)}\n`);
    const child = spawn(runner.command, [...runner.args, taskPath], { stdio: ["ignore","pipe","pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", (error) => resolveResult({ task_id: task.id, status: "RUNNER_ERROR", error: error.message }));
    child.on("close", (code) => {
      if (code !== 0) {
        resolveResult({ task_id: task.id, status: "RUNNER_FAILED", exit_code: code, stderr: stderr.trim() });
        return;
      }
      try {
        const result = validateWorkerResult(task, JSON.parse(stdout));
        resolveResult({ task_id: task.id, status: "RETURNED", result });
      } catch (error) {
        resolveResult({
          task_id: task.id,
          status: "INVALID_RUNNER_RESULT",
          error: error.message,
          stdout: stdout.trim(),
          stderr: stderr.trim(),
        });
      }
    });
  });
}

export async function dispatchFleetWave({ plan, provider, execute = false } = {}) {
  if (!plan || plan.schema !== "chronica.system-atlas.fleet-work-plan.v1") throw new Error("invalid fleet plan");
  if (!PROVIDERS[provider]) throw new Error(`unsupported provider ${provider}`);
  const tasksById = new Map(plan.tasks.map((task) => [task.id, task]));
  const tasks = plan.ready_wave.map((id) => tasksById.get(id)).filter(Boolean);

  const repos = new Set();
  for (const task of tasks) {
    if (LEGACY_NON_EXECUTABLE_TASKS.has(task.kind)) {
      throw new Error(`legacy task kind is non-executable after Cell refoundation: ${task.kind}`);
    }
    if (repos.has(task.target_repo)) throw new Error(`ready wave mutates ${task.target_repo} more than once`);
    repos.add(task.target_repo);
  }

  if (!execute) {
    return {
      provider,
      mode: "DRY_RUN",
      tasks_total: tasks.length,
      tasks: tasks.map((task) => ({
        id: task.id,
        target_repo: task.target_repo,
        target_cell_id: task.target_cell_id ?? null,
        target_package_id: task.target_package_id ?? null,
        target_ref: task.target_ref,
        base_sha: task.base_sha,
        chronica_reference_sha: task.chronica_reference_sha,
      })),
    };
  }

  const runner = resolveProviderRunner(provider);

  // Different repositories can run concurrently. The plan guarantees one mutation task per repo in this wave.
  const results = await Promise.all(tasks.map((task) => runProvider(runner, task)));
  return { provider, mode: "EXECUTE", tasks_total: tasks.length, results };
}

async function main(args = process.argv.slice(2)) {
  const planPath = argValue(args, "--plan");
  const provider = (argValue(args, "--provider") ?? "CLAUDE").toUpperCase();
  const execute = args.includes("--execute");
  const out = argValue(args, "--out");
  if (!planPath) throw new Error("--plan <fleet-plan.json> is required");
  const plan = JSON.parse(readFileSync(resolve(planPath), "utf8"));
  const result = await dispatchFleetWave({ plan, provider, execute });
  const text = `${JSON.stringify(result, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
}

try {
  if (process.argv[1] && process.argv[1].endsWith("fleet-dispatcher.mjs")) await main();
} catch (error) {
  console.error(`fleet dispatcher failed: ${error.message}`);
  process.exit(2);
}
