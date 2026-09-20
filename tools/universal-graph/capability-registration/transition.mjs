#!/usr/bin/env node
// evaluateCapabilityRegistrationTransition(): the single fail-closed reference transition from
// a D2 CANDIDATE_ONLY CapabilityRegistryFragment to canonical Chronica capability-registry
// state.
//
//   evaluateCapabilityRegistrationTransition(
//     { raw_capability_observation, request },                        // UNTRUSTED
//     { canonical_world_snapshot,                                     // TRUSTED CANONICAL CONTEXT
//       canonical_capability_registry_snapshot,
//       canonical_registration_authority_snapshot,
//       authenticated_requester_context },
//     ajv,
//   )
//
// D3 answers exactly: "under what explicit canonical registration authority may a
// deterministic D2 capability candidate become canonical capability-registry state, after
// proving its bindings target canonical WorldEntities, and what deterministic commit proves
// that transition?" D3 does NOT answer "who may invoke this capability?" -- registration
// authority (chronica.capability.register) is never capability-use, execution, machine-control,
// external-action, or WorkRequest authority. CAPABILITY CAN BE CANONICAL != CAPABILITY MAY BE
// INVOKED.
//
// The untrusted channel (raw_capability_observation, request) is never read as a source of
// authority, canonical state, policy, or authenticated identity: it is only ever normalized
// (via D2's normalizeCapabilityRegistry, reused unmodified), schema-validated against the
// closed CapabilityRegistrationRequest schema, or checked against trusted context. The trusted
// channel is the only source of registration authority (CanonicalCapabilityRegistrationAuthoritySnapshot,
// looked up by decision id, never accepted inline), the only source of canonical WorldEntity
// membership (CanonicalWorldSnapshot, D1's schema reused read-only), the only source of
// canonical capability-registry state (CanonicalCapabilityRegistrySnapshot), AND the only
// source of authenticated identity (AuthenticatedRequesterContext, D1's schema reused
// read-only).
//
// This is a REFERENCE transition only: it computes next_registry_snapshot/commit in memory and
// returns them. It never writes a production store, calls an external system, or grants
// capability-use/execution authority. See docs/.../capability-registration/v0/README.md.
//
// Pipeline, in the exact order enforced below:
//   validate exact untrusted/trusted wrapper shape
//   -> D2 normalizeCapabilityRegistry() (reused, read-only) -> unreferenced-evidence check
//   -> CapabilityRegistrationRequest schema validation
//   -> AuthenticatedRequesterContext schema validation (D1, reused)
//   -> request.requester_principal_id == authenticated_principal_id (REQUESTER_IDENTITY_MISMATCH)
//   -> candidate_content_hash binding -> requested_{definition,binding,evidence_ref}_ids ==
//      complete candidate sets
//   -> CanonicalWorldSnapshot schema validation (D1, reused) -> entity_id ambiguity check
//      -> every candidate binding.entity_id resolves to exactly one canonical WorldEntity
//   -> CanonicalCapabilityRegistrySnapshot schema validation -> registry LEDGER INTEGRITY
//      (commit id/idempotency_key/request_id/revision-pair uniqueness, historical commit_id
//      re-verification, new-id-resolves-in-current-registry, no double ownership of new ids)
//      -- proven BEFORE any lookup runs
//   -> registry content re-validated through D2's normalizeCapabilityRegistry() (reused) ->
//      registry state_hash verification
//   -> CanonicalCapabilityRegistrationAuthoritySnapshot schema validation -> authority id
//      integrity -> authority canonicalization -> authority state_hash verification
//   -> request fingerprint recomputation
//   -> idempotency replay/conflict check on the now-unambiguous canonical ledger (BEFORE
//      stale-revision checks) -> request_id conflict check -> expected-revision/world-context
//      fencing
//   -> canonical CapabilityRegistrationDecision lookup -> decision/request/candidate/world
//      binding
//   -> CapabilityRegistrationAuthorityGrant resolution: status, policy binding, validity
//      window, capability-namespace scope, resolved-entity-type-namespace scope, relation scope,
//      evidence binding
//   -> Separation of Duties using the TRUSTED authenticated identity
//   -> DENY short-circuits to the canonical (order-independent) unchanged registry state
//   -> ALLOW: candidate-vs-canonical merge classification (existing-identical no-op / new /
//      conflict) -> all-or-nothing conflict check -> ALREADY_REGISTERED short-circuit when
//      nothing is new -> revision-overflow check
//   -> deterministic CapabilityRegistrationCommit -> registry_revision+1 -> new canonical
//      registry snapshot
//
// Any failure at any stage returns { status: "REJECTED", commit: null, next_registry_snapshot:
// null, errors: [{code, path, message}] }. Never mutates raw_capability_observation, request,
// canonical_world_snapshot, canonical_capability_registry_snapshot,
// canonical_registration_authority_snapshot, or authenticated_requester_context in place.

import { createHash } from "node:crypto";
import { normalizeCapabilityRegistry, canonicalStringify } from "../capability/normalize.mjs";
import { validate } from "../schema-validate.mjs";
import { verifyCanonicalWorldSnapshot } from "../admission/transition.mjs";

const REQUEST_SCHEMA = "chronica.universal-graph.capability-registration.capability-registration-request.v0";
const AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA = "chronica.universal-graph.admission.authenticated-requester-context.v0";
const CANONICAL_CAPABILITY_REGISTRY_SNAPSHOT_SCHEMA = "chronica.universal-graph.capability-registration.canonical-capability-registry-snapshot.v0";
const CANONICAL_CAPABILITY_REGISTRATION_AUTHORITY_SNAPSHOT_SCHEMA =
  "chronica.universal-graph.capability-registration.canonical-capability-registration-authority-snapshot.v0";

function isPlainObject(v) {
  return v !== null && typeof v === "object" && !Array.isArray(v);
}

function hasExactKeys(obj, keys) {
  if (!isPlainObject(obj)) return false;
  const objKeys = Object.keys(obj);
  if (objKeys.length !== keys.length) return false;
  return keys.every((k) => Object.prototype.hasOwnProperty.call(obj, k));
}

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value !== null && typeof value === "object") {
    const out = {};
    for (const key of Object.keys(value).sort()) out[key] = canonicalize(value[key]);
    return out;
  }
  return value;
}

function sha256(value) {
  return `sha256:${createHash("sha256").update(canonicalStringify(value)).digest("hex")}`;
}

function compareKeyParts(a, b) {
  if (typeof a === "number" && typeof b === "number") return a - b;
  const as = String(a);
  const bs = String(b);
  if (as < bs) return -1;
  if (as > bs) return 1;
  return 0;
}

function compareKeys(a, b) {
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const c = compareKeyParts(a[i] ?? "", b[i] ?? "");
    if (c !== 0) return c;
  }
  return 0;
}

/** Total order: primary key, then canonicalStringify() content as a strict tie-breaker. */
function sortByKeyThenContent(items, keyFn) {
  return items
    .slice()
    .sort((a, b) => compareKeys(keyFn(a), keyFn(b)) || compareKeys([canonicalStringify(a)], [canonicalStringify(b)]));
}

/**
 * Groups canonicalized `items` by `idField` and classifies any id that appears more than once:
 * DUPLICATE if every occurrence is byte-identical canonical content, CONFLICT if they differ.
 * Returns null when every id is unique. Proves a trusted collection unambiguous BEFORE any
 * lookup reads it -- order of the input never affects the result.
 */
function findIdDuplicateOrConflict(canonicalItems, idField) {
  const byId = new Map();
  for (const item of canonicalItems) {
    const key = item[idField];
    if (!byId.has(key)) byId.set(key, []);
    byId.get(key).push(canonicalStringify(item));
  }
  for (const [id, contents] of byId) {
    if (contents.length <= 1) continue;
    return { id, kind: new Set(contents).size > 1 ? "CONFLICT" : "DUPLICATE" };
  }
  return null;
}

/** Returns the first value of `field` that repeats across `items`, or null if all are unique. */
function findDuplicateValue(items, field) {
  const seen = new Set();
  for (const item of items) {
    if (seen.has(item[field])) return item[field];
    seen.add(item[field]);
  }
  return null;
}

/** True if any value under `field` repeats across two or more different commits. */
function hasCrossCommitDuplicate(commits, field) {
  const seen = new Set();
  for (const c of commits) {
    for (const id of c[field]) {
      if (seen.has(id)) return true;
      seen.add(id);
    }
  }
  return false;
}

function rootNamespace(namespacedValue) {
  return namespacedValue.split(".")[0];
}

function policyRefEqual(a, b) {
  return a.scheme === b.scheme && a.value === b.value;
}

function rejected(code, message, path = "/") {
  return { status: "REJECTED", commit: null, next_registry_snapshot: null, errors: [{ code, path, message: message ?? code }] };
}

function unchanged(status, commit, canonicalRegistry) {
  return { status, commit, next_registry_snapshot: canonicalRegistry, errors: [] };
}

// ---------------------------------------------------------------------------
// Canonicalization -- declared order-insensitive set fields, per record kind.
// ---------------------------------------------------------------------------

function canonicalizeGrant(grant) {
  return {
    ...grant,
    allowed_capability_namespaces: [...grant.allowed_capability_namespaces].sort(),
    allowed_entity_type_namespaces: [...grant.allowed_entity_type_namespaces].sort(),
    allowed_relations: [...grant.allowed_relations].sort(),
  };
}

function canonicalizeDecision(decision) {
  return { ...decision, evidence_ref_ids: [...decision.evidence_ref_ids].sort() };
}

function canonicalizeCommit(commit) {
  return {
    ...commit,
    evidence_ref_ids: [...commit.evidence_ref_ids].sort(),
    candidate_definition_ids: [...commit.candidate_definition_ids].sort(),
    candidate_binding_ids: [...commit.candidate_binding_ids].sort(),
    candidate_evidence_ref_ids: [...commit.candidate_evidence_ref_ids].sort(),
    new_definition_ids: [...commit.new_definition_ids].sort(),
    new_binding_ids: [...commit.new_binding_ids].sort(),
    new_evidence_ref_ids: [...commit.new_evidence_ref_ids].sort(),
  };
}

/** requested_{definition,binding,evidence_ref}_ids are declared order-insensitive sets. */
function canonicalizeRequest(request) {
  return {
    ...request,
    requested_definition_ids: [...request.requested_definition_ids].sort(),
    requested_binding_ids: [...request.requested_binding_ids].sort(),
    requested_evidence_ref_ids: [...request.requested_evidence_ref_ids].sort(),
  };
}

function sortDefinitions(items) {
  return sortByKeyThenContent(items, (d) => [d.capability_definition_id]);
}
function sortBindings(items) {
  return sortByKeyThenContent(items, (b) => [b.binding_id]);
}
function sortEvidenceRefs(items) {
  return sortByKeyThenContent(items, (e) => [e.evidence_ref_id]);
}
function sortCommits(items) {
  return sortByKeyThenContent(items.map(canonicalizeCommit), (c) => [c.commit_id]);
}
function sortGrants(items) {
  return sortByKeyThenContent(items.map(canonicalizeGrant), (g) => [g.grant_id]);
}
function sortDecisions(items) {
  return sortByKeyThenContent(items.map(canonicalizeDecision), (d) => [d.decision_id]);
}

export function computeRequestFingerprint(request) {
  return sha256(canonicalizeRequest(request));
}

/** The single deterministic derivation used both to mint a new commit_id and to re-verify every historical commit already in the ledger. */
export function computeCommitId({ request_fingerprint, candidate_content_hash, authority_decision_id, registry_revision_before, registry_revision_after }) {
  return sha256({ request_fingerprint, candidate_content_hash, authority_decision_id, registry_revision_before, registry_revision_after });
}

/** Acyclic content-only hash over {definitions, bindings, evidence_refs} -- excludes registration_commits, registry_revision, and state_hash so no self-reference is possible. */
export function computeRegistryContentHash(definitions, bindings, evidence_refs) {
  return sha256({ definitions: sortDefinitions(definitions), bindings: sortBindings(bindings), evidence_refs: sortEvidenceRefs(evidence_refs) });
}

export function computeRegistryStateHash(snapshotLike) {
  return sha256({
    registry_revision: snapshotLike.registry_revision,
    definitions: sortDefinitions(snapshotLike.definitions),
    bindings: sortBindings(snapshotLike.bindings),
    evidence_refs: sortEvidenceRefs(snapshotLike.evidence_refs),
    registration_commits: sortCommits(snapshotLike.registration_commits),
  });
}

export function computeAuthoritySnapshotStateHash(snapshotLike) {
  return sha256({ authority_revision: snapshotLike.authority_revision, grants: sortGrants(snapshotLike.grants), decisions: sortDecisions(snapshotLike.decisions) });
}

/**
 * Proves a schema-valid CanonicalCapabilityRegistrySnapshot's registration_commits ledger is
 * unambiguous BEFORE any replay/idempotency lookup is allowed to read it: exactly one commit
 * per commit_id/idempotency_key/request_id, internally consistent revision pairs, every
 * historical commit_id a real recomputation of its own semantic fields, every id a commit
 * claims to have newly registered still resolving in the current registry, and no id claimed
 * as "newly registered" by more than one historical commit.
 */
function validateRegistryLedgerIntegrity(snapshot) {
  const commits = snapshot.registration_commits;
  const canonicalCommitsList = commits.map(canonicalizeCommit);

  const commitIdConflict = findIdDuplicateOrConflict(canonicalCommitsList, "commit_id");
  if (commitIdConflict) return commitIdConflict.kind === "DUPLICATE" ? "CAPABILITY_REGISTRATION_COMMIT_ID_DUPLICATE" : "CAPABILITY_REGISTRATION_COMMIT_ID_CONFLICT";

  if (findDuplicateValue(commits, "idempotency_key") !== null) return "CAPABILITY_REGISTRATION_IDEMPOTENCY_LEDGER_CONFLICT";
  if (findDuplicateValue(commits, "request_id") !== null) return "CAPABILITY_REGISTRATION_REQUEST_LEDGER_CONFLICT";

  for (const c of commits) {
    const expectedAfter = c.registry_revision_before + 1;
    if (!Number.isSafeInteger(expectedAfter) || expectedAfter !== c.registry_revision_after || c.registry_revision_after > snapshot.registry_revision) {
      return "CAPABILITY_REGISTRATION_COMMIT_REVISION_INVALID";
    }
  }
  if (findDuplicateValue(commits, "registry_revision_before") !== null) return "CAPABILITY_REGISTRATION_REVISION_LEDGER_CONFLICT";
  if (findDuplicateValue(commits, "registry_revision_after") !== null) return "CAPABILITY_REGISTRATION_REVISION_LEDGER_CONFLICT";

  for (const c of commits) {
    if (computeCommitId(c) !== c.commit_id) return "CAPABILITY_REGISTRATION_COMMIT_ID_MISMATCH";
  }

  const definitionIdSet = new Set(snapshot.definitions.map((d) => d.capability_definition_id));
  const bindingIdSet = new Set(snapshot.bindings.map((b) => b.binding_id));
  const evidenceIdSet = new Set(snapshot.evidence_refs.map((e) => e.evidence_ref_id));
  for (const c of commits) {
    for (const id of c.new_definition_ids) if (!definitionIdSet.has(id)) return "CAPABILITY_REGISTRATION_COMMIT_NEW_DEFINITION_ID_MISSING";
    for (const id of c.new_binding_ids) if (!bindingIdSet.has(id)) return "CAPABILITY_REGISTRATION_COMMIT_NEW_BINDING_ID_MISSING";
    for (const id of c.new_evidence_ref_ids) if (!evidenceIdSet.has(id)) return "CAPABILITY_REGISTRATION_COMMIT_NEW_EVIDENCE_ID_MISSING";
  }

  if (hasCrossCommitDuplicate(commits, "new_definition_ids")) return "CAPABILITY_REGISTRATION_NEW_DEFINITION_ID_DOUBLE_OWNERSHIP";
  if (hasCrossCommitDuplicate(commits, "new_binding_ids")) return "CAPABILITY_REGISTRATION_NEW_BINDING_ID_DOUBLE_OWNERSHIP";
  if (hasCrossCommitDuplicate(commits, "new_evidence_ref_ids")) return "CAPABILITY_REGISTRATION_NEW_EVIDENCE_ID_DOUBLE_OWNERSHIP";

  return null;
}

/**
 * Candidate-vs-canonical merge classification for one id-bearing collection (Section 30):
 * an absent id (and, for definitions, no natural-key collision under a different id) is
 * eligible NEW; a present id with byte-identical canonical content is an existing no-op; a
 * present id (or, for definitions, a natural-key match under a different id) with different
 * content is a conflict. `extra` optionally supplies definition-specific natural-key logic.
 */
function classifyRecords(candidateItems, existingItems, idField, conflictCode, naturalKey) {
  const existingById = new Map(existingItems.map((item) => [item[idField], item]));
  const existingByNaturalKey = new Map();
  if (naturalKey) {
    for (const item of existingItems) existingByNaturalKey.set(canonicalStringify(naturalKey(item)), item[idField]);
  }
  const conflicts = [];
  const newIds = [];
  for (const item of candidateItems) {
    const existing = existingById.get(item[idField]);
    if (existing) {
      if (canonicalStringify(existing) !== canonicalStringify(item)) {
        conflicts.push({ code: conflictCode, id: item[idField] });
      }
      continue;
    }
    if (naturalKey) {
      const key = canonicalStringify(naturalKey(item));
      const conflictingId = existingByNaturalKey.get(key);
      if (conflictingId !== undefined && conflictingId !== item[idField]) {
        conflicts.push({ code: "CAPABILITY_KEY_VERSION_CONFLICT", id: item[idField] });
        continue;
      }
    }
    newIds.push(item[idField]);
  }
  return { conflicts, newIds };
}

/**
 * @param {{raw_capability_observation: object, request: object}} untrusted
 * @param {{canonical_world_snapshot: object, canonical_capability_registry_snapshot: object, canonical_registration_authority_snapshot: object, authenticated_requester_context: object}} trusted
 * @param {import("ajv").default} ajv - from compileCapabilityRegistrationSchemas()
 * @returns {object} CapabilityRegistrationTransitionResult
 */
export function evaluateCapabilityRegistrationTransition(untrusted, trusted, ajv) {
  if (!hasExactKeys(untrusted, ["raw_capability_observation", "request"])) {
    return rejected("UNTRUSTED_CHANNEL_UNKNOWN_FIELD", "untrusted wrapper must contain exactly {raw_capability_observation, request}");
  }
  if (!hasExactKeys(trusted, ["canonical_world_snapshot", "canonical_capability_registry_snapshot", "canonical_registration_authority_snapshot", "authenticated_requester_context"])) {
    return rejected(
      "TRUSTED_CHANNEL_UNKNOWN_FIELD",
      "trusted wrapper must contain exactly {canonical_world_snapshot, canonical_capability_registry_snapshot, canonical_registration_authority_snapshot, authenticated_requester_context}"
    );
  }
  const { raw_capability_observation, request } = untrusted;
  const { canonical_world_snapshot, canonical_capability_registry_snapshot, canonical_registration_authority_snapshot, authenticated_requester_context } = trusted;

  // D2 is reused, read-only, as the ONLY capability-candidate normalizer (section 8): no
  // alternate/second normalizer is implemented here.
  const candidateResult = normalizeCapabilityRegistry(raw_capability_observation, ajv);
  if (!candidateResult.valid) return rejected("D2_CANDIDATE_INVALID", JSON.stringify(candidateResult.errors));
  const candidate = candidateResult.fragment;

  // No unreferenced candidate evidence may hitchhike in (section 31): every evidence_ref must
  // be referenced by at least one candidate definition or binding.
  const referencedEvidenceIds = new Set();
  for (const def of candidate.definitions) for (const id of def.evidence_ref_ids) referencedEvidenceIds.add(id);
  for (const b of candidate.bindings) for (const id of b.evidence_ref_ids) referencedEvidenceIds.add(id);
  const unreferencedEvidence = candidate.evidence_refs.find((e) => !referencedEvidenceIds.has(e.evidence_ref_id));
  if (unreferencedEvidence) {
    return rejected("UNREFERENCED_CANDIDATE_EVIDENCE", `evidence_ref "${unreferencedEvidence.evidence_ref_id}" is not referenced by any candidate definition or binding`, "/evidence_refs");
  }

  const requestValidation = validate(request, REQUEST_SCHEMA, ajv);
  if (!requestValidation.valid) return rejected("CAPABILITY_REGISTRATION_REQUEST_SCHEMA_INVALID", JSON.stringify(requestValidation.errors));

  const contextValidation = validate(authenticated_requester_context, AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA, ajv);
  if (!contextValidation.valid) return rejected("AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA_INVALID", JSON.stringify(contextValidation.errors));

  // Requester identity binding: proven before any replay/DENY/COMMIT/ALREADY_REGISTERED result
  // can be produced (section 11).
  if (request.requester_principal_id !== authenticated_requester_context.authenticated_principal_id) {
    return rejected("REQUESTER_IDENTITY_MISMATCH", "request.requester_principal_id does not match authenticated_requester_context.authenticated_principal_id", "/requester_principal_id");
  }

  if (request.candidate_content_hash !== candidate.content_hash) {
    return rejected("CANDIDATE_CONTENT_HASH_MISMATCH", "request.candidate_content_hash does not equal the D2 candidate's content_hash");
  }

  const candidateDefinitionIds = new Set(candidate.definitions.map((d) => d.capability_definition_id));
  const requestedDefinitionIds = new Set(request.requested_definition_ids);
  if ([...candidateDefinitionIds].some((id) => !requestedDefinitionIds.has(id))) {
    return rejected("REQUESTED_DEFINITION_SET_INCOMPLETE", "requested_definition_ids is missing one or more candidate definition ids", "/requested_definition_ids");
  }
  if ([...requestedDefinitionIds].some((id) => !candidateDefinitionIds.has(id))) {
    return rejected("REQUESTED_DEFINITION_SET_HAS_EXTRA", "requested_definition_ids names an id outside the candidate's definition set", "/requested_definition_ids");
  }

  const candidateBindingIds = new Set(candidate.bindings.map((b) => b.binding_id));
  const requestedBindingIds = new Set(request.requested_binding_ids);
  if ([...candidateBindingIds].some((id) => !requestedBindingIds.has(id))) {
    return rejected("REQUESTED_BINDING_SET_INCOMPLETE", "requested_binding_ids is missing one or more candidate binding ids", "/requested_binding_ids");
  }
  if ([...requestedBindingIds].some((id) => !candidateBindingIds.has(id))) {
    return rejected("REQUESTED_BINDING_SET_HAS_EXTRA", "requested_binding_ids names an id outside the candidate's binding set", "/requested_binding_ids");
  }

  const candidateEvidenceIds = new Set(candidate.evidence_refs.map((e) => e.evidence_ref_id));
  const requestedEvidenceIds = new Set(request.requested_evidence_ref_ids);
  if ([...candidateEvidenceIds].some((id) => !requestedEvidenceIds.has(id))) {
    return rejected("REQUESTED_EVIDENCE_SET_INCOMPLETE", "requested_evidence_ref_ids is missing one or more candidate evidence ids", "/requested_evidence_ref_ids");
  }
  if ([...requestedEvidenceIds].some((id) => !candidateEvidenceIds.has(id))) {
    return rejected("REQUESTED_EVIDENCE_SET_HAS_EXTRA", "requested_evidence_ref_ids names an id outside the candidate's evidence set", "/requested_evidence_ref_ids");
  }

  // Canonical WorldEntity state is verified through D1's own exported verifier (section 2 of
  // D.CLEAN-3R) -- there is exactly ONE canonical-world-verification implementation, reused
  // read-only, not a second reimplementation of D1's schema/ledger-integrity/state_hash checks.
  // A forged or internally-ambiguous world snapshot (duplicate/conflicting entity_id, a forged
  // historical AdmissionCommit, a corrupted state_hash, ...) is rejected here, before any
  // candidate binding is resolved against it.
  const worldVerification = verifyCanonicalWorldSnapshot(canonical_world_snapshot, ajv);
  if (!worldVerification.valid) return rejected(worldVerification.code, worldVerification.message, worldVerification.path);
  const canonicalWorld = worldVerification.canonical_world_snapshot;

  const registrySchemaResult = validate(canonical_capability_registry_snapshot, CANONICAL_CAPABILITY_REGISTRY_SNAPSHOT_SCHEMA, ajv);
  if (!registrySchemaResult.valid) return rejected("CANONICAL_CAPABILITY_REGISTRY_SNAPSHOT_SCHEMA_INVALID", JSON.stringify(registrySchemaResult.errors));

  // Canonical registry record-id uniqueness (section 8 of D.CLEAN-3R): a canonical registry
  // snapshot -- unlike an untrusted D2 candidate -- may never contain two physical records
  // (even byte-identical ones) under the same id. D2's own normalizeCapabilityRegistry()
  // deliberately COLLAPSES exact-duplicate candidate records; that collapsing semantic is wrong
  // for something already claiming to be canonical, so this is checked BEFORE the registry's
  // content is ever handed to D2 below, using the same order-independent duplicate/conflict
  // detector already used for canonical world entities and authority grants/decisions.
  const definitionIdConflict = findIdDuplicateOrConflict(canonical_capability_registry_snapshot.definitions, "capability_definition_id");
  if (definitionIdConflict) {
    return rejected(
      definitionIdConflict.kind === "DUPLICATE" ? "CANONICAL_CAPABILITY_DEFINITION_ID_DUPLICATE" : "CANONICAL_CAPABILITY_DEFINITION_ID_CONFLICT",
      `capability_definition_id "${definitionIdConflict.id}" is ambiguous in the supplied canonical registry snapshot`,
      "/canonical_capability_registry_snapshot/definitions"
    );
  }
  const registryBindingIdConflict = findIdDuplicateOrConflict(canonical_capability_registry_snapshot.bindings, "binding_id");
  if (registryBindingIdConflict) {
    return rejected(
      registryBindingIdConflict.kind === "DUPLICATE" ? "CANONICAL_CAPABILITY_BINDING_ID_DUPLICATE" : "CANONICAL_CAPABILITY_BINDING_ID_CONFLICT",
      `binding_id "${registryBindingIdConflict.id}" is ambiguous in the supplied canonical registry snapshot`,
      "/canonical_capability_registry_snapshot/bindings"
    );
  }
  const registryEvidenceIdConflict = findIdDuplicateOrConflict(canonical_capability_registry_snapshot.evidence_refs, "evidence_ref_id");
  if (registryEvidenceIdConflict) {
    return rejected(
      registryEvidenceIdConflict.kind === "DUPLICATE" ? "CANONICAL_CAPABILITY_EVIDENCE_REF_ID_DUPLICATE" : "CANONICAL_CAPABILITY_EVIDENCE_REF_ID_CONFLICT",
      `evidence_ref_id "${registryEvidenceIdConflict.id}" is ambiguous in the supplied canonical registry snapshot`,
      "/canonical_capability_registry_snapshot/evidence_refs"
    );
  }

  const registryLedgerErrorCode = validateRegistryLedgerIntegrity(canonical_capability_registry_snapshot);
  if (registryLedgerErrorCode) return rejected(registryLedgerErrorCode, "canonical capability registry ledger integrity check failed", "/canonical_capability_registry_snapshot/registration_commits");

  // Canonical registry content re-validated through D2's own normalizer -- no second
  // capability-candidate/registry normalizer is implemented in D3 (section 15).
  const registryContentResult = normalizeCapabilityRegistry(
    { definitions: canonical_capability_registry_snapshot.definitions, bindings: canonical_capability_registry_snapshot.bindings, evidence_refs: canonical_capability_registry_snapshot.evidence_refs },
    ajv
  );
  if (!registryContentResult.valid) {
    return rejected("CANONICAL_CAPABILITY_REGISTRY_CONTENT_INVALID", JSON.stringify(registryContentResult.errors), "/canonical_capability_registry_snapshot");
  }
  const existingDefinitions = registryContentResult.fragment.definitions;
  const existingBindings = registryContentResult.fragment.bindings;
  const existingEvidenceRefs = registryContentResult.fragment.evidence_refs;

  const canonicalCommitsList = sortCommits(canonical_capability_registry_snapshot.registration_commits);
  const registryStateHashPre = { registry_revision: canonical_capability_registry_snapshot.registry_revision, definitions: existingDefinitions, bindings: existingBindings, evidence_refs: existingEvidenceRefs, registration_commits: canonicalCommitsList };
  if (computeRegistryStateHash(registryStateHashPre) !== canonical_capability_registry_snapshot.state_hash) {
    return rejected("CANONICAL_CAPABILITY_REGISTRY_STATE_HASH_MISMATCH", "recomputed registry state_hash does not match the supplied snapshot");
  }
  const canonicalRegistry = canonicalize({
    schema: CANONICAL_CAPABILITY_REGISTRY_SNAPSHOT_SCHEMA,
    registry_revision: canonical_capability_registry_snapshot.registry_revision,
    definitions: existingDefinitions,
    bindings: existingBindings,
    evidence_refs: existingEvidenceRefs,
    registration_commits: canonicalCommitsList,
    state_hash: canonical_capability_registry_snapshot.state_hash,
  });

  const authoritySchemaResult = validate(canonical_registration_authority_snapshot, CANONICAL_CAPABILITY_REGISTRATION_AUTHORITY_SNAPSHOT_SCHEMA, ajv);
  if (!authoritySchemaResult.valid) return rejected("CANONICAL_CAPABILITY_REGISTRATION_AUTHORITY_SNAPSHOT_SCHEMA_INVALID", JSON.stringify(authoritySchemaResult.errors));

  const grantConflict = findIdDuplicateOrConflict(canonical_registration_authority_snapshot.grants.map(canonicalizeGrant), "grant_id");
  if (grantConflict) return rejected(grantConflict.kind === "DUPLICATE" ? "REGISTRATION_AUTHORITY_GRANT_ID_DUPLICATE" : "REGISTRATION_AUTHORITY_GRANT_ID_CONFLICT", "two grants share a grant_id", "/canonical_registration_authority_snapshot/grants");
  const decisionConflict = findIdDuplicateOrConflict(canonical_registration_authority_snapshot.decisions.map(canonicalizeDecision), "decision_id");
  if (decisionConflict) return rejected(decisionConflict.kind === "DUPLICATE" ? "REGISTRATION_AUTHORITY_DECISION_ID_DUPLICATE" : "REGISTRATION_AUTHORITY_DECISION_ID_CONFLICT", "two decisions share a decision_id", "/canonical_registration_authority_snapshot/decisions");

  const canonicalGrantList = sortGrants(canonical_registration_authority_snapshot.grants);
  const canonicalDecisionList = sortDecisions(canonical_registration_authority_snapshot.decisions);
  if (
    computeAuthoritySnapshotStateHash({ authority_revision: canonical_registration_authority_snapshot.authority_revision, grants: canonicalGrantList, decisions: canonicalDecisionList }) !==
    canonical_registration_authority_snapshot.state_hash
  ) {
    return rejected("REGISTRATION_AUTHORITY_SNAPSHOT_HASH_MISMATCH", "recomputed registration authority state_hash does not match the supplied snapshot");
  }

  const requestFingerprint = computeRequestFingerprint(request);

  // Idempotency replay/conflict check runs on the now-proven-unambiguous canonical ledger, and
  // BEFORE any stale-revision/world-context check (section 34/36).
  const priorByIdemKey = canonicalCommitsList.find((c) => c.idempotency_key === request.idempotency_key);
  if (priorByIdemKey) {
    if (priorByIdemKey.request_fingerprint === requestFingerprint && priorByIdemKey.candidate_content_hash === candidate.content_hash) {
      return unchanged("REPLAY", canonicalize(priorByIdemKey), canonicalRegistry);
    }
    return rejected("IDEMPOTENCY_KEY_REUSE_CONFLICT", "idempotency_key was already used for a different request_fingerprint/candidate_content_hash", "/idempotency_key");
  }

  const priorByRequestId = canonicalCommitsList.find((c) => c.request_id === request.request_id);
  if (priorByRequestId && priorByRequestId.request_fingerprint !== requestFingerprint) {
    return rejected("REQUEST_ID_CONFLICT", "request_id was already committed for a different request_fingerprint", "/request_id");
  }

  if (request.expected_registry_revision !== canonical_capability_registry_snapshot.registry_revision) {
    return rejected("STALE_REGISTRY_REVISION", "expected_registry_revision does not match the current canonical registry_revision", "/expected_registry_revision");
  }
  // Every semantic world read past the trust boundary uses ONLY canonicalWorld (D1-verified),
  // never the raw supplied canonical_world_snapshot (section 6 of D.CLEAN-3R2) -- the two are
  // proven equal by verifyCanonicalWorldSnapshot() above, but canonicalWorld is the source of
  // record, not the raw value that merely happens to agree.
  if (request.expected_world_revision !== canonicalWorld.world_revision) {
    return rejected("STALE_WORLD_REVISION", "expected_world_revision does not match the current canonical world_revision", "/expected_world_revision");
  }
  if (request.expected_world_state_hash !== canonicalWorld.state_hash) {
    return rejected("STALE_WORLD_STATE_HASH", "expected_world_state_hash does not match the current canonical world state_hash", "/expected_world_state_hash");
  }

  // Canonical WorldEntity membership check for every candidate binding (sections 12/13) is
  // NEW-commit authorization work, not trusted-context validity -- it runs only once we know
  // this is not an idempotent replay (an exact retry of an already-committed registration is
  // not a new authorization decision and must not be re-blocked by a WorldEntity that has since
  // been retired from the CURRENT world snapshot). The world snapshot's own integrity was
  // already fully proven above via verifyCanonicalWorldSnapshot(), unconditionally, before the
  // replay lookup. The resolved WorldEntity -- never a provider-supplied or inferred type --
  // is what grant entity-type scope reads below.
  const entityById = new Map(canonicalWorld.entities.map((e) => [e.entity_id, e]));
  const resolvedBindingEntity = new Map();
  for (const b of candidate.bindings) {
    const entity = entityById.get(b.entity_id);
    if (!entity) {
      return rejected("CAPABILITY_BINDING_ENTITY_NOT_CANONICAL", `binding "${b.binding_id}" references entity_id "${b.entity_id}", which is not a canonical WorldEntity`, "/bindings");
    }
    resolvedBindingEntity.set(b.binding_id, entity);
  }

  const decision = canonicalDecisionList.find((d) => d.decision_id === request.authority_decision_id);
  if (!decision) return rejected("MISSING_REGISTRATION_DECISION", "authority_decision_id does not resolve in the canonical registration authority snapshot", "/authority_decision_id");

  if (decision.request_id !== request.request_id) return rejected("DECISION_REQUEST_ID_MISMATCH", "decision.request_id does not match request.request_id");
  if (decision.request_fingerprint !== requestFingerprint) return rejected("REQUEST_FINGERPRINT_MISMATCH", "decision.request_fingerprint does not match the recomputed request fingerprint");
  if (decision.candidate_content_hash !== candidate.content_hash) return rejected("DECISION_CANDIDATE_CONTENT_HASH_MISMATCH", "decision.candidate_content_hash does not match the D2 candidate's content_hash");
  if (decision.world_revision !== request.expected_world_revision) return rejected("DECISION_WORLD_REVISION_MISMATCH", "decision.world_revision does not match request.expected_world_revision");
  if (decision.world_state_hash !== request.expected_world_state_hash) return rejected("DECISION_WORLD_STATE_HASH_MISMATCH", "decision.world_state_hash does not match request.expected_world_state_hash");

  const grant = canonicalGrantList.find((g) => g.grant_id === decision.grant_id);
  if (!grant) return rejected("MISSING_REGISTRATION_GRANT", "decision.grant_id does not resolve in the canonical registration authority snapshot");
  if (decision.authority_principal_id !== grant.authority_principal_id) return rejected("DECISION_GRANT_PRINCIPAL_MISMATCH", "decision.authority_principal_id does not match grant.authority_principal_id");
  if (grant.status !== "ACTIVE") return rejected("GRANT_NOT_ACTIVE", `grant.status is "${grant.status}", not ACTIVE`);
  if (!policyRefEqual(decision.policy_ref, grant.policy_ref)) return rejected("POLICY_REF_MISMATCH", "decision.policy_ref does not match grant.policy_ref");
  if (decision.policy_hash !== grant.policy_hash) return rejected("POLICY_HASH_MISMATCH", "decision.policy_hash does not match grant.policy_hash");
  if (!(decision.decided_at >= grant.valid_from && decision.decided_at < grant.valid_until)) {
    return rejected("GRANT_VALIDITY_WINDOW_VIOLATION", "decision.decided_at falls outside [grant.valid_from, grant.valid_until)");
  }

  const capabilityNamespaces = new Set(candidate.definitions.map((d) => rootNamespace(d.capability_key)));
  for (const ns of capabilityNamespaces) {
    if (!grant.allowed_capability_namespaces.includes(ns)) {
      return rejected("CAPABILITY_NAMESPACE_NOT_ALLOWED", `capability root namespace "${ns}" is not in grant.allowed_capability_namespaces`, "/requested_definition_ids");
    }
  }

  const entityTypeNamespaces = new Set(candidate.bindings.map((b) => rootNamespace(resolvedBindingEntity.get(b.binding_id).entity_type)));
  for (const ns of entityTypeNamespaces) {
    if (!grant.allowed_entity_type_namespaces.includes(ns)) {
      return rejected("ENTITY_TYPE_NAMESPACE_NOT_ALLOWED", `resolved entity_type root namespace "${ns}" is not in grant.allowed_entity_type_namespaces`, "/requested_binding_ids");
    }
  }

  for (const b of candidate.bindings) {
    if (!grant.allowed_relations.includes(b.relation)) {
      return rejected("BINDING_RELATION_NOT_ALLOWED", `binding "${b.binding_id}" relation "${b.relation}" is not in grant.allowed_relations`, "/requested_binding_ids");
    }
  }

  const orphanEvidenceId = decision.evidence_ref_ids.find((id) => !candidateEvidenceIds.has(id));
  if (orphanEvidenceId) {
    return rejected("ORPHAN_EVIDENCE_REF", `decision.evidence_ref_ids "${orphanEvidenceId}" does not resolve in the D2 candidate's evidence_refs`, "/canonical_registration_authority_snapshot/decisions");
  }
  if (decision.effect === "ALLOW" && decision.evidence_ref_ids.length === 0) {
    return rejected("ALLOW_ZERO_EVIDENCE", "an ALLOW decision requires at least one evidence_ref_id");
  }

  // SoD uses the TRUSTED authenticated identity, never the untrusted request field directly.
  if (grant.self_approval_policy === "DISTINCT_REQUIRED" && authenticated_requester_context.authenticated_principal_id === decision.authority_principal_id) {
    return rejected("SELF_APPROVAL_VIOLATION", "grant.self_approval_policy is DISTINCT_REQUIRED but the authenticated requester equals decision.authority_principal_id");
  }

  if (decision.effect === "DENY") {
    return unchanged("DENIED", null, canonicalRegistry);
  }

  // Candidate-vs-canonical merge classification (section 30): existing identical records are
  // no-ops (so an existing definition + a genuinely new binding to it can still register), any
  // conflict fails the whole transition closed with zero mutation (section 32).
  const definitionClassification = classifyRecords(candidate.definitions, existingDefinitions, "capability_definition_id", "CAPABILITY_DEFINITION_ID_CONFLICT", (d) => [d.capability_key, d.definition_version]);
  const bindingClassification = classifyRecords(candidate.bindings, existingBindings, "binding_id", "CAPABILITY_BINDING_ID_CONFLICT", null);
  const evidenceClassification = classifyRecords(candidate.evidence_refs, existingEvidenceRefs, "evidence_ref_id", "EVIDENCE_REF_ID_CONFLICT", null);

  const allConflicts = [...definitionClassification.conflicts, ...bindingClassification.conflicts, ...evidenceClassification.conflicts];
  if (allConflicts.length > 0) {
    const first = allConflicts[0];
    return rejected(first.code, `candidate id(s) already canonical with different content: ${allConflicts.map((c) => `${c.code}:${c.id}`).join(", ")}`, "/");
  }

  const newDefinitionIds = definitionClassification.newIds;
  const newBindingIds = bindingClassification.newIds;
  const newEvidenceIds = evidenceClassification.newIds;

  if (newDefinitionIds.length === 0 && newBindingIds.length === 0 && newEvidenceIds.length === 0) {
    return { status: "ALREADY_REGISTERED", commit: null, next_registry_snapshot: canonicalRegistry, errors: [] };
  }

  const registryRevisionAfter = canonical_capability_registry_snapshot.registry_revision + 1;
  if (!Number.isSafeInteger(registryRevisionAfter)) {
    return rejected("CAPABILITY_REGISTRY_REVISION_OVERFLOW", "registry_revision + 1 exceeds Number.MAX_SAFE_INTEGER");
  }

  const registryContentHashBefore = computeRegistryContentHash(existingDefinitions, existingBindings, existingEvidenceRefs);

  const newDefinitionIdSet = new Set(newDefinitionIds);
  const newBindingIdSet = new Set(newBindingIds);
  const newEvidenceIdSet = new Set(newEvidenceIds);
  const nextDefinitions = sortDefinitions([...existingDefinitions, ...candidate.definitions.filter((d) => newDefinitionIdSet.has(d.capability_definition_id))]);
  const nextBindings = sortBindings([...existingBindings, ...candidate.bindings.filter((b) => newBindingIdSet.has(b.binding_id))]);
  const nextEvidenceRefs = sortEvidenceRefs([...existingEvidenceRefs, ...candidate.evidence_refs.filter((e) => newEvidenceIdSet.has(e.evidence_ref_id))]);

  const registryContentHashAfter = computeRegistryContentHash(nextDefinitions, nextBindings, nextEvidenceRefs);

  const commitId = computeCommitId({
    request_fingerprint: requestFingerprint,
    candidate_content_hash: candidate.content_hash,
    authority_decision_id: decision.decision_id,
    registry_revision_before: canonical_capability_registry_snapshot.registry_revision,
    registry_revision_after: registryRevisionAfter,
  });

  const commit = canonicalize({
    commit_id: commitId,
    event_type: "chronica.capability.registration_committed",
    request_id: request.request_id,
    request_fingerprint: requestFingerprint,
    requester_principal_id: authenticated_requester_context.authenticated_principal_id,
    idempotency_key: request.idempotency_key,
    candidate_content_hash: candidate.content_hash,
    world_revision: request.expected_world_revision,
    world_state_hash: request.expected_world_state_hash,
    authority_revision: canonical_registration_authority_snapshot.authority_revision,
    authority_decision_id: decision.decision_id,
    authority_grant_id: grant.grant_id,
    authority_principal_id: decision.authority_principal_id,
    policy_ref: decision.policy_ref,
    policy_hash: decision.policy_hash,
    evidence_ref_ids: [...decision.evidence_ref_ids].sort(),
    registry_revision_before: canonical_capability_registry_snapshot.registry_revision,
    registry_revision_after: registryRevisionAfter,
    candidate_definition_ids: [...candidateDefinitionIds].sort(),
    candidate_binding_ids: [...candidateBindingIds].sort(),
    candidate_evidence_ref_ids: [...candidateEvidenceIds].sort(),
    new_definition_ids: [...newDefinitionIds].sort(),
    new_binding_ids: [...newBindingIds].sort(),
    new_evidence_ref_ids: [...newEvidenceIds].sort(),
    registry_content_hash_before: registryContentHashBefore,
    registry_content_hash_after: registryContentHashAfter,
    decision_at: decision.decided_at,
  });

  const nextCommits = sortCommits([...canonicalCommitsList, commit]);
  const nextRegistrySnapshotPre = {
    schema: CANONICAL_CAPABILITY_REGISTRY_SNAPSHOT_SCHEMA,
    registry_revision: registryRevisionAfter,
    definitions: nextDefinitions,
    bindings: nextBindings,
    evidence_refs: nextEvidenceRefs,
    registration_commits: nextCommits,
  };
  const nextRegistrySnapshot = canonicalize({ ...nextRegistrySnapshotPre, state_hash: computeRegistryStateHash(nextRegistrySnapshotPre) });

  return { status: "COMMIT", commit, next_registry_snapshot: nextRegistrySnapshot, errors: [] };
}
