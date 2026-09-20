#!/usr/bin/env node
/**
 * Enforce the refounded workspace's dependency direction and physical substrate.
 *
 * 2026-09-16 hard refoundation: the pre-reset crates/{core,cap,adapter,runtime,infra}/ workspace
 * is retired (see docs/decisions/0016-legacy-backend-retired-to-git-history.md). The canonical
 * Rust backend is now exactly four top-level single-package-crate roots: core/, runtime/,
 * adapter/, organism/ (AGENTS.md section 4). There is no more cap tier (absorbed into
 * core/runtime/adapter) and no more infra tier (dev/build tooling is no longer a firewalled
 * workspace member). The checked direction, taken verbatim from each root's own Cargo.toml module
 * doc comment: core depends on nothing else in the product graph; runtime depends only on core;
 * adapter depends on core, runtime, and organism (implementing organism-owned analytical ports,
 * never the reverse); organism depends on core and runtime, never the reverse, and never on
 * adapter (organism has no direct effect path -- see this doc's 2026-09-17 note below). Existing
 * violations are recorded as debt, not silently blessed: an exact edge baseline prevents new
 * violations and becomes stale when an edge is repaired.
 *
 * Product/domain silos must not become a fifth physical crate root either -- a workspace crate
 * proposed for its own root (e.g. `web2app/`) is rejected outright, with NO carve-out list: every
 * crate gets physically converged onto core/runtime/adapter/organism by actual
 * responsibility/dependency direction instead, never an exception line.
 *
 * Dev-dependencies are excluded from the direction check: a dev-only edge never reaches a
 * production binary's dependency graph (cargo strips dev-dependencies from anything that depends
 * on the crate), so a lower-tier crate's own test suite dev-depending on a higher-tier crate for
 * genuine end-to-end interop testing is not the same hazard this gate exists to catch. Only
 * `dependency.kind === null` (an ordinary [dependencies] edge) is checked.
 *
 * 2026-09-17 dependency-inversion correction: `adapter -> organism` is now the intended
 * direction, not a forbidden one. `organism::WorldModel` (docs/architecture/intelligence/
 * world-model.md) is a provider-neutral ANALYZE port that some concrete model-family/provider
 * mechanics must implement; that doc's own runtime-mapping table (section 10) assigns exactly
 * that mechanics to `crates/adapter`. Before this, `adapter` could depend on `organism`'s
 * vocabulary but never actually implement its trait, because doing so required this very edge.
 * The critical invariant this gate exists to protect is unchanged and still absolute:
 * `organism -> adapter` remains HARD forbidden -- organism must never gain a direct effect path,
 * only ever reaching a provider through runtime's governed execution spine. `adapter -> organism`
 * is the opposite direction (an outer provider implementation depending on an inner semantic
 * port, i.e. ports-and-adapters/dependency inversion) and grants organism no new capability at
 * all; organism's own code is unchanged and still cannot see `adapter` exists.
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const DEFAULT_BASELINE = resolve(REPO_ROOT, "tools/refoundation/refoundation-layer-debt.json");
// 2026-09-16 hard refoundation: crates/{core,cap,adapter,runtime,infra}/ no longer exists
// (crates/ is retired -- see docs/decisions/0016-legacy-backend-retired-to-git-history.md and
// AGENTS.md section 4). core/, runtime/, adapter/, organism/ are top-level single-package crates
// now, each owning its own Cargo.toml directly. There is no more cap/infra tier: `cap` was
// absorbed into core/runtime/adapter during the refoundation, and infra (dev/build tooling) is no
// longer a firewalled workspace member at all. The forbidden direction below is taken verbatim
// from each root's own Cargo.toml module doc comment (core: no product deps; runtime: depends
// only on core; adapter: depends on core+runtime+organism, never the reverse; organism: depends
// on core+runtime, never the reverse, and never on adapter). `adapter` has no entry here at
// all -- see this file's own module doc's 2026-09-17 note: adapter implementing organism-owned
// ports (e.g. `organism::world_model::WorldModel`) means `adapter -> organism` is the INTENDED
// direction, not a forbidden one, so adapter has nothing left in this workspace to forbid.
const FORBIDDEN_TARGETS = {
  core: new Set(["runtime", "adapter", "organism"]),
  runtime: new Set(["adapter", "organism"]),
  // The critical invariant: organism must never gain a direct effect path. Organism may only
  // ever reach a provider through runtime's governed execution spine, never by depending on
  // adapter/ directly -- this is that rule's mechanically-checkable form.
  organism: new Set(["adapter"]),
};
// No ALLOWED_SUBSYSTEM_EXCEPTIONS carve-out list: every crate proposed for a non-canonical
// product root gets physically converged onto core/runtime/adapter/organism by actual dependency
// direction instead -- see the module doc above.
const FORBIDDEN_SUBSYSTEM_ROOTS = ["crates/web2app/"];

function repoPath(path, repoRoot = REPO_ROOT) {
  return relative(repoRoot, path).split(sep).join("/");
}

export function crateLayer(manifestPath, repoRoot = REPO_ROOT) {
  const path = repoPath(manifestPath, repoRoot);
  const match = /^(core|runtime|adapter|organism)\/Cargo\.toml$/.exec(path);
  return match?.[1] ?? null;
}

export function forbiddenSubsystemRoots(metadata, repoRoot = REPO_ROOT) {
  const workspaceIds = new Set(metadata.workspace_members);
  return metadata.packages
    .filter((pkg) => workspaceIds.has(pkg.id))
    .map((pkg) => repoPath(pkg.manifest_path, repoRoot))
    .filter((path) => FORBIDDEN_SUBSYSTEM_ROOTS.some((prefix) => path.startsWith(prefix)))
    .sort();
}

export function forbiddenEdges(metadata, repoRoot = REPO_ROOT) {
  const workspaceIds = new Set(metadata.workspace_members);
  const byName = new Map(
    metadata.packages
      .filter((pkg) => workspaceIds.has(pkg.id))
      .map((pkg) => [pkg.name, pkg]),
  );
  const edges = [];
  for (const source of byName.values()) {
    const from = crateLayer(source.manifest_path, repoRoot);
    if (!FORBIDDEN_TARGETS[from]) continue;
    for (const dependency of source.dependencies) {
      if (dependency.kind) continue; // dev/build dependencies never reach a production binary
      const target = byName.get(dependency.name);
      if (!target) continue;
      const to = crateLayer(target.manifest_path, repoRoot);
      if (FORBIDDEN_TARGETS[from].has(to)) {
        edges.push(`${source.name} (${from}) -> ${target.name} (${to})`);
      }
    }
  }
  return [...new Set(edges)].sort();
}

export function compareBaseline(actual, baseline) {
  const actualSet = new Set(actual);
  const baselineSet = new Set(baseline);
  return {
    newEdges: actual.filter((edge) => !baselineSet.has(edge)),
    repairedEdges: baseline.filter((edge) => !actualSet.has(edge)),
  };
}

function loadMetadata() {
  return JSON.parse(execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--no-deps"],
    { cwd: REPO_ROOT, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  ));
}

function main(args = process.argv.slice(2)) {
  const metadata = loadMetadata();
  const misplacedSubsystemCrates = forbiddenSubsystemRoots(metadata);
  for (const path of misplacedSubsystemCrates) {
    console.error(`FORBIDDEN SUBSYSTEM ROOT: ${path} (Web2App belongs in adapter/shared substrate, not a fifth crate layer)`);
  }
  if (misplacedSubsystemCrates.length) return 1;

  const actual = forbiddenEdges(metadata);
  if (args.includes("--write-baseline")) {
    writeFileSync(DEFAULT_BASELINE, `${JSON.stringify({
      schema: "chronica.refoundation.layer-debt.v1",
      rationale: "Exact task-start debt; entries are not architectural exceptions. Remove an entry when its edge is repaired.",
      forbidden_edges: actual,
    }, null, 2)}\n`);
    console.log(`wrote ${actual.length} forbidden edge(s) to ${relative(REPO_ROOT, DEFAULT_BASELINE)}`);
    return 0;
  }

  const baseline = JSON.parse(readFileSync(DEFAULT_BASELINE, "utf8"));
  if (baseline.schema !== "chronica.refoundation.layer-debt.v1"
      || !Array.isArray(baseline.forbidden_edges)) {
    console.error("invalid refoundation layer debt baseline");
    return 2;
  }
  const { newEdges, repairedEdges } = compareBaseline(actual, baseline.forbidden_edges);
  for (const edge of newEdges) console.error(`NEW FORBIDDEN EDGE: ${edge}`);
  for (const edge of repairedEdges) console.error(`STALE DEBT EDGE (remove from baseline): ${edge}`);
  if (newEdges.length || repairedEdges.length) return 1;
  console.log(`refoundation layer gate passed; no new edges (${actual.length} known debt edge(s) remain); no forbidden subsystem roots`);
  return 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exit(main());
