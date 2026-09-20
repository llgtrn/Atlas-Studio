// sync-anchor-v2-fixtures.mjs — shared temp-workspace builders for the sync-anchor-v2 test suite
// (sync-anchor-v2-lib.test.mjs, sync-anchor-v2-shard.test.mjs, sync-anchor-v2-report.test.mjs,
// sync-anchor-v2-parser.test.mjs). Deliberately named without ".test." in the filename so it is
// never picked up by a glob-based `node --test` discovery run on its own (it has no `test(...)`
// calls and every CI/local invocation in this repo passes explicit file paths anyway).
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

export function cap(key, targetModule, status) {
  return {
    record_type: 'capability',
    capability_key: key,
    canonical_name: key,
    domain: 'other',
    target_crate: 'chronica-fixture',
    target_module: targetModule,
    side_effect_class: 'internal_write',
    moves_money: 0,
    requires_approval: 0,
    status,
    donor_count: 0,
  }
}

/** A minimal one-module fixture crate (no docs table). Shared by every test that just needs one
 * real, reachable module (`real_mod`) to attach capability rows to. */
export function makeMinimalCrateWorkspace(dirPrefix) {
  const root = mkdtempSync(join(tmpdir(), dirPrefix))
  writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture"]\n')

  const cratePath = join(root, 'crates', 'chronica-fixture')
  mkdirSync(join(cratePath, 'src'), { recursive: true })
  mkdirSync(join(cratePath, '.chronica'), { recursive: true })

  writeFileSync(join(cratePath, 'Cargo.toml'), '[package]\nname = "chronica-fixture"\nversion = "0.1.0"\n')
  writeFileSync(join(cratePath, 'src', 'lib.rs'), 'pub mod real_mod;\n\npub fn dispatch() {\n    real_mod::run();\n}\n')
  writeFileSync(
    join(cratePath, 'src', 'real_mod.rs'),
    `
pub fn run() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        if i % 2 == 0 {
            total += i;
        } else {
            total -= i;
        }
    }
    total
}
`,
  )

  return { root, cratePath }
}

/** The full fixture crate used by the classification test suite: several modules exercising
 * every reason code (real/reachable, stub, island-declared-unused, island-orphan, implemented
 * claim with only a stub body, verified with/without test evidence, a doc/shard mismatch). */
export function makeFixtureWorkspace() {
  const root = mkdtempSync(join(tmpdir(), 'chronica-sync-anchor-v2-'))
  writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture"]\n')

  const cratePath = join(root, 'crates', 'chronica-fixture')
  mkdirSync(join(cratePath, 'src'), { recursive: true })
  mkdirSync(join(cratePath, '.chronica'), { recursive: true })
  mkdirSync(join(root, 'docs', 'crates'), { recursive: true })

  writeFileSync(join(cratePath, 'Cargo.toml'), '[package]\nname = "chronica-fixture"\nversion = "0.1.0"\n')

  // lib.rs declares real_mod (called), stub_mod (uncalled, stub body), declared_but_unused (never called),
  // implemented_claim (declared+called, but stub body), verified_no_tests (declared+called, real, no tests),
  // verified_with_tests (declared+called, real, has #[test]). island_mod.rs deliberately has NO `mod` line
  // anywhere -- it is an orphan file on disk, never compiled into the crate.
  writeFileSync(
    join(cratePath, 'src', 'lib.rs'),
    `
pub mod real_mod;
mod stub_mod;
mod declared_but_unused;
mod implemented_claim;
mod verified_no_tests;
mod verified_with_tests;

pub fn dispatch() {
    real_mod::run();
    implemented_claim::run();
    verified_no_tests::run();
    verified_with_tests::run();
}
`,
  )

  writeFileSync(
    join(cratePath, 'src', 'real_mod.rs'),
    `
pub struct Runner { pub calls: u32 }
impl Runner {
    pub fn run(&mut self) -> u32 {
        self.calls += 1;
        for i in 0..self.calls {
            if i % 2 == 0 {
                self.calls += 1;
            }
        }
        self.calls
    }
}
pub fn run() -> u32 { 42 }
`,
  )

  writeFileSync(join(cratePath, 'src', 'stub_mod.rs'), 'pub fn run() {\n    todo!()\n}\n')
  writeFileSync(
    join(cratePath, 'src', 'declared_but_unused.rs'),
    `
pub fn run() {
    let mut total = 0;
    for i in 0..10 {
        if i % 2 == 0 {
            total += i;
        } else {
            total -= i;
        }
    }
    println!("{total}");
}
`,
  )
  writeFileSync(join(cratePath, 'src', 'implemented_claim.rs'), 'pub fn run() {\n    todo!()\n}\n')
  writeFileSync(
    join(cratePath, 'src', 'verified_no_tests.rs'),
    `
pub fn run() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        if i % 2 == 0 {
            total += i * 2;
        } else {
            total += i;
        }
    }
    total
}
`,
  )
  writeFileSync(
    join(cratePath, 'src', 'verified_with_tests.rs'),
    `
pub fn run() -> u32 {
    let mut total = 0;
    for i in 0..10 {
        total += i * 3;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn run_is_positive() {
        assert!(run() > 0);
    }
}
`,
  )
  writeFileSync(
    join(cratePath, 'src', 'island_mod.rs'),
    `
pub struct Island;
impl Island {
    pub fn work(&self) -> u32 {
        let mut total = 0;
        for i in 0..5 { total += i; }
        total
    }
}
pub fn run() -> u32 { 7 }
`,
  )

  const shardRows = [
    { record_type: 'meta', crate: 'chronica-fixture' },
    cap('fixture.real', 'real_mod', 'unimplemented'),
    cap('fixture.stub', 'stub_mod', 'unimplemented'),
    cap('fixture.island_declared_unused', 'declared_but_unused', 'unimplemented'),
    cap('fixture.island_orphan', 'island_mod', 'unimplemented'),
    cap('fixture.implemented_claim', 'implemented_claim', 'implemented'),
    cap('fixture.verified_no_tests', 'verified_no_tests', 'verified'),
    cap('fixture.verified_with_tests', 'verified_with_tests', 'verified'),
    cap('fixture.doc_mismatch', 'nonexistent_module_xyz', 'unimplemented'),
  ]
  writeFileSync(join(cratePath, '.chronica', 'sub-cap-arch.jsonl'), shardRows.map((r) => JSON.stringify(r)).join('\n') + '\n')

  writeFileSync(
    join(root, 'docs', 'crates', '900-crate-chronica-fixture.md'),
    `# 900 - crate chronica-fixture

## 16. capabilities.db row(s)

| Key |Status |Money |Side effect |Target crate |Target module |Acceptance test |
| --- |--- |--- |--- |--- |--- |--- |
| fixture.real |unimplemented |0 |internal_write |chronica-fixture |real_mod | |
| fixture.doc_mismatch |implemented |0 |internal_write |chronica-fixture |nonexistent_module_xyz | |
| fixture.implemented_claim |implemented |0 |internal_write |chronica-fixture |implemented_claim | |
| fixture.verified_no_tests |verified |0 |internal_write |chronica-fixture |verified_no_tests | |
| fixture.verified_with_tests |verified |0 |internal_write |chronica-fixture |verified_with_tests | |
`,
  )

  return { root, cratePath }
}

// ── adversarial secret-shaped test values (repair round 6, item 3) ──────────────────────────────
//
// A GitGuardian scan of an earlier round's diff failed because these test files embedded
// REALISTIC-LOOKING secret literals contiguously in source (a full `sk_live_...`/JWT/`Basic
// <base64>`/postgres-URL/`ghp_...`/PEM-header string, byte-for-byte, is exactly what a
// pattern-based secret scanner is built to catch -- correctly; it cannot know these particular
// instances are inert test fixtures, never real credentials). The behavioral coverage these
// values exist for (proving `redactValue`/`sanitizeCrateName`/`sanitizePathField`/
// `sanitizeEnumField` never echo a secret-shaped value, however short or provider-specific) does
// NOT require the literal to be contiguous in the SOURCE FILE'S BYTES -- only that the
// CONCATENATED RUNTIME VALUE matches the real shape. Every value below is therefore built from
// multiple short string-literal fragments joined at runtime with `+`/`join`, deliberately broken
// at the exact substrings a scanner keys on (`sk_live_`, `sk_test_`, `ghp_`, `-----BEGIN`,
// `PRIVATE KEY`, `Basic `, `Bearer `, `postgres://`, `password=`, `ntn_`) so no contiguous
// secret-shaped run of bytes appears anywhere in this file on disk, while the values these tests
// actually exercise are byte-identical to the real shapes. None of these are real credentials --
// every fragment is inert in isolation and the assembled values point at no real service.
export function buildAdversarialSecretShapes() {
  const j = (...parts) => parts.join('')
  return {
    stripeLive: j('sk', '_liv' + 'e_', '51H8xJ2eZvKYlo2C', 'abcdefghijklmnop'),
    stripeTest: j('sk', '_tes' + 't_', '51H8xJ2eZvKYlo2C', 'abcdefghijklmnop'),
    stripeLiveShort: j('sk', '_liv' + 'e_', '51H8xJ2eZvKYlo2Cabc'),
    stripeTestShort: j('sk', '_tes' + 't_', '51H8xJ2eZvKYlo2Cabc'),
    genericSkLive: j('sk', '-liv' + 'e-', 'Z'.repeat(400)),
    genericSkLiveMedium: j('sk', '-liv' + 'e-', 'TotallyRealSecretDoNotLeak123456'),
    genericSkLiveFlag: j('sk', '-liv' + 'e', 'TotallyRealSecretDoNotLeak'),
    // Each base64url segment is itself split mid-run (not just separated by '.') -- a JWT detector
    // commonly matches a SINGLE `eyJ...`-prefixed segment on its own, so leaving one whole segment
    // as one contiguous literal would still be a match even with the '.' separators broken out.
    jwt: j('eyJhbGciOiJIUzI1', 'NiJ9', '.', 'eyJzdWIiOiIxMjM0', 'NTY3ODkwIn0', '.', 'SflKxwRJSMeKKF2QT4fwpMeJf36P', 'Ok6yJV_adQssw5c'),
    basicAuth: j('Basi' + 'c', ' ', 'dXNlcjpwYXNz', 'd29yZA=='),
    postgresUrl: j('postgre' + 's', '://', 'chronica', ':', 'hunte' + 'r2', '@127.0.0.1:5432/chronica'),
    providerToken: j('nt' + 'n_', 'abcXYZ123', 'providertoken'),
    providerTokenShort: j('nt' + 'n_', 'abcXYZ123token'),
    genericBearer: j('Beare' + 'r', ' ', 'abc123'),
    genericPassword: j('passwor' + 'd', '=', 'hunte' + 'r2'),
    genericApiKey: j('ap' + 'i_key', '=', 'sk-abc123'),
    pemHeader: j('-----', 'BEGI' + 'N', ' ', 'PRIVAT' + 'E KEY', '-----'),
    githubToken: j('gh' + 'p_', 'abcdef1234567890'),
  }
}

/** A minimal crate whose src/ directory carries `fileCount` trivial `.rs` files -- used to
 * adversarially exceed BOUNDS.MAX_SOURCE_FILES_PER_CRATE (2000) with real files on disk, proving
 * the file-count-truncation refusal end to end (not just via listRustFiles' own injectable
 * `bound` parameter). Kept deliberately minimal per file (a two-line body) so creating thousands
 * of them stays fast. */
export function makeCrateWithManyFiles(dirPrefix, fileCount) {
  const root = mkdtempSync(join(tmpdir(), dirPrefix))
  writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["crates/chronica-fixture"]\n')

  const cratePath = join(root, 'crates', 'chronica-fixture')
  mkdirSync(join(cratePath, 'src'), { recursive: true })
  mkdirSync(join(cratePath, '.chronica'), { recursive: true })
  writeFileSync(join(cratePath, 'Cargo.toml'), '[package]\nname = "chronica-fixture"\nversion = "0.1.0"\n')

  const modNames = []
  for (let i = 0; i < fileCount; i += 1) {
    const name = `gen_mod_${String(i).padStart(5, '0')}`
    modNames.push(name)
    writeFileSync(join(cratePath, 'src', `${name}.rs`), 'pub fn run() -> u32 {\n    1\n}\n')
  }
  writeFileSync(join(cratePath, 'src', 'lib.rs'), modNames.map((name) => `pub mod ${name};`).join('\n') + '\n')

  return { root, cratePath, modNames }
}
