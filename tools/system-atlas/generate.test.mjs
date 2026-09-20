import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  generate,
  responsibilityForPath,
  computeViolations,
  computeResponsibilitySummary,
  collectArchitectureContracts,
} from "./generate.mjs";
import { compileSchemas, validate } from "./schema-validate.mjs";
import { REPO_ROOT } from "./lib.mjs";
import { loadChecklist } from "../refoundation/donor-corpus.mjs";
import { loadFleetRegistry } from "./fleet/donor-allocator.mjs";

const SCHEMA_DIR = resolve(REPO_ROOT, "tools/system-atlas/schema");

test("responsibilityForPath classifies each top-level directory per AGENTS.md section 4", () => {
  assert.equal(responsibilityForPath("core/"), "CORE");
  assert.equal(responsibilityForPath("runtime/"), "RUNTIME");
  assert.equal(responsibilityForPath("adapter/"), "ADAPTER");
  assert.equal(responsibilityForPath("organism/"), "ORGANISM");
  assert.equal(responsibilityForPath("apps/ui/"), "APP");
  assert.equal(responsibilityForPath("graph/"), "GRAPH");
  assert.equal(responsibilityForPath("bindings/"), "BINDING");
  assert.equal(responsibilityForPath("tools/"), "TOOLING");
  assert.equal(responsibilityForPath("license/"), "PROVENANCE");
  assert.equal(responsibilityForPath("temporary/"), "TEMPORARY_DONOR");
  assert.equal(responsibilityForPath("something-nobody-declared/"), "UNKNOWN");
});

test("computeViolations flags CORE -> ADAPTER as a HARD, PROVEN illegal dependency", () => {
  const components = [
    { id: "core", path: "core", kind: "rust_crate", responsibility: "CORE", evidence: ["core/Cargo.toml"] },
    { id: "adapter", path: "adapter", kind: "rust_crate", responsibility: "ADAPTER", evidence: ["adapter/Cargo.toml"] },
  ];
  const dependencies = [
    {
      source: "core",
      target: "adapter",
      dependency_kind: "cargo",
      source_responsibility: "CORE",
      target_responsibility: "ADAPTER",
      evidence: ["core/Cargo.toml"],
    },
  ];
  const violations = computeViolations(components, dependencies);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].type, "CORE_TO_ADAPTER");
  assert.equal(violations[0].severity, "HARD");
  assert.equal(violations[0].status, "PROVEN");
  assert.equal(violations[0].source, "core");
  assert.equal(violations[0].target, "adapter");
});

test("computeViolations flags RUNTIME -> APP but allows the reverse ADAPTER -> CORE direction", () => {
  const components = [
    { id: "runtime", path: "runtime", kind: "rust_crate", responsibility: "RUNTIME", evidence: ["runtime/Cargo.toml"] },
    { id: "apps/ui", path: "apps/ui", kind: "ts_package", responsibility: "APP", evidence: ["apps/ui/package.json"] },
    { id: "adapter", path: "adapter", kind: "rust_crate", responsibility: "ADAPTER", evidence: ["adapter/Cargo.toml"] },
    { id: "core", path: "core", kind: "rust_crate", responsibility: "CORE", evidence: ["core/Cargo.toml"] },
  ];
  const dependencies = [
    {
      source: "runtime", target: "apps/ui", dependency_kind: "cargo",
      source_responsibility: "RUNTIME", target_responsibility: "APP", evidence: ["runtime/Cargo.toml"],
    },
    {
      source: "adapter", target: "core", dependency_kind: "cargo",
      source_responsibility: "ADAPTER", target_responsibility: "CORE", evidence: ["adapter/Cargo.toml"],
    },
  ];
  const violations = computeViolations(components, dependencies);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].type, "RUNTIME_TO_APP");
});

test("computeViolations flags ORGANISM -> ADAPTER (organism has no direct effect path) but allows ORGANISM -> RUNTIME", () => {
  const components = [
    { id: "organism", path: "organism", kind: "rust_crate", responsibility: "ORGANISM", evidence: ["organism/Cargo.toml"] },
    { id: "adapter", path: "adapter", kind: "rust_crate", responsibility: "ADAPTER", evidence: ["adapter/Cargo.toml"] },
    { id: "runtime", path: "runtime", kind: "rust_crate", responsibility: "RUNTIME", evidence: ["runtime/Cargo.toml"] },
  ];
  const dependencies = [
    {
      source: "organism", target: "adapter", dependency_kind: "cargo",
      source_responsibility: "ORGANISM", target_responsibility: "ADAPTER", evidence: ["organism/Cargo.toml"],
    },
    {
      source: "organism", target: "runtime", dependency_kind: "cargo",
      source_responsibility: "ORGANISM", target_responsibility: "RUNTIME", evidence: ["organism/Cargo.toml"],
    },
  ];
  const violations = computeViolations(components, dependencies);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].type, "ORGANISM_TO_ADAPTER");
  assert.equal(violations[0].source, "organism");
  assert.equal(violations[0].target, "adapter");
});

test("computeViolations flags RUNTIME -> ORGANISM as illegal but allows ADAPTER -> ORGANISM (2026-09-17 dependency-inversion correction: adapter implements organism-owned analytical ports such as organism::world_model::WorldModel)", () => {
  const components = [
    { id: "adapter", path: "adapter", kind: "rust_crate", responsibility: "ADAPTER", evidence: ["adapter/Cargo.toml"] },
    { id: "runtime", path: "runtime", kind: "rust_crate", responsibility: "RUNTIME", evidence: ["runtime/Cargo.toml"] },
    { id: "organism", path: "organism", kind: "rust_crate", responsibility: "ORGANISM", evidence: ["organism/Cargo.toml"] },
  ];
  const dependencies = [
    {
      source: "adapter", target: "organism", dependency_kind: "cargo",
      source_responsibility: "ADAPTER", target_responsibility: "ORGANISM", evidence: ["adapter/Cargo.toml"],
    },
    {
      source: "runtime", target: "organism", dependency_kind: "cargo",
      source_responsibility: "RUNTIME", target_responsibility: "ORGANISM", evidence: ["runtime/Cargo.toml"],
    },
  ];
  const violations = computeViolations(components, dependencies);
  assert.deepEqual(
    violations.map((v) => v.type).sort(),
    ["RUNTIME_TO_ORGANISM"],
  );
});

test("computeViolations still flags ORGANISM -> ADAPTER as illegal even when ADAPTER -> ORGANISM also exists (the inversion grants organism no new effect path)", () => {
  const components = [
    { id: "adapter", path: "adapter", kind: "rust_crate", responsibility: "ADAPTER", evidence: ["adapter/Cargo.toml"] },
    { id: "organism", path: "organism", kind: "rust_crate", responsibility: "ORGANISM", evidence: ["organism/Cargo.toml"] },
  ];
  const dependencies = [
    {
      source: "adapter", target: "organism", dependency_kind: "cargo",
      source_responsibility: "ADAPTER", target_responsibility: "ORGANISM", evidence: ["adapter/Cargo.toml"],
    },
    {
      source: "organism", target: "adapter", dependency_kind: "cargo",
      source_responsibility: "ORGANISM", target_responsibility: "ADAPTER", evidence: ["organism/Cargo.toml"],
    },
  ];
  const violations = computeViolations(components, dependencies);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].type, "ORGANISM_TO_ADAPTER");
});

test("computeViolations flags any LEGACY_CAP component regardless of its edges", () => {
  const components = [
    { id: "legacy/foo", path: "legacy/foo", kind: "rust_crate", responsibility: "LEGACY_CAP", evidence: ["legacy/foo/Cargo.toml"] },
  ];
  const violations = computeViolations(components, []);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].type, "LEGACY_CAP_DEPENDENCY");
  assert.equal(violations[0].target, null);
});

test("computeResponsibilitySummary groups and sorts components deterministically", () => {
  const components = [
    { id: "b", responsibility: "CORE" },
    { id: "a", responsibility: "CORE" },
    { id: "z", responsibility: "APP" },
  ];
  const summary = computeResponsibilitySummary(components);
  assert.equal(summary.counts.CORE, 2);
  assert.equal(summary.counts.APP, 1);
  assert.deepEqual(summary.components_by_responsibility.CORE, ["a", "b"]);
});

test("generate() end-to-end: every emitted file validates against its own schema", () => {
  const schemasById = compileSchemas(SCHEMA_DIR);
  const result = generate();

  assert.ok(result.components.length > 0, "expected at least one real component from the live repo");
  for (const component of result.components) {
    const r = validate(component, "chronica.system-atlas.component.v2", schemasById);
    assert.ok(r.valid, `component ${component.id} invalid: ${JSON.stringify(r.errors)}`);
  }
  for (const dep of result.dependencies) {
    const r = validate(dep, "chronica.system-atlas.dependency.v2", schemasById);
    assert.ok(r.valid, `dependency ${dep.source}->${dep.target} invalid: ${JSON.stringify(r.errors)}`);
  }
  for (const contract of result.architectureContracts) {
    const r = validate(contract, "chronica.system-atlas.architecture-contract.v2", schemasById);
    assert.ok(r.valid, `architecture contract ${contract.id} invalid: ${JSON.stringify(r.errors)}`);
  }
  const censusResult = validate(result.census, "chronica.system-atlas.census.v2", schemasById);
  assert.ok(censusResult.valid, JSON.stringify(censusResult.errors));
  const organismModelResult = validate(result.organismModelAtlas, "chronica.system-atlas.organism-models.v1", schemasById);
  assert.ok(organismModelResult.valid, JSON.stringify(organismModelResult.errors));
  const intelligenceBoundaryResult = validate(result.intelligenceBoundaryAtlas, "chronica.system-atlas.intelligence-boundary.v1", schemasById);
  assert.ok(intelligenceBoundaryResult.valid, JSON.stringify(intelligenceBoundaryResult.errors));
  const connectorResult = validate(result.connectorAtlas, "chronica.system-atlas.connector-matrix.v1", schemasById);
  assert.ok(connectorResult.valid, JSON.stringify(connectorResult.errors));
});

test("generate() dependency endpoints always reference a real component id (no dangling edges)", () => {
  const result = generate();
  const ids = new Set(result.components.map((c) => c.id));
  for (const dep of result.dependencies) {
    assert.ok(ids.has(dep.source), `dangling dependency source: ${dep.source}`);
    assert.ok(ids.has(dep.target), `dangling dependency target: ${dep.target}`);
  }
});

test("generate() produces a reproducible digest for identical repo state (excluding wall-clock)", () => {
  const first = generate();
  const second = generate();
  assert.equal(first.digest.outputs_digest, second.digest.outputs_digest);
});

test("generate() reuses tools/refoundation/donor-corpus.yaml rather than hardcoding a donor count", () => {
  const result = generate();
  const checklist = loadChecklist();
  assert.equal(result.census.donors_identified, checklist.donors.length);
});

test("organizational-control contracts are measured as declarations, not runtime components", () => {
  const contracts = collectArchitectureContracts();
  const organizational = contracts.filter((contract) => contract.owner === "organization");
  assert.ok(organizational.length >= 4, "expected organizational-control architecture contracts");
  assert.ok(organizational.every((contract) => contract.doc_path === "docs/architecture/governance/organization.md"));
  const result = generate();
  assert.equal(result.census.organizational_control_contracts_declared_total, organizational.length);
  assert.ok(result.census.architecture_contracts_declared_total >= organizational.length);
});

test("living architecture reconciliation is declared and Atlas measures only declaration/linkage coverage", () => {
  const contracts = collectArchitectureContracts();
  const living = contracts.filter((contract) => contract.id === "INV-ENGINEERING-CONTRACT-LIVING-DOCS");
  assert.equal(living.length, 1, "expected exactly one living-architecture reconciliation invariant");
  assert.equal(living[0].doc_path, "docs/architecture/constitution/ENGINEERING-CONTRACT.md");
  assert.ok(living[0].runtime_owner.length > 0, "living-architecture invariant must name runtime owners");
  assert.ok(living[0].verification.length > 0, "living-architecture invariant must name verification");

  const runtimeOwnerLinks = contracts.reduce((total, contract) => total + contract.runtime_owner.length, 0);
  const verificationLinks = contracts.reduce((total, contract) => total + contract.verification.length, 0);
  const result = generate();

  assert.equal(result.census.living_architecture_reconciliation_contracts_declared_total, 1);
  assert.equal(result.census.architecture_contract_runtime_owner_links_total, runtimeOwnerLinks);
  assert.equal(result.census.architecture_contract_verification_links_total, verificationLinks);
  assert.ok(
    result.census.architecture_contracts_declared_total >= result.census.living_architecture_reconciliation_contracts_declared_total,
  );
});

test("Development Cell Mirror/package contracts and runtime tooling are measured without claiming remote adoption", () => {
  const contracts = collectArchitectureContracts();
  const mirror = contracts.filter((contract) => contract.id === "INV-SYSTEM-MODEL-OPS-MIRROR");
  assert.equal(mirror.length, 1);
  assert.equal(mirror[0].doc_path, "docs/architecture/foundation/system-model.md");
  const result = generate();
  assert.equal(result.census.ops_mirror_contracts_declared_total, 1);
  assert.equal(result.census.ops_mirror_runtime_tools_present_total, 2);
  assert.equal(result.census.ops_mirror_schemas_present_total, 4);
});

test("System Atlas closed-world fleet coverage equals the entire donor corpus and skips nothing", () => {
  const checklist = loadChecklist();
  const registry = loadFleetRegistry();
  const result = generate();
  assert.equal(result.census.fleet_control_contracts_declared_total, 1);
  assert.equal(result.census.cell_reset_contracts_declared_total, 1);
  assert.equal(result.census.extinction_ratchet_contracts_declared_total, 1);
  const activeCells = registry.development_cells.filter((row) => row.active !== false);
  assert.equal(result.census.development_cells_registered_total, activeCells.length);
  assert.equal(result.census.chronica_packages_declared_total, new Set(activeCells.map((row) => row.package_id)).size);
  assert.equal(result.census.development_cells_with_reset_generation_total, activeCells.length);
  assert.equal(result.census.fleet_ops_registered_total, activeCells.length);
  assert.equal(result.census.fleet_donor_allocations_total, checklist.donors.length);
  assert.equal(result.census.fleet_donor_unallocated_total, 0);
  assert.equal(result.census.fleet_donor_skipped_total, 0);
  assert.equal(result.census.fleet_donor_coverage_complete, true);
  assert.equal(result.census.fleet_control_tools_present_total, 6);
  assert.equal(result.census.fleet_control_schemas_present_total, 6);
  assert.equal(result.census.fleet_provider_runners_present_total, 1);
});

test("core is classified CORE and never depends on adapter, runtime, organism or app in the live repo", () => {
  const result = generate();
  const illegalFromCore = result.dependencies.filter(
    (d) => d.source_responsibility === "CORE" && ["ADAPTER", "RUNTIME", "ORGANISM", "APP"].includes(d.target_responsibility),
  );
  assert.deepEqual(illegalFromCore, [], `core has illegal outbound dependencies: ${JSON.stringify(illegalFromCore)}`);
});

test("organism is classified ORGANISM and never depends on adapter in the live repo", () => {
  const result = generate();
  const illegalFromOrganism = result.dependencies.filter(
    (d) => d.source_responsibility === "ORGANISM" && d.target_responsibility === "ADAPTER",
  );
  assert.deepEqual(illegalFromOrganism, [], `organism has illegal outbound dependencies: ${JSON.stringify(illegalFromOrganism)}`);
});

test("adapter is classified ADAPTER and MAY depend on organism in the live repo (2026-09-17 dependency-inversion correction)", () => {
  const result = generate();
  const adapterToOrganism = result.dependencies.filter(
    (d) => d.source_responsibility === "ADAPTER" && d.target_responsibility === "ORGANISM",
  );
  assert.ok(
    adapterToOrganism.length > 0,
    "expected at least one real adapter -> organism dependency (adapter implements an organism-owned analytical port)",
  );
  const violations = computeViolations(result.components, result.dependencies);
  const flagged = violations.filter((v) => v.type === "ADAPTER_TO_ORGANISM");
  assert.deepEqual(flagged, [], `adapter -> organism must never be flagged as a violation: ${JSON.stringify(flagged)}`);
});


test("System Atlas measures Organism donor/model reality without treating checkpoints as identity", () => {
  const result = generate();
  assert.ok(result.organismModelAtlas.summary.donors_total >= 2);
  assert.equal(result.organismModelAtlas.summary.production_model_artifacts_in_git_total, 0);
  assert.ok(result.organismModelAtlas.donors.some((d) => d.id === "D319"));
  assert.ok(result.organismModelAtlas.donors.some((d) => d.id === "D320"));
  assert.equal(result.census.organism_model_donors_total, result.organismModelAtlas.summary.donors_total);
  assert.equal(result.census.organism_model_source_present_total, result.organismModelAtlas.summary.source_present_total);
  assert.equal(result.census.organism_model_artifact_git_violations_total, 0);
  assert.equal(
    result.census.organism_evolution_readiness_present_total,
    result.organismModelAtlas.summary.evolution_readiness_present_total,
  );
  assert.equal(
    result.census.organism_evolution_readiness_gap_total,
    result.organismModelAtlas.summary.evolution_readiness_gap_total,
  );
  assert.equal(
    result.census.intelligence_boundary_readiness_present_total,
    result.intelligenceBoundaryAtlas.summary.readiness_present_total,
  );
  assert.equal(
    result.census.intelligence_boundary_hard_violations_total,
    result.intelligenceBoundaryAtlas.summary.hard_violations_total,
  );
});


test("System Atlas measures sovereign intelligence boundary separately from Organism model inventory", () => {
  const result = generate();
  assert.equal(result.intelligenceBoundaryAtlas.schema, "chronica.system-atlas.intelligence-boundary.v1");
  assert.ok(result.intelligenceBoundaryAtlas.readiness.some((x) => x.id === "HOLDING_KERNEL_VOCABULARY"));
  assert.ok(result.intelligenceBoundaryAtlas.readiness.some((x) => x.id === "MODEL_ROUTING"));
});


test("System Atlas reports connector protocol maturity from evidence, not architecture prose", () => {
  const result = generate();
  assert.equal(result.connectorAtlas.schema, "chronica.system-atlas.connector-matrix.v1");
  assert.equal(result.connectorAtlas.summary.physical_production_total, 0);
  assert.equal(result.connectorAtlas.protocols.find((x) => x.id === "OPC_UA").status, "DISCOVERED");
  assert.equal(result.connectorAtlas.protocols.find((x) => x.id === "ROS2").status, "DISCOVERED");
  assert.equal(result.connectorAtlas.protocols.find((x) => x.id === "HTTP_API").status, "PRODUCTION");
  assert.equal(result.connectorAtlas.runtime_readiness.find((x) => x.id === "EDGE_RUNTIME").state, "CONTRACT_ONLY");
  assert.equal(result.connectorAtlas.runtime_readiness.find((x) => x.id === "SAFETY_RUNTIME").state, "PARTIAL_RUNTIME");
  assert.equal(result.connectorAtlas.runtime_readiness.find((x) => x.id === "GENERIC_ROBOT_EFFECT").state, "SIMULATED_ONLY");
  assert.equal(result.census.connector_physical_production_total, result.connectorAtlas.summary.physical_production_total);
  assert.equal(result.census.connector_runtime_contract_only_total, result.connectorAtlas.summary.runtime_contract_only_total);
});
