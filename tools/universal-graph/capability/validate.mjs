#!/usr/bin/env node
// CLI: normalize + schema-validate a raw capability observation input file.
//
// Usage:
//   node tools/universal-graph/capability/validate.mjs <input.json>
//   node tools/universal-graph/capability/validate.mjs <input.json> --check-determinism
//
// Exits 0 and prints the CapabilityRegistryFragment on success, serialized
// via canonicalPrettyStringify() (reused from D0's normalize.mjs). Exits 1
// and prints every schema/integrity violation on failure.
// --check-determinism re-runs normalization on the same input and fails if
// the two outputs differ.
//
// No side effect other than stdout/stderr and process exit -- this CLI
// never writes to any canonical store; it is a reference normalizer only.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { formatErrors } from "../schema-validate.mjs";
import { compileCapabilitySchemas, normalizeCapabilityRegistry, canonicalStringify, canonicalPrettyStringify } from "./normalize.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const D0_SCHEMA_DIR = resolve(HERE, "..", "..", "..", "docs", "_machine", "universal-graph", "v0", "schemas");
const CAPABILITY_SCHEMA_DIR = resolve(HERE, "..", "..", "..", "docs", "_machine", "universal-graph", "capability", "v0", "schemas");

function main() {
  const args = process.argv.slice(2);
  const inputPath = args.find((a) => !a.startsWith("--"));
  const checkDeterminism = args.includes("--check-determinism");

  if (!inputPath) {
    console.error("usage: validate.mjs <input.json> [--check-determinism]");
    process.exit(2);
  }

  const ajv = compileCapabilitySchemas([D0_SCHEMA_DIR, CAPABILITY_SCHEMA_DIR]);
  const raw = JSON.parse(readFileSync(inputPath, "utf8"));

  const first = normalizeCapabilityRegistry(raw, ajv);
  if (!first.valid) {
    console.error(`INVALID: ${inputPath}`);
    console.error(formatErrors(first.errors));
    process.exit(1);
  }

  if (checkDeterminism) {
    const second = normalizeCapabilityRegistry(raw, ajv);
    if (canonicalStringify(first.fragment) !== canonicalStringify(second.fragment)) {
      console.error(`NONDETERMINISTIC: ${inputPath} produced different output across two runs`);
      process.exit(1);
    }
  }

  console.log(canonicalPrettyStringify(first.fragment));
  process.exit(0);
}

main();
