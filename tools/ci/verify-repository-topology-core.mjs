#!/usr/bin/env node
// verify-repository-topology.mjs — hard repository-topology gate (CHRONICA HARD REPOSITORY
// TOPOLOGY REFOUNDATION directive). Checks:
//   1. Top-level directory allowlist -- every tracked top-level entry must be an allowed
//      directory or an allowed root file; anything else is a topology violation.
//   2. crates/cap/ monotonic non-increase -- the number of crates under crates/cap/ must never
//      exceed the recorded baseline (tools/ci/topology-baseline.json). It is NOT yet a strict
//      decrease gate (see AUDIT_MODE below); every real convergence slice should still push
//      the baseline down via a dedicated commit, never silently ratchet it up.
//   3. Forbidden crates/<subsystem>/ roots -- crates/core, crates/runtime, crates/adapter, and
//      the transitional crates/cap are the permanent product responsibility roots under crates/;
//      crates/infra is the one permanent NON-product exception (INFRA.FABRIC dev/build tooling,
//      hard-firewalled from the product graph in both directions by
//      tools/refoundation/validate-refoundation-layers.mjs -- no product crate may depend on it,
//      and it may not depend on any product crate). A new crates/<anything-else>/ (e.g.
//      crates/context/, crates/obs/) is a violation on sight, independent of the cap-count check.
//
// FAIL-CLOSED: genesis/, scripts/, skills/, deploy/, docker/, and ui/ have all been dissolved
// (see git history: genesis/'s 10 live crates + genesis/ui were each dispositioned
// SUPERSEDED/SPLIT/DEAD and the tree deleted; scripts/ moved into tools/{release,packaging,dev,
// testing/smoke,repo,agent,system-atlas}/; skills/ moved into tools/agent/skills/; deploy/+docker/
// moved into ops/{deploy,container,postgres,compose}/; ui/ moved into apps/ui/, including the
// pnpm workspace globs, every depth-relative path inside it, and CI/deploy/docs references).
// KNOWN_UNRESOLVED_TOP_LEVEL_DIRS is empty -- there is no remaining tracked exception for a
// product/runtime directory. .claude/ and .specify/ are not migration debt -- they are the
// narrow TOOL-METADATA root category this repo's own live Claude Code skill loader and its
// Spec-Kit dependency require, minimized to a proven content allowlist and enforced by
// checkToolMetadataContentAllowlist regardless of AUDIT_MODE (see TOOL_METADATA_TOP_LEVEL_DIRS
// below). With every prior blocker resolved, this gate is now fail-closed: an unknown top-level
// directory, an unknown root file, a forbidden crates/<x>/ root, a crates/cap/ count increase, or
// a tool-metadata content violation all exit 1.
//
// Usage:
//   node tools/ci/verify-repository-topology.mjs
//
// Exit code: 0 if clean; 1 on any violation. TOPOLOGY_AUDIT_MODE=true can force report-only mode
// back on for local debugging of a new, not-yet-dispositioned violation -- never set it in CI.
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = join(SCRIPT_DIR, '..', '..')
const BASELINE_PATH = join(SCRIPT_DIR, 'topology-baseline.json')

// Fail-closed by default (see the module doc above for why this is now safe). A
// `TOPOLOGY_AUDIT_MODE=true` env var can force report-only mode back on for local debugging of a
// brand-new, not-yet-dispositioned violation without editing this file -- never set it in CI.
const AUDIT_MODE = process.env.TOPOLOGY_AUDIT_MODE === 'true'

// The closed top-level directory contract. Anything else tracked at repo root is a violation.
// `.cargo/` is standard first-party Cargo tooling config (the `cargo devbuild` alias into
// crates/infra/chronica-infra-build-fabric-cli, see docs/dev/build-fabric.md) -- the same
// category as `.github/`, not migration debt.
//
// 2026-09-16 hard refoundation: crates/ is retired (see
// docs/decisions/0016-legacy-backend-retired-to-git-history.md and AGENTS.md section 4). core/,
// runtime/, adapter/, organism/ are now the permanent top-level single-package-crate roots,
// replacing crates/{core,cap,adapter,runtime} directly at repo root (not nested under crates/).
const ALLOWED_TOP_LEVEL_DIRS = new Set(['.cargo', '.github', 'adapter', 'apps', 'bindings', 'core', 'docs', 'graph', 'ops', 'organism', 'runtime', 'tools'])

// Directories still present at repo root that are KNOWN, DOCUMENTED, NOT-YET-RESOLVED exceptions
// -- each has an exact blocker and a retirement condition recorded in the topology refoundation's
// own final report / commit history, not silently tolerated. Listed here ONLY so audit-mode
// output distinguishes "known blocker, tracked" from "brand-new, unreviewed violation" -- this
// list must shrink to empty before AUDIT_MODE can safely flip to false, and every removal from
// it should be a one-line diff in the same commit that actually resolves the item.
const KNOWN_UNRESOLVED_TOP_LEVEL_DIRS = new Set([])

// TOOL-METADATA roots: not migration debt, not a temporary/tracked exception. These are
// third-party-tool-required scaffolding directories that this repo's own live Claude Code
// integration depends on to function at all -- there is no alternate scan-root the harness
// supports, so they are a permanent, narrow, content-restricted category of their own, checked
// on every run regardless of AUDIT_MODE. See docs/workflows/ (topology refoundation final
// report) for the exact content allowlist each one is held to.
const TOOL_METADATA_TOP_LEVEL_DIRS = new Set([
  '.claude', // live Claude Code skills (speckit-*, design-guide) with no alternate scan-root
  '.specify', // spec-kit scaffold the .claude/skills/speckit-* skills hard-depend on
])

// Allowed root-level files (config/entry points genuinely required at repo root). Kept narrow on
// purpose -- anything not on this list is a root-file-hygiene violation, matching the topology
// directive's own root-file audit.
const ALLOWED_ROOT_FILES = new Set([
  'README.md', 'LICENSE', 'LICENSE.md', 'LICENSE.txt', 'AGENTS.md',
  'Cargo.toml', 'Cargo.lock', 'deny.toml',
  'package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'tsconfig.base.json',
  'Dockerfile',
  '.gitignore', '.gitattributes', '.mailmap', '.npmrc', '.env.example', '.dockerignore', '.gitguardian.yaml',
])

// Never git-tracked build/dependency output -- irrelevant to repository topology regardless of
// gitignore specifics, and legitimately present on disk in any real checkout/CI runner.
const IGNORED_UNTRACKED_DIRS = new Set(['.git', 'target', 'node_modules'])

function listTopLevelEntries() {
  return readdirSync(REPO_ROOT, { withFileTypes: true })
    .filter((e) => !IGNORED_UNTRACKED_DIRS.has(e.name))
    .map((e) => ({ name: e.name, isDir: e.isDirectory() }))
}

function checkTopLevelAllowlist(violations, warnings) {
  for (const entry of listTopLevelEntries()) {
    if (entry.isDir) {
      if (ALLOWED_TOP_LEVEL_DIRS.has(entry.name)) continue
      if (TOOL_METADATA_TOP_LEVEL_DIRS.has(entry.name)) {
        warnings.push(`tool-metadata root (permanent, narrow-content category, not migration debt): ${entry.name}/`)
        continue
      }
      if (KNOWN_UNRESOLVED_TOP_LEVEL_DIRS.has(entry.name)) {
        warnings.push(`known unresolved top-level directory (tracked exception): ${entry.name}/`)
        continue
      }
      violations.push(`unknown top-level directory not on the allowlist and not a tracked exception: ${entry.name}/`)
    } else {
      if (ALLOWED_ROOT_FILES.has(entry.name)) continue
      violations.push(`unknown root file not on the allowlist: ${entry.name}`)
    }
  }
}

// Exact content allowlist for the TOOL-METADATA roots (see TOOL_METADATA_TOP_LEVEL_DIRS above).
// Not a directory-shape check -- every individual tracked file must match one of these patterns.
// Grown only when a currently-live Spec-Kit/Claude-Code skill file is proven to require the new
// entry (the same bar this pass used to prune 15 previously-untracked-as-live .specify/ files:
// the upstream commands/*.md source templates, the bash script scaffold, and vscode-settings.json
// were all confirmed, by grepping every live .claude/skills/speckit-*/SKILL.md body for a literal
// reference, to have zero live readers).
const TOOL_METADATA_CONTENT_ALLOWLIST = [
  /^\.claude\/launch\.json$/,
  /^\.claude\/skills\/[^/]+\/.+/,
  /^\.specify\/memory\/constitution\.md$/,
  /^\.specify\/templates\/(checklist|constitution|plan|spec|tasks)-template\.md$/,
]

function listFilesRecursive(absDir, relPrefix) {
  const out = []
  for (const entry of readdirSync(absDir, { withFileTypes: true })) {
    const rel = `${relPrefix}/${entry.name}`
    const abs = join(absDir, entry.name)
    if (entry.isDirectory()) {
      out.push(...listFilesRecursive(abs, rel))
    } else {
      out.push(rel)
    }
  }
  return out
}

function checkToolMetadataContentAllowlist(violations) {
  for (const dirName of TOOL_METADATA_TOP_LEVEL_DIRS) {
    const absDir = join(REPO_ROOT, dirName)
    if (!existsSync(absDir)) continue
    for (const relPath of listFilesRecursive(absDir, dirName)) {
      if (!TOOL_METADATA_CONTENT_ALLOWLIST.some((pattern) => pattern.test(relPath))) {
        violations.push(`tool-metadata root content violation -- ${relPath} is not on the strict allowlist for ${dirName}/ (add it only once a live .claude/skills/**/SKILL.md is proven to require it)`)
      }
    }
  }
}

function checkForbiddenCrateRoots(violations) {
  const cratesDir = join(REPO_ROOT, 'crates')
  if (!existsSync(cratesDir)) return
  const allowed = new Set(['core', 'runtime', 'adapter', 'cap', 'infra'])
  for (const entry of readdirSync(cratesDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue
    if (!allowed.has(entry.name)) {
      violations.push(`forbidden crates/${entry.name}/ root -- only crates/{core,runtime,adapter} are permanent product roots, crates/cap is the one transitional exception, and crates/infra is the one permanent non-product (dev/build tooling) axis`)
    }
  }
}

function countCapCrates() {
  const capDir = join(REPO_ROOT, 'crates', 'cap')
  if (!existsSync(capDir)) return 0
  return readdirSync(capDir, { withFileTypes: true }).filter((e) => e.isDirectory()).length
}

function checkCapMonotonicNonIncrease(violations, warnings) {
  const current = countCapCrates()
  if (!existsSync(BASELINE_PATH)) {
    writeFileSync(BASELINE_PATH, JSON.stringify({ cap_count: current }, null, 2) + '\n')
    warnings.push(`no topology-baseline.json found -- wrote one recording the current crates/cap/ count (${current}) as the new baseline`)
    return
  }
  const baseline = JSON.parse(readFileSync(BASELINE_PATH, 'utf8'))
  if (current > baseline.cap_count) {
    violations.push(`crates/cap/ count increased: baseline ${baseline.cap_count} -> current ${current}. A new crate must never be added under crates/cap/ -- it is transitional debt, not a permanent root. If this crate is real and new, it belongs under crates/core, crates/runtime, or crates/adapter by actual responsibility.`)
  } else if (current < baseline.cap_count) {
    warnings.push(`crates/cap/ count decreased: baseline ${baseline.cap_count} -> current ${current} -- update tools/ci/topology-baseline.json to ${current} in this same change so the new, lower count becomes the enforced floor (never let a real reduction silently allow a future increase back toward the old number).`)
  }
}

function main() {
  const violations = []
  const warnings = []
  // Enforced on every run, independent of AUDIT_MODE: the tool-metadata roots are not migration
  // debt awaiting a flip-over date, they are a permanent contract with a strict content allowlist
  // (see TOOL_METADATA_CONTENT_ALLOWLIST above). A violation here is real today, not "known and
  // tracked."
  const alwaysEnforcedViolations = []

  checkTopLevelAllowlist(violations, warnings)
  checkForbiddenCrateRoots(violations)
  checkCapMonotonicNonIncrease(violations, warnings)
  checkToolMetadataContentAllowlist(alwaysEnforcedViolations)

  if (warnings.length > 0) {
    console.log(`[topology] ${warnings.length} known/tracked item(s):`)
    for (const w of warnings) console.log(`  - ${w}`)
  }

  if (alwaysEnforcedViolations.length > 0) {
    console.error(`[topology] ${alwaysEnforcedViolations.length} tool-metadata content violation(s) (enforced regardless of AUDIT_MODE):`)
    for (const v of alwaysEnforcedViolations) console.error(`  - ${v}`)
    process.exit(1)
  }

  if (violations.length > 0) {
    console.error(`[topology] ${violations.length} violation(s) of the hard repository-topology contract:`)
    for (const v of violations) console.error(`  - ${v}`)
    if (AUDIT_MODE) {
      console.error('[topology] AUDIT MODE: reporting only, not failing the build. See this script\'s own module doc for the fail-closed switch-over plan.')
      process.exit(0)
    }
    process.exit(1)
  }

  console.log('[topology] clean: no unresolved topology violations found.')
  process.exit(0)
}

main()
