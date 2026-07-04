import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  relayDeploymentShapeDecision,
  relayManagedClientKeyAgreementRuntimeSmoke,
  relayManagedPayloadBlindFrameEncryptionSpike,
  relayManagedRuntimeReadinessGate,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-next-mode-planning");
const evidencePath =
  process.env.RA_PWA_RELAY_NEXT_MODE_PLANNING_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-next-mode-planning.json");

const decision = relayDeploymentShapeDecision();
const runtimeReadinessGate = relayManagedRuntimeReadinessGate();
const payloadBlindFrameEncryptionSpike = relayManagedPayloadBlindFrameEncryptionSpike();
const clientKeyAgreementRuntimeSmoke = relayManagedClientKeyAgreementRuntimeSmoke();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.productDefault, "live-loopback");
assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);
assert.ok(decision.guardrails.includes("product_default_remains_live_loopback"));
assert.ok(decision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));
assert.equal(
  clientKeyAgreementRuntimeSmoke.nextLocalSlice,
  "managed-relay-metadata-minimization-review",
);

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Choose the next Relay/M2 mode-planning slice after managed client key agreement runtime smoke",
  currentReadyMode: "self-hosted",
  productDefault: decision.productDefault,
  selectedNextMode: "managed",
  deferredMode: "managed-runtime",
  rationale: [
    "Private-network relay and the managed relay operations/control-plane/abuse-retention/payload-confidentiality/verifier-key/billing-quota/runtime-readiness-gate/payload-blind-frame-encryption/client-key-agreement slices are complete.",
    "Managed relay remains deferred because metadata minimization, key registry, quota enforcement, usage export, and support review evidence are still missing.",
    "The product default remains live-loopback while managed relay stays a deferred service path.",
  ],
  requiredNextEvidence: [
    "managed relay metadata minimization review",
    "managed relay public verifier-key registry runtime smoke",
    "live-loopback remains product default",
    "managed relay remains deferred until remaining runtime evidence exists",
  ],
  runtimeReadinessGate: {
    gateStatus: runtimeReadinessGate.gateStatus,
    implementationCanStart: runtimeReadinessGate.implementationCanStart,
    completedRuntimeEvidence: runtimeReadinessGate.completedRuntimeEvidence,
    remainingRuntimeEvidence: runtimeReadinessGate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: runtimeReadinessGate.remainingRuntimeBlockers,
  },
  payloadBlindFrameEncryptionSpike: {
    completedRuntimeEvidence: payloadBlindFrameEncryptionSpike.completedRuntimeEvidence,
    closedReadinessBlockers: payloadBlindFrameEncryptionSpike.closedReadinessBlockers,
    implementationCanStart: payloadBlindFrameEncryptionSpike.implementationCanStart,
  },
  clientKeyAgreementRuntimeSmoke: {
    completedRuntimeEvidence: clientKeyAgreementRuntimeSmoke.completedRuntimeEvidence,
    closedReadinessBlockers: clientKeyAgreementRuntimeSmoke.closedReadinessBlockers,
    implementationCanStart: clientKeyAgreementRuntimeSmoke.implementationCanStart,
  },
  nextLocalSlice: clientKeyAgreementRuntimeSmoke.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_NEXT_MODE_PLANNING_OK ${evidencePath}`);
