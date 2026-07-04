import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { relayDeploymentShapeDecision } from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-next-mode-planning");
const evidencePath =
  process.env.RA_PWA_RELAY_NEXT_MODE_PLANNING_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-next-mode-planning.json");

const decision = relayDeploymentShapeDecision();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.productDefault, "live-loopback");
assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);
assert.ok(decision.guardrails.includes("product_default_remains_live_loopback"));
assert.ok(decision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Choose the next Relay/M2 mode-planning slice after explicit self-hosted readiness is green",
  currentReadyMode: "self-hosted",
  productDefault: decision.productDefault,
  selectedNextMode: "private-network",
  deferredMode: "managed",
  rationale: [
    "Private-network relay planning can reuse explicit operator-controlled trust boundaries before introducing managed service operations.",
    "Managed relay remains deferred until control-plane ownership, tenant isolation, abuse handling, support, and retention operations are designed.",
    "The product default remains live-loopback while private-network setup remains an explicit advanced path.",
  ],
  requiredNextEvidence: [
    "private-network endpoint discovery contract",
    "operator setup and authentication boundary",
    "PWA setup preflight for private-network endpoints",
    "daemon runtime guardrails that keep public ws blocked",
  ],
  nextLocalSlice: "private-network-relay-setup-contract",
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_NEXT_MODE_PLANNING_OK ${evidencePath}`);
