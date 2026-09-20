#!/usr/bin/env node
// Deterministic intake for the 2026-09-20 sovereign-intelligence donor wave.
// Source snapshots are development evidence only. Production code must not depend on temporary/donors/**.
//
// Usage:
//   node tools/refoundation/intake-sovereign-intelligence-donors.mjs
//   node tools/refoundation/intake-sovereign-intelligence-donors.mjs --donor D340
//   node tools/refoundation/intake-sovereign-intelligence-donors.mjs --force
//
// This script pins exact upstream commits. It never tracks upstream HEAD implicitly.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));

export const SOVEREIGN_INTELLIGENCE_DONORS = [
  {
    id: "D144",
    repo: "kserve/kserve",
    sha: "fc48bbfe365f5fff029c555c8d1b6068c77e405f",
    destination: "temporary/donors/D144-kserve-kserve/source",
  },
  {
    id: "D174",
    repo: "data-privacy-stack/presidio",
    sha: "f251c513e8aad6820e9b2a44983ebf3a93637b07",
    destination: "temporary/donors/D174-data-privacy-stack-presidio/source",
  },
  {
    id: "D307",
    repo: "vllm-project/vllm",
    sha: "03f83013872015f6addb66954e2752d2d9bccd96",
    destination: "temporary/donors/D307-vllm-project-vllm/source",
  },
  {
    id: "D340",
    repo: "BerriAI/litellm",
    sha: "4011367b396cc0deb29752044bc4caf953000d2f",
    destination: "temporary/donors/D340-berriai-litellm/source",
  },
  {
    id: "D341",
    repo: "theagentrouter/agent-router",
    sha: "866ccb60abdc60685aeff686ff9e6199873633db",
    destination: "temporary/donors/D341-theagentrouter-agent-router/source",
  },
  {
    id: "D342",
    repo: "sgl-project/sglang",
    sha: "df0dc44931433b8a3488fbd4c6489e08cdee8703",
    destination: "temporary/donors/D342-sgl-project-sglang/source",
  },
  {
    id: "D343",
    repo: "open-inference/open-inference-protocol",
    sha: "dca50b7ec50db5013d04e168f6deb84134eb03df",
    destination: "temporary/donors/D343-open-inference-open-inference-protocol/source",
  },
  {
    id: "D344",
    repo: "cedar-policy/cedar",
    sha: "9502ae02564a23028c732f8c1f2311635394c34f",
    destination: "temporary/donors/D344-cedar-policy-cedar/source",
  },
  {
    id: "D345",
    repo: "gramineproject/gramine",
    sha: "ff71d7afea730dffd56a97af39bb6a73ee6c7662",
    destination: "temporary/donors/D345-gramineproject-gramine/source",
  },
];

function run(cmd, args, cwd = ROOT) {
  return execFileSync(cmd, args, {
    cwd,
    stdio: "inherit",
    env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
  });
}

function archiveSnapshot(donor, force = false) {
  const destination = resolve(ROOT, donor.destination);
  if (existsSync(destination)) {
    if (!force) {
      console.log(`${donor.id}: source already exists at ${donor.destination}; skip (use --force to replace)`);
      return;
    }
    rmSync(destination, { recursive: true, force: true });
  }

  const work = mkdtempSync(join(tmpdir(), `chronica-${donor.id}-`));
  const clone = join(work, "clone");
  const archive = join(work, "source.tar");

  try {
    run("git", ["init", clone]);
    run("git", ["-C", clone, "remote", "add", "origin", `https://github.com/${donor.repo}.git`]);
    run("git", ["-C", clone, "fetch", "--depth=1", "origin", donor.sha]);
    const resolved = execFileSync("git", ["-C", clone, "rev-parse", "FETCH_HEAD"], { encoding: "utf8" }).trim();
    if (resolved !== donor.sha) {
      throw new Error(`${donor.id}: expected ${donor.sha}, fetched ${resolved}`);
    }

    run("git", ["-C", clone, "archive", "--format=tar", "--output", archive, donor.sha]);
    mkdirSync(destination, { recursive: true });
    run("tar", ["-xf", archive, "-C", destination]);

    console.log(`${donor.id}: ingested ${donor.repo}@${donor.sha} -> ${donor.destination}`);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

function main() {
  const args = process.argv.slice(2);
  const force = args.includes("--force");
  const donorFlag = args.indexOf("--donor");
  const requested = donorFlag >= 0 ? args[donorFlag + 1] : null;
  if (donorFlag >= 0 && !requested) throw new Error("--donor requires an ID");

  const selected = requested
    ? SOVEREIGN_INTELLIGENCE_DONORS.filter((d) => d.id === requested)
    : SOVEREIGN_INTELLIGENCE_DONORS;

  if (requested && selected.length === 0) throw new Error(`unknown donor ${requested}`);
  for (const donor of selected) archiveSnapshot(donor, force);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
