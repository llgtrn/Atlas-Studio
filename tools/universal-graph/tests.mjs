#!/usr/bin/env node
// Test runner for the Universal Graph Admission Kernel (D.CLEAN-0R4).
//
// Runs, in order:
//   - the positive fixture matrix (docs/.../fixtures/positive.json)
//   - the negative fixture matrix (docs/.../fixtures/negative.json), each
//     dispatched by its declared `mode`: reject / determinism-cross-order /
//     accept / ajv-bypass-proof
//   - determinism / collection-order-independence / mutation-propagation
//     checks (D.CLEAN-0R3)
//   - JSON-object-key-order-independence checks, at both the returned
//     in-memory fragment and the actual CLI output boundary (D.CLEAN-0R4)
//   - >=46 ADVERSARIAL cases (section 10 of the D.CLEAN-0R4 brief, plus the
//     full D.CLEAN-0R3 set it extends)
//
// D.CLEAN-0R2 removed the "no-bypass" fixture mode entirely: raw envelope
// input (admission_state, canonical, admitted, schema_validated,
// content_hash injected at the top level; a malformed or missing
// collection) is no longer accepted-and-ignored -- it is rejected by
// stage-zero Ajv raw-envelope validation before normalization runs at all.
// Every such case is now `mode: "reject"`.
//
// D.CLEAN-0R4 closes the last determinism gap: JSON object PROPERTY order
// is semantically irrelevant, so two semantically-identical raw
// observations whose object keys were inserted in different order (at any
// nesting level) must produce byte-identical output, both from
// normalizeWorldFragment() itself and from the actual validate.mjs CLI
// subprocess -- proven below by really invoking the CLI, not by asserting
// a static `true`.
//
// Adversarial cases about layering (System Atlas / product-runtime independence) are executed for
// real here via a static import/content scan of D0's own reference scripts (D.RUNTIME-0R2: a
// durable, branch-context-independent invariant, not a base-branch diff) -- not left as static
// placeholders.
//
// Usage: node tools/universal-graph/tests.mjs

import { readFileSync, writeFileSync, unlinkSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { tmpdir } from "node:os";
import Ajv2020 from "ajv/dist/2020.js";
import { compileSchemas } from "./schema-validate.mjs";
import { normalizeWorldFragment, canonicalStringify } from "./normalize.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..");
const V0_DIR = resolve(HERE, "..", "..", "docs", "_machine", "universal-graph", "v0");
const SCHEMA_DIR = resolve(V0_DIR, "schemas");
const FIXTURES_DIR = resolve(V0_DIR, "fixtures");

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

function errorsInclude(errors, substring) {
  return errors.some((e) => `${e.path} ${e.message}`.includes(substring));
}

const ajv = compileSchemas(SCHEMA_DIR);
const positive = JSON.parse(readFileSync(resolve(FIXTURES_DIR, "positive.json"), "utf8"));
const negative = JSON.parse(readFileSync(resolve(FIXTURES_DIR, "negative.json"), "utf8"));

console.log(`\n== SCHEMA_META_VALIDATION (Draft 2020-12 compile) ==`);
check("all committed schemas registered and compiled under Ajv2020 without throwing", true, "compileSchemas() above already threw if not");

console.log(`\n== POSITIVE fixture matrix (${positive.length} cases) ==`);
const positiveResults = {};
for (const fx of positive) {
  const result = normalizeWorldFragment(fx.input, ajv);
  positiveResults[fx.case] = result;
  check(`POS-${fx.case} ${fx.name}: normalizes and validates`, result.valid === true, result.errors && JSON.stringify(result.errors));
  if (result.valid) {
    check(`POS-${fx.case} ${fx.name}: admission_state is CANDIDATE_ONLY`, result.fragment.admission_state === "CANDIDATE_ONLY");
  }
}

console.log(`\n== NEGATIVE fixture matrix (${negative.length} cases) ==`);
const negativeResults = {};
for (const fx of negative) {
  if (fx.mode === "reject") {
    const result = normalizeWorldFragment(fx.input, ajv);
    negativeResults[fx.case] = result;
    check(`NEG-${fx.case} ${fx.name}: rejected`, result.valid === false, JSON.stringify(result.errors));
    if (result.valid === false) {
      check(`NEG-${fx.case} ${fx.name}: error names the right field/code`, errorsInclude(result.errors, fx.expectedErrorPathIncludes), JSON.stringify(result.errors));
    }
  } else if (fx.mode === "determinism-cross-order") {
    const a = normalizeWorldFragment(fx.input_a, ajv);
    const b = normalizeWorldFragment(fx.input_b, ajv);
    negativeResults[fx.case] = { a, b };
    check(`NEG-${fx.case} ${fx.name}: both orderings normalize`, a.valid && b.valid, JSON.stringify([a.errors, b.errors]));
    check(`NEG-${fx.case} ${fx.name}: reordering + duplication produce byte-identical canonical output`, a.valid && b.valid && canonicalStringify(a.fragment) === canonicalStringify(b.fragment));
  } else if (fx.mode === "accept") {
    const result = normalizeWorldFragment(fx.input, ajv);
    negativeResults[fx.case] = result;
    check(`NEG-${fx.case} ${fx.name}: unfamiliar provider is admitted, not rejected`, result.valid === true, JSON.stringify(result.errors));
  } else if (fx.mode === "ajv-bypass-proof") {
    const result = normalizeWorldFragment(fx.input, ajv);
    negativeResults[fx.case] = result;
    check(`NEG-${fx.case} ${fx.name}: real Ajv rejects the duplicate evidence_ref_ids (uniqueItems)`, result.valid === false && errorsInclude(result.errors, "evidence_ref_ids"), JSON.stringify(result.errors));
    check(
      `NEG-${fx.case} ${fx.name}: a reconstruction of the retired hand-written engine's keyword coverage (no uniqueItems) would have wrongly accepted the same record`,
      oldSubsetWouldAccept(fx.input.identity_links[0]) === true,
    );
  } else {
    check(`NEG-${fx.case} ${fx.name}: has a recognized mode`, false, `unknown mode "${fx.mode}"`);
  }
}

/**
 * A reconstruction of exactly the retired D.CLEAN-0 hand-written engine's
 * keyword coverage (type, required, properties, pattern, minLength, array
 * items type) -- deliberately WITHOUT uniqueItems, which that engine never
 * implemented. Used only by the NEG-30 ajv-bypass-proof case above, to
 * prove concretely that substituting it back in place of real Ajv would
 * let a real violation through. Not used anywhere else in this kernel.
 */
function oldSubsetWouldAccept(link) {
  if (typeof link !== "object" || link === null) return false;
  const required = ["provider_type", "external_id", "external_namespace", "entity_id", "provenance"];
  for (const k of required) if (!(k in link)) return false;
  if (typeof link.provider_type !== "string" || !/^[a-z][a-z0-9]*(\.[a-z0-9_-]+)+$/.test(link.provider_type)) return false;
  if (typeof link.external_id !== "string" || link.external_id.length < 1) return false;
  if (typeof link.external_namespace !== "string" || link.external_namespace.length < 1) return false;
  if (typeof link.entity_id !== "string") return false;
  if (link.evidence_ref_ids !== undefined) {
    if (!Array.isArray(link.evidence_ref_ids)) return false;
    for (const id of link.evidence_ref_ids) if (typeof id !== "string" || id.length < 1) return false;
    // No uniqueItems check here -- this is exactly the gap being proven.
  }
  return true;
}

console.log(`\n== Determinism / recomputation checks ==`);
{
  const fx = positive.find((f) => f.case === 4);
  const run1 = normalizeWorldFragment(fx.input, ajv);
  const run2 = normalizeWorldFragment(fx.input, ajv);
  check("same full input run twice -> identical full normalized output", canonicalStringify(run1.fragment) === canonicalStringify(run2.fragment));
  check("candidate entity_id is stable across recomputation", run1.fragment.entities[0].entity_id === run2.fragment.entities[0].entity_id);
}
{
  const base = positive.find((f) => f.case === 5);
  const mutated = JSON.parse(JSON.stringify(base.input));
  mutated.semantic_attachments[0].semantic_claims[0].confidence = 0.91;

  const baseResult = normalizeWorldFragment(base.input, ajv);
  const mutatedResult = normalizeWorldFragment(mutated, ajv);
  const baseResult2 = normalizeWorldFragment(base.input, ajv);

  check("source semantic mutation -> content_hash changes", baseResult.valid && mutatedResult.valid && baseResult.fragment.content_hash !== mutatedResult.fragment.content_hash);
  check("unmutated source reproduces its own hash exactly on repeat", baseResult.fragment.content_hash === baseResult2.fragment.content_hash);
}
{
  const e = positive.find((f) => f.case === 1).input.entities[0];
  const raw = { entities: [e, JSON.parse(JSON.stringify(e))], identity_links: [], evidence_refs: [], semantic_attachments: [] };
  const r = normalizeWorldFragment(raw, ajv);
  check("exact duplicate entities collapse to one, not silently multiplied", r.valid && r.fragment.entities.length === 1);
}

console.log(`\n== Total-order / nested-collection order-independence checks (D.CLEAN-0R3) ==`);

const PROV = { source: "chronica.system-atlas", observed_at: "2026-08-20T12:00:00Z" };
const entityFx = (id, type) => ({ entity_id: id, entity_type: type, display_name: id, provenance: { ...PROV } });
const evidenceFx = (id) => ({ evidence_ref_id: id, evidence_kind: "chronica.event", locator: { scheme: "chronica.event_id", value: `evt_${id}` }, provenance: { ...PROV } });

let equalKeyResult;
{
  // Section 6: two distinct IdentityLinks sharing entity_id/provider_type/
  // external_namespace/external_id (the primary sort key) but differing in
  // evidence_ref_ids and provenance.source_record_id -- neither an
  // EXTERNAL_IDENTITY_CONFLICT (same entity_id) nor an exact duplicate.
  const e = entityFx("human.person:tie-x", "human.person");
  const evA = evidenceFx("tie-a");
  const evB = evidenceFx("tie-b");
  const linkA = { provider_type: "chronica.repo", external_namespace: "ns", external_id: "tie", entity_id: e.entity_id, evidence_ref_ids: ["tie-a"], provenance: { ...PROV, source_record_id: "observation-a" } };
  const linkB = { provider_type: "chronica.repo", external_namespace: "ns", external_id: "tie", entity_id: e.entity_id, evidence_ref_ids: ["tie-b"], provenance: { ...PROV, source_record_id: "observation-b" } };
  const rawA = { entities: [e], identity_links: [linkA, linkB], evidence_refs: [evA, evB], semantic_attachments: [] };
  const rawB = { entities: [e], identity_links: [linkB, linkA], evidence_refs: [evA, evB], semantic_attachments: [] };
  const resultA = normalizeWorldFragment(rawA, ajv);
  const resultB = normalizeWorldFragment(rawB, ajv);
  equalKeyResult = { resultA, resultB };
  check("EQUAL_PRIMARY_SORT_KEY_ORDER_INDEPENDENCE: both equal-primary-key IdentityLinks survive, in either source order", resultA.valid && resultB.valid && resultA.fragment.identity_links.length === 2);
  check("EQUAL_PRIMARY_SORT_KEY_ORDER_INDEPENDENCE: full normalized output is byte-identical regardless of source order", resultA.valid && resultB.valid && canonicalStringify(resultA.fragment) === canonicalStringify(resultB.fragment));
  check("EQUAL_PRIMARY_SORT_KEY_ORDER_INDEPENDENCE: content_hash is identical regardless of source order", resultA.valid && resultB.valid && resultA.fragment.content_hash === resultB.fragment.content_hash);
}

let identityLinkEvidenceOrderResult;
{
  const e = entityFx("human.person:ev-order-x", "human.person");
  const ev1 = evidenceFx("order-1");
  const ev2 = evidenceFx("order-2");
  const linkForward = { provider_type: "chronica.repo", external_namespace: "ns", external_id: "ev-order", entity_id: e.entity_id, evidence_ref_ids: ["order-1", "order-2"], provenance: { ...PROV } };
  const linkReversed = { ...linkForward, evidence_ref_ids: ["order-2", "order-1"] };
  const resultForward = normalizeWorldFragment({ entities: [e], identity_links: [linkForward], evidence_refs: [ev1, ev2], semantic_attachments: [] }, ajv);
  const resultReversed = normalizeWorldFragment({ entities: [e], identity_links: [linkReversed], evidence_refs: [ev1, ev2], semantic_attachments: [] }, ajv);
  identityLinkEvidenceOrderResult = { resultForward, resultReversed };
  check("IDENTITY_LINK_EVIDENCE_REF_ORDER_INDEPENDENCE: reversed evidence_ref_ids normalizes byte-identically", resultForward.valid && resultReversed.valid && canonicalStringify(resultForward.fragment) === canonicalStringify(resultReversed.fragment));
}

let semanticAttachmentEvidenceOrderResult;
{
  const e = entityFx("human.person:att-ev-order-x", "human.person");
  const ev1 = evidenceFx("att-order-1");
  const ev2 = evidenceFx("att-order-2");
  const attForward = { attachment_id: "att-ev-order", entity_id: e.entity_id, effect_class: "ADVISORY_ONLY", provider_type: "chronica.system-atlas", semantic_claims: [{ claim_type: "provider.classification", value: "x" }], evidence_ref_ids: ["att-order-1", "att-order-2"], provenance: { ...PROV } };
  const attReversed = { ...attForward, evidence_ref_ids: ["att-order-2", "att-order-1"] };
  const resultForward = normalizeWorldFragment({ entities: [e], identity_links: [], evidence_refs: [ev1, ev2], semantic_attachments: [attForward] }, ajv);
  const resultReversed = normalizeWorldFragment({ entities: [e], identity_links: [], evidence_refs: [ev1, ev2], semantic_attachments: [attReversed] }, ajv);
  semanticAttachmentEvidenceOrderResult = { resultForward, resultReversed };
  check("SEMANTIC_ATTACHMENT_EVIDENCE_REF_ORDER_INDEPENDENCE: reversed evidence_ref_ids normalizes byte-identically", resultForward.valid && resultReversed.valid && canonicalStringify(resultForward.fragment) === canonicalStringify(resultReversed.fragment));
}

let semanticClaimOrderResult;
let orderOnlyConflictResult;
{
  const e = entityFx("human.person:claim-order-x", "human.person");
  const claimClassification = { claim_type: "provider.classification", value: "robot" };
  const claimRisk = { claim_type: "provider.risk_signal", value: "low" };
  const attForward = { attachment_id: "att-claim-order", entity_id: e.entity_id, effect_class: "ADVISORY_ONLY", provider_type: "chronica.system-atlas", semantic_claims: [claimClassification, claimRisk], provenance: { ...PROV } };
  const attReversed = { ...attForward, semantic_claims: [claimRisk, claimClassification] };
  const resultForward = normalizeWorldFragment({ entities: [e], identity_links: [], evidence_refs: [], semantic_attachments: [attForward] }, ajv);
  const resultReversed = normalizeWorldFragment({ entities: [e], identity_links: [], evidence_refs: [], semantic_attachments: [attReversed] }, ajv);
  semanticClaimOrderResult = { resultForward, resultReversed };
  check("SEMANTIC_CLAIM_ORDER_INDEPENDENCE: reversed semantic_claims normalizes byte-identically, including content_hash", resultForward.valid && resultReversed.valid && canonicalStringify(resultForward.fragment) === canonicalStringify(resultReversed.fragment) && resultForward.fragment.content_hash === resultReversed.fragment.content_hash);

  // Section 9: the SAME attachment_id supplied twice, differing ONLY in
  // declared-order-insensitive semantic_claims order, must NOT be treated
  // as an ATTACHMENT_ID_CONFLICT -- canonicalization must run before
  // dedupeByIdOrConflict() sees them.
  orderOnlyConflictResult = normalizeWorldFragment({ entities: [e], identity_links: [], evidence_refs: [], semantic_attachments: [attForward, attReversed] }, ajv);
  check("ORDER_ONLY_FALSE_CONFLICT_COUNT: same attachment_id, order-only-different semantic_claims -- collapses, no ATTACHMENT_ID_CONFLICT", orderOnlyConflictResult.valid === true && orderOnlyConflictResult.fragment.semantic_attachments.length === 1);
}

let valueArrayOrderResult;
{
  const e = entityFx("human.person:value-order-x", "human.person");
  const att = { attachment_id: "att-value-order", entity_id: e.entity_id, effect_class: "ADVISORY_ONLY", provider_type: "chronica.system-atlas", semantic_claims: [{ claim_type: "provider.classification", value: ["zebra", "apple", "mango"] }], provenance: { ...PROV } };
  valueArrayOrderResult = normalizeWorldFragment({ entities: [e], identity_links: [], evidence_refs: [], semantic_attachments: [att] }, ajv);
  check("SEMANTIC_CLAIM_VALUE_ARRAY_ORDER_PRESERVED: a claim's own value array is never reordered", valueArrayOrderResult.valid && JSON.stringify(valueArrayOrderResult.fragment.semantic_attachments[0].semantic_claims[0].value) === JSON.stringify(["zebra", "apple", "mango"]));
}

let permutationResults;
{
  // Section 12: one fragment big enough to exercise multiple entities,
  // multiple identity links, evidence refs, multiple semantic attachments,
  // and multiple semantic claims -- permuted at both the top level and
  // within the declared order-insensitive nested arrays, using fixed
  // reproducible permutations (no randomness).
  const e1 = entityFx("vehicle.autonomous:perm-1", "vehicle.autonomous");
  const e2 = entityFx("vehicle.autonomous:perm-2", "vehicle.autonomous");
  const ev1 = evidenceFx("perm-1");
  const ev2 = evidenceFx("perm-2");
  const claimA = { claim_type: "provider.classification", value: "alpha" };
  const claimB = { claim_type: "provider.risk_signal", value: "low" };
  const claimC = { claim_type: "provider.machine_interpretation", value: "nominal" };

  const link1 = (evOrder) => ({ provider_type: "chronica.repo", external_namespace: "ns", external_id: "perm-1", entity_id: e1.entity_id, evidence_ref_ids: evOrder, provenance: { ...PROV } });
  const link2 = (evOrder) => ({ provider_type: "chronica.repo", external_namespace: "ns", external_id: "perm-2", entity_id: e2.entity_id, evidence_ref_ids: evOrder, provenance: { ...PROV } });
  const att1 = (claimOrder, evOrder) => ({ attachment_id: "att-perm-1", entity_id: e1.entity_id, effect_class: "ADVISORY_ONLY", provider_type: "chronica.system-atlas", semantic_claims: claimOrder, evidence_ref_ids: evOrder, provenance: { ...PROV } });
  const att2 = { attachment_id: "att-perm-2", entity_id: e2.entity_id, effect_class: "ADVISORY_ONLY", provider_type: "chronica.system-atlas", semantic_claims: [{ claim_type: "provider.classification", value: "beta" }], evidence_ref_ids: ["perm-1"], provenance: { ...PROV } };

  const permutations = [
    { // V1: baseline order throughout
      entities: [e1, e2],
      identity_links: [link1(["perm-1", "perm-2"]), link2(["perm-2", "perm-1"])],
      evidence_refs: [ev1, ev2],
      semantic_attachments: [att1([claimA, claimB, claimC], ["perm-1", "perm-2"]), att2],
    },
    { // V2: fully reversed top-level collections AND reversed nested arrays
      entities: [e2, e1],
      identity_links: [link2(["perm-1", "perm-2"]), link1(["perm-2", "perm-1"])],
      evidence_refs: [ev2, ev1],
      semantic_attachments: [att2, att1([claimC, claimB, claimA], ["perm-2", "perm-1"])],
    },
    { // V3: mixed -- only entities and one nested array reordered
      entities: [e2, e1],
      identity_links: [link1(["perm-1", "perm-2"]), link2(["perm-2", "perm-1"])],
      evidence_refs: [ev1, ev2],
      semantic_attachments: [att1([claimB, claimC, claimA], ["perm-2", "perm-1"]), att2],
    },
  ];

  permutationResults = permutations.map((p) => normalizeWorldFragment(p, ajv));
  const allValid = permutationResults.every((r) => r.valid);
  const allSameCanonical = allValid && permutationResults.every((r) => canonicalStringify(r.fragment) === canonicalStringify(permutationResults[0].fragment));
  const allSameHash = allValid && permutationResults.every((r) => r.fragment.content_hash === permutationResults[0].fragment.content_hash);
  check("MULTI_COLLECTION_PERMUTATION_DETERMINISM: all 3 fixed permutations normalize", allValid, JSON.stringify(permutationResults.map((r) => r.errors)));
  check("MULTI_COLLECTION_PERMUTATION_DETERMINISM: all 3 fixed permutations produce byte-identical canonical output", allSameCanonical);
  check("MULTI_COLLECTION_PERMUTATION_DETERMINISM: all 3 fixed permutations produce identical content_hash", allSameHash);
}

console.log(`\n== JSON object-key-order-independence checks (D.CLEAN-0R4) ==`);

let objectKeyOrderResult;
let objectKeyOrderRawA;
let objectKeyOrderRawB;
{
  // Two semantically identical raw observations, deliberately built with
  // reversed property insertion order at every nesting level: WorldEntity,
  // Provenance, IdentityLink, EvidenceRef (and its locator), SemanticAttachment,
  // and SemanticClaim. Top-level array order is kept identical between A and
  // B (single-element arrays here) so this test isolates object-KEY order
  // specifically, distinct from the D.CLEAN-0R3 array-order tests above.
  const PROVENANCE_FWD = { source: "chronica.system-atlas", observed_at: "2026-08-20T12:00:00Z" };
  const PROVENANCE_REV = { observed_at: "2026-08-20T12:00:00Z", source: "chronica.system-atlas" };

  const entityA = { entity_id: "human.person:objkey-x", entity_type: "human.person", display_name: "ObjKey X", provenance: PROVENANCE_FWD };
  const entityB = { provenance: PROVENANCE_REV, display_name: "ObjKey X", entity_type: "human.person", entity_id: "human.person:objkey-x" };

  const linkA = { provider_type: "chronica.repo", external_id: "objkey", external_namespace: "ns", entity_id: "human.person:objkey-x", evidence_ref_ids: ["objkey-ev"], provenance: PROVENANCE_FWD };
  const linkB = { provenance: PROVENANCE_REV, evidence_ref_ids: ["objkey-ev"], entity_id: "human.person:objkey-x", external_namespace: "ns", external_id: "objkey", provider_type: "chronica.repo" };

  const evA = { evidence_ref_id: "objkey-ev", evidence_kind: "chronica.event", locator: { scheme: "chronica.event_id", value: "evt_objkey" }, provenance: PROVENANCE_FWD };
  const evB = { provenance: PROVENANCE_REV, locator: { value: "evt_objkey", scheme: "chronica.event_id" }, evidence_kind: "chronica.event", evidence_ref_id: "objkey-ev" };

  const claimA = { claim_type: "provider.classification", value: "robot", confidence: 0.5 };
  const claimB = { confidence: 0.5, value: "robot", claim_type: "provider.classification" };
  const attA = { attachment_id: "att-objkey", entity_id: "human.person:objkey-x", effect_class: "ADVISORY_ONLY", provider_type: "chronica.system-atlas", semantic_claims: [claimA], evidence_ref_ids: ["objkey-ev"], provenance: PROVENANCE_FWD };
  const attB = { provenance: PROVENANCE_REV, evidence_ref_ids: ["objkey-ev"], semantic_claims: [claimB], provider_type: "chronica.system-atlas", effect_class: "ADVISORY_ONLY", entity_id: "human.person:objkey-x", attachment_id: "att-objkey" };

  objectKeyOrderRawA = { entities: [entityA], identity_links: [linkA], evidence_refs: [evA], semantic_attachments: [attA] };
  objectKeyOrderRawB = { entities: [entityB], identity_links: [linkB], evidence_refs: [evB], semantic_attachments: [attB] };

  const resultA = normalizeWorldFragment(objectKeyOrderRawA, ajv);
  const resultB = normalizeWorldFragment(objectKeyOrderRawB, ajv);
  objectKeyOrderResult = { resultA, resultB };

  check("OBJECT_KEY_PERMUTATION_RETURNED_FRAGMENT_BYTES: both inputs normalize", resultA.valid === true && resultB.valid === true, JSON.stringify([resultA.errors, resultB.errors]));
  check(
    "OBJECT_KEY_PERMUTATION_RETURNED_FRAGMENT_BYTES: raw JSON.stringify() of the returned fragment is byte-identical despite reversed input key order at every level",
    resultA.valid && resultB.valid && JSON.stringify(resultA.fragment) === JSON.stringify(resultB.fragment),
  );
  check(
    "OBJECT_KEY_PERMUTATION_RETURNED_FRAGMENT_BYTES: canonicalStringify() is byte-identical",
    resultA.valid && resultB.valid && canonicalStringify(resultA.fragment) === canonicalStringify(resultB.fragment),
  );
  check(
    "OBJECT_KEY_REORDER_CONTENT_HASH_INDEPENDENCE: content_hash is identical despite reversed input key order",
    resultA.valid && resultB.valid && resultA.fragment.content_hash === resultB.fragment.content_hash,
  );
}

let cliByteTestResult;
{
  // Exercises the ACTUAL validate.mjs CLI as a real subprocess -- not just
  // the in-process normalizeWorldFragment() return value -- proving the
  // public serialization boundary itself is byte-deterministic under
  // object-key reordering.
  const fileA = join(tmpdir(), `d0r4-objkey-a-${process.pid}.json`);
  const fileB = join(tmpdir(), `d0r4-objkey-b-${process.pid}.json`);
  const validateCliPath = resolve(HERE, "validate.mjs");
  try {
    writeFileSync(fileA, JSON.stringify(objectKeyOrderRawA));
    writeFileSync(fileB, JSON.stringify(objectKeyOrderRawB));
    const stdoutA = execFileSync("node", [validateCliPath, fileA], { cwd: REPO_ROOT, encoding: "utf8" });
    const stdoutB = execFileSync("node", [validateCliPath, fileB], { cwd: REPO_ROOT, encoding: "utf8" });
    cliByteTestResult = stdoutA === stdoutB;
    check("CLI_OBJECT_KEY_PERMUTATION_BYTE_IDENTITY: actual `node validate.mjs` subprocess stdout is byte-identical for differently-key-ordered semantically-identical inputs", cliByteTestResult);
  } finally {
    try { unlinkSync(fileA); } catch { /* already absent */ }
    try { unlinkSync(fileB); } catch { /* already absent */ }
  }
}

console.log(`\n== ADVERSARIAL pass (D.CLEAN-0R4 section 10, >=46 required) ==`);
let advCount = 0;
function adversarial(n, label, cond, detail) {
  advCount++;
  check(`ADV-${n} ${label}`, cond, detail);
}

adversarial(1, "no hand-written schema keyword walker remains; schema-validate.mjs uses the real Ajv2020 engine", (() => {
  const src = readFileSync(resolve(HERE, "schema-validate.mjs"), "utf8");
  return /ajv\/dist\/2020(\.js)?/.test(src) && !/function\s+validateNode/.test(src) && !/function\s+matchesType/.test(src);
})());

adversarial(2, "an unsupported/misspelled schema keyword is NOT silently ignored -- Ajv strict mode throws at compile time", (() => {
  try {
    new Ajv2020({ strict: true }).compile({ type: "object", properties: { x: { type: "string", patern: "typo-keyword-must-not-be-silently-accepted" } } });
    return false;
  } catch {
    return true;
  }
})());

adversarial(3, "provider normalization never claims canonical truth -- admission_state is always CANDIDATE_ONLY", positiveResults[1].valid && positiveResults[1].fragment.admission_state === "CANDIDATE_ONLY");

adversarial(4, "provider supplies admitted_at -- rejected", negativeResults[21].valid === false);

adversarial(5, "provider supplies admitted_by -- rejected", negativeResults[22].valid === false);

adversarial(6, "fragment admission_state cannot be widened to CANONICAL -- schema is const-locked AND raw injection is rejected outright, not silently normalized away", (() => {
  const fragSchema = JSON.parse(readFileSync(resolve(SCHEMA_DIR, "normalized-world-fragment.schema.json"), "utf8"));
  const schemaLocksSingleValue = fragSchema.properties.admission_state.const === "CANDIDATE_ONLY" && fragSchema.properties.admission_state.enum === undefined;
  return schemaLocksSingleValue && negativeResults[7].valid === false;
})());

adversarial(7, "orphan identity-link target -- rejected", negativeResults[11].valid === false && errorsInclude(negativeResults[11].errors, "IDENTITY_LINK_TARGET_ORPHAN"));
adversarial(8, "orphan semantic-attachment target -- rejected", negativeResults[12].valid === false && errorsInclude(negativeResults[12].errors, "SEMANTIC_ATTACHMENT_TARGET_ORPHAN"));
adversarial(9, "orphan evidence ref -- rejected", negativeResults[13].valid === false && errorsInclude(negativeResults[13].errors, "EVIDENCE_REF_ID_ORPHAN"));
adversarial(10, "duplicate entity ID with conflicting content -- rejected, not silently picked", negativeResults[14].valid === false && errorsInclude(negativeResults[14].errors, "ENTITY_ID_CONFLICT"));
adversarial(11, "duplicate evidence ID with conflicting content -- rejected, not silently picked", negativeResults[15].valid === false && errorsInclude(negativeResults[15].errors, "EVIDENCE_REF_ID_CONFLICT"));
adversarial(12, "duplicate attachment ID with conflicting content -- rejected, not silently picked", negativeResults[16].valid === false && errorsInclude(negativeResults[16].errors, "ATTACHMENT_ID_CONFLICT"));
adversarial(13, "same provider external identity resolving to two entities -- rejected", negativeResults[17].valid === false && errorsInclude(negativeResults[17].errors, "EXTERNAL_IDENTITY_CONFLICT"));
adversarial(14, "WorldEntity embedding a second identity-link representation -- rejected", negativeResults[18].valid === false);
adversarial(15, "WorldEntity embedding a second evidence representation -- rejected", negativeResults[19].valid === false);
adversarial(16, "WorldEntity embedding a second semantic-attachment representation -- rejected", negativeResults[20].valid === false);
adversarial(17, "authority.* semantic claim -- rejected", negativeResults[25].valid === false);
adversarial(18, "execution.* semantic claim -- rejected", negativeResults[26].valid === false);
adversarial(19, "control.* semantic claim -- rejected", negativeResults[27].valid === false);
adversarial(20, "approval.* semantic claim -- rejected", negativeResults[28].valid === false);
adversarial(21, "act.* semantic claim -- rejected", negativeResults[29].valid === false);

adversarial(22, "arbitrary nested object used to smuggle a command via semantic_claims[].value -- rejected (value is bounded scalar/scalar-list only)", (() => {
  const attempt = JSON.parse(JSON.stringify(positive.find((f) => f.case === 7).input));
  attempt.semantic_attachments[0].semantic_claims[0].value = { command: "START" };
  const result = normalizeWorldFragment(attempt, ajv);
  return result.valid === false && errorsInclude(result.errors, "value");
})());

adversarial(23, "unknown future provider is NOT incorrectly rejected", negativeResults[10].valid === true);
adversarial(24, "non-filesystem evidence is NOT incorrectly rejected", positiveResults[2].valid === true && positiveResults[3].valid === true && positiveResults[6].valid === true);
adversarial(25, "source reordering does not change normalized output", negativeResults[9].b.valid && canonicalStringify(negativeResults[9].a.fragment) === canonicalStringify(negativeResults[9].b.fragment));
adversarial(26, "exact duplicate input does not change/multiply normalized output", (() => {
  const e = positive.find((f) => f.case === 1).input.entities[0];
  const raw = { entities: [e, JSON.parse(JSON.stringify(e))], identity_links: [], evidence_refs: [], semantic_attachments: [] };
  const r = normalizeWorldFragment(raw, ajv);
  return r.valid && r.fragment.entities.length === 1;
})());
adversarial(27, "conflicting duplicate is fail-closed, never silently deduped by picking first/last", negativeResults[14].valid === false);
adversarial(28, "normalize.mjs and schema-validate.mjs never read the wall clock", (() => {
  const src = readFileSync(resolve(HERE, "normalize.mjs"), "utf8") + readFileSync(resolve(HERE, "schema-validate.mjs"), "utf8");
  return !/Date\.now\(|new Date\(/.test(src);
})());
// D.RUNTIME-0R2 repair: a base-branch `git diff` is not a durable permanent invariant -- it is
// blind to uncommitted state, depends entirely on which commit happens to be checked out, and (for
// "product/runtime untouched") is inherently incompatible with any later Rust runtime-materialization
// wave that legitimately touches crates/Cargo metadata. Both ADV-29 and ADV-30 below are replaced
// with a single content-based invariant that remains true on any checkout, at any point in commit
// history: the D0 reference scripts import nothing from System Atlas OR product/runtime, so no
// legitimate change to either can ever be CAUSED by this reference layer.
adversarial(29, "D0 reference scripts (normalize/schema-validate/validate/tests.mjs) import nothing from System Atlas (docs/_machine/system-atlas, scripts/system-atlas)", (() => {
  const files = ["normalize.mjs", "schema-validate.mjs", "validate.mjs", "tests.mjs"];
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
adversarial(30, "D0 reference scripts (normalize/schema-validate/validate/tests.mjs) import nothing from product/runtime (crates, ui)", (() => {
  const files = ["normalize.mjs", "schema-validate.mjs", "validate.mjs", "tests.mjs"];
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

adversarial(31, "raw admission_state=CANONICAL is rejected before normalization (stage-zero raw-envelope validation)", negativeResults[7].valid === false && errorsInclude(negativeResults[7].errors, "admission_state"));
adversarial(32, "raw canonical=true is rejected before normalization", negativeResults[31].valid === false && errorsInclude(negativeResults[31].errors, "canonical"));
adversarial(33, "raw schema_validated=true is rejected before normalization", negativeResults[33].valid === false && errorsInclude(negativeResults[33].errors, "schema_validated"));
adversarial(34, "a raw collection of the wrong type (entities: {}) is rejected, not coerced", negativeResults[35].valid === false && errorsInclude(negativeResults[35].errors, "entities"));
adversarial(35, "a missing required raw collection (entities absent) is rejected, not defaulted to []", negativeResults[37].valid === false && errorsInclude(negativeResults[37].errors, "entities"));
adversarial(36, "two external identity tuples that collide under the old space-join encoding remain distinct (no EXTERNAL_IDENTITY_CONFLICT), and both target their own entity_id", (() => {
  const r = positiveResults[11];
  return r.valid === true && r.fragment.identity_links.length === 2 && new Set(r.fragment.identity_links.map((l) => l.entity_id)).size === 2;
})());
adversarial(37, "a missing required raw collection (semantic_attachments absent) is rejected, not defaulted to []", negativeResults[38].valid === false && errorsInclude(negativeResults[38].errors, "semantic_attachments"));
adversarial(38, "a malformed collection is never silently converted to an empty array -- rejection produces fragment: null, not a fragment with entities: []", negativeResults[35].fragment === null);

adversarial(39, "equal-primary-key IdentityLinks reversed -- both survive, byte-identical output regardless of source order", equalKeyResult.resultA.valid && equalKeyResult.resultB.valid && canonicalStringify(equalKeyResult.resultA.fragment) === canonicalStringify(equalKeyResult.resultB.fragment));
adversarial(40, "IdentityLink.evidence_ref_ids reversed -- byte-identical normalized output", identityLinkEvidenceOrderResult.resultForward.valid && canonicalStringify(identityLinkEvidenceOrderResult.resultForward.fragment) === canonicalStringify(identityLinkEvidenceOrderResult.resultReversed.fragment));
adversarial(41, "SemanticAttachment.evidence_ref_ids reversed -- byte-identical normalized output", semanticAttachmentEvidenceOrderResult.resultForward.valid && canonicalStringify(semanticAttachmentEvidenceOrderResult.resultForward.fragment) === canonicalStringify(semanticAttachmentEvidenceOrderResult.resultReversed.fragment));
adversarial(42, "semantic_claims reversed -- byte-identical normalized output including content_hash", semanticClaimOrderResult.resultForward.valid && semanticClaimOrderResult.resultForward.fragment.content_hash === semanticClaimOrderResult.resultReversed.fragment.content_hash);
adversarial(43, "an order-only attachment variant does NOT falsely trigger ATTACHMENT_ID_CONFLICT -- canonicalization runs before conflict detection", orderOnlyConflictResult.valid === true && orderOnlyConflictResult.fragment.semantic_attachments.length === 1);
adversarial(44, "deterministic multi-collection permutation test: 3 fixed permutations of a multi-entity/link/evidence/attachment/claim fragment all normalize to the same canonical output and content_hash", permutationResults.every((r) => r.valid) && permutationResults.every((r) => r.fragment.content_hash === permutationResults[0].fragment.content_hash));

adversarial(45, "semantically identical WorldEntity with reversed property insertion order -- returned fragment JSON.stringify() byte-identical", objectKeyOrderResult.resultA.valid && objectKeyOrderResult.resultB.valid && JSON.stringify(objectKeyOrderResult.resultA.fragment.entities) === JSON.stringify(objectKeyOrderResult.resultB.fragment.entities));
adversarial(46, "nested Provenance property insertion order variation (source/observed_at reversed, at entity/link/evidence/attachment level) -- byte-identical output", JSON.stringify(objectKeyOrderResult.resultA.fragment) === JSON.stringify(objectKeyOrderResult.resultB.fragment));
adversarial(47, "SemanticClaim property insertion order variation (claim_type/value/confidence reversed) -- byte-identical output including the claim itself", JSON.stringify(objectKeyOrderResult.resultA.fragment.semantic_attachments[0].semantic_claims) === JSON.stringify(objectKeyOrderResult.resultB.fragment.semantic_attachments[0].semantic_claims));
adversarial(48, "actual CLI byte-output comparison for differently-key-ordered semantically-identical raw JSON files -- real subprocess invocation, not asserted", cliByteTestResult === true);

console.log(`\n== summary ==`);
console.log(`positive: ${positive.length}, negative: ${negative.length}, adversarial: ${advCount}`);
console.log(`checks: ${pass + fail}, pass: ${pass}, fail: ${fail}`);
if (fail > 0) {
  console.log(`\nFAILING: ${failures.join(", ")}`);
  process.exit(1);
}
console.log("\nALL PASS");
process.exit(0);
