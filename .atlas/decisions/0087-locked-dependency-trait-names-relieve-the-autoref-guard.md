---
id: atlas.decision.0087.locked-dependency-trait-names-relieve-the-autoref-guard
type: decision
status: accepted
canonical: true
---
# ADR 0087 — Locked dependency trait names relieve the autoref guard (G173)

## Context

G173 took NA-CALL-TYPE-RESIDUAL, the native queue head (pressure 73). Its largest measured residual is the external-declaration boundary G167 found, which NA-DEPENDENCY-DECLARATIONS was queued to attack.

The path-call resolver claims `value.name()` for an inherent `&self` or `&mut self` method through the method probe's autoref step. Since G142, it withholds that claim whenever any scope on the lookup chain imports anything from a registry crate. The reason: a trait of that crate, in scope, could supply a by-value `name`, which rustc's probe tries before the autoref step.

Measured on Atlas with an instrumented guard, 1,257 of 1,443 guard refusals came from registry imports alone: `syn` (`Spanned`, `Visit`, `Punctuated`), `serde` (`Serialize`, `Deserialize`), `tree_sitter` (`Node`, `Parser`), `quote`, `proc_macro2` and `blake3`. Atlas could not see what those crates declare.

Reviewing the guard for this change found a second, older hole (a falsification of G142). The guard ignored two kinds of binding:
- an unresolved named import (`Def::Unknown`);
- a name two globs bind differently (`Def::Ambiguous`).

Either may be a trait. A regression test shows two claims the guard made: `use super::gen::Tr`, where `gen` is a module whose file is missing, and a glob-ambiguous `X` that is an external trait. In Rust, a by-value `Tr::look` implemented for the receiver would be chosen instead.

An independent adversarial review of the first version of this change found 15 more holes, each reproduced on stable Rust. All are closed here:
- **Three in the guard, older than this change:**
  - `use Tr as _` was dropped, so a trait in scope went unseen;
  - a procedural attribute or derive on a workspace item may emit a trait into its module;
  - a workspace impl of a trait whose method names outside the eight by-value std traits the guard knew (`impl Add for S { fn add(self, ..) }`) was ignored, so the trait's by-value method came first.
- **Eleven in the first reading:**
  - a keyword macro reached through a rename;
  - a `<X = u8>` default or a const-generic `{ 1 }` block in a trait header;
  - a trait body smuggled through a metavariable;
  - a `#[macro_export]` macro defined inside a statement macro;
  - a `cfg_attr`-spelled procedural export;
  - a procedural macro invoked through a metavariable;
  - an `include!` or `#[path]` of a non-`.rs` file;
  - a library rooted at the package root reaching `tests/`;
  - a feature on a target-specific dependency wrongly judged impossible;
  - a same-named copy under another registry's index.

## Decision

1. **The sources Cargo compiles are a census input, recorded by digest.**
   - `adapter::read_locked_trait_methods(lock, registry_src)` reads every registry package that the census root's `Cargo.lock` pins, from the directory Cargo extracts and compiles: `<CARGO_HOME>/registry/src/<index>/<name>-<version>`, complete only when `.cargo-ok` exists.
   - The sources are read as data: parsed with `syn`, never built, run or expanded.
   - The resolution batches' input fingerprint carries a BLAKE3 digest over the lockfile, every file read, and every answer.
   - These sources are not an oracle. The oracle stays test-time only (rust-analyzer SCIP).
   - A machine without the sources gets UNKNOWN (the G172 behavior), never a different claim. Only the digest differs.
2. **One answer per package: the union of trait method names over its lock closure, or UNKNOWN.**
   - A package's traits come only from its own sources and its dependencies'.
   - A crate outside the lock (`proc_macro`) may be used by a package, never re-exported: a `pub use` or `pub extern crate` of one makes the package UNKNOWN.
   - Standard-library traits a dependency re-exports are the traits a direct `std` import names, which G142's std rules already cover.
   - Top-level `tests/`, `benches/` and `examples/` directories are separate crates and are not read. A `#[path]` or `include!` that could reach them, or leave the package, is refused.
3. **What makes a closure UNKNOWN.** Anything that could declare a trait method the reading cannot see:
   - **Unreadable sources:** a non-registry or unextracted package, a symlink, a file that does not parse or that the recursion pre-scan refuses, an unparsed item (`macro` 2.0).
   - **Macros among a trait's items:** a macro invocation in a trait body.
   - **Unseen expansions.** Only a procedural macro turns an attribute, derive or macro invocation into new items (the built-in derives emit impls), and a package can only invoke one that its own closure exports. So each of the following is UNKNOWN when a procedural macro of the package's closure exports its name:
     - an attribute or derive, on a module-level item, on a trait item, or spelled inside a `macro_rules!` transcriber or item-macro input;
     - a macro invocation, likewise at module level or inside a transcriber or item-macro input;
     - a `use` renaming such a macro.

     Beyond names:
     - a procedural macro named like a built-in attribute or derive makes its closure UNKNOWN;
     - any `include!` other than one of a literal path inside the package is refused. Serde's crate root splices `include!(concat!(env!("OUT_DIR"), "/private.rs"))` from its build script, so `serde`, `serde_json` and `tree-sitter` stay UNKNOWN.

     Any other such name is either built in, or feature-gated behind a crate that is not locked (it would not compile).
   - **Trait bodies assembled in macros.** In a transcriber or item-macro input, a trait body is refused when it holds a metavariable other than `$crate`, a method named by a metavariable, or a macro invocation among its own items. A `trait` whose body comes from elsewhere is refused too.
     - A brace group whose header a metavariable may complete into `trait` counts only when some macro input in the closure spells the keyword. A metavariable binds only tokens an invocation spells, so without such an input no metavariable can become `trait`.
     - A lone `Token![trait]` binds the keyword only if a rule of that macro could capture it with a metavariable.
   - **Features.** An item under `cfg(feature = F)` is skipped only when no build under this lock can enable `F`: `F` would activate a dependency that the lock does not record for the package.
4. **Imports carry their locked package.**
   - `Def::External` records the registry package its root names. The mapping is `CrateInput.foreign`: a manifest dependency key, a rename, or an entry inherited from the root manifest. It is kept only when the lockfile records that package as a direct dependency of the target's package.
   - A binding reached through two packages has an unknown package.
   - The autoref guard trusts a registry import when its package's closure is known and declares no trait method of the called name.
5. **Unknown, ambiguous and hidden bindings withhold the claim.**
   - `use Tr as _` binds the trait under a name no path can spell, so the guard sees it.
   - A module whose item carries an attribute or derive outside the allowlists (G145's, plus the inert built-ins) is open: a procedural macro may emit any item there.
   - A trait impl on the receiver type that defines the called name withholds the claim.
   - `Def::Unknown(true)` withholds it: the import's target is unknown or open, or the target is a settled module holding an item macro whose transcriber or invocation spells `trait`. A settled module that binds no such name and whose item macros never spell `trait` defines no trait by that name (`Def::Unknown(false)`).
   - `Def::Ambiguous` always withholds.

## Evidence

- **Atlas's own lock** (37 packages): every registry closure reads except `serde`, `serde_core`, `serde_json` and `tree-sitter` (the build-script include).
- **Atlas itself:** the G172 resolver and this one ran over the same snapshot of the sources.
  - The relief resolves 115 more path calls.
  - The hardening withdraws 7. All 7 were correct, but Atlas can no longer justify them:
    - 6 are in `weights/safetensors.rs`, which imports names from `core::weights`. G145 leaves that module open because `vocabulary_enum!` is `#[macro_export]`ed from another module.
    - 1 is in a runtime test module in the same state.
  - The review's other fixes withdraw nothing on Atlas.
- **Oracle:** rust-analyzer 1.90.0's SCIP index of the final tree confirms all 115 gained and all 7 withdrawn edges. Over every path call Atlas claims, 7,643 of 7,644 are confirmed; the last is on a line with a multi-byte `±` and reads correct. None is wrong.
- **Tests:**
  - the relief: a closure that lacks the name, one that declares it, an unknown closure, an unlocked package, a binding through two packages, a glob carrying its package, no table;
  - the hardening: a missing module, a glob ambiguity, a struct import, a settled macro-defined name, a macro trait, a keyword passed as macro input;
  - every UNKNOWN rule, with a fixture registry;
  - each review finding as an isolated regression case, with a benign control where one applies.
- **Mutants:** 39 killed (22 for the first version, 17 for the review fixes). Two equivalent ones were removed as dead code: a redundant directory check, and a `macro_rules` shadowing check (a procedural macro cannot take over `macro_rules!`).

## Consequences

- **DEBT-CALL:** the external-declaration boundary is crossed for every registry crate whose closure reads. Capability epoch E20.
- **Still UNKNOWN:**
  - crates whose closure holds build-script output, a procedural macro Atlas cannot see through, or a git or path source;
  - per-item resolution through dependency re-exports (knowing that `serde::Serialize` is `serde_core::Serialize`, whatever an unseen include adds beside it);
  - `#[macro_export]` macros invoked by name from another module (G145 leaves those modules open);
  - a derive in G145's allowlist (serde's `Serialize` and `Deserialize`) is still trusted to emit no trait, as G145 decided. serde_derive wraps its output in an anonymous `const _`.
- **Queued:**
  - NA-DEPENDENCY-DECLARATIONS continues with that per-item step;
  - NA-MACRO-EXPORT-NAMES for the G145 residual.
