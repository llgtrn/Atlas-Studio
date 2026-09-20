#!/usr/bin/env node
// CLI: evaluate one D3 capability-registration transition input file.
//
// Usage:
//   node tools/universal-graph/capability-registration/validate.mjs <input.json>
//   node tools/universal-graph/capability-registration/validate.mjs <input.json> --check-determinism
//
// <input.json> shape: { "untrusted": { "raw_capability_observation": ..., "request": ... },
//                        "trusted": { "canonical_world_snapshot": ..., "canonical_capability_registry_snapshot": ...,
//                                      "canonical_registration_authority_snapshot": ..., "authenticated_requester_context": ... } }
// Both wrappers are exact-shape: an unknown key on either is rejected
// (UNTRUSTED_CHANNEL_UNKNOWN_FIELD / TRUSTED_CHANNEL_UNKNOWN_FIELD), not silently dropped.
//
// Exits 0 for COMMIT/REPLAY/DENIED/ALREADY_REGISTERED and prints the
// CapabilityRegistrationTransitionResult via canonicalPrettyStringify(). Exits 1 for REJECTED
// and prints the machine-readable error(s). --check-determinism re-runs the same transition and
// fails if the two results differ.
//
// No side effect other than stdout/stderr and process exit -- this CLI never writes to any
// production store; it is a reference transition only.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { compileCapabilityRegistrationSchemas } from "./schema-validate.mjs";
import { evaluateCapabilityRegistrationTransition } from "./transition.mjs";
import { canonicalStringify, canonicalPrettyStringify } from "../normalize.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const D0_SCHEMA_DIR = resolve(HERE, "..", "..", "..", "docs", "_machine", "universal-graph", "v0", "schemas");
const ADMISSION_SCHEMA_DIR = resolve(HERE, "..", "..", "..", "docs", "_machine", "universal-graph", "admission", "v0", "schemas");
const CAPABILITY_SCHEMA_DIR = resolve(HERE, "..", "..", "..", "docs", "_machine", "universal-graph", "capability", "v0", "schemas");
const CAPABILITY_REGISTRATION_SCHEMA_DIR = resolve(HERE, "..", "..", "..", "docs", "_machine", "universal-graph", "capability-registration", "v0", "schemas");

function main() {
  const args = process.argv.slice(2);
  const inputPath = args.find((a) => !a.startsWith("--"));
  const checkDeterminism = args.includes("--check-determinism");

  if (!inputPath) {
    console.error("usage: validate.mjs <input.json> [--check-determinism]");
    process.exit(2);
  }

  const ajv = compileCapabilityRegistrationSchemas([D0_SCHEMA_DIR, ADMISSION_SCHEMA_DIR, CAPABILITY_SCHEMA_DIR, CAPABILITY_REGISTRATION_SCHEMA_DIR]);
  const input = JSON.parse(readFileSync(inputPath, "utf8"));

  const first = evaluateCapabilityRegistrationTransition(input.untrusted, input.trusted, ajv);

  if (checkDeterminism) {
    const second = evaluateCapabilityRegistrationTransition(input.untrusted, input.trusted, ajv);
    if (canonicalStringify(first) !== canonicalStringify(second)) {
      console.error(`NONDETERMINISTIC: ${inputPath} produced different results across two runs`);
      process.exit(1);
    }
  }

  if (first.status === "REJECTED") {
    console.error(`REJECTED: ${inputPath}`);
    console.error(canonicalPrettyStringify(first.errors));
    process.exit(1);
  }

  console.log(canonicalPrettyStringify(first));
  process.exit(0);
}

main();
