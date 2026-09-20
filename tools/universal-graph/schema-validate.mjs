#!/usr/bin/env node
// Thin wrapper around the real Ajv Draft 2020-12 engine for the Chronica
// Universal Graph Admission Kernel schema files under
// docs/_machine/universal-graph/v0/schemas/*.schema.json.
//
// This module contains NO hand-written implementation of any JSON Schema
// keyword. All keyword semantics (type, required, properties,
// additionalProperties, pattern, enum, const, items, oneOf, not, minItems,
// uniqueItems, $ref, $defs, ...) are Ajv's, not ours. This file only:
//   - discovers and registers the committed schema files with Ajv2020,
//     which validates each one against the real Draft 2020-12 meta-schema
//     as it is added (an invalid schema throws here, at compile time);
//   - runs real ajv validation against a compiled schema;
//   - reshapes Ajv's own error objects into a flat {path, message} list.
//
// D.CLEAN-0's original hand-written keyword walker has been removed
// entirely per the D.CLEAN-0R architect finding: a bespoke "Draft 2020-12
// subset" is not an acceptable substitute for real JSON Schema validation.

import { readdirSync, readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";

/**
 * Reads every `*.schema.json` file directly inside `schemaDir`, registers
 * it with a fresh Ajv2020 instance (which meta-validates it against the
 * real Draft 2020-12 meta-schema as part of addSchema), and returns the
 * Ajv instance. Throws if any schema is meta-invalid or fails to compile.
 *
 * @param {string} schemaDir
 * @returns {import("ajv").default}
 */
export function compileSchemas(schemaDir) {
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  const files = readdirSync(schemaDir).filter((f) => f.endsWith(".schema.json"));
  for (const file of files) {
    let schema;
    try {
      schema = JSON.parse(readFileSync(`${schemaDir}/${file}`, "utf8"));
    } catch (err) {
      throw new Error(`schema-validate: failed to parse ${file}: ${err.message}`);
    }
    if (!schema.$id) {
      throw new Error(`schema-validate: ${file} has no top-level "$id"`);
    }
    ajv.addSchema(schema, schema.$id);
  }
  // Force every schema to compile now (Ajv otherwise compiles lazily on
  // first use) so a structurally-broken schema fails loudly here, not on
  // whatever fixture happens to hit it first.
  for (const file of files) {
    const schema = JSON.parse(readFileSync(`${schemaDir}/${file}`, "utf8"));
    ajv.getSchema(schema.$id);
  }
  return ajv;
}

/**
 * Validates `instance` against a registered schema $id using real Ajv
 * Draft 2020-12 validation, and returns every violation Ajv reports.
 *
 * @param {*} instance
 * @param {string} schemaId - a $id registered via compileSchemas()
 * @param {import("ajv").default} ajv
 * @returns {{valid: boolean, errors: Array<{path: string, message: string}>}}
 */
export function validate(instance, schemaId, ajv) {
  const validateFn = ajv.getSchema(schemaId);
  if (!validateFn) {
    throw new Error(`schema-validate: unknown schema $id "${schemaId}"`);
  }
  const valid = validateFn(instance);
  if (valid) return { valid: true, errors: [] };
  const errors = (validateFn.errors ?? []).map((e) => ({
    path: e.instancePath || "/",
    message: `${e.keyword}: ${e.message}${e.params ? ` ${JSON.stringify(e.params)}` : ""}`,
  }));
  return { valid: false, errors };
}

export function formatErrors(errors) {
  if (!errors || errors.length === 0) return "  (no errors)";
  return errors.map((e) => `  ${e.path || "/"}: ${e.message}`).join("\n");
}
