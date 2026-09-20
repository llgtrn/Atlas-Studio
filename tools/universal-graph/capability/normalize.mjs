#!/usr/bin/env node
// Deterministic CANDIDATE_ONLY normalizer for the Universal Capability
// Registry Candidate Kernel (D.CLEAN-2).
//
// normalizeCapabilityRegistry() takes a raw capability observation and runs
// the required D.CLEAN-2 pipeline, in this order:
//
//   raw capability observation
//     -> real Ajv Draft-2020-12 raw-envelope validation (stage zero): the
//        whole input is checked in one shot against
//        raw-capability-observation.schema.json, a CLOSED envelope
//        (additionalProperties: false, all three collections required and
//        each strictly an array of the correctly-shaped record). This also
//        covers per-record structural validation, since the envelope
//        schema $ref's into capability-definition/capability-binding/
//        evidence-ref for every array item. An unknown top-level field
//        (registration_state, canonical, registered, authorized,
//        execution_authority, approval, grant, work_request, command,
//        content_hash, ...) is rejected here, never silently ignored.
//     -> definition_hash recomputation/verification: computeDefinitionHash()
//        rebuilds each definition's canonical semantic payload (excluding
//        definition_hash itself -- acyclic) and compares it to the
//        provider-supplied value. A provider-supplied hash is never
//        trusted; a mismatch fails closed.
//     -> known order-insensitive nested-collection canonicalization:
//        CapabilityDefinition/CapabilityBinding.constraints (a total order
//        over canonicalStringify(constraint)) and .evidence_ref_ids (a
//        reference SET, deduplicated and lexically sorted). Fresh copies
//        only -- the caller's raw objects are never mutated. A
//        CapabilityConstraint's own `value` array, when present, is left
//        exactly as supplied: its order is not declared order-insensitive
//        and may be vocabulary-semantic.
//     -> exact-duplicate normalization (content-identical records collapse)
//        + identifier conflict detection (capability_definition_id /
//        binding_id / evidence_ref_id)
//     -> capability_key+definition_version natural-key conflict detection,
//        using an injective tuple encoding (a canonicalStringify()'d JSON
//        array, never a delimiter-joined string)
//     -> cross-record referential integrity: every binding must resolve to
//        a definition in the same fragment with a matching
//        capability_definition_hash; every evidence_ref_id referenced by a
//        definition or binding must resolve inside the same fragment
//     -> TOTAL canonical deterministic sorting: every top-level collection
//        sorted by its useful primary key, then by canonicalStringify() of
//        the whole record as a strict tie-breaker for two distinct records
//        that legitimately share a primary key (e.g. two bindings for the
//        same entity/definition pair, differing only in binding_id already
//        being the primary key -- ties only arise pathologically and are
//        still resolved deterministically)
//     -> recursive object-key canonicalization
//     -> content_hash
//     -> CANDIDATE_ONLY CapabilityRegistryFragment
//
// Any failure at any stage returns { fragment: null, valid: false, errors }.
// There is no fail-open partial candidate.
//
// This module never proves canonical Chronica capability registry
// membership, canonical WorldEntity membership for any bound entity_id, or
// any execution/registration authority -- the output's registration_state
// is const-locked to "CANDIDATE_ONLY" and there is no code path that ever
// writes any other value. A provider cannot self-register: nothing in the
// raw input schema accepts a canonical/registered/authorized flag.
//
// Determinism rules this module must never violate:
//   - never read the current wall-clock time to produce content;
//   - never depend on input array order, at the top level OR within a
//     declared order-insensitive nested collection, nor on object key
//     insertion order;
//   - the same input, or any semantically-equivalent reordering of it
//     (top-level collections, definition/binding constraints,
//     evidence_ref_ids, or JSON object-key order at any nesting level),
//     must always produce byte-identical canonical JSON and therefore an
//     identical content_hash, run twice, run anywhere.

import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
import { validate } from "../schema-validate.mjs";
import { canonicalStringify, canonicalPrettyStringify } from "../normalize.mjs";

export const RAW_ENVELOPE_SCHEMA = "chronica.universal-graph.capability.raw-capability-observation.v0";
export const FRAGMENT_SCHEMA = "chronica.universal-graph.capability.capability-registry-fragment.v0";

/**
 * Compiles committed schema files from one or more directories into a
 * single Ajv2020 instance, so capability schemas (which $ref into D0's
 * common/provenance/evidence-ref $ids) resolve against the same registry
 * that D0's schemas are compiled into. Each schema is meta-validated
 * against the real Draft 2020-12 meta-schema as it is added by Ajv itself
 * -- this module contains no hand-written JSON-Schema keyword logic, only
 * file discovery and registration glue, identical in spirit to (but
 * independent of, since D0's compileSchemas() is single-directory only)
 * tools/universal-graph/schema-validate.mjs.
 *
 * @param {string[]} dirs
 * @returns {import("ajv").default}
 */
export function compileCapabilitySchemas(dirs) {
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  const ids = [];
  for (const dir of dirs) {
    for (const file of readdirSync(dir).filter((f) => f.endsWith(".schema.json"))) {
      const schema = JSON.parse(readFileSync(`${dir}/${file}`, "utf8"));
      if (!schema.$id) {
        throw new Error(`compileCapabilitySchemas: ${dir}/${file} has no top-level "$id"`);
      }
      ajv.addSchema(schema, schema.$id);
      ids.push(schema.$id);
    }
  }
  // Force every schema to compile now, not lazily on first use, so a
  // structurally-broken schema fails loudly here.
  for (const id of ids) ajv.getSchema(id);
  return ajv;
}

function compareKeys(a, b) {
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const av = a[i] ?? "";
    const bv = b[i] ?? "";
    if (av < bv) return -1;
    if (av > bv) return 1;
  }
  return 0;
}

/** Total order over a list of records by canonicalStringify() -- used to sort constraints and to tie-break same-primary-key records. */
function canonicalOrder(a, b) {
  const ca = canonicalStringify(a);
  const cb = canonicalStringify(b);
  return ca < cb ? -1 : ca > cb ? 1 : 0;
}

/** Fresh sorted+deduplicated copy of an evidence_ref_ids array: a reference SET, never mutates the input. */
function sortEvidenceRefIds(ids) {
  return Array.from(new Set(ids)).sort();
}

/** Fresh sorted copy of a constraints array by full-record canonical order. Never mutates the input, never touches a constraint's own `value` array order. */
function sortConstraints(constraints) {
  return [...constraints].sort(canonicalOrder);
}

/**
 * Produces a non-mutating copy of a CapabilityDefinition or
 * CapabilityBinding with its declared order-insensitive nested collections
 * (constraints, evidence_ref_ids) canonicalized. Both record kinds share
 * exactly the same two order-insensitive field names, so one function
 * covers both -- consistent with there being one shared capability
 * primitive rather than parallel per-kind models.
 */
function canonicalizeNestedOrder(item) {
  const out = { ...item };
  if (Array.isArray(item.constraints)) out.constraints = sortConstraints(item.constraints);
  if (Array.isArray(item.evidence_ref_ids)) out.evidence_ref_ids = sortEvidenceRefIds(item.evidence_ref_ids);
  return out;
}

/**
 * The exact canonical hash payload for a CapabilityDefinition's
 * definition_hash: semantic content only, excluding capability_definition_id
 * (a record identity, not semantic content), provenance (observational
 * metadata -- two observations of the same semantic definition may have
 * different observed_at), and definition_hash itself (acyclic -- no
 * self-reference). constraints/evidence_ref_ids are canonicalized inside
 * this payload builder itself, so hash verification never depends on
 * whatever order the raw input happened to supply them in.
 */
function definitionHashPayload(def) {
  return {
    capability_key: def.capability_key,
    definition_version: def.definition_version,
    request_contract_ref: def.request_contract_ref,
    result_contract_ref: def.result_contract_ref,
    constraints: sortConstraints(def.constraints ?? []),
    evidence_ref_ids: sortEvidenceRefIds(def.evidence_ref_ids ?? []),
  };
}

/**
 * Recomputes the definition_hash a CapabilityDefinition SHOULD carry, from
 * its own semantic content. Same semantic definition (any array-order
 * permutation of constraints/evidence_ref_ids included) always produces the
 * same hash; any meaningful change to capability_key, definition_version,
 * either contract ref, constraints, or evidence_ref_ids produces a
 * different hash.
 *
 * @param {object} def
 * @returns {string} "sha256:<hex>"
 */
export function computeDefinitionHash(def) {
  const digest = createHash("sha256").update(canonicalStringify(definitionHashPayload(def))).digest("hex");
  return `sha256:${digest}`;
}

const SORT_KEY = {
  definitions: (d) => [d?.capability_definition_id ?? ""],
  bindings: (b) => [b?.binding_id ?? ""],
  evidence_refs: (e) => [e?.evidence_ref_id ?? ""],
};

/** Total order over a top-level collection: primary key first, then full canonical record content as a strict tie-breaker. Never falls back to source index or sort stability. */
function sortItems(kind, items) {
  const keyer = SORT_KEY[kind];
  return items.slice().sort((a, b) => {
    const primary = compareKeys(keyer(a), keyer(b));
    return primary !== 0 ? primary : canonicalOrder(a, b);
  });
}

/**
 * Groups `items` by `idField`, collapsing exact-content duplicates within a
 * group. A group whose members differ in canonicalized content is a
 * conflict: returned under `conflicts` (not `deduped`), tagged with `code`.
 * No first-wins, no last-wins.
 */
function dedupeByIdOrConflict(items, idField, code) {
  const byId = new Map();
  for (const item of items) {
    const key = item[idField];
    if (!byId.has(key)) byId.set(key, []);
    byId.get(key).push(item);
  }
  const deduped = [];
  const conflicts = [];
  for (const [key, group] of byId) {
    const uniqueByContent = [];
    for (const item of group) {
      const cj = canonicalStringify(item);
      if (!uniqueByContent.some((u) => u.cj === cj)) uniqueByContent.push({ cj, item });
    }
    if (uniqueByContent.length > 1) {
      conflicts.push({ path: `/${idField}`, message: `${code}: "${key}" has ${uniqueByContent.length} conflicting representations with the same ${idField}` });
    } else {
      deduped.push(uniqueByContent[0].item);
    }
  }
  return { deduped, conflicts };
}

/**
 * @param {object} raw - { definitions, bindings, evidence_refs }
 * @param {import("ajv").default} ajv - from compileCapabilitySchemas()
 * @returns {{fragment: object|null, valid: boolean, errors: Array<{path: string, message: string}>}}
 */
export function normalizeCapabilityRegistry(raw, ajv) {
  // Stage 0: real Ajv Draft-2020-12 raw-envelope + per-record structural
  // validation, in one call.
  const envelopeResult = validate(raw, RAW_ENVELOPE_SCHEMA, ajv);
  if (!envelopeResult.valid) {
    return { fragment: null, valid: false, errors: envelopeResult.errors };
  }

  // Stage 1: definition_hash recomputation/verification, against the raw
  // (pre-pipeline-reordering) definitions -- computeDefinitionHash() itself
  // canonicalizes constraints/evidence_ref_ids internally, so this is
  // robust to whatever array order the provider supplied.
  const hashErrors = [];
  raw.definitions.forEach((def, i) => {
    const expected = computeDefinitionHash(def);
    if (def.definition_hash !== expected) {
      hashErrors.push({
        path: `/definitions/${i}/definition_hash`,
        message: `CAPABILITY_DEFINITION_HASH_MISMATCH: expected ${expected}, got ${def.definition_hash}`,
      });
    }
  });
  if (hashErrors.length > 0) {
    return { fragment: null, valid: false, errors: hashErrors };
  }

  // Stage 2: known order-insensitive nested-collection canonicalization.
  // Fresh copies only -- raw.definitions/raw.bindings (and their elements)
  // are never mutated.
  const definitions0 = raw.definitions.map(canonicalizeNestedOrder);
  const bindings0 = raw.bindings.map(canonicalizeNestedOrder);
  const evidenceRefs0 = raw.evidence_refs;

  // Stage 3: exact-duplicate normalization + identifier conflict detection.
  const defResult = dedupeByIdOrConflict(definitions0, "capability_definition_id", "CAPABILITY_DEFINITION_ID_CONFLICT");
  const bindResult = dedupeByIdOrConflict(bindings0, "binding_id", "CAPABILITY_BINDING_ID_CONFLICT");
  const evResult = dedupeByIdOrConflict(evidenceRefs0, "evidence_ref_id", "EVIDENCE_REF_ID_CONFLICT");

  const conflictErrors = [...defResult.conflicts, ...bindResult.conflicts, ...evResult.conflicts];

  // capability_key + definition_version natural key (section 22): encoded
  // as a canonicalStringify()'d JSON array -- NOT a delimiter-joined
  // string, which would not be injective for arbitrary namespaced keys.
  const keyVersionMap = new Map();
  for (const def of defResult.deduped) {
    const key = canonicalStringify([def.capability_key, def.definition_version]);
    if (!keyVersionMap.has(key)) keyVersionMap.set(key, new Set());
    keyVersionMap.get(key).add(def.capability_definition_id);
  }
  for (const [key, ids] of keyVersionMap) {
    if (ids.size > 1) {
      conflictErrors.push({
        path: "/definitions",
        message: `CAPABILITY_KEY_VERSION_CONFLICT: natural key ${key} resolves to ${ids.size} different capability_definition_ids`,
      });
    }
  }

  if (conflictErrors.length > 0) {
    return { fragment: null, valid: false, errors: conflictErrors };
  }

  // Stage 4: cross-record referential integrity.
  const referentialErrors = [];
  const definitionById = new Map(defResult.deduped.map((d) => [d.capability_definition_id, d]));
  const evidenceIdSet = new Set(evResult.deduped.map((e) => e.evidence_ref_id));

  defResult.deduped.forEach((def, i) => {
    for (const refId of def.evidence_ref_ids) {
      if (!evidenceIdSet.has(refId)) {
        referentialErrors.push({
          path: `/definitions/${i}/evidence_ref_ids`,
          message: `CAPABILITY_DEFINITION_ORPHAN_EVIDENCE: "${refId}" does not resolve to any evidence_ref in this fragment`,
        });
      }
    }
  });

  bindResult.deduped.forEach((b, i) => {
    const def = definitionById.get(b.capability_definition_id);
    if (!def) {
      referentialErrors.push({
        path: `/bindings/${i}/capability_definition_id`,
        message: `CAPABILITY_BINDING_ORPHAN_DEFINITION: "${b.capability_definition_id}" does not resolve to any definition in this fragment`,
      });
    } else if (b.capability_definition_hash !== def.definition_hash) {
      referentialErrors.push({
        path: `/bindings/${i}/capability_definition_hash`,
        message: `CAPABILITY_BINDING_DEFINITION_HASH_MISMATCH: expected ${def.definition_hash}, got ${b.capability_definition_hash}`,
      });
    }
    for (const refId of b.evidence_ref_ids) {
      if (!evidenceIdSet.has(refId)) {
        referentialErrors.push({
          path: `/bindings/${i}/evidence_ref_ids`,
          message: `CAPABILITY_BINDING_ORPHAN_EVIDENCE: "${refId}" does not resolve to any evidence_ref in this fragment`,
        });
      }
    }
  });

  if (referentialErrors.length > 0) {
    return { fragment: null, valid: false, errors: referentialErrors };
  }

  // Stage 5: TOTAL canonical deterministic sorting + content_hash.
  const definitions = sortItems("definitions", defResult.deduped);
  const bindings = sortItems("bindings", bindResult.deduped);
  const evidence_refs = sortItems("evidence_refs", evResult.deduped);

  const contentDigest = createHash("sha256")
    .update(canonicalStringify({ schema: FRAGMENT_SCHEMA, registration_state: "CANDIDATE_ONLY", definitions, bindings, evidence_refs }))
    .digest("hex");

  // JSON.parse(canonicalStringify(...)) reuses D0's canonicalStringify()
  // (which recursively sorts object keys) to produce a plain object whose
  // own key insertion order already matches the canonical sort -- so the
  // returned fragment's JSON.stringify() bytes never leak whatever key
  // order the provider's raw JSON happened to use, at any nesting level.
  const fragment = JSON.parse(
    canonicalStringify({
      schema: FRAGMENT_SCHEMA,
      registration_state: "CANDIDATE_ONLY",
      definitions,
      bindings,
      evidence_refs,
      content_hash: `sha256:${contentDigest}`,
    })
  );

  const fragmentResult = validate(fragment, FRAGMENT_SCHEMA, ajv);
  if (!fragmentResult.valid) {
    // Defensive: per-record validation above already passed, so this
    // should be unreachable.
    return { fragment: null, valid: false, errors: fragmentResult.errors };
  }

  return { fragment, valid: true, errors: [] };
}

export { canonicalStringify, canonicalPrettyStringify };
