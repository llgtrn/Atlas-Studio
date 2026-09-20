// Provisions the Chronica Rust kernel's dev database with NEUTRAL branding.
//
// The temporary TS monolith's embedded-postgres ships a `paperclip` superuser +
// `paperclip` db (hardcoded in packages/db, which is transitional and not edited
// here). The Rust system owns its OWN Chronica-branded database: this script
// connects to the running embedded cluster as the bootstrap superuser, creates a
// `chronica` role + `chronica` database, and migrates the full schema into it.
//
// Usage:
//   PAPERCLIP_CONFIG=<abs .chronica-dev/config.json> node tools/build/provision-chronica-db.mjs
//
// After it runs, the Rust sqlx layer connects with:
//   DATABASE_URL=postgres://chronica:chronica@127.0.0.1:<port>/chronica
import { resolveMigrationConnection } from "@chronica/db/migration-runtime";
import { applyPendingMigrations } from "@chronica/db/client";
import postgres from "postgres";

const CHRONICA_ROLE = "chronica";
const CHRONICA_PW = "chronica";
const CHRONICA_DB = "chronica";

const resolved = await resolveMigrationConnection();
const bootstrapUrl = resolved.connectionString; // e.g. postgres://paperclip:paperclip@127.0.0.1:55433/paperclip
const u = new URL(bootstrapUrl);
const host = u.hostname;
const port = u.port;
console.log(`bootstrap: ${u.username}@${host}:${port}/${u.pathname.slice(1)} (source: ${resolved.source})`);

const admin = postgres(bootstrapUrl, { max: 1, onnotice: () => {} });
try {
  // Create the neutral chronica role (idempotent).
  const roleExists = await admin`select 1 from pg_roles where rolname = ${CHRONICA_ROLE}`;
  if (roleExists.length === 0) {
    await admin.unsafe(
      `CREATE ROLE "${CHRONICA_ROLE}" LOGIN PASSWORD '${CHRONICA_PW}' CREATEDB`,
    );
    console.log(`created role ${CHRONICA_ROLE}`);
  } else {
    console.log(`role ${CHRONICA_ROLE} already exists`);
  }
  // Create the neutral chronica database owned by it (idempotent).
  const dbExists = await admin`select 1 from pg_database where datname = ${CHRONICA_DB}`;
  if (dbExists.length === 0) {
    await admin.unsafe(`CREATE DATABASE "${CHRONICA_DB}" OWNER "${CHRONICA_ROLE}"`);
    console.log(`created database ${CHRONICA_DB}`);
  } else {
    console.log(`database ${CHRONICA_DB} already exists`);
  }
} finally {
  await admin.end();
}

// Migrate the full schema into the chronica database (as the chronica role).
const chronicaUrl = `postgres://${CHRONICA_ROLE}:${CHRONICA_PW}@${host}:${port}/${CHRONICA_DB}`;
console.log(`migrating schema into chronica db...`);
await applyPendingMigrations(chronicaUrl);

// Verify.
const sql = postgres(chronicaUrl, { max: 1, onnotice: () => {} });
try {
  const n = await sql`select count(*)::int n from information_schema.tables where table_schema='public'`;
  const k = await sql`select to_regclass('public.companies')::text c, to_regclass('public.approvals')::text a, to_regclass('public.cost_events')::text ce`;
  console.log(`chronica db ready: ${n[0].n} public tables; key: ${JSON.stringify(k[0])}`);
  console.log(`DATABASE_URL=${chronicaUrl}`);
} finally {
  await sql.end();
}

try {
  await resolved.stop();
} catch {}
process.exit(0);
