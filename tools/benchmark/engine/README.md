# Benchmark truth engine

Local tooling that recomputes FIC, family admission, five-plane data, NO_AI taxonomy, contract relationships, and first-vertical status from an exact repository SHA.

Architecture verdicts stay frozen as `PROVISIONAL_BASELINE` / `B — HARNESS-HEAVY HYBRID` until post-Thread4 product-truth main.

WEB2APP (`docs/benchmarks/205-web2app-standard.md`) is the application-completeness overlay (W0–W5 plus `1.1.0` observation maturity O0–O5). It does not replace FIC or the five-plane freeze. Observation fields are optional on `chronica.web2app.evaluation.v1`; absent fields default to O0.

Agent Employment Contract (`docs/benchmarks/206-agent-employment-contract-standard.md`) is the digital-worker employment overlay (E0–E6). It does not implement CAP16. Fixtures under `fixtures/employment/` test the benchmark engine; they are not Chronica runtime. WEB2APP W-level and employment E-level are independent.

Chronica Store Platform (`docs/benchmarks/207-chronica-store-platform-standard.md`) is the hosted business application estate overlay (S0–S6). It does not implement CAP17. Fixtures under `fixtures/store/` test the benchmark engine; they are not Chronica runtime. W-level, E-level, and S-level are independent.

Confidential Black-Box Data Plane (`docs/benchmarks/208-chronica-confidential-blackbox-data-plane-standard.md`) is the disclosure/confidentiality overlay (B0–B6). It does not implement CAP18. Fixtures under `fixtures/blackbox/` test the benchmark engine; they are not Chronica runtime. W/E/S and B-level are independent.

The Benchmark Contract Compiler (`docs/benchmarks/209-benchmark-contract-compiler.md`, schema `chronica.benchmark.contract.v1`) sits above these evaluators. It compiles flow contracts, CAP envelopes, proof obligations, CAP packets, a pinned release (`BR-2026.08.21.1`), and a CI matrix. Generated packets are not source of truth. Do not replace this engine with Engine V2.

```bash
node tools/benchmark/engine/run.mjs
node --test tools/benchmark/engine/engine.test.mjs
node tools/benchmark/engine/web2app-run.mjs --fixtures
node --test tools/benchmark/engine/web2app.test.mjs
node tools/benchmark/engine/employment-run.mjs --fixtures
node --test tools/benchmark/engine/employment.test.mjs
node tools/benchmark/engine/store-run.mjs --fixtures
node --test tools/benchmark/engine/store.test.mjs
node tools/benchmark/engine/blackbox-run.mjs --fixtures
node --test tools/benchmark/engine/blackbox.test.mjs
node tools/benchmark/engine/contract-run.mjs compile
node --test tools/benchmark/engine/contract.test.mjs
```

Does not push, open PRs, or implement product WorkRun/spine/CAP16/CAP17/CAP18 runtime.
