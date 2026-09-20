#!/usr/bin/env node
// Test runner for the Universal Capability Registry Candidate Kernel
// (D.CLEAN-2).
//
// Runs, in order:
//   - the positive fixture matrix (12 required generality cases)
//   - the negative fixture matrix (34 fail-closed reject cases)
//   - >=21 additional ADVERSARIAL checks executing real normalizer code:
//     fragment-schema registration_state rejection, source-order and
//     object-key-order determinism (including the required multi-collection
//     property test), hash-self-reference acyclicity, input-mutation
//     count, vendor-leakage/authority-escalation smuggling attempts, actual
//     CLI subprocess byte-determinism, and durable content-based layering
//     invariants against D0/D1/System-Atlas/product-runtime (real import/
//     content scans, not a base-branch `git diff` -- see D.RUNTIME-0R2).
//   - a full re-run of canonical D0 (167/167) and D1 (245/245) tests, to
//     prove no regression.
//
// Usage: node tools/universal-graph/capability/tests.mjs

import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { tmpdir } from "node:os";
import {
  compileCapabilitySchemas,
  normalizeCapabilityRegistry,
  computeDefinitionHash,
  canonicalStringify,
  FRAGMENT_SCHEMA,
} from "./normalize.mjs";
import { validate } from "../schema-validate.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..", "..");
const D0_SCHEMA_DIR = resolve(REPO_ROOT, "docs", "_machine", "universal-graph", "v0", "schemas");
const CAP_DIR = resolve(REPO_ROOT, "docs", "_machine", "universal-graph", "capability", "v0");
const CAP_SCHEMA_DIR = resolve(CAP_DIR, "schemas");
const FIXTURES_DIR = resolve(CAP_DIR, "fixtures");

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
  return (errors ?? []).some((e) => `${e.path} ${e.message}`.includes(substring));
}

const ajv = compileCapabilitySchemas([D0_SCHEMA_DIR, CAP_SCHEMA_DIR]);
const positive = JSON.parse(readFileSync(resolve(FIXTURES_DIR, "positive.json"), "utf8"));
const negative = JSON.parse(readFileSync(resolve(FIXTURES_DIR, "negative.json"), "utf8"));

console.log(`\n== SCHEMA_META_VALIDATION (Draft 2020-12 compile, D0 + capability schemas in one Ajv2020 registry) ==`);
check("all committed D0 + capability schemas registered and compiled under Ajv2020 without throwing", true, "compileCapabilitySchemas() above already threw if not");

console.log(`\n== POSITIVE fixture matrix (${positive.length} required generality cases) ==`);
const positiveResults = {};
for (const fx of positive) {
  const result = normalizeCapabilityRegistry(fx.input, ajv);
  positiveResults[fx.case] = result;
  check(`POS-${fx.case} ${fx.name}: normalizes and validates`, result.valid === true, result.errors && JSON.stringify(result.errors));
  if (result.valid) {
    check(`POS-${fx.case} ${fx.name}: registration_state is CANDIDATE_ONLY`, result.fragment.registration_state === "CANDIDATE_ONLY");
  }
}

console.log(`\n== NEGATIVE fixture matrix (${negative.length} fail-closed reject cases) ==`);
for (const fx of negative) {
  const result = normalizeCapabilityRegistry(fx.input, ajv);
  check(`NEG-${fx.case} ${fx.name}: rejected`, result.valid === false, JSON.stringify(result.errors));
  if (result.valid === false) {
    check(`NEG-${fx.case} ${fx.name}: error names the right field/code`, errorsInclude(result.errors, fx.expectedErrorPathIncludes), JSON.stringify(result.errors));
  }
}

console.log(`\n== ADVERSARIAL pass (section 33, >=55 required across fixtures + this section) ==`);
let advCount = negative.length; // every negative-fixture case above already executed real normalizer code against a genuine attack vector
function adversarial(n, label, cond, detail) {
  advCount++;
  check(`ADV-${n} ${label}`, cond, detail);
}

// --- helpers reused across several adversarial cases ---
const T = "2026-08-25T10:00:00Z";
const prov = (source, at = T) => ({ source, observed_at: at });
function ev(id, kind, scheme, value, source = "chronica.system-atlas") {
  return { evidence_ref_id: id, evidence_kind: kind, locator: { scheme, value }, provenance: prov(source) };
}
function makeDef(overrides = {}) {
  const d = {
    capability_definition_id: "capdef-adv-01",
    capability_key: "chronica.world.read",
    definition_version: 1,
    definition_hash: "sha256:" + "0".repeat(64),
    request_contract_ref: { scheme: "chronica.contract_id", value: "adv.request.v1" },
    result_contract_ref: { scheme: "chronica.contract_id", value: "adv.result.v1" },
    constraints: [],
    evidence_ref_ids: ["ev-adv-01"],
    provenance: prov("chronica.system-atlas"),
    ...overrides,
  };
  d.definition_hash = computeDefinitionHash(d);
  return d;
}
function makeBind(def, overrides = {}) {
  return {
    binding_id: "bind-adv-01",
    entity_id: "software.service:adv-01",
    capability_definition_id: def.capability_definition_id,
    capability_definition_hash: def.definition_hash,
    relation: "PROVIDES",
    constraints: [],
    evidence_ref_ids: ["ev-adv-01"],
    provenance: prov("chronica.system-atlas"),
    ...overrides,
  };
}
function baseCandidate() {
  const evidence_refs = [ev("ev-adv-01", "chronica.event", "chronica.event_id", "evt-adv-01")];
  const d = makeDef();
  const b = makeBind(d);
  return { definitions: [d], bindings: [b], evidence_refs };
}

// ADV 1-5: the fragment schema itself directly rejects every forbidden registration_state.
for (const [n, state] of [[1, "CANONICAL"], [2, "AUTHORIZED"], [3, "APPROVED"], [4, "EXECUTABLE"], [5, "ACTIVE"]]) {
  const fakeFragment = { schema: FRAGMENT_SCHEMA, registration_state: state, definitions: [], bindings: [], evidence_refs: [], content_hash: "sha256:" + "0".repeat(64) };
  const r = validate(fakeFragment, FRAGMENT_SCHEMA, ajv);
  adversarial(n, `CapabilityRegistryFragment schema directly rejects registration_state=${state}`, r.valid === false && errorsInclude(r.errors, "registration_state"), JSON.stringify(r.errors));
}

// ADV 6-7: conflicting duplicates fail identically under reversed source order.
{
  const evidence_refs = [ev("ev-rev-01", "chronica.event", "chronica.event_id", "evt-rev-01")];
  const d1 = makeDef({ capability_definition_id: "capdef-rev-conflict", capability_key: "chronica.world.read", evidence_ref_ids: ["ev-rev-01"] });
  const d2 = makeDef({ capability_definition_id: "capdef-rev-conflict", capability_key: "chronica.world.write", evidence_ref_ids: ["ev-rev-01"] });
  const b = makeBind(d1, { evidence_ref_ids: ["ev-rev-01"] });
  const forward = normalizeCapabilityRegistry({ definitions: [d1, d2], bindings: [b], evidence_refs }, ajv);
  const reversed = normalizeCapabilityRegistry({ definitions: [d2, d1], bindings: [b], evidence_refs }, ajv);
  adversarial(6, "CAPABILITY_DEFINITION_ID_CONFLICT fires identically regardless of source array order", forward.valid === false && reversed.valid === false && errorsInclude(forward.errors, "CAPABILITY_DEFINITION_ID_CONFLICT") && errorsInclude(reversed.errors, "CAPABILITY_DEFINITION_ID_CONFLICT"));
}
{
  const evidence_refs = [ev("ev-rev-02", "chronica.event", "chronica.event_id", "evt-rev-02")];
  const d = makeDef({ capability_definition_id: "capdef-rev-b", evidence_ref_ids: ["ev-rev-02"] });
  const b1 = makeBind(d, { binding_id: "bind-rev-conflict", entity_id: "software.service:x", evidence_ref_ids: ["ev-rev-02"] });
  const b2 = makeBind(d, { binding_id: "bind-rev-conflict", entity_id: "software.service:y", evidence_ref_ids: ["ev-rev-02"] });
  const forward = normalizeCapabilityRegistry({ definitions: [d], bindings: [b1, b2], evidence_refs }, ajv);
  const reversed = normalizeCapabilityRegistry({ definitions: [d], bindings: [b2, b1], evidence_refs }, ajv);
  adversarial(7, "CAPABILITY_BINDING_ID_CONFLICT fires identically regardless of source array order", forward.valid === false && reversed.valid === false && errorsInclude(forward.errors, "CAPABILITY_BINDING_ID_CONFLICT") && errorsInclude(reversed.errors, "CAPABILITY_BINDING_ID_CONFLICT"));
}

// ADV 8-9: required Section 34 determinism property test -- one heterogeneous
// candidate with multiple definitions/bindings/evidence_refs/constraints,
// permuted at every declared order-insensitive level simultaneously: top-level
// collection order, definition/binding constraint order, and
// definition/binding evidence_ref_ids order, all at once.
{
  const evidence_refs = [
    ev("ev-multi-01", "chronica.event", "chronica.event_id", "evt-multi-01"),
    ev("ev-multi-02", "machine.telemetry", "machine.telemetry_ref", "opcua://fleet/x#1", "machine.opcua"),
  ];
  const dA = makeDef({
    capability_definition_id: "capdef-multi-a",
    capability_key: "human.translation.perform",
    constraints: [
      { constraint_type: "human.constraint.language", value: ["en", "fr"] },
      { constraint_type: "capability.limit.max_batch_size", value: 10 },
    ],
    evidence_ref_ids: ["ev-multi-01", "ev-multi-02"],
  });
  const dB = makeDef({
    capability_definition_id: "capdef-multi-b",
    capability_key: "machine.axis.move",
    constraints: [{ constraint_type: "machine.limit.max_speed", value: 250, unit: "mm/s" }],
    evidence_ref_ids: ["ev-multi-02"],
  });
  const bA = makeBind(dA, { binding_id: "bind-multi-a", entity_id: "human.person:multi-01", constraints: [{ constraint_type: "human.constraint.language", value: "en" }], evidence_ref_ids: ["ev-multi-01"] });
  const bB = makeBind(dB, { binding_id: "bind-multi-b", entity_id: "machine.robot:multi-02", relation: "PROVIDES", evidence_ref_ids: ["ev-multi-02", "ev-multi-01"] });

  const reverseArr = (a) => [...a].reverse();
  const variantForward = { definitions: [dA, dB], bindings: [bA, bB], evidence_refs: [evidence_refs[0], evidence_refs[1]] };
  const variantTopLevelReversed = { definitions: [dB, dA], bindings: [bB, bA], evidence_refs: [evidence_refs[1], evidence_refs[0]] };
  const variantNestedReversed = {
    definitions: [{ ...dA, constraints: reverseArr(dA.constraints), evidence_ref_ids: reverseArr(dA.evidence_ref_ids) }, dB],
    bindings: [{ ...bA, evidence_ref_ids: reverseArr(bA.evidence_ref_ids) }, { ...bB, evidence_ref_ids: reverseArr(bB.evidence_ref_ids) }],
    evidence_refs: [evidence_refs[1], evidence_refs[0]],
  };

  const results = [variantForward, variantTopLevelReversed, variantNestedReversed].map((raw) => normalizeCapabilityRegistry(raw, ajv));
  adversarial(
    8,
    "MULTI_COLLECTION_CAPABILITY_DETERMINISM: 3 permutations (top-level collection order + nested constraint/evidence_ref_id order) of a multi-definition/binding/evidence heterogeneous candidate all produce identical content_hash",
    results.every((r) => r.valid) && results.every((r) => r.fragment.content_hash === results[0].fragment.content_hash)
  );
  adversarial(
    9,
    "the same 3 permutations also produce byte-identical canonicalStringify() output (not just equal content_hash)",
    results.every((r) => canonicalStringify(r.fragment) === canonicalStringify(results[0].fragment))
  );
}

// ADV 10: permuting a definition's constraints array does not change its definition_hash.
{
  const d1 = makeDef({ constraints: [{ constraint_type: "capability.limit.max_batch_size", value: 5 }, { constraint_type: "human.constraint.language", value: ["en", "fr"] }] });
  const d2 = makeDef({ constraints: [{ constraint_type: "human.constraint.language", value: ["en", "fr"] }, { constraint_type: "capability.limit.max_batch_size", value: 5 }] });
  adversarial(10, "CAPABILITY_DEFINITION_HASH_ACYCLIC/order-independence: reversed constraints array produces the identical definition_hash", d1.definition_hash === d2.definition_hash);
}

// ADV 11: permuting a definition's evidence_ref_ids does not change its definition_hash, and duplicates collapse.
{
  const d1 = makeDef({ evidence_ref_ids: ["ev-a", "ev-b"] });
  const d2 = makeDef({ evidence_ref_ids: ["ev-b", "ev-a", "ev-a"] });
  adversarial(11, "reversed + duplicated evidence_ref_ids produce the identical definition_hash (order-insensitive set)", d1.definition_hash === d2.definition_hash);
}

// ADV 12: JSON object-key-order permutation of the raw envelope produces byte-identical normalized output.
{
  const raw = baseCandidate();
  function reverseKeys(o) {
    if (Array.isArray(o)) return o.map(reverseKeys);
    if (o !== null && typeof o === "object") {
      const out = {};
      for (const k of Object.keys(o).reverse()) out[k] = reverseKeys(o[k]);
      return out;
    }
    return o;
  }
  const reordered = reverseKeys(raw);
  const rA = normalizeCapabilityRegistry(raw, ajv);
  const rB = normalizeCapabilityRegistry(reordered, ajv);
  adversarial(12, "CAPABILITY_OBJECT_KEY_PERMUTATION_DETERMINISM: reversed JSON object-key insertion order (at every nesting level) of the raw envelope -- returned fragment JSON.stringify() byte-identical", rA.valid && rB.valid && JSON.stringify(rA.fragment) === JSON.stringify(rB.fragment));
}

// ADV 13: input object mutation count == 0.
{
  const raw = baseCandidate();
  const before = JSON.stringify(raw);
  normalizeCapabilityRegistry(raw, ajv);
  const after = JSON.stringify(raw);
  adversarial(13, "INPUT_OBJECT_MUTATION_COUNT=0: raw input is byte-identical (JSON.stringify) before and after normalization", before === after);
}

// ADV 14: hash self-reference -- definition_hash is excluded from its own payload (acyclic), proven directly: two raw definition objects, identical in every field EXCEPT a forged definition_hash, must compute the identical expected hash (computeDefinitionHash() never reads its own input's definition_hash field).
{
  const shared = makeDef();
  const d1 = { ...shared, definition_hash: "sha256:" + "1".repeat(64) };
  const d2 = { ...shared, definition_hash: "sha256:" + "2".repeat(64) };
  adversarial(14, "CAPABILITY_DEFINITION_HASH_ACYCLIC: two definitions differing only in the (forged) definition_hash field itself compute the identical expected hash", d1.definition_hash !== d2.definition_hash && computeDefinitionHash(d1) === computeDefinitionHash(d2));
}

// ADV 15: filesystem-only assumption -- an evidence_ref using a raw filesystem "path" instead of {scheme, value} locator is rejected.
{
  const raw = baseCandidate();
  raw.evidence_refs = [{ evidence_ref_id: "ev-adv-01", evidence_kind: "legacy.filesystem", path: "/var/log/foo.log", provenance: prov("chronica.system-atlas") }];
  const r = normalizeCapabilityRegistry(raw, ajv);
  adversarial(15, "filesystem-only-evidence-assumption: a raw filesystem path instead of a {scheme,value} locator is rejected, not silently accepted", r.valid === false && errorsInclude(r.errors, "locator"));
}

// ADV 16-17: vendor-specific core leakage attempts beyond the negative-fixture matrix.
{
  const raw = baseCandidate();
  raw.definitions[0].sql_query = "SELECT * FROM users";
  const r = normalizeCapabilityRegistry(raw, ajv);
  adversarial(16, "vendor-specific-core-leakage: a raw SQL query field on a definition is rejected", r.valid === false && errorsInclude(r.errors, "sql_query"));
}
{
  const raw = baseCandidate();
  raw.bindings[0].shell_command = "rm -rf /";
  const r = normalizeCapabilityRegistry(raw, ajv);
  adversarial(17, "vendor-specific-core-leakage: a raw shell command field on a binding is rejected", r.valid === false && errorsInclude(r.errors, "shell_command"));
}

// ADV 18: machine-control authority escalation -- attempting to smuggle can_execute directly onto an effectful machine capability definition.
{
  const raw = baseCandidate();
  raw.definitions[0].capability_key = "machine.axis.move";
  raw.definitions[0].can_execute = true;
  const r = normalizeCapabilityRegistry(raw, ajv);
  adversarial(18, "machine-control-authority-escalation: can_execute cannot be smuggled onto an effectful machine.axis.move definition", r.valid === false && errorsInclude(r.errors, "can_execute"));
}

// ADV 19: canonical registration claim smuggled at the binding level.
{
  const raw = baseCandidate();
  raw.bindings[0].registration_state = "CANONICAL";
  const r = normalizeCapabilityRegistry(raw, ajv);
  adversarial(19, "canonical-registration-claim: registration_state cannot be smuggled onto an individual binding", r.valid === false && errorsInclude(r.errors, "registration_state"));
}

// ADV 20: binding ambiguity is NOT falsely flagged -- multiple distinct binding_ids for the same entity/definition pair (e.g. two separate implementations) are legitimate and both survive.
{
  const evidence_refs = [ev("ev-ambig-01", "chronica.event", "chronica.event_id", "evt-ambig-01")];
  const d = makeDef({ capability_definition_id: "capdef-ambig", evidence_ref_ids: ["ev-ambig-01"] });
  const b1 = makeBind(d, { binding_id: "bind-ambig-impl-a", entity_id: "software.service:ambig-01", evidence_ref_ids: ["ev-ambig-01"], implementation_ref: { scheme: "software.service_endpoint_id", value: "impl-a" } });
  const b2 = makeBind(d, { binding_id: "bind-ambig-impl-b", entity_id: "software.service:ambig-01", evidence_ref_ids: ["ev-ambig-01"], implementation_ref: { scheme: "software.service_endpoint_id", value: "impl-b" } });
  const r = normalizeCapabilityRegistry({ definitions: [d], bindings: [b1, b2], evidence_refs }, ajv);
  adversarial(20, "binding-ambiguity: two distinct binding_ids for the same entity_id+capability_definition_id (different implementations) are NOT falsely conflated -- both survive", r.valid === true && r.fragment.bindings.length === 2);
}

// ADV 21: D0 layering purity -- D.RUNTIME-0R2 repair: a base-branch `git diff` only ever proves
// something about one point in commit history (empty and therefore vacuously "passing" on any
// checkout where the working head already equals the upstream default branch). Replaced with the
// same durable content-based layering invariant class as ADV 22/24 below: D2's own production files
// only ever reach into D0's
// PUBLIC surface (normalize.mjs, schema-validate.mjs, both one directory up) or their own
// capability/ directory -- never anything else, regardless of git history.
adversarial(21, "D0_LAYERING_RESPECTED=YES: real import scan proves normalize.mjs/validate.mjs only import D0's public surface (normalize.mjs, schema-validate.mjs) or their own capability/ directory", (() => {
  const productionFiles = ["normalize.mjs", "validate.mjs"];
  const importRe = /from\s+["']([^"']+)["']/g;
  const d0PublicFiles = new Set(["normalize.mjs", "schema-validate.mjs"]);
  for (const file of productionFiles) {
    const src = readFileSync(resolve(HERE, file), "utf8");
    let m;
    while ((m = importRe.exec(src))) {
      const spec = m[1];
      if (!spec.startsWith(".")) continue;
      const resolved = resolve(HERE, spec);
      const isOwnDir = dirname(resolved) === HERE;
      const isD0PublicFile = dirname(resolved) === resolve(HERE, "..") && d0PublicFiles.has(resolved.split(/[\\/]/).pop());
      if (isOwnDir || isD0PublicFile) continue;
      return false;
    }
  }
  return true;
})());

// ADV 22: D2-D1 runtime independence -- a DURABLE architectural invariant, not a wave-scoped
// branch-diff assertion. D.CLEAN-3R2 discovered that a blanket "D1 is byte-identical to the
// upstream default branch" check is not a legitimate permanent D2 invariant: a later layer (D3)
// may make an architect-authorized, semantics-preserving change to D1 (the verifyCanonicalWorldSnapshot()
// extraction/reuse) without that touching anything D2 actually depends on. What D2 -- the
// Universal Capability Registry CANDIDATE kernel -- must durably guarantee instead is that its
// own PRODUCTION source never depends on D1's admission runtime (or any later-layer runtime):
// D2 represents entity_id in the same identifier universe D1 admits into, but never itself
// performs D1 canonical-world-membership admission, and must remain independently loadable/
// runnable without D1 or D3 existing at all. Proven by a real source/import scan of D2's own
// production files -- not prose in this test file, not a git-diff-against-a-branch check.
{
  // D2 has no schema-validate.mjs of its own -- it reuses D0's ../schema-validate.mjs
  // (legitimate D0 reuse, not a D1/later-layer dependency). Its own production files, siblings
  // of this tests.mjs in tools/universal-graph/capability/, are normalize.mjs and validate.mjs.
  const productionFiles = ["normalize.mjs", "validate.mjs"];
  const forbiddenImportPattern = /from\s+["']([^"']*\/(admission|capability-registration)\/[^"']*|\.\.\/admission[^"']*|\.\.\/capability-registration[^"']*)["']/;
  const offenders = [];
  for (const file of productionFiles) {
    const src = readFileSync(resolve(HERE, file), "utf8");
    if (forbiddenImportPattern.test(src)) offenders.push(file);
  }
  adversarial(
    22,
    "D2_D1_RUNTIME_DEPENDENCY=NONE / D2_LATER_LAYER_RUNTIME_DEPENDENCY=NONE: real import scan proves normalize.mjs/validate.mjs never import from ../admission or ../capability-registration (durable architectural invariant, not a wave-scoped branch-diff check)",
    offenders.length === 0,
    JSON.stringify(offenders)
  );
}

// ADV 23: System Atlas independence -- D.RUNTIME-0R2 repair: replaced the base-branch `git diff`
// purity check with the same durable content-based invariant class used elsewhere in this wave:
// D2's own production source never imports from System Atlas at all, on any checkout.
adversarial(23, "SYSTEM_ATLAS_DEPENDENCY=NONE: real import scan proves normalize.mjs/validate.mjs never import from System Atlas (docs/_machine/system-atlas, scripts/system-atlas)", (() => {
  const productionFiles = ["normalize.mjs", "validate.mjs"];
  const importRe = /from\s+["']([^"']+)["']/g;
  for (const file of productionFiles) {
    const src = readFileSync(resolve(HERE, file), "utf8");
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

// ADV 24: product/runtime independence -- repaired the same way ADV 22 above was (D.RUNTIME-0R):
// "product/runtime untouched" via a base-branch git diff is inherently incompatible with any
// later Rust runtime-materialization wave that legitimately touches crates/Cargo metadata, and it
// only ever proved something about one point in commit history, not a property of D2 itself.
// Replaced with the same durable architectural invariant class as ADV 22: D2's own production
// source imports nothing from product/runtime (crates, ui) at all.
adversarial(24, "PRODUCT_RUNTIME_DEPENDENCY=NONE: real import scan proves normalize.mjs/validate.mjs never import from crates or ui (durable architectural invariant, not a wave-scoped branch-diff check)", (() => {
  const productionFiles = ["normalize.mjs", "validate.mjs"];
  const importRe = /from\s+["']([^"']+)["']/g;
  for (const file of productionFiles) {
    const src = readFileSync(resolve(HERE, file), "utf8");
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

// ADV 25: old-D-prototype / Lane-C resurrection -- D.RUNTIME-0R2 repair: the previous version
// discovered which files to scan by asking git which paths differ from the upstream default
// branch, a base-branch-relative mechanism that is empty -- and therefore vacuously scans zero
// files -- on any checkout where the working head already equals that upstream branch, exactly
// the branch-context-dependence this wave closes. Replaced with a direct scan of this layer's own
// static, known file set -- every D2 schema/fixture/doc/reference-script file that exists on disk,
// checked unconditionally, regardless of git history or which branch is checked out.
{
  const forbidden = /lane[\s_-]?c\b|old[\s_-]?d[\s_-]?prototype/i;
  const SELF = fileURLToPath(import.meta.url); // this file's own label text below necessarily names what it forbids -- exclude it from its own scan, not the concepts it checks for.
  const d2Files = [
    resolve(CAP_DIR, "schemas", "capability-constraint.schema.json"),
    resolve(CAP_DIR, "schemas", "capability-registry-fragment.schema.json"),
    resolve(CAP_DIR, "schemas", "capability-definition.schema.json"),
    resolve(CAP_DIR, "schemas", "capability-binding.schema.json"),
    resolve(CAP_DIR, "schemas", "raw-capability-observation.schema.json"),
    resolve(CAP_DIR, "fixtures", "positive.json"),
    resolve(CAP_DIR, "fixtures", "negative.json"),
    resolve(CAP_DIR, "README.md"),
    resolve(CAP_DIR, "examples", "normalized-example.json"),
    resolve(HERE, "validate.mjs"),
    resolve(HERE, "normalize.mjs"),
    resolve(HERE, "tests.mjs"),
  ];
  let leaked = null;
  for (const abs of d2Files) {
    if (abs === SELF) continue;
    const text = readFileSync(abs, "utf8");
    if (forbidden.test(text)) { leaked = abs; break; }
  }
  adversarial(25, "OLD_D_IMPORTED=NO / C_RESURRECTED=NO: no D2 schema/fixture/doc/reference-script file references the old D prototype or Lane C (direct static-file scan, not a base-branch git diff)", leaked === null, leaked);
}

// ADV 26-27: actual CLI subprocess byte-determinism (array order + object-key order).
{
  const tmp = mkdtempSync(join(tmpdir(), "d2-cli-"));
  try {
    const evidence_refs = [ev("ev-cli-01", "chronica.event", "chronica.event_id", "evt-cli-01"), ev("ev-cli-02", "chronica.event", "chronica.event_id", "evt-cli-02")];
    const dA = makeDef({ capability_definition_id: "capdef-cli-a", evidence_ref_ids: ["ev-cli-01"] });
    const dB = makeDef({ capability_definition_id: "capdef-cli-b", capability_key: "software.repository.read", evidence_ref_ids: ["ev-cli-02"] });
    const bA = makeBind(dA, { binding_id: "bind-cli-a", evidence_ref_ids: ["ev-cli-01"] });
    const bB = makeBind(dB, { binding_id: "bind-cli-b", entity_id: "software.service:cli-02", evidence_ref_ids: ["ev-cli-02"] });

    const rawArrayA = { definitions: [dA, dB], bindings: [bA, bB], evidence_refs: [evidence_refs[0], evidence_refs[1]] };
    const rawArrayB = { definitions: [dB, dA], bindings: [bB, bA], evidence_refs: [evidence_refs[1], evidence_refs[0]] };

    function reverseKeys(o) {
      if (Array.isArray(o)) return o.map(reverseKeys);
      if (o !== null && typeof o === "object") {
        const out = {};
        for (const k of Object.keys(o).reverse()) out[k] = reverseKeys(o[k]);
        return out;
      }
      return o;
    }
    const rawKeyA = rawArrayA;
    const rawKeyB = reverseKeys(rawArrayA);

    const fArrayA = join(tmp, "array-a.json");
    const fArrayB = join(tmp, "array-b.json");
    const fKeyA = join(tmp, "key-a.json");
    const fKeyB = join(tmp, "key-b.json");
    writeFileSync(fArrayA, JSON.stringify(rawArrayA));
    writeFileSync(fArrayB, JSON.stringify(rawArrayB));
    writeFileSync(fKeyA, JSON.stringify(rawKeyA));
    writeFileSync(fKeyB, JSON.stringify(rawKeyB));

    const cli = resolve(HERE, "validate.mjs");
    const stdoutArrayA = execFileSync("node", [cli, fArrayA], { encoding: "utf8" });
    const stdoutArrayB = execFileSync("node", [cli, fArrayB], { encoding: "utf8" });
    const stdoutKeyA = execFileSync("node", [cli, fKeyA], { encoding: "utf8" });
    const stdoutKeyB = execFileSync("node", [cli, fKeyB], { encoding: "utf8" });

    adversarial(26, "CAPABILITY_CLI_BYTE_DETERMINISM: real CLI subprocess -- two array-order-permuted semantically-identical input files produce byte-identical stdout", stdoutArrayA.length > 0 && stdoutArrayA === stdoutArrayB);
    adversarial(27, "CAPABILITY_CLI_BYTE_DETERMINISM: real CLI subprocess -- two JSON object-key-order-permuted semantically-identical input files produce byte-identical stdout", stdoutKeyA.length > 0 && stdoutKeyA === stdoutKeyB);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

// ADV 28: --check-determinism CLI flag actually re-runs normalization and reports success on a valid input.
{
  const tmp = mkdtempSync(join(tmpdir(), "d2-cli-det-"));
  try {
    const f = join(tmp, "input.json");
    writeFileSync(f, JSON.stringify(baseCandidate()));
    const stdout = execFileSync("node", [resolve(HERE, "validate.mjs"), f, "--check-determinism"], { encoding: "utf8" });
    adversarial(28, "the CLI's --check-determinism flag succeeds (exit 0) on a genuinely deterministic input", stdout.includes("\"registration_state\": \"CANDIDATE_ONLY\""));
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

console.log(`\n== D2R repair: definition_version safe-integer domain + identity-reference scheme boundary ==`);
let nextAdv = 29;

function withVersion(version) {
  const evidence_refs = [ev("ev-ver-01", "chronica.event", "chronica.event_id", "evt-ver-01")];
  const d = makeDef({ capability_definition_id: "capdef-ver", definition_version: version, evidence_ref_ids: ["ev-ver-01"] });
  const b = makeBind(d, { binding_id: "bind-ver", evidence_ref_ids: ["ev-ver-01"] });
  return { definitions: [d], bindings: [b], evidence_refs };
}

// definition_version domain: 1 and Number.MAX_SAFE_INTEGER accepted; 0, negative,
// non-integer, and MAX_SAFE_INTEGER+1 all rejected at the schema boundary.
for (const [version, expectPass] of [
  [1, true],
  [9007199254740991, true],
  [0, false],
  [-1, false],
  [1.5, false],
  [9007199254740992, false],
]) {
  const r = normalizeCapabilityRegistry(withVersion(version), ajv);
  adversarial(
    nextAdv++,
    `definition_version=${version} ${expectPass ? "accepted (within the positive safe-integer domain)" : "rejected at the schema boundary (outside the positive safe-integer domain)"}`,
    expectPass ? r.valid === true : r.valid === false && errorsInclude(r.errors, "definition_version")
  );
}

// The capability_key/definition_version natural key never silently loses precision at the MAX_SAFE_INTEGER boundary.
{
  const evidence_refs = [ev("ev-natkey-01", "chronica.event", "chronica.event_id", "evt-natkey-01")];
  const d1 = makeDef({ capability_definition_id: "capdef-natkey-a", definition_version: 9007199254740991, evidence_ref_ids: ["ev-natkey-01"] });
  const d2 = makeDef({ capability_definition_id: "capdef-natkey-b", definition_version: 9007199254740991, constraints: [{ constraint_type: "capability.limit.max_batch_size", value: 1 }], evidence_ref_ids: ["ev-natkey-01"] });
  const b = makeBind(d1, { evidence_ref_ids: ["ev-natkey-01"] });
  const r = normalizeCapabilityRegistry({ definitions: [d1, d2], bindings: [b], evidence_refs }, ajv);
  adversarial(nextAdv++, "UNSAFE_CAPABILITY_DEFINITION_VERSION_ACCEPTED=NO: capability_key/definition_version natural-key conflict detection operates correctly at the MAX_SAFE_INTEGER boundary", r.valid === false && errorsInclude(r.errors, "CAPABILITY_KEY_VERSION_CONFLICT"));
}

function withImplementationRef(ref) {
  const evidence_refs = [ev("ev-implref-01", "chronica.event", "chronica.event_id", "evt-implref-01")];
  const d = makeDef({ capability_definition_id: "capdef-implref", evidence_ref_ids: ["ev-implref-01"] });
  const b = makeBind(d, { binding_id: "bind-implref", evidence_ref_ids: ["ev-implref-01"], implementation_ref: ref });
  return { definitions: [d], bindings: [b], evidence_refs };
}

// IMPLEMENTATION_REF_SEMANTICS=IDENTITY_REFERENCE_ONLY: scheme must denote an ID
// reference (ends in "_id"), never a raw locator/register/command mechanic --
// even though both fields are individually well-formed strings.
for (const [scheme, value, expectPass] of [
  ["software.service_endpoint_id", "impl-123", true],
  ["future.adapter_implementation_id", "future-123", true],
  ["web2app.dom_selector", "#login-button", false],
  ["modbus.register", "40001", false],
  ["shell.command", "anything", false],
]) {
  const r = normalizeCapabilityRegistry(withImplementationRef({ scheme, value }), ajv);
  adversarial(
    nextAdv++,
    `implementation_ref.scheme="${scheme}" ${expectPass ? "accepted as an identity reference" : "rejected -- a raw adapter mechanic is not an ID reference"}`,
    expectPass ? r.valid === true : r.valid === false && errorsInclude(r.errors, "scheme")
  );
}

adversarial(nextAdv++, "implementation_ref with a valid identity-reference scheme PLUS an extra token field still rejects (additionalProperties, not just scheme)", (() => {
  const r = normalizeCapabilityRegistry(withImplementationRef({ scheme: "software.service_endpoint_id", value: "impl-123", token: "secret-api-token" }), ajv);
  return r.valid === false && errorsInclude(r.errors, "token");
})());
adversarial(nextAdv++, "implementation_ref with a valid identity-reference scheme PLUS an extra selector field still rejects (additionalProperties, not just scheme)", (() => {
  const r = normalizeCapabilityRegistry(withImplementationRef({ scheme: "software.service_endpoint_id", value: "impl-123", selector: "#login-button" }), ajv);
  return r.valid === false && errorsInclude(r.errors, "selector");
})());

function withContractRef(which, ref) {
  const evidence_refs = [ev("ev-cref-01", "chronica.event", "chronica.event_id", "evt-cref-01")];
  const d = makeDef({ capability_definition_id: "capdef-cref", evidence_ref_ids: ["ev-cref-01"], [which]: ref });
  const b = makeBind(d, { binding_id: "bind-cref", evidence_ref_ids: ["ev-cref-01"] });
  return { definitions: [d], bindings: [b], evidence_refs };
}

// CONTRACT_REF_SEMANTICS=SEMANTIC_ARTIFACT_ID_REFERENCE, for BOTH
// request_contract_ref and result_contract_ref.
for (const which of ["request_contract_ref", "result_contract_ref"]) {
  for (const [scheme, value, expectPass] of [
    ["chronica.contract_id", "adv.contract.v1", true],
    ["software.openapi_operation_id", "postTranslate", true],
    ["future.semantic_contract_id", "future-contract-1", true],
    ["web2app.dom_selector", "#login-button", false],
    ["modbus.register", "40001", false],
    ["shell.command", "anything", false],
  ]) {
    const r = normalizeCapabilityRegistry(withContractRef(which, { scheme, value }), ajv);
    adversarial(
      nextAdv++,
      `${which}.scheme="${scheme}" ${expectPass ? "accepted as a semantic artifact/id reference" : "rejected -- a raw adapter mechanic is not a semantic contract reference"}`,
      expectPass ? r.valid === true : r.valid === false && errorsInclude(r.errors, "scheme")
    );
  }
}

adversarial(nextAdv++, "changing a valid contract ref scheme/value still alters definition_hash (contract refs remain inside the hash payload, unaffected by the identity-reference boundary repair)", (() => {
  const d1 = makeDef({ request_contract_ref: { scheme: "chronica.contract_id", value: "adv.request.v1" } });
  const d2 = makeDef({ request_contract_ref: { scheme: "chronica.contract_id", value: "adv.request.v2" } });
  return d1.definition_hash !== d2.definition_hash;
})());

console.log(`\n== D0 CANONICAL REGRESSION (required 167/167) ==`);
let d0Pass = 0, d0Fail = 0, d0Ok = false;
try {
  const out = execFileSync("node", [resolve(REPO_ROOT, "tools", "universal-graph", "tests.mjs")], { cwd: REPO_ROOT, encoding: "utf8" });
  const m = out.match(/checks: (\d+), pass: (\d+), fail: (\d+)/);
  if (m) { d0Pass = Number(m[2]); d0Fail = Number(m[3]); }
  d0Ok = d0Fail === 0;
} catch (err) {
  d0Ok = false;
  console.log(err.stdout ?? String(err));
}
check(`D0 canonical suite: 167/167 pass, no regression (actual: ${d0Pass}/${d0Pass + d0Fail})`, d0Ok && d0Pass === 167 && d0Fail === 0);

console.log(`\n== D1 CANONICAL REGRESSION (required 245/245) ==`);
let d1Pass = 0, d1Fail = 0, d1Ok = false;
try {
  const out = execFileSync("node", [resolve(REPO_ROOT, "tools", "universal-graph", "admission", "tests.mjs")], { cwd: REPO_ROOT, encoding: "utf8" });
  const m = out.match(/checks: (\d+), pass: (\d+), fail: (\d+)/);
  if (m) { d1Pass = Number(m[2]); d1Fail = Number(m[3]); }
  d1Ok = d1Fail === 0;
} catch (err) {
  d1Ok = false;
  console.log(err.stdout ?? String(err));
}
check(`D1 canonical suite: 245/245 pass, no regression (actual: ${d1Pass}/${d1Pass + d1Fail})`, d1Ok && d1Pass === 245 && d1Fail === 0);

console.log(`\n== summary ==`);
console.log(`positive: ${positive.length}, negative: ${negative.length}, adversarial: ${advCount}`);
console.log(`checks: ${pass + fail}, pass: ${pass}, fail: ${fail}`);
if (fail > 0) {
  console.log(`\nFAILING: ${failures.join(", ")}`);
  process.exit(1);
}
console.log("\nALL PASS");
process.exit(0);
