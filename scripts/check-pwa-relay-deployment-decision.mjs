import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  relayDeploymentShapeDecision,
  relayTransportUxPreflight,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-deployment-decision");
const evidencePath =
  process.env.RA_PWA_RELAY_DEPLOYMENT_DECISION_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-deployment-decision.json");

const decision = relayDeploymentShapeDecision();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.selectedSubstrate, "websocket");
assert.equal(decision.productDefault, "live-loopback");
assert.equal(decision.relayTransportReadiness, "planned");
assert.equal(decision.endpointPolicy, "wss-production-localhost-ws-development");
assert.equal(decision.ticketSecretOwner, "daemon");
assert.equal(decision.ticketVerifierMode, "ed25519-public-verifier-preferred");
assert.equal(decision.payloadConfidentiality, "explicit-self-hosted-operator-trust-decision");
assert.deepEqual(decision.knownModes, ["self-hosted", "private-network", "managed"]);
assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);
assert.ok(decision.guardrails.includes("product_default_remains_live_loopback"));
assert.ok(decision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));
assert.ok(decision.guardrails.includes("hosted_relay_prefers_public_verifier_keys"));
assert.ok(decision.guardrails.includes("self_hosted_relay_operator_trust_required"));

const identity = {
  deviceId: "web-deploy1",
  noisePubkeyHex: "b".repeat(64),
  approvalPubkeyHex: "c".repeat(64),
};
const ticket = createRelaySessionTicket({
  sessionId: "relay-deployment-decision",
  sessionToken: "token_relay_deployment_decision_123456",
  issuedAtMs: 1000,
  expiresAtMs: 2000,
  daemonPubkeyHex: "a".repeat(64),
  companionDeviceId: identity.deviceId,
  companionNoisePubkeyHex: identity.noisePubkeyHex,
  companionApprovalPubkeyHex: identity.approvalPubkeyHex,
});
const signedTicket = await createSignedRelaySessionTicket(
  ticket,
  "relay-ticket-secret-1234567890abcdef",
  webcrypto,
);

const selfHostedReady = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    deploymentMode: "self-hosted",
    operatorSetupText: "Self-hosted relay setup is ready.",
  },
  1500,
);
assert.equal(selfHostedReady.status, "ready");
assert.deepEqual(selfHostedReady.blockers, []);

const managedDeferred = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    deploymentMode: "managed",
    operatorSetupText: "Managed relay setup is documented but deferred.",
  },
  1500,
);
assert.equal(managedDeferred.status, "hidden");
assert.ok(managedDeferred.blockers.includes("relay_deployment_mode_not_selected"));

const privateNetworkDeferred = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    deploymentMode: "private-network",
    operatorSetupText: "Private network relay setup is documented but deferred.",
  },
  1500,
);
assert.equal(privateNetworkDeferred.status, "hidden");
assert.ok(privateNetworkDeferred.blockers.includes("relay_deployment_mode_not_selected"));

const productDefaultHidden = relayTransportUxPreflight(
  {
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    deploymentMode: "self-hosted",
    operatorSetupText: "Self-hosted relay setup is ready.",
  },
  1500,
);
assert.equal(productDefaultHidden.status, "hidden");
assert.ok(productDefaultHidden.blockers.includes("transport_mode_not_relay"));

await mkdir(artifactRoot, { recursive: true });
const evidence = {
  status: "ok",
  generatedAt: new Date().toISOString(),
  objective: "Record the first RA/PWA relay deployment shape decision",
  decision,
  result: {
    selfHostedReady,
    managedDeferred,
    privateNetworkDeferred,
    productDefaultHidden,
  },
  rationale: [
    "Self-hosted WebSocket relay matches the current local bridge evidence without requiring managed infrastructure.",
    "Hosted relay code validates signed tickets and the service artifact now supports Ed25519 public verifier keys.",
    "The current self-hosted shape records an explicit relay-operator trust decision instead of claiming payload confidentiality.",
    "Managed relay and private-network modes stay documented candidates until separate operations and network evidence exist.",
    "The product default remains live-loopback while relay is still planned.",
  ],
};
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_DEPLOYMENT_DECISION_OK ${evidencePath}`);
