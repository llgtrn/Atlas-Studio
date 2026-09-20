#!/usr/bin/env node
// B2 -- Capability graph census. Deterministic: parses every distinct
// repository-supported definition of "capability" and reports them SEPARATELY.
// Do not merge counts across definitions (see docs/_machine/system-atlas README).
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT, atlasGenerationSha, buildWorkspaceGraph, loadCargoMetadata, nowIso, productSourceSha, readJson, repoRel } from "./lib.mjs";

const OUT = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards/capabilities.json");
const CANONICAL_DIR = resolve(REPO_ROOT, "docs/capabilities-canonical/domains");

function findDescriptorFiles(dir, matches = []) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return matches;
  }
  for (const e of entries) {
    const full = join(dir, e.name);
    if (e.isDirectory()) findDescriptorFiles(full, matches);
    else if (e.name.endsWith(".chronica.json")) matches.push(full);
  }
  return matches;
}

function loadDescriptors() {
  const files = findDescriptorFiles(resolve(REPO_ROOT, "crates"));
  const out = [];
  for (const f of files) {
    let d;
    try {
      d = readJson(f);
    } catch {
      continue;
    }
    out.push({ path: repoRel(f), doc: d });
  }
  return out;
}

function loadCanonicalRegistry() {
  let files;
  try {
    files = readdirSync(CANONICAL_DIR).filter((f) => f.endsWith(".jsonl"));
  } catch {
    return [];
  }
  const rows = [];
  for (const f of files) {
    const text = readFileSync(join(CANONICAL_DIR, f), "utf8");
    for (const line of text.split("\n")) {
      if (!line.trim()) continue;
      try {
        rows.push({ file: f, row: JSON.parse(line) });
      } catch {
        // malformed line -- recorded as an anomaly below
        rows.push({ file: f, row: null, malformed: line.slice(0, 120) });
      }
    }
  }
  return rows;
}

function main() {
  const metadata = loadCargoMetadata();
  const { nodes: crateNodes } = buildWorkspaceGraph(metadata);
  const currentCrateNames = new Set(crateNodes.keys());
  const capCrateNames = [...crateNodes.values()].filter((n) => n.layer === "cap").map((n) => n.name);

  // Definition 2: per-crate .chronica.json descriptors (capability.v1 + adapter.v1)
  const descriptors = loadDescriptors();
  const descriptorsBySchema = {};
  const outNodes = [];
  const outEdges = [];
  const anomalies = [];
  const highConfidence = [];

  for (const { path, doc } of descriptors) {
    const schema = doc.schema ?? "UNKNOWN_SCHEMA";
    descriptorsBySchema[schema] = (descriptorsBySchema[schema] ?? 0) + 1;
    const capId = doc.capability?.id;
    const ownerCrate = doc.ownership?.owner_crate;
    if (ownerCrate && !currentCrateNames.has(ownerCrate)) {
      anomalies.push(`DESCRIPTOR_OWNER_CRATE_NOT_IN_WORKSPACE: ${path} claims owner_crate=${ownerCrate}, not a current workspace member`);
    }
    if (!capId) continue;
    const id = `cap:${capId}`;
    const callerEntries = doc.ownership?.caller_entries ?? [];
    outNodes.push({
      id,
      kind: "CAPABILITY",
      label: doc.capability?.id ?? capId,
      existence: "PRESENT",
      maturity: mapMaturity(doc.capability?.maturity),
      reachability: callerEntries.length > 0 ? "WIRED_UNVERIFIED" : "UNKNOWN",
      source_class: "CODE",
      confidence: "MEDIUM",
      evidence: [path],
      notes: doc.purpose?.reason_for_existence?.slice(0, 200),
    });
    if (ownerCrate) {
      outEdges.push({
        from: id,
        to: `crate:${ownerCrate}`,
        type: "IMPLEMENTS_PORT",
        source_class: "CODE",
        evidence: [path],
      });
    }
    for (const caller of callerEntries) {
      outEdges.push({
        from: id,
        to: `runtime:${caller}`,
        type: "MOUNTS",
        source_class: "DOCUMENTATION",
        notes: "self-reported caller_entries field; not independently verified by B2 (see B3/B9)",
        evidence: [path],
      });
    }
  }

  // Definition 3: docs/capabilities-canonical/domains/*.jsonl (the "5,151" registry
  // referenced by the separate correctness-closure campaign -- NOT the same thing
  // as workspace cap crates or .chronica.json descriptors).
  const canonicalRows = loadCanonicalRegistry();
  const statusCounts = {};
  const domainCounts = {};
  const targetCrateStale = new Set();
  const targetCrateValid = new Set();
  let malformedRows = 0;
  for (const { row, malformed } of canonicalRows) {
    if (!row) {
      malformedRows += 1;
      continue;
    }
    statusCounts[row.status] = (statusCounts[row.status] ?? 0) + 1;
    domainCounts[row.domain] = (domainCounts[row.domain] ?? 0) + 1;
    if (row.target_crate) {
      if (currentCrateNames.has(row.target_crate)) targetCrateValid.add(row.target_crate);
      else targetCrateStale.add(row.target_crate);
    }
  }
  if (targetCrateStale.size > 0) {
    anomalies.push(
      `CANONICAL_REGISTRY_STALE_CRATE_REFERENCES: ${targetCrateStale.size} distinct target_crate values in docs/capabilities-canonical/domains/*.jsonl (of ${targetCrateValid.size + targetCrateStale.size} distinct values referenced) are pre-refoundation flat crate names that no longer exist as workspace members (e.g. ${[...targetCrateStale].slice(0, 8).join(", ")}). This registry appears to predate the core/cap/adapter/runtime refoundation and was not re-pointed at the new crate layout.`,
    );
  }
  if (malformedRows > 0) {
    anomalies.push(`CANONICAL_REGISTRY_MALFORMED_ROWS: ${malformedRows} unparseable JSONL line(s) in docs/capabilities-canonical/domains/*.jsonl`);
  }

  highConfidence.push(
    `capability_definition_1_workspace_cap_crates=${capCrateNames.length} (crates/cap/*, physical Cargo packages)`,
    `capability_definition_2_chronica_descriptors=${descriptors.length} (crates/**/.chronica/*.chronica.json)`,
    `capability_definition_2_by_schema=${JSON.stringify(descriptorsBySchema)}`,
    `capability_definition_3_canonical_registry_rows=${canonicalRows.length} (docs/capabilities-canonical/domains/*.jsonl)`,
    `capability_definition_3_status_counts=${JSON.stringify(statusCounts)}`,
    `capability_definition_3_domain_counts=${JSON.stringify(domainCounts)}`,
    `capability_definition_3_distinct_target_crate_values=${targetCrateValid.size + targetCrateStale.size} (${targetCrateValid.size} resolve to current workspace crates, ${targetCrateStale.size} do not)`,
    "these three definitions are NOT the same population and must not be summed or conflated",
  );

  const shard = {
    schema: "chronica.system-atlas.shard.v0",
    lane: "B2_capabilities",
    source_base_sha: productSourceSha(),
    atlas_generation_sha: atlasGenerationSha(),
    generated_at: nowIso(),
    generation_method: "SCRIPT_GENERATED",
    nodes: outNodes,
    edges: outEdges,
    high_confidence_findings: highConfidence,
    low_confidence_findings: [
      "reachability=WIRED_UNVERIFIED on capability nodes reflects only the descriptor's self-reported caller_entries field; B3/B9 must independently confirm actual runtime mounting before any node is upgraded to LIVE",
    ],
    major_anomalies: anomalies,
    repair_candidates: targetCrateStale.size > 0 ? ["CANONICAL_REGISTRY_STALE_CRATE_REFERENCES: re-point or retire docs/capabilities-canonical/domains/*.jsonl target_crate fields against the post-refoundation crate layout"] : [],
    commands_run: [
      "cargo metadata --format-version 1 --no-deps",
      "node tools/system-atlas/capability-census.mjs",
    ],
    ready_to_integrate: true,
  };

  writeFileSync(OUT, `${JSON.stringify(shard, null, 2)}\n`);
  console.log(`wrote ${outNodes.length} capability nodes, ${outEdges.length} edges to ${repoRel(OUT)}`);
  for (const f of highConfidence) console.log(f);
  for (const a of anomalies) console.log(`ANOMALY: ${a}`);
}

function mapMaturity(raw) {
  switch (raw) {
    case "RUNTIME_BOUND":
    case "PARTIAL_RUNTIME_BOUND":
    case "PERSISTENCE_BOUND":
    case "CI_VERIFIED":
    case "UI_BOUND":
      return "IMPLEMENTED";
    case "PORTED":
      return "IMPLEMENTED";
    default:
      return "UNKNOWN";
  }
}

// Guarded so this module can be imported (e.g. by tests) without the
// side effect of regenerating and overwriting capabilities.json.
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
