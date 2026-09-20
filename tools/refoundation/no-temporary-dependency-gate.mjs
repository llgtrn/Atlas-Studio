#!/usr/bin/env node
// CHRONICA -- GIT IS THE ABSORPTION LEDGER, section 15: "TEMPORARY IS NEVER PRODUCTION."
// temporary/ holds donor source snapshots for absorption reading only. Production code
// (core/runtime/adapter/organism, apps/ui and its packages, tools/) must never import, build,
// or link against it. This is a real, mechanically-derived gate, not a manually maintained list:
//   - Cargo: every workspace crate's `path = "..."` dependency is resolved and checked.
//   - TS workspace: every package.json dependency value using the file:/link: protocol is
//     resolved and checked.
//   - Source scan (defensive backstop): any literal "temporary/" path string inside
//     core/runtime/adapter/organism/apps/tools source files (catches include_str!/fs::read/
//     require() style references a manifest-level check can't see).
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";
import { REPO_ROOT, repoRel, loadCargoMetadata } from "../system-atlas/lib.mjs";

// Scoped to actual production roots only -- NOT tools/, whose entire job is to scan, measure,
// and exclude temporary/ by path (donor-burndown, reality-atlas, this gate itself); a tooling
// script referencing "temporary/" as a string is doing its job, not a production dependency.
const SOURCE_SCAN_ROOTS = ["core", "runtime", "adapter", "organism", "apps"];
const SOURCE_EXTENSIONS = new Set([".rs", ".ts", ".tsx", ".mjs", ".cjs", ".js"]);
const TEMPORARY_LITERAL = /["'`](\.\.\/)*temporary\//;

function cargoPathDependencyViolations(metadata) {
  const workspaceIds = new Set(metadata.workspace_members);
  const violations = [];
  for (const pkg of metadata.packages) {
    if (!workspaceIds.has(pkg.id)) continue;
    const manifestDir = dirname(pkg.manifest_path);
    for (const dep of pkg.dependencies) {
      if (!dep.path) continue;
      const resolved = resolve(manifestDir, dep.path);
      if (repoRel(resolved).startsWith("temporary/")) {
        violations.push(`${repoRel(pkg.manifest_path)}: crate "${pkg.name}" path-depends on ${repoRel(resolved)}`);
      }
    }
  }
  return violations;
}

function tsWorkspacePackagePaths() {
  const workspace = parseYaml(readFileSync(resolve(REPO_ROOT, "pnpm-workspace.yaml"), "utf8"));
  const out = [];
  for (const pattern of workspace.packages ?? []) {
    if (pattern.endsWith("/*")) {
      const base = pattern.slice(0, -2);
      const baseDir = resolve(REPO_ROOT, base);
      if (!existsSync(baseDir)) continue;
      for (const entry of readdirSync(baseDir, { withFileTypes: true })) {
        if (entry.isDirectory()) out.push(`${base}/${entry.name}`);
      }
    } else {
      out.push(pattern);
    }
  }
  return out.filter((p) => existsSync(resolve(REPO_ROOT, p, "package.json")));
}

function tsFileDependencyViolations() {
  const violations = [];
  for (const path of tsWorkspacePackagePaths()) {
    const pkgJsonPath = resolve(REPO_ROOT, path, "package.json");
    const pkg = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
    const allDeps = { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };
    for (const [name, spec] of Object.entries(allDeps)) {
      const match = /^(?:file|link):(.+)$/.exec(spec);
      if (!match) continue;
      const resolved = resolve(REPO_ROOT, path, match[1]);
      if (repoRel(resolved).startsWith("temporary/")) {
        violations.push(`${join(path, "package.json")}: dependency "${name}" resolves to ${repoRel(resolved)}`);
      }
    }
  }
  return violations;
}

function walk(dir, out) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "target" || entry.name === ".git") continue;
      walk(full, out);
    } else if (SOURCE_EXTENSIONS.has(`.${entry.name.split(".").pop()}`)) {
      out.push(full);
    }
  }
}

// Strips `//` line comments (which covers Rust's `//`/`///`/`//!` doc-comment forms and JS/TS
// line comments alike -- all SOURCE_EXTENSIONS use this comment style) before scanning for a
// literal temporary/ reference. Citing a donor's provenance path in a doc comment -- e.g.
// "pressure-formed from X (temporary/foo/bar.rs: ...)" or the same wrapped in backticks for
// markdown -- is the established, encouraged style throughout this codebase's absorption
// commits; it is not a production dependency and must never trip this gate. A real dependency
// (include_str!, fs::read, a path constant) appears in actual code, never only after a `//`.
// This can only produce false negatives (a real dependency hidden behind a same-line trailing
// comment marker), never false positives, which is the correct asymmetry for a gate that exists
// to catch mistakes without blocking legitimate documentation.
export function stripLineComments(content) {
  return content
    .split("\n")
    .map((line) => {
      const idx = line.indexOf("//");
      return idx === -1 ? line : line.slice(0, idx);
    })
    .join("\n");
}

function sourceLiteralViolations() {
  const violations = [];
  for (const root of SOURCE_SCAN_ROOTS) {
    const dir = resolve(REPO_ROOT, root);
    if (!existsSync(dir)) continue;
    const files = [];
    walk(dir, files);
    for (const file of files) {
      let content;
      try {
        content = readFileSync(file, "utf8");
      } catch {
        continue; // binary or unreadable, not a source reference
      }
      if (TEMPORARY_LITERAL.test(stripLineComments(content))) {
        violations.push(`${repoRel(file)}: literal reference to a temporary/ path`);
      }
    }
  }
  return violations;
}

export function findTemporaryDependencyViolations() {
  const metadata = loadCargoMetadata();
  return [
    ...cargoPathDependencyViolations(metadata),
    ...tsFileDependencyViolations(),
    ...sourceLiteralViolations(),
  ];
}

function main() {
  const violations = findTemporaryDependencyViolations();
  if (violations.length) {
    console.error(`TEMPORARY IS NEVER PRODUCTION: ${violations.length} violation(s):`);
    for (const v of violations) console.error(`  - ${v}`);
    return 1;
  }
  console.log("no-temporary-dependency-gate: passed -- no production code depends on temporary/.");
  return 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.exit(main());
