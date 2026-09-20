# P0 CI integration handoff

P0_AUTHORITATIVE_CI_TOUCHED: NO
P0_INTEGRATION_REQUIRED: YES

Do not modify `.github/workflows` from the benchmark branch.
When a later explicit task authorizes P0 integration:
1. Add one data-driven job that runs `pnpm benchmark:contracts:verify`.
2. Consume `benchmark:ci-matrix` JSON for affected CAP sharding.
3. Keep TIER2 global hard invariants as merge gates.
4. Do not create one workflow file per flow.
