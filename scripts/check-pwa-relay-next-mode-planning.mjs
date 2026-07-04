import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  relayDeploymentShapeDecision,
  relayManagedPayloadConfidentialityPlan,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-next-mode-planning");
const evidencePath =
  process.env.RA_PWA_RELAY_NEXT_MODE_PLANNING_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-next-mode-planning.json");

const decision = relayDeploymentShapeDecision();
const payloadPlan = relayManagedPayloadConfidentialityPlan();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.productDefault, "live-loopback");
assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);
assert.ok(decision.guardrails.includes("product_default_remains_live_loopback"));
assert.ok(decision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));
assert.equal(payloadPlan.nextLocalSlice, "managed-relay-verifier-key-operations-policy");

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Choose the next Relay/M2 mode-planning slice after managed payload confidentiality is specified",
  currentReadyMode: "self-hosted",
  productDefault: decision.productDefault,
  selectedNextMode: "managed",
  deferredMode: "managed-runtime",
  rationale: [
    "Private-network relay and the managed relay operations/control-plane/abuse-retention/payload-confidentiality slices are complete.",
    "Managed relay remains deferred until verifier-key operations and billing/quota contracts are specified before any managed runtime implementation.",
    "The product default remains live-loopback while managed relay stays a deferred service path.",
  ],
  requiredNextEvidence: [
    "managed relay verifier-key operations policy",
    "managed relay verifier-key rotation and revocation evidence",
    "managed relay billing and quota policy",
    "live-loopback remains product default",
    "managed relay remains deferred until verifier-key operations are green",
  ],
  nextLocalSlice: payloadPlan.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_NEXT_MODE_PLANNING_OK ${evidencePath}`);
