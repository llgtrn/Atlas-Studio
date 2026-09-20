#!/usr/bin/env node
/**
 * Refoundation graph tools.
 *
 * These commands intentionally stay small and local: they validate the new
 * capability-home descriptors and domain aggregates, print a census, and map
 * changed files back to the graph without creating durable report forests.
 */

import { execFileSync } from "node:child_process";
import {
  existsSync,
  readdirSync,
  readFileSync,
  statSync,
} from "node:fs";
import { basename, dirname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import {
  findCapCrateDirs,
  validateCapabilityDescriptor,
} from "./validate-capability-descriptors.mjs";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

const DOMAIN_STATUS_ENUM = new Set([
  "REFOUNDATION_ACTIVE",
  "CURRENT_REALITY",
  "INTENDED_FUTURE_CONTRACT",
  "MIXED_REQUIRES_SPLIT",
]);

const RELATION_TYPE_ENUM = new Set([
  "REQUIRES",
  "CALLS",
  "READS_FROM",
  "PRODUCES",
  "CONSUMES",
  "AUTHORIZES_THROUGH",
  "PERSISTS_TO",
  "PROJECTS_TO",
  "NOTIFIES",
  "SCHEDULES",
  "TRIGGERS",
  "EXPORTS",
  "IMPORTS",
  "RENDERS_IN",
  "SUPERSEDES",
  "COMPOSES_WITH",
]);

const ID_DOMAIN_PATTERN = /^[a-z0-9._-]+$/;

function rel(path) {
  return relative(REPO_ROOT, path).split(sep).join("/");
}

function readJson(path, errors) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (err) {
    errors.push(`${rel(path)}: invalid JSON (${err.message})`);
    return null;
  }
}

function isStringList(value) {
  return Array.isArray(value)
    && value.every((item) => typeof item === "string" && item.length > 0)
    && new Set(value).size === value.length;
}

function listDomainFiles() {
  const root = resolve(REPO_ROOT, "docs/domains");
  if (!existsSync(root)) return [];
  return readdirSync(root)
    .map((name) => resolve(root, name, "DOMAIN.chronica.json"))
    .filter((path) => existsSync(path));
}

function loadCapabilities() {
  const errors = [];
  const descriptors = [];
  const seen = new Map();
  const capRoot = resolve(REPO_ROOT, "crates/cap");

  for (const dir of findCapCrateDirs(capRoot)) {
    const crateName = basename(dir);
    const descriptorPath = resolve(dir, ".chronica/capability.chronica.json");
    if (!existsSync(descriptorPath)) {
      errors.push(`${rel(dir)}: missing .chronica/capability.chronica.json`);
      continue;
    }
    const parsed = readJson(descriptorPath, errors);
    if (!parsed) continue;
    const schemaErrors = validateCapabilityDescriptor(parsed, rel(descriptorPath));
    errors.push(...schemaErrors);
    const id = parsed.capability?.id;
    if (typeof id === "string") {
      if (seen.has(id)) {
        errors.push(`${rel(descriptorPath)}: duplicate capability id "${id}" also used by ${seen.get(id)}`);
      } else {
        seen.set(id, rel(descriptorPath));
      }
    }
    descriptors.push({
      crateName,
      dir,
      path: descriptorPath,
      relPath: rel(descriptorPath),
      descriptor: parsed,
    });
  }
  return { descriptors, errors };
}

function validateDomainAggregate(domain, path) {
  const errors = [];
  const allowedTop = new Set([
    "schema",
    "domain",
    "capabilities",
    "relationships",
    "major_flows",
    "external_boundaries",
    "shared_substrate",
    "invariants",
    "known_structural_gaps",
  ]);
  const requiredTop = [
    "schema",
    "domain",
    "capabilities",
    "relationships",
    "invariants",
    "known_structural_gaps",
  ];

  if (typeof domain !== "object" || domain === null || Array.isArray(domain)) {
    return [`${rel(path)}: domain aggregate must be a JSON object`];
  }
  for (const key of requiredTop) {
    if (domain[key] === undefined) errors.push(`${rel(path)}: missing required top-level field "${key}"`);
  }
  for (const key of Object.keys(domain)) {
    if (!allowedTop.has(key)) errors.push(`${rel(path)}: unexpected top-level field "${key}"`);
  }
  if (domain.schema !== "chronica.refoundation.domain.v1") {
    errors.push(`${rel(path)}: schema must equal "chronica.refoundation.domain.v1"`);
  }

  const meta = domain.domain;
  if (typeof meta !== "object" || meta === null || Array.isArray(meta)) {
    errors.push(`${rel(path)}: domain must be an object`);
  } else {
    const allowedMeta = new Set(["id", "name", "current_status", "owner_notes"]);
    for (const key of ["id", "name", "current_status"]) {
      if (meta[key] === undefined) errors.push(`${rel(path)}: domain.${key} is required`);
    }
    for (const key of Object.keys(meta)) {
      if (!allowedMeta.has(key)) errors.push(`${rel(path)}: unexpected field "domain.${key}"`);
    }
    if (typeof meta.id !== "string" || !ID_DOMAIN_PATTERN.test(meta.id)) {
      errors.push(`${rel(path)}: domain.id must match ^[a-z0-9._-]+$`);
    }
    if (typeof meta.name !== "string" || meta.name.length === 0) {
      errors.push(`${rel(path)}: domain.name must be a non-empty string`);
    }
    if (!DOMAIN_STATUS_ENUM.has(meta.current_status)) {
      errors.push(`${rel(path)}: domain.current_status "${meta.current_status}" is not valid`);
    }
  }

  for (const key of [
    "capabilities",
    "major_flows",
    "external_boundaries",
    "shared_substrate",
    "invariants",
    "known_structural_gaps",
  ]) {
    if (domain[key] !== undefined && !isStringList(domain[key])) {
      errors.push(`${rel(path)}: ${key} must be an array of unique non-empty strings`);
    }
  }

  if (!Array.isArray(domain.relationships)) {
    errors.push(`${rel(path)}: relationships must be an array`);
  } else {
    domain.relationships.forEach((edge, index) => {
      if (typeof edge !== "object" || edge === null || Array.isArray(edge)) {
        errors.push(`${rel(path)}: relationships[${index}] must be an object`);
        return;
      }
      const allowedEdge = new Set(["from", "type", "to", "evidence"]);
      for (const key of ["from", "type", "to"]) {
        if (edge[key] === undefined) errors.push(`${rel(path)}: relationships[${index}].${key} is required`);
      }
      for (const key of Object.keys(edge)) {
        if (!allowedEdge.has(key)) errors.push(`${rel(path)}: unexpected field "relationships[${index}].${key}"`);
      }
      if (typeof edge.from !== "string" || edge.from.length === 0) {
        errors.push(`${rel(path)}: relationships[${index}].from must be a non-empty string`);
      }
      if (typeof edge.to !== "string" || edge.to.length === 0) {
        errors.push(`${rel(path)}: relationships[${index}].to must be a non-empty string`);
      }
      if (!RELATION_TYPE_ENUM.has(edge.type)) {
        errors.push(`${rel(path)}: relationships[${index}].type "${edge.type}" is not valid`);
      }
      if (edge.evidence !== undefined && typeof edge.evidence !== "string") {
        errors.push(`${rel(path)}: relationships[${index}].evidence must be a string`);
      }
    });
  }
  return errors;
}

function loadDomains() {
  const errors = [];
  const domains = [];
  const seen = new Map();
  for (const path of listDomainFiles()) {
    const parsed = readJson(path, errors);
    if (!parsed) continue;
    errors.push(...validateDomainAggregate(parsed, path));
    const id = parsed.domain?.id;
    if (typeof id === "string") {
      if (seen.has(id)) {
        errors.push(`${rel(path)}: duplicate domain id "${id}" also used by ${seen.get(id)}`);
      } else {
        seen.set(id, rel(path));
      }
    }
    domains.push({ id, path, relPath: rel(path), aggregate: parsed });
  }
  return { domains, errors };
}

function graphSnapshot() {
  const caps = loadCapabilities();
  const domains = loadDomains();
  return {
    descriptors: caps.descriptors,
    domains: domains.domains,
    errors: [...caps.errors, ...domains.errors],
  };
}

function printCensus(json) {
  const snapshot = graphSnapshot();
  const byDomain = new Map();
  const byMaturity = new Map();
  for (const cap of snapshot.descriptors) {
    const domain = cap.descriptor.capability?.domain ?? "UNKNOWN";
    const maturity = cap.descriptor.capability?.maturity ?? "UNKNOWN";
    byDomain.set(domain, (byDomain.get(domain) ?? 0) + 1);
    byMaturity.set(maturity, (byMaturity.get(maturity) ?? 0) + 1);
  }
  const payload = {
    cap_crates: snapshot.descriptors.length,
    domain_aggregates: snapshot.domains.length,
    errors: snapshot.errors,
    by_domain: Object.fromEntries([...byDomain.entries()].sort()),
    by_maturity: Object.fromEntries([...byMaturity.entries()].sort()),
  };
  if (json) {
    console.log(JSON.stringify(payload, null, 2));
  } else {
    console.log(`cap crates with descriptors: ${payload.cap_crates}`);
    console.log(`domain aggregates: ${payload.domain_aggregates}`);
    for (const [domain, count] of Object.entries(payload.by_domain)) {
      console.log(`  ${domain}: ${count}`);
    }
    for (const error of snapshot.errors) console.error(`ERROR: ${error}`);
  }
  return snapshot.errors.length === 0 ? 0 : 1;
}

function validateGraph() {
  const snapshot = graphSnapshot();
  if (snapshot.errors.length === 0) {
    console.log(`refoundation graph valid: ${snapshot.descriptors.length} capability descriptor(s), ${snapshot.domains.length} domain aggregate(s).`);
    return 0;
  }
  for (const error of snapshot.errors) console.error(`ERROR: ${error}`);
  return 1;
}

function changedFiles(args) {
  const passthrough = args.indexOf("--");
  if (passthrough >= 0) return args.slice(passthrough + 1);
  const baseIndex = args.indexOf("--base");
  if (baseIndex >= 0 && args[baseIndex + 1]) {
    const out = execFileSync("git", ["diff", "--name-only", `${args[baseIndex + 1]}...HEAD`], {
      cwd: REPO_ROOT,
      encoding: "utf8",
    });
    return out.split(/\r?\n/).filter(Boolean);
  }
  const out = execFileSync("git", ["diff", "--name-only", "--cached", "--"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });
  return out.split(/\r?\n/).filter(Boolean);
}

function affectsDescriptor(file, cap) {
  const descriptor = cap.descriptor;
  const needles = [
    rel(cap.dir),
    descriptor.ownership?.implementation_home,
    ...(descriptor.ownership?.caller_entries ?? []),
    ...(descriptor.ownership?.runtime_owners ?? []),
    ...(descriptor.relations?.ui_surfaces ?? []),
    ...(descriptor.knowledge?.covered_sources ?? []),
    ...(descriptor.verification?.integration_surfaces ?? []),
  ].filter(Boolean);
  return needles.some((needle) => file === needle || file.startsWith(`${needle}/`));
}

function printAffected(args) {
  const files = changedFiles(args).map((file) => file.split("\\").join("/"));
  const snapshot = graphSnapshot();
  const capabilities = snapshot.descriptors
    .filter((cap) => files.some((file) => affectsDescriptor(file, cap)))
    .map((cap) => ({
      id: cap.descriptor.capability?.id,
      domain: cap.descriptor.capability?.domain,
      maturity: cap.descriptor.capability?.maturity,
      descriptor: cap.relPath,
    }));
  const domains = snapshot.domains
    .filter((domain) => files.includes(domain.relPath))
    .map((domain) => ({ id: domain.id, descriptor: domain.relPath }));
  const FILE_LIST_LIMIT = 50;
  const CAPABILITY_LIST_LIMIT = 50;
  console.log(JSON.stringify({
    file_count: files.length,
    ...(files.length <= FILE_LIST_LIMIT
      ? { files }
      : {
          files_truncated: true,
          sample_files: files.slice(0, FILE_LIST_LIMIT),
        }),
    capability_count: capabilities.length,
    ...(capabilities.length <= CAPABILITY_LIST_LIMIT
      ? { capabilities }
      : {
          capabilities_truncated: true,
          sample_capabilities: capabilities.slice(0, CAPABILITY_LIST_LIMIT),
        }),
    domain_count: domains.length,
    domains,
    graph_errors: snapshot.errors,
    suggested_commands: [
      "pnpm refoundation:validate-graph",
      "node tools/refoundation/validate-capability-descriptors.mjs",
    ],
  }, null, 2));
  return snapshot.errors.length === 0 ? 0 : 1;
}

function main() {
  const [command, ...args] = process.argv.slice(2);
  if (command === "census") return printCensus(args.includes("--json"));
  if (command === "validate") return validateGraph();
  if (command === "affected") return printAffected(args);
  console.error("usage: node tools/refoundation/refoundation-graph.mjs <census|validate|affected> [--json] [--base <ref>] [-- <files...>]");
  return 2;
}

process.exit(main());
