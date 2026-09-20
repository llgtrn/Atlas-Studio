import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { scanFileLineLimits } from "./file-line-limit.mjs";

function writeLines(path, count) {
  writeFileSync(path, Array.from({ length: count }, (_, i) => `line ${i + 1}`).join("\n"));
}

test("scanFileLineLimits reports files over the configured limit", () => {
  const root = mkdtempSync(join(tmpdir(), "chronica-line-limit-"));
  try {
    mkdirSync(join(root, "src"));
    writeLines(join(root, "src", "small.rs"), 3);
    writeLines(join(root, "src", "large.rs"), 6);

    const result = scanFileLineLimits(root, { limit: 5, extensions: [".rs"] });

    assert.deepEqual(result.violations.map((v) => [v.relativePath, v.lines]), [["src/large.rs", 6]]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("scanFileLineLimits defaults to a 600 line hard ceiling", () => {
  const root = mkdtempSync(join(tmpdir(), "chronica-line-limit-"));
  try {
    mkdirSync(join(root, "src"));
    writeLines(join(root, "src", "soft.rs"), 550);
    writeLines(join(root, "src", "hard.rs"), 601);

    const result = scanFileLineLimits(root, { extensions: [".rs"] });

    assert.equal(result.limit, 600);
    assert.deepEqual(result.violations.map((v) => [v.relativePath, v.lines]), [["src/hard.rs", 601]]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("scanFileLineLimits skips ignored directories", () => {
  const root = mkdtempSync(join(tmpdir(), "chronica-line-limit-"));
  try {
    mkdirSync(join(root, "target"), { recursive: true });
    mkdirSync(join(root, "docs", "_machine"), { recursive: true });
    writeLines(join(root, "target", "large.rs"), 6);
    writeLines(join(root, "docs", "_machine", "large.json"), 6);

    const result = scanFileLineLimits(root, { limit: 5, extensions: [".json", ".rs"] });

    assert.deepEqual(result.violations, []);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("scanFileLineLimits can scan only selected paths", () => {
  const root = mkdtempSync(join(tmpdir(), "chronica-line-limit-"));
  try {
    mkdirSync(join(root, "src"), { recursive: true });
    mkdirSync(join(root, "vendor"), { recursive: true });
    writeLines(join(root, "src", "large.rs"), 6);
    writeLines(join(root, "vendor", "large.rs"), 7);

    const result = scanFileLineLimits(root, { limit: 5, extensions: [".rs"], includePaths: ["src"] });

    assert.deepEqual(result.violations.map((v) => [v.relativePath, v.lines]), [["src/large.rs", 6]]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("scanFileLineLimits exempts extracted *_tests.rs files (parent prod files stay gated)", () => {
  const dir = mkdtempSync(join(tmpdir(), "line-limit-"));
  try {
    writeLines(join(dir, "decision_tests.rs"), 900); // extracted test module — exempt
    writeLines(join(dir, "decision.rs"), 900); // prod file — still a violation
    const result = scanFileLineLimits(dir, { extensions: [".rs"] });
    assert.deepEqual(
      result.violations.map((v) => v.relativePath),
      ["decision.rs"],
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
