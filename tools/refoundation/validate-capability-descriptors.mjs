#!/usr/bin/env node
/**
 * validate-capability-descriptors.mjs
 *
 * Validates every crates/cap/<crate>/.chronica/capability.chronica.json against
 * docs/schemas/refoundation/capability-chronica.schema.json. This descriptor is
 * the replacement authority for the legacy canonical-JSONL/architecture.db
 * capability model (docs/doctrines/2026-08-23-refoundation-capability-graph-architecture.md),
 * so every RUNTIME_BOUND cap crate must carry one.
 *
 * Not a full JSON Schema Draft 2020-12 engine (no ajv dependency in this repo) --
 * hand-rolled to cover what the schema actually constrains: required fields at
 * every level, additionalProperties:false, enums, id/domain patterns, and the
 * stringList shape (array of unique non-empty strings).
 */

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const MATURITY_ENUM = new Set([
  "REFOUNDATION_ACTIVE",
  "CAPABILITY_MIGRATED",
  "GRAPH_BOUND",
  "DOMAIN_BOUND",
  "UI_BOUND",
  "RUNTIME_BOUND",
  "PERSISTENCE_BOUND",
  "PARTIAL_RUNTIME_BOUND",
  "PORTED",
  "LEGACY_SUPERSEDED",
  "CI_PENDING",
  "CI_VERIFIED",
  "NOT_BUILT",
]);

const CENSUS_STATE_ENUM = new Set(["NOT_STARTED", "PARTIAL", "CENSUSED_PARTIAL", "CENSUSED", "BLOCKED_WITH_EVIDENCE"]);

const ID_DOMAIN_PATTERN = /^[a-z0-9._-]+$/;

const RELATIONS_KEYS = new Set([
  "requires",
  "calls",
  "called_by",
  "produces",
  "consumes",
  "persists_to",
  "exposes_to",
  "authority_depends_on",
  "ui_surfaces",
  "supersedes",
  "superseded_by",
  "composes_with",
]);

function isStringList(value) {
  if (!Array.isArray(value)) return false;
  if (!value.every((item) => typeof item === "string" && item.length >= 1)) return false;
  return new Set(value).size === value.length;
}

function pushError(errors, descriptorPath, message) {
  errors.push(`${descriptorPath}: ${message}`);
}

function checkStringListField(errors, descriptorPath, obj, key, prefix) {
  if (obj[key] === undefined) return;
  if (!isStringList(obj[key])) {
    pushError(errors, descriptorPath, `${prefix}.${key} must be an array of unique non-empty strings`);
  }
}

export function validateCapabilityDescriptor(descriptor, descriptorPath) {
  const errors = [];

  if (typeof descriptor !== "object" || descriptor === null || Array.isArray(descriptor)) {
    return [`${descriptorPath}: descriptor must be a JSON object`];
  }

  const topRequired = ["schema", "capability", "purpose", "ownership", "relations", "knowledge", "verification"];
  for (const key of topRequired) {
    if (descriptor[key] === undefined) {
      pushError(errors, descriptorPath, `missing required top-level field "${key}"`);
    }
  }

  const topAllowed = new Set(topRequired);
  for (const key of Object.keys(descriptor)) {
    if (!topAllowed.has(key)) {
      pushError(errors, descriptorPath, `unexpected top-level field "${key}" (additionalProperties: false)`);
    }
  }

  if (descriptor.schema !== undefined && descriptor.schema !== "chronica.refoundation.capability.v1") {
    pushError(errors, descriptorPath, `schema must equal "chronica.refoundation.capability.v1", got ${JSON.stringify(descriptor.schema)}`);
  }

  if (descriptor.capability !== undefined) {
    const cap = descriptor.capability;
    if (typeof cap !== "object" || cap === null) {
      pushError(errors, descriptorPath, "capability must be an object");
    } else {
      for (const key of ["id", "domain", "maturity"]) {
        if (cap[key] === undefined) pushError(errors, descriptorPath, `capability.${key} is required`);
      }
      for (const key of Object.keys(cap)) {
        if (!["id", "domain", "maturity"].includes(key)) {
          pushError(errors, descriptorPath, `unexpected field "capability.${key}"`);
        }
      }
      if (typeof cap.id === "string" && !ID_DOMAIN_PATTERN.test(cap.id)) {
        pushError(errors, descriptorPath, `capability.id "${cap.id}" must match ^[a-z0-9._-]+$`);
      }
      if (typeof cap.domain === "string" && !ID_DOMAIN_PATTERN.test(cap.domain)) {
        pushError(errors, descriptorPath, `capability.domain "${cap.domain}" must match ^[a-z0-9._-]+$`);
      }
      if (cap.maturity !== undefined && !MATURITY_ENUM.has(cap.maturity)) {
        pushError(errors, descriptorPath, `capability.maturity "${cap.maturity}" is not a valid enum value`);
      }
    }
  }

  if (descriptor.purpose !== undefined) {
    const purpose = descriptor.purpose;
    if (typeof purpose !== "object" || purpose === null) {
      pushError(errors, descriptorPath, "purpose must be an object");
    } else {
      for (const key of ["reason_for_existence", "product_behavior"]) {
        if (typeof purpose[key] !== "string" || purpose[key].length < 1) {
          pushError(errors, descriptorPath, `purpose.${key} must be a non-empty string`);
        }
      }
      checkStringListField(errors, descriptorPath, purpose, "anti_claims", "purpose");
      for (const key of Object.keys(purpose)) {
        if (!["reason_for_existence", "product_behavior", "anti_claims"].includes(key)) {
          pushError(errors, descriptorPath, `unexpected field "purpose.${key}"`);
        }
      }
    }
  }

  if (descriptor.ownership !== undefined) {
    const ownership = descriptor.ownership;
    if (typeof ownership !== "object" || ownership === null) {
      pushError(errors, descriptorPath, "ownership must be an object");
    } else {
      for (const key of ["implementation_home", "owner_crate"]) {
        if (typeof ownership[key] !== "string" || ownership[key].length < 1) {
          pushError(errors, descriptorPath, `ownership.${key} must be a non-empty string`);
        }
      }
      if (ownership.caller_entries === undefined || !isStringList(ownership.caller_entries)) {
        pushError(errors, descriptorPath, "ownership.caller_entries must be an array of unique non-empty strings");
      }
      checkStringListField(errors, descriptorPath, ownership, "runtime_owners", "ownership");
      const ownershipAllowed = [
        "implementation_home",
        "owner_crate",
        "caller_entries",
        "runtime_owners",
        "authority_boundary",
        "tenant_boundary",
        "provider_boundary",
      ];
      for (const key of Object.keys(ownership)) {
        if (!ownershipAllowed.includes(key)) {
          pushError(errors, descriptorPath, `unexpected field "ownership.${key}"`);
        }
      }
    }
  }

  if (descriptor.relations !== undefined) {
    const relations = descriptor.relations;
    if (typeof relations !== "object" || relations === null) {
      pushError(errors, descriptorPath, "relations must be an object");
    } else {
      for (const key of Object.keys(relations)) {
        if (!RELATIONS_KEYS.has(key)) {
          pushError(errors, descriptorPath, `unexpected field "relations.${key}"`);
        } else {
          checkStringListField(errors, descriptorPath, relations, key, "relations");
        }
      }
    }
  }

  if (descriptor.knowledge !== undefined) {
    const knowledge = descriptor.knowledge;
    if (typeof knowledge !== "object" || knowledge === null) {
      pushError(errors, descriptorPath, "knowledge must be an object");
    } else {
      for (const key of ["census_state", "reference_families", "known_gaps"]) {
        if (knowledge[key] === undefined) pushError(errors, descriptorPath, `knowledge.${key} is required`);
      }
      if (knowledge.census_state !== undefined && !CENSUS_STATE_ENUM.has(knowledge.census_state)) {
        pushError(errors, descriptorPath, `knowledge.census_state "${knowledge.census_state}" is not a valid enum value`);
      }
      checkStringListField(errors, descriptorPath, knowledge, "reference_families", "knowledge");
      checkStringListField(errors, descriptorPath, knowledge, "covered_sources", "knowledge");
      checkStringListField(errors, descriptorPath, knowledge, "known_gaps", "knowledge");
      const knowledgeAllowed = ["census_state", "reference_families", "covered_sources", "known_gaps"];
      for (const key of Object.keys(knowledge)) {
        if (!knowledgeAllowed.includes(key)) {
          pushError(errors, descriptorPath, `unexpected field "knowledge.${key}"`);
        }
      }
    }
  }

  if (descriptor.verification !== undefined) {
    const verification = descriptor.verification;
    if (typeof verification !== "object" || verification === null) {
      pushError(errors, descriptorPath, "verification must be an object");
    } else {
      for (const key of ["positive_witnesses", "negative_witnesses", "integration_surfaces"]) {
        if (verification[key] === undefined) pushError(errors, descriptorPath, `verification.${key} is required`);
        else checkStringListField(errors, descriptorPath, verification, key, "verification");
      }
      if (verification.last_verified_command !== undefined && typeof verification.last_verified_command !== "string") {
        pushError(errors, descriptorPath, "verification.last_verified_command must be a string");
      }
      checkStringListField(errors, descriptorPath, verification, "local_structural_commands", "verification");
      const verificationAllowed = [
        "positive_witnesses",
        "negative_witnesses",
        "integration_surfaces",
        "last_verified_command",
        "local_structural_commands",
      ];
      for (const key of Object.keys(verification)) {
        if (!verificationAllowed.includes(key)) {
          pushError(errors, descriptorPath, `unexpected field "verification.${key}"`);
        }
      }
    }
  }

  return errors;
}

export function findCapCrateDirs(capRoot) {
  if (!existsSync(capRoot)) return [];
  return readdirSync(capRoot)
    .map((name) => resolve(capRoot, name))
    .filter((dir) => statSync(dir).isDirectory());
}

export function runValidation({ repoRoot, log = console.log, error = console.error }) {
  const capRoot = resolve(repoRoot, "crates/cap");
  const crateDirs = findCapCrateDirs(capRoot);

  let missing = [];
  let invalid = [];
  let ok = 0;

  for (const dir of crateDirs) {
    const crateName = basename(dir);
    const descriptorPath = resolve(dir, ".chronica/capability.chronica.json");
    if (!existsSync(descriptorPath)) {
      missing.push(crateName);
      continue;
    }

    let parsed;
    try {
      parsed = JSON.parse(readFileSync(descriptorPath, "utf8"));
    } catch (err) {
      invalid.push(`${crateName}: invalid JSON (${err.message})`);
      continue;
    }

    const errors = validateCapabilityDescriptor(parsed, descriptorPath);
    if (errors.length > 0) {
      invalid.push(...errors.map((e) => `${crateName}: ${e}`));
      continue;
    }

    ok += 1;
  }

  log(`  capability.chronica.json: ${ok}/${crateDirs.length} cap crates valid.`);

  if (missing.length > 0) {
    error(`\nERROR: ${missing.length} cap crate(s) missing .chronica/capability.chronica.json:`);
    for (const name of missing) error(`  ${name}`);
  }

  if (invalid.length > 0) {
    error(`\nERROR: ${invalid.length} schema violation(s):`);
    for (const line of invalid) error(`  ${line}`);
  }

  if (missing.length > 0 || invalid.length > 0) {
    error("\nEvery crates/cap/<crate> must carry a schema-valid .chronica/capability.chronica.json");
    error("(docs/schemas/refoundation/capability-chronica.schema.json, docs/templates/refoundation/capability.chronica.json).");
    return 1;
  }

  return 0;
}

function main() {
  const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
  process.exit(runValidation({ repoRoot }));
}

const isMainModule = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMainModule) {
  main();
}
