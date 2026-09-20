import { test } from "node:test";
import assert from "node:assert/strict";
import { loadChecklist } from "../../refoundation/donor-corpus.mjs";
import { allocateDonors, loadFleetRegistry } from "./donor-allocator.mjs";

test("fleet donor allocation is closed-world over the entire current donor corpus", () => {
  const checklist = loadChecklist();
  const report = allocateDonors({ checklist, registry: loadFleetRegistry(), now: "2026-09-19T00:00:00.000Z" });
  assert.equal(report.donor_corpus_total, checklist.donors.length);
  assert.equal(report.allocations_total, checklist.donors.length);
  assert.equal(report.skipped_total, 0);
  assert.equal(report.unallocated_total, 0);
  assert.equal(report.coverage_complete, true);
  assert.equal(report.allocations.filter((row) => row.source_relocation === "RELOCATION_REQUIRED").length, report.source_relocation_required_total);
  assert.equal(new Set(report.allocations.map((row) => row.donor_id)).size, checklist.donors.length);
  assert.ok(report.allocations.every((row) => ["YES","NO","UNKNOWN"].includes(row.needed)));
});

test("same donor identity has one primary absorber and optional secondary consumers", () => {
  const report = allocateDonors({ now: "2026-09-19T00:00:00.000Z" });
  for (const row of report.allocations) {
    assert.ok(row.primary_absorber, `${row.donor_id} must have one primary absorber`);
    assert.ok(!row.secondary_consumers.includes(row.primary_absorber));
  }
  assert.equal(report.allocations.find((row) => row.repo === "chatwoot/chatwoot")?.primary_absorber, "llgtrn/HelpdeskOps");
  assert.equal(report.allocations.find((row) => row.repo === "twentyhq/twenty")?.primary_absorber, "llgtrn/CustomerOps");
});

test("all active Development Cells and package identities are counted even without donor allocation", () => {
  const registry = loadFleetRegistry();
  const active = registry.development_cells.filter((row) => row.active !== false);
  const report = allocateDonors({ registry, now: "2026-09-19T00:00:00.000Z" });
  assert.equal(report.development_cells_total, active.length);
  assert.equal(report.packages_total, new Set(active.map((row) => row.package_id)).size);
  assert.equal(report.ops_targets_total, active.length);
  assert.equal(report.primary_development_cell_total, report.primary_ops_total);
  assert.ok(report.allocations.every((row) =>
    row.primary_kind !== "DEVELOPMENT_CELL" || (row.primary_cell_id && row.primary_package_id)
  ));
});
