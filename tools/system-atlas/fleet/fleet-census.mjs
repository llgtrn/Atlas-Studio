#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { allocateDonors, loadFleetRegistry } from "./donor-allocator.mjs";
import { compileSchemas, validate, formatErrors } from "../schema-validate.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..", "..");
const SCHEMA_DIR = resolve(ROOT, "tools/system-atlas/schema");

function validateInstance(instance, schemaId) {
  const result = validate(instance, schemaId, compileSchemas(SCHEMA_DIR));
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}

function violation(type, severity, evidence = []) {
  return { type, severity, evidence };
}

export function buildFleetCensus({
  allocation = allocateDonors(),
  registry = loadFleetRegistry(),
  reports = [],
  now = new Date().toISOString(),
} = {}) {
  for (const report of reports) validateInstance(report, "chronica.system-atlas.ops-mirror-report.v1");

  const expectedCells = registry.development_cells.filter((row) => row.active !== false);
  const expectedRepos = expectedCells.map((row) => row.repo).sort();
  const expectedSet = new Set(expectedRepos);
  const expectedCellByRepo = new Map(expectedCells.map((row) => [row.repo, row]));
  const reportByRepo = new Map();
  const duplicateReportRepos = [];
  const packageIdentityMismatches = new Set();
  const cellResetMismatches = new Set();
  for (const report of reports) {
    const repo = report.cell_repo ?? report.ops_repo;
    if (!repo) continue;
    if (reportByRepo.has(repo)) duplicateReportRepos.push(repo);
    else reportByRepo.set(repo, report);
    const expected = expectedCellByRepo.get(repo);
    if (expected) {
      if (report.development_cell_id !== expected.cell_id) {
        packageIdentityMismatches.add(`${repo}:cell=${report.development_cell_id}!=${expected.cell_id}`);
      }
      if (report.package_id !== expected.package_id) {
        packageIdentityMismatches.add(`${repo}:package=${report.package_id}!=${expected.package_id}`);
      }
      if (report.package_kind !== expected.package_kind) {
        packageIdentityMismatches.add(`${repo}:kind=${report.package_kind}!=${expected.package_kind}`);
      }
      if (report.standalone_sovereignty !== false) {
        packageIdentityMismatches.add(`${repo}:standalone_sovereignty=${report.standalone_sovereignty}`);
      }
      if (report.canonical_runtime_mode !== "CHRONICA_REQUIRED") {
        packageIdentityMismatches.add(`${repo}:canonical_runtime=${report.canonical_runtime_mode}`);
      }
      const requiredGeneration = Number(expected.refoundation_generation ?? 1);
      const observedGeneration = Number(report.refoundation_generation ?? 0);
      if (observedGeneration < requiredGeneration) {
        cellResetMismatches.add(`${repo}:generation=${observedGeneration}<${requiredGeneration}`);
      }
      if (report.reset_mode !== "CLEAN_PACKAGE_BASELINE") {
        cellResetMismatches.add(`${repo}:reset_mode=${report.reset_mode}`);
      }
      if (!report.pre_reset_head_sha || !report.preservation_ref) {
        cellResetMismatches.add(`${repo}:reset_provenance_incomplete`);
      }
    }
  }

  const opsMissing = expectedRepos.filter((repo) => !reportByRepo.has(repo));
  const opsUnknown = [...reportByRepo.keys()].filter((repo) => !expectedSet.has(repo)).sort();

  const masterById = new Map(allocation.allocations.map((row) => [row.donor_id, row]));
  const primarySeen = new Map();
  const unknownDonors = new Set();
  const primaryMismatches = new Set();

  for (const report of reports) {
    for (const donor of report.donor_census) {
      const master = masterById.get(donor.id);
      if (!master) {
        unknownDonors.add(`${(report.cell_repo ?? report.ops_repo)}:${donor.id}`);
        continue;
      }
      if (donor.repo !== master.repo) {
        primaryMismatches.add(`${donor.id}:repo:${donor.repo}!=${master.repo}`);
      }
      if (donor.allocation_role === "PRIMARY_ABSORBER") {
        const rows = primarySeen.get(donor.id) ?? [];
        rows.push((report.cell_repo ?? report.ops_repo));
        primarySeen.set(donor.id, rows);
        if (master.primary_absorber !== (report.cell_repo ?? report.ops_repo)) {
          primaryMismatches.add(`${donor.id}:primary:${(report.cell_repo ?? report.ops_repo)}!=${master.primary_absorber}`);
        }
      }
    }
  }

  const duplicatePrimary = [...primarySeen.entries()]
    .filter(([, repos]) => new Set(repos).size > 1)
    .map(([id, repos]) => `${id}:${[...new Set(repos)].sort().join(",")}`)
    .sort();

  const primaryMissing = [];
  for (const row of allocation.allocations) {
    if (row.primary_kind !== "DEVELOPMENT_CELL") continue;
    const report = reportByRepo.get(row.primary_absorber);
    if (!report) {
      primaryMissing.push(`${row.donor_id}@${row.primary_absorber}:REPORT_MISSING`);
      continue;
    }
    const donor = report.donor_census.find((item) => item.id === row.donor_id);
    if (!donor) {
      primaryMissing.push(`${row.donor_id}@${row.primary_absorber}:DONOR_NOT_CENSUSED`);
      continue;
    }
    if (donor.allocation_role !== "PRIMARY_ABSORBER") {
      primaryMismatches.add(`${row.donor_id}@${row.primary_absorber}:role=${donor.allocation_role}`);
    }
  }

  const violations = [];
  if (!allocation.coverage_complete || allocation.skipped_total !== 0 || allocation.unallocated_total !== 0) {
    violations.push(violation("DONOR_CLOSED_WORLD_COVERAGE_BROKEN", "HARD", [
      `corpus=${allocation.donor_corpus_total}`,
      `allocations=${allocation.allocations_total}`,
      `skipped=${allocation.skipped_total}`,
      `unallocated=${allocation.unallocated_total}`,
    ]));
  }
  if (opsMissing.length) violations.push(violation("DEVELOPMENT_CELL_REPORTS_MISSING", "HARD", opsMissing));
  if (opsUnknown.length) violations.push(violation("UNREGISTERED_DEVELOPMENT_CELL_REPORTS", "HARD", opsUnknown));
  if (duplicateReportRepos.length) violations.push(violation("DUPLICATE_DEVELOPMENT_CELL_REPORT", "HARD", duplicateReportRepos));
  if (unknownDonors.size) violations.push(violation("DONOR_NOT_IN_MASTER_CORPUS", "HARD", [...unknownDonors].sort()));
  if (duplicatePrimary.length) violations.push(violation("MULTIPLE_PRIMARY_ABSORBERS", "HARD", duplicatePrimary));
  if (primaryMissing.length) violations.push(violation("PRIMARY_DONOR_CENSUS_INCOMPLETE", "HARD", primaryMissing));
  if (primaryMismatches.size) violations.push(violation("DONOR_ALLOCATION_MISMATCH", "HARD", [...primaryMismatches].sort()));
  if (packageIdentityMismatches.size) {
    violations.push(violation("DEVELOPMENT_CELL_PACKAGE_IDENTITY_MISMATCH", "HARD", [...packageIdentityMismatches].sort()));
  }
  if (cellResetMismatches.size) {
    violations.push(violation("DEVELOPMENT_CELL_RESET_REQUIRED", "HARD", [...cellResetMismatches].sort()));
  }

  for (const report of reports) {
    if (report.readiness === "BLOCKED") {
      violations.push(violation("CELL_MIRROR_BLOCKED", "HARD", [(report.cell_repo ?? report.ops_repo)]));
    } else if (report.readiness === "REVIEW_REQUIRED") {
      violations.push(violation("CELL_MIRROR_REVIEW_REQUIRED", "REVIEW_REQUIRED", [(report.cell_repo ?? report.ops_repo)]));
    }
  }

  const hard = violations.filter((row) => row.severity === "HARD").length;
  const review = violations.filter((row) => row.severity === "REVIEW_REQUIRED").length;
  const census = {
    schema: "chronica.system-atlas.fleet-census.v1",
    generated_at: now,
    canonical_repo: registry.canonical_repo,
    readiness: hard ? "BLOCKED" : review ? "REVIEW_REQUIRED" : "READY",
    donor_corpus_total: allocation.donor_corpus_total,
    donor_allocations_total: allocation.allocations_total,
    donor_skipped_total: allocation.skipped_total,
    donor_unallocated_total: allocation.unallocated_total,
    donor_coverage_complete: allocation.coverage_complete,
    development_cells_expected_total: expectedRepos.length,
    development_cells_reported_total: reports.filter((row) => {
      const repo = row.cell_repo ?? row.ops_repo;
      return repo && expectedSet.has(repo);
    }).length,
    development_cells_missing_reports: opsMissing,
    development_cells_unknown_reports: opsUnknown,
    package_identity_mismatches: [...packageIdentityMismatches].sort(),
    cell_reset_mismatches: [...cellResetMismatches].sort(),
    // Compatibility aliases retained during legacy Ops naming migration.
    ops_expected_total: expectedRepos.length,
    ops_reported_total: reports.filter((row) => {
      const repo = row.cell_repo ?? row.ops_repo;
      return repo && expectedSet.has(repo);
    }).length,
    ops_missing_reports: opsMissing,
    ops_unknown_reports: opsUnknown,
    donor_unknown_in_reports: [...unknownDonors].sort(),
    duplicate_primary_absorbers: duplicatePrimary,
    primary_donor_census_missing: primaryMissing.sort(),
    primary_donor_mismatches: [...primaryMismatches].sort(),
    violations,
  };
  validateInstance(census, "chronica.system-atlas.fleet-census.v1");
  return census;
}

function argValue(args, name) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : null;
}

function main(args = process.argv.slice(2)) {
  const outIndex = args.indexOf("--out");
  const out = outIndex >= 0 ? args[outIndex + 1] : null;
  const reportPaths = args.filter((arg, index) => arg !== "--out" && index !== outIndex + 1);
  const reports = reportPaths.map((path) => JSON.parse(readFileSync(resolve(path), "utf8")));
  const census = buildFleetCensus({ reports });
  const text = `${JSON.stringify(census, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
  return census.readiness === "BLOCKED" ? 1 : 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try { process.exit(main()); } catch (error) { console.error(`fleet census failed: ${error.message}`); process.exit(2); }
}
