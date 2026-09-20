import test from "node:test";
import assert from "node:assert/strict";
import { buildConnectorAtlas } from "./connector-census.mjs";

test("protocol maturity requires source/proof/production evidence rather than docs", () => {
  const registry = {
    protocols: [
      { id: "OPC_UA", category: "industrial", donor_ids: ["D1"], production_evidence_paths: [], proof_evidence_paths: [] },
      { id: "MQTT", category: "iot", donor_ids: ["D2"], production_evidence_paths: [], proof_evidence_paths: ["adapter/src/mqtt_kernel_proof.rs"] },
      { id: "HTTP_API", category: "digital", donor_ids: [], production_evidence_paths: ["adapter/src/api_effect.rs"], proof_evidence_paths: [] },
    ],
  };
  const donorCorpus = {
    donors: [
      { id: "D1", source_present: true },
      { id: "D2", source_present: false },
    ],
  };
  const atlas = buildConnectorAtlas({
    registry,
    donorCorpus,
    trackedFiles: ["adapter/src/mqtt_kernel_proof.rs", "adapter/src/api_effect.rs"],
    root: "/definitely/no/source/root",
    sourceSha: "1".repeat(40),
    generatedAt: "2026-09-20T00:00:00.000Z",
  });

  assert.equal(atlas.protocols.find((x) => x.id === "OPC_UA").status, "SOURCE_PRESENT");
  assert.equal(atlas.protocols.find((x) => x.id === "MQTT").status, "PROOF_ONLY");
  assert.equal(atlas.protocols.find((x) => x.id === "HTTP_API").status, "PRODUCTION");
});

test("generic simulated robot code never proves a named physical protocol", () => {
  const atlas = buildConnectorAtlas({
    registry: {
      protocols: [
        { id: "ROS2", category: "robotics", donor_ids: [], production_evidence_paths: [], proof_evidence_paths: [] },
      ],
    },
    donorCorpus: { donors: [] },
    trackedFiles: ["adapter/src/robot_command_effect.rs"],
    root: "/definitely/no/source/root",
    sourceSha: "2".repeat(40),
    generatedAt: "2026-09-20T00:00:00.000Z",
  });
  assert.equal(atlas.protocols[0].status, "DISCOVERED");
  assert.equal(atlas.summary.physical_production_total, 0);
});
