// Makes migration 0087_dapper_xorn.sql idempotent. The snapshot's 0087
// redundantly re-states tables/columns/indexes/constraints that migrations
// 0082-0086 already created (a regenerated-migration artifact), so a fresh
// `pnpm db:migrate` from empty collides. The fixes use the SAME idempotency
// conventions the repo already uses elsewhere:
//   - CREATE TABLE / INDEX / ADD COLUMN -> IF NOT EXISTS
//   - ADD CONSTRAINT -> the repo's own DO $$ ... IF NOT EXISTS(pg_constraint) $$
//     guard (exactly as 0082_dry_vision.sql does).
// All are strictly backward-compatible: they never break an already-applied DB
// and fix fresh installs. Intent is unchanged.
import { readFileSync, writeFileSync } from "node:fs";

const path = "packages/db/src/migrations/0087_dapper_xorn.sql";
let sql = readFileSync(path, "utf8");
const before = sql;
let n = 0;

// 1) CREATE TABLE "x" ( -> CREATE TABLE IF NOT EXISTS "x" (
sql = sql.replace(/CREATE TABLE (?!IF NOT EXISTS)("?\w)/g, (_m, g1) => {
  n++;
  return `CREATE TABLE IF NOT EXISTS ${g1}`;
});
// 2) CREATE [UNIQUE] INDEX "x" -> ... IF NOT EXISTS "x"
sql = sql.replace(/CREATE (UNIQUE )?INDEX (?!IF NOT EXISTS)("?\w)/g, (_m, u, g1) => {
  n++;
  return `CREATE ${u ?? ""}INDEX IF NOT EXISTS ${g1}`;
});
// 3) ADD COLUMN "y" -> ADD COLUMN IF NOT EXISTS "y"
sql = sql.replace(/ADD COLUMN (?!IF NOT EXISTS)("?\w)/g, (_m, g1) => {
  n++;
  return `ADD COLUMN IF NOT EXISTS ${g1}`;
});

// 4) Wrap each bare `ALTER TABLE ... ADD CONSTRAINT "name" ...;` in the repo's
//    pg_constraint guard. Match a full statement up to the line-final `;`
//    (these are single-line in 0087). Skip ones already inside a DO block.
const constraintRe =
  /^ALTER TABLE ("[^"]+"|\S+) ADD CONSTRAINT "([^"]+)" ([^\n]*?);(?=--> statement-breakpoint|\s*$)/gm;
sql = sql.replace(constraintRe, (_m, table, conname, rest) => {
  n++;
  return (
    `DO $$ BEGIN\n` +
    `\tIF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = '${conname}') THEN\n` +
    `\t\tALTER TABLE ${table} ADD CONSTRAINT "${conname}" ${rest};\n` +
    `\tEND IF;\n` +
    `END $$;`
  );
});

if (sql === before) {
  console.log("0087 already idempotent (no changes)");
} else {
  writeFileSync(path, sql);
  console.log(`0087 idempotent transforms applied: ${n} statement(s)`);
}
