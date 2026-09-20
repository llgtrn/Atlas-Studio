#!/usr/bin/env node
// D1 schema loader: reuses D0's real Ajv2020 wrapper (tools/universal-graph/schema-validate.mjs)
// unmodified, then registers the D1 admission schemas into the SAME Ajv instance so that
// cross-directory $ref (e.g. CanonicalWorldSnapshot.entities -> D0's world-entity.v0,
// AdmissionRequest.requester_principal_id -> D0's common.v0#/$defs/entityId) resolves.
//
// No hand-written JSON Schema keyword implementation lives here either -- this file only
// discovers and registers D1's *.schema.json files and delegates every validation call to
// D0's validate()/formatErrors(), reused verbatim.

import { readdirSync, readFileSync } from "node:fs";
import { compileSchemas, validate, formatErrors } from "../schema-validate.mjs";

/**
 * @param {string} d0SchemaDir - docs/_machine/universal-graph/v0/schemas
 * @param {string} admissionSchemaDir - docs/_machine/universal-graph/admission/v0/schemas
 * @returns {import("ajv").default} one Ajv2020 instance carrying both D0 and D1 schemas
 */
export function compileAdmissionSchemas(d0SchemaDir, admissionSchemaDir) {
  const ajv = compileSchemas(d0SchemaDir);
  const files = readdirSync(admissionSchemaDir).filter((f) => f.endsWith(".schema.json"));
  for (const file of files) {
    const schema = JSON.parse(readFileSync(`${admissionSchemaDir}/${file}`, "utf8"));
    if (!schema.$id) {
      throw new Error(`admission/schema-validate: ${file} has no top-level "$id"`);
    }
    ajv.addSchema(schema, schema.$id);
  }
  for (const file of files) {
    const schema = JSON.parse(readFileSync(`${admissionSchemaDir}/${file}`, "utf8"));
    ajv.getSchema(schema.$id);
  }
  return ajv;
}

export { validate, formatErrors };
