#!/usr/bin/env node
// System Atlas v2 generator.
//
// Answers only: WHAT ACTUALLY EXISTS IN THE REPOSITORY, and HOW IS IT
// CONNECTED? Never an architecture authority -- AGENTS.md/NORTH-STAR.md/
// system-model.md/docs/architecture/* stay authoritative; this only measures
// reality against those rules. Reuses tools/system-atlas/lib.mjs's
// buildWorkspaceGraph() (Cargo side, already fixed for the core/runtime/
// adapter/organism top-level layout) and tools/refoundation/donor-corpus.yaml
// + donor-burndown.mjs (donor evidence) rather than re-deriving either.
//
// Writes docs/_machine/system-atlas/v2/generated/*.json (gitignored,
// regenerate-on-demand -- see docs/_machine/ forbidden-documentation-root
// rule in tools/docs/architecture-registry.mjs). Run: `pnpm atlas:generate`.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";
import { REPO_ROOT, repoRel, loadCargoMetadata, buildWorkspaceGraph } from "./lib.mjs";
import { compileSchemas, validate, formatErrors } from "./schema-validate.mjs";
import { loadChecklist } from "../refoundation/donor-corpus.mjs";
import { allocateDonors, loadFleetRegistry } from "./fleet/donor-allocator.mjs";
import { currentOrganismModelAtlas } from "./organism-model-census.mjs";
import { currentIntelligenceBoundaryAtlas } from "./intelligence-boundary-census.mjs";
import { currentConnectorAtlas } from "./connector-census.mjs";

const SCHEMA_DIR = resolve(REPO_ROOT, "tools/system-atlas/schema");
const OUT_DIR = resolve(REPO_ROOT, "docs/_machine/system-atlas/v2/generated");

// Top-level directory -> responsibility, per AGENTS.md section 4 ("Repository
// responsibility"). `organism` is a first-class responsibility in the v2 enum (CORE, RUNTIME,
// ADAPTER, ORGANISM, APP, GRAPH, BINDING, OPS, TOOLING, PROVENANCE, LEGACY_CAP,
// TEMPORARY_DONOR, UNKNOWN): Digital Organism (ANALYZE: observe/predict/imagine/plan/simulate/
// evaluate/learn/propose) is architecturally distinct from APP (projections/UI) -- see the
// North-Star loop directive's Track B and docs/architecture/organism/organism.md. It was
// previously folded into APP as the closest existing bucket before this responsibility existed;
// that mapping undercounted organism/ and made its forbidden-edge rules (organism -X-> adapter)
// unrepresentable, so it is corrected here rather than left as a known-wrong disclosed mapping.
const DIR_RESPONSIBILITY = [
  ["core/", "CORE"],
  ["runtime/", "RUNTIME"],
  ["adapter/", "ADAPTER"],
  ["organism/", "ORGANISM"],
  ["apps/", "APP"],
  ["graph/", "GRAPH"],
  ["bindings/", "BINDING"],
  ["deploy/", "OPS"],
  ["tools/", "TOOLING"],
  ["license/", "PROVENANCE"],
  ["temporary/", "TEMPORARY_DONOR"],
];

export function responsibilityForPath(path) {
  for (const [prefix, responsibility] of DIR_RESPONSIBILITY) {
    if (path === prefix.slice(0, -1) || path.startsWith(prefix)) return responsibility;
  }
  return "UNKNOWN";
}

// Direction rules mirroring tools/refoundation/validate-refoundation-layers.mjs's
// FORBIDDEN_TARGETS intent (adapter -> core -> ... ordering), restated over the coarser v2
// responsibility enum (CORE/RUNTIME/ADAPTER/ORGANISM/APP/...) used elsewhere in this generator,
// rather than reusing that file's four-layer crateLayer()/FORBIDDEN_TARGETS directly.
//
// ORGANISM's rules encode the North-Star loop directive's dependency-direction contract:
// "core: no runtime/adapter/organism deps; runtime: never adapter/organism; organism: never
// adapter" -- most importantly ORGANISM -> ADAPTER, which is the mechanically-checkable form of
// "ORGANISM HAS NO DIRECT EFFECT PATH" (organism must reach any provider only through runtime's
// governed execution spine, never by depending on adapter/ directly).
//
// 2026-09-17 dependency-inversion correction: ADAPTER has no entry in either map below at all.
// `organism::WorldModel` (docs/architecture/intelligence/world-model.md) is a provider-neutral
// ANALYZE port; that doc's own runtime-mapping table (section 10) assigns the concrete
// model-family/provider mechanics implementing it to `adapter/`. ADAPTER -> ORGANISM is
// therefore the INTENDED direction (an outer provider implementation depending on an inner
// semantic port -- ports-and-adapters/dependency inversion), not a forbidden one, mirroring
// tools/refoundation/validate-refoundation-layers.mjs's own identical correction. The critical
// invariant is unchanged and still absolute: ORGANISM -> ADAPTER remains HARD forbidden below.
const FORBIDDEN_TARGETS = {
  CORE: new Set(["ADAPTER", "APP", "RUNTIME", "ORGANISM"]),
  RUNTIME: new Set(["APP", "ORGANISM"]),
  ORGANISM: new Set(["ADAPTER"]),
};
const VIOLATION_TYPE = {
  CORE: { ADAPTER: "CORE_TO_ADAPTER", APP: "CORE_TO_APP", RUNTIME: "CORE_TO_RUNTIME", ORGANISM: "CORE_TO_ORGANISM" },
  RUNTIME: { APP: "RUNTIME_TO_APP", ORGANISM: "RUNTIME_TO_ORGANISM" },
  ORGANISM: { ADAPTER: "ORGANISM_TO_ADAPTER" },
};

function rustComponents(metadata) {
  const { nodes } = buildWorkspaceGraph(metadata);
  const components = [];
  for (const node of nodes.values()) {
    const path = repoRel(dirname(resolve(REPO_ROOT, node.manifestPath)));
    components.push({
      id: path,
      path,
      kind: "rust_crate",
      responsibility: responsibilityForPath(`${path}/`),
      evidence: [node.manifestPath],
    });
  }
  return components;
}

function rustDependencies(metadata, componentById) {
  const { edges } = buildWorkspaceGraph(metadata);
  const { nodes } = buildWorkspaceGraph(metadata);
  const nameToPath = new Map();
  for (const node of nodes.values()) nameToPath.set(node.name, repoRel(dirname(resolve(REPO_ROOT, node.manifestPath))));
  const deps = [];
  for (const edge of edges) {
    const sourceId = nameToPath.get(edge.from);
    const targetId = nameToPath.get(edge.to);
    if (!sourceId || !targetId || sourceId === targetId) continue;
    const source = componentById.get(sourceId);
    const target = componentById.get(targetId);
    deps.push({
      source: sourceId,
      target: targetId,
      dependency_kind: "cargo",
      source_responsibility: source.responsibility,
      target_responsibility: target.responsibility,
      evidence: [source.evidence[0]],
    });
  }
  return deps;
}

function expandWorkspaceGlob(pattern) {
  if (!pattern.endsWith("/*")) return existsSync(resolve(REPO_ROOT, pattern, "package.json")) ? [pattern] : [];
  const base = pattern.slice(0, -2);
  const baseDir = resolve(REPO_ROOT, base);
  if (!existsSync(baseDir)) return [];
  return readdirSync(baseDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => `${base}/${entry.name}`)
    .filter((path) => existsSync(resolve(REPO_ROOT, path, "package.json")));
}

function tsComponentsAndDependencies() {
  const workspaceFile = resolve(REPO_ROOT, "pnpm-workspace.yaml");
  const workspace = parseYaml(readFileSync(workspaceFile, "utf8"));
  const packagePaths = (workspace.packages ?? []).flatMap(expandWorkspaceGlob);

  const components = [];
  const byName = new Map();
  for (const path of packagePaths) {
    const pkgJsonPath = join(path, "package.json");
    const pkg = JSON.parse(readFileSync(resolve(REPO_ROOT, pkgJsonPath), "utf8"));
    const component = {
      id: path,
      path,
      kind: "ts_package",
      responsibility: responsibilityForPath(`${path}/`),
      evidence: [pkgJsonPath],
    };
    components.push(component);
    byName.set(pkg.name, { path, pkg });
  }

  const deps = [];
  for (const { path, pkg } of byName.values()) {
    const allDeps = { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };
    for (const depName of Object.keys(allDeps)) {
      const target = byName.get(depName);
      if (!target || target.path === path) continue;
      const source = components.find((c) => c.id === path);
      const targetComponent = components.find((c) => c.id === target.path);
      deps.push({
        source: path,
        target: target.path,
        dependency_kind: "ts_workspace",
        source_responsibility: source.responsibility,
        target_responsibility: targetComponent.responsibility,
        evidence: [join(path, "package.json")],
      });
    }
  }
  return { components, deps };
}

export function computeViolations(components, dependencies) {
  const violations = [];
  let seq = 0;
  for (const dep of dependencies) {
    const forbidden = FORBIDDEN_TARGETS[dep.source_responsibility];
    if (!forbidden || !forbidden.has(dep.target_responsibility)) continue;
    const type = VIOLATION_TYPE[dep.source_responsibility]?.[dep.target_responsibility];
    if (!type) continue;
    seq += 1;
    violations.push({
      id: `V${String(seq).padStart(4, "0")}`,
      type,
      severity: "HARD",
      status: "PROVEN",
      source: dep.source,
      target: dep.target,
      evidence: dep.evidence,
    });
  }
  for (const component of components) {
    if (component.responsibility !== "LEGACY_CAP") continue;
    seq += 1;
    violations.push({
      id: `V${String(seq).padStart(4, "0")}`,
      type: "LEGACY_CAP_DEPENDENCY",
      severity: "HARD",
      status: "PROVEN",
      source: component.id,
      target: null,
      evidence: component.evidence,
    });
  }
  return violations;
}

function markdownFilesUnder(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const absolute = resolve(dir, entry.name);
    if (entry.isDirectory()) files.push(...markdownFilesUnder(absolute));
    else if (entry.isFile() && entry.name.endsWith(".md")) files.push(absolute);
  }
  return files.sort();
}

export function collectArchitectureContracts(root = resolve(REPO_ROOT, "docs/architecture")) {
  const contracts = [];
  const block = /```chronica-contract\s*\n([\s\S]*?)\n```/g;

  for (const file of markdownFilesUnder(root)) {
    const source = readFileSync(file, "utf8");
    let match;
    while ((match = block.exec(source)) !== null) {
      let declared;
      try {
        declared = JSON.parse(match[1]);
      } catch (err) {
        throw new Error(`invalid chronica-contract JSON in ${repoRel(file)}: ${err.message}`);
      }
      for (const field of ["id", "owner", "kind", "severity"]) {
        if (typeof declared[field] !== "string" || declared[field].length === 0) {
          throw new Error(`chronica-contract in ${repoRel(file)} is missing required string field "${field}"`);
        }
      }
      contracts.push({
        id: declared.id,
        owner: declared.owner,
        kind: declared.kind,
        severity: declared.severity,
        doc_path: repoRel(file),
        runtime_owner: Array.isArray(declared.runtime_owner) ? declared.runtime_owner : [],
        verification: Array.isArray(declared.verification) ? declared.verification : [],
      });
    }
  }

  contracts.sort((a, b) => a.id.localeCompare(b.id) || a.doc_path.localeCompare(b.doc_path));
  return contracts;
}

export function computeResponsibilitySummary(components) {
  const counts = {};
  const componentsByResponsibility = {};
  for (const component of components) {
    counts[component.responsibility] = (counts[component.responsibility] ?? 0) + 1;
    (componentsByResponsibility[component.responsibility] ??= []).push(component.id);
  }
  for (const list of Object.values(componentsByResponsibility)) list.sort();
  return { counts, components_by_responsibility: componentsByResponsibility };
}

// Donor evidence is reused wholesale from tools/refoundation/donor-corpus.yaml
// (the single canonical donor checklist) -- never re-derived or hand-counted
// here, per the refound directive's own instruction.
function computeDonorCensus(components, dependencies) {
  const doc = loadChecklist();
  const donors = doc.donors ?? [];
  const RESOLVED_STATUS = new Set(["ABSORBED", "RETIRED", "REFERENCE_ONLY", "NOT_NEEDED", "DUPLICATE"]);
  const donorsIdentified = donors.length;
  const donorsResolved = donors.filter((d) => RESOLVED_STATUS.has(d.status)).length;
  const donorResponsibilitiesUnaccounted = donors.filter(
    (d) => !RESOLVED_STATUS.has(d.status) && (!Array.isArray(d.chronica_targets) || d.chronica_targets.length === 0),
  ).length;

  // A real (not assumed) dependency from a non-donor component onto
  // temporary/ would mean donor source got wired into the build graph
  // instead of being pressure-formed into native code -- forbidden by the
  // absorption directive. Measured, not asserted, from the actual generated
  // dependency edges.
  const donorRuntimeDependencies = dependencies.filter(
    (dep) => dep.target_responsibility === "TEMPORARY_DONOR" && dep.source_responsibility !== "TEMPORARY_DONOR",
  ).length;

  return {
    donors_identified: donorsIdentified,
    donors_resolved: donorsResolved,
    donor_responsibilities_unaccounted: donorResponsibilitiesUnaccounted,
    donor_runtime_dependencies: donorRuntimeDependencies,
  };
}

function sourceSha() {
  return execFileSync("git", ["rev-parse", "HEAD"], { cwd: REPO_ROOT, encoding: "utf8" }).trim();
}

function digestFor(payloads) {
  const hash = createHash("sha256");
  for (const [name, value] of payloads) {
    hash.update(name);
    hash.update(JSON.stringify(value));
  }
  return hash.digest("hex");
}

function writeOne(name, schemaId, instance, schemasById) {
  const result = validate(instance, schemaId, schemasById);
  if (!result.valid) {
    throw new Error(`generated ${name} fails its own schema (${schemaId}):\n${formatErrors(result.errors)}`);
  }
  mkdirSync(OUT_DIR, { recursive: true });
  writeFileSync(resolve(OUT_DIR, name), `${JSON.stringify(instance, null, 2)}\n`);
}

function writeList(name, schemaId, items, schemasById, keyOf) {
  for (const item of items) {
    const result = validate(item, schemaId, schemasById);
    if (!result.valid) {
      throw new Error(`${name} item "${keyOf(item)}" fails schema (${schemaId}):\n${formatErrors(result.errors)}`);
    }
  }
  mkdirSync(OUT_DIR, { recursive: true });
  writeFileSync(resolve(OUT_DIR, name), `${JSON.stringify(items, null, 2)}\n`);
}

export function generate() {
  const schemasById = compileSchemas(SCHEMA_DIR);

  const metadata = loadCargoMetadata();
  const rustComponentList = rustComponents(metadata);
  const componentById = new Map(rustComponentList.map((c) => [c.id, c]));
  const rustDeps = rustDependencies(metadata, componentById);

  const { components: tsComponentList, deps: tsDeps } = tsComponentsAndDependencies();

  const components = [...rustComponentList, ...tsComponentList].sort((a, b) => a.id.localeCompare(b.id));
  const dependencies = [...rustDeps, ...tsDeps].sort(
    (a, b) => a.source.localeCompare(b.source) || a.target.localeCompare(b.target),
  );
  const violations = computeViolations(components, dependencies);
  const responsibility = computeResponsibilitySummary(components);
  const donorCensus = computeDonorCensus(components, dependencies);
  const fleetRegistry = loadFleetRegistry();
  const fleetAllocation = allocateDonors({ registry: fleetRegistry });
  const architectureContracts = collectArchitectureContracts();
  const organismModelAtlas = currentOrganismModelAtlas();
  const intelligenceBoundaryAtlas = currentIntelligenceBoundaryAtlas();
  const connectorAtlas = currentConnectorAtlas();

  // components.json/dependencies.json/violations.json are lists; each schema
  // file describes one element, not the wrapper array, so each element is
  // validated individually.
  writeList("components.json", "chronica.system-atlas.component.v2", components, schemasById, (c) => c.id);
  writeList("dependencies.json", "chronica.system-atlas.dependency.v2", dependencies, schemasById, (d) => `${d.source}->${d.target}`);
  writeOne("responsibilities.json", "chronica.system-atlas.responsibility-summary.v2", responsibility, schemasById);
  writeList("violations.json", "chronica.system-atlas.violation.v2", violations, schemasById, (v) => v.id);
  writeOne("organism-models.json", "chronica.system-atlas.organism-models.v1", organismModelAtlas, schemasById);
  writeOne("intelligence-boundary.json", "chronica.system-atlas.intelligence-boundary.v1", intelligenceBoundaryAtlas, schemasById);
  writeOne("connector-matrix.json", "chronica.system-atlas.connector-matrix.v1", connectorAtlas, schemasById);
  writeList(
    "architecture-contracts.json",
    "chronica.system-atlas.architecture-contract.v2",
    architectureContracts,
    schemasById,
    (contract) => contract.id,
  );

  const rustCrateCount = components.filter((c) => c.kind === "rust_crate").length;
  const tsPackageCount = components.filter((c) => c.kind === "ts_package").length;
  const countOf = (r) => responsibility.counts[r] ?? 0;

  const census = {
    schema: "chronica.system-atlas.census.v2",
    generated_at: new Date().toISOString(),
    source_sha: sourceSha(),
    components_total: components.length,
    rust_crates_total: rustCrateCount,
    ts_packages_total: tsPackageCount,
    core_components: countOf("CORE"),
    runtime_components: countOf("RUNTIME"),
    adapter_components: countOf("ADAPTER"),
    organism_components: countOf("ORGANISM"),
    organism_model_donors_total: organismModelAtlas.summary.donors_total,
    organism_model_source_present_total: organismModelAtlas.summary.source_present_total,
    organism_model_not_ingested_total: organismModelAtlas.summary.not_ingested_total,
    organism_model_censused_total: organismModelAtlas.summary.censused_total,
    organism_model_absorption_started_total: organismModelAtlas.summary.absorption_started_total,
    organism_model_roles_covered_total: organismModelAtlas.summary.roles_covered_total,
    organism_model_artifact_git_violations_total: organismModelAtlas.summary.production_model_artifacts_in_git_total,
    organism_evolution_readiness_present_total: organismModelAtlas.summary.evolution_readiness_present_total,
    organism_evolution_readiness_proof_only_total: organismModelAtlas.summary.evolution_readiness_proof_only_total,
    organism_evolution_readiness_gap_total: organismModelAtlas.summary.evolution_readiness_gap_total,
    intelligence_boundary_readiness_present_total: intelligenceBoundaryAtlas.summary.readiness_present_total,
    intelligence_boundary_readiness_proof_only_total: intelligenceBoundaryAtlas.summary.readiness_proof_only_total,
    intelligence_boundary_readiness_gap_total: intelligenceBoundaryAtlas.summary.readiness_gap_total,
    intelligence_boundary_hard_violations_total: intelligenceBoundaryAtlas.summary.hard_violations_total,
    intelligence_boundary_review_required_total: intelligenceBoundaryAtlas.summary.review_required_total,
    holding_id_reference_files_total: intelligenceBoundaryAtlas.summary.holding_id_reference_files_total,
    connector_protocol_total: connectorAtlas.summary.protocol_total,
    connector_discovered_total: connectorAtlas.summary.discovered_total,
    connector_source_present_total: connectorAtlas.summary.source_present_total,
    connector_proof_only_total: connectorAtlas.summary.proof_only_total,
    connector_production_total: connectorAtlas.summary.production_total,
    connector_physical_protocol_total: connectorAtlas.summary.physical_protocol_total,
    connector_physical_production_total: connectorAtlas.summary.physical_production_total,
    connector_runtime_contract_only_total: connectorAtlas.summary.runtime_contract_only_total,
    connector_runtime_partial_total: connectorAtlas.summary.runtime_partial_total,
    connector_runtime_simulated_only_total: connectorAtlas.summary.runtime_simulated_only_total,
    connector_runtime_gap_total: connectorAtlas.summary.runtime_gap_total,
    connector_hard_violations_total: connectorAtlas.summary.hard_violations_total,
    connector_review_required_total: connectorAtlas.summary.review_required_total,
    app_components: countOf("APP"),
    legacy_cap_components: countOf("LEGACY_CAP"),
    dependency_edges_total: dependencies.length,
    architecture_violations_total: violations.filter((v) => v.severity === "HARD").length,
    review_required_total: violations.filter((v) => v.severity === "REVIEW_REQUIRED").length,
    cap_legacy_remaining: countOf("LEGACY_CAP"),
    // TS backend extinction: no scanner for this yet (deferred, disclosed in
    // the final report rather than faked) -- 0 is a real count of currently-
    // classified TS_BACKEND_MIGRATION_REQUIRED violations, not an assumption.
    ts_backend_remaining: violations.filter((v) => v.type === "TS_BACKEND_MIGRATION_REQUIRED").length,
    // graph/ and bindings/ dual-source drift detection is deferred (see final
    // report); UNKNOWN is the honest state, not DUAL_MANUAL/AUTHORITATIVE
    // which would claim a measurement that wasn't made.
    graph_source_mode: "UNKNOWN",
    graph_drift_count: 0,
    binding_source_mode: "UNKNOWN",
    binding_drift_count: 0,
    ...donorCensus,
    duplicate_truth_store_signals: violations.filter((v) => v.type === "DUPLICATE_EVENT_STORE" || v.type === "DUPLICATE_MEMORY_TRUTH_STORE").length,
    duplicate_execution_universe_signals: violations.filter((v) => v.type === "DIRECT_ADAPTER_EXECUTION_FROM_APP" || v.type === "EXECUTION_ADMISSION_BYPASS").length,
    duplicate_authority_universe_signals: violations.filter((v) => v.type === "AUTHORITY_BYPASS" || v.type === "DUPLICATE_AUTHORITY_ENGINE").length,
    // Declaration/linkage coverage only: these counts do NOT claim runtime
    // materialization, documentation freshness or semantic correctness.
    architecture_contracts_declared_total: architectureContracts.length,
    organizational_control_contracts_declared_total: architectureContracts.filter((contract) => contract.owner === "organization").length,
    living_architecture_reconciliation_contracts_declared_total: architectureContracts.filter(
      (contract) => contract.id === "INV-ENGINEERING-CONTRACT-LIVING-DOCS",
    ).length,
    // Mirror capability presence is a fact about this Chronica checkout only.
    // It does NOT imply any external Ops has adopted/configured the protocol.
    ops_mirror_contracts_declared_total: architectureContracts.filter(
      (contract) => contract.id === "INV-SYSTEM-MODEL-OPS-MIRROR",
    ).length,
    fleet_control_contracts_declared_total: architectureContracts.filter(
      (contract) => contract.id === "INV-SYSTEM-MODEL-FLEET-CLOSED-WORLD",
    ).length,
    cell_reset_contracts_declared_total: architectureContracts.filter(
      (contract) => contract.id === "INV-SYSTEM-MODEL-CELL-RESET",
    ).length,
    extinction_ratchet_contracts_declared_total: architectureContracts.filter(
      (contract) => contract.id === "INV-SYSTEM-MODEL-EXTINCTION-RATCHET",
    ).length,
    ops_mirror_runtime_tools_present_total: [
      "tools/system-atlas/ops-mirror.mjs",
      "tools/system-atlas/ops-mirror-fleet.mjs",
    ].filter((path) => existsSync(resolve(REPO_ROOT, path))).length,
    ops_mirror_schemas_present_total: [
      "tools/system-atlas/schema/ops-mirror-contract.schema.json",
      "tools/system-atlas/schema/ops-mirror-report.schema.json",
      "tools/system-atlas/schema/ops-mirror-fleet.schema.json",
      "tools/system-atlas/schema/package-contract.schema.json",
    ].filter((path) => existsSync(resolve(REPO_ROOT, path))).length,
    development_cells_registered_total: fleetRegistry.development_cells.filter((row) => row.active !== false).length,
    chronica_packages_declared_total: new Set(
      fleetRegistry.development_cells.filter((row) => row.active !== false).map((row) => row.package_id),
    ).size,
    development_cells_with_reset_generation_total: fleetRegistry.development_cells.filter(
      (row) => row.active !== false && Number.isInteger(Number(row.refoundation_generation)) && Number(row.refoundation_generation) >= 1,
    ).length,
    // compatibility alias during Ops->Development Cell naming migration
    fleet_ops_registered_total: fleetRegistry.development_cells.filter((row) => row.active !== false).length,
    fleet_donor_allocations_total: fleetAllocation.allocations_total,
    fleet_donor_unallocated_total: fleetAllocation.unallocated_total,
    fleet_donor_skipped_total: fleetAllocation.skipped_total,
    fleet_donor_coverage_complete: fleetAllocation.coverage_complete,
    fleet_control_tools_present_total: [
      "tools/system-atlas/fleet/donor-allocator.mjs",
      "tools/system-atlas/fleet/fleet-census.mjs",
      "tools/system-atlas/fleet/fleet-plan.mjs",
      "tools/system-atlas/fleet/fleet-dispatcher.mjs",
      "tools/system-atlas/fleet/fleet-reconverge.mjs",
      "tools/system-atlas/fleet/fleet-advance.mjs",
    ].filter((path) => existsSync(resolve(REPO_ROOT, path))).length,
    fleet_control_schemas_present_total: [
      "tools/system-atlas/schema/fleet-donor-allocation.schema.json",
      "tools/system-atlas/schema/fleet-census.schema.json",
      "tools/system-atlas/schema/fleet-work-plan.schema.json",
      "tools/system-atlas/schema/chronica-candidate.schema.json",
      "tools/system-atlas/schema/fleet-worker-result.schema.json",
      "tools/system-atlas/schema/fleet-advance.schema.json",
    ].filter((path) => existsSync(resolve(REPO_ROOT, path))).length,
    fleet_provider_runners_present_total: [
      "tools/system-atlas/fleet/providers/claude-code-runner.mjs",
    ].filter((path) => existsSync(resolve(REPO_ROOT, path))).length,
    architecture_contract_runtime_owner_links_total: architectureContracts.reduce(
      (total, contract) => total + contract.runtime_owner.length,
      0,
    ),
    architecture_contract_verification_links_total: architectureContracts.reduce(
      (total, contract) => total + contract.verification.length,
      0,
    ),
  };
  writeOne("census.json", "chronica.system-atlas.census.v2", census, schemasById);

  const digest = {
    schema: "chronica.system-atlas.digest.v2",
    schema_version: "v2",
    scanner: "tools/system-atlas/generate.mjs",
    source_sha: census.source_sha,
    inputs_digest: digestFor([
      ["pnpm-workspace.yaml", readFileSync(resolve(REPO_ROOT, "pnpm-workspace.yaml"), "utf8")],
      ["cargo-metadata-packages", metadata.packages.map((p) => p.id).sort()],
    ]),
    outputs_digest: digestFor([
      ["components", components],
      ["dependencies", dependencies],
      ["responsibilities", responsibility],
      ["violations", violations],
      ["census", { ...census, generated_at: undefined }],
      ["architecture-contracts", architectureContracts],
      ["organism-models", { ...organismModelAtlas, generated_at: undefined }],
      ["intelligence-boundary", { ...intelligenceBoundaryAtlas, generated_at: undefined }],
      ["connector-matrix", { ...connectorAtlas, generated_at: undefined }],
    ]),
  };
  writeFileSync(resolve(OUT_DIR, "digest.json"), `${JSON.stringify(digest, null, 2)}\n`);

  return { components, dependencies, responsibility, violations, architectureContracts, organismModelAtlas, intelligenceBoundaryAtlas, connectorAtlas, census, digest };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const result = generate();
    console.log(
      `system-atlas v2 generated: ${result.components.length} components, ${result.dependencies.length} dependency edges, `
      + `${result.violations.length} violation(s) -> ${repoRel(OUT_DIR)}/`,
    );
    process.exit(0);
  } catch (err) {
    console.error(`system-atlas generate failed: ${err.message}`);
    process.exit(1);
  }
}
