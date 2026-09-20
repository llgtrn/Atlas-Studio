import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { parse as parseYaml } from "yaml";
import { REPO_ROOT } from "./lib.mjs";

const REGISTRY_PATH = resolve(REPO_ROOT, "tools/system-atlas/connector-protocols.yaml");
const DONOR_PATH = resolve(REPO_ROOT, "tools/refoundation/donor-corpus.yaml");

function readText(root, path) {
  try {
    return readFileSync(resolve(root, path), "utf8");
  } catch {
    return "";
  }
}

function gitTrackedFiles(root = REPO_ROOT) {
  return execFileSync("git", ["ls-files"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  }).split("\n").map((x) => x.trim()).filter(Boolean);
}

function loadYaml(path) {
  return parseYaml(readFileSync(path, "utf8"));
}

function protocolStatus(entry, donorsById, trackedFiles) {
  const productionEvidence = (entry.production_evidence_paths ?? [])
    .filter((path) => trackedFiles.includes(path));
  const proofEvidence = (entry.proof_evidence_paths ?? [])
    .filter((path) => trackedFiles.includes(path));
  const donorRefs = (entry.donor_ids ?? []).map((id) => donorsById.get(id)).filter(Boolean);
  const sourceDonors = donorRefs.filter((donor) => donor.source_present === true);

  let status = "DISCOVERED";
  if (sourceDonors.length > 0) status = "SOURCE_PRESENT";
  if (proofEvidence.length > 0) status = "PROOF_ONLY";
  if (productionEvidence.length > 0) status = "PRODUCTION";

  return {
    id: entry.id,
    category: entry.category,
    status,
    donor_ids: [...(entry.donor_ids ?? [])],
    source_present_donor_ids: sourceDonors.map((donor) => donor.id),
    proof_evidence_paths: proofEvidence,
    production_evidence_paths: productionEvidence,
  };
}

function runtimeState(id, state, evidence = []) {
  return { id, state, evidence };
}

function buildRuntimeReadiness(trackedFiles, root, physicalProductionTotal) {
  const edge = readText(root, "runtime/src/edge.rs");
  const safety = readText(root, "runtime/src/safety.rs");
  const robot = readText(root, "adapter/src/robot_command_effect.rs");

  return [
    trackedFiles.includes("runtime/src/edge.rs")
      ? runtimeState(
          "EDGE_RUNTIME",
          edge.includes("CONTRACT_ONLY") ? "CONTRACT_ONLY" : "PRESENT",
          ["runtime/src/edge.rs"],
        )
      : runtimeState("EDGE_RUNTIME", "GAP"),
    trackedFiles.includes("runtime/src/safety.rs")
      ? runtimeState(
          "SAFETY_RUNTIME",
          safety.includes("PARTIAL_RUNTIME") ? "PARTIAL_RUNTIME" : "PRESENT",
          ["runtime/src/safety.rs"],
        )
      : runtimeState("SAFETY_RUNTIME", "GAP"),
    trackedFiles.includes("runtime/src/operational_observation.rs")
      ? runtimeState("OPERATIONAL_OBSERVATION_RUNTIME", "PRESENT", ["runtime/src/operational_observation.rs"])
      : runtimeState("OPERATIONAL_OBSERVATION_RUNTIME", "GAP"),
    trackedFiles.includes("runtime/src/binding.rs")
      ? runtimeState("BINDING_RUNTIME", "PRESENT", ["runtime/src/binding.rs"])
      : runtimeState("BINDING_RUNTIME", "GAP"),
    trackedFiles.includes("runtime/src/capability.rs")
      ? runtimeState("CAPABILITY_RUNTIME", "PRESENT", ["runtime/src/capability.rs"])
      : runtimeState("CAPABILITY_RUNTIME", "GAP"),
    trackedFiles.includes("runtime/src/reconciliation.rs")
      ? runtimeState("RECONCILIATION_RUNTIME", "PRESENT", ["runtime/src/reconciliation.rs"])
      : runtimeState("RECONCILIATION_RUNTIME", "GAP"),
    trackedFiles.includes("adapter/src/robot_command_effect.rs")
      ? runtimeState(
          "GENERIC_ROBOT_EFFECT",
          robot.includes("SIMULATED") || robot.includes("SimulatedRobotAdapter")
            ? "SIMULATED_ONLY"
            : "PRESENT",
          ["adapter/src/robot_command_effect.rs"],
        )
      : runtimeState("GENERIC_ROBOT_EFFECT", "GAP"),
    trackedFiles.includes("adapter/src/durable_safety_robot_kernel_proof.rs")
      ? runtimeState("DURABLE_ROBOT_SAFETY_PROOF", "PROOF_ONLY", ["adapter/src/durable_safety_robot_kernel_proof.rs"])
      : runtimeState("DURABLE_ROBOT_SAFETY_PROOF", "GAP"),
    physicalProductionTotal > 0
      ? runtimeState("REAL_PHYSICAL_PROTOCOL_ADAPTER", "PRESENT", [])
      : runtimeState("REAL_PHYSICAL_PROTOCOL_ADAPTER", "GAP"),
    /\bThingManifest\b/.test(
      trackedFiles
        .filter((p) => (p.startsWith("core/src/") || p.startsWith("runtime/src/") || p.startsWith("adapter/src/")) && p.endsWith(".rs"))
        .map((p) => readText(root, p))
        .join("\n"),
    )
      ? runtimeState("THING_MANIFEST_RUNTIME", "PRESENT", ["production Rust symbol: ThingManifest"])
      : runtimeState("THING_MANIFEST_RUNTIME", "GAP"),
  ];
}

function cargoDependencyNames(root, path) {
  const source = readText(root, path);
  const lines = source.split("\n");
  const names = [];
  let inDependencies = false;
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

const CORE_PROTOCOL_DEPENDENCY_TOKENS = [
  "opcua", "opc-ua", "modbus", "ros2", "rclrs", "mqtt", "bacnet", "knx",
  "ethercat", "socketcan", "canopen", "j1939", "matter", "connectedhomeip", "plc4x",
];

function coreProtocolDependencyViolations(root) {
  return cargoDependencyNames(root, "core/Cargo.toml")
    .filter((name) => CORE_PROTOCOL_DEPENDENCY_TOKENS.some((token) => name.toLowerCase().includes(token)))
    .map((dependency) => ({
      type: "PROTOCOL_DEPENDENCY_LEAKED_INTO_CORE",
      severity: "HARD",
      path: "core/Cargo.toml",
      detail: dependency,
    }));
}

const RUNTIME_PROTOCOL_PATH_TOKENS = [
  "opcua", "modbus", "ros2", "mqtt", "bacnet", "knx", "ethercat",
  "socketcan", "canopen", "j1939", "obd", "matter", "s7", "ethernet_ip",
];

function runtimeProtocolPathReviews(trackedFiles) {
  return trackedFiles
    .filter((path) => path.startsWith("runtime/src/"))
    .filter((path) => RUNTIME_PROTOCOL_PATH_TOKENS.some((token) => path.toLowerCase().includes(token)))
    .map((path) => ({
      type: "PROTOCOL_SPECIFIC_RUNTIME_PATH_REVIEW",
      severity: "REVIEW_REQUIRED",
      path,
      detail: "protocol-specific path should normally remain adapter-local",
    }));
}

export function buildConnectorAtlas({
  registry = loadYaml(REGISTRY_PATH),
  donorCorpus = loadYaml(DONOR_PATH),
  trackedFiles = gitTrackedFiles(),
  root = REPO_ROOT,
  sourceSha = null,
  generatedAt = new Date().toISOString(),
} = {}) {
  const donorsById = new Map((donorCorpus?.donors ?? []).map((donor) => [donor.id, donor]));
  const protocols = (registry?.protocols ?? [])
    .map((entry) => protocolStatus(entry, donorsById, trackedFiles))
    .sort((a, b) => a.id.localeCompare(b.id));

  const counts = {
    DISCOVERED: protocols.filter((x) => x.status === "DISCOVERED").length,
    SOURCE_PRESENT: protocols.filter((x) => x.status === "SOURCE_PRESENT").length,
    PROOF_ONLY: protocols.filter((x) => x.status === "PROOF_ONLY").length,
    PRODUCTION: protocols.filter((x) => x.status === "PRODUCTION").length,
  };

  const physicalProtocols = protocols.filter((x) => x.category !== "digital");
  const physicalProductionTotal = physicalProtocols.filter((x) => x.status === "PRODUCTION").length;
  const runtimeReadiness = buildRuntimeReadiness(trackedFiles, root, physicalProductionTotal);

  const rawViolations = [
    ...coreProtocolDependencyViolations(root),
    ...runtimeProtocolPathReviews(trackedFiles),
  ];
  const violations = rawViolations.map((row, index) => ({
    id: "CONNECTOR-" + String(index + 1).padStart(4, "0"),
    ...row,
  }));

  return {
    schema: "chronica.system-atlas.connector-matrix.v1",
    generated_at: generatedAt,
    source_sha: sourceSha,
    source_of_truth: "generated projection over connector target registry, donor corpus and tracked runtime/adapter evidence; never canonical machine truth",
    summary: {
      protocol_total: protocols.length,
      discovered_total: counts.DISCOVERED,
      source_present_total: counts.SOURCE_PRESENT,
      proof_only_total: counts.PROOF_ONLY,
      production_total: counts.PRODUCTION,
      physical_protocol_total: physicalProtocols.length,
      physical_production_total: physicalProductionTotal,
      runtime_present_total: runtimeReadiness.filter((x) => x.state === "PRESENT").length,
      runtime_partial_total: runtimeReadiness.filter((x) => x.state === "PARTIAL_RUNTIME").length,
      runtime_contract_only_total: runtimeReadiness.filter((x) => x.state === "CONTRACT_ONLY").length,
      runtime_proof_only_total: runtimeReadiness.filter((x) => x.state === "PROOF_ONLY").length,
      runtime_simulated_only_total: runtimeReadiness.filter((x) => x.state === "SIMULATED_ONLY").length,
      runtime_gap_total: runtimeReadiness.filter((x) => x.state === "GAP").length,
      hard_violations_total: violations.filter((x) => x.severity === "HARD").length,
      review_required_total: violations.filter((x) => x.severity === "REVIEW_REQUIRED").length,
    },
    protocols,
    runtime_readiness: runtimeReadiness,
    violations,
  };
}

export function currentConnectorAtlas() {
  const sourceSha = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  }).trim();
  return buildConnectorAtlas({ sourceSha });
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].replaceAll("\\", "/"))) {
  console.log(JSON.stringify(currentConnectorAtlas(), null, 2));
}
