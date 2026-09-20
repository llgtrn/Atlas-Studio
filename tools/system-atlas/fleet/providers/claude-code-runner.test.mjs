import { test } from "node:test";
import assert from "node:assert/strict";
import { parseClaudePayload, taskExecutionClass } from "./claude-code-runner.mjs";

test("Claude payload parser accepts strict and fenced JSON", () => {
  assert.equal(parseClaudePayload('{"status":"COMPLETE"}').status, "COMPLETE");
  assert.equal(parseClaudePayload('```json\n{"status":"BLOCKED"}\n```').status, "BLOCKED");
});

test("Claude runner classifies read-only, candidate and direct mutation tasks", () => {
  assert.equal(taskExecutionClass("DONOR_ABSORB_SCOUT"), "READ_ONLY");
  assert.equal(taskExecutionClass("PACKAGE_BUILD"), "CANDIDATE_ONLY");
  assert.equal(taskExecutionClass("CHRONICA_CANDIDATE"), "CANDIDATE_ONLY");
  assert.equal(taskExecutionClass("PACKAGE_VERIFY_INTEGRATE"), "DIRECT_TARGET_MUTATION");
  assert.equal(taskExecutionClass("CHRONICA_VERIFY_INTEGRATE"), "DIRECT_TARGET_MUTATION");
  assert.equal(taskExecutionClass("CELL_RESET_BASELINE"), "DIRECT_TARGET_MUTATION");
  assert.equal(taskExecutionClass("DONOR_INTAKE"), "DIRECT_TARGET_MUTATION");
  assert.throws(() => taskExecutionClass("CELL_ENROLL"));
  assert.throws(() => taskExecutionClass("DONOR_RELOCATE"));
  assert.throws(() => taskExecutionClass("DONOR_ABSORB_BUILD"));
  assert.throws(() => taskExecutionClass("OPS_VERIFY_INTEGRATE"));
  assert.throws(() => taskExecutionClass("OPS_RECONVERGE"));
  assert.equal(taskExecutionClass("PACKAGE_RECONVERGE"), "DIRECT_TARGET_MUTATION");
  assert.throws(() => taskExecutionClass("UNKNOWN"));
});
