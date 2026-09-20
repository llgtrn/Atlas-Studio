import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { parse as parseYaml } from "yaml";
import { REPO_ROOT } from "./lib.mjs";

const CHECKLIST = resolve(REPO_ROOT, "tools/refoundation/donor-corpus.yaml");

export const MODEL_ARTIFACT_EXTENSIONS = new Set([
  ".safetensors", ".gguf", ".ckpt", ".pth", ".pt", ".onnx",
]);

export const EXPECTED_ORGANISM_SURFACES = {
  organism_semantics: [
    "organism/src/observation.rs",
    "organism/src/world_model.rs",
    "organism/src/simulation.rs",
    "organism/src/planning.rs",
    "organism/src/evaluation.rs",
    "organism/src/dream.rs",
  ],
  runtime_model_lifecycle: [
    "runtime/src/experience.rs",
    "runtime/src/dataset_registry.rs",
    "runtime/src/model_training.rs",
    "runtime/src/model_evaluation.rs",
    "runtime/src/model_registry.rs",
    "runtime/src/model_lifecycle.rs",
  ],
  adapter_model_mechanics: [
    "adapter/src/linear_world_model.rs",
  ],
};

function extname(path) {
  const base = String(path).split("/").pop() ?? "";
  const idx = base.lastIndexOf(".");
  return idx >= 0 ? base.slice(idx).toLowerCase() : "";
}

export function isProductionModelArtifactPath(path) {
  const normalized = String(path).replaceAll("\\", "/");
  if (normalized.startsWith("temporary/donors/")) return false;
  if (!MODEL_ARTIFACT_EXTENSIONS.has(extname(normalized))) return false;
  return normalized.startsWith("core/")
    || normalized.startsWith("runtime/")
    || normalized.startsWith("adapter/")
    || normalized.startsWith("organism/")
    || normalized.startsWith("apps/")
    || normalized.startsWith("deploy/");
}

export function selectOrganismDonors(checklist) {
  return (checklist?.donors ?? []).filter((donor) =>
    donor?.domain === "organism"
    || (Array.isArray(donor?.organism_roles) && donor.organism_roles.length > 0),
  );
}

function readChecklist() {
  return parseYaml(readFileSync(CHECKLIST, "utf8"));
}

function gitTrackedFiles(root = REPO_ROOT) {
  return execFileSync("git", ["ls-files"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  }).split("\n").map((x) => x.trim()).filter(Boolean);
}

function safeSourceContains(root, path, tokens) {
  try {
    const source = readFileSync(resolve(root, path), "utf8");
    return tokens.every((token) => source.includes(token));
  } catch {
    return false;
  }
}

function hasMaterializedSymbol(trackedFiles, root, token) {
  const prefixes = ["core/", "runtime/", "adapter/", "organism/"];
  return trackedFiles
    .filter((path) => prefixes.some((prefix) => path.startsWith(prefix)) && path.endsWith(".rs"))
    .some((path) => safeSourceContains(root, path, [token]));
}

export function buildEvolutionReadiness(trackedFiles, root = REPO_ROOT) {
  const present = (id, evidence) => ({ id, state: "PRESENT", evidence: [evidence] });
  const gap = (id) => ({ id, state: "GAP", evidence: [] });
  const proof = (id, evidence) => ({ id, state: "PROOF_ONLY", evidence: [evidence] });

  const out = [];
  out.push(trackedFiles.includes("runtime/src/experience.rs")
    ? present("EXPERIENCE_RUNTIME", "runtime/src/experience.rs")
    : gap("EXPERIENCE_RUNTIME"));
  out.push(safeSourceContains(root, "runtime/src/experience.rs", ["TeacherValidation"])
    ? present("TEACHER_VALIDATION_RUNTIME", "runtime/src/experience.rs")
    : gap("TEACHER_VALIDATION_RUNTIME"));
  out.push(trackedFiles.includes("runtime/src/dataset_registry.rs")
    ? present("DATASET_REGISTRY_RUNTIME", "runtime/src/dataset_registry.rs")
    : gap("DATASET_REGISTRY_RUNTIME"));
  out.push(trackedFiles.includes("runtime/src/model_training.rs")
    ? present("MODEL_TRAINING_RUNTIME", "runtime/src/model_training.rs")
    : gap("MODEL_TRAINING_RUNTIME"));
  out.push(trackedFiles.includes("runtime/src/model_evaluation.rs")
    ? present("MODEL_EVALUATION_RUNTIME", "runtime/src/model_evaluation.rs")
    : gap("MODEL_EVALUATION_RUNTIME"));
  out.push(trackedFiles.includes("runtime/src/model_registry.rs")
    ? present("MODEL_REGISTRY_RUNTIME", "runtime/src/model_registry.rs")
    : gap("MODEL_REGISTRY_RUNTIME"));
  out.push(safeSourceContains(root, "runtime/src/model_lifecycle.rs", ["Shadowed", "CanaryStarted", "Activated", "Retired"])
    ? present("SHADOW_CANARY_LIFECYCLE_RUNTIME", "runtime/src/model_lifecycle.rs")
    : gap("SHADOW_CANARY_LIFECYCLE_RUNTIME"));
  out.push(trackedFiles.includes("adapter/src/context_disclosure_kernel_proof.rs")
    ? proof("CONTEXT_DISCLOSURE", "adapter/src/context_disclosure_kernel_proof.rs")
    : gap("CONTEXT_DISCLOSURE"));
  out.push(trackedFiles.includes("adapter/src/model_routing_kernel_proof.rs")
    ? proof("MODEL_ROUTING", "adapter/src/model_routing_kernel_proof.rs")
    : gap("MODEL_ROUTING"));
  out.push(hasMaterializedSymbol(trackedFiles, root, "LearningUseDecision")
    ? present("LEARNING_USE_DECISION_RUNTIME", "production Rust symbol: LearningUseDecision")
    : gap("LEARNING_USE_DECISION_RUNTIME"));
  out.push(hasMaterializedSymbol(trackedFiles, root, "InvocationTrace")
    ? present("REQUEST_MODEL_CONTEXT_TRACE_RUNTIME", "production Rust symbol: InvocationTrace")
    : gap("REQUEST_MODEL_CONTEXT_TRACE_RUNTIME"));
  out.push(hasMaterializedSymbol(trackedFiles, root, "ReplacementProof")
    ? present("MODEL_REPLACEMENT_PROOF_RUNTIME", "production Rust symbol: ReplacementProof")
    : gap("MODEL_REPLACEMENT_PROOF_RUNTIME"));
  out.push(hasMaterializedSymbol(trackedFiles, root, "CapabilityRegressionGate")
    ? present("CAPABILITY_REGRESSION_GATE_RUNTIME", "production Rust symbol: CapabilityRegressionGate")
    : gap("CAPABILITY_REGRESSION_GATE_RUNTIME"));
  out.push(hasMaterializedSymbol(trackedFiles, root, "MergeCompatibility")
    ? present("MODEL_MERGE_COMPATIBILITY_RUNTIME", "production Rust symbol: MergeCompatibility")
    : gap("MODEL_MERGE_COMPATIBILITY_RUNTIME"));
  out.push(hasMaterializedSymbol(trackedFiles, root, "DeletableModelArtifact")
    ? present("WEIGHT_RETIREMENT_DELETION_RUNTIME", "production Rust symbol: DeletableModelArtifact")
    : gap("WEIGHT_RETIREMENT_DELETION_RUNTIME"));
  return out;
}

function provenanceFor(donor, root = REPO_ROOT) {
  const path = resolve(root, "provenance", "donors", `${donor.id}.json`);
  if (!existsSync(path)) return null;
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

function nativeRefs(donor, provenance) {
  const refs = new Set([
    ...(Array.isArray(donor?.absorbed_into) ? donor.absorbed_into : []),
    ...(Array.isArray(provenance?.native_implementation_refs) ? provenance.native_implementation_refs : []),
  ]);
  return [...refs].filter(Boolean).sort();
}

export function buildOrganismModelAtlas({
  checklist = readChecklist(),
  trackedFiles = gitTrackedFiles(),
  root = REPO_ROOT,
  sourceSha = null,
  generatedAt = new Date().toISOString(),
} = {}) {
  const donors = selectOrganismDonors(checklist).map((donor) => {
    const provenance = provenanceFor(donor, root);
    return {
      id: donor.id,
      repo: donor.repo,
      status: donor.status,
      needed: String(donor.needed),
      source_present: donor.source_present === true,
      census_complete: donor.census_complete === true,
      absorption_state: donor.absorption_state ?? "none",
      roles: [...new Set(donor.organism_roles ?? [])].sort(),
      chronica_targets: [...new Set(donor.chronica_targets ?? [])].sort(),
      native_implementation_refs: nativeRefs(donor, provenance),
      provenance_present: provenance !== null,
      temporary_path: donor.temporary_path ?? null,
    };
  }).sort((a, b) => a.id.localeCompare(b.id, undefined, { numeric: true }));

  const roles = [...new Set(donors.flatMap((d) => d.roles))].sort();
  const weightViolations = trackedFiles.filter(isProductionModelArtifactPath).sort();

  const surfaces = {};
  for (const [group, paths] of Object.entries(EXPECTED_ORGANISM_SURFACES)) {
    surfaces[group] = paths.map((path) => ({ path, present: trackedFiles.includes(path) }));
  }

  const evolutionReadiness = buildEvolutionReadiness(trackedFiles, root);
  const evolutionPresent = evolutionReadiness.filter((x) => x.state === "PRESENT").length;
  const evolutionProofOnly = evolutionReadiness.filter((x) => x.state === "PROOF_ONLY").length;
  const evolutionGap = evolutionReadiness.filter((x) => x.state === "GAP").length;

  const sourcePresent = donors.filter((d) => d.source_present).length;
  const censused = donors.filter((d) => d.census_complete).length;
  const absorptionStarted = donors.filter((d) =>
    d.absorption_state !== "none"
    || d.native_implementation_refs.length > 0,
  ).length;

  const violations = weightViolations.map((path, index) => ({
    id: `ORG-MODEL-${String(index + 1).padStart(4, "0")}`,
    type: "PRODUCTION_MODEL_ARTIFACT_TRACKED_IN_GIT",
    severity: "HARD",
    path,
  }));

  return {
    schema: "chronica.system-atlas.organism-models.v1",
    generated_at: generatedAt,
    source_sha: sourceSha,
    source_of_truth: "generated projection over donor checklist, provenance and tracked repository files; never canonical runtime truth",
    summary: {
      donors_total: donors.length,
      source_present_total: sourcePresent,
      not_ingested_total: donors.length - sourcePresent,
      censused_total: censused,
      absorption_started_total: absorptionStarted,
      roles_covered_total: roles.length,
      production_model_artifacts_in_git_total: weightViolations.length,
      organism_semantic_surfaces_present_total: surfaces.organism_semantics.filter((x) => x.present).length,
      runtime_model_lifecycle_surfaces_present_total: surfaces.runtime_model_lifecycle.filter((x) => x.present).length,
      adapter_model_mechanics_surfaces_present_total: surfaces.adapter_model_mechanics.filter((x) => x.present).length,
      evolution_readiness_present_total: evolutionPresent,
      evolution_readiness_proof_only_total: evolutionProofOnly,
      evolution_readiness_gap_total: evolutionGap,
    },
    roles,
    donors,
    surfaces,
    evolution_readiness: evolutionReadiness,
    violations,
  };
}

export function currentOrganismModelAtlas() {
  const sourceSha = execFileSync("git", ["rev-parse", "HEAD"], { cwd: REPO_ROOT, encoding: "utf8" }).trim();
  return buildOrganismModelAtlas({ sourceSha });
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].replaceAll("\\", "/"))) {
  console.log(JSON.stringify(currentOrganismModelAtlas(), null, 2));
}
