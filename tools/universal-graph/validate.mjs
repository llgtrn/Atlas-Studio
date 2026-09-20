#!/usr/bin/env node
// CLI: normalize + schema-validate a raw world-fragment input file.
//
// Usage:
//   node tools/universal-graph/validate.mjs <input.json>
//   node tools/universal-graph/validate.mjs <input.json> --check-determinism
//
// Exits 0 and prints the NormalizedWorldFragment on success, serialized via
// canonicalPrettyStringify() -- an explicit canonical-pretty-JSON
// serializer, not a bare JSON.stringify() trusting incidental object-key
// insertion order. JSON object member order is semantically irrelevant, so
// this public output path re-canonicalizes explicitly at the serialization
// boundary rather than relying on normalizeWorldFragment() already
// returning a canonicalized fragment (which it does, but this boundary
// does not assume that of whatever value it is handed). Exits 1 and
// prints every schema violation on failure. --check-determinism additionally
// re-runs normalization on the same input and fails if the two content
// hashes (or full canonical outputs) differ.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { compileSchemas, formatErrors } from "./schema-validate.mjs";
import { normalizeWorldFragment, canonicalStringify, canonicalPrettyStringify } from "./normalize.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(HERE, "..", "..", "docs", "_machine", "universal-graph", "v0", "schemas");

function main() {
  const args = process.argv.slice(2);
  const inputPath = args.find((a) => !a.startsWith("--"));
  const checkDeterminism = args.includes("--check-determinism");

  if (!inputPath) {
    console.error("usage: validate.mjs <input.json> [--check-determinism]");
    process.exit(2);
  }

  const ajv = compileSchemas(SCHEMA_DIR);
  const raw = JSON.parse(readFileSync(inputPath, "utf8"));

  const first = normalizeWorldFragment(raw, ajv);
  if (!first.valid) {
    console.error(`INVALID: ${inputPath}`);
    console.error(formatErrors(first.errors));
    process.exit(1);
  }

  if (checkDeterminism) {
    const second = normalizeWorldFragment(raw, ajv);
    if (canonicalStringify(first.fragment) !== canonicalStringify(second.fragment)) {
      console.error(`NONDETERMINISTIC: ${inputPath} produced different output across two runs`);
      process.exit(1);
    }
  }

  console.log(canonicalPrettyStringify(first.fragment));
  process.exit(0);
}

main();
