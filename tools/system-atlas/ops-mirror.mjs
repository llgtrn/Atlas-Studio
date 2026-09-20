#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, extname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compileSchemas, validate, formatErrors } from "./schema-validate.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const CHRONICA_ROOT = resolve(HERE, "..", "..");
const SCHEMA_DIR = resolve(HERE, "schema");

const DOCS_KERNEL = [
  "docs/README.md",
  "docs/INDEX.md",
  "docs/TEMPLATE.md",
  "docs/architecture/",
  "docs/blueprints/",
  "docs/decisions/",
  "docs/guides/",
  "docs/references/",
];

const BACKEND_CODE_EXTENSIONS = new Set([
  ".rs", ".py", ".rb", ".go", ".java", ".kt", ".kts", ".cs", ".php", ".ex", ".exs",
  ".js", ".jsx", ".ts", ".tsx", ".c", ".cc", ".cpp", ".h", ".hpp",
]);
const FRONTEND_CODE_EXTENSIONS = new Set([".ts", ".tsx", ".js", ".jsx", ".vue", ".svelte"]);
const TS_FRONTEND_EXTENSIONS = new Set([".ts", ".tsx"]);
const SOVEREIGNTY_TOP_LEVEL_SIGNALS = new Set([
  "authority", "execution", "world", "canonical", "canonical-history",
  "event-store", "identity", "memory-kernel", "kernel",
]);
const LOCAL_PERSISTENCE_TOP_LEVEL_SIGNALS = new Set([
  "db", "database", "migrations", "prisma", "supabase",
]);
const CELL_REQUIRED_MIRROR_ROOTS = [
  "core","runtime","adapter","organism","graph","bindings","apps",
  "deploy","tools","docs","license","temporary","provenance",
];
const CELL_ALLOWED_TOP_LEVEL_DIRS = new Set([
  ...CELL_REQUIRED_MIRROR_ROOTS,
  ".github",
]);

function norm(path) {
  return String(path).replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/+$/, "");
}

function git(root, args, fallback = null) {
  try {
    return execFileSync("git", args, { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 }).trim();
  } catch {
    return fallback;
  }
}

function gitHead(root) {
  const value = git(root, ["rev-parse", "HEAD"]);
  return value && /^[0-9a-f]{40}$/.test(value) ? value : null;
}

function gitClean(root) {
  const value = git(root, ["status", "--porcelain"], null);
  return value !== null && value === "";
}

function trackedFiles(root) {
  try {
    const raw = execFileSync("git", ["ls-files", "-z"], { cwd: root, encoding: "utf8", maxBuffer: 128 * 1024 * 1024 });
    return raw.split("\0").filter(Boolean).map(norm).sort();
  } catch {
    return [];
  }
}

function trackedFilesAt(root, commit, donorRoot) {
  if (!commit) return null;
  const scopedRoot = norm(donorRoot);
  try {
    const args = ["ls-tree", "-r", "--name-only", "-z", commit];
    if (scopedRoot && scopedRoot !== ".") args.push("--", scopedRoot);
    const raw = execFileSync("git", args, { cwd: root, encoding: "utf8", maxBuffer: 128 * 1024 * 1024 });
    return raw.split("\0").filter(Boolean).map(norm).sort();
  } catch {
    return null;
  }
}

function remoteRepo(root) {
  const remote = git(root, ["config", "--get", "remote.origin.url"]);
  if (!remote) return null;
  const match = remote.match(/github\.com[/:]([^/]+)\/([^/]+?)(?:\.git)?$/);
  return match ? `${match[1]}/${match[2]}` : null;
}

function under(path, root) {
  const r = norm(root);
  const p = norm(path);
  return p === r || p.startsWith(`${r}/`);
}

function anyUnder(files, root) {
  return files.some((file) => under(file, root));
}

function filesUnder(files, roots) {
  return files.filter((file) => roots.some((root) => under(file, root)));
}

function excludedByRoots(file, roots) {
  return roots.some((root) => under(file, root));
}

function readContract(contractPath) {
  if (!existsSync(contractPath)) return null;
  return JSON.parse(readFileSync(contractPath, "utf8"));
}

function validateInstance(instance, schemaId) {
  const schemas = compileSchemas(SCHEMA_DIR);
  const result = validate(instance, schemaId, schemas);
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}

function observedChronicaHead(chronicaRoot) {
  if (!chronicaRoot) return null;
  const canonical = git(chronicaRoot, ["rev-parse", "--verify", "origin/main"], null);
  return canonical && /^[0-9a-f]{40}$/.test(canonical) ? canonical : gitHead(chronicaRoot);
}

function chronicaReferenceState(contract, chronicaRoot, observedHead) {
  if (!contract) return "UNPINNED";
  if (!chronicaRoot) return "PINNED_UNVERIFIED";
  if (!observedHead) return "PIN_UNAVAILABLE_LOCAL";
  if (observedHead === contract.chronica_reference_sha) return "MATCHES_LOCAL_HEAD";
  const known = git(chronicaRoot, ["cat-file", "-e", `${contract.chronica_reference_sha}^{commit}`], null);
  return known === null ? "PIN_UNAVAILABLE_LOCAL" : "STALE_AGAINST_LOCAL_HEAD";
}

function stripLineComments(content) {
  return content
    .split("\n")
    .map((line) => {
      const index = line.indexOf("//");
      return index === -1 ? line : line.slice(0, index);
    })
    .join("\n");
}

function sourceFilesWithDonorReferences(opsRoot, productionFiles, donorRoots) {
  const hits = new Set();
  const needles = donorRoots.map((root) => `${norm(root)}/`);
  for (const file of productionFiles) {
    if (!BACKEND_CODE_EXTENSIONS.has(extname(file)) && !FRONTEND_CODE_EXTENSIONS.has(extname(file))) continue;
    let content;
    try {
      content = stripLineComments(readFileSync(resolve(opsRoot, file), "utf8"));
    } catch {
      continue;
    }
    if (needles.some((needle) => content.includes(needle))) hits.add(file);
  }
  return [...hits].sort();
}

function violation(type, severity, evidence = []) {
  return { type, severity, evidence };
}

export function generateOpsMirrorReport({
  opsRoot,
  chronicaRoot = CHRONICA_ROOT,
  contractPath = resolve(opsRoot, "chronica-mirror.json"),
  now = new Date().toISOString(),
} = {}) {
  if (!opsRoot) throw new Error("opsRoot is required");
  const resolvedOpsRoot = resolve(opsRoot);
  const relativeContract = norm(relative(resolvedOpsRoot, contractPath));
  const files = trackedFiles(resolvedOpsRoot);
  const contract = readContract(contractPath);
  if (contract) validateInstance(contract, "chronica.system-atlas.ops-mirror-contract.v1");
  const packagePath = resolve(
    resolvedOpsRoot,
    contract?.package_contract_path ?? "chronica-package.json",
  );
  const relativePackageContract = norm(relative(resolvedOpsRoot, packagePath));
  const packageContract = readContract(packagePath);
  if (packageContract) validateInstance(packageContract, "chronica.system-atlas.package-contract.v1");

  const opsSha = gitHead(resolvedOpsRoot);
  const clean = gitClean(resolvedOpsRoot);
  const actualRepo = remoteRepo(resolvedOpsRoot);
  const backendRoots = (contract?.backend_roots ?? []).map(norm);
  const frontendRoots = (contract?.frontend_roots ?? []).map(norm);
  const donors = contract?.donors ?? [];
  const donorRoots = donors.map((donor) => norm(donor.root));
  const backendFiles = filesUnder(files, backendRoots).filter((file) => !excludedByRoots(file, donorRoots));
  const frontendFiles = filesUnder(files, frontendRoots).filter((file) => !excludedByRoots(file, donorRoots));
  const productionFiles = [...new Set([...backendFiles, ...frontendFiles])].sort();
  const productionCodeFiles = productionFiles.filter(
    (file) => BACKEND_CODE_EXTENSIONS.has(extname(file)) || FRONTEND_CODE_EXTENSIONS.has(extname(file)),
  );
  const topLevelDirs = [...new Set(
    files
      .filter((file) => norm(file).includes("/"))
      .map((file) => norm(file).split("/")[0])
      .filter(Boolean),
  )].sort();
  const missingMirrorRoots = CELL_REQUIRED_MIRROR_ROOTS.filter((root) => !anyUnder(files, root));
  const unexpectedTopLevelDirectories = topLevelDirs.filter((root) => !CELL_ALLOWED_TOP_LEVEL_DIRS.has(root));

  const backendNonRust = backendFiles
    .filter((file) => BACKEND_CODE_EXTENSIONS.has(extname(file)) && extname(file) !== ".rs")
    .sort();
  const frontendNonTs = frontendFiles
    .filter((file) => FRONTEND_CODE_EXTENSIONS.has(extname(file)) && !TS_FRONTEND_EXTENSIONS.has(extname(file)))
    .sort();

  const docsPresent = DOCS_KERNEL.filter((path) => path.endsWith("/") ? anyUnder(files, path) : files.includes(path));
  const docsMissing = DOCS_KERNEL.filter((path) => !docsPresent.includes(path));
  const currentFiles = new Set(files);
  const donorCensus = donors.map((donor) => {
    const root = norm(donor.root);
    const tracked = root === "." ? files : files.filter((file) => under(file, root));
    const baselineFiles = donor.mode === "REFERENCE_ONLY"
      ? []
      : trackedFilesAt(resolvedOpsRoot, donor.ops_baseline_commit, root);
    const baselineKnown = Array.isArray(baselineFiles);
    const remainingBaseline = baselineKnown
      ? baselineFiles.filter((file) => currentFiles.has(file))
      : [];
    const drainedBaseline = baselineKnown ? baselineFiles.length - remainingBaseline.length : 0;
    const state = donor.mode === "REFERENCE_ONLY"
      ? "REFERENCE_ONLY"
      : !donor.ops_baseline_commit || !baselineKnown
        ? "NOT_BASELINED"
        : baselineFiles.length > 0 && remainingBaseline.length === baselineFiles.length
          ? "INGESTED_NOT_STARTED"
          : remainingBaseline.length === 0
            ? "FULLY_DRAINED"
            : "ACTIVE_ABSORPTION";
    return {
      id: donor.id,
      repo: donor.repo,
      revision: donor.revision,
      root,
      mode: donor.mode,
      allocation_role: donor.allocation_role,
      provenance: donor.provenance,
      ops_baseline_commit: donor.ops_baseline_commit,
      census_complete: donor.census_complete === true,
      present: tracked.length > 0,
      tracked_files_total: tracked.length,
      baseline_files_total: baselineKnown ? baselineFiles.length : 0,
      remaining_baseline_files: remainingBaseline.length,
      drained_baseline_files: drainedBaseline,
      state,
    };
  }).sort((a, b) => a.id.localeCompare(b.id));
  const detectedDonorRoots = donorCensus.filter((donor) => donor.present).map((donor) => donor.root).sort();
  const donorRootPolicyViolations = donorCensus
    .filter((donor) =>
      donor.mode === "DEEP_ABSORB"
      && donor.root !== `temporary/donors/${donor.id}/source`
    )
    .map((donor) => `${donor.id}:${donor.root}`)
    .sort();
  const donorIntakeEvidenceMissing = donorCensus
    .filter((donor) =>
      donor.mode === "DEEP_ABSORB"
      && donor.present
      && !currentFiles.has(`temporary/donors/${donor.id}/intake.json`)
    )
    .map((donor) => donor.id)
    .sort();
  const donorProvenanceEvidenceMissing = donorCensus
    .filter((donor) =>
      donor.mode === "DEEP_ABSORB"
      && donor.present
      && !currentFiles.has(`provenance/donors/${donor.id}.json`)
    )
    .map((donor) => donor.id)
    .sort();
  const donorLicenseEvidenceMissing = donorCensus
    .filter((donor) =>
      donor.mode === "DEEP_ABSORB"
      && donor.present
      && !anyUnder(files, `license/donors/${donor.id}`)
    )
    .map((donor) => donor.id)
    .sort();
  const censusedPrimaryDonorTotal = donorCensus.filter((donor) =>
    donor.mode === "DEEP_ABSORB"
    && donor.allocation_role === "PRIMARY_ABSORBER"
    && donor.present
    && donor.census_complete === true
    && !donorIntakeEvidenceMissing.includes(donor.id)
    && !donorProvenanceEvidenceMissing.includes(donor.id)
    && !donorLicenseEvidenceMissing.includes(donor.id)
  ).length;
  // Donor burn-down is Git-derived from the pinned Ops donor baseline. New native files under a
  // deep-fork root do not count as donor legacy merely because they share the same directory.
  const donorFilesRemaining = donorCensus.reduce((total, donor) => total + donor.remaining_baseline_files, 0);
  const pathAddressableDonorRoots = donorRoots.filter((root) => root !== ".");
  const productionDonorReferences = sourceFilesWithDonorReferences(resolvedOpsRoot, productionFiles, pathAddressableDonorRoots);
  const mappings = contract?.semantic_mappings ?? [];
  const unresolvedMappings = mappings.filter((mapping) => mapping.disposition === "DEFER_UNRESOLVED_WITH_EVIDENCE");
  const topLevelRoots = [...new Set(files.map((file) => norm(file).split("/")[0]).filter(Boolean))].sort();
  const sovereigntySignals = topLevelRoots.filter((root) => SOVEREIGNTY_TOP_LEVEL_SIGNALS.has(root));
  const localPersistenceSignals = topLevelRoots.filter((root) => LOCAL_PERSISTENCE_TOP_LEVEL_SIGNALS.has(root));

  const violations = [];
  if (!contract) violations.push(violation("CELL_MIRROR_CONTRACT_MISSING", "HARD", [relativeContract]));
  if (contract && !contract.development_cell_id) {
    violations.push(violation("DEVELOPMENT_CELL_ID_MISSING", "HARD", [relativeContract]));
  }
  if (contract && !(contract.cell_repo ?? contract.ops_repo)) {
    violations.push(violation("CELL_REPO_IDENTITY_MISSING", "HARD", [relativeContract]));
  }
  if (contract && !contract.package_id) {
    violations.push(violation("PACKAGE_ID_MISSING", "HARD", [relativeContract]));
  }
  if (contract && !contract.package_contract_path) {
    violations.push(violation("PACKAGE_CONTRACT_PATH_MISSING", "HARD", [relativeContract]));
  }
  if (contract && contract.donor_extinction_policy !== "REPLACE_DELETE_RATCHET") {
    violations.push(violation("DONOR_EXTINCTION_POLICY_MIGRATION_REQUIRED", "HARD", [String(contract.donor_extinction_policy)]));
  }
  if (contract) {
    const requiredBackendRoots = ["adapter","core","organism","runtime"];
    const declaredBackendRoots = [...new Set(contract.backend_roots ?? [])].sort();
    if (JSON.stringify(declaredBackendRoots) !== JSON.stringify(requiredBackendRoots)) {
      violations.push(violation("CELL_BACKEND_ROOTS_POLICY_MISMATCH", "HARD", declaredBackendRoots));
    }
    const declaredFrontendRoots = [...new Set(contract.frontend_roots ?? [])].sort();
    if (JSON.stringify(declaredFrontendRoots) !== JSON.stringify(["apps/ui"])) {
      violations.push(violation("CELL_FRONTEND_ROOTS_POLICY_MISMATCH", "HARD", declaredFrontendRoots));
    }
  }
  if (!packageContract) violations.push(violation("PACKAGE_CONTRACT_MISSING", "HARD", [relativePackageContract]));
  if (packageContract && packageContract.reset_mode !== "CLEAN_PACKAGE_BASELINE") {
    violations.push(violation("CELL_RESET_MODE_INVALID", "HARD", [String(packageContract.reset_mode)]));
  }
  if (packageContract && (!Number.isInteger(packageContract.refoundation_generation) || packageContract.refoundation_generation < 1)) {
    violations.push(violation("CELL_REFOUNDATION_GENERATION_INVALID", "HARD", [String(packageContract.refoundation_generation)]));
  }
  if (packageContract && !packageContract.pre_reset_head_sha) {
    violations.push(violation("CELL_PRE_RESET_HEAD_MISSING", "HARD", [relativePackageContract]));
  }
  if (packageContract && !packageContract.preservation_ref) {
    violations.push(violation("CELL_PRESERVATION_REF_MISSING", "HARD", [relativePackageContract]));
  }
  if (packageContract && packageContract.repository_layout !== "CHRONICA_MIRROR_V1") {
    violations.push(violation("CELL_REPOSITORY_LAYOUT_RESET_REQUIRED", "HARD", [String(packageContract.repository_layout)]));
  }
  if (packageContract && packageContract.donor_root !== "temporary/donors") {
    violations.push(violation("CELL_DONOR_ROOT_POLICY_MISMATCH", "HARD", [String(packageContract.donor_root)]));
  }
  if (packageContract && packageContract.backend_language !== "RUST") {
    violations.push(violation("CELL_BACKEND_LANGUAGE_POLICY_MISMATCH", "HARD", [String(packageContract.backend_language)]));
  }
  if (packageContract && packageContract.frontend_language !== "TYPESCRIPT") {
    violations.push(violation("CELL_FRONTEND_LANGUAGE_POLICY_MISMATCH", "HARD", [String(packageContract.frontend_language)]));
  }
  if (packageContract && packageContract.frontend_root !== "apps/ui") {
    violations.push(violation("CELL_FRONTEND_ROOT_POLICY_MISMATCH", "HARD", [String(packageContract.frontend_root)]));
  }
  if (packageContract && packageContract.ui_policy !== "CHRONICA_UI_PROJECTION") {
    violations.push(violation("CELL_UI_POLICY_MISMATCH", "HARD", [String(packageContract.ui_policy)]));
  }
  if (packageContract && packageContract.feature_source_policy !== "DONOR_INTAKE_REQUIRED") {
    violations.push(violation("CELL_FEATURE_SOURCE_POLICY_MISMATCH", "HARD", [String(packageContract.feature_source_policy)]));
  }
  if (packageContract && packageContract.production_donor_dependency !== false) {
    violations.push(violation("CELL_PRODUCTION_DONOR_DEPENDENCY_POLICY_MISMATCH", "HARD", [String(packageContract.production_donor_dependency)]));
  }
  if (missingMirrorRoots.length > 0) {
    violations.push(violation("CELL_MIRROR_ROOTS_MISSING", "HARD", missingMirrorRoots));
  }
  if (unexpectedTopLevelDirectories.length > 0) {
    violations.push(violation("CELL_UNEXPECTED_TOP_LEVEL_DIRECTORIES", "HARD", unexpectedTopLevelDirectories));
  }
  if (donorRootPolicyViolations.length > 0) {
    violations.push(violation("CELL_DONOR_SOURCE_OUTSIDE_STANDARD_ROOT", "HARD", donorRootPolicyViolations));
  }
  if (donorIntakeEvidenceMissing.length > 0) {
    violations.push(violation("DONOR_INTAKE_EVIDENCE_MISSING", "HARD", donorIntakeEvidenceMissing));
  }
  if (donorProvenanceEvidenceMissing.length > 0) {
    violations.push(violation("DONOR_PROVENANCE_EVIDENCE_MISSING", "HARD", donorProvenanceEvidenceMissing));
  }
  if (donorLicenseEvidenceMissing.length > 0) {
    violations.push(violation("DONOR_LICENSE_EVIDENCE_MISSING", "HARD", donorLicenseEvidenceMissing));
  }
  if (productionCodeFiles.length > 0 && censusedPrimaryDonorTotal === 0) {
    violations.push(violation("NO_OSS_NO_CODE", "HARD", productionCodeFiles.slice(0, 50)));
  }
  if (!opsSha) violations.push(violation("CELL_HEAD_UNRESOLVED", "HARD", [resolvedOpsRoot]));
  if (!clean) violations.push(violation("CELL_WORKTREE_DIRTY", "HARD", ["git status --porcelain is non-empty or unavailable"]));
  const declaredRepo = contract?.cell_repo ?? contract?.ops_repo ?? null;
  if (contract && actualRepo && actualRepo !== declaredRepo) {
    violations.push(violation("CELL_REPO_IDENTITY_MISMATCH", "HARD", [`contract=${declaredRepo}`, `remote=${actualRepo}`]));
  }
  if (packageContract && actualRepo && packageContract.repo !== actualRepo) {
    violations.push(violation("PACKAGE_REPO_IDENTITY_MISMATCH", "HARD", [`package=${packageContract.repo}`, `remote=${actualRepo}`]));
  }
  if (contract && packageContract && contract.development_cell_id !== packageContract.cell_id) {
    violations.push(violation("DEVELOPMENT_CELL_ID_MISMATCH", "HARD", [contract.development_cell_id, packageContract.cell_id]));
  }
  if (contract && packageContract && contract.package_id !== packageContract.package_id) {
    violations.push(violation("PACKAGE_ID_MISMATCH", "HARD", [contract.package_id, packageContract.package_id]));
  }
  if (sovereigntySignals.length > 0) {
    violations.push(violation("STANDALONE_SOVEREIGNTY_SIGNAL", "REVIEW_REQUIRED", sovereigntySignals));
  }
  if (localPersistenceSignals.length > 0) {
    violations.push(violation("LOCAL_PERSISTENCE_REQUIRES_CLASSIFICATION", "REVIEW_REQUIRED", localPersistenceSignals));
  }

  const observedHead = observedChronicaHead(chronicaRoot);
  const refState = chronicaReferenceState(contract, chronicaRoot, observedHead);
  if (refState === "STALE_AGAINST_LOCAL_HEAD") {
    violations.push(violation("CHRONICA_REFERENCE_BEHIND_LOCAL_HEAD", "REVIEW_REQUIRED", [contract.chronica_reference_sha, observedHead]));
  } else if (refState === "PINNED_UNVERIFIED") {
    violations.push(violation("CHRONICA_REFERENCE_NOT_LOCALLY_VERIFIED", "REVIEW_REQUIRED", [contract.chronica_reference_sha]));
  } else if (refState === "PIN_UNAVAILABLE_LOCAL") {
    violations.push(violation("CHRONICA_REFERENCE_UNAVAILABLE_LOCAL", "HARD", [contract.chronica_reference_sha]));
  }

  if (contract && backendRoots.length === 0) violations.push(violation("BACKEND_ROOTS_UNDECLARED", "REVIEW_REQUIRED"));
  if (contract && frontendRoots.length === 0) violations.push(violation("FRONTEND_ROOTS_UNDECLARED", "REVIEW_REQUIRED"));
  if (docsMissing.length > 0) violations.push(violation("OPS_DOCS_KERNEL_INCOMPLETE", "HARD", docsMissing));
  if (backendNonRust.length > 0) violations.push(violation("NON_RUST_BACKEND_SOURCE", "HARD", backendNonRust));
  if (frontendNonTs.length > 0) violations.push(violation("NON_TYPESCRIPT_FRONTEND_SOURCE", "HARD", frontendNonTs));
  if (productionDonorReferences.length > 0) {
    violations.push(violation("PRODUCTION_DEPENDS_ON_DONOR_SOURCE", "HARD", productionDonorReferences));
  }
  for (const donor of donorCensus) {
    if (donor.allocation_role === "PRIMARY_ABSORBER" && donor.mode !== "DEEP_ABSORB") {
      violations.push(violation("PRIMARY_ABSORBER_NOT_DEEP_ABSORB", "HARD", [donor.id, donor.mode]));
    }
    if (donor.allocation_role === "REFERENCE_CONSUMER" && donor.mode !== "REFERENCE_ONLY") {
      violations.push(violation("REFERENCE_CONSUMER_NOT_REFERENCE_ONLY", "HARD", [donor.id, donor.mode]));
    }
    if (donor.allocation_role === "REFERENCE_CONSUMER" && donor.tracked_files_total > 0) {
      violations.push(violation("REFERENCE_CONSUMER_HAS_DONOR_SOURCE", "HARD", [
        donor.id,
        donor.root,
        String(donor.tracked_files_total),
      ]));
    }
    if (donor.mode === "DEEP_ABSORB" && donor.state === "NOT_BASELINED") {
      violations.push(violation(
        "DONOR_BASELINE_UNRESOLVED",
        "HARD",
        [donor.id, donor.ops_baseline_commit ?? "missing ops_baseline_commit"],
      ));
    }
  }
  if (contract && mappings.length === 0) {
    violations.push(violation("SEMANTIC_MAPPING_EMPTY", "REVIEW_REQUIRED", ["semantic_mappings"]));
  }
  if (unresolvedMappings.length > 0) {
    violations.push(violation(
      "UNRESOLVED_SEMANTIC_CONVERGENCE",
      "REVIEW_REQUIRED",
      unresolvedMappings.map((mapping) => `${mapping.local_semantic} -> ${mapping.chronica_owner}`),
    ));
  }

  const hard = violations.filter((row) => row.severity === "HARD").length;
  const review = violations.filter((row) => row.severity === "REVIEW_REQUIRED").length;
  const report = {
    schema: "chronica.system-atlas.ops-mirror-report.v1",
    generated_at: now,
    configuration_state: contract ? "CONFIGURED" : "UNCONFIGURED",
    readiness: hard > 0 ? "BLOCKED" : review > 0 ? "REVIEW_REQUIRED" : "READY",
    development_cell_id: contract?.development_cell_id ?? packageContract?.cell_id ?? null,
    package_id: contract?.package_id ?? packageContract?.package_id ?? null,
    package_kind: packageContract?.package_kind ?? null,
    package_contract_path: relativePackageContract,
    package_contract_present: Boolean(packageContract),
    repository_layout_state: packageContract?.repository_layout === "CHRONICA_MIRROR_V1"
      && missingMirrorRoots.length === 0
      && unexpectedTopLevelDirectories.length === 0
      ? "MATCHES_CHRONICA"
      : packageContract ? "DRIFT" : "UNCONFIGURED",
    missing_mirror_roots: missingMirrorRoots,
    unexpected_top_level_directories: unexpectedTopLevelDirectories,
    donor_root_policy_violations: donorRootPolicyViolations,
    donor_intake_evidence_missing: donorIntakeEvidenceMissing,
    donor_provenance_evidence_missing: donorProvenanceEvidenceMissing,
    donor_license_evidence_missing: donorLicenseEvidenceMissing,
    censused_primary_donor_total: censusedPrimaryDonorTotal,
    feature_code_without_donor_evidence: productionCodeFiles.length > 0 && censusedPrimaryDonorTotal === 0,
    canonical_runtime_mode: packageContract?.canonical_runtime ?? null,
    standalone_sovereignty: packageContract?.standalone_sovereignty ?? null,
    refoundation_generation: packageContract?.refoundation_generation ?? null,
    reset_mode: packageContract?.reset_mode ?? null,
    pre_reset_head_sha: packageContract?.pre_reset_head_sha ?? null,
    preservation_ref: packageContract?.preservation_ref ?? null,
    sovereignty_signals: sovereigntySignals,
    local_persistence_signals: localPersistenceSignals,
    cell_repo: contract?.cell_repo ?? contract?.ops_repo ?? actualRepo,
    cell_sha: opsSha,
    // Compatibility fields retained while external consumers migrate from legacy Ops naming.
    ops_repo: contract?.cell_repo ?? contract?.ops_repo ?? actualRepo,
    ops_sha: opsSha,
    working_tree_clean: clean,
    chronica_repo: contract?.chronica_repo ?? null,
    chronica_reference_sha: contract?.chronica_reference_sha ?? null,
    chronica_observed_head_sha: observedHead,
    chronica_reference_state: refState,
    atlas_controller: "chronica-system-atlas",
    mirror_contract_path: relativeContract,
    local_atlas_present: anyUnder(files, "tools/system-atlas"),
    tracked_files_total: files.length,
    backend_language_evaluated: backendRoots.length > 0,
    frontend_language_evaluated: frontendRoots.length > 0,
    backend_non_rust_source_files: backendNonRust,
    frontend_non_typescript_source_files: frontendNonTs,
    docs_kernel_required_total: DOCS_KERNEL.length,
    docs_kernel_present_total: docsPresent.length,
    docs_kernel_missing: docsMissing,
    donors_declared_total: donors.length,
    donor_census: donorCensus,
    donor_roots_detected: detectedDonorRoots,
    donor_files_total: donorFilesRemaining,
    production_donor_references: productionDonorReferences,
    semantic_mappings_total: mappings.length,
    unresolved_semantic_mappings_total: unresolvedMappings.length,
    violations,
  };
  validateInstance(report, "chronica.system-atlas.ops-mirror-report.v1");
  return report;
}

function argValue(args, name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : null;
}

function main(args = process.argv.slice(2)) {
  const opsRoot = resolve(argValue(args, "--ops-root") ?? process.cwd());
  const chronicaArg = argValue(args, "--chronica-root");
  const chronicaRoot = chronicaArg === "none" ? null : resolve(chronicaArg ?? CHRONICA_ROOT);
  const contractPath = resolve(argValue(args, "--contract") ?? resolve(opsRoot, "chronica-mirror.json"));
  const out = argValue(args, "--out");
  const report = generateOpsMirrorReport({ opsRoot, chronicaRoot, contractPath });
  const text = `${JSON.stringify(report, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
  return report.readiness === "BLOCKED" ? 1 : 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    process.exit(main());
  } catch (error) {
    console.error(`ops-mirror failed: ${error.message}`);
    process.exit(2);
  }
}
