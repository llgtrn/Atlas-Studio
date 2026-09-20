import test from "node:test";
import assert from "node:assert/strict";
import { buildIntelligenceBoundaryAtlas } from "./intelligence-boundary-census.mjs";

test("boundary atlas distinguishes real runtime from kernel proof and gaps", () => {
  const atlas = buildIntelligenceBoundaryAtlas({
    trackedFiles: [
      "core/src/holding.rs",
      "core/src/identity.rs",
      "core/src/resource.rs",
      "runtime/src/authority.rs",
      "runtime/src/privacy.rs",
      "runtime/src/context.rs",
      "runtime/src/resource_ownership.rs",
      "runtime/src/execution_admission.rs",
      "runtime/src/model_registry.rs",
      "runtime/src/model_evaluation.rs",
      "runtime/src/model_lifecycle.rs",
      "adapter/src/provider_adapter.rs",
      "adapter/src/context_disclosure_kernel_proof.rs",
      "adapter/src/model_routing_kernel_proof.rs",
    ],
    root: "/definitely/no/source/root",
    sourceSha: "1".repeat(40),
    generatedAt: "2026-09-20T00:00:00.000Z",
  });

  assert.equal(atlas.readiness.find((x) => x.id === "MODEL_ROUTING").state, "PROOF_ONLY");
  assert.equal(atlas.readiness.find((x) => x.id === "INTELLIGENCE_INVOCATION_CONTRACT").state, "GAP");
  assert.equal(atlas.readiness.find((x) => x.id === "HOLDING_KERNEL_VOCABULARY").state, "PRESENT");
});

test("boundary atlas reports provider-named runtime paths for review, not automatic hard failure", () => {
  const atlas = buildIntelligenceBoundaryAtlas({
    trackedFiles: ["runtime/src/openai_router.rs"],
    root: "/definitely/no/source/root",
    sourceSha: "2".repeat(40),
    generatedAt: "2026-09-20T00:00:00.000Z",
  });
  assert.equal(atlas.violations.length, 1);
  assert.equal(atlas.violations[0].severity, "REVIEW_REQUIRED");
  assert.equal(atlas.violations[0].type, "PROVIDER_SPECIFIC_RUNTIME_PATH_REVIEW");
});

test("strategic donor readiness follows canonical donor corpus source presence", () => {
  const atlas = buildIntelligenceBoundaryAtlas({
    trackedFiles: [],
    sourceSha: "3".repeat(40),
    generatedAt: "2026-09-20T00:00:00.000Z",
  });
  assert.equal(atlas.readiness.find((x) => x.id === "DONOR_PROVIDER_NEUTRAL_INFERENCE_PROTOCOL").state, "PRESENT");
  assert.equal(atlas.readiness.find((x) => x.id === "DONOR_MULTI_PROVIDER_ROUTING").state, "GAP");
  assert.match(
    atlas.readiness.find((x) => x.id === "DONOR_MULTI_PROVIDER_ROUTING").evidence[0],
    /D340 BerriAI\/litellm/,
  );
});
