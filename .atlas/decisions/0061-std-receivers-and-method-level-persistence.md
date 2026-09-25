---
id: atlas.decision.0061.std-receivers-and-method-level-persistence
type: decision
status: accepted
canonical: true
---
# ADR 0061 — Std receivers through declared std signatures: method-level persistence and concurrency (G144)

## Context

The std-path persistence table has declared `std::fs::File::sync_all` since G125, but only path calls reach it. A method call such as `file.sync_all()` needs its receiver's std type, so PERSISTENCE never saw Atlas's own durability: the `.atlas` container writer forces the partial file, then its directory, to stable storage (`runtime/src/atlas.rs`).

DEBT-PERSISTENCE and DEBT-CONCURRENCY both named method receivers (`sync_all`, `lock`, `send`, `recv`) as their residual. The concurrency kinds `Lock`, `ChannelSend` and `ChannelReceive` were "reserved, never emitted", because a bare method name cannot be claimed.

## Decision

1. **A std receiver** (`LocalType::Std`) is one of:
   - a parameter or `let` whose declared type is a std type. Generic arguments are allowed, since an inherent std method is the same for every instantiation.
   - the result of a declared std function (`STD_RETURNS`: `File::create`, `File::open`, `Mutex::new`), with `?` unwrapping a fallible one.
2. **Nested lets.** `let`s anywhere in the body are typed now, each bounded by the end of its block. A binding bound once cannot be named elsewhere as a local, and the bound keeps a same-named static used after the block out of the claim.
3. **Claims.** On a std receiver, only a declared inherent std method (`STD_INHERENT`: `File::sync_all`, `File::sync_data`, `Mutex::lock`, `Receiver::recv`, `Sender::send`, each with its documented receiver form) is claimed. It is claimed as the std path `<type>::<method>`, which the declared tables turn into DERIVED records.
   - An inherent method wins the probe step its form matches.
   - A differing form is claimed only through the G142 autoref guards on the name (no by-value workspace trait method, not a blanket name, no non-std import or open scope).
   - Any other method is `std-method-undeclared`, never claimed.
4. **Tables.** The std concurrency table gains `Mutex::lock` (Lock), `Sender::send` (ChannelSend) and `Receiver::recv` (ChannelReceive).

## Evidence

`evidence/census/G144/std-receiver-scip-differential.json`:

- **Container writer.** Both fsync sites of the writer (`runtime/src/atlas.rs:225` and `:229`) are now DERIVED `SYNC` persistence. SCIP names `fs/impl#[File]sync_all()`, the inherent impl.
- **Nested lets.** Typing them adds 10 workspace method resolutions (5,773 → 5,783 on the same tree, none lost). All 10 agree with SCIP.
- **Concurrency.** No lock, send or recv site in Atlas has a known std receiver type. The resolver test exercises them.

Five mutants were each caught by a test:

- a std method's form ignored;
- fallibility ignored;
- undeclared std methods claimed;
- the block end ignored;
- std generic arguments rejected.

The self-census test asserts the container writer's `SYNC` site.

## What stays open

- **Table size.** The declared std tables are small by design, and grow only with documented contracts.
- **Trait methods.** `writer.flush()` is `Write::flush`, not inherent, so it stays uncovered.
- **Receiver shapes.** Std receivers on closure parameters, on fields of generic types and on untyped `let`s remain residual.
