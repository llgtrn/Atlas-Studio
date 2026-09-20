#!/usr/bin/env node
// D3 schema loader: registers D0 + D1 (the two schemas D3 reuses: WorldEntity's
// canonical-world-snapshot and authenticated-requester-context) + D2 (CapabilityDefinition/
// CapabilityBinding/CapabilityConstraint/EvidenceRef) + D3's own capability-registration
// schemas into ONE Ajv2020 instance, so cross-directory $ref resolves (e.g.
// CanonicalCapabilityRegistrySnapshot.definitions -> D2's capability-definition.v0,
// CapabilityRegistrationRequest.requester_principal_id -> D0's common.v0#/$defs/entityId).
//
// No hand-written JSON Schema keyword implementation lives here -- this file only discovers
// and registers *.schema.json files across the four directories and delegates every
// validation call to D0's validate()/formatErrors(), reused verbatim. Same glue pattern as
// D2's compileCapabilitySchemas(dirs) (tools/universal-graph/capability/normalize.mjs),
// applied to one more directory.

import { readdirSync, readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import { validate, formatErrors } from "../schema-validate.mjs";

/**
 * @param {string[]} dirs - schema directories to register, in any order
 * @returns {import("ajv").default} one Ajv2020 instance carrying every registered schema
 */
export function compileCapabilityRegistrationSchemas(dirs) {
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  const ids = [];
  for (const dir of dirs) {
    for (const file of readdirSync(dir).filter((f) => f.endsWith(".schema.json"))) {
      const schema = JSON.parse(readFileSync(`${dir}/${file}`, "utf8"));
      if (!schema.$id) {
        throw new Error(`compileCapabilityRegistrationSchemas: ${dir}/${file} has no top-level "$id"`);
      }
      ajv.addSchema(schema, schema.$id);
      ids.push(schema.$id);
    }
  }
  // Force every schema to compile now, not lazily on first use, so a structurally-broken
  // schema fails loudly here.
  for (const id of ids) ajv.getSchema(id);
  return ajv;
}

export { validate, formatErrors };
