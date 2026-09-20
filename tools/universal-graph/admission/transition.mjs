#!/usr/bin/env node
// evaluateAdmissionTransition(): the single fail-closed reference transition from a D0
// CANDIDATE_ONLY fragment to canonical Chronica WorldEntity state.
//
//   evaluateAdmissionTransition(
//     { raw_observation, request },                                  // UNTRUSTED
//     { canonical_world_snapshot,                                    // TRUSTED CANONICAL CONTEXT
//       canonical_authority_snapshot,
//       authenticated_requester_context },
//     ajv,
//   )
//
// The untrusted channel (raw_observation, request) is never read as a source of authority,
// canonical state, policy, or -- as of D.CLEAN-1R -- authenticated identity: it is only ever
// normalized (via D0's normalizeWorldFragment, reused unmodified), schema-validated against
// the closed AdmissionRequest schema, or checked against trusted context. The trusted channel
// is the only source of admission authority (CanonicalAuthoritySnapshot, looked up by decision
// id, never accepted inline) AND the only source of authenticated identity
// (AuthenticatedRequesterContext). Identity authentication is NOT an authority grant:
// AuthenticatedRequesterContext proves who the caller is, never what they may admit --
// CanonicalAuthoritySnapshot remains the only admission authority root.
//
// This is a REFERENCE transition only (see docs/.../admission/v0/README.md "What is NOT
// production-ready yet"): it computes next_world_snapshot/commit in memory and returns them.
// It never writes a production store, calls an external system, or performs a physical/
// execution action -- world admission authority is not execution authority.
//
// Pipeline (section 31 of the D.CLEAN-1R repair brief), in the exact order enforced below:
//   validate exact untrusted/trusted wrapper shape (no unknown field silently dropped)
//   -> D0 normalizeWorldFragment() -> AdmissionRequest schema validation
//   -> AuthenticatedRequesterContext schema validation
//   -> request.requester_principal_id == authenticated_principal_id (REQUESTER_IDENTITY_MISMATCH)
//   -> candidate_content_hash binding -> requested_entity_ids == complete candidate set
//   -> CanonicalWorldSnapshot schema validation
//   -> canonical-world LEDGER INTEGRITY (entity/commit id uniqueness, idempotency/request-id
//      ledger uniqueness, revision-ledger integrity, historical commit_id re-verification,
//      admitted-entity current-reference check) -- proven BEFORE any lookup runs
//   -> world canonicalization (order-insensitive collections normalized into one
//      deterministic form, used for every subsequent lookup AND for transition output)
//   -> world state_hash verification (over the canonical semantic form)
//   -> CanonicalAuthoritySnapshot schema validation -> authority id integrity
//      (grant_id/decision_id duplicate-vs-conflict) -> authority canonicalization
//      (allowed_entity_namespaces as a lexical set, decision.evidence_ref_ids order-insensitive)
//   -> authority state_hash verification
//   -> request fingerprint recomputation
//   -> idempotency replay/conflict check on the now-unambiguous canonical ledger (BEFORE
//      stale-revision check) -> request_id conflict check -> expected-revision fencing
//   -> canonical AdmissionDecision lookup -> decision/request/candidate binding
//   -> AdmissionAuthorityGrant resolution: status, policy binding, validity window, scope,
//      evidence binding
//   -> Separation of Duties using the TRUSTED authenticated identity (never the untrusted
//      request field, even though the two were already proven equal above)
//   -> DENY short-circuits to the canonical (order-independent) unchanged world state
//   -> ALLOW: all-or-nothing entity conflict checks -> revision-overflow check
//   -> deterministic AdmissionCommit (now recording the authenticated requester_principal_id)
//   -> revision+1 -> new canonical world snapshot
//
// Any failure at any stage returns { status: "REJECTED", commit: null, next_world_snapshot:
// null, errors: [{code, path, message}] }. Never mutates raw_observation, request,
// canonical_world_snapshot, canonical_authority_snapshot, or authenticated_requester_context
// in place.

import { createHash } from "node:crypto";
import { normalizeWorldFragment, canonicalStringify } from "../normalize.mjs";
import { validate } from "../schema-validate.mjs";

const ADMISSION_REQUEST_SCHEMA = "chronica.universal-graph.admission.admission-request.v0";
const AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA = "chronica.universal-graph.admission.authenticated-requester-context.v0";
const CANONICAL_WORLD_SNAPSHOT_SCHEMA = "chronica.universal-graph.admission.canonical-world-snapshot.v0";
const CANONICAL_AUTHORITY_SNAPSHOT_SCHEMA = "chronica.universal-graph.admission.canonical-authority-snapshot.v0";

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
 * Groups canonicalized `items` (already order-insensitive-normalized) by `idField` and
 * classifies any id that appears more than once: DUPLICATE if every occurrence is
 * byte-identical canonical content, CONFLICT if they differ. Returns null when every id is
 * unique. Used to prove a trusted collection has exactly one representation per id BEFORE
 * anything (.find()/Map) is allowed to read it -- order of the input never affects the result.
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

/** decision.evidence_ref_ids is declared order-insensitive; canonicalize before hashing/comparing content. */
function canonicalizeDecision(decision) {
  return { ...decision, evidence_ref_ids: [...decision.evidence_ref_ids].sort() };
}

/** grant.allowed_entity_namespaces is a set; canonicalize before hashing/comparing content. */
function canonicalizeGrant(grant) {
  return { ...grant, allowed_entity_namespaces: [...grant.allowed_entity_namespaces].sort() };
}

/** commit.evidence_ref_ids and commit.admitted_entity_ids are lexical sets; canonicalize before hashing/comparing/emitting. */
function canonicalizeCommit(commit) {
  return { ...commit, evidence_ref_ids: [...commit.evidence_ref_ids].sort(), admitted_entity_ids: [...commit.admitted_entity_ids].sort() };
}

function canonicalEntities(entities) {
  return sortByKeyThenContent(entities, (e) => [e.entity_id]);
}
function canonicalCommits(commits) {
  return sortByKeyThenContent(commits.map(canonicalizeCommit), (c) => [c.commit_id]);
}
function canonicalGrants(grants) {
  return sortByKeyThenContent(grants.map(canonicalizeGrant), (g) => [g.grant_id]);
}
function canonicalDecisions(decisions) {
  return sortByKeyThenContent(decisions.map(canonicalizeDecision), (d) => [d.decision_id]);
}

function computeWorldStateHash(snapshot) {
  return sha256({ world_revision: snapshot.world_revision, entities: canonicalEntities(snapshot.entities), admission_commits: canonicalCommits(snapshot.admission_commits) });
}

/**
 * Non-mutating: the canonical semantic form of a schema-valid, ledger-integrity-clean world
 * snapshot, used for every subsequent lookup AND as the transition's own output -- so neither
 * caller-supplied ARRAY ordering (sorted by canonicalEntities()/canonicalCommits(), per D1's
 * declared order-insensitive-collection semantics) nor caller-supplied JSON OBJECT-KEY
 * insertion order (recursively sorted by canonicalize(), which never reorders arrays) leaks
 * into results. Two semantically-identical trusted snapshots differing only in nested object
 * property order (WorldEntity, Provenance, AdmissionCommit, policy_ref, ...) therefore produce
 * a byte-identical DENY/REPLAY/COMMIT next_world_snapshot.
 */
function canonicalizeWorldSnapshot(snapshot) {
  return canonicalize({
    schema: snapshot.schema,
    world_revision: snapshot.world_revision,
    entities: canonicalEntities(snapshot.entities),
    admission_commits: canonicalCommits(snapshot.admission_commits),
    state_hash: snapshot.state_hash,
  });
}

function computeAuthoritySnapshotStateHash(snapshot) {
  return sha256({ authority_revision: snapshot.authority_revision, grants: canonicalGrants(snapshot.grants), decisions: canonicalDecisions(snapshot.decisions) });
}

function entitySetHash(entities) {
  return sha256(canonicalEntities(entities));
}

/** requested_entity_ids is declared order-insensitive; sorting it is the only normalization the whole AdmissionRequest needs before fingerprinting. */
function canonicalizeRequest(request) {
  return { ...request, requested_entity_ids: [...request.requested_entity_ids].sort() };
}

export function computeRequestFingerprint(request) {
  return sha256(canonicalizeRequest(request));
}

/** The single deterministic derivation used both to mint a new commit_id and to re-verify every historical commit already in the ledger (section 15/27). */
function computeCommitId({ request_fingerprint, candidate_content_hash, authority_decision_id, world_revision_before, world_revision_after }) {
  return sha256({ request_fingerprint, candidate_content_hash, authority_decision_id, world_revision_before, world_revision_after });
}

function rootNamespace(entityType) {
  return entityType.split(".")[0];
}

function policyRefEqual(a, b) {
  return a.scheme === b.scheme && a.value === b.value;
}

function rejected(code, message, path = "/") {
  return { status: "REJECTED", commit: null, next_world_snapshot: null, errors: [{ code, path, message: message ?? code }] };
}

function unchanged(status, commit, canonicalWorld) {
  return { status, commit, next_world_snapshot: canonicalWorld, errors: [] };
}

/**
 * Proves a schema-valid CanonicalWorldSnapshot has exactly one representation per entity_id,
 * per commit_id, per idempotency_key, and per world_revision_before/_after, that every commit's
 * revision claim is internally consistent, that every commit_id is a real recomputation of its
 * own semantic fields (not a self-attested, schema-shaped id), and that every admitted entity
 * id still resolves in the current entity set. Runs BEFORE canonicalization and BEFORE any
 * .find()/Map lookup is allowed to read the ledger -- ambiguity is eliminated first, not
 * tolerated and then resolved by array order.
 */
function validateWorldLedgerIntegrity(snapshot) {
  const entityConflict = findIdDuplicateOrConflict(snapshot.entities, "entity_id");
  if (entityConflict) return entityConflict.kind === "DUPLICATE" ? "CANONICAL_ENTITY_ID_DUPLICATE" : "CANONICAL_ENTITY_ID_CONFLICT";

  const commitConflict = findIdDuplicateOrConflict(snapshot.admission_commits.map(canonicalizeCommit), "commit_id");
  if (commitConflict) return commitConflict.kind === "DUPLICATE" ? "CANONICAL_COMMIT_ID_DUPLICATE" : "CANONICAL_COMMIT_ID_CONFLICT";

  if (findDuplicateValue(snapshot.admission_commits, "idempotency_key") !== null) return "CANONICAL_IDEMPOTENCY_LEDGER_CONFLICT";
  if (findDuplicateValue(snapshot.admission_commits, "request_id") !== null) return "CANONICAL_REQUEST_LEDGER_CONFLICT";

  for (const c of snapshot.admission_commits) {
    const expectedAfter = c.world_revision_before + 1;
    if (!Number.isSafeInteger(expectedAfter) || expectedAfter !== c.world_revision_after || c.world_revision_after > snapshot.world_revision) {
      return "ADMISSION_COMMIT_REVISION_INVALID";
    }
  }
  if (findDuplicateValue(snapshot.admission_commits, "world_revision_before") !== null) return "CANONICAL_ADMISSION_REVISION_CONFLICT";
  if (findDuplicateValue(snapshot.admission_commits, "world_revision_after") !== null) return "CANONICAL_ADMISSION_REVISION_CONFLICT";

  for (const c of snapshot.admission_commits) {
    const recomputed = computeCommitId(c);
    if (recomputed !== c.commit_id) return "ADMISSION_COMMIT_ID_MISMATCH";
  }

  const entityIdSet = new Set(snapshot.entities.map((e) => e.entity_id));
  for (const c of snapshot.admission_commits) {
    for (const id of c.admitted_entity_ids) {
      if (!entityIdSet.has(id)) return "ADMISSION_COMMIT_ENTITY_MISSING";
    }
  }

  return null;
}

/**
 * Proves a canonical_world_snapshot is schema-valid, internally ledger-unambiguous (entity_id/
 * commit_id/idempotency_key/request_id uniqueness, revision-pair integrity, historical commit_id
 * recomputation, admitted_entity_ids current-reference integrity), and state_hash-verified, and
 * returns its canonical (order-independent) form. This is D.CLEAN-1R's own world-verification
 * sequence (schema -> ledger integrity -> canonicalize -> state_hash), extracted verbatim and
 * exported (D.CLEAN-3R) so it has exactly ONE implementation: evaluateAdmissionTransition() below
 * calls it, and any future layer that must trust canonical WorldEntity state (D3's capability
 * registration transition) reuses this instead of reimplementing world-ledger verification.
 *
 * @param {object} canonical_world_snapshot
 * @param {import("ajv").default} ajv
 * @returns {{valid: true, canonical_world_snapshot: object} | {valid: false, code: string, path: string, message: string}}
 */
export function verifyCanonicalWorldSnapshot(canonical_world_snapshot, ajv) {
  const worldSchemaResult = validate(canonical_world_snapshot, CANONICAL_WORLD_SNAPSHOT_SCHEMA, ajv);
  if (!worldSchemaResult.valid) {
    return { valid: false, code: "CANONICAL_WORLD_SNAPSHOT_SCHEMA_INVALID", path: "/", message: JSON.stringify(worldSchemaResult.errors) };
  }

  const ledgerErrorCode = validateWorldLedgerIntegrity(canonical_world_snapshot);
  if (ledgerErrorCode) {
    return { valid: false, code: ledgerErrorCode, path: "/canonical_world_snapshot/admission_commits", message: "canonical world snapshot ledger integrity check failed" };
  }

  // From here on, every world lookup and every piece of transition output uses ONLY this
  // canonical, order-independent form -- never the raw caller-supplied snapshot.
  const canonicalWorld = canonicalizeWorldSnapshot(canonical_world_snapshot);
  if (computeWorldStateHash(canonicalWorld) !== canonicalWorld.state_hash) {
    return { valid: false, code: "CANONICAL_WORLD_STATE_HASH_MISMATCH", path: "/", message: "recomputed world state_hash does not match the supplied snapshot" };
  }

  return { valid: true, canonical_world_snapshot: canonicalWorld };
}

/**
 * @param {{raw_observation: object, request: object}} untrusted
 * @param {{canonical_world_snapshot: object, canonical_authority_snapshot: object, authenticated_requester_context: object}} trusted
 * @param {import("ajv").default} ajv - from compileAdmissionSchemas()
 * @returns {object} AdmissionTransitionResult
 */
export function evaluateAdmissionTransition(untrusted, trusted, ajv) {
  if (!hasExactKeys(untrusted, ["raw_observation", "request"])) {
    return rejected("UNTRUSTED_CHANNEL_UNKNOWN_FIELD", "untrusted wrapper must contain exactly {raw_observation, request}");
  }
  if (!hasExactKeys(trusted, ["canonical_world_snapshot", "canonical_authority_snapshot", "authenticated_requester_context"])) {
    return rejected("TRUSTED_CHANNEL_UNKNOWN_FIELD", "trusted wrapper must contain exactly {canonical_world_snapshot, canonical_authority_snapshot, authenticated_requester_context}");
  }
  const { raw_observation, request } = untrusted;
  const { canonical_world_snapshot, canonical_authority_snapshot, authenticated_requester_context } = trusted;

  const candidateResult = normalizeWorldFragment(raw_observation, ajv);
  if (!candidateResult.valid) return rejected("D0_CANDIDATE_INVALID", JSON.stringify(candidateResult.errors));
  const candidate = candidateResult.fragment;

  const requestValidation = validate(request, ADMISSION_REQUEST_SCHEMA, ajv);
  if (!requestValidation.valid) return rejected("ADMISSION_REQUEST_SCHEMA_INVALID", JSON.stringify(requestValidation.errors));

  const contextValidation = validate(authenticated_requester_context, AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA, ajv);
  if (!contextValidation.valid) return rejected("AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA_INVALID", JSON.stringify(contextValidation.errors));

  // Requester identity binding: proven BEFORE candidate/entity-set binding, BEFORE any ledger
  // lookup, and therefore before idempotent replay can ever be reached (section 7/31) --
  // a same-content replay from a different authenticated principal cannot pass this gate, and
  // if the request's own requester_principal_id is instead changed to match the new caller, the
  // request_fingerprint changes too and idempotency lookup rejects it as a conflict, not a replay.
  if (request.requester_principal_id !== authenticated_requester_context.authenticated_principal_id) {
    return rejected("REQUESTER_IDENTITY_MISMATCH", "request.requester_principal_id does not match authenticated_requester_context.authenticated_principal_id", "/requester_principal_id");
  }

  if (request.candidate_content_hash !== candidate.content_hash) {
    return rejected("CANDIDATE_CONTENT_HASH_MISMATCH", "request.candidate_content_hash does not equal the D0 candidate's content_hash");
  }

  const candidateEntityIds = new Set(candidate.entities.map((e) => e.entity_id));
  const requestedIds = new Set(request.requested_entity_ids);
  if ([...candidateEntityIds].some((id) => !requestedIds.has(id))) {
    return rejected("REQUESTED_ENTITY_SET_INCOMPLETE", "requested_entity_ids is missing one or more candidate entity ids", "/requested_entity_ids");
  }
  if ([...requestedIds].some((id) => !candidateEntityIds.has(id))) {
    return rejected("REQUESTED_ENTITY_SET_HAS_EXTRA", "requested_entity_ids names an id outside the candidate's entity set", "/requested_entity_ids");
  }

  const worldVerification = verifyCanonicalWorldSnapshot(canonical_world_snapshot, ajv);
  if (!worldVerification.valid) return rejected(worldVerification.code, worldVerification.message, worldVerification.path);
  // From here on, every world lookup and every piece of transition output uses ONLY this
  // canonical, order-independent form -- never the raw caller-supplied snapshot.
  const canonicalWorld = worldVerification.canonical_world_snapshot;

  const authoritySchemaResult = validate(canonical_authority_snapshot, CANONICAL_AUTHORITY_SNAPSHOT_SCHEMA, ajv);
  if (!authoritySchemaResult.valid) return rejected("CANONICAL_AUTHORITY_SNAPSHOT_SCHEMA_INVALID", JSON.stringify(authoritySchemaResult.errors));

  const grantConflict = findIdDuplicateOrConflict(canonical_authority_snapshot.grants.map(canonicalizeGrant), "grant_id");
  if (grantConflict) return rejected(grantConflict.kind === "DUPLICATE" ? "AUTHORITY_GRANT_ID_DUPLICATE" : "AUTHORITY_GRANT_ID_CONFLICT", "two grants share a grant_id", "/canonical_authority_snapshot/grants");
  const decisionConflict = findIdDuplicateOrConflict(canonical_authority_snapshot.decisions.map(canonicalizeDecision), "decision_id");
  if (decisionConflict) return rejected(decisionConflict.kind === "DUPLICATE" ? "AUTHORITY_DECISION_ID_DUPLICATE" : "AUTHORITY_DECISION_ID_CONFLICT", "two decisions share a decision_id", "/canonical_authority_snapshot/decisions");

  const canonicalGrantList = canonicalGrants(canonical_authority_snapshot.grants);
  const canonicalDecisionList = canonicalDecisions(canonical_authority_snapshot.decisions);
  if (computeAuthoritySnapshotStateHash({ authority_revision: canonical_authority_snapshot.authority_revision, grants: canonicalGrantList, decisions: canonicalDecisionList }) !== canonical_authority_snapshot.state_hash) {
    return rejected("AUTHORITY_SNAPSHOT_HASH_MISMATCH", "recomputed authority state_hash does not match the supplied snapshot");
  }

  const requestFingerprint = computeRequestFingerprint(request);

  // Idempotency replay/conflict check runs on the now-proven-unambiguous canonical ledger,
  // and BEFORE the stale-revision check (section 12/19).
  const priorByIdemKey = canonicalWorld.admission_commits.find((c) => c.idempotency_key === request.idempotency_key);
  if (priorByIdemKey) {
    if (priorByIdemKey.request_fingerprint === requestFingerprint && priorByIdemKey.candidate_content_hash === candidate.content_hash) {
      return unchanged("REPLAY", priorByIdemKey, canonicalWorld);
    }
    return rejected("IDEMPOTENCY_KEY_REUSE_CONFLICT", "idempotency_key was already used for a different request_fingerprint/candidate_content_hash", "/idempotency_key");
  }

  const priorByRequestId = canonicalWorld.admission_commits.find((c) => c.request_id === request.request_id);
  if (priorByRequestId && priorByRequestId.request_fingerprint !== requestFingerprint) {
    return rejected("REQUEST_ID_CONFLICT", "request_id was already committed for a different request_fingerprint", "/request_id");
  }

  if (request.expected_world_revision !== canonicalWorld.world_revision) {
    return rejected("STALE_WORLD_REVISION", "expected_world_revision does not match the current canonical world_revision", "/expected_world_revision");
  }

  const decision = canonicalDecisionList.find((d) => d.decision_id === request.authority_decision_id);
  if (!decision) return rejected("MISSING_CANONICAL_DECISION", "authority_decision_id does not resolve in the canonical authority snapshot", "/authority_decision_id");

  if (decision.request_id !== request.request_id) return rejected("DECISION_REQUEST_ID_MISMATCH", "decision.request_id does not match request.request_id");
  if (decision.request_fingerprint !== requestFingerprint) return rejected("REQUEST_FINGERPRINT_MISMATCH", "decision.request_fingerprint does not match the recomputed request fingerprint");
  if (decision.candidate_content_hash !== candidate.content_hash) return rejected("CANDIDATE_CONTENT_HASH_MISMATCH", "decision.candidate_content_hash does not match the D0 candidate's content_hash");

  const grant = canonicalGrantList.find((g) => g.grant_id === decision.grant_id);
  if (!grant) return rejected("MISSING_AUTHORITY_GRANT", "decision.grant_id does not resolve in the canonical authority snapshot");
  if (decision.authority_principal_id !== grant.authority_principal_id) return rejected("DECISION_GRANT_PRINCIPAL_MISMATCH", "decision.authority_principal_id does not match grant.authority_principal_id");
  if (grant.status !== "ACTIVE") return rejected("GRANT_NOT_ACTIVE", `grant.status is "${grant.status}", not ACTIVE`);
  if (!policyRefEqual(decision.policy_ref, grant.policy_ref)) return rejected("POLICY_REF_MISMATCH", "decision.policy_ref does not match grant.policy_ref");
  if (decision.policy_hash !== grant.policy_hash) return rejected("POLICY_HASH_MISMATCH", "decision.policy_hash does not match grant.policy_hash");
  if (!(decision.decided_at >= grant.valid_from && decision.decided_at < grant.valid_until)) {
    return rejected("GRANT_VALIDITY_WINDOW_VIOLATION", "decision.decided_at falls outside [grant.valid_from, grant.valid_until)");
  }

  const namespaces = new Set(candidate.entities.map((e) => rootNamespace(e.entity_type)));
  for (const ns of namespaces) {
    if (!grant.allowed_entity_namespaces.includes(ns)) {
      return rejected("ENTITY_NAMESPACE_NOT_ALLOWED", `entity root namespace "${ns}" is not in grant.allowed_entity_namespaces`, "/requested_entity_ids");
    }
  }

  const evidenceIdSet = new Set(candidate.evidence_refs.map((r) => r.evidence_ref_id));
  const orphanEvidenceId = decision.evidence_ref_ids.find((id) => !evidenceIdSet.has(id));
  if (orphanEvidenceId) {
    return rejected("ORPHAN_EVIDENCE_REF", `decision.evidence_ref_ids "${orphanEvidenceId}" does not resolve in the D0 candidate's evidence_refs`, "/canonical_authority_snapshot/decisions");
  }
  if (decision.effect === "ALLOW" && decision.evidence_ref_ids.length === 0) {
    return rejected("ALLOW_ZERO_EVIDENCE", "an ALLOW decision requires at least one evidence_ref_id");
  }

  // SoD uses the TRUSTED authenticated identity, never the untrusted request field directly --
  // the two are already proven equal above, but the trusted value is the source of record.
  if (grant.self_approval_policy === "DISTINCT_REQUIRED" && authenticated_requester_context.authenticated_principal_id === decision.authority_principal_id) {
    return rejected("SELF_APPROVAL_VIOLATION", "grant.self_approval_policy is DISTINCT_REQUIRED but the authenticated requester equals decision.authority_principal_id");
  }

  if (decision.effect === "DENY") {
    return unchanged("DENIED", null, canonicalWorld);
  }

  const canonicalEntityById = new Map(canonicalWorld.entities.map((e) => [e.entity_id, e]));
  const conflictingIds = [];
  const alreadyCanonicalIds = [];
  for (const entity of candidate.entities) {
    const existing = canonicalEntityById.get(entity.entity_id);
    if (!existing) continue;
    if (canonicalStringify(existing) === canonicalStringify(entity)) alreadyCanonicalIds.push(entity.entity_id);
    else conflictingIds.push(entity.entity_id);
  }
  if (conflictingIds.length > 0) {
    return rejected("CANONICAL_ENTITY_CONFLICT", `entity id(s) already canonical with different content: ${conflictingIds.join(", ")}`, "/requested_entity_ids");
  }
  if (alreadyCanonicalIds.length > 0) {
    return rejected("ENTITY_ALREADY_CANONICAL", `entity id(s) already canonical with identical content, and this is not an idempotent replay: ${alreadyCanonicalIds.join(", ")}`, "/requested_entity_ids");
  }

  const worldRevisionAfter = canonicalWorld.world_revision + 1;
  if (!Number.isSafeInteger(worldRevisionAfter)) {
    return rejected("WORLD_REVISION_OVERFLOW", "world_revision + 1 exceeds Number.MAX_SAFE_INTEGER");
  }

  const entitySetHashBefore = entitySetHash(canonicalWorld.entities);
  const nextEntities = sortByKeyThenContent([...canonicalWorld.entities, ...candidate.entities], (e) => [e.entity_id]);
  const entitySetHashAfter = entitySetHash(nextEntities);

  const commitId = computeCommitId({
    request_fingerprint: requestFingerprint,
    candidate_content_hash: candidate.content_hash,
    authority_decision_id: decision.decision_id,
    world_revision_before: canonicalWorld.world_revision,
    world_revision_after: worldRevisionAfter,
  });

  const commit = canonicalize({
    commit_id: commitId,
    event_type: "chronica.world.admission_committed",
    request_id: request.request_id,
    request_fingerprint: requestFingerprint,
    idempotency_key: request.idempotency_key,
    candidate_content_hash: candidate.content_hash,
    requester_principal_id: authenticated_requester_context.authenticated_principal_id,
    authority_revision: canonical_authority_snapshot.authority_revision,
    authority_decision_id: decision.decision_id,
    authority_grant_id: grant.grant_id,
    authority_principal_id: decision.authority_principal_id,
    policy_ref: decision.policy_ref,
    policy_hash: decision.policy_hash,
    evidence_ref_ids: [...decision.evidence_ref_ids].sort(),
    world_revision_before: canonicalWorld.world_revision,
    world_revision_after: worldRevisionAfter,
    admitted_entity_ids: candidate.entities.map((e) => e.entity_id).sort(),
    entity_set_hash_before: entitySetHashBefore,
    entity_set_hash_after: entitySetHashAfter,
    decision_at: decision.decided_at,
  });

  const nextCommits = sortByKeyThenContent([...canonicalWorld.admission_commits, commit], (c) => [c.commit_id]);
  const nextWorldSnapshotPre = {
    schema: "chronica.universal-graph.admission.canonical-world-snapshot.v0",
    world_revision: worldRevisionAfter,
    entities: nextEntities,
    admission_commits: nextCommits,
  };
  const nextWorldSnapshot = canonicalize({ ...nextWorldSnapshotPre, state_hash: computeWorldStateHash(nextWorldSnapshotPre) });

  return { status: "COMMIT", commit, next_world_snapshot: nextWorldSnapshot, errors: [] };
}

export { computeWorldStateHash, computeAuthoritySnapshotStateHash, entitySetHash, canonicalizeRequest, computeCommitId, canonicalizeWorldSnapshot };
export { CANONICAL_WORLD_SNAPSHOT_SCHEMA };
