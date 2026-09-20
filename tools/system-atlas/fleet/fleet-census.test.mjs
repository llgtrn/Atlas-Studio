import { test } from "node:test";
import assert from "node:assert/strict";
import { allocateDonors, loadFleetRegistry } from "./donor-allocator.mjs";
import { buildFleetCensus } from "./fleet-census.mjs";

test("fleet census never hides missing Ops reports or donor rows", () => {
  const allocation = allocateDonors({ now: "2026-09-19T00:00:00.000Z" });
  const registry = loadFleetRegistry();
  const census = buildFleetCensus({ allocation, registry, reports: [], now: "2026-09-19T00:00:00.000Z" });

  assert.equal(census.donor_corpus_total, allocation.donor_corpus_total);
  assert.equal(census.donor_allocations_total, allocation.donor_corpus_total);
  assert.equal(census.donor_skipped_total, 0);
  assert.equal(census.donor_unallocated_total, 0);
  assert.equal(census.donor_coverage_complete, true);
  const activeCells = registry.development_cells.filter((row) => row.active !== false);
  assert.equal(census.development_cells_expected_total, activeCells.length);
  assert.equal(census.development_cells_reported_total, 0);
  assert.equal(census.development_cells_missing_reports.length, census.development_cells_expected_total);
  assert.equal(census.ops_expected_total, activeCells.length);
  assert.equal(census.ops_reported_total, 0);
  assert.ok(census.primary_donor_census_missing.length > 0);
  assert.equal(census.readiness, "BLOCKED");
});

test("duplicate primary absorber and unknown donor are hard violations", () => {
  const allocation = {
    schema: "chronica.system-atlas.fleet-donor-allocation.v1",
    generated_at: "x",
    canonical_repo: "llgtrn/Chronica",
    donor_corpus_total: 1,
    allocations_total: 1,
    unallocated_total: 0,
    skipped_total: 0,
    coverage_complete: true,
    development_cells_total: 2,
    packages_total: 2,
    ops_targets_total: 2,
    primary_development_cell_total: 1,
    primary_ops_total: 1,
    primary_chronica_total: 0,
    source_relocation_required_total: 0,
    allocations: [{
      donor_id: "D1", repo: "owner/donor", status: "DISCOVERED", source_present: false, source_path: null, source_revision_hint: null, canonical_upstream: "https://github.com/owner/donor",
      primary_absorber: "llgtrn/AOps", primary_kind: "DEVELOPMENT_CELL",
      primary_cell_id: "a-cell", primary_package_id: "a-package",
      secondary_consumers: ["llgtrn/BOps"],
      secondary_consumer_packages: [{ repo: "llgtrn/BOps", cell_id: "b-cell", package_id: "b-package", kind: "DEVELOPMENT_CELL" }],
      basis: ["family:X"], allocation_state: "ALLOCATED", source_relocation: "SOURCE_NOT_PRESENT"
    }],
  };
  const registry = { canonical_repo: "llgtrn/Chronica", development_cells: [
    { cell_id: "a-cell", repo: "llgtrn/AOps", package_id: "a-package", package_kind: "DOMAIN_PACKAGE", active: true, refoundation_generation: 1 },
    { cell_id: "b-cell", repo: "llgtrn/BOps", package_id: "b-package", package_kind: "DOMAIN_PACKAGE", active: true, refoundation_generation: 1 },
  ]};
  const baseReport = {
    schema: "chronica.system-atlas.ops-mirror-report.v1", generated_at: "x",
    configuration_state: "CONFIGURED", readiness: "READY",
    development_cell_id: "a-cell", package_id: "a-package", package_kind: "DOMAIN_PACKAGE",
    package_contract_path: "chronica-package.json", package_contract_present: true,
    repository_layout_state: "MATCHES_CHRONICA", missing_mirror_roots: [],
    unexpected_top_level_directories: [], donor_root_policy_violations: [],
    feature_code_without_donor_evidence: false,
    canonical_runtime_mode: "CHRONICA_REQUIRED", standalone_sovereignty: false,
    refoundation_generation: 1, reset_mode: "CLEAN_PACKAGE_BASELINE",
    pre_reset_head_sha: "f".repeat(40), preservation_ref: "archive/chronica-pre-reset/g1-ffffffffffff",
    sovereignty_signals: [], local_persistence_signals: [],
    cell_repo: "llgtrn/AOps", cell_sha: "a".repeat(40),
    ops_repo: "llgtrn/AOps", ops_sha: "a".repeat(40), working_tree_clean: true,
    chronica_repo: "llgtrn/Chronica", chronica_reference_sha: "c".repeat(40), chronica_observed_head_sha: null,
    chronica_reference_state: "PINNED_UNVERIFIED", atlas_controller: "chronica-system-atlas",
    mirror_contract_path: "chronica-mirror.json", local_atlas_present: false, tracked_files_total: 1,
    backend_language_evaluated: true, frontend_language_evaluated: true, backend_non_rust_source_files: [],
    frontend_non_typescript_source_files: [], docs_kernel_required_total: 8, docs_kernel_present_total: 8, docs_kernel_missing: [],
    donors_declared_total: 1, donor_roots_detected: [], donor_files_total: 0, production_donor_references: [],
    semantic_mappings_total: 1, unresolved_semantic_mappings_total: 0, violations: [],
  };
  const donor = {
    id: "D1", repo: "owner/donor", revision: "v1", root: "temporary/donors/D1/source", mode: "DEEP_ABSORB",
    allocation_role: "PRIMARY_ABSORBER", provenance: "fixture", ops_baseline_commit: null, census_complete: true, present: false,
    tracked_files_total: 0, baseline_files_total: 0, remaining_baseline_files: 0, drained_baseline_files: 0, state: "NOT_BASELINED"
  };
  const reports = [
    { ...baseReport, donor_census: [donor] },
    { ...baseReport, development_cell_id: "b-cell", package_id: "b-package",
      cell_repo: "llgtrn/BOps", cell_sha: "b".repeat(40),
      ops_repo: "llgtrn/BOps", ops_sha: "b".repeat(40), donors_declared_total: 2,
      donor_census: [donor, { ...donor, id: "DX", repo: "other/unknown", allocation_role: "REFERENCE_CONSUMER" }] },
  ];
  const census = buildFleetCensus({ allocation, registry, reports, now: "2026-09-19T00:00:00.000Z" });
  assert.equal(census.readiness, "BLOCKED");
  assert.equal(census.duplicate_primary_absorbers.length, 1);
  assert.equal(census.donor_unknown_in_reports.length, 1);
  assert.equal(census.package_identity_mismatches.length, 0);
  assert.equal(census.cell_reset_mismatches.length, 0);
});

test("registered repository with wrong package identity is a hard Fleet violation", () => {
  const allocation = {
    schema: "chronica.system-atlas.fleet-donor-allocation.v1",
    generated_at: "x",
    canonical_repo: "llgtrn/Chronica",
    donor_corpus_total: 0,
    allocations_total: 0,
    unallocated_total: 0,
    skipped_total: 0,
    coverage_complete: true,
    development_cells_total: 1,
    packages_total: 1,
    ops_targets_total: 1,
    primary_development_cell_total: 0,
    primary_ops_total: 0,
    primary_chronica_total: 0,
    source_relocation_required_total: 0,
    allocations: [],
  };
  const registry = { canonical_repo: "llgtrn/Chronica", development_cells: [
    { cell_id: "trade-logistics", repo: "llgtrn/TradeOps", package_id: "trade-logistics", package_kind: "DOMAIN_PACKAGE", active: true, refoundation_generation: 1 },
  ]};
  const report = {
    schema: "chronica.system-atlas.ops-mirror-report.v1",
    generated_at: "x",
    configuration_state: "CONFIGURED",
    readiness: "READY",
    development_cell_id: "wrong-cell",
    package_id: "wrong-package",
    package_kind: "CAPABILITY_PACKAGE",
    package_contract_path: "chronica-package.json",
    package_contract_present: true,
    repository_layout_state: "MATCHES_CHRONICA",
    missing_mirror_roots: [],
    unexpected_top_level_directories: [],
    donor_root_policy_violations: [],
    feature_code_without_donor_evidence: false,
    canonical_runtime_mode: "CHRONICA_REQUIRED",
    standalone_sovereignty: false,
    refoundation_generation: 1,
    reset_mode: "CLEAN_PACKAGE_BASELINE",
    pre_reset_head_sha: "f".repeat(40),
    preservation_ref: "archive/chronica-pre-reset/g1-ffffffffffff",
    sovereignty_signals: [],
    local_persistence_signals: [],
    cell_repo: "llgtrn/TradeOps",
    cell_sha: "a".repeat(40),
    ops_repo: "llgtrn/TradeOps",
    ops_sha: "a".repeat(40),
    working_tree_clean: true,
    chronica_repo: "llgtrn/Chronica",
    chronica_reference_sha: "c".repeat(40),
    chronica_observed_head_sha: null,
    chronica_reference_state: "PINNED_UNVERIFIED",
    atlas_controller: "chronica-system-atlas",
    mirror_contract_path: "chronica-mirror.json",
    local_atlas_present: true,
    tracked_files_total: 1,
    backend_language_evaluated: true,
    frontend_language_evaluated: true,
    backend_non_rust_source_files: [],
    frontend_non_typescript_source_files: [],
    docs_kernel_required_total: 8,
    docs_kernel_present_total: 8,
    docs_kernel_missing: [],
    donors_declared_total: 0,
    donor_census: [],
    donor_roots_detected: [],
    donor_files_total: 0,
    production_donor_references: [],
    semantic_mappings_total: 1,
    unresolved_semantic_mappings_total: 0,
    violations: [],
  };
  const census = buildFleetCensus({ allocation, registry, reports: [report], now: "2026-09-19T00:00:00.000Z" });
  assert.equal(census.readiness, "BLOCKED");
  assert.equal(census.package_identity_mismatches.length, 3);
  assert.ok(census.violations.some((v) => v.type === "DEVELOPMENT_CELL_PACKAGE_IDENTITY_MISMATCH"));
});
