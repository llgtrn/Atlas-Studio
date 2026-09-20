#!/usr/bin/env node
// B7 (static portion) -- clusters the known-reverse-edge debt baseline
// (tools/refoundation/refoundation-layer-debt.json) into root-cause families instead of
// treating all edges as unrelated tasks. Deterministic pattern classification;
// anything that doesn't match a known pattern is UNKNOWN, not force-fit.
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT, atlasGenerationSha, buildWorkspaceGraph, loadCargoMetadata, nowIso, productSourceSha, readJson, repoRel } from "./lib.mjs";

const OUT = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards/legacy.json");
const DEBT_BASELINE = resolve(REPO_ROOT, "tools/refoundation/refoundation-layer-debt.json");

const EDGE_RE = /^(\S+) \((\w+)\) -> (\S+) \((\w+)\)$/;

export function classify(from, fromLayer, to, toLayer) {
  if (/^chronica-core-cli-/.test(from)) return "CLI_MISPLACED_IN_CORE";
  if (from === "chronica-core-api-state") return "API_STATE_COMPOSITION_MISPLACED_IN_CORE";
  if (fromLayer === "core" && /^chronica-cap-policy-/.test(to)) return "CORE_TO_POLICY_CAP_INVERSION";
  if (fromLayer === "core" && /^chronica-cap-simulation-/.test(to)) return "CORE_TO_SIMULATION_CAP_INVERSION";
  if (fromLayer === "core" && to === "chronica-cap-runtime-money-gateway") return "CORE_TO_MONEY_GATEWAY_CAP_INVERSION";
  if (fromLayer === "core" && /^chronica-core-osint-/.test(from) && toLayer === "adapter") return "OSINT_ADAPTER_MISPLACEMENT";
  if (fromLayer === "cap" && toLayer === "adapter") return "CAP_TO_ADAPTER_PORT_INVERSION";
  if (fromLayer === "cap" && toLayer === "runtime") return "RUNTIME_OWNERSHIP_INVERSION";
  if (fromLayer === "core" && toLayer === "adapter") return "PROTOCOL_IMPLEMENTATION_MISPLACEMENT";
  if (fromLayer === "core" && toLayer === "cap") return "CORE_TO_DOMAIN_CAP_STORAGE_INVERSION";
  return "UNKNOWN";
}

const ROOT_CAUSE_NOTES = {
  CLI_MISPLACED_IN_CORE:
    "A crate named chronica-core-cli-* lives under crates/core but implements CLI command surfaces, so it legitimately needs to reach cap/adapter to do its job. The crate itself is misclassified as core -- it is a runtime/entrypoint composition root, not domain logic.",
  API_STATE_COMPOSITION_MISPLACED_IN_CORE:
    "chronica-core-api-state composes the whole HTTP AppState (gateway, oauth, policy, projections, money gateway, sandbox) but lives in crates/core. It is an adapter/runtime composition root misclassified as core.",
  CORE_TO_POLICY_CAP_INVERSION:
    "Domain core crates (money/ERP/strategy/company) call directly into chronica-cap-policy-approval-gate / chronica-cap-policy-posture-evidence for their own authority gating. Policy authority is a cap-layer capability that core depends on -- an inversion of the intended core-owns-primitives direction.",
  CORE_TO_SIMULATION_CAP_INVERSION:
    "Domain core crates (agent cognition, strategy, evidence, replication) call into chronica-cap-simulation-scenario-engine / chronica-cap-simulation-signal-bridge for scenario/simulation support that core logic depends on.",
  CORE_TO_MONEY_GATEWAY_CAP_INVERSION:
    "Many core engine/workflow crates depend directly on chronica-cap-runtime-money-gateway. Money-moving authority lives in the cap layer while core domain logic reaches into it directly -- high-risk (T3) inversion given moves_money semantics.",
  OSINT_ADAPTER_MISPLACEMENT:
    "chronica-core-osint-* crates reach directly into chronica-adapter-osint-* connector crates, skipping the cap layer that should mediate external OSINT integrations.",
  CAP_TO_ADAPTER_PORT_INVERSION:
    "cap crates depend directly on adapter crates instead of adapters depending on cap ports. This is the largest single family and spans commerce, CRM/support, workforce, and platform domains.",
  RUNTIME_OWNERSHIP_INVERSION:
    "cap crates depend on runtime process-composition crates (chronica-runtime-event-dispatch, chronica-runtime-social-retention-gc), inverting the intended runtime-composes-cap direction.",
  PROTOCOL_IMPLEMENTATION_MISPLACEMENT:
    "A core crate (not cli-prefixed, not osint-prefixed) depends directly on an adapter crate.",
  CORE_TO_DOMAIN_CAP_STORAGE_INVERSION:
    "A core domain-logic crate depends directly on its own domain's cap-layer storage/state crate (session store, notifications, template store, identity credentials, etc.) rather than through a core-owned port.",
  UNKNOWN: "Edge did not match any known classification rule; needs manual root-cause investigation before repair dispatch.",
};

function main() {
  const baseline = readJson(DEBT_BASELINE);
  const metadata = loadCargoMetadata();
  const { nodes } = buildWorkspaceGraph(metadata);

  const families = new Map();
  const unclassified = [];

  for (const edgeStr of baseline.forbidden_edges) {
    const m = EDGE_RE.exec(edgeStr);
    if (!m) {
      unclassified.push(edgeStr);
      continue;
    }
    const [, from, fromLayer, to, toLayer] = m;
    const family = classify(from, fromLayer, to, toLayer);
    if (!families.has(family)) families.set(family, []);
    families.get(family).push({ from, to, fromLayer, toLayer, raw: edgeStr });
  }

  const debtFamilies = [...families.entries()]
    .sort((a, b) => b[1].length - a[1].length)
    .map(([family_id, edges]) => ({
      family_id,
      root_cause: ROOT_CAUSE_NOTES[family_id] ?? "unclassified",
      edge_count: edges.length,
      example_edges: edges.slice(0, 6).map((e) => e.raw),
      distinct_source_crates: [...new Set(edges.map((e) => e.from))].length,
      distinct_target_crates: [...new Set(edges.map((e) => e.to))].length,
    }));

  const totalClassified = debtFamilies.reduce((s, f) => s + f.edge_count, 0);

  const outEdges = [];
  for (const [family, edges] of families) {
    for (const e of edges) {
      outEdges.push({
        from: `crate:${e.from}`,
        to: `crate:${e.to}`,
        type: "DEPENDS_ON",
        forbidden: true,
        source_class: "BUILD_METADATA",
        evidence: ["tools/refoundation/refoundation-layer-debt.json"],
        notes: `debt_family=${family}`,
      });
    }
  }

  const anomalies = [];
  if (unclassified.length > 0) {
    anomalies.push(`${unclassified.length} baseline edge(s) did not parse against the expected "name (layer) -> name (layer)" format: ${JSON.stringify(unclassified)}`);
  }
  if (totalClassified !== baseline.forbidden_edges.length) {
    anomalies.push(`classified ${totalClassified} of ${baseline.forbidden_edges.length} baseline edges (${baseline.forbidden_edges.length - totalClassified} unaccounted for)`);
  }

  const shard = {
    schema: "chronica.system-atlas.shard.v0",
    lane: "B7_legacy",
    source_base_sha: productSourceSha(),
    atlas_generation_sha: atlasGenerationSha(),
    generated_at: nowIso(),
    generation_method: "SCRIPT_GENERATED",
    nodes: [],
    edges: outEdges,
    debt_families: debtFamilies,
    high_confidence_findings: [
      `known_reverse_edge_count=${baseline.forbidden_edges.length}`,
      `debt_family_count=${debtFamilies.length}`,
      `largest_family=${debtFamilies[0]?.family_id} (${debtFamilies[0]?.edge_count} edges)`,
    ],
    major_anomalies: anomalies,
    repair_candidates: debtFamilies.map((f) => `${f.family_id} (${f.edge_count} edges, ${f.distinct_source_crates} source crates)`),
    commands_run: ["node tools/refoundation/validate-refoundation-layers.mjs", "node tools/system-atlas/legacy-debt-census.mjs"],
    ready_to_integrate: true,
  };

  writeFileSync(OUT, `${JSON.stringify(shard, null, 2)}\n`);
  console.log(`classified ${totalClassified}/${baseline.forbidden_edges.length} debt edges into ${debtFamilies.length} families`);
  for (const f of debtFamilies) console.log(`  ${f.family_id}: ${f.edge_count} edges (${f.distinct_source_crates} source crates -> ${f.distinct_target_crates} target crates)`);
  if (anomalies.length) console.log("ANOMALIES:", anomalies);
  console.log(`wrote ${repoRel(OUT)}`);
}

// Guarded so this module can be imported (e.g. by tests, to reuse classify())
// without the side effect of regenerating and overwriting legacy.json -- this
// bit the Atlas once already: importing classify() from a test module
// silently re-ran main() and wiped the B7 supplement's hand-merged nodes.
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
