#!/usr/bin/env node
// Generate the FULL idempotent base schema (crates/chronica-core/migrations/0000_chronica_init.sql)
// from the FINAL legacy drizzle snapshot. One-shot provenance tool, kept in-repo so the translation
// is reviewable + re-runnable. The legacy TS journal is no longer required for fresh installs.
import { readFileSync, writeFileSync } from 'node:fs';

const SNAPSHOT = 'legacy/packages/db/dist/migrations/meta/0094_snapshot.json';
// Tables OWNED by later sqlx migrations (0001-0011) — they keep authority; 0000 must not duplicate them.
const SQLX_OWNED = new Set(['company_secrets','company_secret_versions','integrations','money_commitments','sandbox_executions','sandbox_leases','secret_access_audit']);

const snap = JSON.parse(readFileSync(SNAPSHOT, 'utf8'));
const tables = Object.values(snap.tables).filter(t => !SQLX_OWNED.has(t.name));

const q = (s) => `"${s}"`;
const colDdl = (c) => {
  let d = `    ${q(c.name)} ${c.type}`;
  if (c.primaryKey) d += ' PRIMARY KEY';
  if (c.notNull && !c.primaryKey) d += ' NOT NULL';
  if (c.default !== undefined) d += ` DEFAULT ${c.default}`;
  return d;
};

let out = [];
let fkOut = [];
let idxOut = [];

for (const t of tables) {
  const cols = Object.values(t.columns).map(colDdl);
  const cpk = Object.values(t.compositePrimaryKeys || {});
  if (cpk.length) cols.push(`    PRIMARY KEY (${cpk[0].columns.map(q).join(', ')})`);
  out.push(`CREATE TABLE IF NOT EXISTS ${q(t.name)} (\n${cols.join(',\n')}\n);`);

  for (const ix of Object.values(t.indexes || {})) {
    const cols = ix.columns.map(c => c.expression && !c.isExpression ? q(c.expression) : c.expression).join(', ');
    idxOut.push(`CREATE ${ix.isUnique ? 'UNIQUE ' : ''}INDEX IF NOT EXISTS ${q(ix.name)} ON ${q(t.name)} (${cols});`);
  }
  for (const uc of Object.values(t.uniqueConstraints || {})) {
    // unique constraints as UNIQUE INDEXES: same ON CONFLICT (cols) semantics, naturally idempotent
    // (no ON CONFLICT ON CONSTRAINT usage exists in the Rust tree — grep-verified at generation time).
    idxOut.push(`CREATE UNIQUE INDEX IF NOT EXISTS ${q(uc.name)} ON ${q(t.name)} (${uc.columns.map(q).join(', ')});`);
  }
  for (const fk of Object.values(t.foreignKeys || {})) {
    if (SQLX_OWNED.has(fk.tableTo)) continue; // target owned by a LATER sqlx migration — FK deferred (integrity sugar, not load-bearing)

    const od = fk.onDelete && fk.onDelete !== 'no action' ? ` ON DELETE ${fk.onDelete.toUpperCase()}` : '';
    const ou = fk.onUpdate && fk.onUpdate !== 'no action' ? ` ON UPDATE ${fk.onUpdate.toUpperCase()}` : '';
    fkOut.push(
`DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = '${fk.name}') THEN
    ALTER TABLE ${q(t.tableFrom || t.name)} ADD CONSTRAINT ${q(fk.name)} FOREIGN KEY (${fk.columnsFrom.map(q).join(', ')}) REFERENCES ${q(fk.tableTo)} (${fk.columnsTo.map(q).join(', ')})${od}${ou};
  END IF;
END $$;`);
  }
  for (const cc of Object.values(t.checkConstraints || {})) {
    fkOut.push(
`DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = '${cc.name}') THEN
    ALTER TABLE ${q(t.name)} ADD CONSTRAINT ${q(cc.name)} CHECK (${cc.value});
  END IF;
END $$;`);
  }
}

const header = `-- Chronica BASE SCHEMA (full, idempotent) — reconstructed 2026-06-10 from the FINAL legacy drizzle
-- snapshot (legacy/packages/db/dist/migrations/meta/0094_snapshot.json) by tools/db/gen-base-schema.mjs.
--
-- WHY: 0000 used to be a no-op ("the transitional TS drizzle journal bootstraps the base schema"),
-- which meant a FRESH database could never migrate (0003 ALTERs runtime_artifacts; ~45 code-queried
-- tables had no sqlx CREATE). The legacy journal is now fully absorbed: fresh installs need ONLY this
-- sqlx chain. Every statement is IF NOT EXISTS / guarded, so this migration is a no-op on any DB that
-- was bootstrapped by the legacy drizzle journal.
--
-- Tables owned by LATER sqlx migrations are deliberately NOT created here (authority preserved):
-- ${[...SQLX_OWNED].join(', ')}.
--
-- MANUAL section at the end: 4 tables the Rust tree queries that the legacy snapshot never carried
-- (their DDL is derived from the Rust queries that use them): runtime_memory, runtime_daemon_leases,
-- runtime_mcp_servers, cohort_membership.

CREATE EXTENSION IF NOT EXISTS pgcrypto;
`;

const manual = `
-- ──────────────────────────────────────────────────────────────────────────────
-- MANUAL: Rust-queried tables absent from the legacy snapshot (DDL from the queries).
CREATE TABLE IF NOT EXISTS "runtime_memory" (
    "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    "company_id" uuid NOT NULL,
    "namespace" text NOT NULL,
    "key" text NOT NULL,
    "value_json" jsonb NOT NULL,
    "source" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "version" integer NOT NULL DEFAULT 1,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "updated_at" timestamptz NOT NULL DEFAULT now()
);
-- (0006 adds the (company_id, namespace, key) unique index powering the versioned upsert.)
CREATE TABLE IF NOT EXISTS "runtime_daemon_leases" (
    "lease_key" text PRIMARY KEY,
    "owner_id" text NOT NULL,
    "expires_at" timestamptz NOT NULL,
    "last_heartbeat_at" timestamptz NOT NULL DEFAULT now(),
    "metadata" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "updated_at" timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS "workflow_task_queue" (
    "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    "execution_id" uuid NOT NULL,
    "task_kind" text NOT NULL DEFAULT 'node',
    "node_id" text,
    "payload" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "status" text NOT NULL DEFAULT 'pending',
    "claimed_by" text,
    "claimed_until" timestamptz,
    "available_at" timestamptz NOT NULL DEFAULT now(),
    "attempts" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "updated_at" timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS "runtime_mcp_servers" (
    "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    "company_id" uuid NOT NULL,
    "name" text NOT NULL,
    "enabled" boolean NOT NULL DEFAULT true,
    "transport_kind" text NOT NULL,
    "transport_config" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "env_allowlist" jsonb NOT NULL DEFAULT '[]'::jsonb,
    "status" text NOT NULL DEFAULT 'configured',
    "discovered_tools" jsonb NOT NULL DEFAULT '[]'::jsonb,
    "last_error" text,
    "activation_failure_count" integer NOT NULL DEFAULT 0,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "updated_at" timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS "cohort_membership" (
    "cohort_id" uuid NOT NULL,
    "person_id" uuid NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("cohort_id", "person_id")
);
`;

const sql = [header, ...out, '', ...idxOut, '', ...fkOut, manual].join('\n\n');
writeFileSync('crates/chronica-core/migrations/0000_chronica_init.sql', sql);
console.log(`tables: ${tables.length} (+4 manual), indexes/uniques: ${idxOut.length}, guarded FK/check: ${fkOut.length}`);
