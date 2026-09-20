#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";
import { loadChecklist, validateChecklist } from "../../refoundation/donor-corpus.mjs";
import { compileSchemas, validate, formatErrors } from "../schema-validate.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..", "..");
const REGISTRY_PATH = resolve(HERE, "ops-registry.yaml");
const SCHEMA_DIR = resolve(ROOT, "tools/system-atlas/schema");

function validateInstance(instance, schemaId) {
  const result = validate(instance, schemaId, compileSchemas(SCHEMA_DIR));
  if (!result.valid) throw new Error(`${schemaId} validation failed:\n${formatErrors(result.errors)}`);
}

export function loadFleetRegistry(path = REGISTRY_PATH) {
  const registry = parseYaml(readFileSync(path, "utf8"));
  if (registry?.schema_version !== 2) throw new Error("fleet registry schema_version must be 2");
  if (!Array.isArray(registry.development_cells)) throw new Error("fleet registry must contain development_cells[]");
  return registry;
}

// Transitional compatibility alias for callers that have not yet renamed the import.
export const loadOpsRegistry = loadFleetRegistry;

function targetSet(registry) {
  return new Set([registry.canonical_repo, ...registry.development_cells.map((row) => row.repo)]);
}

function cellByRepo(registry) {
  return new Map(registry.development_cells.map((row) => [row.repo, row]));
}

function matchingDomainTarget(domain, rules) {
  if (!domain) return null;
  const matches = Object.entries(rules ?? {})
    .filter(([prefix]) => domain === prefix || domain.startsWith(prefix))
    .sort((a, b) => b[0].length - a[0].length);
  return matches[0] ?? null;
}

export function allocateDonors({ checklist = loadChecklist(), registry = loadFleetRegistry(), now = new Date().toISOString() } = {}) {
  const checklistErrors = validateChecklist(checklist);
  if (checklistErrors.length) throw new Error(`donor corpus invalid:\n${checklistErrors.join("\n")}`);
  if (registry?.schema_version !== 2) throw new Error("fleet registry schema_version must be 2");
  if (!Array.isArray(registry.development_cells)) throw new Error("fleet registry must contain development_cells[]");

  const knownTargets = targetSet(registry);
  const cells = cellByRepo(registry);
  const allocations = [];

  for (const donor of checklist.donors) {
    const basis = [];
    const candidates = [];

    const override = registry.donor_overrides?.[donor.repo];
    if (override) {
      candidates.push(override);
      basis.push(`override:${donor.repo}`);
    }

    const domainMatch = matchingDomainTarget(donor.domain, registry.domain_prefix_rules);
    if (domainMatch) {
      candidates.push(domainMatch[1]);
      basis.push(`domain:${domainMatch[0]}`);
    }

    for (const family of donor.family ?? []) {
      const target = registry.family_rules?.[family];
      if (!target) throw new Error(`unmapped family ${family} for ${donor.id} ${donor.repo}`);
      candidates.push(target);
      basis.push(`family:${family}`);
    }

    const uniqueCandidates = [...new Set(candidates)];
    const primary = uniqueCandidates[0] ?? null;
    if (primary && !knownTargets.has(primary)) {
      throw new Error(`allocation target ${primary} for ${donor.id} is not registered`);
    }
    for (const target of uniqueCandidates.slice(1)) {
      if (!knownTargets.has(target)) throw new Error(`secondary target ${target} for ${donor.id} is not registered`);
    }

    const duplicate = donor.status === "DUPLICATE";
    const allocationState = duplicate ? "DUPLICATE_COLLAPSED" : primary ? "ALLOCATED" : "UNALLOCATED_REVIEW";
    const primaryKind = !primary ? "UNALLOCATED" : primary === registry.canonical_repo ? "CHRONICA_NATIVE" : "DEVELOPMENT_CELL";
    const primaryCell = primaryKind === "DEVELOPMENT_CELL" ? cells.get(primary) : null;
    const secondaryPackages = uniqueCandidates.slice(1).map((repo) => {
      if (repo === registry.canonical_repo) return { repo, cell_id: null, package_id: null, kind: "CHRONICA_NATIVE" };
      const cell = cells.get(repo);
      return {
        repo,
        cell_id: cell?.cell_id ?? null,
        package_id: cell?.package_id ?? null,
        kind: "DEVELOPMENT_CELL",
      };
    });
    const sourceRelocation = !donor.source_present
      ? "SOURCE_NOT_PRESENT"
      : primaryKind === "DEVELOPMENT_CELL"
        ? "RELOCATION_REQUIRED"
        : "NOT_APPLICABLE";

    const revisionMatch = String(donor.notes ?? "").match(/upstream_sha=([0-9a-f]{40})/);
    const sourceRevisionHint = revisionMatch?.[1] ?? null;
    const needed = donor.needed === true
      ? "YES"
      : donor.needed === false
        ? "NO"
        : String(donor.needed ?? "UNKNOWN").toUpperCase();

    allocations.push({
      donor_id: donor.id,
      repo: donor.repo,
      status: donor.status,
      needed,
      why_needed: donor.why_needed ?? null,
      source_present: Boolean(donor.source_present),
      source_path: donor.temporary_path ?? null,
      source_revision_hint: sourceRevisionHint,
      canonical_upstream: donor.canonical_upstream,
      primary_absorber: primary,
      primary_kind: primaryKind,
      primary_cell_id: primaryCell?.cell_id ?? null,
      primary_package_id: primaryCell?.package_id ?? null,
      secondary_consumers: uniqueCandidates.slice(1),
      secondary_consumer_packages: secondaryPackages,
      basis,
      allocation_state: allocationState,
      source_relocation: sourceRelocation,
    });
  }

  const ids = new Set(allocations.map((row) => row.donor_id));
  if (ids.size !== checklist.donors.length || allocations.length !== checklist.donors.length) {
    throw new Error(`closed-world donor coverage mismatch: corpus=${checklist.donors.length} allocations=${allocations.length} unique=${ids.size}`);
  }

  const unallocated = allocations.filter((row) => row.primary_kind === "UNALLOCATED").length;
  const report = {
    schema: "chronica.system-atlas.fleet-donor-allocation.v1",
    generated_at: now,
    canonical_repo: registry.canonical_repo,
    donor_corpus_total: checklist.donors.length,
    allocations_total: allocations.length,
    unallocated_total: unallocated,
    skipped_total: checklist.donors.length - allocations.length,
    coverage_complete: unallocated === 0 && allocations.length === checklist.donors.length,
    development_cells_total: registry.development_cells.filter((row) => row.active !== false).length,
    packages_total: new Set(registry.development_cells.filter((row) => row.active !== false).map((row) => row.package_id)).size,
    // Compatibility counters retained for existing consumers during the naming migration.
    ops_targets_total: registry.development_cells.filter((row) => row.active !== false).length,
    primary_development_cell_total: allocations.filter((row) => row.primary_kind === "DEVELOPMENT_CELL").length,
    primary_ops_total: allocations.filter((row) => row.primary_kind === "DEVELOPMENT_CELL").length,
    primary_chronica_total: allocations.filter((row) => row.primary_kind === "CHRONICA_NATIVE").length,
    source_relocation_required_total: allocations.filter((row) => row.source_relocation === "RELOCATION_REQUIRED").length,
    allocations,
  };
  validateInstance(report, "chronica.system-atlas.fleet-donor-allocation.v1");
  if (!report.coverage_complete) throw new Error(`fleet donor allocation is incomplete: unallocated=${report.unallocated_total} skipped=${report.skipped_total}`);
  return report;
}

function argValue(args, name) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : null;
}

function main(args = process.argv.slice(2)) {
  const registryPath = argValue(args, "--registry");
  const out = argValue(args, "--out");
  const report = allocateDonors({ registry: registryPath ? loadFleetRegistry(resolve(registryPath)) : loadFleetRegistry() });
  const text = `${JSON.stringify(report, null, 2)}\n`;
  if (out) writeFileSync(resolve(out), text);
  else process.stdout.write(text);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { console.error(`fleet donor allocation failed: ${error.message}`); process.exit(1); }
}
