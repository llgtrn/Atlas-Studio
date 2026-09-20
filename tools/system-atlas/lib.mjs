import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

export const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");

export function repoRel(p) {
  return relative(REPO_ROOT, p).split(sep).join("/");
}

export function crateLayer(manifestPath) {
  const path = repoRel(manifestPath);
  // 2026-09-16 hard refoundation: crates/{core,cap,adapter,runtime,obs}/*/Cargo.toml no longer
  // exists (crates/ is retired -- see docs/decisions/0016-legacy-backend-retired-to-git-
  // history.md). core/, runtime/, adapter/, organism/ are top-level single-package crates now,
  // each owning its own Cargo.toml directly, not nested under a crates/<layer>/ wrapper.
  const match = /^(core|runtime|adapter|organism)\/Cargo\.toml$/.exec(path);
  return match?.[1] ?? null;
}

export function loadCargoMetadata() {
  return JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      maxBuffer: 128 * 1024 * 1024,
    }),
  );
}

// Builds the full intra-workspace dependency graph (not just forbidden-layer
// edges): nodes keyed by crate name, edges as {from, to}.
export function buildWorkspaceGraph(metadata) {
  const workspaceIds = new Set(metadata.workspace_members);
  const packages = metadata.packages.filter((pkg) => workspaceIds.has(pkg.id));
  const byName = new Map(packages.map((pkg) => [pkg.name, pkg]));

  const nodes = new Map();
  for (const pkg of packages) {
    const manifestDir = dirname(pkg.manifest_path);
    nodes.set(pkg.name, {
      name: pkg.name,
      layer: crateLayer(pkg.manifest_path),
      manifestPath: repoRel(pkg.manifest_path),
      dir: repoRel(manifestDir),
      targets: pkg.targets.map((t) => ({ name: t.name, kinds: t.kind })),
      hasBin: pkg.targets.some((t) => t.kind.includes("bin")),
      hasLib: pkg.targets.some((t) => t.kind.includes("lib") || t.kind.includes("rlib")),
      hasExample: pkg.targets.some((t) => t.kind.includes("example")),
      hasTest: pkg.targets.some((t) => t.kind.includes("test")),
      hasBuildScript: pkg.targets.some((t) => t.kind.includes("custom-build")),
      features: Object.keys(pkg.features ?? {}),
      hasChronicaDescriptor: hasDescriptor(manifestDir),
    });
  }

  const edges = [];
  const seen = new Set();
  for (const pkg of packages) {
    for (const dep of pkg.dependencies) {
      const target = byName.get(dep.name);
      if (!target) continue; // external dependency, not a workspace edge
      const key = `${pkg.name}->${target.name}`;
      if (seen.has(key)) continue;
      seen.add(key);
      edges.push({ from: pkg.name, to: target.name });
    }
  }

  return { nodes, edges };
}

function hasDescriptor(manifestDir) {
  try {
    const chronicaDir = join(manifestDir, ".chronica");
    return statSync(chronicaDir).isDirectory() && readdirSync(chronicaDir).some((f) => f.endsWith(".chronica.json"));
  } catch {
    return false;
  }
}

export function fanInOut(nodeNames, edges) {
  const fanin = new Map(nodeNames.map((n) => [n, 0]));
  const fanout = new Map(nodeNames.map((n) => [n, 0]));
  for (const { from, to } of edges) {
    if (fanout.has(from)) fanout.set(from, fanout.get(from) + 1);
    if (fanin.has(to)) fanin.set(to, fanin.get(to) + 1);
  }
  return { fanin, fanout };
}

export function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

export function nowIso() {
  return new Date().toISOString();
}

export function gitHeadSha() {
  return execFileSync("git", ["rev-parse", "HEAD"], { cwd: REPO_ROOT, encoding: "utf8" }).trim();
}

// atlas_generation_sha: the git SHA of the Atlas tooling/docs commit at the
// moment a given file was written. This is gitHeadSha() -- it moves with
// every Atlas-only commit and is expected to differ across files generated
// at different points in the campaign.
export const atlasGenerationSha = gitHeadSha;

// source_base_sha: the exact product-code snapshot every fact in the Atlas
// was derived from. Defined as the most recent commit, reachable from `ref`
// (default HEAD), that changed anything OUTSIDE the Atlas's own tooling/docs
// trees. This must be IDENTICAL across every shard and aggregate in one
// generation -- unlike atlas_generation_sha, it does NOT move just because
// the Atlas tooling itself was committed again.
export function productSourceSha(ref = "HEAD") {
  return execFileSync(
    "git",
    ["log", "-1", "--format=%H", ref, "--", ".", ":!docs/_machine/system-atlas", ":!tools/system-atlas"],
    { cwd: REPO_ROOT, encoding: "utf8" },
  ).trim();
}

// canonical_ref_sha: the resolved SHA of a named ref (default "origin/main"),
// distinct from productSourceSha(). productSourceSha() is the last commit
// that actually changed product-relevant paths (what the Atlas's facts were
// derived from); canonical_ref_sha is simply "what does origin/main point at
// right now" -- e.g. a merge commit that is tree-identical to its own
// productSourceSha() for non-Atlas paths, as happened with 641fed38 vs
// 8fadb975 in this repo's history. Both are useful, different truths. Returns
// null (never throws) if the ref can't be resolved locally (e.g. no fetch yet).
export function canonicalRefSha(ref = "origin/main") {
  try {
    return execFileSync("git", ["rev-parse", "--verify", "-q", ref], { cwd: REPO_ROOT, encoding: "utf8" }).trim();
  } catch {
    return null;
  }
}
