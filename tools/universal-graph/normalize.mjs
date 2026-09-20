#!/usr/bin/env node
// Deterministic CANDIDATE-only admission pipeline for the Universal Graph
// Admission Kernel.
//
// normalizeWorldFragment() takes a raw provider observation and runs the
// required D.CLEAN-0R3 pipeline:
//
//   raw provider observation
//     -> real Ajv Draft-2020-12 RAW-ENVELOPE validation (stage zero): the
//        whole input is checked in one shot against
//        raw-world-observation.schema.json, which is a CLOSED envelope
//        (additionalProperties: false, all four collections required and
//        each strictly an array of the correctly-shaped record). An
//        unknown top-level field (admission_state, canonical, admitted,
//        schema_validated, content_hash, ...), a collection of the wrong
//        type, or a missing collection all fail here -- none of them is
//        ever silently ignored or defaulted to [].
//     -> known unordered-nested-collection canonicalization: the D0
//        contract declares exactly two nested arrays order-insensitive --
//        IdentityLink/SemanticAttachment.evidence_ref_ids (a reference
//        *set*) and SemanticAttachment.semantic_claims (a claim
//        *collection*) -- and sorts fresh copies of each into a
//        deterministic order before anything downstream looks at record
//        content. This never mutates the caller's raw objects, and it
//        never touches a SemanticClaim's own `value` array, whose order
//        is not declared order-insensitive and may be meaningful to a
//        future namespaced claim vocabulary.
//     -> exact-duplicate normalization (content-identical records collapse,
//        now correctly comparing canonicalized content, so two records
//        that differ only in declared-order-insensitive array order are
//        recognized as identical)
//     -> graph identifier conflict detection (entity_id / evidence_ref_id /
//        attachment_id / external-identity-key)
//     -> cross-record referential integrity validation (every reference
//        must resolve inside the same fragment; no orphans)
//     -> TOTAL canonical deterministic sorting: every top-level collection
//        is sorted by its useful primary key, then -- when two distinct
//        valid records share that primary key (e.g. two IdentityLinks for
//        the same entity/provider/namespace/external_id that differ only
//        in evidence_ref_ids or provenance) -- by canonicalStringify() of
//        the whole record as a final tie-breaker. This is a strict total
//        order: source array index is never consulted, and a stable sort's
//        insertion-order fallback never leaks into the output.
//     -> content_hash
//     -> CANDIDATE_ONLY normalized fragment
//
// Any failure at any stage returns { fragment: null, valid: false, errors }.
// There is no fail-open partial candidate.
//
// Passing this pipeline proves structural and referential soundness. It
// does NOT and cannot prove canonical Chronica admission -- the output's
// admission_state is const-locked to "CANDIDATE_ONLY". A provider cannot
// self-admit: nothing in the input schema accepts a canonical/admitted
// flag, and this module never writes one either.
//
// Determinism rules this module must never violate:
//   - never read the current wall-clock time to produce content -- all
//     timestamps come from the input records, not from a live clock read;
//   - never depend on input array order, at the top level OR within a
//     declared order-insensitive nested collection, nor on object key
//     insertion order;
//   - the same input, or any semantically-equivalent reordering of it,
//     must always produce byte-identical canonical JSON and therefore an
//     identical content_hash, run twice, run anywhere.

import { createHash } from "node:crypto";
import { validate } from "./schema-validate.mjs";

const RAW_ENVELOPE_SCHEMA = "chronica.universal-graph.raw-world-observation.v0";

const SORT_KEY = {
  entities: (e) => [e?.entity_id ?? ""],
  identity_links: (l) => [l?.entity_id ?? "", l?.provider_type ?? "", l?.external_namespace ?? "", l?.external_id ?? ""],
  evidence_refs: (r) => [r?.evidence_ref_id ?? ""],
  semantic_attachments: (a) => [a?.entity_id ?? "", a?.attachment_id ?? ""],
};

/** Recursively sorts object keys so JSON.stringify is order-independent. */
function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value !== null && typeof value === "object") {
    const out = {};
    for (const key of Object.keys(value).sort()) out[key] = canonicalize(value[key]);
    return out;
  }
  return value;
}

function canonicalStringify(value) {
  return JSON.stringify(canonicalize(value));
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

/**
 * Total order over a collection: primary key first, then -- for two
 * distinct valid records that legitimately share that key (see
 * dedupeExact()/dedupeByIdOrConflict(), which already collapse
 * exact-content duplicates before this runs) -- the full canonical record
 * content as a strict tie-breaker. Never falls back to source index or
 * sort stability.
 */
function sortItems(kind, items) {
  const keyer = SORT_KEY[kind];
  return items.slice().sort((a, b) => {
    const primary = compareKeys(keyer(a), keyer(b));
    if (primary !== 0) return primary;
    const ca = canonicalStringify(a);
    const cb = canonicalStringify(b);
    if (ca < cb) return -1;
    if (ca > cb) return 1;
    return 0;
  });
}

/** Sorts a fresh copy of a string array lexically; passes through non-arrays unchanged. */
function sortedCopy(arr) {
  return Array.isArray(arr) ? [...arr].sort() : arr;
}

/**
 * Produces a non-mutating copy of `item` with its declared
 * order-insensitive nested collections canonicalized into a deterministic
 * order:
 *   - IdentityLink/SemanticAttachment.evidence_ref_ids: a reference SET,
 *     sorted lexically.
 *   - SemanticAttachment.semantic_claims: a claim COLLECTION, sorted by
 *     canonicalStringify(claim) -- a total order over claims. Each claim's
 *     own `value` array (when value is an array) is left exactly as
 *     supplied: that order is not declared order-insensitive by this
 *     contract and may carry meaning for a future namespaced claim
 *     vocabulary.
 * entities and evidence_refs have no order-insensitive nested arrays in
 * this contract and pass through unchanged.
 */
function canonicalizeNestedOrder(kind, item) {
  if (kind === "identity_links") {
    if (item.evidence_ref_ids === undefined) return item;
    return { ...item, evidence_ref_ids: sortedCopy(item.evidence_ref_ids) };
  }
  if (kind === "semantic_attachments") {
    const out = { ...item };
    if (item.evidence_ref_ids !== undefined) {
      out.evidence_ref_ids = sortedCopy(item.evidence_ref_ids);
    }
    if (Array.isArray(item.semantic_claims)) {
      out.semantic_claims = [...item.semantic_claims].sort((a, b) => {
        const ca = canonicalStringify(a);
        const cb = canonicalStringify(b);
        if (ca < cb) return -1;
        if (ca > cb) return 1;
        return 0;
      });
    }
    return out;
  }
  return item;
}

/**
 * Groups `items` by `idField`, collapsing exact-content duplicates within a
 * group. A group whose members differ in content is a conflict: returns it
 * under `conflicts` instead of `deduped`, using `code` as the error tag.
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

function dedupeExact(items) {
  const seen = new Set();
  const out = [];
  for (const item of items) {
    const key = canonicalStringify(item);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(item);
  }
  return out;
}

/**
 * @param {object} raw - { entities, identity_links, evidence_refs, semantic_attachments }
 * @param {import("ajv").default} ajv - from compileSchemas()
 * @returns {{fragment: object|null, valid: boolean, errors: Array<{path: string, message: string}>}}
 */
export function normalizeWorldFragment(raw, ajv) {
  // Stage 0: real Ajv Draft-2020-12 raw-envelope validation. This is a
  // single validate() call against the closed envelope schema, which
  // itself $ref's into the four per-record schemas for every array item --
  // so it simultaneously catches an unknown top-level field, a
  // wrong-typed or missing collection, AND a structurally invalid record,
  // all in one fail-closed pass. Nothing here is silently ignored,
  // defaulted, or coerced.
  const envelopeResult = validate(raw, RAW_ENVELOPE_SCHEMA, ajv);
  if (!envelopeResult.valid) {
    return { fragment: null, valid: false, errors: envelopeResult.errors };
  }
  // Stage 1: known unordered-nested-collection canonicalization. Produces
  // fresh per-record copies -- raw.identity_links / raw.semantic_attachments
  // (and their elements) are never mutated. Only evidence_ref_ids (a
  // reference set) and semantic_claims (a claim collection) are reordered;
  // everything else, including a claim's own `value` array, is untouched.
  const collected = {
    entities: raw.entities,
    identity_links: raw.identity_links.map((l) => canonicalizeNestedOrder("identity_links", l)),
    evidence_refs: raw.evidence_refs,
    semantic_attachments: raw.semantic_attachments.map((a) => canonicalizeNestedOrder("semantic_attachments", a)),
  };

  // Stage 2: exact-duplicate normalization + graph identifier conflict
  // detection (section 11.D/E/F):
  // exact-duplicate collapse per id-bearing collection, conflicting content
  // under the same id fails closed rather than silently picking one.
  const entitiesResult = dedupeByIdOrConflict(collected.entities, "entity_id", "ENTITY_ID_CONFLICT");
  const evidenceResult = dedupeByIdOrConflict(collected.evidence_refs, "evidence_ref_id", "EVIDENCE_REF_ID_CONFLICT");
  const attachmentsResult = dedupeByIdOrConflict(collected.semantic_attachments, "attachment_id", "ATTACHMENT_ID_CONFLICT");
  const identityLinksDeduped = dedupeExact(collected.identity_links);

  const conflictErrors = [...entitiesResult.conflicts, ...evidenceResult.conflicts, ...attachmentsResult.conflicts];

  // External identity key (provider_type, external_namespace, external_id)
  // must resolve to exactly one entity_id per fragment (section 11.G).
  // Encoded as a canonical JSON array -- NOT a delimiter-joined string.
  // external_namespace and external_id are arbitrary non-empty strings and
  // may themselves contain spaces (or any other delimiter), so a
  // space/colon/slash/pipe-joined string is not injective: distinct tuples
  // like ("future.provider", "tenant alpha", "device") and
  // ("future.provider", "tenant", "alpha device") would collide under a
  // naive join. canonicalStringify() of the 3-element array treats the key
  // as the tuple it is -- JSON's own string escaping and array framing
  // keep every distinct tuple distinct.
  const externalIdentityMap = new Map();
  for (const link of identityLinksDeduped) {
    const key = canonicalStringify([link.provider_type, link.external_namespace, link.external_id]);
    if (!externalIdentityMap.has(key)) externalIdentityMap.set(key, new Set());
    externalIdentityMap.get(key).add(link.entity_id);
  }
  for (const [key, entityIds] of externalIdentityMap) {
    if (entityIds.size > 1) {
      conflictErrors.push({
        path: "/identity_links",
        message: `EXTERNAL_IDENTITY_CONFLICT: external identity ${key} resolves to ${entityIds.size} different entity_ids`,
      });
    }
  }

  if (conflictErrors.length > 0) {
    return { fragment: null, valid: false, errors: conflictErrors };
  }

  // Stage 3: cross-record referential integrity (section 11.A/B/C, 17).
  const referentialErrors = [];
  const entityIdSet = new Set(entitiesResult.deduped.map((e) => e.entity_id));
  const evidenceRefIdSet = new Set(evidenceResult.deduped.map((r) => r.evidence_ref_id));

  identityLinksDeduped.forEach((link, i) => {
    if (!entityIdSet.has(link.entity_id)) {
      referentialErrors.push({ path: `/identity_links/${i}/entity_id`, message: `IDENTITY_LINK_TARGET_ORPHAN: "${link.entity_id}" does not resolve to any entity in this fragment` });
    }
    for (const refId of link.evidence_ref_ids ?? []) {
      if (!evidenceRefIdSet.has(refId)) {
        referentialErrors.push({ path: `/identity_links/${i}/evidence_ref_ids`, message: `EVIDENCE_REF_ID_ORPHAN: "${refId}" does not resolve to any evidence_ref in this fragment` });
      }
    }
  });

  attachmentsResult.deduped.forEach((att, i) => {
    if (!entityIdSet.has(att.entity_id)) {
      referentialErrors.push({ path: `/semantic_attachments/${i}/entity_id`, message: `SEMANTIC_ATTACHMENT_TARGET_ORPHAN: "${att.entity_id}" does not resolve to any entity in this fragment` });
    }
    for (const refId of att.evidence_ref_ids ?? []) {
      if (!evidenceRefIdSet.has(refId)) {
        referentialErrors.push({ path: `/semantic_attachments/${i}/evidence_ref_ids`, message: `EVIDENCE_REF_ID_ORPHAN: "${refId}" does not resolve to any evidence_ref in this fragment` });
      }
    }
    (att.semantic_claims ?? []).forEach((claim, ci) => {
      if (claim.related_entity_id !== undefined && !entityIdSet.has(claim.related_entity_id)) {
        referentialErrors.push({
          path: `/semantic_attachments/${i}/semantic_claims/${ci}/related_entity_id`,
          message: `RELATED_ENTITY_ID_ORPHAN: "${claim.related_entity_id}" does not resolve to any entity in this fragment`,
        });
      }
    });
  });

  if (referentialErrors.length > 0) {
    return { fragment: null, valid: false, errors: referentialErrors };
  }

  // Stage 4: TOTAL canonical deterministic sorting + content hash.
  const entities = sortItems("entities", entitiesResult.deduped);
  const identity_links = sortItems("identity_links", identityLinksDeduped);
  const evidence_refs = sortItems("evidence_refs", evidenceResult.deduped);
  const semantic_attachments = sortItems("semantic_attachments", attachmentsResult.deduped);

  const contentDigest = createHash("sha256")
    .update(canonicalStringify({ entities, identity_links, evidence_refs, semantic_attachments }))
    .digest("hex");

  // The returned fragment's own in-memory object-key insertion order must
  // not leak whatever order the provider's original JSON keys happened to
  // arrive in -- JSON object member order is semantically irrelevant, so
  // two semantically-identical raw observations differing only in key
  // insertion order (at any nesting level) must produce a returned
  // fragment object with identical JSON.stringify() bytes, not just an
  // identical canonicalStringify(). canonicalize() recursively sorts
  // object keys while leaving array element order exactly as already
  // normalized above (it never touches SemanticClaim.value arrays).
  const fragment = canonicalize({
    schema: "chronica.universal-graph.normalized-world-fragment.v0",
    admission_state: "CANDIDATE_ONLY",
    entities,
    identity_links,
    evidence_refs,
    semantic_attachments,
    content_hash: `sha256:${contentDigest}`,
    schema_validated: true,
  });

  const fragmentResult = validate(fragment, "chronica.universal-graph.normalized-world-fragment.v0", ajv);
  if (!fragmentResult.valid) {
    // Defensive: per-record validation above already passed, so this should
    // be unreachable. Surfaced as an error rather than silently emitting an
    // unvalidated fragment.
    return { fragment: null, valid: false, errors: fragmentResult.errors };
  }

  return { fragment, valid: true, errors: [] };
}

/**
 * Explicit canonical-pretty-JSON serializer for public output paths (the
 * CLI). Semantically equivalent to JSON.stringify(canonicalize(value),
 * null, 2) -- deliberately not relying on incidental object-key insertion
 * order, even though normalizeWorldFragment() already returns a
 * canonicalized fragment. A public serialization boundary re-canonicalizes
 * explicitly rather than trusting that whatever object it was handed
 * happens to already be in canonical form.
 */
export function canonicalPrettyStringify(value) {
  return JSON.stringify(canonicalize(value), null, 2);
}

export { canonicalStringify };
