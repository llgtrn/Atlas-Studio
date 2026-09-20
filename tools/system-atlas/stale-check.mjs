#!/usr/bin/env node
// stale-check.mjs — detects whether the System Atlas (docs/_machine/system-atlas/)
// is stale relative to the product code it claims to describe.
//
// "Product code" excludes the Atlas's own tooling/docs trees so that changes to
// the Atlas itself never look like product drift:
//   docs/_machine/system-atlas/  and  tools/system-atlas/
//
// Usage:
//   node tools/system-atlas/stale-check.mjs [--fetch]
//
// --fetch attempts `git fetch origin main` first (best-effort; network may be
// unavailable in this environment, which is reported, not treated as a crash).
//
// Read-only w.r.t. Atlas facts: this script writes exactly one file,
// docs/_machine/system-atlas/v0/staleness.json, and never touches shards,
// aggregates, or frontier files.

import { execFileSync } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { REPO_ROOT, canonicalRefSha, nowIso, readJson, repoRel } from "./lib.mjs";

// Pathspecs excluding the Atlas's own trees from a `git log`/`git diff` walk.
const PRODUCT_CODE_EXCLUDES = [".", ":!docs/_machine/system-atlas", ":!tools/system-atlas"];

const SNAPSHOT_PATH = join(REPO_ROOT, "docs/_machine/system-atlas/v0/snapshot.json");
const STALENESS_PATH = join(REPO_ROOT, "docs/_machine/system-atlas/v0/staleness.json");

/**
 * The canonical product-code SHA reachable from `ref`: the most recent commit
 * that changed anything outside docs/_machine/system-atlas/ and
 * tools/system-atlas/. Returns null if `ref` doesn't resolve, or if git
 * fails for any other reason (e.g. no matching commit reachable, corrupt repo).
 *
 * @param {string} [ref] git ref to walk from; omitted means HEAD.
 * @returns {string|null}
 */
export function productSourceSha(ref) {
  const args = ["log", "-1", "--format=%H"];
  if (ref) args.push(ref);
  args.push("--", ...PRODUCT_CODE_EXCLUDES);
  try {
    const out = execFileSync("git", args, { cwd: REPO_ROOT, encoding: "utf8" }).trim();
    return out || null;
  } catch {
    return null;
  }
}

/**
 * Whether `ref` names a commit git can currently resolve locally (no network).
 * @param {string} ref
 * @returns {boolean}
 */
export function refExists(ref) {
  try {
    execFileSync("git", ["rev-parse", "--verify", "-q", ref], {
      cwd: REPO_ROOT,
      encoding: "utf8",
      stdio: ["ignore", "ignore", "ignore"],
    });
    return true;
  } catch {
    return false;
  }
}

/**
 * `git diff --stat` between two product-code SHAs, capped to `maxLines`.
 * Never throws; a git failure is reported inline instead.
 *
 * @param {string} fromSha
 * @param {string} toSha
 * @param {number} [maxLines]
 */
export function diffStatSummary(fromSha, toSha, maxLines = 40) {
  if (!fromSha || !toSha || fromSha === toSha) return null;
  try {
    const out = execFileSync(
      "git",
      ["diff", "--stat", `${fromSha}..${toSha}`, "--", ...PRODUCT_CODE_EXCLUDES],
      { cwd: REPO_ROOT, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
    );
    const allLines = out.split("\n").filter((l) => l.length > 0);
    return {
      from: fromSha,
      to: toSha,
      lines: allLines.slice(0, maxLines),
      truncated: allLines.length > maxLines,
      total_lines: allLines.length,
    };
  } catch (err) {
    return { from: fromSha, to: toSha, error: String(err?.message ?? err) };
  }
}

/**
 * Pure classifier: given the recorded (Atlas-stamped), local (HEAD), and
 * remote (origin/main, or null if unknown) product-code SHAs, decide the
 * staleness status. No I/O.
 *
 * @param {{recorded: string|null, local: string|null, remote: string|null}} args
 * @returns {"FRESH"|"LOCAL_DRIFTED"|"REMOTE_AHEAD"|"DIVERGED"|"UNKNOWN"}
 */
export function classifyStaleness({ recorded, local, remote }) {
  if (!local || !recorded) return "UNKNOWN";

  const remoteKnown = Boolean(remote);

  // recorded matches the current checkout, and either we don't know what
  // remote thinks or remote agrees too.
  if (recorded === local && (!remoteKnown || remote === recorded)) return "FRESH";

  // All three known and pairwise distinct.
  if (remoteKnown && recorded !== local && local !== remote && recorded !== remote) {
    return "DIVERGED";
  }

  // Local checkout matches what the Atlas recorded, but origin/main moved on.
  if (remoteKnown && recorded === local && remote !== recorded) return "REMOTE_AHEAD";

  // Default: the current checkout has moved past what the Atlas recorded
  // (whether or not remote is known / agrees with local).
  return "LOCAL_DRIFTED";
}

/**
 * Human-readable next step for a given status.
 * @param {string} status
 * @param {{recorded: string|null, local: string|null, remote: string|null}} shas
 */
export function buildRemediation(status, { recorded, local, remote }) {
  switch (status) {
    case "FRESH":
      return `Atlas is fresh relative to product code at ${local}. No action needed.`;
    case "LOCAL_DRIFTED":
      return (
        `Product code on this checkout has moved to ${local} since the Atlas was generated ` +
        `at ${recorded}. Re-run the B1-B8 census/research passes against ${local} and ` +
        `regenerate aggregates before trusting current Atlas facts.`
      );
    case "REMOTE_AHEAD":
      return (
        `origin/main has moved to ${remote} since the Atlas was generated at ${recorded} ` +
        `(this checkout is still at ${local}, matching the recorded snapshot). Sync to main, ` +
        `then re-run the B1-B8 census/research passes against ${remote} and regenerate aggregates.`
      );
    case "DIVERGED":
      return (
        `Local (${local}), origin/main (${remote}), and the Atlas's recorded snapshot ` +
        `(${recorded}) are all different. Reconcile local vs. remote drift first, then re-run ` +
        `the B1-B8 census/research passes against the agreed-upon target SHA and regenerate aggregates.`
      );
    case "UNKNOWN":
    default:
      return (
        `Could not fully determine staleness (recorded=${recorded ?? "null"}, local=${local ?? "null"}, ` +
        `remote=${remote ?? "null"}). Investigate manually before trusting this report.`
      );
  }
}

/**
 * Reads the recorded product-code SHA out of snapshot.json, preferring the
 * new `source_base_sha` field and falling back to the legacy `atlas_base_sha`
 * field (noting the fallback) if the coordinator hasn't migrated it yet.
 *
 * @param {string} snapshotPath
 * @returns {{recorded: string|null, fieldUsed: string|null, deprecationNote: string|null}}
 */
export function readRecordedSourceBaseSha(snapshotPath) {
  const snapshot = readJson(snapshotPath);
  if (typeof snapshot.source_base_sha === "string" && snapshot.source_base_sha) {
    return { recorded: snapshot.source_base_sha, fieldUsed: "source_base_sha", deprecationNote: null };
  }
  if (typeof snapshot.atlas_base_sha === "string" && snapshot.atlas_base_sha) {
    return {
      recorded: snapshot.atlas_base_sha,
      fieldUsed: "atlas_base_sha",
      deprecationNote:
        "snapshot.json has not yet been migrated to 'source_base_sha'; fell back to the legacy " +
        "'atlas_base_sha' field. Re-run stale-check.mjs once the migration lands.",
    };
  }
  return {
    recorded: null,
    fieldUsed: null,
    deprecationNote:
      "Neither 'source_base_sha' nor legacy 'atlas_base_sha' found on snapshot.json; recorded SHA is UNKNOWN.",
  };
}

async function main() {
  const args = process.argv.slice(2);
  const doFetch = args.includes("--fetch");

  let remoteStatus = "OK";

  if (doFetch) {
    try {
      execFileSync("git", ["fetch", "origin", "main"], {
        cwd: REPO_ROOT,
        stdio: ["ignore", "pipe", "pipe"],
      });
    } catch (err) {
      remoteStatus = "UNKNOWN_NETWORK_UNAVAILABLE";
      console.error(`[stale-check] git fetch origin main failed (continuing best-effort): ${err?.message ?? err}`);
    }
  }

  const localProductSha = productSourceSha();

  let remoteProductSha = null;
  if (remoteStatus === "OK") {
    if (refExists("origin/main")) {
      remoteProductSha = productSourceSha("origin/main");
      if (remoteProductSha === null) remoteStatus = "UNKNOWN_NO_PRODUCT_COMMITS_ON_REMOTE";
    } else {
      remoteStatus = "UNKNOWN_NO_REMOTE_REF";
    }
  }

  if (!existsSync(SNAPSHOT_PATH)) {
    console.log(
      `[stale-check] no snapshot.json yet at ${repoRel(SNAPSHOT_PATH)} -- nothing to check yet.`,
    );
    process.exit(0);
  }

  const { recorded, fieldUsed, deprecationNote } = readRecordedSourceBaseSha(SNAPSHOT_PATH);
  if (deprecationNote) console.error(`[stale-check] ${deprecationNote}`);

  const remoteForClassification = remoteStatus === "OK" ? remoteProductSha : null;

  const status = classifyStaleness({
    recorded,
    local: localProductSha,
    remote: remoteForClassification,
  });

  let changedPathsSummary = null;
  if (status === "LOCAL_DRIFTED") {
    changedPathsSummary = { vs_local: diffStatSummary(recorded, localProductSha) };
  } else if (status === "REMOTE_AHEAD") {
    changedPathsSummary = { vs_remote: diffStatSummary(recorded, remoteProductSha) };
  } else if (status === "DIVERGED") {
    changedPathsSummary = {
      vs_local: diffStatSummary(recorded, localProductSha),
      vs_remote: diffStatSummary(recorded, remoteProductSha),
    };
  }

  const remediation = buildRemediation(status, {
    recorded,
    local: localProductSha,
    remote: remoteForClassification,
  });

  const report = {
    schema: "chronica.system-atlas.staleness.v0",
    checked_at: nowIso(),
    recorded_source_base_sha: recorded,
    recorded_source_base_sha_field: fieldUsed,
    local_product_sha: localProductSha,
    remote_product_sha: remoteProductSha,
    remote_status: remoteStatus,
    canonical_ref_sha: canonicalRefSha("origin/main"),
    status,
    changed_paths_summary: changedPathsSummary,
    remediation,
  };

  writeFileSync(STALENESS_PATH, JSON.stringify(report, null, 2) + "\n", "utf8");

  console.log(`[stale-check] status: ${status}`);
  console.log(`[stale-check] recorded_source_base_sha: ${recorded ?? "UNKNOWN"} (field: ${fieldUsed ?? "none"})`);
  console.log(`[stale-check] local_product_sha:        ${localProductSha ?? "UNKNOWN"}`);
  console.log(`[stale-check] remote_product_sha:        ${remoteProductSha ?? "UNKNOWN"} (${remoteStatus})`);
  console.log(`[stale-check] remediation: ${remediation}`);
  console.log(`[stale-check] wrote ${repoRel(STALENESS_PATH)}`);

  const failStatuses = new Set(["LOCAL_DRIFTED", "REMOTE_AHEAD", "DIVERGED"]);
  process.exit(failStatuses.has(status) ? 1 : 0);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
