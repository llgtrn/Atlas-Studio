import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { generateOpsMirrorReport } from "./ops-mirror.mjs";
import { summarizeOpsMirrorFleet } from "./ops-mirror-fleet.mjs";

function git(root, args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

function initRepo(prefix) {
  const root = mkdtempSync(join(tmpdir(), prefix));
  git(root, ["init"]);
  git(root, ["config", "user.email", "atlas@example.invalid"]);
  git(root, ["config", "user.name", "Atlas Test"]);
  return root;
}

function put(root, path, content = "x\n") {
  const full = join(root, path);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, content);
}

function commitAll(root, message) {
  git(root, ["add", "."]);
  git(root, ["commit", "-m", message]);
  return git(root, ["rev-parse", "HEAD"]);
}

function completeDocs(root) {
  put(root, "docs/README.md");
  put(root, "docs/INDEX.md");
  put(root, "docs/TEMPLATE.md");
  for (const dir of ["architecture", "blueprints", "decisions", "guides", "references"]) put(root, `docs/${dir}/README.md`);
}

function mirrorSkeleton(root) {
  completeDocs(root);
  for (const dir of ["core","runtime","adapter","organism","graph","bindings","apps","deploy","tools","license","temporary","provenance"]) {
    put(root, `${dir}/README.md`);
  }
  put(root, "apps/ui/README.md");
  put(root, "temporary/donors/README.md");
  put(root, "provenance/donors/README.md");
  put(root, "license/donors/README.md");
}

function packageContract() {
  return {
    schema: "chronica.system-atlas.package-contract.v1",
    cell_id: "test-cell",
    repo: "llgtrn/TestOps",
    package_id: "test-package",
    package_kind: "DOMAIN_PACKAGE",
    development_mode: "INDEPENDENTLY_DEVELOPABLE",
    production_mode: "CHRONICA_COMPOSED",
    canonical_runtime: "CHRONICA_REQUIRED",
    standalone_sovereignty: false,
    projection_may_deploy_separately: true,
    repository_layout: "CHRONICA_MIRROR_V1",
    production_roots: ["core","runtime","adapter","organism","graph","bindings","apps"],
    backend_roots: ["core","runtime","adapter","organism"],
    frontend_root: "apps/ui",
    donor_root: "temporary/donors",
    provenance_root: "provenance/donors",
    license_root: "license/donors",
    backend_language: "RUST",
    frontend_language: "TYPESCRIPT",
    ui_policy: "CHRONICA_UI_PROJECTION",
    feature_source_policy: "DONOR_INTAKE_REQUIRED",
    production_donor_dependency: false,
    refoundation_generation: 1,
    reset_mode: "CLEAN_PACKAGE_BASELINE",
    pre_reset_head_sha: "f".repeat(40),
    preservation_ref: "archive/chronica-pre-reset/g1-ffffffffffff",
    canonical_ownership: {
      world: "CHRONICA_ONLY",
      identity: "CHRONICA_ONLY",
      authority: "CHRONICA_ONLY",
      execution: "CHRONICA_ONLY",
      evidence: "CHRONICA_ONLY",
      memory: "CHRONICA_ONLY",
      canonical_history: "CHRONICA_ONLY",
      shared_truth: "CHRONICA_ONLY",
    },
    allowed_ownership: ["DOMAIN_SEMANTICS","ADAPTERS","PROJECTIONS","PACKAGE_UI","PACKAGE_API","LOCAL_CACHE","TEST_HARNESS"],
    package_roots: ["core","runtime","adapter","organism","graph","bindings","apps"],
  };
}

function contract(chronicaSha, donorBaseline) {
  return {
    schema: "chronica.system-atlas.ops-mirror-contract.v1",
    mode: "CELL_MIRROR",
    development_cell_id: "test-cell",
    cell_repo: "llgtrn/TestOps",
    package_id: "test-package",
    package_contract_path: "chronica-package.json",
    chronica_repo: "llgtrn/Chronica",
    chronica_reference_sha: chronicaSha,
    chronica_access: "READ_ONLY_REFERENCE",
    target_access: "MUTABLE",
    chronica_write_policy: "NO_DIRECT_WRITE_FROM_CELL_WORKER",
    upward_candidate_policy: "EXPLICIT_CHRONICA_CANDIDATE_ONLY",
    donor_extinction_policy: "REPLACE_DELETE_RATCHET",
    backend_roots: ["core","runtime","adapter","organism"],
    frontend_roots: ["apps/ui"],
    donors: [{
      id: "donor",
      repo: "example/donor",
      revision: "v1.0.0",
      root: "temporary/donors/donor/source",
      mode: "DEEP_ABSORB",
      allocation_role: "PRIMARY_ABSORBER",
      provenance: "provenance/donors/donor.json",
      ops_baseline_commit: donorBaseline,
      census_complete: true,
    }],
    semantic_mappings: [{
      local_semantic: "test.execute",
      local_path: "runtime/src/main.rs",
      chronica_owner: "runtime/",
      disposition: "MAP_TO_CHRONICA_SEMANTIC",
      evidence: ["runtime/src/main.rs"],
    }],
  };
}

test("configured Ops Mirror report is READY for clean Rust/TypeScript repo with complete docs", () => {
  const chronica = initRepo("chronica-mirror-");
  put(chronica, "README.md");
  const chronicaSha = commitAll(chronica, "chronica base");

  const ops = initRepo("ops-mirror-");
  git(ops, ["remote", "add", "origin", "https://github.com/llgtrn/TestOps.git"]);
  mirrorSkeleton(ops);
  put(ops, "temporary/donors/donor/source/legacy.py", "print('legacy')\n");
  const donorBaseline = commitAll(ops, "donor baseline");
  put(ops, "runtime/src/main.rs", "pub fn run() {}\n");
  put(ops, "apps/ui/src/app.ts", "export const ok = true;\n");
  put(ops, "chronica-package.json", JSON.stringify(packageContract(), null, 2));
  put(ops, "chronica-mirror.json", JSON.stringify(contract(chronicaSha, donorBaseline), null, 2));
  const opsSha = commitAll(ops, "ops mirror configured");

  const report = generateOpsMirrorReport({ opsRoot: ops, chronicaRoot: chronica, now: "2026-09-19T00:00:00.000Z" });
  assert.equal(report.readiness, "READY");
  assert.equal(report.configuration_state, "CONFIGURED");
  assert.equal(report.development_cell_id, "test-cell");
  assert.equal(report.package_id, "test-package");
  assert.equal(report.package_kind, "DOMAIN_PACKAGE");
  assert.equal(report.canonical_runtime_mode, "CHRONICA_REQUIRED");
  assert.equal(report.standalone_sovereignty, false);
  assert.equal(report.refoundation_generation, 1);
  assert.equal(report.reset_mode, "CLEAN_PACKAGE_BASELINE");
  assert.equal(report.pre_reset_head_sha, "f".repeat(40));
  assert.equal(report.preservation_ref, "archive/chronica-pre-reset/g1-ffffffffffff");
  assert.equal(report.cell_sha, opsSha);
  assert.equal(report.ops_sha, opsSha);
  assert.equal(report.chronica_observed_head_sha, chronicaSha);
  assert.equal(report.chronica_reference_state, "MATCHES_LOCAL_HEAD");
  assert.deepEqual(report.backend_non_rust_source_files, []);
  assert.deepEqual(report.frontend_non_typescript_source_files, []);
  assert.deepEqual(report.docs_kernel_missing, []);
  assert.equal(report.donors_declared_total, 1);
  assert.equal(report.donor_census[0].repo, "example/donor");
  assert.equal(report.donor_census[0].allocation_role, "PRIMARY_ABSORBER");
  assert.equal(report.donor_census[0].revision, "v1.0.0");
  assert.equal(report.donor_census[0].census_complete, true);
  assert.equal(report.repository_layout_state, "MATCHES_CHRONICA");
  assert.deepEqual(report.missing_mirror_roots, []);
  assert.deepEqual(report.unexpected_top_level_directories, []);
  assert.deepEqual(report.donor_root_policy_violations, []);
  assert.equal(report.feature_code_without_donor_evidence, false);
  assert.equal(report.donor_census[0].tracked_files_total, 1);
  assert.equal(report.donor_census[0].ops_baseline_commit, donorBaseline);
  assert.equal(report.donor_census[0].baseline_files_total, 1);
  assert.equal(report.donor_census[0].remaining_baseline_files, 1);
  assert.equal(report.donor_census[0].drained_baseline_files, 0);
  assert.equal(report.donor_census[0].state, "INGESTED_NOT_STARTED");
  assert.equal(report.donor_files_total, 1);
});

test("Ops Mirror blocks non-Rust backend and production dependency on donor source", () => {
  const chronica = initRepo("chronica-mirror-bad-");
  put(chronica, "README.md");
  const chronicaSha = commitAll(chronica, "chronica base");

  const ops = initRepo("ops-mirror-bad-");
  git(ops, ["remote", "add", "origin", "https://github.com/llgtrn/TestOps.git"]);
  mirrorSkeleton(ops);
  put(ops, "temporary/donors/donor/source/data.json", "{}\n");
  const donorBaseline = commitAll(ops, "donor baseline");
  put(ops, "runtime/src/main.rs", "const P: &str = \"temporary/donors/donor/source/data.json\";\n");
  put(ops, "runtime/src/legacy.py", "print('bad backend')\n");
  put(ops, "apps/ui/src/app.ts");
  put(ops, "chronica-package.json", JSON.stringify(packageContract(), null, 2));
  put(ops, "chronica-mirror.json", JSON.stringify(contract(chronicaSha, donorBaseline), null, 2));
  commitAll(ops, "bad ops");

  const report = generateOpsMirrorReport({ opsRoot: ops, chronicaRoot: chronica });
  assert.equal(report.readiness, "BLOCKED");
  assert.ok(report.backend_non_rust_source_files.includes("runtime/src/legacy.py"));
  assert.ok(report.production_donor_references.includes("runtime/src/main.rs"));
  assert.ok(report.violations.some((v) => v.type === "NON_RUST_BACKEND_SOURCE"));
  assert.ok(report.violations.some((v) => v.type === "PRODUCTION_DEPENDS_ON_DONOR_SOURCE"));
});

test("deep-fork donor burn-down is derived from the pinned Ops baseline rather than current root size", () => {
  const chronica = initRepo("chronica-mirror-drain-");
  put(chronica, "README.md");
  const chronicaSha = commitAll(chronica, "chronica base");

  const ops = initRepo("ops-mirror-drain-");
  git(ops, ["remote", "add", "origin", "https://github.com/llgtrn/TestOps.git"]);
  mirrorSkeleton(ops);
  put(ops, "temporary/donors/donor/source/legacy.py", "print('legacy')\n");
  const donorBaseline = commitAll(ops, "donor baseline");

  git(ops, ["rm", "temporary/donors/donor/source/legacy.py"]);
  put(ops, "runtime/src/main.rs", "pub fn native() {}\n");
  put(ops, "apps/ui/src/app.ts", "export const native = true;\n");
  put(ops, "chronica-package.json", JSON.stringify(packageContract(), null, 2));
  put(ops, "chronica-mirror.json", JSON.stringify(contract(chronicaSha, donorBaseline), null, 2));
  commitAll(ops, "native replacement");

  const report = generateOpsMirrorReport({ opsRoot: ops, chronicaRoot: chronica });
  assert.equal(report.readiness, "READY");
  assert.equal(report.donor_census[0].baseline_files_total, 1);
  assert.equal(report.donor_census[0].remaining_baseline_files, 0);
  assert.equal(report.donor_census[0].drained_baseline_files, 1);
  assert.equal(report.donor_census[0].state, "FULLY_DRAINED");
  assert.equal(report.donor_files_total, 0);
});

test("reference consumer cannot carry a second deep donor source tree", () => {
  const chronica = initRepo("chronica-mirror-reference-");
  put(chronica, "README.md");
  const chronicaSha = commitAll(chronica, "chronica base");

  const ops = initRepo("ops-mirror-reference-");
  git(ops, ["remote", "add", "origin", "https://github.com/llgtrn/TestOps.git"]);
  mirrorSkeleton(ops);
  put(ops, "temporary/donors/donor/source/legacy.py", "print('duplicate source')\n");
  const cfg = contract(chronicaSha, null);
  cfg.donors[0].allocation_role = "REFERENCE_CONSUMER";
  cfg.donors[0].mode = "REFERENCE_ONLY";
  put(ops, "chronica-package.json", JSON.stringify(packageContract(), null, 2));
  put(ops, "chronica-mirror.json", JSON.stringify(cfg, null, 2));
  commitAll(ops, "reference consumer with donor source");

  const report = generateOpsMirrorReport({ opsRoot: ops, chronicaRoot: chronica });
  assert.equal(report.readiness, "BLOCKED");
  assert.ok(report.violations.some((v) => v.type === "REFERENCE_CONSUMER_HAS_DONOR_SOURCE"));
});

test("standalone sovereignty shaped roots are surfaced as package review signals", () => {
  const chronica = initRepo("chronica-package-sovereignty-");
  put(chronica, "README.md");
  const chronicaSha = commitAll(chronica, "chronica base");

  const cell = initRepo("cell-package-sovereignty-");
  git(cell, ["remote", "add", "origin", "https://github.com/llgtrn/TestOps.git"]);
  mirrorSkeleton(cell);
  put(cell, "runtime/src/main.rs", "pub fn run() {}\n");
  put(cell, "apps/ui/src/app.ts", "export const ok = true;\n");
  put(cell, "authority/local.rs", "pub fn local_authority() {}\n");
  put(cell, "chronica-package.json", JSON.stringify(packageContract(), null, 2));
  const cfg = contract(chronicaSha, null);
  cfg.donors = [];
  put(cell, "chronica-mirror.json", JSON.stringify(cfg, null, 2));
  commitAll(cell, "cell with sovereignty signal");

  const report = generateOpsMirrorReport({ opsRoot: cell, chronicaRoot: chronica });
  assert.equal(report.readiness, "BLOCKED");
  assert.ok(report.sovereignty_signals.includes("authority"));
  assert.ok(report.violations.some((v) => v.type === "STANDALONE_SOVEREIGNTY_SIGNAL"));
});

test("legacy Ops Mirror contract parses but is BLOCKED until Cell/package identity is migrated", () => {
  const chronica = initRepo("chronica-legacy-mirror-");
  put(chronica, "README.md");
  const chronicaSha = commitAll(chronica, "chronica base");

  const cell = initRepo("legacy-cell-mirror-");
  git(cell, ["remote", "add", "origin", "https://github.com/llgtrn/TestOps.git"]);
  mirrorSkeleton(cell);
  put(cell, "runtime/src/main.rs", "pub fn run() {}\n");
  put(cell, "apps/ui/src/app.ts", "export const ok = true;\n");
  const legacy = contract(chronicaSha, null);
  legacy.mode = "OPS_MIRROR";
  legacy.ops_repo = legacy.cell_repo;
  legacy.chronica_write_policy = "NO_DIRECT_WRITE_FROM_OPS_WORKER";
  delete legacy.development_cell_id;
  delete legacy.cell_repo;
  delete legacy.package_id;
  delete legacy.package_contract_path;
  legacy.donors = [];
  put(cell, "chronica-mirror.json", JSON.stringify(legacy, null, 2));
  commitAll(cell, "legacy mirror");

  const report = generateOpsMirrorReport({ opsRoot: cell, chronicaRoot: chronica });
  assert.equal(report.readiness, "BLOCKED");
  assert.ok(report.violations.some((v) => v.type === "DEVELOPMENT_CELL_ID_MISSING"));
  assert.ok(report.violations.some((v) => v.type === "PACKAGE_ID_MISSING"));
  assert.ok(report.violations.some((v) => v.type === "PACKAGE_CONTRACT_MISSING"));
});

test("missing mirror contract is measured as UNCONFIGURED/BLOCKED rather than guessed", () => {
  const ops = initRepo("ops-mirror-unconfigured-");
  put(ops, "README.md");
  commitAll(ops, "empty ops");
  const report = generateOpsMirrorReport({ opsRoot: ops, chronicaRoot: null });
  assert.equal(report.configuration_state, "UNCONFIGURED");
  assert.equal(report.readiness, "BLOCKED");
  assert.ok(report.violations.some((v) => v.type === "CELL_MIRROR_CONTRACT_MISSING"));
  assert.ok(report.violations.some((v) => v.type === "PACKAGE_CONTRACT_MISSING"));
});

test("fleet aggregation stays derived from reports", () => {
  const base = {
    schema: "chronica.system-atlas.ops-mirror-report.v1",
    generated_at: "2026-09-19T00:00:00.000Z",
    configuration_state: "CONFIGURED",
    readiness: "READY",
    development_cell_id: "a-cell",
    package_id: "a-package",
    package_kind: "DOMAIN_PACKAGE",
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
    cell_repo: "llgtrn/AOps",
    cell_sha: "a".repeat(40),
    ops_repo: "llgtrn/AOps",
    ops_sha: "a".repeat(40),
    working_tree_clean: true,
    chronica_repo: "llgtrn/Chronica",
    chronica_reference_sha: "c".repeat(40),
    chronica_observed_head_sha: null,
    chronica_reference_state: "PINNED_UNVERIFIED",
    atlas_controller: "chronica-system-atlas",
    mirror_contract_path: "chronica-mirror.json",
    local_atlas_present: false,
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
  const blocked = {
    ...base,
    development_cell_id: "b-cell",
    package_id: "b-package",
    cell_repo: "llgtrn/BOps",
    cell_sha: "b".repeat(40),
    ops_repo: "llgtrn/BOps",
    ops_sha: "b".repeat(40),
    readiness: "BLOCKED",
    backend_non_rust_source_files: ["backend/x.py"],
    violations: [{ type: "NON_RUST_BACKEND_SOURCE", severity: "HARD", evidence: ["backend/x.py"] }],
  };
  const fleet = summarizeOpsMirrorFleet([base, blocked], "2026-09-19T00:00:00.000Z");
  assert.equal(fleet.readiness, "BLOCKED");
  assert.equal(fleet.reports_total, 2);
  assert.equal(fleet.blocked_total, 1);
  assert.equal(fleet.backend_non_rust_source_files_total, 1);
  assert.equal(fleet.donors_declared_total, 0);
  assert.deepEqual(fleet.chronica_reference_shas, ["c".repeat(40)]);

  const empty = summarizeOpsMirrorFleet([], "2026-09-19T00:00:00.000Z");
  assert.equal(empty.readiness, "REVIEW_REQUIRED");
});
