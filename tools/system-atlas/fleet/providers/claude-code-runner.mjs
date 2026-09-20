#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const MUTATING_DIRECT = new Set([
  "CELL_RESET_BASELINE",
  "DONOR_INTAKE",
  "DONOR_ACCOUNT",
  "DONOR_CENSUS",
  "PACKAGE_VERIFY_INTEGRATE",
  "CHRONICA_VERIFY_INTEGRATE",
  "PACKAGE_RECONVERGE",
]);
const CANDIDATE_ONLY = new Set(["PACKAGE_BUILD", "CHRONICA_CANDIDATE"]);
const READ_ONLY = new Set(["DONOR_ABSORB_SCOUT"]);

function commandExists(command) {
  const result = spawnSync(command, ["--version"], { encoding: "utf8" });
  return result.status === 0;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd,
    env: options.env ?? process.env,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    input: options.input,
  });
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || "").trim();
    throw new Error(`${command} ${args.join(" ")} failed (${result.status}): ${detail}`);
  }
  return (result.stdout || "").trim();
}

function git(cwd, args) {
  return run("git", args, { cwd });
}

function safe(value) {
  return String(value).replace(/[^A-Za-z0-9_.-]+/g, "-").slice(0, 120);
}

export function parseClaudePayload(value) {
  if (typeof value !== "string") throw new Error("Claude result payload must be a string");
  let text = value.trim();
  if (text.startsWith("```")) {
    text = text.replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/, "");
  }
  const parsed = JSON.parse(text);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Claude payload must be a JSON object");
  }
  return parsed;
}

export function taskExecutionClass(kind) {
  if (READ_ONLY.has(kind)) return "READ_ONLY";
  if (CANDIDATE_ONLY.has(kind)) return "CANDIDATE_ONLY";
  if (MUTATING_DIRECT.has(kind)) return "DIRECT_TARGET_MUTATION";
  throw new Error(`unsupported Claude runner task kind ${kind}`);
}

function resolveTargetHead(repoDir, targetRef) {
  git(repoDir, ["fetch", "origin", "--prune", "+refs/heads/*:refs/remotes/origin/*"]);
  try {
    return git(repoDir, ["rev-parse", "--verify", `origin/${targetRef}`]);
  } catch {
    git(repoDir, ["fetch", "origin", targetRef]);
    return git(repoDir, ["rev-parse", "FETCH_HEAD"]);
  }
}

function underScope(path, scope) {
  const normalized = String(scope ?? "").replaceAll("\\", "/").replace(/^\.\//, "").replace(/\/+$/, "");
  if (!normalized || normalized === ".") return true;
  return path === normalized || path.startsWith(`${normalized}/`);
}

function extinctionEvidence(repoDir, task, baseSha, head) {
  if (!["PACKAGE_BUILD"].includes(task.kind)) {
    return {
      applicable: false,
      files_removed: 0,
      remaining_scoped_files: null,
      state: "NOT_APPLICABLE",
      blocker: null,
      deleted_paths: [],
    };
  }
  const scopes = Array.isArray(task.deletion_scope) ? task.deletion_scope : [];
  const deleted = git(repoDir, ["diff","--diff-filter=D","--name-only",`${baseSha}..${head}`])
    .split("\n").filter(Boolean)
    .filter((path) => scopes.some((scope) => underScope(path, scope)));
  const tracked = git(repoDir, ["ls-files"]).split("\n").filter(Boolean);
  const remaining = tracked.filter((path) => scopes.some((scope) => underScope(path, scope))).length;
  return {
    applicable: true,
    files_removed: deleted.length,
    remaining_scoped_files: remaining,
    state: remaining === 0 && deleted.length > 0 ? "SOURCE_EXTINCT" : deleted.length > 0 ? "PROGRESSED" : "BLOCKED",
    blocker: deleted.length > 0 ? null : `NO_DONOR_SOURCE_REMOVED scopes=${scopes.join(",")}`,
    deleted_paths: deleted,
  };
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function performCellReset(repoDir, task, baseSha, branchName) {
  const generation = Number(task.refoundation_generation ?? 1);
  const preservationRef = `archive/chronica-pre-reset/g${generation}-${baseSha.slice(0, 12)}`;
  git(repoDir, ["push","origin",`${baseSha}:refs/heads/${preservationRef}`]);

  git(repoDir, ["rm","-r","--ignore-unmatch","."]);
  for (const dir of [
    "core","runtime","adapter","organism","graph","bindings","apps/ui",
    "deploy","tools","license/donors","temporary/donors","provenance/donors",
    "docs/architecture","docs/blueprints","docs/decisions","docs/guides","docs/references",
  ]) {
    mkdirSync(join(repoDir, dir), { recursive: true });
  }

  const packageContract = {
    schema: "chronica.system-atlas.package-contract.v1",
    cell_id: task.target_cell_id,
    repo: task.target_repo,
    package_id: task.target_package_id,
    package_kind: task.target_package_kind,
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
    refoundation_generation: generation,
    reset_mode: "CLEAN_PACKAGE_BASELINE",
    pre_reset_head_sha: baseSha,
    preservation_ref: preservationRef,
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
    allowed_ownership: [
      "DOMAIN_SEMANTICS","CAPABILITY_DECLARATIONS","ADAPTERS","PROJECTIONS",
      "PACKAGE_UI","PACKAGE_API","PROVIDER_STATE","LOCAL_CACHE","TEST_HARNESS","SIMULATION_FIXTURES",
    ],
    package_roots: ["core","runtime","adapter","organism","graph","bindings","apps"],
  };
  const mirror = {
    schema: "chronica.system-atlas.ops-mirror-contract.v1",
    mode: "CELL_MIRROR",
    development_cell_id: task.target_cell_id,
    cell_repo: task.target_repo,
    package_id: task.target_package_id,
    package_contract_path: "chronica-package.json",
    chronica_repo: "llgtrn/Chronica",
    chronica_reference_sha: task.chronica_reference_sha,
    chronica_access: "READ_ONLY_REFERENCE",
    target_access: "MUTABLE",
    chronica_write_policy: "NO_DIRECT_WRITE_FROM_CELL_WORKER",
    upward_candidate_policy: "EXPLICIT_CHRONICA_CANDIDATE_ONLY",
    donor_extinction_policy: "REPLACE_DELETE_RATCHET",
    backend_roots: ["core","runtime","adapter","organism"],
    frontend_roots: ["apps/ui"],
    semantic_mappings: [],
    donors: [],
  };

  writeJson(join(repoDir, "chronica-package.json"), packageContract);
  writeJson(join(repoDir, "chronica-mirror.json"), mirror);
  writeFileSync(join(repoDir, "README.md"), `# ${task.target_package_id}\n\nChronica Development Cell / Package baseline.\n\nCanonical runtime: Chronica required. Standalone sovereignty: false.\n`);
  writeFileSync(join(repoDir, "AGENTS.md"), `# Development Cell Contract\n\nThis repository is Development Cell ${task.target_cell_id} for Chronica Package ${task.target_package_id}.\n\nDo not create peer canonical World, Identity, Authority, Execution, Evidence, Memory, history or shared truth.\n\nAbsorption rule: READ MINIMUM -> BUILD -> PROVE -> DELETE DONOR SLICE -> COMMIT -> NEXT.\n`);
  writeFileSync(join(repoDir, ".gitignore"), "target/\nnode_modules/\n.env\n");
  for (const dir of ["core","runtime","adapter","organism","graph","bindings","deploy","tools"]) {
    writeFileSync(join(repoDir, dir, "README.md"), `# ${dir}\n\nMirror-isomorphic Chronica responsibility root. Add implementation only through allocated donor absorption or governed package reconvergence.\n`);
  }
  writeFileSync(join(repoDir, "apps", "README.md"), "# apps\n\nThin package projections only. UI lives under apps/ui and uses TypeScript.\n");
  writeFileSync(join(repoDir, "apps", "ui", "README.md"), "# UI\n\nTypeScript-only UI/projection root. No feature code before donor intake.\n");
  writeFileSync(join(repoDir, "temporary", "README.md"), "# temporary\n\nAcquisition-only donor source. Production roots must never import or link from here.\n");
  writeFileSync(join(repoDir, "temporary", "donors", "README.md"), "# donors\n\nEach donor lives at temporary/donors/<DONOR_ID>/source with an intake record.\n");
  writeFileSync(join(repoDir, "provenance", "donors", "README.md"), "# donor provenance\n\nExact donor URL/revision/intake evidence.\n");
  writeFileSync(join(repoDir, "license", "donors", "README.md"), "# donor license evidence\n\nPreserved donor license artifacts.\n");
  writeFileSync(join(repoDir, "docs", "README.md"), "# Documentation\n\nLiving package architecture only; Git is history.\n");
  writeFileSync(join(repoDir, "docs", "INDEX.md"), "# Documentation Index\n\n- architecture/\n- blueprints/\n- decisions/\n- guides/\n- references/\n");
  writeFileSync(join(repoDir, "docs", "TEMPLATE.md"), "# Documentation Template\n\nDescribe current package architecture, not progress notes.\n");
  for (const dir of ["architecture","blueprints","decisions","guides","references"]) {
    writeFileSync(join(repoDir, "docs", dir, "README.md"), `# ${dir}\n\nPackage-local documentation must not redefine Chronica canonical semantics.\n`);
  }

  git(repoDir, ["add","-A"]);
  git(repoDir, ["config","user.name","Chronica Fleet"]);
  git(repoDir, ["config","user.email","chronica-fleet@users.noreply.github.com"]);
  git(repoDir, ["commit","-m",`refoundation: reset ${task.target_package_id} to clean Chronica package baseline g${generation}`]);
  const head = git(repoDir, ["rev-parse","HEAD"]);
  return { head, preservationRef, generation, branchName };
}

function copyTrackedTree(sourceRepo, targetRoot) {
  const files = git(sourceRepo, ["ls-files", "-z"]).split("\0").filter(Boolean);
  const staged = git(sourceRepo, ["ls-files", "--stage"]).split("\n").filter(Boolean);
  const submodules = staged
    .filter((line) => line.startsWith("160000 "))
    .map((line) => line.split("\t")[1])
    .filter(Boolean);
  if (submodules.length) {
    throw new Error(`DONOR_SUBMODULES_REQUIRE_EXPLICIT_INTAKE: ${submodules.join(",")}`);
  }
  mkdirSync(targetRoot, { recursive: true });
  run("git", ["checkout-index","-a",`--prefix=${resolve(targetRoot)}/`], { cwd: sourceRepo });
  return files;
}

function donorLicenseFiles(files) {
  return files.filter((file) => {
    const name = file.split("/").pop() ?? "";
    return /^(?:license|licence|copying|notice)(?:\.|$)/i.test(name);
  });
}

function performDonorIntake(repoDir, workDir, task, ghBin) {
  if (!task.donor_id || !task.donor_repo || !task.donor_source_path) {
    throw new Error("DONOR_INTAKE requires donor_id, donor_repo and donor_source_path");
  }
  const expectedRoot = `temporary/donors/${task.donor_id}/source`;
  if (task.donor_source_path !== expectedRoot) {
    throw new Error(`DONOR_INTAKE path mismatch: ${task.donor_source_path} != ${expectedRoot}`);
  }
  const targetSource = join(repoDir, task.donor_source_path);
  if (existsSync(targetSource)) {
    throw new Error(`donor source path already exists: ${task.donor_source_path}`);
  }

  const donorRepoDir = join(workDir, "donor-source");
  run(ghBin, ["repo","clone",task.donor_repo,donorRepoDir,"--","--no-tags"]);
  if (task.donor_revision) {
    git(donorRepoDir, ["fetch","origin",task.donor_revision]);
    git(donorRepoDir, ["checkout","--detach",task.donor_revision]);
  }
  const upstreamSha = git(donorRepoDir, ["rev-parse","HEAD"]);
  if (task.donor_revision && upstreamSha !== task.donor_revision) {
    throw new Error(`DONOR_REVISION_MISMATCH expected=${task.donor_revision} actual=${upstreamSha}`);
  }
  const upstreamRemote = git(donorRepoDir, ["config","--get","remote.origin.url"]);
  const tracked = copyTrackedTree(donorRepoDir, targetSource);

  const donorBase = join(repoDir, "temporary", "donors", task.donor_id);
  mkdirSync(donorBase, { recursive: true });
  const intakePath = join(donorBase, "intake.json");
  writeJson(intakePath, {
    schema: "chronica.development-cell.donor-intake.v1",
    donor_id: task.donor_id,
    repo: task.donor_repo,
    upstream: upstreamRemote,
    revision: upstreamSha,
    source_root: task.donor_source_path,
    tracked_files_total: tracked.length,
    target_cell_id: task.target_cell_id,
    target_package_id: task.target_package_id,
  });

  const provenancePath = join(repoDir, "provenance", "donors", `${task.donor_id}.json`);
  mkdirSync(dirname(provenancePath), { recursive: true });
  writeJson(provenancePath, {
    schema: "chronica.development-cell.donor-provenance.v1",
    donor_id: task.donor_id,
    repo: task.donor_repo,
    upstream: upstreamRemote,
    revision: upstreamSha,
    intake_record: `temporary/donors/${task.donor_id}/intake.json`,
    source_root: task.donor_source_path,
  });

  const licenseRoot = join(repoDir, "license", "donors", task.donor_id);
  mkdirSync(licenseRoot, { recursive: true });
  for (const file of donorLicenseFiles(tracked)) {
    const source = join(donorRepoDir, file);
    const target = join(licenseRoot, file.replaceAll("/", "__"));
    copyFileSync(source, target);
  }
  if (donorLicenseFiles(tracked).length === 0) {
    writeFileSync(join(licenseRoot, "README.md"), "No recognized tracked license filename was found automatically. Manual license review is required before production release.\n");
  }

  const mirrorPath = join(repoDir, "chronica-mirror.json");
  const mirror = JSON.parse(readFileSync(mirrorPath, "utf8"));
  mirror.donors = (mirror.donors ?? []).filter((donor) => donor.id !== task.donor_id);
  mirror.donors.push({
    id: task.donor_id,
    repo: task.donor_repo,
    revision: upstreamSha,
    root: task.donor_source_path,
    mode: "DEEP_ABSORB",
    provenance: `provenance/donors/${task.donor_id}.json`,
    ops_baseline_commit: null,
    allocation_role: "PRIMARY_ABSORBER",
    census_complete: false,
  });
  writeJson(mirrorPath, mirror);

  git(repoDir, ["config","user.name","Chronica Fleet"]);
  git(repoDir, ["config","user.email","chronica-fleet@users.noreply.github.com"]);
  git(repoDir, ["add","-A"]);
  git(repoDir, ["commit","-m",`donor: intake ${task.donor_id} ${task.donor_repo}@${upstreamSha.slice(0, 12)}`]);
  const donorBaselineSha = git(repoDir, ["rev-parse","HEAD"]);

  const pinnedMirror = JSON.parse(readFileSync(mirrorPath, "utf8"));
  const entry = pinnedMirror.donors.find((donor) => donor.id === task.donor_id);
  entry.ops_baseline_commit = donorBaselineSha;
  writeJson(mirrorPath, pinnedMirror);
  git(repoDir, ["add","chronica-mirror.json"]);
  git(repoDir, ["commit","-m",`donor: pin ${task.donor_id} Cell baseline`]);
  const head = git(repoDir, ["rev-parse","HEAD"]);

  return { head, upstreamSha, donorBaselineSha, trackedFilesTotal: tracked.length };
}

function changedPaths(repoDir, baseSha, head) {
  return git(repoDir, ["diff","--name-only",`${baseSha}..${head}`]).split("\n").filter(Boolean);
}

function packageShapeViolations(paths) {
  const allowedTop = new Set([
    "core","runtime","adapter","organism","graph","bindings","apps",
    "deploy","tools","docs","license","temporary","provenance",".github",
  ]);
  const backendRoots = ["core","runtime","adapter","organism"];
  const backendCodeExt = new Set([".rs",".py",".go",".java",".kt",".c",".cc",".cpp",".h",".hpp",".js",".ts",".tsx",".mjs",".cjs"]);
  const frontendCodeExt = new Set([".js",".jsx",".ts",".tsx",".mjs",".cjs",".vue",".svelte"]);
  const violations = [];
  for (const raw of paths) {
    const path = raw.replaceAll("\\", "/");
    if (!path.includes("/")) continue;
    const top = path.split("/")[0];
    if (!allowedTop.has(top)) {
      violations.push(`unexpected top-level directory: ${path}`);
      continue;
    }
    const dot = path.lastIndexOf(".");
    const ext = dot >= 0 ? path.slice(dot).toLowerCase() : "";
    if (backendRoots.some((root) => path === root || path.startsWith(`${root}/`))
        && backendCodeExt.has(ext) && ext !== ".rs") {
      violations.push(`non-Rust backend source: ${path}`);
    }
    if (path.startsWith("apps/") && frontendCodeExt.has(ext)) {
      if (!path.startsWith("apps/ui/")) violations.push(`frontend code outside apps/ui: ${path}`);
      else if (![".ts",".tsx"].includes(ext)) violations.push(`non-TypeScript UI source: ${path}`);
    }
  }
  return violations;
}

function productionDonorDependencyViolations(repoDir) {
  const productionRoots = ["core/","runtime/","adapter/","organism/","apps/"];
  const codeExt = new Set([".rs",".ts",".tsx",".js",".jsx",".mjs",".cjs",".py",".go",".java",".kt",".c",".cc",".cpp"]);
  const tracked = git(repoDir, ["ls-files"]).split("\n").filter(Boolean);
  const violations = [];
  for (const path of tracked) {
    const normalized = path.replaceAll("\\", "/");
    if (!productionRoots.some((root) => normalized.startsWith(root))) continue;
    const dot = normalized.lastIndexOf(".");
    const ext = dot >= 0 ? normalized.slice(dot).toLowerCase() : "";
    if (!codeExt.has(ext)) continue;
    let content = readFileSync(join(repoDir, normalized), "utf8");
    content = content.split("\n").map((line) => {
      const index = line.indexOf("//");
      return index === -1 ? line : line.slice(0, index);
    }).join("\n");
    if (content.includes("temporary/donors/")) violations.push(normalized);
  }
  return violations;
}

function assertDonorCensusComplete(repoDir, task) {
  if (!task.donor_id) throw new Error("donor_id required");
  const mirror = JSON.parse(readFileSync(join(repoDir, "chronica-mirror.json"), "utf8"));
  const donor = (mirror.donors ?? []).find((row) => row.id === task.donor_id);
  if (!donor) throw new Error(`donor ${task.donor_id} missing from chronica-mirror.json`);
  if (donor.root !== task.donor_source_path) {
    throw new Error(`donor root mismatch: ${donor.root} != ${task.donor_source_path}`);
  }
  if (donor.census_complete !== true) {
    throw new Error(`DONOR_CENSUS incomplete for ${task.donor_id}`);
  }
  return donor;
}

function buildPrompt(task, baseSha, executionClass) {
  const example = {
    status: "COMPLETE | BLOCKED | FAILED",
    tests: ["commands actually run"],
    evidence: ["concise evidence"],
    discovered_slices: executionClass === "READ_ONLY"
      ? [{
          id: "bounded-slice-id",
          behavior: "one bounded behavior",
          target_responsibility: "CORE | RUNTIME | ADAPTER | ORGANISM | APP | PRODUCT_LOCAL",
          owned_scope: ["repo paths"],
          proof_expectation: ["proof"],
          deletion_scope: ["donor paths"],
        }]
      : [],
    chronica_candidates: [],
    extinction: {
      applicable: false,
      files_removed: 0,
      remaining_scoped_files: null,
      state: "NOT_APPLICABLE | PROGRESSED | SOURCE_EXTINCT | CLASSIFIED_AND_REMOVED | BLOCKED",
      blocker: null,
      deleted_paths: [],
    },
    reset_evidence: {
      applicable: false,
      generation: null,
      pre_reset_head_sha: null,
      preservation_ref: null,
    },
  };

  return [
    "You are an isolated Chronica Fleet engineering worker.",
    `TASK_KIND: ${task.kind}`,
    `TARGET_REPO: ${task.target_repo}`,
    `TARGET_CELL_ID: ${task.target_cell_id ?? "null"}`,
    `TARGET_PACKAGE_ID: ${task.target_package_id ?? "null"}`,
    `TARGET_REF: ${task.target_ref}`,
    `EXACT_BASE_SHA: ${baseSha}`,
    `CHRONICA_REFERENCE_SHA: ${task.chronica_reference_sha ?? "null"}`,
    task.donor_source_path ? `DONOR_SOURCE_PATH: ${task.donor_source_path}` : "",
    task.candidate_sha ? `INPUT_CANDIDATE_SHA: ${task.candidate_sha}` : "",
    "",
    "Follow the assigned task exactly:",
    task.instructions,
    "",
    "Hard laws:",
    "- Permanent backend under core/runtime/adapter/organism is Rust only.",
    "- UI/frontend lives under apps/ui and is TypeScript only.",
    "- Cell production roots mirror Chronica: core/runtime/adapter/organism/graph/bindings/apps.",
    "- OSS source may exist only under temporary/donors/<DONOR_ID>/source and production must never import/link from it.",
    "- Feature implementation is forbidden without an allocated donor intake record for this task.",
    "- A Development Cell is an engineering boundary, not a product universe.",
    "- A package may be independently developed/tested, but canonical runtime is Chronica-required.",
    "- Do not create a second canonical World/Identity/Authority/Execution/Evidence/Memory/history/truth universe.",
    "- Do not treat Claude session state as truth.",
    "- Do not change unrelated paths.",
    "- Run relevant tests and inspect the actual Git diff.",
    executionClass === "READ_ONLY"
      ? "- READ-ONLY task: do not edit or commit."
      : "- Mutation task: make bounded changes and COMMIT before returning COMPLETE.",
    (["PACKAGE_VERIFY_INTEGRATE","CHRONICA_VERIFY_INTEGRATE"].includes(task.kind))
      ? `- Candidate ${task.candidate_sha} is pre-merged by the runner. Resolve conflicts if needed, verify independently, and commit only if proof passes.`
      : "",
    "",
    "Return ONLY one JSON object with these semantic fields. No markdown:",
    JSON.stringify(example, null, 2),
    "",
    "Do not invent Git SHAs or session IDs. The runner derives them independently.",
    ["PACKAGE_VERIFY_INTEGRATE"].includes(task.kind)
      ? "chronica_candidates may contain only reusable universal semantics. Each candidate must include affected_packages as [{repo,cell_id,package_id}]. Use canonical semantic_owner roots such as core/, runtime/, adapter/, organism/, apps/, graph/, bindings/. The runner overwrites source_cell_id/source_package_id/source_repo/source_repo_sha/chronica_reference_sha with measured values."
      : "chronica_candidates must be an empty array.",
  ].filter(Boolean).join("\n");
}

function normalizePayload(task, payload, structural) {
  const allowed = new Set(["COMPLETE", "BLOCKED", "FAILED"]);
  const status = allowed.has(payload.status) ? payload.status : "FAILED";
  const rawCandidates = ["PACKAGE_VERIFY_INTEGRATE"].includes(task.kind) && Array.isArray(payload.chronica_candidates)
    ? payload.chronica_candidates
    : [];
  const candidates = rawCandidates.map((candidate) => ({
    ...candidate,
    schema: "chronica.system-atlas.chronica-candidate.v1",
    source_cell_id: task.target_cell_id,
    source_package_id: task.target_package_id,
    source_repo: task.target_repo,
    source_repo_sha: structural.candidateSha,
    chronica_reference_sha: task.chronica_reference_sha,
    donor_ids: Array.isArray(candidate.donor_ids) ? candidate.donor_ids : (task.donor_id ? [task.donor_id] : []),
    affected_packages: Array.isArray(candidate.affected_packages) ? candidate.affected_packages : [],
    // Compatibility aliases retained for downstream consumers still migrating naming.
    source_ops_repo: task.target_repo,
    source_ops_sha: structural.candidateSha,
    affected_ops: Array.isArray(candidate.affected_packages) ? candidate.affected_packages.map((row) => row.repo) : [],
    evidence: Array.isArray(candidate.evidence) ? candidate.evidence.map(String) : [],
  }));

  return {
    schema: "chronica.system-atlas.fleet-worker-result.v1",
    task_id: task.id,
    provider: "CLAUDE",
    status,
    target_repo: task.target_repo,
    target_cell_id: task.target_cell_id ?? null,
    target_package_id: task.target_package_id ?? null,
    base_sha: structural.baseSha,
    candidate_sha: status === "COMPLETE" ? structural.candidateSha : null,
    thread_id: structural.sessionId ?? null,
    tests: Array.isArray(payload.tests) ? payload.tests.map(String) : [],
    evidence: [
      ...(Array.isArray(payload.evidence) ? payload.evidence.map(String) : []),
      ...structural.runnerEvidence,
    ],
    discovered_slices: task.kind === "DONOR_ABSORB_SCOUT" && Array.isArray(payload.discovered_slices)
      ? payload.discovered_slices
      : [],
    chronica_candidates: candidates,
    extinction: structural.extinction ?? {
      applicable: false,
      files_removed: 0,
      remaining_scoped_files: null,
      state: "NOT_APPLICABLE",
      blocker: null,
      deleted_paths: [],
    },
    reset_evidence: structural.resetEvidence ?? {
      applicable: false,
      generation: null,
      pre_reset_head_sha: null,
      preservation_ref: null,
    },
  };
}

function blockedResult(task, baseSha, evidence) {
  return normalizePayload(task, {
    status: "BLOCKED",
    tests: [],
    evidence,
    discovered_slices: [],
    chronica_candidates: [],
  }, {
    baseSha,
    candidateSha: null,
    sessionId: null,
    runnerEvidence: [],
    extinction: {
      applicable: ["PACKAGE_BUILD"].includes(task.kind),
      files_removed: 0,
      remaining_scoped_files: null,
      state: ["PACKAGE_BUILD"].includes(task.kind) ? "BLOCKED" : "NOT_APPLICABLE",
      blocker: ["PACKAGE_BUILD"].includes(task.kind) ? evidence.join("; ") : null,
      deleted_paths: [],
    },
    resetEvidence: {
      applicable: false,
      generation: null,
      pre_reset_head_sha: null,
      preservation_ref: null,
    },
  });
}

function main(taskPath) {
  const task = JSON.parse(readFileSync(resolve(taskPath), "utf8"));
  const executionClass = taskExecutionClass(task.kind);
  const claudeBin = process.env.CHRONICA_CLAUDE_BIN || "claude";
  const ghBin = process.env.CHRONICA_GH_BIN || "gh";

  if (!commandExists("git")) throw new Error("git is required");
  if (!commandExists(ghBin)) throw new Error(`${ghBin} is required and must be authenticated`);
  if (!["CELL_RESET_BASELINE","DONOR_INTAKE"].includes(task.kind) && !commandExists(claudeBin)) {
    throw new Error(`${claudeBin} is required and must be authenticated`);
  }

  const root = process.env.CHRONICA_FLEET_WORK_ROOT || tmpdir();
  const workDir = mkdtempSync(join(root, "chronica-fleet-claude-"));
  const repoDir = join(workDir, "repo");

  try {
    run(ghBin, ["repo", "clone", task.target_repo, repoDir, "--", "--no-tags"]);
    const currentTargetHead = resolveTargetHead(repoDir, task.target_ref);
    const baseSha = task.base_sha ?? currentTargetHead;

    if (task.base_sha && task.base_sha !== currentTargetHead && executionClass !== "CANDIDATE_ONLY") {
      process.stdout.write(`${JSON.stringify(blockedResult(task, currentTargetHead, [
        `STALE_TARGET_HEAD expected=${task.base_sha} current=${currentTargetHead}`,
      ]))}\n`);
      return;
    }

    git(repoDir, ["checkout", "--detach", baseSha]);
    const branchName = `fleet/${safe(task.id)}-${baseSha.slice(0, 8)}-${Date.now()}`;
    if (executionClass !== "READ_ONLY") git(repoDir, ["checkout", "-b", branchName]);

    if (task.kind === "CELL_RESET_BASELINE") {
      const reset = performCellReset(repoDir, task, baseSha, branchName);
      const runnerEvidence = [
        `work_branch=${branchName}`,
        `work_head=${reset.head}`,
        `preservation_ref=${reset.preservationRef}`,
      ];
      const payload = { status: "COMPLETE", tests: ["deterministic Cell reset"], evidence: ["CLEAN_PACKAGE_BASELINE"], discovered_slices: [], chronica_candidates: [] };
      try {
        git(repoDir, ["push","origin",`HEAD:refs/heads/${task.target_ref}`]);
        const remoteAfter = resolveTargetHead(repoDir, task.target_ref);
        if (remoteAfter !== reset.head) throw new Error(`remote target mismatch ${remoteAfter} != ${reset.head}`);
        process.stdout.write(`${JSON.stringify(normalizePayload(task, payload, {
          baseSha,
          candidateSha: remoteAfter,
          sessionId: null,
          runnerEvidence: [...runnerEvidence, `integrated_target=${task.target_ref}`],
          resetEvidence: {
            applicable: true,
            generation: reset.generation,
            pre_reset_head_sha: baseSha,
            preservation_ref: reset.preservationRef,
          },
        }))}\n`);
        return;
      } catch (error) {
        git(repoDir, ["push","-u","origin",`HEAD:refs/heads/${branchName}`]);
        process.stdout.write(`${JSON.stringify(normalizePayload(task, { ...payload, status: "BLOCKED", evidence: [`TARGET_INTEGRATION_BLOCKED ${error.message}`] }, {
          baseSha,
          candidateSha: null,
          sessionId: null,
          runnerEvidence: [...runnerEvidence, `preserved_candidate_branch=${branchName}`],
          resetEvidence: {
            applicable: true,
            generation: reset.generation,
            pre_reset_head_sha: baseSha,
            preservation_ref: reset.preservationRef,
          },
        }))}\n`);
        return;
      }
    }

    if (task.kind === "DONOR_INTAKE") {
      const intake = performDonorIntake(repoDir, workDir, task, ghBin);
      const runnerEvidence = [
        `work_branch=${branchName}`,
        `work_head=${intake.head}`,
        `donor_revision=${intake.upstreamSha}`,
        `donor_baseline_sha=${intake.donorBaselineSha}`,
        `donor_source_path=${task.donor_source_path}`,
        `donor_tracked_files_total=${intake.trackedFilesTotal}`,
      ];
      const payload = {
        status: "COMPLETE",
        tests: ["deterministic donor intake"],
        evidence: ["DONOR_INTAKE_PINNED"],
        discovered_slices: [],
        chronica_candidates: [],
      };
      try {
        git(repoDir, ["push","origin",`HEAD:refs/heads/${task.target_ref}`]);
        const remoteAfter = resolveTargetHead(repoDir, task.target_ref);
        if (remoteAfter !== intake.head) throw new Error(`remote target mismatch ${remoteAfter} != ${intake.head}`);
        process.stdout.write(`${JSON.stringify(normalizePayload(task, payload, {
          baseSha,
          candidateSha: remoteAfter,
          sessionId: null,
          runnerEvidence: [...runnerEvidence, `integrated_target=${task.target_ref}`],
        }))}\n`);
        return;
      } catch (error) {
        git(repoDir, ["push","-u","origin",`HEAD:refs/heads/${branchName}`]);
        process.stdout.write(`${JSON.stringify(normalizePayload(task, { ...payload, status: "BLOCKED", evidence: [`TARGET_INTEGRATION_BLOCKED ${error.message}`] }, {
          baseSha,
          candidateSha: null,
          sessionId: null,
          runnerEvidence: [...runnerEvidence, `preserved_candidate_branch=${branchName}`],
        }))}\n`);
        return;
      }
    }

    if ((["PACKAGE_VERIFY_INTEGRATE","CHRONICA_VERIFY_INTEGRATE"].includes(task.kind)) && task.candidate_sha) {
      git(repoDir, ["fetch", "origin", "--prune", "+refs/heads/*:refs/remotes/origin/*"]);
      git(repoDir, ["cat-file", "-e", `${task.candidate_sha}^{commit}`]);
      const merge = spawnSync("git", ["merge", "--no-ff", "--no-commit", task.candidate_sha], {
        cwd: repoDir,
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
      });
      if (merge.status !== 0 && !existsSync(join(repoDir, ".git", "MERGE_HEAD"))) {
        throw new Error(`candidate merge setup failed: ${(merge.stderr || merge.stdout || "").trim()}`);
      }
    }

    if (["DONOR_ABSORB_SCOUT","PACKAGE_BUILD","DONOR_ABSORB_BUILD"].includes(task.kind)) {
      if (!task.donor_id || !task.donor_source_path) {
        throw new Error(`${task.kind} requires donor_id and donor_source_path`);
      }
      const expectedDonorRoot = `temporary/donors/${task.donor_id}/source`;
      if (task.donor_source_path !== expectedDonorRoot) {
        throw new Error(`${task.kind} donor_source_path must be ${expectedDonorRoot}`);
      }
      if (!existsSync(join(repoDir, task.donor_source_path))) {
        throw new Error(`NO_OSS_NO_CODE: donor source not present at ${task.donor_source_path}`);
      }
      if (["PACKAGE_BUILD"].includes(task.kind)) {
        assertDonorCensusComplete(repoDir, task);
        const scopes = Array.isArray(task.deletion_scope) ? task.deletion_scope : [];
        if (!scopes.length || scopes.some((scope) => !underScope(scope, task.donor_source_path))) {
          throw new Error(`PACKAGE_BUILD deletion_scope must stay inside ${task.donor_source_path}`);
        }
      }
    }

    const prompt = buildPrompt(task, baseSha, executionClass);
    const maxTurns = process.env.CHRONICA_FLEET_CLAUDE_MAX_TURNS || "24";
    const permissionMode = executionClass === "READ_ONLY" ? "plan" : "acceptEdits";
    const claudeOut = run(claudeBin, [
      "-p", prompt,
      "--output-format", "json",
      "--max-turns", maxTurns,
      "--permission-mode", permissionMode,
    ], { cwd: repoDir });

    const envelope = JSON.parse(claudeOut);
    const payload = parseClaudePayload(envelope.result);

    if (executionClass === "READ_ONLY") {
      if (task.kind === "DONOR_CENSUS" && payload.status === "COMPLETE") {
      assertDonorCensusComplete(repoDir, task);
    }

    const status = git(repoDir, ["status", "--porcelain"]);
      const head = git(repoDir, ["rev-parse", "HEAD"]);
      if (status || head !== baseSha) throw new Error("read-only scout mutated repository state");
      process.stdout.write(`${JSON.stringify(normalizePayload(task, payload, {
        baseSha,
        candidateSha: null,
        sessionId: envelope.session_id ?? null,
        runnerEvidence: [`git_head=${head}`],
      }))}\n`);
      return;
    }

    const status = git(repoDir, ["status", "--porcelain"]);
    if (status) throw new Error(`Claude returned with uncommitted changes:\n${status}`);
    const head = git(repoDir, ["rev-parse", "HEAD"]);
    if (payload.status === "COMPLETE" && head === baseSha) {
      throw new Error("mutation task returned COMPLETE without a new Git commit");
    }

    if (payload.status === "COMPLETE" && ["PACKAGE_BUILD","DONOR_ABSORB_BUILD","PACKAGE_VERIFY_INTEGRATE","PACKAGE_RECONVERGE"].includes(task.kind)) {
      const shapeViolations = packageShapeViolations(changedPaths(repoDir, baseSha, head));
      if (shapeViolations.length) {
        throw new Error(`CELL_REPOSITORY_SHAPE_VIOLATION:\n${shapeViolations.join("\n")}`);
      }
      const donorDeps = productionDonorDependencyViolations(repoDir);
      if (donorDeps.length) {
        throw new Error(`PRODUCTION_DEPENDS_ON_DONOR_SOURCE:\n${donorDeps.join("\n")}`);
      }
    }

    let resultingSha = head;
    const runnerEvidence = [`work_branch=${branchName}`, `work_head=${head}`];
    const extinction = extinctionEvidence(repoDir, task, baseSha, head);

    if (payload.status === "COMPLETE" && ["PACKAGE_BUILD"].includes(task.kind)
        && extinction.files_removed <= 0 && !["SOURCE_EXTINCT","CLASSIFIED_AND_REMOVED"].includes(extinction.state)) {
      git(repoDir, ["push","-u","origin",`HEAD:refs/heads/${branchName}`]);
      const blocked = normalizePayload(task, {
        ...payload,
        status: "BLOCKED",
        evidence: [...(Array.isArray(payload.evidence) ? payload.evidence : []), extinction.blocker],
      }, {
        baseSha,
        candidateSha: null,
        sessionId: envelope.session_id ?? null,
        runnerEvidence: [...runnerEvidence, `preserved_candidate_branch=${branchName}`],
        extinction,
      });
      process.stdout.write(`${JSON.stringify(blocked)}\n`);
      return;
    }

    if (payload.status === "COMPLETE" && CANDIDATE_ONLY.has(task.kind)) {
      git(repoDir, ["push", "-u", "origin", `HEAD:refs/heads/${branchName}`]);
      runnerEvidence.push(`candidate_branch=${branchName}`);
    } else if (payload.status === "COMPLETE" && MUTATING_DIRECT.has(task.kind)) {
      try {
        git(repoDir, ["push", "origin", `HEAD:refs/heads/${task.target_ref}`]);
        const remoteAfter = resolveTargetHead(repoDir, task.target_ref);
        if (remoteAfter !== head) throw new Error(`remote target mismatch ${remoteAfter} != ${head}`);
        resultingSha = remoteAfter;
        runnerEvidence.push(`integrated_target=${task.target_ref}`);
      } catch (error) {
        git(repoDir, ["push", "-u", "origin", `HEAD:refs/heads/${branchName}`]);
        const blocked = normalizePayload(task, {
          ...payload,
          status: "BLOCKED",
          evidence: [
            ...(Array.isArray(payload.evidence) ? payload.evidence : []),
            `TARGET_INTEGRATION_BLOCKED ${error.message}`,
          ],
        }, {
          baseSha,
          candidateSha: null,
          sessionId: envelope.session_id ?? null,
          runnerEvidence: [...runnerEvidence, `preserved_candidate_branch=${branchName}`],
        });
        process.stdout.write(`${JSON.stringify(blocked)}\n`);
        return;
      }
    }

    process.stdout.write(`${JSON.stringify(normalizePayload(task, payload, {
      baseSha,
      candidateSha: resultingSha,
      sessionId: envelope.session_id ?? null,
      runnerEvidence,
      extinction,
    }))}\n`);
  } finally {
    if (process.env.CHRONICA_FLEET_KEEP_WORKDIR !== "1") {
      rmSync(workDir, { recursive: true, force: true });
    }
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const taskPath = process.argv[2];
    if (!taskPath) throw new Error("usage: claude-code-runner.mjs <task.json>");
    main(taskPath);
  } catch (error) {
    console.error(`claude-code-runner failed: ${error.message}`);
    process.exit(2);
  }
}
