---
id: atlas.decision.0039.concurrency-from-resolved-std-paths
type: decision
status: accepted
canonical: true
---
# ADR 0039 — Concurrency sites derived from resolved standard-library paths

## Context

The G116 audit (ADR 0038) put `NA-CONCURRENCY-RESOLVED` at the head of the native attack queue. The census recorded zero CONCURRENCY records. That zero was falsified: Atlas runs a scoped worker thread twice, in `adapter/src/semantic/rust/mod.rs` and `adapter/src/semantic/rust/resolve.rs`.

The existing engine could not see those sites, for three reasons:
- It records `.await` syntax and callee spellings that end in `spawn`. `spawn_scoped` does not end in `spawn`.
- Method receivers have no type, so a `.spawn_scoped(..)` method call cannot be tied to `std`.
- Both spawns sit inside the closure passed to `std::thread::scope`, and closure bodies are excluded from every dimension's profile.

The rust-analyzer 1.90.0 SCIP index measured the workspace's whole standard-library concurrency surface:
- two `std::thread::scope` calls, each wrapping `Builder::new().stack_size(..).spawn_scoped(..)` and a `join()`;
- one `thread::current().id()`, which reads a thread's identity and is not a concurrency operation.

No Mutex, Arc, channel or atomic is used anywhere.

## Decision

1. **A declared std-path concurrency table.** `atlas_core::std_path_concurrency`, sorted and looked up exactly, maps:
   - `std::thread::scope` to `THREAD_SCOPE`, a new `ConcurrencyKind`. It opens a structured thread scope, and every thread spawned in the scope is joined before the call returns.
   - `std::thread::spawn` to `SPAWN`.
   - `std::sync::mpsc::channel` and `std::sync::mpsc::sync_channel` to `CHANNEL_CREATE`.

   A path absent from the table declares nothing. That never means "no concurrency".
2. **The resolution engine derives CONCURRENCY.** `atlas.resolution.rust-paths` is now asked for CALL, CONCURRENCY, EFFECT and TYPE.
   - A path call resolved to a declared path is a DERIVED concurrency site. It is attributed to the caller named by its syntactic CALL claim and anchored at the call.
   - A declared call without a claim is a diagnosed disagreement.
   - The engine's CONCURRENCY obligation is UNKNOWN, because method calls, closure bodies and macro arguments are outside it.
3. **Not adopted.** Receiver-typed method concurrency (`spawn_scoped`, `lock`, `send`, `recv`, atomics) needs typed receivers. Attribution inside closures needs executable-region identity, now queued as `NA-CLOSURE-REGIONS`. Neither is guessed from spellings.

## Consequences

- The census now derives 2 CONCURRENCY records (`THREAD_SCOPE`), where it had 0. The SCIP oracle resolves both anchors to `std … thread/scoped/scope()` at the same position (2 of 2).
- CONCURRENCY has two engines, so the certificate's multi-engine blocker no longer names it.
- Each Rust artifact gains the new engine's UNKNOWN CONCURRENCY obligation (+97 unknown facts), which the G117 proof explains.
- Coverage stays UNKNOWN. The remaining residual is recorded in the debt ledger: 2 `spawn_scoped` and 2 `join` calls inside closures.
- Falsification: 5 mutants, all killed:
  - no derivation;
  - a guard that skips paths only the concurrency table declares;
  - `scope` mislabelled as `SPAWN`;
  - the engine not asked for CONCURRENCY;
  - the site overclaimed as OBSERVED.
