#!/usr/bin/env node
// B1 — Physical topology census. Deterministic: cargo metadata + filesystem only.
// Writes docs/_machine/system-atlas/v0/shards/topology.json (owned by coordinator on
// integration; script itself may be re-run by any lane to regenerate the same facts).
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT, atlasGenerationSha, buildWorkspaceGraph, fanInOut, loadCargoMetadata, nowIso, productSourceSha, repoRel } from "./lib.mjs";
import { tarjanScc } from "./scc.mjs";

const OUT = resolve(REPO_ROOT, "docs/_machine/system-atlas/v0/shards/topology.json");

// core/runtime/adapter/organism are the four post-refoundation top-level layers
// (lib.mjs's crateLayer()); "cap" was the pre-refoundation crates/cap/ layer, retired
// 2026-09-16 (docs/decisions/0016-legacy-backend-retired-to-git-history.md) and no longer
// reachable from crateLayer(), so it is not carried forward here as dead lookup data.
const KIND_BY_LAYER = {
  core: "CORE_COMPONENT",
  adapter: "ADAPTER",
  runtime: "RUNTIME",
  organism: "ORGANISM",
};

// Dependency-direction rules the North-Star loop directive fixes for the four top-level roots:
// core has no runtime/adapter/organism deps; runtime never adapter/organism; organism never
// adapter. ORGANISM -> ADAPTER is the mechanically-checkable form of "ORGANISM HAS NO DIRECT
// EFFECT PATH" (organism must reach any provider only through runtime's governed execution
// spine, never by depending on adapter/ directly).
//
// 2026-09-17 dependency-inversion correction: `adapter` has no entry here. `organism::WorldModel`
// is a provider-neutral ANALYZE port that `adapter` implements with concrete model-family/
// provider mechanics (docs/architecture/intelligence/world-model.md section 10), so
// `adapter -> organism` is the INTENDED direction now, not a forbidden one -- mirroring
// tools/refoundation/validate-refoundation-layers.mjs's identical correction. The critical
// invariant (`organism -> adapter` forbidden) is unchanged.
const FORBIDDEN_LAYER_TARGETS = {
  core: new Set(["runtime", "adapter", "organism"]),
  runtime: new Set(["adapter", "organism"]),
  organism: new Set(["adapter"]),
};

function main() {
  const metadata = loadCargoMetadata();
  const { nodes, edges } = buildWorkspaceGraph(metadata);
  const nodeNames = [...nodes.keys()];
  const { fanin, fanout } = fanInOut(nodeNames, edges);
  const sccs = tarjanScc(nodeNames, edges);
  const sccIdByNode = new Map();
  const nonTrivialSccs = [];
  sccs.forEach((component, i) => {
    const id = `scc:${i}`;
    for (const n of component) sccIdByNode.set(n, id);
    if (component.length > 1) nonTrivialSccs.push({ id, members: component.sort() });
  });

  const byLayer = { core: 0, runtime: 0, adapter: 0, organism: 0, other: 0 };
  const descriptorCoverage = { core: 0, adapter: 0, runtime: 0, organism: 0 };
  const targetCounts = { bin: 0, lib: 0, example: 0, test: 0, buildScript: 0 };
  const anomalies = [];

  const outNodes = [];
  for (const name of nodeNames.sort()) {
    const n = nodes.get(name);
    const layer = n.layer ?? "other";
    byLayer[layer] = (byLayer[layer] ?? 0) + 1;
    if (n.hasChronicaDescriptor && layer in descriptorCoverage) descriptorCoverage[layer] += 1;
    if (n.hasBin) targetCounts.bin += 1;
    if (n.hasLib) targetCounts.lib += 1;
    if (n.hasExample) targetCounts.example += 1;
    if (n.hasTest) targetCounts.test += 1;
    if (n.hasBuildScript) targetCounts.buildScript += 1;

    if (layer === "core" && n.hasBin) {
      anomalies.push(`CLI_MISPLACED_IN_CORE: ${name} (${n.manifestPath}) is a core crate with a [[bin]] target`);
    }
    if (layer === "core" && /-cli-/.test(name)) {
      anomalies.push(`CLI_NAMING_IN_CORE: ${name} carries -cli- naming while classified as core`);
    }
    if (layer === "core" && /(api-state|api-server|router)/.test(name)) {
      anomalies.push(`API_STATE_NAMING_IN_CORE: ${name} carries API/router naming while classified as core`);
    }

    outNodes.push({
      id: `crate:${name}`,
      kind: KIND_BY_LAYER[layer] ?? "CORE_COMPONENT",
      label: name,
      layer: layer in KIND_BY_LAYER ? layer : "not_applicable",
      existence: "PRESENT",
      source_class: "BUILD_METADATA",
      evidence: [n.manifestPath],
      fanin: fanin.get(name) ?? 0,
      fanout: fanout.get(name) ?? 0,
      scc_id: nonTrivialSccs.find((s) => s.id === sccIdByNode.get(name)) ? sccIdByNode.get(name) : null,
    });
  }

  const outEdges = edges.map(({ from, to }) => {
    const fromLayer = nodes.get(from)?.layer;
    const toLayer = nodes.get(to)?.layer;
    const forbidden = FORBIDDEN_LAYER_TARGETS[fromLayer]?.has(toLayer) ?? false;
    return {
      from: `crate:${from}`,
      to: `crate:${to}`,
      type: "DEPENDS_ON",
      forbidden,
      source_class: "BUILD_METADATA",
      evidence: ["cargo metadata --format-version 1 --no-deps"],
    };
  });

  const topFanout = [...fanout.entries()].sort((a, b) => b[1] - a[1]).slice(0, 15);
  const topFanin = [...fanin.entries()].sort((a, b) => b[1] - a[1]).slice(0, 15);

  const shard = {
    schema: "chronica.system-atlas.shard.v0",
    lane: "B1_topology",
    source_base_sha: productSourceSha(),
    atlas_generation_sha: atlasGenerationSha(),
    generated_at: nowIso(),
    generation_method: "SCRIPT_GENERATED",
    nodes: outNodes,
    edges: outEdges,
    high_confidence_findings: [
      `workspace_package_count=${nodeNames.length}`,
      `layer_counts=${JSON.stringify(byLayer)}`,
      `descriptor_coverage(.chronica/*.chronica.json present)=${JSON.stringify(descriptorCoverage)}`,
      `target_counts=${JSON.stringify(targetCounts)}`,
      `scc_count_total=${sccs.length}`,
      `scc_count_non_trivial=${nonTrivialSccs.length}`,
      `top_fanout=${JSON.stringify(topFanout)}`,
      `top_fanin=${JSON.stringify(topFanin)}`,
    ],
    major_anomalies: anomalies,
    non_trivial_sccs: nonTrivialSccs,
    commands_run: ["cargo metadata --format-version 1 --no-deps", `node ${repoRel(import.meta.url.replace("file://", ""))}`],
    ready_to_integrate: true,
  };

  writeFileSync(OUT, `${JSON.stringify(shard, null, 2)}\n`);
  console.log(`wrote ${outNodes.length} nodes, ${outEdges.length} edges to ${repoRel(OUT)}`);
  console.log(`layer_counts=${JSON.stringify(byLayer)}`);
  console.log(`scc_total=${sccs.length} non_trivial=${nonTrivialSccs.length}`);
  console.log(`anomalies=${anomalies.length}`);
}

// Guarded so this module can be imported (e.g. by tests) without the
// side effect of regenerating and overwriting topology.json.
if (process.argv[1] === fileURLToPath(import.meta.url)) main();
