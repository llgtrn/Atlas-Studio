#!/usr/bin/env node
// Scaffolds the 18 chronica-* crate skeletons in dependency order
// (docs/architecture/03-rust-kernel-plan.md §6). Idempotent: only writes a
// crate's Cargo.toml/lib.rs if missing, so it never clobbers real implementation.
// Phase 1 brings every crate into `cargo build --workspace` as a compiling stub;
// later waves fill each one in. chronica-core and chronica-api are written by
// hand (real Phase-1 content) and are skipped here.
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const CRATES = join(ROOT, "crates");

// crate -> { deps: [workspace crate names], extra: [extra workspace.dependencies keys], doc }
// Edges follow §6 exactly. core/api handwritten -> not generated here.
const GRAPH = {
  "chronica-policy": {
    deps: ["chronica-core"],
    doc: "Policy engine: SideEffectAction classification + money-moving detection, BudgetPolicy evaluate/hard-stop/invocation-block, dollar-threshold bands, reversible/irreversible flag, SecretReference resolution.",
  },
  "chronica-approvals": {
    deps: ["chronica-core", "chronica-policy"],
    doc: "ApprovalPolicy/Request/Decision, approval gates, spend_approval + budget_override_required flows, dollar-threshold band evaluation, board-only decision.",
  },
  "chronica-artifacts": {
    deps: ["chronica-core"],
    doc: "Artifact registry, versioning, types, review, evidence links.",
  },
  "chronica-observability": {
    deps: ["chronica-core"],
    doc: "Traces, observations, tool/model call logs, CostRecord ingestion, AuditEvent chain store + verify, evals, quality scores.",
  },
  "chronica-tools": {
    deps: ["chronica-core", "chronica-policy"],
    doc: "Tool registry, capability manifests, MCP server, the single typed kernel<->browser-capability transport, capability-gated dispatch.",
  },
  "chronica-runtime": {
    deps: [
      "chronica-core",
      "chronica-policy",
      "chronica-approvals",
      "chronica-observability",
      "chronica-tools",
      "chronica-artifacts",
    ],
    extra: ["tokio"],
    doc: "RuntimeWorker/Run/Task, capability execution, tool dispatch, event bus, audit-chain write path, memory store, process/browser/MCP managers; runs the §10 money gate so no dispatch path skips policy.",
  },
  "chronica-scheduler": {
    deps: ["chronica-core", "chronica-runtime"],
    doc: "Schedules, cron, due-time calc, schedule delivery.",
  },
  "chronica-workflows": {
    deps: ["chronica-core", "chronica-runtime"],
    doc: "Workflow definitions/executions, durable execution, graph executor, node registry, triggers, signals, timers.",
  },
  "chronica-worker-supervisor": {
    deps: ["chronica-core", "chronica-tools", "chronica-runtime"],
    doc: "Capability-task lifecycle, heartbeat, supervision; launch/health/cancel of the one TS/Playwright browser capability.",
  },
  "chronica-company": {
    deps: ["chronica-core", "chronica-approvals", "chronica-artifacts"],
    doc: "CompanyModel/Template/Project, Department, AgentRole, Goal/KPI, model generator, specialist review graph, ERP/ledger/KPI backbone.",
  },
  "chronica-strategy": {
    deps: ["chronica-core", "chronica-approvals"],
    doc: "StrategyCanvas + hypotheses/segments/offers/channels/pricing/cost/risk/experiments/scoring/review/decision.",
  },
  "chronica-osint": {
    deps: ["chronica-core", "chronica-runtime"],
    doc: "Investigation, targets/entities/evidence/sources, collector runs, findings, confidence, profiles (Rust async capabilities, no Python worker).",
  },
  "chronica-internet-hand": {
    deps: ["chronica-core", "chronica-policy", "chronica-approvals", "chronica-tools"],
    doc: "InternetTask/Action/Workflow, browser profiles, site memory, evidence cards, replay; read-only nav runs freely, any side effect is a gated SideEffectAction; drives the one TS/Playwright capability via chronica-tools.",
  },
  "chronica-memory": {
    deps: ["chronica-core"],
    doc: "Memory model, datasets, RAG/retrieval, chunking, memory scope.",
  },
  "chronica-integrations": {
    deps: ["chronica-core", "chronica-policy", "chronica-approvals", "chronica-observability"],
    doc: "The universal connector framework — one provider-agnostic integration hub where `provider` is an open string; money-committing connector actions are gated SideEffectActions; secrets resolved as SecretReferences.",
  },
};

function cargoToml(name, { deps = [], extra = [] }) {
  const depLines = [
    ...deps.map((d) => `${d} = { workspace = true }`),
    ...extra.map((e) => `${e} = { workspace = true }`),
    `serde = { workspace = true }`,
    `serde_json = { workspace = true }`,
    `thiserror = { workspace = true }`,
  ];
  return `[package]
name = "${name}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
${depLines.join("\n")}
`;
}

function libRs(name, doc) {
  const ident = name.replace(/-/g, "_");
  return `//! \`${name}\` — ${doc}
//!
//! Phase-1 skeleton (docs/implementation/02-phase-1-rust-kernel.md). This crate
//! compiles in \`cargo build --workspace\` and is filled in by its dependency wave
//! per docs/architecture/03-rust-kernel-plan.md §6. The parity gate
//! (\`pnpm parity:check\`) decides when each capability is \`verified\`.

/// Crate identity marker; replaced by real types as this crate's wave lands.
pub const CRATE_NAME: &str = "${name}";

#[cfg(test)]
mod tests {
    #[test]
    fn ${ident}_skeleton_compiles() {
        assert_eq!(super::CRATE_NAME, "${name}");
    }
}
`;
}

let created = 0;
for (const [name, spec] of Object.entries(GRAPH)) {
  const dir = join(CRATES, name);
  const srcDir = join(dir, "src");
  const cargoPath = join(dir, "Cargo.toml");
  const libPath = join(srcDir, "lib.rs");
  mkdirSync(srcDir, { recursive: true });
  if (!existsSync(cargoPath)) {
    writeFileSync(cargoPath, cargoToml(name, spec));
    created++;
  }
  if (!existsSync(libPath)) {
    writeFileSync(libPath, libRs(name, spec.doc));
    created++;
  }
}
console.log(`scaffold-crates: wrote ${created} file(s) for ${Object.keys(GRAPH).length} crates`);
