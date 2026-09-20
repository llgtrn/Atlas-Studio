import assert from "node:assert/strict";
import test from "node:test";

import {
  compareBaseline,
  crateLayer,
  forbiddenEdges,
  forbiddenSubsystemRoots,
} from "./validate-refoundation-layers.mjs";

const root = "/repo";
// 2026-09-16 hard refoundation: core/, runtime/, adapter/, organism/ are top-level
// single-package crates -- their own Cargo.toml directly, not nested under a
// crates/<layer>/<name>/ wrapper. Each fixture package therefore represents a whole layer.
// A dependency entry is either a plain name (an ordinary [dependencies] edge, kind: null) or
// {name, dev: true} for a [dev-dependencies] edge (cargo metadata's kind: "dev").
const pkg = (layer, dependencies = [], name = layer) => ({
  id: `${name} 0.1.0 (path+file:///repo/${layer})`,
  name,
  manifest_path: `${root}/${layer}/Cargo.toml`,
  dependencies: dependencies.map((dependency) =>
    typeof dependency === "string"
      ? { name: dependency, kind: null }
      : { name: dependency.name, kind: dependency.dev ? "dev" : null },
  ),
});

test("classifies only physical refoundation layers", () => {
  assert.equal(crateLayer("/repo/core/Cargo.toml", root), "core");
  assert.equal(crateLayer("/repo/runtime/Cargo.toml", root), "runtime");
  assert.equal(crateLayer("/repo/adapter/Cargo.toml", root), "adapter");
  assert.equal(crateLayer("/repo/organism/Cargo.toml", root), "organism");
  // The retired crates/ wrapper no longer classifies -- see docs/decisions/0016.
  assert.equal(crateLayer("/repo/crates/core/ids/Cargo.toml", root), null);
  assert.equal(crateLayer("/repo/web2app/Cargo.toml", root), null);
  assert.equal(crateLayer("/repo/tools/example/Cargo.toml", root), null);
});

test("rejects Web2App as a fifth physical workspace root", () => {
  const packages = [
    pkg("core"),
    {
      id: "browser-mechanics 0.1.0 (path+file:///repo/crates/web2app/browser)",
      name: "browser-mechanics",
      manifest_path: `${root}/crates/web2app/browser/Cargo.toml`,
      dependencies: [],
    },
  ];
  const metadata = { packages, workspace_members: packages.map(({ id }) => id) };
  assert.deepEqual(forbiddenSubsystemRoots(metadata, root), [
    "crates/web2app/browser/Cargo.toml",
  ]);
});

test("reports reverse dependencies and permits intended direction", () => {
  const packages = [
    pkg("core", ["adapter"]), // forbidden: core must not depend on adapter
    pkg("runtime", ["core"]), // intended: runtime depends only on core
    pkg("adapter", ["core", "runtime"]), // intended: adapter depends on core + runtime
    pkg("organism", ["core", "runtime"]), // intended: organism depends on core + runtime
  ];
  const metadata = { packages, workspace_members: packages.map(({ id }) => id) };
  assert.deepEqual(forbiddenEdges(metadata, root), [
    "core (core) -> adapter (adapter)",
  ]);
});

test("a dev-dependency in the forbidden direction is not flagged (never reaches a production binary)", () => {
  const packages = [
    pkg("core", [{ name: "adapter", dev: true }]),
    pkg("adapter"),
  ];
  const metadata = { packages, workspace_members: packages.map(({ id }) => id) };
  assert.deepEqual(forbiddenEdges(metadata, root), []);
});

test("runtime may not depend on adapter or organism (runtime depends only on core)", () => {
  const packages = [
    pkg("runtime", ["adapter", "organism"]),
    pkg("adapter"),
    pkg("organism"),
  ];
  const metadata = { packages, workspace_members: packages.map(({ id }) => id) };
  assert.deepEqual(forbiddenEdges(metadata, root), [
    "runtime (runtime) -> adapter (adapter)",
    "runtime (runtime) -> organism (organism)",
  ]);
});

test("organism may not depend on adapter (protocol/provider mechanics stay out of organism semantics)", () => {
  const packages = [
    pkg("organism", ["adapter"]),
    pkg("adapter"),
  ];
  const metadata = { packages, workspace_members: packages.map(({ id }) => id) };
  assert.deepEqual(forbiddenEdges(metadata, root), [
    "organism (organism) -> adapter (adapter)",
  ]);
});

test("adapter MAY depend on organism (2026-09-17 dependency-inversion correction: adapter implements organism-owned analytical ports such as organism::world_model::WorldModel; organism itself still never depends on adapter, so this grants organism no new effect path)", () => {
  const packages = [
    pkg("adapter", ["organism"]),
    pkg("organism"),
  ];
  const metadata = { packages, workspace_members: packages.map(({ id }) => id) };
  assert.deepEqual(forbiddenEdges(metadata, root), []);
});

test("the reverse direction remains HARD forbidden: organism may still never depend on adapter even though adapter now depends on organism", () => {
  const packages = [
    pkg("adapter", ["organism"]),
    pkg("organism", ["adapter"]),
  ];
  const metadata = { packages, workspace_members: packages.map(({ id }) => id) };
  assert.deepEqual(forbiddenEdges(metadata, root), [
    "organism (organism) -> adapter (adapter)",
  ]);
});

test("baseline comparison rejects additions and stale repaired debt", () => {
  assert.deepEqual(compareBaseline(["core -> adapter", "runtime -> adapter"], ["core -> adapter", "core -> organism"]), {
    newEdges: ["runtime -> adapter"],
    repairedEdges: ["core -> organism"],
  });
});
