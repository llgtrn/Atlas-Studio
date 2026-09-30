---
id: atlas.decision.0096.declarations-written-in-unexpanded-macros
type: decision
status: accepted
canonical: true
---
# ADR 0096 — Declarations written where the walk cannot see hold declaration coverage UNKNOWN; unions and source directories with output names are censused (G184)

## Context

FULL_OSS_REPLAY R18 censused verus-lang/verus at d85f7578. 297 .rs files hold a `verus! { ... }` block at item position, and about 5,170 `fn` declarations are written inside those blocks, about 3,019 of them in `source/vstd`. Atlas recorded none of them: no function and no edge lies inside any `verus!` block. CALL was correctly UNKNOWN on those files. SYMBOL, TYPE, FUNCTION_IDENTITY and FUNCTION_SIGNATURE were OBSERVED on all of them.

The Rust extractor classes the four declaration dimensions `Exhaustive`. Its profile excludes "item-level function-like macro-expanded declarations" as OUT_OF_PROFILE. That exclusion was read as covering everything inside a macro invocation. It cannot cover a declaration whose text is in the file: `verus! { fn f() {} }` writes `fn f` in source, the walk records nothing inside the invocation, and the obligation still said OBSERVED. No gap named the missing declarations.

The review of this fix found the same false claim in other places:
- **Closures and `async` blocks** written inside a macro in a function body. Each is a FUNCTION_IDENTITY region (G133, G159).
- **Items in blocks the walk never enters**: `const`/`static` initializers and enum discriminants. The cxx expansion in the verus checkout has 16 `const _: () = { .. }` blocks holding 128 functions.
- **Local `macro_rules!` templates** invoked in the same file.
- **`union`, foreign `type` and trait aliases**, which the walk skipped.
- **Source directories with a build-output name.** The inventory ignored every directory named `build`, `dist`, `.next` or `coverage` as a "generated-or-external-directory-boundary". Atlas's own `core/src/coverage/mod.rs` was out of its census under that label.

## Decision

1. **The rule.** A declaration dimension of a file stays OBSERVED only if no place the walk cannot see into holds a declaration written in the file. Those places are:
   - an invocation of a macro the extractor does not expand, at item, associated-item, statement, expression, pattern or type position, anywhere in the file;
   - an item that `syn` keeps as unparsed tokens (`Verbatim`);
   - an item written in a block the walk does not enter: a `const`/`static` initializer (including one inside a function), an enum discriminant, or an expression inside a type.

   A place holds a declaration when:
   - its tokens spell one of `fn struct enum union trait impl type mod const static`, searched through nested groups (an identifier after `'` or `$` is not a keyword); or
   - it invokes a declaring `macro_rules!` of the same file, by any path whose last segment names it (`name!`, `self::`, `super::`, `crate::m::`, or `$crate::m::` inside another template), or through a `use` alias of one. The declaring templates are a fixpoint: a template declares when its transcribers spell a keyword or invoke a declaring template (`impl_all!` through `impl_one!`). The fixpoint is a worklist over reverse edges (callee to callers, alias target to alias). A name is queued again only when what it reaches grows, and that can happen at most three times, so the cost is linear in templates plus invocations. An 8,000-link chain takes 0.26 s, against 18.3 s for the pass-until-stable loop it replaced.

   Such a place holds SYMBOL and TYPE UNKNOWN. When `fn` is spelled (in the tokens or in the local template), it also holds FUNCTION_IDENTITY and FUNCTION_SIGNATURE UNKNOWN.

   Inside a function body, where closures and `async` blocks are regions, a macro that spells one, or invokes a same-file template that does, holds all four UNKNOWN.
   - A closure is a `|` or `||` where an expression starts, closed by a second `|` with a body after it.
   - An expression starts first in its group, after punctuation other than `?` and `'`, or after `return`, `move`, `async`, `break` or `yield`.
   - Pattern alternation (`A | B`), bit-or (`a | b`) and `Token![|]` are not closures.
   - An `async` block is `async` before a brace group or `move`.

   Records the walk did make are kept (UNKNOWN with observations).
2. **What is searched and what is not.**
   - A recovered standard macro (G119) has its arguments walked, so the search goes into those arguments rather than stopping at the macro.
   - A `macro_rules!` definition is a template. It declares nothing until it is invoked.
   - A closure outside a function body, such as in an initializer or an item macro, is not a region (G133) and is not counted.
3. **The reason is named.**
   - Each held dimension gets an `INCOMPLETE_ANALYSIS` diagnostic on its obligation. The message starts with `HIDDEN_DECLARATION_DIAGNOSTIC` (`"hidden declarations: "`).
   - The message names each place with its path and line, then a tag: `(fn)`, `(closure or async)`, or `(a local macro_rules! template, ..)`. Unparsed items are named `item`, and items in unentered blocks `item in an initializer` or `item in an unwalked block`. The first eight are listed, then a count. Each dimension names only the places that can hide its records: a struct-only place is named for SYMBOL and TYPE, not for FUNCTION_IDENTITY.
   - The composed world model reads the message back onto the component (`hidden_declarations`), onto the unknowns lens, and into `GAP-HIDDEN-DECLARATION` (DEBT-SYMBOL). The gap's magnitude is the number of such components.
   - These are text on an existing diagnostic and lists of text, not statuses, so no carrier is added to the vocabulary map.
4. **CALL.** The rule holds only the four declaration dimensions. CALL keeps its own rule (G119: exhaustive when `opaque_sites` is 0), with one change: a hidden place that holds a function CALL's own count does not see now counts as an opaque site. Those places are a `Verbatim` item, an item in an unentered block, and a macro in a `const`/`static` initializer. A function hidden there can hide calls.
5. **Unions, foreign types and trait aliases are recorded.**
   - A `union` is recorded like a struct with named fields: a SYMBOL with kind UNION and shape NAMED, a SYMBOL for each field, and a TYPE for each field type.
   - A foreign `type` is recorded as a SYMBOL declaration.
   - A trait alias is recorded as a SYMBOL definition.
6. **Directories with a build-output name.** The decision reads which files are present and no file's contents.
   - `.atlas`, `.git`, `.pnpm-store`, `target` and `node_modules` are ignored whatever they hold.
   - A directory `P/N` named `build`, `dist`, `.next` or `coverage` is walked as source only if one of these holds:
     - (a) it contains `mod.rs`;
     - (b) it contains `Cargo.toml`, so a package lives there;
     - (c) `P` contains `N.rs` that is not a crate root, and `P/N` contains at least one `.rs` file.
   - A crate root is `build.rs`, `main.rs` or `lib.rs` beside a `Cargo.toml`, or any file under a `src/bin`, `examples`, `tests` or `benches` directory.
   - Files are checked with `symlink_metadata`, so no symlink is followed. At most 4,096 entries of `P/N` are examined for a `.rs` file. If that cap is reached first, the directory keeps the boundary, and its reason says the decision was not made.
   - Otherwise the directory stays the IgnoredByExplicitPolicy "generated-or-external-directory-boundary", as before G184.
   - The following are disclosed boundaries exactly as before G184, and form one residual:
     - a module directory reached only through `#[path]`, `#[cfg_attr(.., path = ..)]`, `cfg_if!`, or an inline `mod N { mod x; }` with no `N.rs` or `mod.rs`;
     - a workspace member or Cargo `path`/`build` target inside such a directory;
     - non-Rust source under such a name.

## Each macro class

- **Macros with no workspace definition** (procedural macros such as `verus!`, dependency macros): their tokens are written source. If the tokens spell a declaration, the declaration dimensions are UNKNOWN. This is the falsified case.
- **Workspace `macro_rules!`:**
  - G145 (ADR 0062) bounds the names an invocation may define, for name resolution only, and records no declaration. A declaration written in the invocation is as missing as any other: `crate::vocabulary_enum! { pub enum E { .. } }` holds SYMBOL and TYPE UNKNOWN.
  - A template of the same file that declares (directly or through the templates it invokes) makes each of its invocations a hidden place, by any path or `use` alias. An example is `typed_id!(NodeId)` in `core/src/identity/mod.rs`. A same-file template that spells only closures (`string!` in `adapter/src/lib.rs`) hides regions when it is invoked in a body.
  - A template defined in another file, invoked with tokens that spell nothing, is read as generated. The extractor reads one file; this is a residual.
- **Derives, and attribute macros and their arguments:** they generate items that are not written in source, and the items they are attached to are recorded. They stay out of profile. A declaration spelled inside an attribute's arguments, such as `#[dep::attr(fn f() {})]`, is also out of profile by this choice.
- **Standard item macros:**
  - `thread_local!` declares statics, which the walk records as SYMBOL and TYPE, so it holds those UNKNOWN. It spells no `fn`.
  - `include!` spells no declaration; the included file is its own artifact.
  - `macro_rules!` is a template.

## Boundary

- **Still out of profile:** declarations a macro generates without spelling them in this file. This covers another file's transcriber, a procedural macro's output, DSL declarations that use no Rust keyword (verus `assume_specification`, `broadcast group`), derive and attribute output, and monomorphized instances.
- **UNKNOWN when it need not be:** a keyword that declares nothing (`*const T`, a `fn()` type, `parse_quote! { fn .. }`, `test_verify_one_file!` test data), and a `|`-led expression that is closed like a closure. Atlas cannot tell data from declarations without expanding. UNKNOWN is the honest answer and never a false claim.
- **Const-generic expressions** (`g::<{ fn f() {} 1 }>()`, a const parameter default, a method turbofish) are neither walked nor searched. This is a residual.
- **Crate-root territory is judged by path alone.** A `tests`, `examples` or `benches` component anywhere in the parent's path, or `src/bin`, marks crate-root territory, so a module directory such as `src/tests/coverage/` stays a boundary, as before G184.
- **Rule (c) does not check declarations.** It walks all of `P/N` when `P/N.rs` is a non-root module and `P/N` holds any `.rs` file, even if `N.rs` declares no out-of-line module. This is accepted as unrealistic for generated directories.
- **Directories** are recognized only by decision 6: `#[path]`, `cfg_attr`, `cfg_if!`, inline-module-only, workspace and Cargo-target module directories, and non-Rust source under an output name, stay generated-or-external boundaries as before. This is a residual.
- **Unchanged:** no function or edge is lost. Records change only where something written is newly censused: unions, and the re-admitted Atlas module. The `matches!` call miss remains a residual.

## Consequences (measured on the same tree; N = HEAD 023b782c binary, final = this decision)

- **Review fixtures.**
  - p1: every repro (initializer items and macros, a discriminant item, a local template, a type-position macro, a trait default body, closures after `,` and `break`) goes UNKNOWN with its place named. The foreign type, trait alias and union are recorded and keep OBSERVED. An attribute argument, a raw `r#fn`, a doc comment and `include!` keep OBSERVED.
  - p2 (first review): `#[path]`, inline-module and Cargo-target directories stay boundaries, as disclosed in decision 6. The root `build/`, beside a `build.rs` and a `Cargo.toml`, stays ignored. `pub macro m2() { fn inner() {} }` holds the declaration dimensions UNKNOWN, and holds CALL UNKNOWN as an opaque site under CALL's own diagnostic.
  - p3 (100,000-deep `src/coverage.rs`): no file is read. `src/coverage/` is walked by (c), and the extractor's own guard handles the deep file.
  - Second review:
    - the quadratic and allocation-failure fixtures (`qd_out_16000`, `m4`) complete in 0.02 s, and their directories stay boundaries;
    - `src/bin/build/` stays a boundary;
    - q1 chained templates (`c26`, `c27`), path forms (`c06`, `c07`, `c21`, `c22`), the alias (`c15`) and the closure template (`c14`) go UNKNOWN;
    - winnow's `parser.rs` (`impl_parser_for_tuples!` through `impl_parser_for_tuple!`) and tracing-core's `field.rs` (`impl_values!` through `impl_value!`) go UNKNOWN;
    - the const-generic cases (`c11`, `c13`, `c20`, `c23`) stay OBSERVED (residual).
- **verus d85f7578, N to final.**
  - 513 of 971 .rs files move SYMBOL and TYPE from OBSERVED to UNKNOWN; 482 of them also move FUNCTION_IDENTITY and FUNCTION_SIGNATURE. `GAP-HIDDEN-DECLARATION` = 513.
  - One file's CALL moves to UNKNOWN: `cargo-expand/cxx-1.0.69.rs`, with 128 functions in initializers.
  - The macro-keyword rule accounts for 494 files, including 289 of the 297 `verus!`-block files.
  - The closure rule accounts for 16 files.
  - Local templates account for these files: `vstd/arithmetic/overflow.rs`, `vstd/wrapping.rs`, `builtin/src/lib.rs`, `vir/src/printer.rs`, and in syn `token.rs`, `benches/rust.rs` and `tests/common/eq.rs`.
  - Initializer items account for the cxx and serde_derive expansions and `marker_traits.rs`.
  - `vstd/std_specs/atomic.rs` stays OBSERVED: its template holds only DSL declarations.
  - The two unions add 6 SYMBOL and 4 TYPE records (39,347 to 39,353, and 44,618 to 44,622).
  - No output-named directory exists in the checkout.
  - Functions stay 25,061 with identical ids. Relations stay 33,045 and identical. INVOKES stays 13,487.
- **Atlas itself (self-census snapshot, N to final).**
  - Artifacts: the `core/src/coverage` boundary becomes the Parsed artifact `core/src/coverage/mod.rs`. It brings SYMBOL 45, TYPE 34 observed and 8 derived, FUNCTION_IDENTITY 21, FUNCTION_SIGNATURE 10, and CALL 97 observed and 21 derived. INVOKES goes from 7,303 to 7,313. `runtime/src/agent.rs` and `runtime/src/lib.rs` gain 3 derived CALL records and 1 derived TYPE record that now resolve into it.
  - Obligations of `atlas.rust.source-semantic.v1`:

    | dimension | N | final |
    |---|---|---|
    | SYMBOL | OBSERVED 152 | OBSERVED 131, UNKNOWN 22 |
    | TYPE | OBSERVED 152 | OBSERVED 131, UNKNOWN 22 |
    | FUNCTION_IDENTITY | OBSERVED 152 | OBSERVED 148, UNKNOWN 5 |
    | FUNCTION_SIGNATURE | OBSERVED 152 | OBSERVED 148, UNKNOWN 5 |

    Every other dimension gains one obligation for the new artifact, with its usual status. No existing file's CALL status changes.
  - The snapshot's SYMBOL, TYPE, FUNCTION_IDENTITY and FUNCTION_SIGNATURE coverage is UNKNOWN.
  - SYMBOL and TYPE only, in 17 files, because of `vocabulary_enum!` with a written `pub enum` (50 enums unrecorded). The files are in `core/src`:
    - `atlasx/codec.rs`, `atlasx/precondition.rs`;
    - `composition/closure.rs`, `composition/lens.rs`, `composition/mod.rs`;
    - `construction/mod.rs`, `coverage/mod.rs`, `design/mod.rs`, `graph/mod.rs`, `integrity/mod.rs`;
    - `seal/gate.rs`, `seal/mod.rs`;
    - `semantic/function.rs`, `semantic/resource.rs`, `semantic/symbol.rs`;
    - `verification/mod.rs`, `weights/mod.rs`.
  - All four dimensions, in 5 files:
    - `core/src/identity/mod.rs`: 11 `typed_id!` invocations of the file's own template, whose ID types and methods no record holds;
    - `adapter/src/lib.rs`: the local `string!`, `boolean!` and `array!` templates, whose `unwrap_or_else(|| ..)` closures are invoked in `parse_repo_manifest` and recorded as no region;
    - `adapter/src/semantic/rust/locked_traits.rs` (lines 1429, 1484), `adapter/src/semantic/rust/resolve.rs` (378, 385) and `runtime/src/census/typescript_modules/tests.rs` (160): closures inside `matches!`.
- **Falsification.**
  - Fourteen extractor tests, two inventory tests (each directory arm, the crate-root exclusions, a generated root `build/`, the entry cap) and a runtime model test fix the rules.
  - The final set of 57 hand mutants, anchored to the final code, was all killed: 53 from round 4 whose code is unchanged, and 4 for the worklist (no enqueue, reversed call edge, reversed alias edge, empty initial queue). The long-chain test (20,000 links, both definition orders, plus a non-declaring control) fixes the worklist, and every file was restored byte-identical. The set covers each directory arm and crate-root exclusion, the cap, the template fixpoint, aliases, path matching, group search, template regions, declarations and `fn`, and the per-dimension listing, as well as the earlier rules.
  - Earlier rounds killed 17, 16 and 51 mutants. The text-scanner mutants were retired with the scanner.
