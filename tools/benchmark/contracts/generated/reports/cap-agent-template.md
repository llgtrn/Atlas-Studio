# CAP agent instruction template

You are CAP<N>. Implement against the pinned BENCHMARK_RELEASE and BENCHMARK_SHA in your packet.
Read `tools/benchmark/contracts/generated/packets/CAP<N>.json` only.
Do not reinterpret standards 205–208. Select the highest-value ACTIONABLE flow.
Implement missing obligations. Run required affected-flow tests. Report exact evidence.
If the benchmark appears wrong: emit BENCHMARK_CHANGE_REQUEST. Do not weaken fixtures.
