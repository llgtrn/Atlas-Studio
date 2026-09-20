import { test } from "node:test";
import assert from "node:assert/strict";
import {
  checkDuplicateComponentIds,
  checkDanglingDependencyEndpoints,
  checkDeterministicOrdering,
  checkSidecarHardViolations,
  main,
} from "./validate-v2.mjs";

test("checkDuplicateComponentIds catches a repeated id", () => {
  const errors = checkDuplicateComponentIds([{ id: "core" }, { id: "adapter" }, { id: "core" }]);
  assert.equal(errors.length, 1);
  assert.match(errors[0], /duplicate component id: core/);
});

test("checkDuplicateComponentIds passes clean input", () => {
  assert.deepEqual(checkDuplicateComponentIds([{ id: "core" }, { id: "adapter" }]), []);
});

test("checkDanglingDependencyEndpoints catches an unknown source and target", () => {
  const components = [{ id: "core" }];
  const dependencies = [{ source: "core", target: "nonexistent" }, { source: "ghost", target: "core" }];
  const errors = checkDanglingDependencyEndpoints(components, dependencies);
  assert.equal(errors.length, 2);
});

test("checkDeterministicOrdering catches an unsorted component list", () => {
  const errors = checkDeterministicOrdering([{ id: "b" }, { id: "a" }], []);
  assert.equal(errors.length, 1);
  assert.match(errors[0], /components\.json is not sorted/);
});

test("checkDeterministicOrdering catches an unsorted dependency list", () => {
  const components = [{ id: "a" }, { id: "b" }, { id: "c" }];
  const dependencies = [{ source: "c", target: "a" }, { source: "a", target: "b" }];
  const errors = checkDeterministicOrdering(components, dependencies);
  assert.equal(errors.length, 1);
  assert.match(errors[0], /dependencies\.json is not sorted/);
});

test("checkDeterministicOrdering passes already-sorted lists", () => {
  const components = [{ id: "a" }, { id: "b" }];
  const dependencies = [{ source: "a", target: "b" }, { source: "b", target: "c" }];
  assert.deepEqual(checkDeterministicOrdering(components, dependencies), []);
});

test("main() passes closed (exit 0) on the live repo, since generate() itself already validated everything it wrote", () => {
  const exitCode = main();
  assert.equal(exitCode, 0);
});


test("checkSidecarHardViolations fails on connector/core protocol leaks but ignores review-only findings", () => {
  const errors = checkSidecarHardViolations({
    connectorAtlas: {
      violations: [
        { id: "CONNECTOR-0001", type: "PROTOCOL_DEPENDENCY_LEAKED_INTO_CORE", severity: "HARD", path: "core/Cargo.toml", detail: "modbus" },
        { id: "CONNECTOR-0002", type: "PROTOCOL_SPECIFIC_RUNTIME_PATH_REVIEW", severity: "REVIEW_REQUIRED", path: "runtime/src/modbus.rs", detail: "review" },
      ],
    },
    organismModelAtlas: { violations: [] },
    intelligenceBoundaryAtlas: { violations: [] },
  });
  assert.equal(errors.length, 1);
  assert.match(errors[0], /PROTOCOL_DEPENDENCY_LEAKED_INTO_CORE/);
});
