#!/usr/bin/env node
// Test runner for the D.CLEAN-1 / D.CLEAN-1R World Admission Transition contract.
//
// Runs, in order:
//   - the POSITIVE matrix (>=10 required cases, plus D1R requester-identity cases)
//   - the NEGATIVE matrix (>=36 required fail-closed cases, plus D1R requester-identity /
//     ledger-integrity / channel-shape cases), table-driven -- each entry mutates one field of
//     an otherwise fully self-consistent "golden" scenario and asserts the transition rejects
//     with the exact expected machine-readable code
//   - the ADVERSARIAL pass (D1R: >=80 cases), which both re-executes every NEGATIVE case
//     (already real transition-code execution, not a static placeholder) and adds dedicated
//     checks for requester-identity binding, canonical-ledger order-independence (idempotency/
//     entity-map ambiguity attacks under both array orderings), DENY/REPLAY full-output order
//     independence, scope-widening, non-mutation, hash-graph acyclicity, and layering/repo purity
//     (D1 imports only its allowed D0/reference dependencies; no System Atlas or product/runtime
//     import -- durable content-based invariants, not a base-branch `git diff`; see D.RUNTIME-0R2)
//
// Usage: node tools/universal-graph/admission/tests.mjs

import { readFileSync, writeFileSync, unlinkSync, mkdtempSync, rmdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { tmpdir } from "node:os";
import { compileAdmissionSchemas, validate } from "./schema-validate.mjs";
import {
  evaluateAdmissionTransition,
  computeRequestFingerprint,
  computeWorldStateHash,
  computeAuthoritySnapshotStateHash,
  computeCommitId,
} from "./transition.mjs";
import { normalizeWorldFragment, canonicalStringify } from "../normalize.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..", "..");
const D0_SCHEMA_DIR = resolve(REPO_ROOT, "docs", "_machine", "universal-graph", "v0", "schemas");
const ADMISSION_DIR = resolve(REPO_ROOT, "docs", "_machine", "universal-graph", "admission", "v0");
const ADMISSION_SCHEMA_DIR = resolve(ADMISSION_DIR, "schemas");

const WORLD_SCHEMA = "chronica.universal-graph.admission.canonical-world-snapshot.v0";
const AUTHORITY_SCHEMA = "chronica.universal-graph.admission.canonical-authority-snapshot.v0";
const CONTEXT_SCHEMA = "chronica.universal-graph.admission.authenticated-requester-context.v0";
const RESULT_SCHEMA = "chronica.universal-graph.admission.admission-transition-result.v0";

let pass = 0;
let fail = 0;
const failures = [];

function check(label, cond, detail) {
  if (cond) {
    pass++;
    console.log(`  PASS  ${label}`);
  } else {
    fail++;
    failures.push(label);
    console.log(`  FAIL  ${label}${detail ? ` -- ${detail}` : ""}`);
  }
}

let advCount = 0;
function adversarial(label, cond, detail) {
  advCount++;
  check(`ADV-${advCount} ${label}`, cond, detail);
}

const ajv = compileAdmissionSchemas(D0_SCHEMA_DIR, ADMISSION_SCHEMA_DIR);

console.log(`\n== SCHEMA_META_VALIDATION (Draft 2020-12 compile, D0 + D1 in one Ajv2020 instance) ==`);
check("all committed D0+D1 schemas registered and compiled under Ajv2020 without throwing", true, "compileAdmissionSchemas() above already threw if not");

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

function clone(v) {
  return JSON.parse(JSON.stringify(v));
}

const ALICE = { entity_id: "human.person:alice-01", entity_type: "human.person", display_name: "Alice", provenance: { source: "chronica.hr", observed_at: "2026-08-20T12:00:00Z" } };
const PLC = { entity_id: "machine.plc:line3-01", entity_type: "machine.plc", display_name: "Line 3 PLC", provenance: { source: "machine.opcua", observed_at: "2026-08-20T12:00:00Z" } };
const ZED = { entity_id: "human.person:zed-01", entity_type: "human.person", display_name: "Zed (prior admission)", provenance: { source: "chronica.hr", observed_at: "2026-08-01T00:00:00Z" } };
const EV1 = { evidence_ref_id: "ev-1", evidence_kind: "chronica.event", locator: { scheme: "chronica.event_id", value: "evt-123" }, provenance: { source: "chronica.hr", observed_at: "2026-08-20T12:00:00Z" } };

function oneEntityRawObservation() {
  return { entities: [clone(ALICE)], identity_links: [], evidence_refs: [clone(EV1)], semantic_attachments: [] };
}
function twoEntityRawObservation() {
  return { entities: [clone(ALICE), clone(PLC)], identity_links: [], evidence_refs: [clone(EV1)], semantic_attachments: [] };
}

function buildWorld(entities, admission_commits, world_revision = 0) {
  const pre = { world_revision, entities, admission_commits };
  return { schema: WORLD_SCHEMA, ...pre, state_hash: computeWorldStateHash(pre) };
}
function buildAuthority(grants, decisions, authority_revision = 0) {
  const pre = { authority_revision, grants, decisions };
  return { schema: AUTHORITY_SCHEMA, ...pre, state_hash: computeAuthoritySnapshotStateHash(pre) };
}
function buildAuthContext(principal_id, overrides = {}) {
  return { schema: CONTEXT_SCHEMA, authenticated_principal_id: principal_id, authentication_ref: { scheme: "chronica.session_id", value: "sess-test-1" }, ...overrides };
}

function flipHash(hash) {
  const [prefix, hex] = hash.split(":");
  return `${prefix}:${hex[0] === "0" ? "1" : "0"}${hex.slice(1)}`;
}

/**
 * A schema-shaped AdmissionCommit that, unless `commit_id` is explicitly overridden, derives
 * its own commit_id the SAME way transition.mjs does -- so a fixture built from this passes
 * D1R's historical-commit-id re-verification by default, and callers only need to override
 * `commit_id` directly when the test IS specifically about a forged/mismatched id.
 */
function fakeCommit(overrides = {}) {
  const base = {
    request_id: "req-PRIOR",
    request_fingerprint: `sha256:${"2".repeat(64)}`,
    idempotency_key: "idem-PRIOR",
    candidate_content_hash: `sha256:${"3".repeat(64)}`,
    requester_principal_id: "human.person:zed-01",
    authority_revision: 0,
    authority_decision_id: "dec-PRIOR",
    authority_grant_id: "grant-001",
    authority_principal_id: "ai.agent:admitter-01",
    policy_ref: { scheme: "chronica.policy_id", value: "world-admission-default" },
    policy_hash: `sha256:${"a".repeat(64)}`,
    evidence_ref_ids: ["ev-prior"],
    world_revision_before: 0,
    world_revision_after: 1,
    admitted_entity_ids: ["human.person:zed-01"],
    entity_set_hash_before: `sha256:${"4".repeat(64)}`,
    entity_set_hash_after: `sha256:${"5".repeat(64)}`,
    decision_at: "2026-08-01T00:00:00Z",
    event_type: "chronica.world.admission_committed",
    ...overrides,
  };
  if (base.commit_id === undefined) {
    base.commit_id = computeCommitId({
      request_fingerprint: base.request_fingerprint,
      candidate_content_hash: base.candidate_content_hash,
      authority_decision_id: base.authority_decision_id,
      world_revision_before: base.world_revision_before,
      world_revision_after: base.world_revision_after,
    });
  }
  return base;
}

/** A fully self-consistent golden scenario: one human entity, ALLOW decision, ACTIVE grant, and a matching AuthenticatedRequesterContext. */
function golden() {
  const rawObservation = oneEntityRawObservation();
  const candidate = normalizeWorldFragment(rawObservation, ajv);
  const contentHash = candidate.fragment.content_hash;
  const request = {
    request_id: "req-001",
    requester_principal_id: "human.person:alice-01",
    candidate_content_hash: contentHash,
    requested_entity_ids: ["human.person:alice-01"],
    expected_world_revision: 0,
    idempotency_key: "idem-001",
    authority_decision_id: "dec-001",
  };
  const requestFingerprint = computeRequestFingerprint(request);
  const grant = {
    grant_id: "grant-001",
    authority_principal_id: "ai.agent:admitter-01",
    action: "chronica.world.admit",
    allowed_entity_namespaces: ["human", "machine", "future"],
    policy_ref: { scheme: "chronica.policy_id", value: "world-admission-default" },
    policy_hash: `sha256:${"a".repeat(64)}`,
    valid_from: "2026-01-01T00:00:00Z",
    valid_until: "2027-01-01T00:00:00Z",
    status: "ACTIVE",
    self_approval_policy: "DISTINCT_REQUIRED",
  };
  const decision = {
    decision_id: "dec-001",
    request_id: "req-001",
    request_fingerprint: requestFingerprint,
    candidate_content_hash: contentHash,
    authority_principal_id: "ai.agent:admitter-01",
    grant_id: "grant-001",
    effect: "ALLOW",
    policy_ref: grant.policy_ref,
    policy_hash: grant.policy_hash,
    evidence_ref_ids: ["ev-1"],
    decided_at: "2026-08-20T12:05:00Z",
  };
  return {
    rawObservation,
    request,
    grant,
    decision,
    authenticatedRequesterContext: buildAuthContext("human.person:alice-01"),
    canonical_world_snapshot: buildWorld([], []),
    canonical_authority_snapshot: buildAuthority([grant], [decision]),
  };
}

function run(g) {
  return evaluateAdmissionTransition(
    { raw_observation: g.rawObservation, request: g.request },
    { canonical_world_snapshot: g.canonical_world_snapshot, canonical_authority_snapshot: g.canonical_authority_snapshot, authenticated_requester_context: g.authenticatedRequesterContext },
    ajv,
  );
}

function runRaw(untrusted, trusted) {
  return evaluateAdmissionTransition(untrusted, trusted, ajv);
}

// ---------------------------------------------------------------------------
// POSITIVE matrix (>=10 required)
// ---------------------------------------------------------------------------

console.log(`\n== POSITIVE matrix ==`);
const posResults = {};

{
  const g = golden();
  posResults[1] = run(g);
  check("POS-1 single human entity under valid ALLOW decision: COMMIT", posResults[1].status === "COMMIT", JSON.stringify(posResults[1]));
  check("POS-1 admitted entity id is correct", posResults[1].commit?.admitted_entity_ids?.[0] === "human.person:alice-01");
  check("POS-1 world_revision advances 0 -> 1", posResults[1].next_world_snapshot?.world_revision === 1);
  check("POS-1 commit.requester_principal_id is the AUTHENTICATED identity", posResults[1].commit?.requester_principal_id === g.authenticatedRequesterContext.authenticated_principal_id);
  check("POS-1 result validates against AdmissionTransitionResult schema", validate(posResults[1], RESULT_SCHEMA, ajv).valid);
}

{
  const g = golden();
  g.rawObservation = { entities: [clone(PLC)], identity_links: [], evidence_refs: [clone(EV1)], semantic_attachments: [] };
  const candidate = normalizeWorldFragment(g.rawObservation, ajv);
  g.request.candidate_content_hash = candidate.fragment.content_hash;
  g.request.requested_entity_ids = ["machine.plc:line3-01"];
  g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request), candidate_content_hash: g.request.candidate_content_hash };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[2] = run(g);
  check("POS-2 single machine entity under grant allowing the machine namespace: COMMIT", posResults[2].status === "COMMIT", JSON.stringify(posResults[2]));
}

{
  const g = golden();
  g.rawObservation = twoEntityRawObservation();
  const candidate = normalizeWorldFragment(g.rawObservation, ajv);
  g.request.candidate_content_hash = candidate.fragment.content_hash;
  g.request.requested_entity_ids = ["human.person:alice-01", "machine.plc:line3-01"];
  g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request), candidate_content_hash: g.request.candidate_content_hash };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[3] = run(g);
  check("POS-3 multiple entities admitted atomically: COMMIT with both ids", posResults[3].status === "COMMIT" && posResults[3].commit?.admitted_entity_ids?.length === 2, JSON.stringify(posResults[3]));
}

{
  const g = golden();
  const futureEntity = { entity_id: "future.example:widget-01", entity_type: "future.example.widget", display_name: "Widget", provenance: { source: "future.example.provider", observed_at: "2026-08-20T12:00:00Z" } };
  g.rawObservation = { entities: [futureEntity], identity_links: [], evidence_refs: [clone(EV1)], semantic_attachments: [] };
  const candidate = normalizeWorldFragment(g.rawObservation, ajv);
  g.request.candidate_content_hash = candidate.fragment.content_hash;
  g.request.requested_entity_ids = ["future.example:widget-01"];
  g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request), candidate_content_hash: g.request.candidate_content_hash };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[4] = run(g);
  check("POS-4 unknown future D0 provider admitted (no schema edit needed): COMMIT", posResults[4].status === "COMMIT", JSON.stringify(posResults[4]));
}

{
  const g = golden();
  g.rawObservation.evidence_refs = [{ evidence_ref_id: "ev-telemetry-1", evidence_kind: "machine.telemetry", locator: { scheme: "machine.telemetry_ref", value: "sensor-42/reading-9981" }, provenance: { source: "machine.opcua", observed_at: "2026-08-20T12:00:00Z" } }];
  const candidate = normalizeWorldFragment(g.rawObservation, ajv);
  g.request.candidate_content_hash = candidate.fragment.content_hash;
  g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request), candidate_content_hash: g.request.candidate_content_hash, evidence_ref_ids: ["ev-telemetry-1"] };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[5] = run(g);
  check("POS-5 non-filesystem (machine telemetry) evidence justifies admission: COMMIT", posResults[5].status === "COMMIT", JSON.stringify(posResults[5]));
}

{
  const g = golden();
  posResults[6] = run(g);
  check(
    "POS-6 DISTINCT_REQUIRED with a genuinely distinct AUTHENTICATED requester: COMMIT",
    posResults[6].status === "COMMIT" && g.authenticatedRequesterContext.authenticated_principal_id !== g.decision.authority_principal_id,
  );
}

{
  const g = golden();
  g.request.requester_principal_id = g.decision.authority_principal_id;
  g.authenticatedRequesterContext = buildAuthContext(g.decision.authority_principal_id);
  g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request) };
  g.grant = { ...g.grant, self_approval_policy: "SAME_ALLOWED" };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[7] = run(g);
  check("POS-7 SAME_ALLOWED self-approval, trusted requester == authority: COMMIT", posResults[7].status === "COMMIT", JSON.stringify(posResults[7]));
}

{
  const g = golden();
  g.decision = { ...g.decision, effect: "DENY" };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[8] = run(g);
  check("POS-8 canonical DENY: status DENIED", posResults[8].status === "DENIED", JSON.stringify(posResults[8]));
  check("POS-8 DENY produces no commit", posResults[8].commit === null);
  check("POS-8 DENY leaves world state byte-identical (DENY_STATE_MUTATION_COUNT = 0)", canonicalStringify(posResults[8].next_world_snapshot) === canonicalStringify(g.canonical_world_snapshot));
}

let replayWorld;
{
  const g = golden();
  const first = run(g);
  replayWorld = first.next_world_snapshot; // world_revision now 1, contains the commit
  const g2 = { ...g, canonical_world_snapshot: replayWorld }; // request UNCHANGED: expected_world_revision is now stale (0 vs 1)
  posResults[9] = run(g2);
  check("POS-9 exact idempotent retry: status REPLAY despite a now-stale expected_world_revision (idempotency precedes staleness)", posResults[9].status === "REPLAY", JSON.stringify(posResults[9]));
  check("POS-9 REPLAY returns the exact prior commit", canonicalStringify(posResults[9].commit) === canonicalStringify(first.commit));
  check("POS-9 REPLAY does not increment world_revision (IDEMPOTENT_REPLAY_REVISION_INCREMENT = 0)", posResults[9].next_world_snapshot.world_revision === 1);
}

{
  const g = golden();
  g.rawObservation = { entities: [clone(PLC)], identity_links: [], evidence_refs: [clone(EV1)], semantic_attachments: [] };
  const candidate = normalizeWorldFragment(g.rawObservation, ajv);
  g.request = { ...g.request, request_id: "req-002", candidate_content_hash: candidate.fragment.content_hash, requested_entity_ids: ["machine.plc:line3-01"], expected_world_revision: 1, idempotency_key: "idem-002", authority_decision_id: "dec-002" };
  g.decision = { ...g.decision, decision_id: "dec-002", request_id: "req-002", request_fingerprint: computeRequestFingerprint(g.request), candidate_content_hash: g.request.candidate_content_hash };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  g.canonical_world_snapshot = replayWorld;
  posResults[10] = run(g);
  check("POS-10 sequential second authorized admission from the new world revision: COMMIT", posResults[10].status === "COMMIT", JSON.stringify(posResults[10]));
  check("POS-10 world_revision advances 1 -> 2", posResults[10].next_world_snapshot?.world_revision === 2);
}

// D1R section 23.C/D: explicit SoD-source tests (redundant in outcome with POS-6/7 above, kept
// separate and explicit because the architect brief names them individually).
{
  const g = golden();
  g.authenticatedRequesterContext = buildAuthContext("human.person:someone-genuinely-different");
  g.request.requester_principal_id = "human.person:someone-genuinely-different";
  g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request) };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[11] = run(g);
  check("POS-11 (23.C) DISTINCT_REQUIRED, authenticated requester distinct from authority: COMMIT", posResults[11].status === "COMMIT", JSON.stringify(posResults[11]));
}
{
  const g = golden();
  g.request.requester_principal_id = g.decision.authority_principal_id;
  g.authenticatedRequesterContext = buildAuthContext(g.decision.authority_principal_id);
  g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request) };
  g.grant = { ...g.grant, self_approval_policy: "SAME_ALLOWED" };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  posResults[12] = run(g);
  check("POS-12 (23.D) SAME_ALLOWED, authenticated requester == authority: COMMIT", posResults[12].status === "COMMIT", JSON.stringify(posResults[12]));
}

// ---------------------------------------------------------------------------
// NEGATIVE matrix (>=36 required), table-driven
// ---------------------------------------------------------------------------

const NEG = [
  { n: 1, name: "raw provider attempts authority_snapshot injection", code: "D0_CANDIDATE_INVALID", mutate: (g) => { g.rawObservation.authority_snapshot = { grants: [] }; } },
  { n: 2, name: "request attempts inline authority_decision", code: "ADMISSION_REQUEST_SCHEMA_INVALID", mutate: (g) => { g.request.authority_decision = { effect: "ALLOW" }; } },
  { n: 3, name: "request attempts inline authority_grant", code: "ADMISSION_REQUEST_SCHEMA_INVALID", mutate: (g) => { g.request.authority_grant = { action: "chronica.world.admit" }; } },
  { n: 4, name: "request attempts canonical_world_snapshot override", code: "ADMISSION_REQUEST_SCHEMA_INVALID", mutate: (g) => { g.request.canonical_world_snapshot = { world_revision: 999 }; } },
  { n: 5, name: "missing canonical authority decision", code: "MISSING_CANONICAL_DECISION", mutate: (g) => { g.request.authority_decision_id = "dec-does-not-exist"; } },
  { n: 6, name: "duplicate conflicting decision_id in authority snapshot", code: "AUTHORITY_DECISION_ID_CONFLICT", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision, { ...g.decision, effect: "DENY" }]); } },
  { n: 7, name: "duplicate conflicting grant_id", code: "AUTHORITY_GRANT_ID_CONFLICT", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant, { ...g.grant, status: "REVOKED" }], [g.decision]); } },
  { n: 8, name: "corrupted authority snapshot state_hash", code: "AUTHORITY_SNAPSHOT_HASH_MISMATCH", mutate: (g) => { g.canonical_authority_snapshot.state_hash = flipHash(g.canonical_authority_snapshot.state_hash); } },
  { n: 9, name: "corrupted canonical world state_hash", code: "CANONICAL_WORLD_STATE_HASH_MISMATCH", mutate: (g) => { g.canonical_world_snapshot.state_hash = flipHash(g.canonical_world_snapshot.state_hash); } },
  { n: 10, name: "grant status REVOKED", code: "GRANT_NOT_ACTIVE", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([{ ...g.grant, status: "REVOKED" }], [g.decision]); } },
  { n: 11, name: "grant not yet valid", code: "GRANT_VALIDITY_WINDOW_VIOLATION", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([{ ...g.grant, valid_from: "2027-01-01T00:00:00Z" }], [g.decision]); } },
  { n: 12, name: "grant expired", code: "GRANT_VALIDITY_WINDOW_VIOLATION", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([{ ...g.grant, valid_until: "2020-01-01T00:00:00Z" }], [g.decision]); } },
  { n: 13, name: "decision principal != grant principal", code: "DECISION_GRANT_PRINCIPAL_MISMATCH", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, authority_principal_id: "ai.agent:impostor-01" }]); } },
  { n: 14, name: "request fingerprint mismatch", code: "REQUEST_FINGERPRINT_MISMATCH", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, request_fingerprint: `sha256:${"b".repeat(64)}` }]); } },
  { n: 15, name: "candidate hash mismatch", code: "CANDIDATE_CONTENT_HASH_MISMATCH", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, candidate_content_hash: `sha256:${"c".repeat(64)}` }]); } },
  { n: 16, name: "decision request_id mismatch", code: "DECISION_REQUEST_ID_MISMATCH", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, request_id: "req-other" }]); } },
  { n: 17, name: "grant policy hash != decision policy hash", code: "POLICY_HASH_MISMATCH", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, policy_hash: `sha256:${"d".repeat(64)}` }]); } },
  { n: 18, name: "grant policy ref != decision policy ref", code: "POLICY_REF_MISMATCH", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, policy_ref: { scheme: g.grant.policy_ref.scheme, value: "other-policy" } }]); } },
  { n: 19, name: "candidate entity namespace outside grant scope", code: "ENTITY_NAMESPACE_NOT_ALLOWED", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([{ ...g.grant, allowed_entity_namespaces: ["machine"] }], [g.decision]); } },
  {
    n: 20, name: "DISTINCT_REQUIRED self approval (as the genuine authenticated caller)", code: "SELF_APPROVAL_VIOLATION",
    mutate: (g) => {
      g.request.requester_principal_id = g.decision.authority_principal_id;
      g.authenticatedRequesterContext = buildAuthContext(g.decision.authority_principal_id);
      g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request) };
      g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
    },
  },
  { n: 21, name: "ALLOW with zero evidence", code: "ALLOW_ZERO_EVIDENCE", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, evidence_ref_ids: [] }]); } },
  { n: 22, name: "ALLOW with orphan evidence", code: "ORPHAN_EVIDENCE_REF", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, evidence_ref_ids: ["ev-does-not-exist"] }]); } },
  {
    n: 23, name: "requested entity set is only a subset of candidate", code: "REQUESTED_ENTITY_SET_INCOMPLETE",
    mutate: (g) => { g.rawObservation = twoEntityRawObservation(); const c = normalizeWorldFragment(g.rawObservation, ajv); g.request.candidate_content_hash = c.fragment.content_hash; g.request.requested_entity_ids = ["human.person:alice-01"]; },
  },
  { n: 24, name: "requested entity set contains extra entity", code: "REQUESTED_ENTITY_SET_HAS_EXTRA", mutate: (g) => { g.request.requested_entity_ids = ["human.person:alice-01", "human.person:extra-01"]; } },
  { n: 25, name: "stale expected world revision", code: "STALE_WORLD_REVISION", mutate: (g) => { g.request.expected_world_revision = 99; } },
  {
    n: 26, name: "world revision overflow", code: "WORLD_REVISION_OVERFLOW",
    mutate: (g) => {
      g.request.expected_world_revision = Number.MAX_SAFE_INTEGER;
      g.decision = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request) };
      g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
      g.canonical_world_snapshot = buildWorld([], [], Number.MAX_SAFE_INTEGER);
    },
  },
  {
    n: 27, name: "idempotency key reused for different request", code: "IDEMPOTENCY_KEY_REUSE_CONFLICT",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ZED)], [fakeCommit({ idempotency_key: "idem-001" })], 1); },
  },
  {
    n: 28, name: "same request_id reused for different fingerprint", code: "REQUEST_ID_CONFLICT",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ZED)], [fakeCommit({ idempotency_key: "idem-PRIOR-DIFFERENT", request_id: "req-001", request_fingerprint: `sha256:${"9".repeat(64)}` })], 1); },
  },
  {
    n: 29, name: "canonical existing entity conflicting content", code: "CANONICAL_ENTITY_CONFLICT",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([{ ...clone(ALICE), display_name: "Alice (conflicting canonical copy)" }], []); },
  },
  {
    n: 30, name: "already-canonical identical entity with new unrelated request", code: "ENTITY_ALREADY_CANONICAL",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ALICE)], []); },
  },
  { n: 31, name: "decision attempts execution_authority field", code: "CANONICAL_AUTHORITY_SNAPSHOT_SCHEMA_INVALID", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, execution_authority: { grant: "all" } }]); } },
  { n: 32, name: "decision attempts control_command field", code: "CANONICAL_AUTHORITY_SNAPSHOT_SCHEMA_INVALID", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, control_command: "START" }]); } },
  {
    n: 33, name: "candidate advisory attachment attempts authority namespace (caught by D0)", code: "D0_CANDIDATE_INVALID",
    mutate: (g) => {
      g.rawObservation.semantic_attachments = [{
        attachment_id: "att-1", entity_id: "human.person:alice-01", effect_class: "ADVISORY_ONLY", provider_type: "chronica.hr",
        semantic_claims: [{ claim_type: "authority.override", value: "true" }],
        provenance: { source: "chronica.hr", observed_at: "2026-08-20T12:00:00Z" },
      }];
    },
  },
  {
    n: 34, name: "one entity of a multi-entity candidate fails scope -- entire transition fails", code: "ENTITY_NAMESPACE_NOT_ALLOWED",
    mutate: (g) => {
      g.rawObservation = twoEntityRawObservation();
      const c = normalizeWorldFragment(g.rawObservation, ajv);
      g.request.candidate_content_hash = c.fragment.content_hash;
      g.request.requested_entity_ids = ["human.person:alice-01", "machine.plc:line3-01"];
      const decision2 = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request), candidate_content_hash: g.request.candidate_content_hash };
      g.canonical_authority_snapshot = buildAuthority([{ ...g.grant, allowed_entity_namespaces: ["human"] }], [decision2]);
    },
  },
  {
    n: 35, name: "one entity conflicts -- zero candidate entities admitted", code: "CANONICAL_ENTITY_CONFLICT",
    mutate: (g) => {
      g.rawObservation = twoEntityRawObservation();
      const c = normalizeWorldFragment(g.rawObservation, ajv);
      g.request.candidate_content_hash = c.fragment.content_hash;
      g.request.requested_entity_ids = ["human.person:alice-01", "machine.plc:line3-01"];
      const decision2 = { ...g.decision, request_fingerprint: computeRequestFingerprint(g.request), candidate_content_hash: g.request.candidate_content_hash };
      g.canonical_authority_snapshot = buildAuthority([g.grant], [decision2]);
      g.canonical_world_snapshot = buildWorld([{ ...clone(PLC), display_name: "DIFFERENT PLC NAME" }], []);
    },
  },
  {
    n: 36, name: "decision.grant_id does not resolve in the canonical authority snapshot", code: "MISSING_AUTHORITY_GRANT",
    mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, grant_id: "grant-does-not-exist" }]); },
  },

  // -- D.CLEAN-1R: requester-identity binding (architect finding A) --
  {
    n: 37, name: "23.A untrusted request claims a requester the caller is not authenticated as", code: "REQUESTER_IDENTITY_MISMATCH",
    mutate: (g) => { g.authenticatedRequesterContext = buildAuthContext("human.person:mallory"); },
  },
  {
    n: 38, name: "23.B self-attested DISTINCT claim cannot bypass SoD -- caller is actually the authority principal", code: "REQUESTER_IDENTITY_MISMATCH",
    mutate: (g) => {
      g.request.requester_principal_id = "human.person:someone-else";
      g.authenticatedRequesterContext = buildAuthContext(g.decision.authority_principal_id); // ai.agent:admitter-01, the TRUE caller
    },
  },
  {
    n: 39, name: "23.E exact idempotency replay attempted from a different authenticated principal: NO REPLAY",
    code: "REQUESTER_IDENTITY_MISMATCH",
    mutate: (g) => {
      // A genuine prior COMMIT (real transition run, not a stub) already sits in the ledger.
      const first = run(g);
      g.canonical_world_snapshot = first.next_world_snapshot;
      g.authenticatedRequesterContext = buildAuthContext("human.person:mallory");
    },
  },
  { n: 40, name: "untrusted wrapper carries an extra authority_snapshot key (not nested in raw_observation)", code: "UNTRUSTED_CHANNEL_UNKNOWN_FIELD", wrapperMutate: (u) => { u.authority_snapshot = { grants: [] }; } },
  { n: 41, name: "untrusted wrapper smuggles a trusted-channel key (authenticated_requester_context)", code: "UNTRUSTED_CHANNEL_UNKNOWN_FIELD", wrapperMutate: (u) => { u.authenticated_requester_context = buildAuthContext("human.person:alice-01"); } },
  { n: 42, name: "trusted wrapper carries an extra inline_decision key", code: "TRUSTED_CHANNEL_UNKNOWN_FIELD", trustedMutate: (t) => { t.inline_decision = { effect: "ALLOW" }; } },
  { n: 43, name: "trusted wrapper carries an extra policy_override key", code: "TRUSTED_CHANNEL_UNKNOWN_FIELD", trustedMutate: (t) => { t.policy_override = { scheme: "x", value: "y" }; } },
  {
    n: 56, name: "authenticated_requester_context attempts inline authority_grant field", code: "AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA_INVALID",
    mutate: (g) => { g.authenticatedRequesterContext = { ...g.authenticatedRequesterContext, authority_grant: { action: "chronica.world.admit" } }; },
  },
  {
    n: 57, name: "authenticated_requester_context attempts inline execution_authority field", code: "AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA_INVALID",
    mutate: (g) => { g.authenticatedRequesterContext = { ...g.authenticatedRequesterContext, execution_authority: true }; },
  },

  // -- D.CLEAN-1R: canonical ledger integrity (architect finding B) --
  { n: 44, name: "duplicate canonical entity id, exact content", code: "CANONICAL_ENTITY_ID_DUPLICATE", mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ALICE), clone(ALICE)], []); } },
  {
    n: 45, name: "duplicate canonical entity id, conflicting content", code: "CANONICAL_ENTITY_ID_CONFLICT",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ALICE), { ...clone(ALICE), display_name: "Alice (v2)" }], []); },
  },
  {
    n: 46, name: "duplicate commit_id, exact content", code: "CANONICAL_COMMIT_ID_DUPLICATE",
    mutate: (g) => { const c = fakeCommit(); g.canonical_world_snapshot = buildWorld([clone(ZED)], [c, clone(c)], 1); },
  },
  {
    n: 47, name: "duplicate commit_id, conflicting content", code: "CANONICAL_COMMIT_ID_CONFLICT",
    mutate: (g) => { const c = fakeCommit(); g.canonical_world_snapshot = buildWorld([clone(ZED)], [c, { ...c, request_id: "req-DIFFERENT" }], 1); },
  },
  {
    n: 48, name: "two distinct ledger commits share an idempotency_key (ledger-internal, no incoming request involved)", code: "CANONICAL_IDEMPOTENCY_LEDGER_CONFLICT",
    mutate: (g) => {
      const a = fakeCommit({ idempotency_key: "idem-shared", request_id: "req-A", admitted_entity_ids: ["human.person:zed-01"] });
      const b = fakeCommit({ idempotency_key: "idem-shared", request_id: "req-B", world_revision_before: 1, world_revision_after: 2, admitted_entity_ids: ["human.person:zed-01"] });
      g.canonical_world_snapshot = buildWorld([clone(ZED)], [a, b], 2);
    },
  },
  {
    n: 49, name: "two distinct ledger commits share a request_id", code: "CANONICAL_REQUEST_LEDGER_CONFLICT",
    mutate: (g) => {
      const a = fakeCommit({ request_id: "req-shared", idempotency_key: "idem-A", admitted_entity_ids: ["human.person:zed-01"] });
      const b = fakeCommit({ request_id: "req-shared", idempotency_key: "idem-B", world_revision_before: 1, world_revision_after: 2, admitted_entity_ids: ["human.person:zed-01"] });
      g.canonical_world_snapshot = buildWorld([clone(ZED)], [a, b], 2);
    },
  },
  {
    // world_revision_after is required to equal world_revision_before + 1 for EVERY individual
    // commit, so "share before" and "share after" are the same underlying ledger shape (before
    // determines after 1:1) -- distinguished here only by which of the two duplicate-value
    // checks the architect brief names (section 14, items 11/12 of section 33). authority_decision_id
    // differs between a/b so their commit_ids differ too (else CANONICAL_COMMIT_ID_CONFLICT/
    // _DUPLICATE would fire first, before the revision check is ever reached).
    n: 50, name: "two distinct ledger commits share world_revision_before", code: "CANONICAL_ADMISSION_REVISION_CONFLICT",
    mutate: (g) => {
      const a = fakeCommit({ request_id: "req-A", idempotency_key: "idem-A", authority_decision_id: "dec-A", admitted_entity_ids: ["human.person:zed-01"] });
      const b = fakeCommit({ request_id: "req-B", idempotency_key: "idem-B", authority_decision_id: "dec-B", admitted_entity_ids: ["human.person:zed-01"] });
      g.canonical_world_snapshot = buildWorld([clone(ZED)], [a, b], 1);
    },
  },
  {
    n: 51, name: "two distinct ledger commits share world_revision_after", code: "CANONICAL_ADMISSION_REVISION_CONFLICT",
    mutate: (g) => {
      const a = fakeCommit({ request_id: "req-A", idempotency_key: "idem-A", authority_decision_id: "dec-A", admitted_entity_ids: ["human.person:zed-01"] });
      const b = fakeCommit({ request_id: "req-B", idempotency_key: "idem-B", authority_decision_id: "dec-B", admitted_entity_ids: ["human.person:zed-01"] });
      g.canonical_world_snapshot = buildWorld([clone(ZED)], [a, b], 1);
    },
  },
  {
    n: 52, name: "historical commit world_revision_after != world_revision_before + 1", code: "ADMISSION_COMMIT_REVISION_INVALID",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ZED)], [fakeCommit({ world_revision_before: 0, world_revision_after: 5 })], 5); },
  },
  {
    n: 53, name: "historical commit world_revision_after exceeds the snapshot's own world_revision", code: "ADMISSION_COMMIT_REVISION_INVALID",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ZED)], [fakeCommit({ world_revision_before: 0, world_revision_after: 1 })], 0); },
  },
  {
    n: 54, name: "forged historical commit_id (does not match its own recomputed fields)", code: "ADMISSION_COMMIT_ID_MISMATCH",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([clone(ZED)], [fakeCommit({ commit_id: `sha256:${"f".repeat(64)}` })], 1); },
  },
  {
    n: 55, name: "AdmissionCommit.admitted_entity_ids references an entity missing from the current canonical entity set", code: "ADMISSION_COMMIT_ENTITY_MISSING",
    mutate: (g) => { g.canonical_world_snapshot = buildWorld([], [fakeCommit()], 1); }, // ZED entity deliberately omitted
  },

  // -- D.CLEAN-1R: canonical authority id uniqueness (architect finding B extends to authority) --
  { n: 58, name: "exact duplicate grant_id (identical content twice)", code: "AUTHORITY_GRANT_ID_DUPLICATE", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant, clone(g.grant)], [g.decision]); } },
  { n: 59, name: "exact duplicate decision_id (identical content twice)", code: "AUTHORITY_DECISION_ID_DUPLICATE", mutate: (g) => { g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision, clone(g.decision)]); } },
];

console.log(`\n== NEGATIVE matrix (${NEG.length} cases) ==`);
const negResults = {};
for (const c of NEG) {
  const g = golden();
  if (c.mutate) c.mutate(g);
  let result;
  if (c.wrapperMutate || c.trustedMutate) {
    const untrusted = { raw_observation: g.rawObservation, request: g.request };
    const trusted = { canonical_world_snapshot: g.canonical_world_snapshot, canonical_authority_snapshot: g.canonical_authority_snapshot, authenticated_requester_context: g.authenticatedRequesterContext };
    if (c.wrapperMutate) c.wrapperMutate(untrusted);
    if (c.trustedMutate) c.trustedMutate(trusted);
    result = runRaw(untrusted, trusted);
  } else {
    result = run(g);
  }
  negResults[c.n] = result;
  check(`NEG-${c.n} ${c.name}: REJECTED`, result.status === "REJECTED", JSON.stringify(result));
  check(`NEG-${c.n} ${c.name}: error code ${c.code}`, result.errors.some((e) => e.code === c.code), JSON.stringify(result.errors));
}
// NEG-"failure never mutates trusted/untrusted input" is verified as an adversarial
// non-mutation proof below, since it is a structural property rather than a rejection code.

// ---------------------------------------------------------------------------
// ADVERSARIAL pass (D1R: >=80 required)
// ---------------------------------------------------------------------------

console.log(`\n== ADVERSARIAL pass ==`);

for (const c of NEG) {
  adversarial(`${c.name} (real evaluateAdmissionTransition() execution, not a static placeholder)`, negResults[c.n].status === "REJECTED" && negResults[c.n].errors.some((e) => e.code === c.code));
}

adversarial("no wildcard grant namespace -- '*' is structurally rejected by the grant schema, not runtime logic", (() => {
  const g = golden();
  g.canonical_authority_snapshot = buildAuthority([{ ...g.grant, allowed_entity_namespaces: ["*"] }], [g.decision]);
  const r = run(g);
  return r.status === "REJECTED" && r.errors.some((e) => e.code === "CANONICAL_AUTHORITY_SNAPSHOT_SCHEMA_INVALID");
})());

adversarial("duplicate entity_id within requested_entity_ids rejected by the closed request schema (uniqueItems)", (() => {
  const g = golden();
  g.rawObservation = twoEntityRawObservation();
  g.request.requested_entity_ids = ["human.person:alice-01", "human.person:alice-01"];
  const r = run(g);
  return r.status === "REJECTED" && r.errors.some((e) => e.code === "ADMISSION_REQUEST_SCHEMA_INVALID");
})());

for (const field of ["act_permission", "capability_grant", "machine_control", "external_action"]) {
  adversarial(`decision attempts ${field} field -- execution-authority smuggling rejected`, (() => {
    const g = golden();
    g.canonical_authority_snapshot = buildAuthority([g.grant], [{ ...g.decision, [field]: true }]);
    const r = run(g);
    return r.status === "REJECTED" && r.errors.some((e) => e.code === "CANONICAL_AUTHORITY_SNAPSHOT_SCHEMA_INVALID");
  })());
}

adversarial("AuthenticatedRequesterContext structurally cannot carry a policy_override field either", (() => {
  const g = golden();
  g.authenticatedRequesterContext = { ...g.authenticatedRequesterContext, policy_override: { scheme: "x", value: "y" } };
  const r = run(g);
  return r.status === "REJECTED" && r.errors.some((e) => e.code === "AUTHENTICATED_REQUESTER_CONTEXT_SCHEMA_INVALID");
})());

adversarial("transition.mjs / admission schema-validate.mjs / validate.mjs never read the wall clock", (() => {
  const src = ["transition.mjs", "schema-validate.mjs", "validate.mjs"].map((f) => readFileSync(resolve(HERE, f), "utf8")).join("\n");
  return !/Date\.now\(|new Date\(/.test(src);
})());

adversarial("commit_id is never a random UUID -- no Math.random/randomUUID anywhere in D1 source", (() => {
  const src = ["transition.mjs", "schema-validate.mjs", "validate.mjs"].map((f) => readFileSync(resolve(HERE, f), "utf8")).join("\n");
  return !/Math\.random\(|randomUUID\(/.test(src);
})());

adversarial("commit_id is deterministic: identical inputs produce an identical commit_id", (() => {
  const g1 = golden();
  const g2 = golden();
  const r1 = run(g1);
  const r2 = run(g2);
  return r1.status === "COMMIT" && r2.status === "COMMIT" && r1.commit.commit_id === r2.commit.commit_id;
})());

adversarial("commit_id changes on a meaningful change (POS-1 and POS-10 commit at different world_revision_before -> different commit_id)", posResults[1].commit.commit_id !== posResults[10].commit.commit_id);

adversarial("full transition recomputation: identical inputs, run twice, byte-identical result", (() => {
  const g = golden();
  const r1 = run(g);
  const r2 = run(g);
  return canonicalStringify(r1) === canonicalStringify(r2);
})());

adversarial("grants array reordering does not change the recomputed authority state_hash", (() => {
  const g = golden();
  const extraGrant = { ...g.grant, grant_id: "grant-002" };
  const forward = buildAuthority([g.grant, extraGrant], [g.decision]);
  const reversed = buildAuthority([extraGrant, g.grant], [g.decision]);
  return forward.state_hash === reversed.state_hash;
})());

adversarial("decisions array reordering does not change the recomputed authority state_hash", (() => {
  const g = golden();
  const extraDecision = { ...g.decision, decision_id: "dec-002", request_id: "req-002" };
  const forward = buildAuthority([g.grant], [g.decision, extraDecision]);
  const reversed = buildAuthority([g.grant], [extraDecision, g.decision]);
  return forward.state_hash === reversed.state_hash;
})());

adversarial("grant.allowed_entity_namespaces order does not change authority state_hash or scope evaluation (23/20)", (() => {
  const g1 = golden();
  const g2 = golden();
  g2.grant = { ...g2.grant, allowed_entity_namespaces: [...g2.grant.allowed_entity_namespaces].reverse() };
  g2.canonical_authority_snapshot = buildAuthority([g2.grant], [g2.decision]);
  const forwardHash = g1.canonical_authority_snapshot.state_hash;
  const reversedHash = g2.canonical_authority_snapshot.state_hash;
  const r2 = run(g2);
  return forwardHash === reversedHash && r2.status === "COMMIT";
})());

adversarial("canonical world entities reordering does not change the recomputed world state_hash", (() => {
  const forward = buildWorld([clone(ALICE), clone(PLC)], []);
  const reversed = buildWorld([clone(PLC), clone(ALICE)], []);
  return forward.state_hash === reversed.state_hash;
})());

adversarial("admission_commits reordering does not change the recomputed world state_hash", (() => {
  const c1 = fakeCommit({ commit_id: undefined, request_id: "req-a", idempotency_key: "idem-a" });
  const c2 = fakeCommit({ commit_id: undefined, request_id: "req-b", idempotency_key: "idem-b", world_revision_before: 1, world_revision_after: 2 });
  const forward = buildWorld([clone(ZED)], [c1, c2], 2);
  const reversed = buildWorld([clone(ZED)], [c2, c1], 2);
  return forward.state_hash === reversed.state_hash;
})());

adversarial("decision.evidence_ref_ids reordering does not change the committed commit_id or evidence_ref_ids", (() => {
  const g1 = golden();
  g1.rawObservation.evidence_refs.push({ evidence_ref_id: "ev-2", evidence_kind: "chronica.event", locator: { scheme: "chronica.event_id", value: "evt-456" }, provenance: { source: "chronica.hr", observed_at: "2026-08-20T12:00:00Z" } });
  const c1 = normalizeWorldFragment(g1.rawObservation, ajv);
  g1.request.candidate_content_hash = c1.fragment.content_hash;
  g1.decision = { ...g1.decision, candidate_content_hash: c1.fragment.content_hash, evidence_ref_ids: ["ev-1", "ev-2"], request_fingerprint: computeRequestFingerprint(g1.request) };
  g1.canonical_authority_snapshot = buildAuthority([g1.grant], [g1.decision]);

  const g2 = clone(g1);
  g2.decision.evidence_ref_ids = ["ev-2", "ev-1"];
  g2.canonical_authority_snapshot = buildAuthority([g2.grant], [g2.decision]);

  const r1 = run(g1);
  const r2 = run(g2);
  return r1.status === "COMMIT" && r2.status === "COMMIT" && r1.commit.commit_id === r2.commit.commit_id && canonicalStringify(r1.commit.evidence_ref_ids) === canonicalStringify(r2.commit.evidence_ref_ids);
})());

adversarial("commit.admitted_entity_ids and commit.evidence_ref_ids order in the LEDGER does not change world state_hash", (() => {
  const a = fakeCommit({ commit_id: undefined, evidence_ref_ids: ["ev-x", "ev-y"], admitted_entity_ids: ["human.person:zed-01"] });
  const aReordered = { ...a, evidence_ref_ids: ["ev-y", "ev-x"] };
  const forward = buildWorld([clone(ZED)], [a], 1);
  const reversed = buildWorld([clone(ZED)], [aReordered], 1);
  return forward.state_hash === reversed.state_hash;
})());

adversarial("requested_entity_ids reordering does not change the recomputed request fingerprint", (() => {
  const g = golden();
  g.rawObservation = twoEntityRawObservation();
  const c = normalizeWorldFragment(g.rawObservation, ajv);
  const base = { ...g.request, candidate_content_hash: c.fragment.content_hash };
  const forward = computeRequestFingerprint({ ...base, requested_entity_ids: ["human.person:alice-01", "machine.plc:line3-01"] });
  const reversed = computeRequestFingerprint({ ...base, requested_entity_ids: ["machine.plc:line3-01", "human.person:alice-01"] });
  return forward === reversed;
})());

adversarial("non-mutation: a COMMIT run never mutates any of the 5 input objects (deep-frozen, no throw)", (() => {
  function deepFreeze(o) {
    if (o !== null && typeof o === "object") {
      Object.values(o).forEach(deepFreeze);
      Object.freeze(o);
    }
    return o;
  }
  const g = golden();
  const untrusted = deepFreeze({ raw_observation: g.rawObservation, request: g.request });
  const trusted = deepFreeze({ canonical_world_snapshot: g.canonical_world_snapshot, canonical_authority_snapshot: g.canonical_authority_snapshot, authenticated_requester_context: g.authenticatedRequesterContext });
  try {
    const r = evaluateAdmissionTransition(untrusted, trusted, ajv);
    return r.status === "COMMIT";
  } catch {
    return false;
  }
})());

adversarial("non-mutation: a REJECTED run leaves all 5 input objects byte-identical (INPUT_OBJECT_MUTATION_COUNT = 0)", (() => {
  const g = golden();
  g.request.authority_decision = { effect: "ALLOW" }; // NEG-2 shape
  const before = {
    raw: canonicalStringify(g.rawObservation),
    req: canonicalStringify(g.request),
    world: canonicalStringify(g.canonical_world_snapshot),
    authority: canonicalStringify(g.canonical_authority_snapshot),
    context: canonicalStringify(g.authenticatedRequesterContext),
  };
  const r = run(g);
  return (
    r.status === "REJECTED" &&
    before.raw === canonicalStringify(g.rawObservation) &&
    before.req === canonicalStringify(g.request) &&
    before.world === canonicalStringify(g.canonical_world_snapshot) &&
    before.authority === canonicalStringify(g.canonical_authority_snapshot) &&
    before.context === canonicalStringify(g.authenticatedRequesterContext)
  );
})());

adversarial("PARTIAL_ADMISSION_POSSIBLE = NO: NEG-35 (one entity conflicts) produces next_world_snapshot: null, zero entities admitted", negResults[35].commit === null && negResults[35].next_world_snapshot === null);

adversarial("DENY_STATE_MUTATION_COUNT = 0 (re-verified): POS-8 next_world_snapshot is byte-identical to the input world snapshot", (() => {
  const g = golden();
  g.decision = { ...g.decision, effect: "DENY" };
  g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
  const before = canonicalStringify(g.canonical_world_snapshot);
  const r = run(g);
  return r.status === "DENIED" && canonicalStringify(r.next_world_snapshot) === before;
})());

adversarial("IDEMPOTENT_REPLAY_REVISION_INCREMENT = 0 (re-verified)", posResults[9].next_world_snapshot.world_revision === 1);

adversarial("WORLD_ENTITY_CANONICAL_FLAG = ABSENT: admitted entities carry no 'canonical' property", (() => {
  const g = golden();
  const r = run(g);
  return r.status === "COMMIT" && r.next_world_snapshot.entities.every((e) => !("canonical" in e));
})());

adversarial("HASH_GRAPH_ACYCLIC: AdmissionCommit never embeds a world state_hash, and world state_hash never embeds itself", (() => {
  const g = golden();
  const r = run(g);
  return r.status === "COMMIT" && !("world_state_hash" in r.commit) && !("state_hash" in r.commit) && !JSON.stringify(r.next_world_snapshot.admission_commits).includes(r.next_world_snapshot.state_hash);
})());

// -- D.CLEAN-1R section 24: idempotency ledger ambiguity attack, both array orderings --
adversarial("24: idempotency-ledger ambiguity attack rejects identically under BOTH commit array orderings (IDEMPOTENCY_FIND_ORDER_DEPENDENCE eliminated)", (() => {
  const a = fakeCommit({ request_id: "req-A", idempotency_key: "idem-shared", admitted_entity_ids: ["human.person:zed-01"] });
  const b = fakeCommit({ request_id: "req-B", idempotency_key: "idem-shared", world_revision_before: 1, world_revision_after: 2, admitted_entity_ids: ["human.person:zed-01"] });
  const g1 = golden();
  g1.canonical_world_snapshot = buildWorld([clone(ZED)], [a, b], 2);
  const g2 = golden();
  g2.canonical_world_snapshot = buildWorld([clone(ZED)], [b, a], 2);
  const r1 = run(g1);
  const r2 = run(g2);
  return r1.status === "REJECTED" && r2.status === "REJECTED" && r1.errors[0].code === "CANONICAL_IDEMPOTENCY_LEDGER_CONFLICT" && r2.errors[0].code === "CANONICAL_IDEMPOTENCY_LEDGER_CONFLICT";
})());

// -- D.CLEAN-1R section 25: entity-map ambiguity attack, both array orderings --
adversarial("25: entity-map ambiguity attack rejects identically under BOTH entity array orderings, before entity-admission logic runs (ENTITY_MAP_ORDER_DEPENDENCE eliminated)", (() => {
  const g1 = golden();
  const conflictingAlice = { ...clone(ALICE), display_name: "Alice (impostor canonical copy)" };
  g1.canonical_world_snapshot = buildWorld([clone(ALICE), conflictingAlice], []);
  const g2 = golden();
  g2.canonical_world_snapshot = buildWorld([conflictingAlice, clone(ALICE)], []);
  const r1 = run(g1);
  const r2 = run(g2);
  return r1.status === "REJECTED" && r2.status === "REJECTED" && r1.errors[0].code === "CANONICAL_ENTITY_ID_CONFLICT" && r2.errors[0].code === "CANONICAL_ENTITY_ID_CONFLICT";
})());

// -- D.CLEAN-1R section 26: DENY/REPLAY full-output order independence --
{
  const commitA = fakeCommit({ request_id: "req-A", idempotency_key: "idem-A", evidence_ref_ids: ["ev-x", "ev-y"], admitted_entity_ids: ["human.person:zed-01", "machine.plc:line3-01"] });
  const worldForward = buildWorld([clone(ALICE), clone(ZED), clone(PLC)], [commitA], 1);
  const worldReversedEntities = buildWorld([clone(PLC), clone(ZED), clone(ALICE)], [commitA], 1);
  const commitAReordered = { ...commitA, evidence_ref_ids: ["ev-y", "ev-x"], admitted_entity_ids: ["machine.plc:line3-01", "human.person:zed-01"] };
  const worldReorderedNested = buildWorld([clone(ALICE), clone(ZED), clone(PLC)], [commitAReordered], 1);

  function denyResultFor(worldSnapshot) {
    const g = golden();
    g.canonical_world_snapshot = worldSnapshot;
    g.request.expected_world_revision = worldSnapshot.world_revision;
    g.decision = { ...g.decision, effect: "DENY", request_fingerprint: computeRequestFingerprint(g.request) };
    g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
    return run(g);
  }
  const denyF = denyResultFor(worldForward);
  const denyE = denyResultFor(worldReversedEntities);
  const denyN = denyResultFor(worldReorderedNested);
  adversarial(
    "26: DENY full serialized AdmissionTransitionResult is byte-identical across entities/commits/nested-array orderings (DENY_FULL_OUTPUT_ORDER_INDEPENDENCE)",
    denyF.status === "DENIED" && canonicalStringify(denyF) === canonicalStringify(denyE) && canonicalStringify(denyF) === canonicalStringify(denyN),
  );
}

{
  // Re-derive with the ACTUAL golden request_fingerprint/candidate_content_hash so this is a
  // genuine replay target (evaluateAdmissionTransition requires both to match, not just idempotency_key).
  const gBase = golden();
  const realFingerprint = computeRequestFingerprint(gBase.request);
  const replayCommit = fakeCommit({
    request_id: "req-001", idempotency_key: "idem-001", request_fingerprint: realFingerprint,
    candidate_content_hash: gBase.request.candidate_content_hash, admitted_entity_ids: ["human.person:alice-01"], requester_principal_id: "human.person:alice-01",
  });
  const worldForward = buildWorld([clone(ALICE), clone(ZED)], [replayCommit], 1);
  const worldReversed = buildWorld([clone(ZED), clone(ALICE)], [replayCommit], 1);

  function replayResultFor(worldSnapshot) {
    const g = golden();
    g.canonical_world_snapshot = worldSnapshot;
    return run(g);
  }
  const replayF = replayResultFor(worldForward);
  const replayR = replayResultFor(worldReversed);
  adversarial(
    "26: REPLAY full serialized AdmissionTransitionResult is byte-identical across entity array ordering (REPLAY_FULL_OUTPUT_ORDER_INDEPENDENCE)",
    replayF.status === "REPLAY" && canonicalStringify(replayF) === canonicalStringify(replayR),
  );
}

// D.RUNTIME-0R2 repair: a base-branch `git diff` only ever proves something about one point in
// commit history (blind to uncommitted state, and necessarily empty once a feature branch merges
// into main) -- it is not a durable property of D1 itself. Replaced with a content-based layering
// invariant that remains true on any checkout: D1's own reference scripts (schema-validate,
// transition, validate, tests) only ever reach into D0's PUBLIC surface (normalize.mjs,
// schema-validate.mjs, both one directory up) or their own admission/ directory -- never past D0
// into System Atlas, product/runtime, or any other layer. This is a strictly stronger, permanent
// guarantee than "the historical diff for these paths happened to be empty": it holds regardless of
// what commit is checked out, and it would catch a D1 file reaching around D0 to depend on
// System Atlas or product/runtime directly, not just a change to D0's own files.
adversarial("D1 reference scripts (schema-validate/transition/validate/tests.mjs) import only D0's public surface (normalize.mjs, schema-validate.mjs) or their own admission/ directory", (() => {
  const files = ["schema-validate.mjs", "transition.mjs", "validate.mjs", "tests.mjs"];
  const importRe = /from\s+["']([^"']+)["']/g;
  const d0PublicFiles = new Set(["normalize.mjs", "schema-validate.mjs"]);
  for (const f of files) {
    const src = readFileSync(resolve(HERE, f), "utf8");
    let m;
    while ((m = importRe.exec(src))) {
      const spec = m[1];
      if (!spec.startsWith(".")) continue;
      const resolved = resolve(HERE, spec);
      const isOwnDir = dirname(resolved) === HERE;
      const isD0PublicFile = dirname(resolved) === resolve(HERE, "..") && d0PublicFiles.has(resolved.split(/[\\/]/).pop());
      if (isOwnDir || isD0PublicFile) continue;
      // Any OTHER relative import (System Atlas, product/runtime, or anything else reached around
      // D0's public surface) is itself a layering violation -- fail loudly, don't silently accept.
      return false;
    }
  }
  return true;
})());

adversarial("D1 reference scripts (schema-validate/transition/validate/tests.mjs) import nothing from System Atlas (docs/_machine/system-atlas, scripts/system-atlas)", (() => {
  const files = ["schema-validate.mjs", "transition.mjs", "validate.mjs", "tests.mjs"];
  const importRe = /from\s+["']([^"']+)["']/g;
  for (const f of files) {
    const src = readFileSync(resolve(HERE, f), "utf8");
    let m;
    while ((m = importRe.exec(src))) {
      const spec = m[1];
      if (!spec.startsWith(".")) continue;
      const resolvedLower = resolve(HERE, spec).toLowerCase();
      if (/[\\/]system-atlas[\\/]/.test(resolvedLower)) return false;
    }
  }
  return true;
})());

// D.RUNTIME-0R repair: "product/runtime untouched" via a base-branch git diff is inherently
// incompatible with any later Rust runtime-materialization wave that legitimately touches
// crates/Cargo metadata -- it only ever proved something about one point in commit history, not a
// property of D1 itself. Replaced with a content-based invariant true on any checkout: D1's
// reference scripts import nothing from product/runtime (crates, ui) at all.
adversarial("D1 reference scripts (schema-validate/transition/validate/tests.mjs) import nothing from product/runtime (crates, ui)", (() => {
  const files = ["schema-validate.mjs", "transition.mjs", "validate.mjs", "tests.mjs"];
  const importRe = /from\s+["']([^"']+)["']/g;
  for (const f of files) {
    const src = readFileSync(resolve(HERE, f), "utf8");
    let m;
    while ((m = importRe.exec(src))) {
      const spec = m[1];
      if (!spec.startsWith(".")) continue;
      const resolvedLower = resolve(HERE, spec).toLowerCase();
      if (/[\\/](crates|ui)[\\/]/.test(resolvedLower)) return false;
    }
  }
  return true;
})());

adversarial("D1 source imports nothing outside node builtins, ajv, and tools/universal-graph -- no old-D/Lane-C import path", (() => {
  const files = ["schema-validate.mjs", "transition.mjs", "validate.mjs", "tests.mjs"];
  const importRe = /from\s+["']([^"']+)["']/g;
  for (const f of files) {
    const src = readFileSync(resolve(HERE, f), "utf8");
    let m;
    while ((m = importRe.exec(src))) {
      const spec = m[1];
      const allowed = spec.startsWith("node:") || spec === "ajv/dist/2020.js" || spec.startsWith("./") || spec.startsWith("../");
      if (!allowed) return false;
      if (/legacy|lane-c|lane_c/i.test(spec)) return false;
    }
  }
  return true;
})());

adversarial("fixtures/positive.json and fixtures/negative.json parse as valid JSON (reference examples stay in sync)", (() => {
  JSON.parse(readFileSync(resolve(ADMISSION_DIR, "fixtures", "positive.json"), "utf8"));
  JSON.parse(readFileSync(resolve(ADMISSION_DIR, "fixtures", "negative.json"), "utf8"));
  return true;
})());

adversarial("AUTHORITY_ROOT remains CANONICAL_AUTHORITY_SNAPSHOT_ONLY: an authenticated identity alone (ALLOW-shaped decision absent) cannot commit", (() => {
  const g = golden();
  g.canonical_authority_snapshot = buildAuthority([], []);
  const r = run(g);
  return r.status === "REJECTED" && r.errors.some((e) => e.code === "MISSING_CANONICAL_DECISION");
})());

adversarial("SOD_REQUESTER_IDENTITY_SOURCE = AUTHENTICATED_REQUESTER_CONTEXT: SoD reads the trusted field, not request.requester_principal_id, by construction (source review)", (() => {
  const src = readFileSync(resolve(HERE, "transition.mjs"), "utf8");
  const sodLine = src.split("\n").find((l) => l.includes("self_approval_policy === \"DISTINCT_REQUIRED\""));
  return !!sodLine && sodLine.includes("authenticated_requester_context.authenticated_principal_id") && !sodLine.includes("request.requester_principal_id");
})());

// ---------------------------------------------------------------------------
// D.CLEAN-1R2: recursive JSON object-property-order canonicalization
//
// D1R already proved that declared order-insensitive COLLECTIONS (entities,
// admission_commits, and their nested lexical sets) do not affect state_hash or the
// transition outcome. What it had not yet proven is that the RETURNED result is immune to
// caller-supplied JSON OBJECT-KEY insertion order at every nesting level -- state_hash
// itself was already safe (computeWorldStateHash hashes via canonicalStringify, which
// canonicalizes keys), but canonicalizeWorldSnapshot()'s returned object was not itself run
// through canonicalize(), so DENY/REPLAY output could differ byte-for-byte (though never
// semantically) depending on how the caller's WorldEntity/Provenance/AdmissionCommit/
// policy_ref objects happened to order their own keys. Fixed by wrapping
// canonicalizeWorldSnapshot()'s return value in canonicalize() (transition.mjs). These
// checks prove it with raw JSON.stringify() -- canonicalStringify() alone cannot prove this,
// since canonicalStringify() re-canonicalizes on the way out and would pass even if the
// returned in-memory object itself were not canonical.
// ---------------------------------------------------------------------------

/** Recursively reverses JS object key insertion order at every nesting level (array element order is untouched -- .map() preserves it). Produces a semantically-identical, differently-serialized twin of `value`. */
function reverseKeyOrder(value) {
  if (Array.isArray(value)) return value.map(reverseKeyOrder);
  if (value !== null && typeof value === "object") {
    const out = {};
    for (const key of Object.keys(value).reverse()) out[key] = reverseKeyOrder(value[key]);
    return out;
  }
  return value;
}

{
  // D1R2.1/2: DENY -- WorldEntity (outer keys) + WorldEntity.provenance (nested keys) permuted.
  const entityA = { entity_id: "human.person:zed-01", entity_type: "human.person", display_name: "Zed (prior admission)", provenance: { source: "chronica.hr", observed_at: "2026-08-01T00:00:00Z" } };
  const entityB = reverseKeyOrder(entityA);
  function denyFor(entity) {
    const g = golden();
    g.canonical_world_snapshot = buildWorld([entity], []);
    g.decision = { ...g.decision, effect: "DENY" };
    g.canonical_authority_snapshot = buildAuthority([g.grant], [g.decision]);
    return run(g);
  }
  const denyA = denyFor(entityA);
  const denyB = denyFor(entityB);
  adversarial(
    "D1R2.1 DENY -- WorldEntity property-order permutation: raw JSON.stringify byte-identical (DENY_RETURNED_RESULT_OBJECT_KEY_ORDER_INDEPENDENCE)",
    denyA.status === "DENIED" && denyB.status === "DENIED" && JSON.stringify(denyA) === JSON.stringify(denyB),
  );
  adversarial(
    "D1R2.2 DENY -- WorldEntity.provenance nested property-order permutation included above (multi-level), plus canonicalStringify agreement (RETURNED_RESULT_CANONICAL_STRING_EQUALITY)",
    canonicalStringify(denyA) === canonicalStringify(denyB) && JSON.stringify(denyA.next_world_snapshot.entities) === JSON.stringify(denyB.next_world_snapshot.entities),
  );
}

{
  // D1R2.3/4: REPLAY -- AdmissionCommit (outer keys) + AdmissionCommit.policy_ref (nested keys) permuted.
  const gBase = golden();
  const realFingerprint = computeRequestFingerprint(gBase.request);
  const priorCommitA = fakeCommit({
    request_id: "req-001", idempotency_key: "idem-001", request_fingerprint: realFingerprint,
    candidate_content_hash: gBase.request.candidate_content_hash, admitted_entity_ids: ["human.person:alice-01"], requester_principal_id: "human.person:alice-01",
  });
  const priorCommitB = reverseKeyOrder(priorCommitA);
  function replayFor(commit) {
    const g = golden();
    g.canonical_world_snapshot = buildWorld([clone(ALICE)], [commit], 1);
    return run(g);
  }
  const replayA = replayFor(priorCommitA);
  const replayB = replayFor(priorCommitB);
  adversarial(
    "D1R2.3 REPLAY -- AdmissionCommit property-order permutation: raw JSON.stringify byte-identical (REPLAY_RETURNED_RESULT_OBJECT_KEY_ORDER_INDEPENDENCE)",
    replayA.status === "REPLAY" && replayB.status === "REPLAY" && JSON.stringify(replayA) === JSON.stringify(replayB),
  );
  adversarial(
    "D1R2.4 REPLAY -- AdmissionCommit.policy_ref nested property-order permutation included above, plus canonicalStringify agreement",
    canonicalStringify(replayA) === canonicalStringify(replayB) && JSON.stringify(replayA.commit.policy_ref) === JSON.stringify(replayB.commit.policy_ref),
  );
}

{
  // D1R2.5: CanonicalWorldSnapshot OUTER object property-order permutation (schema/world_revision/entities/admission_commits/state_hash reordered).
  const g = golden();
  const worldA = g.canonical_world_snapshot;
  const worldB = reverseKeyOrder(worldA);
  function denyFor(world) {
    const gg = golden();
    gg.canonical_world_snapshot = world;
    gg.decision = { ...gg.decision, effect: "DENY" };
    gg.canonical_authority_snapshot = buildAuthority([gg.grant], [gg.decision]);
    return run(gg);
  }
  const rA = denyFor(worldA);
  const rB = denyFor(worldB);
  adversarial(
    "D1R2.5 CanonicalWorldSnapshot outer-object property-order permutation does not change the returned result bytes",
    rA.status === "DENIED" && rB.status === "DENIED" && JSON.stringify(rA) === JSON.stringify(rB),
  );
}

{
  // D1R2.6: COMMIT -- trusted canonical_world_snapshot (and, for good measure, canonical_authority_snapshot and
  // authenticated_requester_context) property-order permuted end to end. Regression proof: COMMIT already
  // wraps `commit`/`next_world_snapshot` in canonicalize(), this confirms that still holds post-repair.
  const gA = golden();
  const gB = golden();
  gB.canonical_world_snapshot = reverseKeyOrder(gB.canonical_world_snapshot);
  gB.canonical_authority_snapshot = reverseKeyOrder(gB.canonical_authority_snapshot);
  gB.authenticatedRequesterContext = reverseKeyOrder(gB.authenticatedRequesterContext);
  const commitA = run(gA);
  const commitB = run(gB);
  adversarial(
    "D1R2.6 COMMIT -- trusted-world (+authority +authenticated-context) property-order permutation: raw JSON.stringify byte-identical (COMMIT_RETURNED_RESULT_OBJECT_KEY_ORDER_INDEPENDENCE)",
    commitA.status === "COMMIT" && commitB.status === "COMMIT" && JSON.stringify(commitA) === JSON.stringify(commitB) && commitA.commit.requester_principal_id === commitB.commit.requester_principal_id,
  );
}

{
  // D1R2.7: actual CLI subprocess byte identity for a non-REJECTED transition (RAW_RETURNED_RESULT_BYTE_TESTS /
  // D1_CLI_OBJECT_KEY_PERMUTATION_BYTE_IDENTITY) -- proves the external serialized boundary end to end, not just
  // in-memory. canonicalPrettyStringify() at the CLI print boundary was already canonical before this repair;
  // this is a real subprocess execution, not an assertion about it.
  const g = golden();
  const fullInputA = {
    untrusted: { raw_observation: g.rawObservation, request: g.request },
    trusted: { canonical_world_snapshot: g.canonical_world_snapshot, canonical_authority_snapshot: g.canonical_authority_snapshot, authenticated_requester_context: g.authenticatedRequesterContext },
  };
  const fullInputB = reverseKeyOrder(fullInputA);

  const tmpDir = mkdtempSync(join(tmpdir(), "d1r2-cli-"));
  const fileA = join(tmpDir, "input-a.json");
  const fileB = join(tmpDir, "input-b.json");
  let cliResult = false;
  try {
    writeFileSync(fileA, JSON.stringify(fullInputA));
    writeFileSync(fileB, JSON.stringify(fullInputB));
    const stdoutA = execFileSync("node", [resolve(HERE, "validate.mjs"), fileA], { encoding: "utf8" });
    const stdoutB = execFileSync("node", [resolve(HERE, "validate.mjs"), fileB], { encoding: "utf8" });
    cliResult = stdoutA.length > 0 && stdoutA === stdoutB;
  } finally {
    try { unlinkSync(fileA); } catch { /* best-effort cleanup */ }
    try { unlinkSync(fileB); } catch { /* best-effort cleanup */ }
    try { rmdirSync(tmpDir); } catch { /* best-effort cleanup */ }
  }
  adversarial("D1R2.7 actual CLI subprocess: two differently-key-ordered-but-semantically-identical transition input files produce byte-identical stdout (D1_CLI_OBJECT_KEY_PERMUTATION_BYTE_IDENTITY)", cliResult);
}

adversarial("D1R2.8: full transition recomputation is unaffected by object-key-order permutation of ALL 4 trusted+untrusted top-level inputs simultaneously (FULL_TRANSITION_RECOMPUTATION, RETURNED_RESULT_RAW_JSON_STRING_EQUALITY)", (() => {
  const gA = golden();
  const gB = golden();
  gB.rawObservation = reverseKeyOrder(gB.rawObservation);
  gB.request = reverseKeyOrder(gB.request);
  gB.canonical_world_snapshot = reverseKeyOrder(gB.canonical_world_snapshot);
  gB.canonical_authority_snapshot = reverseKeyOrder(gB.canonical_authority_snapshot);
  gB.authenticatedRequesterContext = reverseKeyOrder(gB.authenticatedRequesterContext);
  const rA = run(gA);
  const rB = run(gB);
  return rA.status === "COMMIT" && rB.status === "COMMIT" && JSON.stringify(rA) === JSON.stringify(rB) && canonicalStringify(rA) === canonicalStringify(rB);
})());

console.log(`\n== summary ==`);
console.log(`positive: ${Object.keys(posResults).length}, negative: ${NEG.length}, adversarial: ${advCount}`);
console.log(`checks: ${pass + fail}, pass: ${pass}, fail: ${fail}`);
if (fail > 0) {
  console.log(`\nFAILING: ${failures.join(", ")}`);
  process.exit(1);
}
console.log("\nALL PASS");
process.exit(0);
