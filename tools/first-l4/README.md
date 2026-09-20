# First-L4 strike harness

The one action the entire Planning Brain converged on: drive a real human through
`login → propose → board-approve → execute → audit` so the **Reality Ratio lifts off 0.00%**.
Everything an agent can prepare is here. The two things only a human can do are marked 👤.

## The fast path needs NO prod IdP
The dev-auth bridge (`crates/chronica-api/src/dev_auth.rs`, verified) lets a real person authenticate
against the **live, unchanged** Signed-mode verifier locally — L1 disjoint key + L2 prod-abort make it
safe. So the first L4-candidate does **not** wait on the prod-OIDC escalation; it can happen today.

## Steps
1. dev Postgres up on `127.0.0.1:55432` (the Rust kernel's dev DB).
2. seed the company: `psql "postgres://chronica:chronica@127.0.0.1:55432/chronica" -f tools/first-l4/seed.sql`
3. boot the server with dev-auth (boot ABORTS if `CHRONICA_ENV=production` — invariant L2):
   ```
   CHRONICA_DEV_AUTH=1 CHRONICA_ENV=dev \
   DATABASE_URL=postgres://chronica:chronica@127.0.0.1:55432/chronica \
   cargo run -p chronica-api --features db
   ```
4. note the address it logs; `export BASE_URL=http://127.0.0.1:<port>`.
5. 👤 **run the chain as a real person** (requester ≠ board, SoD):
   ```
   REQ_ACTOR="<your real name>" BOARD_ACTOR="<a real board member>" bash tools/first-l4/strike.sh
   ```
   👤 the login + the $1 money action + the board approval are the **real human actions** — the point of L4.
   The flow is code-verified (companies.rs `money_action_pre_approved` + approvals.rs `create_approval`/`approve_approval`):
   requester **creates** a bound `money_action` approval → **board approves** it (SoD) → requester posts the
   money action **with** `approvalRequestId`. The money-action route does NOT mint the approval — it consumes a
   board-approved, bound, single-use one. Fully code-verified, runs verbatim: the create response `.id` field
   (DecisionLedgerRow `to_redacted_json`) and the approve body (`resolve_decision` reads only an optional
   `decisionNote`; SoD enforced — board `actor_id` must differ from the requester) — no first-run confirmation needed.
6. when a `finance_event` appears (the audit row), tell the loop / agent. It will then, and only then:
   - add the reality_events to `tools/capabilities/reality-events.json` (honest tier: L4-candidate dev run,
     or `L4_production` only if it was real production data),
   - set the matching `reality_milestone` statuses → `done` in `reality-path.json`,
   - resolve the matching predictions in `predictions.json`,
   - `pnpm caps:rebuild` → the **Reality Ratio lifts off 0** and the Planning Freeze re-evaluates.

## Portable path — NO psql, NO jq (Windows dev box / minimal envs)
`strike.sh` needs `psql` + `jq`. When those are absent, use the zero-dependency Node driver instead
(Node >=18 global `fetch`). This was used to drive the chain live end-to-end on 2026-06-20.

1. dev Postgres up on `127.0.0.1:55432`.
2. seed the company WITHOUT psql — any Node `postgres`/`pg` client works; the whole seed is one upsert:
   ```js
   // node seed.cjs   (point the require at any installed `postgres` client)
   const postgres = require("<path>/node_modules/postgres/cjs/src/index.js");
   const sql = postgres("postgres://chronica:chronica@127.0.0.1:55432/chronica", { max: 1, onnotice: () => {} });
   await sql`INSERT INTO companies (id, name, account_id, status)
     VALUES ('f1f1f1f1-0000-4000-8000-000000000001'::uuid, 'First-L4 Strike Co', 'acct-first-l4', 'active')
     ON CONFLICT (id) DO UPDATE SET account_id = EXCLUDED.account_id, name = EXCLUDED.name, status = 'active'`;
   await sql.end();
   ```
3. boot the server with dev-auth (the prebuilt `target/debug/chronica-api.exe` already has the `db` feature):
   ```
   CHRONICA_DEV_AUTH=1 CHRONICA_ENV=dev CHRONICA_API_BIND=127.0.0.1:8787 \
   DATABASE_URL=postgres://chronica:chronica@127.0.0.1:55432/chronica ./target/debug/chronica-api.exe
   ```
4. drive the chain (zero-dep): `node tools/first-l4/drive.mjs`
   - prints each step; exits 0 only if `committed=true, replay-deduped=true, finance_event=true`.
   - synthetic identities (default) => **L3_synthetic**. For a real L4, set `REQ_ACTOR` + `BOARD_ACTOR`
     to two real people and arm a real money provider — see the escalation in the loop, NEVER hand-claim L4.

## Money-safety is unchanged
This exercises the same audited gate (F1/F2/F1.b). A board principal equal to the requester is rejected (SoD).
Nothing here weakens the gate; a human simply runs it for real for the first time.
