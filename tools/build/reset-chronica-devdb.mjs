// DEV-ONLY: drop + recreate the throwaway `chronica` database in the running
// embedded cluster, so the Rust `run_chronica_migrations` runner applies the
// CURRENT migration set from scratch (clears a stale checksum-mismatch where a
// migration applied earlier was later modified). Safe: this DB holds only
// dev/test rows; the route-parity tests self-seed and clean up.
//
// Usage: node tools/build/reset-chronica-devdb.mjs
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const postgres = require(
  "C:/Users/trngh/Documents/GitHub/Chronica/node_modules/.pnpm/postgres@3.4.8/node_modules/postgres/cjs/src/index.js",
);

const PORT = 55432;
// The `chronica` role (CREATEDB, owns the chronica db) connects to the `postgres`
// maintenance db so it can DROP/CREATE its own database. The bootstrap superuser
// password is not the embedded default; the owner role suffices here.
const ADMIN = `postgres://chronica:chronica@127.0.0.1:${PORT}/postgres`;

const admin = postgres(ADMIN, { max: 1, onnotice: () => {} });
try {
  // Terminate any live connections to the chronica db so DROP can proceed.
  await admin`select pg_terminate_backend(pid) from pg_stat_activity where datname = 'chronica' and pid <> pg_backend_pid()`;
  await admin.unsafe(`DROP DATABASE IF EXISTS "chronica"`);
  console.log("dropped chronica db");
  await admin.unsafe(`CREATE DATABASE "chronica" OWNER "chronica"`);
  console.log("created chronica db (empty; Rust migrations will populate it)");
} finally {
  await admin.end();
}
process.exit(0);
