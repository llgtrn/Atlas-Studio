#!/usr/bin/env node
// Real, non-test, build-time/audit caller for the chronica-platform-plane claim gate
// (docs/doctrines/008-doctrine-platform-replication-plane.md; docs/doctrines/023-five-dimension-cloud-shard-contract.md).
//
// For every `<name>.json` plan under tools/platform-plane/claims/, this pipes the plan into the
// *compiled* `chronica platform-claim` binary (crates/chronica-cli) and diffs the resulting
// PlatformReplicationReport against the committed `<name>.expected-report.json`. It never opens
// docs/capabilities.db or docs/architecture.db and never sets `ready: true` itself -- it only
// mechanically re-runs chronica_platform_plane::platform_replication_report against a checked-in
// plan and fails the build if the result silently drifts (whether from an edited plan, an edited
// claim-gate schema, or an accidental/unjustified flip toward `ready: true`).
//
// This is what gives the claim gate a genuine build-time caller distinct from its own
// #[cfg(test)] modules: this script is invoked as a real CI step (`pnpm platform:verify-claims`,
// wired into .github/workflows/rust.yml), not through `cargo test`.
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";

const claimsDir = path.join(process.cwd(), "tools/platform-plane/claims");
const EXPECTED_SUFFIX = ".expected-report.json";

const planFiles = readdirSync(claimsDir)
  .filter((f) => f.endsWith(".json") && !f.endsWith(EXPECTED_SUFFIX))
  .sort();

if (planFiles.length === 0) {
  console.error(
    "no platform-plane claim files found under tools/platform-plane/claims/ -- nothing to verify",
  );
  process.exit(1);
}

function runPlatformClaim(planJson) {
  try {
    const stdout = execFileSync(
      "cargo",
      ["run", "--quiet", "--locked", "-p", "chronica-cli", "--", "platform-claim"],
      { input: planJson, encoding: "utf8" },
    );
    return { code: 0, stdout, stderr: "" };
  } catch (err) {
    return {
      code: typeof err.status === "number" ? err.status : 2,
      stdout: err.stdout ?? "",
      stderr: err.stderr ?? "",
    };
  }
}

let failures = 0;

for (const planFile of planFiles) {
  const base = planFile.slice(0, -".json".length);
  const expectedPath = path.join(claimsDir, `${base}${EXPECTED_SUFFIX}`);

  let expected;
  try {
    expected = JSON.parse(readFileSync(expectedPath, "utf8"));
  } catch (err) {
    console.error(
      `FAIL ${planFile}: missing/invalid pinned expectation ${base}${EXPECTED_SUFFIX} (${err.message})`,
    );
    failures++;
    continue;
  }

  const planJson = readFileSync(path.join(claimsDir, planFile), "utf8");
  const { code, stdout, stderr } = runPlatformClaim(planJson);

  if (code === 2) {
    console.error(
      `FAIL ${planFile}: platform-claim rejected the plan or errored (exit 2): ${stderr || stdout}`,
    );
    failures++;
    continue;
  }

  const expectedExitCode = expected.ready ? 0 : 1;
  if (code !== expectedExitCode) {
    console.error(
      `FAIL ${planFile}: exit code ${code} does not match pinned expectation's exit code ${expectedExitCode} ` +
        `(expected.ready=${expected.ready}). This means the claim's readiness state changed -- update ` +
        `${base}${EXPECTED_SUFFIX} deliberately (with a reviewed reason) if this is a real, evidenced change.`,
    );
    failures++;
    continue;
  }

  const raw = (code === 0 ? stdout : stderr).trim();
  let actual;
  try {
    actual = JSON.parse(raw);
  } catch (err) {
    console.error(`FAIL ${planFile}: platform-claim output was not valid JSON: ${err.message}\n${raw}`);
    failures++;
    continue;
  }

  const actualNorm = JSON.stringify(actual);
  const expectedNorm = JSON.stringify(expected);
  if (actualNorm !== expectedNorm) {
    console.error(
      `FAIL ${planFile}: computed report does not match pinned ${base}${EXPECTED_SUFFIX}.\n` +
        `  actual:   ${actualNorm}\n` +
        `  expected: ${expectedNorm}`,
    );
    failures++;
    continue;
  }

  console.log(`OK ${planFile}: report matches pinned expectation (ready=${expected.ready})`);
}

if (failures > 0) {
  console.error(`\nplatform-plane claim gate FAILED: ${failures} of ${planFiles.length} claim(s) drifted`);
  process.exit(1);
}

console.log(`\nplatform-plane claim gate OK: ${planFiles.length} claim(s) verified`);
