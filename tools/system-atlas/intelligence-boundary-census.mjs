import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { parse as parseYaml } from "yaml";
import { REPO_ROOT } from "./lib.mjs";

const FORBIDDEN_CORE_DEPENDENCIES = new Set([
  "tokio",
  "reqwest",
  "hyper",
  "tonic",
  "axum",
  "candle-core",
  "candle-nn",
  "burn",
  "tch",
  "ort",
  "async-openai",
  "anthropic",
  "google-generative-ai",
]);

const STRATEGIC_INTELLIGENCE_DONORS = [
  ["D144", "kserve/kserve", "INFERENCE_SERVING"],
  ["D174", "data-privacy-stack/presidio", "CONTEXT_REDACTION"],
  ["D307", "vllm-project/vllm", "LOCAL_INFERENCE_ENGINE"],
  ["D340", "BerriAI/litellm", "MULTI_PROVIDER_ROUTING"],
  ["D341", "theagentrouter/agent-router", "AI_GATEWAY_CONTROL_PLANE"],
  ["D342", "sgl-project/sglang", "HIGH_PERFORMANCE_LOCAL_SERVING"],
  ["D343", "open-inference/open-inference-protocol", "PROVIDER_NEUTRAL_INFERENCE_PROTOCOL"],
  ["D344", "cedar-policy/cedar", "POLICY_AUTHORIZATION_SEMANTICS"],
  ["D345", "gramineproject/gramine", "CONFIDENTIAL_COMPUTE"],
];

const PROVIDER_TOKENS = [
  "openai",
  "anthropic",
  "claude",
  "gemini",
  "qwen",
  "deepseek",
  "sglang",
  "vllm",
  "openrouter",
  "litellm",
];

function gitTrackedFiles(root = REPO_ROOT) {
  return execFileSync("git", ["ls-files"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  }).split("\n").map((x) => x.trim()).filter(Boolean);
}

function read(root, path) {
  try {
    return readFileSync(resolve(root, path), "utf8");
  } catch {
    return "";
  }
}

function productionRustFiles(trackedFiles) {
  return trackedFiles.filter((path) =>
    (path.startsWith("core/src/")
      || path.startsWith("runtime/src/")
      || path.startsWith("adapter/src/")
      || path.startsWith("organism/src/"))
    && path.endsWith(".rs")
    && !path.endsWith("_kernel_proof.rs"),
  );
}

function containsProductionSymbol(trackedFiles, root, pattern) {
  return productionRustFiles(trackedFiles).some((path) => pattern.test(read(root, path)));
}

function containsCoreSymbol(trackedFiles, root, pattern) {
  return productionRustFiles(trackedFiles)
    .filter((path) => path.startsWith("core/src/"))
    .some((path) => pattern.test(read(root, path)));
}

function coreDependencyNames(root) {
  const source = read(root, "core/Cargo.toml");
  const lines = source.split("\n");
  let inDependencies = false;
  const names = [];
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === "[dependencies]") {
      inDependencies = true;
      continue;
    }
    if (inDependencies && trimmed.startsWith("[")) break;
    if (!inDependencies || trimmed.length === 0 || trimmed.startsWith("#")) continue;
    const match = trimmed.match(/^([A-Za-z0-9_.-]+)\s*=/);
    if (match) names.push(match[1]);
  }
  return names;
}

function coreSourceMechanicLeaks(trackedFiles, root) {
  const leaks = [];
  const importPattern = /^\s*(?:use|extern crate)\s+([^;]+);/gm;
  for (const path of trackedFiles.filter((p) => p.startsWith("core/src/") && p.endsWith(".rs"))) {
    const source = read(root, path);
    for (const match of source.matchAll(importPattern)) {
      const imported = match[1].toLowerCase();
      if (
        imported.includes("std::net")
        || imported.includes("tokio")
        || imported.includes("reqwest")
        || imported.includes("hyper")
        || imported.includes("tonic")
        || PROVIDER_TOKENS.some((token) => imported.includes(token))
      ) {
        leaks.push({ path, import: match[1] });
      }
    }
  }
  return leaks;
}

function providerNamedRuntimePaths(trackedFiles) {
  return trackedFiles.filter((path) => {
    if (!path.startsWith("runtime/")) return false;
    const lower = path.toLowerCase();
    return PROVIDER_TOKENS.some((token) => lower.includes(token));
  });
}

function holdingReferencePaths(trackedFiles, root) {
  return productionRustFiles(trackedFiles)
    .filter((path) => path.startsWith("core/") || path.startsWith("runtime/"))
    .filter((path) => read(root, path).includes("HoldingId"))
    .sort();
}

function state(id, value, evidence = []) {
  return { id, state: value, evidence };
}

function strategicDonorSourceReadiness(root) {
  const source = read(root, "tools/refoundation/donor-corpus.yaml");
  let checklist = {};
  try {
    checklist = parseYaml(source) ?? {};
  } catch {
    checklist = {};
  }
  const donors = Array.isArray(checklist.donors) ? checklist.donors : [];
  return STRATEGIC_INTELLIGENCE_DONORS.map(([donorId, repo, role]) => {
    const donor = donors.find((row) => row?.id === donorId || row?.repo === repo);
    if (!donor) return state(`DONOR_${role}`, "GAP", []);
    if (donor.source_present === true) {
      return state(`DONOR_${role}`, "PRESENT", [
        `${donor.id} ${donor.repo}`,
        donor.temporary_path ?? "source-present donor",
      ]);
    }
    return state(`DONOR_${role}`, "GAP", [`${donor.id} ${donor.repo} registered; source not present`]);
  });
}

export function buildIntelligenceBoundaryAtlas({
  trackedFiles = gitTrackedFiles(),
  root = REPO_ROOT,
  sourceSha = null,
  generatedAt = new Date().toISOString(),
} = {}) {
  const present = (id, path) => state(id, "PRESENT", [path]);
  const gap = (id) => state(id, "GAP", []);
  const proof = (id, path) => state(id, "PROOF_ONLY", [path]);

  const readiness = [
    trackedFiles.includes("core/src/holding.rs") ? present("HOLDING_KERNEL_VOCABULARY", "core/src/holding.rs") : gap("HOLDING_KERNEL_VOCABULARY"),
    trackedFiles.includes("core/src/identity.rs") ? present("IDENTITY_KERNEL_VOCABULARY", "core/src/identity.rs") : gap("IDENTITY_KERNEL_VOCABULARY"),
    trackedFiles.includes("core/src/resource.rs") ? present("RESOURCE_KERNEL_VOCABULARY", "core/src/resource.rs") : gap("RESOURCE_KERNEL_VOCABULARY"),
    containsCoreSymbol(trackedFiles, root, /pub\s+(?:struct|enum)\s+Purpose\b/)
      ? state("PURPOSE_KERNEL_VOCABULARY", "PRESENT", ["production Rust symbol: Purpose"])
      : gap("PURPOSE_KERNEL_VOCABULARY"),
    containsCoreSymbol(trackedFiles, root, /pub\s+(?:struct|enum)\s+Sensitivity\b/)
      ? state("SENSITIVITY_KERNEL_VOCABULARY", "PRESENT", ["production Rust symbol: Sensitivity"])
      : gap("SENSITIVITY_KERNEL_VOCABULARY"),
    trackedFiles.includes("runtime/src/authority.rs") ? present("AUTHORITY_RUNTIME", "runtime/src/authority.rs") : gap("AUTHORITY_RUNTIME"),
    trackedFiles.includes("runtime/src/privacy.rs") ? present("PRIVACY_RUNTIME", "runtime/src/privacy.rs") : gap("PRIVACY_RUNTIME"),
    trackedFiles.includes("runtime/src/context.rs") ? present("CONTEXT_RUNTIME", "runtime/src/context.rs") : gap("CONTEXT_RUNTIME"),
    trackedFiles.includes("runtime/src/resource_ownership.rs") ? present("OWNERSHIP_RUNTIME", "runtime/src/resource_ownership.rs") : gap("OWNERSHIP_RUNTIME"),
    trackedFiles.includes("runtime/src/execution_admission.rs") ? present("EXECUTION_ADMISSION_RUNTIME", "runtime/src/execution_admission.rs") : gap("EXECUTION_ADMISSION_RUNTIME"),
    trackedFiles.includes("runtime/src/model_registry.rs") ? present("MODEL_REGISTRY_RUNTIME", "runtime/src/model_registry.rs") : gap("MODEL_REGISTRY_RUNTIME"),
    trackedFiles.includes("runtime/src/model_evaluation.rs") ? present("MODEL_EVALUATION_RUNTIME", "runtime/src/model_evaluation.rs") : gap("MODEL_EVALUATION_RUNTIME"),
    trackedFiles.includes("runtime/src/model_lifecycle.rs") ? present("MODEL_LIFECYCLE_RUNTIME", "runtime/src/model_lifecycle.rs") : gap("MODEL_LIFECYCLE_RUNTIME"),
    trackedFiles.includes("adapter/src/provider_adapter.rs") ? present("GENERIC_PROVIDER_ADAPTER", "adapter/src/provider_adapter.rs") : gap("GENERIC_PROVIDER_ADAPTER"),
    trackedFiles.includes("adapter/src/context_disclosure_kernel_proof.rs") ? proof("EXTERNAL_CONTEXT_DISCLOSURE", "adapter/src/context_disclosure_kernel_proof.rs") : gap("EXTERNAL_CONTEXT_DISCLOSURE"),
    trackedFiles.includes("adapter/src/model_routing_kernel_proof.rs") ? proof("MODEL_ROUTING", "adapter/src/model_routing_kernel_proof.rs") : gap("MODEL_ROUTING"),
    containsProductionSymbol(trackedFiles, root, /pub\s+struct\s+IntelligenceInvocation\b/)
      ? state("INTELLIGENCE_INVOCATION_CONTRACT", "PRESENT", ["production Rust symbol: IntelligenceInvocation"])
      : gap("INTELLIGENCE_INVOCATION_CONTRACT"),
    containsProductionSymbol(trackedFiles, root, /pub\s+struct\s+IntelligenceSelection\b/)
      ? state("MULTI_PROVIDER_SELECTION_RUNTIME", "PRESENT", ["production Rust symbol: IntelligenceSelection"])
      : gap("MULTI_PROVIDER_SELECTION_RUNTIME"),
    containsProductionSymbol(trackedFiles, root, /pub\s+(?:struct|enum)\s+LearningUseDecision\b/)
      ? state("LEARNING_USE_DECISION_RUNTIME", "PRESENT", ["production Rust symbol: LearningUseDecision"])
      : gap("LEARNING_USE_DECISION_RUNTIME"),
    ...strategicDonorSourceReadiness(root),
  ];

  const dependencyLeaks = coreDependencyNames(root)
    .filter((name) => FORBIDDEN_CORE_DEPENDENCIES.has(name))
    .map((dependency) => ({
      type: "FORBIDDEN_CORE_IO_OR_PROVIDER_DEPENDENCY",
      severity: "HARD",
      path: "core/Cargo.toml",
      detail: dependency,
    }));

  const sourceLeaks = coreSourceMechanicLeaks(trackedFiles, root).map((leak) => ({
    type: "FORBIDDEN_CORE_PROVIDER_OR_NETWORK_MECHANIC",
    severity: "HARD",
    path: leak.path,
    detail: leak.import,
  }));

  const runtimeProviderPaths = providerNamedRuntimePaths(trackedFiles).map((path) => ({
    type: "PROVIDER_SPECIFIC_RUNTIME_PATH_REVIEW",
    severity: "REVIEW_REQUIRED",
    path,
    detail: basename(path),
  }));

  const violations = [...dependencyLeaks, ...sourceLeaks, ...runtimeProviderPaths]
    .map((row, index) => ({
      id: "INTEL-BOUNDARY-" + String(index + 1).padStart(4, "0"),
      ...row,
    }));

  const holdingRefs = holdingReferencePaths(trackedFiles, root);
  const presentTotal = readiness.filter((x) => x.state === "PRESENT").length;
  const proofOnlyTotal = readiness.filter((x) => x.state === "PROOF_ONLY").length;
  const gapTotal = readiness.filter((x) => x.state === "GAP").length;

  return {
    schema: "chronica.system-atlas.intelligence-boundary.v1",
    generated_at: generatedAt,
    source_sha: sourceSha,
    source_of_truth: "generated projection over repository structure/source; never policy or canonical runtime truth",
    summary: {
      readiness_present_total: presentTotal,
      readiness_proof_only_total: proofOnlyTotal,
      readiness_gap_total: gapTotal,
      hard_violations_total: violations.filter((v) => v.severity === "HARD").length,
      review_required_total: violations.filter((v) => v.severity === "REVIEW_REQUIRED").length,
      holding_id_reference_files_total: holdingRefs.length,
      core_dependencies_total: coreDependencyNames(root).length,
    },
    readiness,
    holding_id_reference_paths: holdingRefs,
    violations,
  };
}

export function currentIntelligenceBoundaryAtlas() {
  const sourceSha = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  }).trim();
  return buildIntelligenceBoundaryAtlas({ sourceSha });
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].replaceAll("\\", "/"))) {
  console.log(JSON.stringify(currentIntelligenceBoundaryAtlas(), null, 2));
}
