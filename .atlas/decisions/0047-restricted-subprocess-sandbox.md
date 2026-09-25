---
id: atlas.decision.0047.restricted-subprocess-sandbox
type: decision
status: accepted
canonical: true
---
# ADR 0047 — The restricted-subprocess sandbox (G126, NA-SANDBOX-SUBPROCESS)

## Context

`DEBT-SANDBOXED_EXECUTION` had been contract-only since G35. Builds and tests ran with ambient authority, and no `SandboxBackend` existed: podman (G106) found no consumer, and wasm-tools, wasmtime and nix exhausted its skip budget. After G125 the tightened pressure selection made it the queue head (construction node M16).

Through the Agent-Worn interface, Atlas answered where process-spawning authority originates: Runtime 4 functions, Adapter 8, CLI and Core none. All are `INFERRED`, because spawning goes through methods Atlas does not yet resolve.

## Decision

1. **Vocabulary** (`core::sandbox`):
   - A `SandboxRequest` declares the program, its arguments, digest-bearing input files confined to the staging root, the only environment the run sees, expected outputs, a timeout and network denial.
   - A `SandboxRun` records the request digest (order-independent), the outcome (`EXITED`, `TIMED_OUT` or `REFUSED` before running), the exit code, input and output digests, stdout and stderr digests, and every `IsolationProperty` as `ENFORCED` (with its mechanism) or `UNENFORCED` (with its reason).
2. **The restricted-subprocess backend** (`runtime::sandbox`):
   - **Refusal before running.** A missing, tampered or escaping input refuses the run, and so does a program absent from the declared `PATH`. Nothing runs.
   - **Enforced:** the environment is cleared and only the declared one supplied; declared inputs are copied, digest-verified, into a fresh directory; the working directory is that directory; stdin is closed.
   - **Timeout.** The run's own process group is killed at the timeout. Killing only the child left a `sleep` descendant alive and holding the output pipes; the first test run caught this.
   - **Network.** Denied through `unshare --user --map-root-user --net` only when a probe shows unprivileged namespaces work; the run then sees loopback alone. Otherwise the record says `UNENFORCED`.
   - **Filesystem confinement is never claimed.** Absolute-path and `..` reads are not confined: Atlas carries no `unsafe` code, so this class sets up no Landlock ruleset or mount namespace.
3. **`atlas-systemizer sandbox probe`** reports what this host enforces.

## Falsification

Five mutants were each caught by a test:

- the environment inherited;
- digests unverified;
- filesystem confinement claimed;
- network denial claimed without a namespace;
- only the direct child killed.

The timeout test proves that no descendant outlives the timeout: a background process that would write a marker after the kill never writes it.

## What stays open

`DEBT-SANDBOXED_EXECUTION` advances (contract-only to partial) but its success condition, that a sandboxed build cannot read outside its declared inputs, does not yet hold.

- **Next attack:** `NA-SANDBOX-FS-CONFINEMENT`, which brings filesystem confinement without introducing `unsafe` code and records toolchain identity per run.
- **Still outside the sandbox:** the generation gates (cargo build and test).
- **Construction node M16 stays `MISSING`:** it also requires toolchain identity and runs M15's delegated backend.
