import test from "node:test";
import assert from "node:assert/strict";
import { buildEvolutionReadiness, buildOrganismModelAtlas, isProductionModelArtifactPath, selectOrganismDonors } from "./organism-model-census.mjs";

test("production model artifacts are forbidden outside temporary donor source", () => {
  assert.equal(isProductionModelArtifactPath("organism/models/a.safetensors"), true);
  assert.equal(isProductionModelArtifactPath("adapter/models/a.gguf"), true);
  assert.equal(isProductionModelArtifactPath("temporary/donors/D1-x/source/fixture.pt"), false);
  assert.equal(isProductionModelArtifactPath("docs/example.md"), false);
});

test("organism donor selection is explicit, not every AI_RUNTIME donor", () => {
  const checklist = {
    donors: [
      { id: "D1", repo: "x/a", family: ["AI_RUNTIME"], domain: null },
      { id: "D2", repo: "x/b", family: ["AI_RUNTIME"], domain: "organism" },
      { id: "D3", repo: "x/c", family: ["OTHER"], domain: null, organism_roles: ["AUDIO_PERCEPTION"] },
    ],
  };
  assert.deepEqual(selectOrganismDonors(checklist).map((d) => d.id), ["D2", "D3"]);
});

test("organism model atlas preserves source/weight/runtime distinctions", () => {
  const checklist = {
    donors: [
      {
        id: "D319",
        repo: "facebookresearch/vjepa2",
        domain: "organism",
        status: "SOURCE_PRESENT",
        needed: "YES",
        source_present: true,
        census_complete: false,
        absorption_state: "none",
        organism_roles: ["PREDICTIVE_REPRESENTATION"],
        chronica_targets: ["organism::world_model"],
        absorbed_into: [],
        temporary_path: "temporary/donors/D319-facebookresearch-vjepa2/source",
      },
      {
        id: "D324",
        repo: "QwenLM/Qwen3",
        domain: "organism",
        status: "DISCOVERED",
        needed: "YES",
        source_present: false,
        census_complete: false,
        absorption_state: "none",
        organism_roles: ["LANGUAGE_REASONING"],
        chronica_targets: ["organism::cognition"],
        absorbed_into: [],
        temporary_path: null,
      },
    ],
  };
  const atlas = buildOrganismModelAtlas({
    checklist,
    trackedFiles: [
      "organism/src/world_model.rs",
      "runtime/src/model_registry.rs",
      "adapter/src/linear_world_model.rs",
      "adapter/models/bad.safetensors",
      "temporary/donors/D319-facebookresearch-vjepa2/source/fixture.pt",
    ],
    root: "/definitely/no/provenance/here",
    sourceSha: "1".repeat(40),
    generatedAt: "2026-09-20T00:00:00.000Z",
  });

  assert.equal(atlas.summary.donors_total, 2);
  assert.equal(atlas.summary.source_present_total, 1);
  assert.equal(atlas.summary.not_ingested_total, 1);
  assert.equal(atlas.summary.production_model_artifacts_in_git_total, 1);
  assert.equal(atlas.violations[0].path, "adapter/models/bad.safetensors");
  assert.deepEqual(atlas.roles, ["LANGUAGE_REASONING", "PREDICTIVE_REPRESENTATION"]);
});


test("evolution readiness distinguishes proof-only routing from real runtime materialization", () => {
  const readiness = buildEvolutionReadiness([
    "runtime/src/experience.rs",
    "runtime/src/dataset_registry.rs",
    "runtime/src/model_training.rs",
    "runtime/src/model_evaluation.rs",
    "runtime/src/model_registry.rs",
    "runtime/src/model_lifecycle.rs",
    "adapter/src/model_routing_kernel_proof.rs",
  ], "/definitely/no/source/root");

  const routing = readiness.find((x) => x.id === "MODEL_ROUTING");
  const learningUse = readiness.find((x) => x.id === "LEARNING_USE_DECISION_RUNTIME");
  const replacementProof = readiness.find((x) => x.id === "MODEL_REPLACEMENT_PROOF_RUNTIME");
  const regressionGate = readiness.find((x) => x.id === "CAPABILITY_REGRESSION_GATE_RUNTIME");
  const mergeCompatibility = readiness.find((x) => x.id === "MODEL_MERGE_COMPATIBILITY_RUNTIME");
  const weightRetirement = readiness.find((x) => x.id === "WEIGHT_RETIREMENT_DELETION_RUNTIME");
  assert.equal(routing.state, "PROOF_ONLY");
  assert.equal(learningUse.state, "GAP");
  assert.equal(replacementProof.state, "GAP");
  assert.equal(regressionGate.state, "GAP");
  assert.equal(mergeCompatibility.state, "GAP");
  assert.equal(weightRetirement.state, "GAP");
});
