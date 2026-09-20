#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compileSchemas, validate, formatErrors } from "./schema-validate.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(HERE, "schema");

function validateInstance(instance, schemaId) {
  const schemas = compileSchemas(SCHEMA_DIR);
  const result = validate(instance, schemaId, schemas);
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}

export function summarizeOpsMirrorFleet(reports, now = new Date().toISOString()) {
  for (const report of reports) validateInstance(report, "chronica.system-atlas.ops-mirror-report.v1");
  const hard = reports.reduce((n, report) => n + report.violations.filter((v) => v.severity === "HARD").length, 0);
  const review = reports.reduce((n, report) => n + report.violations.filter((v) => v.severity === "REVIEW_REQUIRED").length, 0);
  const refs = [...new Set(reports.map((report) => report.chronica_reference_sha).filter(Boolean))].sort();
  const fleet = {
    schema: "chronica.system-atlas.ops-mirror-fleet.v1",
    generated_at: now,
    readiness: reports.length === 0
      ? "REVIEW_REQUIRED"
      : reports.some((report) => report.readiness === "BLOCKED")
        ? "BLOCKED"
        : reports.some((report) => report.readiness === "REVIEW_REQUIRED")
          ? "REVIEW_REQUIRED"
          : "READY",
    reports_total: reports.length,
    configured_total: reports.filter((r) => r.configuration_state === "CONFIGURED").length,
    unconfigured_total: reports.filter((r) => r.configuration_state === "UNCONFIGURED").length,
    ready_total: reports.filter((r) => r.readiness === "READY").length,
    review_required_total: reports.filter((r) => r.readiness === "REVIEW_REQUIRED").length,
    blocked_total: reports.filter((r) => r.readiness === "BLOCKED").length,
    hard_violations_total: hard,
    review_violations_total: review,
    backend_non_rust_source_files_total: reports.reduce((n, r) => n + r.backend_non_rust_source_files.length, 0),
    frontend_non_typescript_source_files_total: reports.reduce((n, r) => n + r.frontend_non_typescript_source_files.length, 0),
    production_donor_references_total: reports.reduce((n, r) => n + r.production_donor_references.length, 0),
    donors_declared_total: reports.reduce((n, r) => n + r.donors_declared_total, 0),
    donor_files_total: reports.reduce((n, r) => n + r.donor_files_total, 0),
    docs_kernel_missing_total: reports.reduce((n, r) => n + r.docs_kernel_missing.length, 0),
    development_cells_total: new Set(reports.map((r) => r.development_cell_id).filter(Boolean)).size,
    packages_total: new Set(reports.map((r) => r.package_id).filter(Boolean)).size,
    standalone_sovereignty_signal_total: reports.reduce((n, r) => n + r.sovereignty_signals.length, 0),
    local_persistence_signal_total: reports.reduce((n, r) => n + r.local_persistence_signals.length, 0),
    clean_reset_total: reports.filter((r) =>
      Number(r.refoundation_generation ?? 0) >= 1
      && r.reset_mode === "CLEAN_PACKAGE_BASELINE"
      && Boolean(r.pre_reset_head_sha)
      && Boolean(r.preservation_ref)
    ).length,
    reset_required_total: reports.filter((r) =>
      Number(r.refoundation_generation ?? 0) < 1
      || r.reset_mode !== "CLEAN_PACKAGE_BASELINE"
      || !r.pre_reset_head_sha
      || !r.preservation_ref
    ).length,
    layout_drift_total: reports.filter((r) => r.repository_layout_state !== "MATCHES_CHRONICA").length,
    no_oss_no_code_violation_total: reports.filter((r) => r.feature_code_without_donor_evidence === true).length,
    donor_root_policy_violation_total: reports.reduce((n, r) => n + (r.donor_root_policy_violations?.length ?? 0), 0),
    chronica_reference_shas: refs,
    repos: reports
      .map((report) => ({
        development_cell_id: report.development_cell_id,
        package_id: report.package_id,
        package_kind: report.package_kind,
        cell_repo: report.cell_repo,
        cell_sha: report.cell_sha,
        refoundation_generation: report.refoundation_generation,
        reset_mode: report.reset_mode,
        pre_reset_head_sha: report.pre_reset_head_sha,
        preservation_ref: report.preservation_ref,
        // compatibility aliases
        ops_repo: report.ops_repo,
        ops_sha: report.ops_sha,
        configuration_state: report.configuration_state,
        readiness: report.readiness,
        chronica_reference_sha: report.chronica_reference_sha,
        hard_violations: report.violations.filter((v) => v.severity === "HARD").length,
        review_violations: report.violations.filter((v) => v.severity === "REVIEW_REQUIRED").length,
      }))
      .sort((a, b) => String(a.cell_repo ?? a.ops_repo).localeCompare(String(b.cell_repo ?? b.ops_repo))),
  };
  validateInstance(fleet, "chronica.system-atlas.ops-mirror-fleet.v1");
  return fleet;
}

function main(args = process.argv.slice(2)) {
  const outIndex = args.indexOf("--out");
  const out = outIndex >= 0 ? args[outIndex + 1] : null;
  const inputs = args.filter((arg, index) => arg !== "--out" && index !== outIndex + 1);
  if (inputs.length === 0) {
    console.error("usage: ops-mirror-fleet.mjs [--out <path>] <report.json> [report2.json ...]");
    return 2;
  }
  const reports = inputs.map((path) => JSON.parse(readFileSync(resolve(path), "utf8")));
  const fleet = summarizeOpsMirrorFleet(reports);
  const text = `${JSON.stringify(fleet, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
  return fleet.readiness === "BLOCKED" ? 1 : 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    process.exit(main());
  } catch (error) {
    console.error(`ops-mirror-fleet failed: ${error.message}`);
    process.exit(2);
  }
}
