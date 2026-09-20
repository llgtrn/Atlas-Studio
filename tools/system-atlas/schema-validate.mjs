#!/usr/bin/env node
// Minimal, dependency-free JSON Schema (Draft 2020-12 subset) validator for
// the Chronica System Atlas schema files under
// tools/system-atlas/schema/*.schema.json (hand-authored source, committed
// here rather than under docs/_machine/ -- see validate.mjs's SCHEMA_DIR
// comment for why).
//
// This module reads the schema files themselves at runtime and validates
// purely from what they declare -- it does not hardcode any field list, so
// it stays correct as the coordinator adds/renames required fields or new
// schema files. Supported keywords: type, additionalProperties (boolean or
// schema), required, properties, enum, const, pattern, minItems, maxItems,
// minimum, maximum, items, propertyNames, $ref (resolved by $id against a
// supplied schemasById map).
//
// Owned exclusively by this file. Do not import product code into it and do
// not let anything import product code out of it -- it must stay a
// standalone, zero-dependency, pure ESM module usable from any script or CLI
// in this directory.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Reads every `*.schema.json` file directly inside `schemaDir`, parses it,
 * and returns a Map keyed by each schema's `$id`.
 *
 * @param {string} schemaDir
 * @returns {Map<string, object>}
 */
export function compileSchemas(schemaDir) {
  const schemasById = new Map();
  for (const file of readdirSync(schemaDir)) {
    if (!file.endsWith(".schema.json")) continue;
    const full = resolve(schemaDir, file);
    let schema;
    try {
      schema = JSON.parse(readFileSync(full, "utf8"));
    } catch (err) {
      throw new Error(`schema-validate: failed to parse ${file}: ${err.message}`);
    }
    if (!schema.$id) {
      throw new Error(`schema-validate: ${file} has no top-level "$id"`);
    }
    if (schemasById.has(schema.$id)) {
      throw new Error(`schema-validate: duplicate $id "${schema.$id}" (in ${file})`);
    }
    schemasById.set(schema.$id, schema);
  }
  return schemasById;
}

/**
 * Validates `instance` against a schema, collecting ALL violations (not just
 * the first).
 *
 * @param {*} instance
 * @param {string|object} schemaIdOrInlineSchema - either a `$id` string to
 *   look up in `schemasById`, or an inline schema object.
 * @param {Map<string, object>} schemasById - map of $id -> schema, as
 *   returned by compileSchemas(), used to resolve $ref.
 * @returns {{valid: boolean, errors: Array<{path: string, message: string}>}}
 */
export function validate(instance, schemaIdOrInlineSchema, schemasById) {
  let schema;
  if (typeof schemaIdOrInlineSchema === "string") {
    schema = schemasById?.get(schemaIdOrInlineSchema);
    if (!schema) {
      throw new Error(`schema-validate: unknown schema $id "${schemaIdOrInlineSchema}"`);
    }
  } else {
    schema = schemaIdOrInlineSchema;
  }
  const errors = [];
  validateNode(instance, schema, "", schemasById ?? new Map(), errors, schema);
  return { valid: errors.length === 0, errors };
}

/**
 * Renders a list of {path, message} errors as a human-readable multi-line
 * string for CLI output.
 *
 * @param {Array<{path: string, message: string}>} errors
 * @returns {string}
 */
export function formatErrors(errors) {
  if (!errors || errors.length === 0) return "  (no errors)";
  return errors.map((e) => `  ${e.path || "/"}: ${e.message}`).join("\n");
}

// ---------------------------------------------------------------------------
// internals
// ---------------------------------------------------------------------------

function describeType(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  return typeof value;
}

function matchesType(value, type) {
  switch (type) {
    case "null":
      return value === null;
    case "boolean":
      return typeof value === "boolean";
    case "object":
      return value !== null && typeof value === "object" && !Array.isArray(value);
    case "array":
      return Array.isArray(value);
    case "string":
      return typeof value === "string";
    case "number":
      return typeof value === "number" && Number.isFinite(value);
    case "integer":
      return typeof value === "number" && Number.isFinite(value) && Number.isInteger(value);
    default:
      return true; // unknown type keyword: don't fail closed on it
  }
}

function deepEqual(a, b) {
  if (a === b) return true;
  if (a === null || b === null || typeof a !== "object" || typeof b !== "object") return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (Array.isArray(a)) {
    if (a.length !== b.length) return false;
    return a.every((v, i) => deepEqual(v, b[i]));
  }
  const aKeys = Object.keys(a);
  const bKeys = Object.keys(b);
  if (aKeys.length !== bKeys.length) return false;
  return aKeys.every((k) => Object.prototype.hasOwnProperty.call(b, k) && deepEqual(a[k], b[k]));
}

function escapePointerToken(token) {
  return String(token).replace(/~/g, "~0").replace(/\//g, "~1");
}

function unescapePointerToken(token) {
  return String(token).replace(/~1/g, "/").replace(/~0/g, "~");
}

function resolveLocalRef(rootSchema, ref) {
  if (ref === "#") return rootSchema;
  if (!ref.startsWith("#/")) return null;
  let current = rootSchema;
  for (const rawPart of ref.slice(2).split("/")) {
    const part = unescapePointerToken(rawPart);
    if (current === null || typeof current !== "object" || !Object.prototype.hasOwnProperty.call(current, part)) {
      return null;
    }
    current = current[part];
  }
  return current;
}

function validateNode(instance, schema, path, schemasById, errors, rootSchema = schema) {
  if (schema === true || schema === undefined || schema === null) return; // no constraints
  if (schema === false) {
    errors.push({ path, message: "no value is allowed here (schema is `false`)" });
    return;
  }

  if (schema.$ref !== undefined) {
    const target = schema.$ref.startsWith("#")
      ? resolveLocalRef(rootSchema, schema.$ref)
      : schemasById.get(schema.$ref);
    if (!target) {
      errors.push({ path, message: `unresolved $ref "${schema.$ref}"` });
    } else {
      validateNode(instance, target, path, schemasById, errors, rootSchema);
    }
  }

  if (Object.prototype.hasOwnProperty.call(schema, "const")) {
    if (!deepEqual(instance, schema.const)) {
      errors.push({ path, message: `expected const ${JSON.stringify(schema.const)}, got ${JSON.stringify(instance)}` });
    }
  }

  if (Array.isArray(schema.enum)) {
    if (!schema.enum.some((v) => deepEqual(v, instance))) {
      errors.push({ path, message: `${JSON.stringify(instance)} is not one of enum ${JSON.stringify(schema.enum)}` });
    }
  }

  if (schema.type !== undefined) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((t) => matchesType(instance, t))) {
      errors.push({ path, message: `expected type ${types.join(" | ")}, got ${describeType(instance)}` });
      return; // further structural checks below would be meaningless/noisy
    }
  }

  if (typeof instance === "string" && schema.pattern !== undefined) {
    if (!new RegExp(schema.pattern).test(instance)) {
      errors.push({ path, message: `"${instance}" does not match pattern /${schema.pattern}/` });
    }
  }

  if (typeof instance === "number") {
    if (schema.minimum !== undefined && instance < schema.minimum) {
      errors.push({ path, message: `${instance} < minimum ${schema.minimum}` });
    }
    if (schema.maximum !== undefined && instance > schema.maximum) {
      errors.push({ path, message: `${instance} > maximum ${schema.maximum}` });
    }
  }

  if (Array.isArray(instance)) {
    if (schema.minItems !== undefined && instance.length < schema.minItems) {
      errors.push({ path, message: `array has ${instance.length} item(s), fewer than minItems ${schema.minItems}` });
    }
    if (schema.maxItems !== undefined && instance.length > schema.maxItems) {
      errors.push({ path, message: `array has ${instance.length} item(s), more than maxItems ${schema.maxItems}` });
    }
    if (schema.items !== undefined) {
      instance.forEach((item, i) => validateNode(item, schema.items, `${path}/${i}`, schemasById, errors, rootSchema));
    }
  }

  if (instance !== null && typeof instance === "object" && !Array.isArray(instance)) {
    const props = schema.properties ?? {};

    if (Array.isArray(schema.required)) {
      for (const key of schema.required) {
        if (!Object.prototype.hasOwnProperty.call(instance, key)) {
          errors.push({ path: `${path}/${escapePointerToken(key)}`, message: `missing required property "${key}"` });
        }
      }
    }

    for (const [key, value] of Object.entries(instance)) {
      const subPath = `${path}/${escapePointerToken(key)}`;

      if (schema.propertyNames !== undefined) {
        validateNode(key, schema.propertyNames, subPath, schemasById, errors, rootSchema);
      }

      if (Object.prototype.hasOwnProperty.call(props, key)) {
        validateNode(value, props[key], subPath, schemasById, errors, rootSchema);
      } else if (schema.additionalProperties === false) {
        errors.push({ path: subPath, message: `additional property "${key}" is not allowed` });
      } else if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
        validateNode(value, schema.additionalProperties, subPath, schemasById, errors, rootSchema);
      }
      // additionalProperties === true or undefined: extra keys are allowed, unconstrained
    }
  }
}

// ---------------------------------------------------------------------------
// self-test (only runs when this file is executed directly, e.g.
// `node tools/system-atlas/schema-validate.mjs` -- never as an import
// side-effect)
// ---------------------------------------------------------------------------

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
  const SCHEMA_DIR = resolve(REPO_ROOT, "tools/system-atlas/schema");

  let failures = 0;
  const check = (label, cond, detail) => {
    if (cond) {
      console.log(`  PASS  ${label}`);
    } else {
      failures++;
      console.log(`  FAIL  ${label}${detail ? ` -- ${detail}` : ""}`);
    }
  };

  console.log(`schema-validate self-test: compiling schemas from ${SCHEMA_DIR}`);
  let schemasById;
  try {
    schemasById = compileSchemas(SCHEMA_DIR);
    check("compileSchemas() reads all *.schema.json files", schemasById.size >= 5, `found ${schemasById.size}`);
  } catch (err) {
    console.log(`  FAIL  compileSchemas() threw: ${err.message}`);
    process.exit(1);
  }

  const HEX40 = "a".repeat(40);

  // -- known-good task -------------------------------------------------
  const goodTask = {
    task_id: "BUILD-0001",
    action: "BUILD",
    node_ids: ["system:identity"],
    priority: 1,
    risk: "T1",
    source_base_sha: HEX40,
    owned_paths: ["tools/system-atlas/schema-validate.mjs"],
    status: "READY",
  };
  const goodTaskResult = validate(goodTask, "chronica.system-atlas.task.v0", schemasById);
  check("known-good task validates clean", goodTaskResult.valid, formatErrors(goodTaskResult.errors));

  // -- known-bad task: THE bug this module exists to catch --------------
  const badTask = { ...goodTask, node_ids: [] };
  const badTaskResult = validate(badTask, "chronica.system-atlas.task.v0", schemasById);
  check("task with empty node_ids is rejected", badTaskResult.valid === false);
  check(
    "rejection names node_ids / minItems",
    badTaskResult.errors.some((e) => e.path.includes("node_ids") && /minItems|fewer/i.test(e.message)),
    JSON.stringify(badTaskResult.errors),
  );

  // -- multi-violation task: all errors collected in one pass -----------
  const multiBadTask = {
    task_id: "BUILD-0001",
    action: "NOT_A_REAL_ACTION",
    node_ids: [],
    priority: 1,
    risk: "T1",
    source_base_sha: HEX40,
    owned_paths: [],
    // status omitted -> missing required
  };
  const multiBadResult = validate(multiBadTask, "chronica.system-atlas.task.v0", schemasById);
  check(
    "multi-violation task collects all 3 distinct problems, not just the first",
    multiBadResult.errors.length >= 3,
    `got ${multiBadResult.errors.length}: ${JSON.stringify(multiBadResult.errors)}`,
  );

  // -- additionalProperties: false rejects an unknown field -------------
  const extraFieldTask = { ...goodTask, totally_unknown_field: true };
  const extraFieldResult = validate(extraFieldTask, "chronica.system-atlas.task.v0", schemasById);
  check(
    "unknown property rejected by additionalProperties: false",
    extraFieldResult.valid === false && extraFieldResult.errors.some((e) => e.path.includes("totally_unknown_field")),
  );

  // -- $ref resolution: shard.nodes / shard.edges items reference node/edge schemas
  const goodNode = {
    id: "system:identity",
    kind: "SYSTEM",
    label: "Identity",
    existence: "PRESENT",
    source_class: "CODE",
  };
  const goodEdge = {
    from: "system:identity",
    to: "system:other",
    type: "DEPENDS_ON",
    source_class: "CODE",
  };
  const goodShard = {
    schema: "chronica.system-atlas.shard.v0",
    lane: "B1_topology",
    source_base_sha: HEX40,
    atlas_generation_sha: HEX40,
    generated_at: "2026-09-01T00:00:00Z",
    nodes: [goodNode],
    edges: [goodEdge],
  };
  const goodShardResult = validate(goodShard, "chronica.system-atlas.shard.v0", schemasById);
  check("$ref-nested shard with valid node/edge validates clean", goodShardResult.valid, formatErrors(goodShardResult.errors));

  const badShard = { ...goodShard, nodes: [{ ...goodNode, kind: "NOT_A_KIND" }] };
  const badShardResult = validate(badShard, "chronica.system-atlas.shard.v0", schemasById);
  check(
    "$ref-nested invalid node kind is caught through shard -> node $ref, path points into /nodes/0",
    badShardResult.valid === false && badShardResult.errors.some((e) => e.path.startsWith("/nodes/0")),
    JSON.stringify(badShardResult.errors),
  );

  const localRefSchema = {
    type: "object",
    properties: {
      names: { $ref: "#/$defs/nameList" },
    },
    required: ["names"],
    additionalProperties: false,
    $defs: {
      nameList: {
        type: "array",
        items: { type: "string" },
      },
    },
  };
  const localRefResult = validate({ names: ["atlas"] }, localRefSchema, schemasById);
  check("local #/$defs $ref validates clean", localRefResult.valid, formatErrors(localRefResult.errors));
  const badLocalRefResult = validate({ names: [1] }, localRefSchema, schemasById);
  check(
    "local #/$defs $ref catches nested violations",
    badLocalRefResult.valid === false && badLocalRefResult.errors.some((e) => e.path === "/names/0"),
    JSON.stringify(badLocalRefResult.errors),
  );

  console.log("");
  if (failures > 0) {
    console.log(`schema-validate self-test: FAIL (${failures} failing check(s))`);
    process.exit(1);
  }
  console.log("schema-validate self-test: PASS");
  process.exit(0);
}
