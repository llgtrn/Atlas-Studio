---
id: atlas.decision.0090.an-import-of-a-known-item-brings-that-item-alone
type: decision
status: accepted
canonical: true
---
# ADR 0090 — An import of a known item brings that item alone (G177)

## Context

G177 took NA-CALL-TYPE-RESIDUAL, the native queue head (pressure 85), with NA-DEPENDENCY-DECLARATIONS step two, which G175 measured as the largest residual on Atlas.

**The residual.** 431 oracle-confirmed `&self` method calls were refused at the autoref step, beside `use serde::{Deserialize, Serialize}`.

**Why the guard refused them.** Since G173 (ADR 0087), the method probe's autoref guard trusts a registry import when the import's package's whole lock closure declares no trait method of the called name. serde's closure is UNKNOWN: its crate root splices build-script output, `include!(concat!(env!("OUT_DIR"), "/private.rs"))`, which could declare any trait.

**Why the names are certain anyway.** An import names one item, and that name can be read with certainty:
- serde's root binds `Serialize` and `Deserialize` by a named `pub use serde_core::{..}`, inside its zero-argument local `crate_root!()`.
- An unseen item of the same name in the same namespace would not compile: E0255 against a `use`, E0252 between imports, E0428 between items. This holds even when the item comes from a `macro_rules!` expansion or an `include!`.
- So the named binding is certain whatever the include adds beside it.
- serde_core declares each trait in its own sources.

## Decision

1. **Registry imports keep their path.** A registry import's binding keeps the path below its package's root: `Def::External(.., Registry { package, path })`, extended one segment per hop. When one binding reaches the same package by two paths, the path is dropped and the G173 closure rule applies.
2. **`item_methods(package, path)`** resolves the item through the package's module tree, as read from the registry sources Cargo extracts (still data, never built or run), to the one item it names. The answer is one of:
   - **its own trait methods**, if it is a trait whose body is fully read. A supertrait's methods are not in scope through a subtrait import (E0599);
   - **none**, if it is certainly no trait: a struct, enum, union, a type alias to a non-trait, or a module. A procedural-macro crate exports nothing in the type namespace;
   - **UNKNOWN**, in every other case:
     - a name bound only in the value or macro namespace (a `fn` beside an unseen `trait` of its name compiles);
     - a name reached only through a glob or an unseen expansion;
     - a trait alias, or a trait body holding a macro, a verbatim item or an unknown attribute;
     - an unreadable package, or a module file it will not follow;
     - an exhausted budget: 256 `cfg` assignments, or 32 hops or expansions;
     - more than one active binding for a name.
3. **What the reading sees.** A binding counts only when the reading sees it:
   - an item or named `use` written in the module;
   - or an item that the transcriber of a local `macro_rules!` spells, when that macro has a single empty rule, is invoked as an item with empty input, and its definition is in textual scope there. Textual scope covers same-file definitions and `#[macro_use] mod` definitions, in order; later ones shadow earlier ones, and an opaque expansion before the invocation ends the certainty.
   - Expanded `mod x;` and `#[path]` resolve from the invoking module.
4. **Paths.** A path is walked segment by segment:
   - **First segment, 2018+:** a name the module visibly binds wins; otherwise the extern prelude, built from the manifest's normal-dependency keys (renames included) matched to the lock's direct dependencies. An unseen shadowing of a crate name would be E0659.
   - **2015 editions:** paths start at the crate root.
5. **`cfg` is evaluated per assignment.** Predicates are kept as expressions, and a feature no build can enable is false (G173's rule).
   - A query evaluates every assignment of the atoms it reaches, taken as independent (a superset of the real builds).
   - The answer is the union over assignments, UNKNOWN if any assignment is.
   - An assignment under which a module file on the path is missing does not compile and answers nothing. This is how serde's `docsrs` branch without serde_core drops out.
6. **The guard uses the per-item answer.** It trusts an import of a known item when `item_lacks(package, path, name)`: the per-item answer lacks the name, or, when that answer is UNKNOWN, the closure answer does. A registry glob still opens its module.
7. **Per-package `cfg` atoms and crate renames** (G177 review):
   - Every `cfg` atom is qualified by its package: a feature, a build-script `--cfg` or a profile setting is each package's own switch.
   - A file module this reading does not walk leaves an opaque point in the textual scope when it carries `#[macro_use]`: its macros may shadow later expansions.
   - A crate-root `extern crate x as n;` or `extern crate self as n;` decides `n` in every module, on both the registry side and in Atlas's own resolver. There, one under `cfg`, one carrying a possibly procedural attribute, or two of one name make `n` unknown.
   - An unrenamed dependency is known by its library name (`[lib] name`), not its manifest key. On the registry side and in the workspace's foreign map, one whose library name differs from its key is dropped.
8. **Digest.** Every file read already entered the resolution batches' input digest (G173); each package's module tree now enters it too. A machine without the registry sources reads no tree and makes the G173 claims; only the digest differs.

## Evidence

**Real crates.** Answers in Atlas's lock:
- `serde::Serialize` → {serialize};
- `serde::Deserialize` → {deserialize, deserialize_in_place};
- `serde::de::DeserializeOwned` → {};
- `serde_json::{Value, Map}`, `tree_sitter::{Node, Parser}` → {} (not traits);
- `serde_json::from_str` → UNKNOWN (value namespace).

**Atlas itself.** The G176 guard and this one were run over the same sources.
- Resolved path calls: 9,677 → 10,051 (+374). Unresolved: 2,852 → 2,482. External and dynamic unchanged. Atlas has no `extern crate` items, and none of its 13 direct dependencies has a library name that differs from its key, so the review fixes change no input of its own.
- Checked against rust-analyzer 1.90.0's SCIP index of the tree:
  - of the 375 changed call sites, 374 are confirmed when columns are compared in bytes, 4 of them on lines holding a multi-byte `±`;
  - the 375th is the edited guard line itself;
  - none withdrawn.
- Resolve time is unchanged.

**Rust facts checked with rustc 1.90.0:**
- E0255 for an item beside a named `use`, including from a `macro_rules!` expansion and an `include!`;
- E0252 for two uses; E0428 for two items; a glob silently shadowed by an item;
- E0599 for a supertrait method through a subtrait-only import;
- a `fn` and a `trait` of one name coexisting;
- a proc-macro crate exporting no trait;
- E0659 for a shadowed extern crate name reached through a glob, a `macro_rules!` expansion or an `include!`;
- `#[macro_use] mod` textual order;
- `#[path]` children resolving relative to their file.

**Tests:**
- `locked_traits/tests.rs`, module `per_item`:
  - a serde-shaped facade, and a core package defining `crate_root!` in another file;
  - a submodule re-export; supertraits excluded; non-traits;
  - a glob, a value-only `fn`, a trait body with a macro or a procedural attribute; a missing package; missing sources;
  - single-, multi-rule, parameterised and nested macros; textual scope and shadowing;
  - `cfg` alternatives; renamed dependencies; `::` paths; 2015 `extern crate .. as ..`;
  - the digest.
- `resolve/tests.rs`: `a_registry_import_of_a_known_item_withholds_autoref_only_when_that_item_may_declare_the_name`.

**Review.** An independent adversarial review found five holes, each reproduced on rustc 1.90.0 and with an Atlas fixture. A second pass confirmed all five closed and found two more, closed here too:
- **First pass:**
  - `cfg` atoms were shared across packages: a build with one package's feature on and another's off was never tried;
  - an unread `#[macro_use]` module's macros left no mark in the textual scope;
  - a crate-root `extern crate c as b` was ignored, both on the registry side and in the resolver. The resolver case is older than this change, but this change now claims through it;
  - dependencies were named by manifest key, not library name.
- **Second pass:**
  - a procedural attribute on a root `extern crate` was trusted by the resolver;
  - the workspace foreign map (`runtime/src/census/resolution.rs`) had the library-name hole too.

Each hole is now a regression test. The review found the visible-binding premise sound (E0255, E0252, E0428, E0659 cases, tool attributes, local shadowing). It also found sound the supertrait exclusion, namespaces, module-file rules, budgets, digest coverage, the version union, the merge rule and edition 2015.

**Mutants:** 29 killed, each run applying one textual mutation and restoring the tree byte-identical. Eight were added for the review fixes: unqualified atoms; no opaque point for an unwalked module; the prelude ignored; library names ignored; the resolver override ignored; a `cfg`'d override read as unconditional; a procedural attribute ignored; the foreign-map library rule bypassed. The first 21:
- globs as certain; parameterised or multi-rule macros expanded;
- supertrait methods included; own methods excluded; the per-item answer ignored; UNKNOWN as empty, twice;
- `cfg` ignored, twice; cross-file `#[macro_use]` before its definition; `#[macro_use]` ignored; textual shadowing ignored;
- proc-macro crates not empty; missing module files read; trait-body macros skipped; procedural attributes on trait items ignored;
- `fn` read as a certain non-trait; two spellings keeping a path; edition 2015 ignored; renames keyed by package name.

## Consequences

- **DEBT-CALL / DEBT-DEPENDENCY:** the guard reads each import item by item where it can see the binding. Capability epoch E23.
- **Still UNKNOWN:**
  - names reached only through globs, value- or macro-namespace names, and standard-library re-exports (`core::…`, not in the lock);
  - macros with parameters or several rules, macros from other crates, and procedural macros;
  - workspace path dependencies and git sources (rustc's own crates at R14);
  - the 90 remaining `receiver-form-differs` refusals on Atlas: 40 in `core/src/composition/lens.rs`, withheld by another guard rule, next to examine.
