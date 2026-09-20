#!/usr/bin/env node
// Computes ground-truth audit-chain hashes using the SAME algorithm as
// packages/runtime/src/audit-chain.ts (sha256-merkle-chain-v1), so the Rust
// port can be tested byte-for-byte against the canonical TS output.
// Reimplements the two pure helpers inline (sha256Hex + stableJsonStringify)
// exactly as registry-state.ts defines them — no TS build step needed.
import { createHash } from "node:crypto";

function sha256Hex(value) {
  return createHash("sha256").update(value).digest("hex");
}
function stableJsonStringify(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map((i) => stableJsonStringify(i)).join(",")}]`;
  const record = value;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableJsonStringify(record[key])}`)
    .join(",")}}`;
}
const GENESIS = "0".repeat(64);

function buildEntry(input) {
  const prevHash = input.prevHash ?? GENESIS;
  const payloadSha256 = sha256Hex(stableJsonStringify(input.payload ?? {}));
  const base = {
    seq: input.seq,
    timestamp: input.timestamp,
    subjectId: input.subjectId,
    action: input.action,
    detail: input.detail ?? "",
    outcome: input.outcome ?? "",
    payloadSha256,
    prevHash,
  };
  return { algorithm: "sha256-merkle-chain-v1", ...base, hash: sha256Hex(stableJsonStringify(base)) };
}

// A fixed 3-entry chain with deterministic content (no Date.now()).
const inputs = [
  { seq: 0, timestamp: "2026-01-01T00:00:00.000Z", subjectId: "company-1", action: "approval.created", detail: "spend_approval", outcome: "pending", payload: { amountCents: 49900, provider: "shopify" } },
  { seq: 1, timestamp: "2026-01-01T00:00:01.000Z", subjectId: "company-1", action: "approval.decided", detail: "board", outcome: "approved", payload: { decidedBy: "board", reversible: false } },
  { seq: 2, timestamp: "2026-01-01T00:00:02.000Z", subjectId: "company-1", action: "ledger.post", detail: "supplier_order", outcome: "posted", payload: { amountCents: 49900 } },
];

const chain = [];
let prevHash = null;
for (const input of inputs) {
  const entry = buildEntry({ ...input, prevHash });
  chain.push(entry);
  prevHash = entry.hash;
}

console.log(JSON.stringify({ genesis: GENESIS, chain }, null, 2));
