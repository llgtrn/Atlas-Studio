#!/usr/bin/env node
// First-L4 chain driver — PORTABLE, ZERO-DEPENDENCY (Node >=18 global fetch; NO jq, NO psql, NO curl).
// Companion to strike.sh, which needs jq+psql that are absent on some boxes (e.g. this Windows dev box).
//
// Drives the exact code-verified chain (companies.rs money_action_pre_approved + approvals.rs
// create/approve), in order:
//   dev-login requester  ->  create BOUND money_action approval  ->  board (DIFFERENT actor) approves [SoD]
//   ->  post money-action WITH approvalRequestId (commit exactly-once)  ->  repost same nonce (single-use
//   check)  ->  GET erp-events (the Merkle-audited finance_event).
//
// HONESTY: synthetic principals + synthetic $ + a seeded company = an L3_synthetic runtime confirmation,
// NOT L4. L4_production is EARNED only when a REAL human authorizes a REAL money action with REAL data
// (LiveMoney armed + a real provider). This script must run with LiveMoney NOT armed — it writes the
// internal finance_event + audit row through the audited gate; it moves NO real funds.
//
// Prereqs (see README.md): dev Postgres up on :55432, the company seeded (README shows the psql-free node
// seed), and the server booted with dev-auth:
//   CHRONICA_DEV_AUTH=1 CHRONICA_ENV=dev CHRONICA_API_BIND=127.0.0.1:8787 \
//   DATABASE_URL=postgres://chronica:chronica@127.0.0.1:55432/chronica ./target/debug/chronica-api.exe
// Then: node tools/first-l4/drive.mjs
//
// Env (all optional; defaults are the seeded synthetic identities):
//   BASE_URL (default http://127.0.0.1:8787), CID (default the seeded company),
//   ACCT (default acct-first-l4), REQ_ACTOR (default l4-requester), BOARD_ACTOR (default l4-board).
// For a REAL L4, a human sets REQ_ACTOR/BOARD_ACTOR to two real people and arms a real money provider.

const BASE = process.env.BASE_URL || "http://127.0.0.1:8787";
const CID = process.env.CID || "f1f1f1f1-0000-4000-8000-000000000001";
const ACCT = process.env.ACCT || "acct-first-l4";
const REQ_ACTOR = process.env.REQ_ACTOR || "l4-requester";
const BOARD_ACTOR = process.env.BOARD_ACTOR || "l4-board";
const NONCE = "first-l4-" + Date.now();

if (REQ_ACTOR === BOARD_ACTOR) {
  console.error("REQ_ACTOR and BOARD_ACTOR must DIFFER (Separation of Duties).");
  process.exit(1);
}

async function call(method, path, { token, body } = {}) {
  const headers = {};
  if (body !== undefined) headers["content-type"] = "application/json";
  if (token) headers.authorization = "Bearer " + token;
  const res = await fetch(BASE + path, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let json;
  try { json = JSON.parse(text); } catch { json = text; }
  return { status: res.status, json };
}
const post = (path, opts) => call("POST", path, opts);
const get = (path, opts) => call("GET", path, opts);
const log = (label, r) => console.log(`\n[${r.status}] ${label}\n` + JSON.stringify(r.json));
const need = (v, msg) => { if (!v) { console.error(msg); process.exit(2); } return v; };

(async () => {
  console.log(`first-l4 drive: BASE=${BASE} CID=${CID} requester=${REQ_ACTOR} board=${BOARD_ACTOR}`);

  // 1) requester logs in
  const u = await post("/api/dev/login", { body: { account_id: ACCT, actor_id: REQ_ACTOR, actor_type: "user" } });
  log("1. dev-login requester", u);
  const TOK_U = need(u.json && u.json.token, "no requester token (is the server booted with CHRONICA_DEV_AUTH=1?)");

  // 2) requester creates the BOUND money_action approval
  const ap = await post(`/api/companies/${CID}/approvals`, {
    token: TOK_U,
    body: { type: "money_action", payload: { provider: "stripe", operation: "payout", reversible: false, maxCostCents: 100 } },
  });
  log("2. create bound money_action approval", ap);
  const AID = need(ap.json && ap.json.id, "no approval id (is the company seeded + owned by this account?)");

  // 3) board (DIFFERENT actor => SoD) logs in and approves
  const b = await post("/api/dev/login", { body: { account_id: ACCT, actor_id: BOARD_ACTOR, actor_type: "board" } });
  log("3a. dev-login board", b);
  const TOK_B = need(b.json && b.json.token, "no board token");
  const appr = await post(`/api/approvals/${AID}/approve`, { token: TOK_B, body: { decisionNote: "first-L4 strike (drive.mjs)" } });
  log("3b. board approves (SoD: board != requester)", appr);

  // 4) requester posts the $1 money action WITH the approval => commit exactly-once + Merkle audit
  const ma = await post(`/api/companies/${CID}/money-actions`, {
    token: TOK_U,
    body: { provider: "stripe", operation: "payout", estimatedCostCents: 100, reversible: false, nonce: NONCE, approvalRequestId: AID },
  });
  log("4. post money-action (commit)", ma);

  // 4b) idempotency / single-use: repost the SAME nonce+approval => must dedupe, not double-commit
  const ma2 = await post(`/api/companies/${CID}/money-actions`, {
    token: TOK_U,
    body: { provider: "stripe", operation: "payout", estimatedCostCents: 100, reversible: false, nonce: NONCE, approvalRequestId: AID },
  });
  log("4b. repost same nonce (idempotency / single-use check)", ma2);

  // 5) read the committed finance_event = the AUDIT EVIDENCE
  const ev = await get(`/api/companies/${CID}/erp-events`, { token: TOK_U });
  log("5. erp-events (audit evidence)", ev);

  const committed = ma.json && ma.json.status === "committed";
  const deduped = ma2.json && ma2.json.deduped === true;
  const hasEvent = ev.json && Array.isArray(ev.json.events) && ev.json.events.length > 0;
  console.log(`\n== chain ${committed && deduped && hasEvent ? "OK" : "INCOMPLETE"} ` +
    `(committed=${committed}, replay-deduped=${deduped}, finance_event=${hasEvent}) ==`);
  console.log("Tier: L3_synthetic (synthetic actors/$/seeded company). For L4, a real human runs this with " +
    "real identities + a real armed money provider. Do NOT hand-claim L4.");
  process.exit(committed && deduped && hasEvent ? 0 : 5);
})().catch((e) => { console.error("drive failed:", e && e.message); process.exit(9); });
