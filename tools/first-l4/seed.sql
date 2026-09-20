-- First-L4 seed — DEV DB ONLY. Idempotent. Creates one account-owned company so a real human can drive
-- the first L4 chain (login -> propose -> approve -> execute -> audit). Principals are minted at login by
-- POST /api/dev/login (no rows needed). All other companies columns have safe defaults (0000_chronica_init.sql).
-- Apply: psql "postgres://chronica:chronica@127.0.0.1:55432/chronica" -f tools/first-l4/seed.sql
INSERT INTO companies (id, name, account_id, status)
VALUES ('f1f1f1f1-0000-4000-8000-000000000001'::uuid, 'First-L4 Strike Co', 'acct-first-l4', 'active')
ON CONFLICT (id) DO UPDATE SET account_id = EXCLUDED.account_id, name = EXCLUDED.name, status = 'active';

-- Confirm it is owned by the account the dev-login will use:
SELECT id, name, account_id, status FROM companies WHERE id = 'f1f1f1f1-0000-4000-8000-000000000001';
