-- Chronica DEMO HOLDING seed — DEV / fresh-deploy ONLY. Idempotent (re-runnable).
--
-- WHY: a freshly migrated Chronica DB is empty, so the feed-first SPA renders a blank
-- /feed and /holding/* (deny-by-default routes return zero rows for an account with no
-- companies). This seed plants ONE demo account + a few companies + agents + a handful of
-- TYPED feed posts so a deployed MVP shows a non-empty, lifelike operational feed — including
-- ONE pending board decision so the inline Approve/Reject control is visible.
--
-- GROUNDED IN THE LIVE SCHEMA (crates/chronica-core/migrations):
--   * 0010 — companies.account_id is the tenant column (TEXT; there is NO `accounts` table;
--            an account id is an external tenant string like the dev-login `account_id`).
--   * 0014 — decision_ledger (routed brief; the pending-board-decision row).
--   * 0000 — companies, agents, approvals, finance_events, runtime_workers, runtime_runs.
--   * 0017 — post_comments  (feed replies).
--   * 0018 — conversations + messages (Messenger).
-- The feed/holding routes are scoped by company_id and the holding union joins
-- company -> account_id, so planting rows under demo-account companies makes the
-- account-scoped, deny-by-default routes return rows for `acct-demo-holding`.
--
-- MONEY SAFETY (UNARMED): this seed writes NO money_commitments row and creates NO money
-- approval-consumption. `finance_events` here is DISPLAY/AUDIT data only (the same table the
-- read-only erp-events feed projects) — it authorizes nothing and moves no funds. The ONE
-- money gate (commit_money_action) and board approval stay the sole authorities, untouched.
--
-- IDEMPOTENT: every row uses a FIXED uuid and the whole seed runs in ONE transaction that
-- first DELETEs all demo-account data (child -> parent FK order) and re-inserts it. Safe to
-- run on every deploy. Re-running NEVER duplicates and NEVER mutates non-demo tenants.
--
-- APPLY (psql):
--   psql "postgres://chronica:chronica@127.0.0.1:55432/chronica" -f tools/seed-demo-holding.sql
-- or, psql-free:
--   node tools/seed-demo-holding.mjs
--
-- VERIFY: the COUNT at the very end prints the seeded row totals per source.

BEGIN;

-- ── Fixed identifiers (stable across re-runs) ──────────────────────────────────────────────
-- Demo tenant (account_id is a plain string; mirrors POST /api/dev/login `account_id`).
--   account_id = 'acct-demo-holding'
-- Companies (the FB-groups in the two-level SNS nav).
--   c1 Northwind Robotics   d11d0001-...   c2 Acme Analytics  d11d0002-...   c3 Tidewater Foods d11d0003-...

-- 1) PURGE all prior demo-account data, child -> parent, so the re-insert is clean.
--    Scoped strictly to companies owned by the demo account => other tenants are untouched.
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM messages       WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM conversations  WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM post_comments  WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM finance_events WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM decision_ledger WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM approvals      WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM runtime_runs   WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM runtime_workers WHERE company_id IN (SELECT id FROM demo);
WITH demo AS (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
DELETE FROM agents         WHERE company_id IN (SELECT id FROM demo);

-- 2) COMPANIES (account-scoped). issue_prefix is unique-ish per company; all other columns
--    have safe defaults from 0000. account_id is bound EXPLICITLY (the real tenant path).
INSERT INTO companies (id, name, description, status, issue_prefix, account_id, brand_color) VALUES
  ('d11d0001-0000-4000-8000-000000000001'::uuid, 'Northwind Robotics', 'Autonomous warehouse robotics holding.', 'active', 'NWR', 'acct-demo-holding', '#2563eb'),
  ('d11d0002-0000-4000-8000-000000000002'::uuid, 'Acme Analytics',     'B2B analytics & dashboards.',           'active', 'ACM', 'acct-demo-holding', '#16a34a'),
  ('d11d0003-0000-4000-8000-000000000003'::uuid, 'Tidewater Foods',    'DTC specialty foods brand.',            'active', 'TWF', 'acct-demo-holding', '#db2777')
ON CONFLICT (id) DO UPDATE
  SET name = EXCLUDED.name, description = EXCLUDED.description, status = 'active',
      account_id = EXCLUDED.account_id, brand_color = EXCLUDED.brand_color,
      issue_prefix = EXCLUDED.issue_prefix, updated_at = now();

-- 3) AGENTS (a handful per company; the "Agent activity" rail + run attribution).
INSERT INTO agents (id, company_id, name, role, title, status) VALUES
  ('a9e70001-0000-4000-8000-000000000001'::uuid, 'd11d0001-0000-4000-8000-000000000001'::uuid, 'Atlas',   'cfo',       'Chief Financial Agent', 'idle'),
  ('a9e70002-0000-4000-8000-000000000002'::uuid, 'd11d0001-0000-4000-8000-000000000001'::uuid, 'Forge',   'engineer',  'Ops Engineer',          'running'),
  ('a9e70003-0000-4000-8000-000000000003'::uuid, 'd11d0002-0000-4000-8000-000000000002'::uuid, 'Beacon',  'analyst',   'Data Analyst',          'idle'),
  ('a9e70004-0000-4000-8000-000000000004'::uuid, 'd11d0002-0000-4000-8000-000000000002'::uuid, 'Quill',   'writer',    'Content Lead',          'idle'),
  ('a9e70005-0000-4000-8000-000000000005'::uuid, 'd11d0003-0000-4000-8000-000000000003'::uuid, 'Harbor',  'ops',       'Supply Coordinator',    'running');

-- 4) RUNTIME WORKERS (the live DB enforces runtime_runs.worker_id -> runtime_workers.id, so a
--    run needs a worker; one worker per company that has a run).
INSERT INTO runtime_workers (id, company_id, agent_id, name) VALUES
  ('700b0001-0000-4000-8000-000000000001'::uuid, 'd11d0001-0000-4000-8000-000000000001'::uuid, 'a9e70002-0000-4000-8000-000000000002'::uuid, 'nwr-worker'),
  ('700b0002-0000-4000-8000-000000000002'::uuid, 'd11d0002-0000-4000-8000-000000000002'::uuid, 'a9e70003-0000-4000-8000-000000000003'::uuid, 'acm-worker'),
  ('700b0003-0000-4000-8000-000000000003'::uuid, 'd11d0003-0000-4000-8000-000000000003'::uuid, 'a9e70005-0000-4000-8000-000000000005'::uuid, 'twf-worker');

-- 5) RUNTIME RUNS (run_update posts in the feed; metadata-only allow-list surfaces these).
INSERT INTO runtime_runs (id, company_id, worker_id, invocation_source, status, input_json, started_at, finished_at, created_at) VALUES
  ('5a4e0001-0000-4000-8000-000000000001'::uuid, 'd11d0001-0000-4000-8000-000000000001'::uuid, '700b0001-0000-4000-8000-000000000001'::uuid, 'schedule',  'running',   '{}'::jsonb, now() - interval '4 minutes',  NULL,                      now() - interval '4 minutes'),
  ('5a4e0002-0000-4000-8000-000000000002'::uuid, 'd11d0002-0000-4000-8000-000000000002'::uuid, '700b0002-0000-4000-8000-000000000002'::uuid, 'on_demand', 'succeeded', '{}'::jsonb, now() - interval '2 hours',    now() - interval '118 minutes', now() - interval '2 hours'),
  ('5a4e0003-0000-4000-8000-000000000003'::uuid, 'd11d0003-0000-4000-8000-000000000003'::uuid, '700b0003-0000-4000-8000-000000000003'::uuid, 'schedule',  'succeeded', '{}'::jsonb, now() - interval '1 day',       now() - interval '1 day' + interval '3 minutes', now() - interval '1 day');

-- 6) APPROVALS — ONE PENDING board decision (decidable inline Approve/Reject) + one already
--    APPROVED (a settled receipt in the feed). type 'decision_brief' = board-decidable.
INSERT INTO approvals (id, company_id, type, status, requested_by_user_id, payload) VALUES
  ('a99e0001-0000-4000-8000-000000000001'::uuid, 'd11d0001-0000-4000-8000-000000000001'::uuid, 'decision_brief', 'pending',  'agent-atlas',
     '{"summary":"Approve Q3 cloud-credit reallocation","recommendation":"go"}'::jsonb),
  ('a99e0002-0000-4000-8000-000000000002'::uuid, 'd11d0002-0000-4000-8000-000000000002'::uuid, 'decision_brief', 'approved', 'agent-beacon',
     '{"summary":"Publish the churn-analysis report","recommendation":"go"}'::jsonb);
UPDATE approvals
   SET decided_by_user_id = 'board-demo', decided_at = now() - interval '90 minutes',
       decision_note = 'Approved — ship it.'
 WHERE id = 'a99e0002-0000-4000-8000-000000000002'::uuid;

-- 7) DECISION LEDGER — the feed's brief posts. The FIRST row is the PENDING BOARD DECISION:
--    status 'routed' + gate_verdict 'RequiresApproval' linked to the PENDING approval above
--    => renders inline Approve/Reject. The SECOND links to the approved one (a settled receipt).
INSERT INTO decision_ledger
  (id, decision_key, company_id, brief_fingerprint, status, approval_id, requested_by,
   amount_cents, verdict, gate_verdict, routed_layer, snapshot, created_at) VALUES
  ('1ed90001-0000-4000-8000-000000000001'::uuid,
     'd11d0001-0000-4000-8000-000000000001:demo:pending-board',
     'd11d0001-0000-4000-8000-000000000001'::uuid, 'fp-demo-pending-board', 'routed',
     'a99e0001-0000-4000-8000-000000000001'::uuid, 'agent-atlas', 250000,
     'RequiresApproval', 'RequiresApproval', 'Board',
     '{"brief":{"decision":"go","title":"Q3 cloud-credit reallocation"},"facts":{"amountCents":250000}}'::jsonb,
     now() - interval '20 minutes'),
  ('1ed90002-0000-4000-8000-000000000002'::uuid,
     'd11d0002-0000-4000-8000-000000000002:demo:approved-receipt',
     'd11d0002-0000-4000-8000-000000000002'::uuid, 'fp-demo-approved-receipt', 'routed',
     'a99e0002-0000-4000-8000-000000000002'::uuid, 'agent-beacon', 0,
     'RequiresApproval', 'RequiresApproval', 'Board',
     '{"brief":{"decision":"go","title":"Publish churn-analysis report"},"facts":{"amountCents":0}}'::jsonb,
     now() - interval '2 hours');

-- 8) FINANCE EVENTS — DISPLAY/AUDIT ROWS ONLY (erp-event feed). Moves NO money; no
--    money_commitments. occurred_at is NOT NULL. direction/biller/event_kind are required.
INSERT INTO finance_events
  (id, company_id, event_kind, direction, biller, provider, amount_cents, currency, estimated,
   description, occurred_at) VALUES
  ('f1a40001-0000-4000-8000-000000000001'::uuid, 'd11d0002-0000-4000-8000-000000000002'::uuid,
     'subscription', 'credit', 'stripe', 'stripe',  4900, 'USD', false, 'Pro plan — monthly', now() - interval '3 hours'),
  ('f1a40002-0000-4000-8000-000000000002'::uuid, 'd11d0003-0000-4000-8000-000000000003'::uuid,
     'cogs',         'debit',  'internal', NULL,    12000, 'USD', false, 'Ingredient restock', now() - interval '6 hours');

-- 9) CONVERSATIONS + MESSAGES — Messenger thread (kind/author_type are CHECK-enforced).
INSERT INTO conversations (id, company_id, kind, subject, participants, created_by, updated_at) VALUES
  ('c0a70001-0000-4000-8000-000000000001'::uuid, 'd11d0001-0000-4000-8000-000000000001'::uuid,
     'dm_agent', 'Atlas — Q3 budget questions',
     '[{"type":"user","id":"demo-operator"},{"type":"agent","id":"a9e70001-0000-4000-8000-000000000001"}]'::jsonb,
     'demo-operator', now() - interval '15 minutes');
INSERT INTO messages (id, conversation_id, company_id, author_type, author_id, body, created_at) VALUES
  ('30e50001-0000-4000-8000-000000000001'::uuid, 'c0a70001-0000-4000-8000-000000000001'::uuid,
     'd11d0001-0000-4000-8000-000000000001'::uuid, 'user',  'demo-operator',
     'Atlas, can you summarize the Q3 cloud-credit reallocation before the board reviews it?', now() - interval '16 minutes'),
  ('30e50002-0000-4000-8000-000000000002'::uuid, 'c0a70001-0000-4000-8000-000000000001'::uuid,
     'd11d0001-0000-4000-8000-000000000001'::uuid, 'agent', 'a9e70001-0000-4000-8000-000000000001',
     'Done — I routed a brief for board approval: reallocate $2,500 of unused credits to the inference budget.', now() - interval '15 minutes');

-- 10) POST COMMENTS — a reply hanging off the pending decision (subject_kind 'decision').
INSERT INTO post_comments (id, company_id, subject_kind, subject_id, author_type, author_id, body, created_at) VALUES
  ('c0de0001-0000-4000-8000-000000000001'::uuid, 'd11d0001-0000-4000-8000-000000000001'::uuid,
     'decision', '1ed90001-0000-4000-8000-000000000001', 'user', 'demo-operator',
     'Looks reasonable — confirming the credits actually expire this quarter.', now() - interval '10 minutes');

COMMIT;

-- VERIFY — per-source counts for the demo account (each should be > 0).
SELECT 'companies'       AS source, count(*) AS n FROM companies      WHERE account_id = 'acct-demo-holding'
UNION ALL SELECT 'agents',          count(*) FROM agents          WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'runtime_runs',    count(*) FROM runtime_runs    WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'approvals',       count(*) FROM approvals       WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'approvals_pending', count(*) FROM approvals     WHERE status = 'pending' AND company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'decision_ledger', count(*) FROM decision_ledger WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'finance_events',  count(*) FROM finance_events  WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'conversations',   count(*) FROM conversations   WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'messages',        count(*) FROM messages        WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
UNION ALL SELECT 'post_comments',   count(*) FROM post_comments   WHERE company_id IN (SELECT id FROM companies WHERE account_id = 'acct-demo-holding')
ORDER BY source;
