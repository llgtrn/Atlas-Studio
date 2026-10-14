# Benchmark truth engine

**SOURCE-SCOPED BENCHMARK IMPLEMENTATION — not a second Chronica product runtime or architecture authority.** This local verifier recomputes FIC, family admission, five-plane data, NO_AI taxonomy, contract relationships and first-vertical status from a pinned repository SHA. Native implementation, bounded caller evidence and verified external proof must still be admitted by the common Ynventa Foundry and Chronica (Lineara, Thymora and Porion).

The existing source-specific verdict `PROVISIONAL_BASELINE` / `B — HARNESS-HEAVY HYBRID` is a benchmark result, never a canonical V1 completion claim. Never infer Chronica product readiness or donor extinction from it.

WEB2APP (`205-web2app-standard.md` (source design label; `docs/benchmarks/` is not tracked in this checkout)) is the application-completeness overlay (W0–W5 plus `1.1.0` observation maturity O0–O5). It does not replace FIC or the five-plane freeze. Observation fields are optional on `chronica.web2app.evaluation.v1`; absent fields default to O0.

Agent Employment Contract (`206-agent-employment-contract-standard.md` (source design label; `docs/benchmarks/` is not tracked in this checkout)) is the digital-worker employment overlay (E0–E6). It does not implement CAP16. Fixtures under `fixtures/employment/` test the benchmark engine; they are not Chronica runtime. WEB2APP W-level and employment E-level are independent.

Chronica Store Platform (`207-chronica-store-platform-standard.md` (source design label; `docs/benchmarks/` is not tracked in this checkout)) is the hosted business application estate overlay (S0–S6). It does not implement CAP17. Fixtures under `fixtures/store/` test the benchmark engine; they are not Chronica runtime. W-level, E-level, and S-level are independent.

Confidential Black-Box Data Plane (`208-chronica-confidential-blackbox-data-plane-standard.md` (source design label; `docs/benchmarks/` is not tracked in this checkout)) is the disclosure/confidentiality overlay (B0–B6). It does not implement CAP18. Fixtures under `fixtures/blackbox/` test the benchmark engine; they are not Chronica runtime. W/E/S and B-level are independent.

The Benchmark Contract Compiler (`209-benchmark-contract-compiler.md` (source design label; `docs/benchmarks/` is not tracked in this checkout), schema `chronica.benchmark.contract.v1`) sits above these evaluators. It compiles flow contracts, CAP envelopes, proof obligations, CAP packets, a pinned release (`BR-2026.08.21.1`), and a CI matrix. Generated packets are not source of truth. Do not create a second permanent benchmark or developer-governance engine. Keep this source verifier usable for its bounded, versioned oracles during migration; preserve its checks while migrating shared technology and evidence into Chronica.

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

**Executable scope check:** the listed `.mjs` commands below exist under `tools/benchmark/engine/` at the inspected revision. They are source-local oracle runners, not Chronica WorkRun/spine/CAP16/CAP17/CAP18 implementations. The `docs/benchmarks/*.md` documents cited by former source design notes do **not** exist in the tracked repository, so those labels cannot be treated as currently available specifications. Preserve source code and fixtures; reject fabricated acceptance claims or phantom links.

**Named architecture boundary:** this benchmark is not Codara, Ontogen or Myelion as a deployed subsystem. Those are registered conceptual functions, and a benchmark becomes Lineara evidence only through scoped verification and Chronica admission. No UI is authored in this checkout; product rendering belongs to `Chronica/phenotype/` and can express many Phenora.
