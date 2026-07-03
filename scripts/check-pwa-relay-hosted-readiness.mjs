import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
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
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-hosted-readiness");
const evidencePath =
  process.env.RA_PWA_RELAY_HOSTED_READINESS_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-hosted-readiness.json");

const daemonPath = path.join(repoRoot, "src", "daemon.rs");
const pwaPath = path.join(repoRoot, "pwa", "app.mjs");
const runbookPath = path.join(repoRoot, "docs", "relay-self-hosted-runbook.md");
const daemonSource = await readFile(daemonPath, "utf8");
const pwaSource = await readFile(pwaPath, "utf8");
const runbook = await readFile(runbookPath, "utf8");

const decision = relayDeploymentShapeDecision();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.selectedSubstrate, "websocket");
assert.equal(decision.productDefault, "live-loopback");
assert.equal(decision.endpointPolicy, "wss-production-localhost-ws-development");

const identity = {
  deviceId: "web-hosted1",
  noisePubkeyHex: "b".repeat(64),
  approvalPubkeyHex: "c".repeat(64),
};
const ticket = createRelaySessionTicket({
  sessionId: "relay-hosted-readiness",
  sessionToken: "token_relay_hosted_readiness_123456",
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

const pwaHostedSetup = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/relay",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    deploymentMode: "self-hosted",
    operatorSetupText: "Self-hosted relay setup is ready.",
  },
  1500,
);
assert.equal(pwaHostedSetup.status, "ready");
assert.deepEqual(pwaHostedSetup.blockers, []);

const daemonWssFailClosed =
  daemonSource.includes('url.starts_with("wss://")') &&
  daemonSource.includes('url.strip_prefix("ws://")') &&
  daemonSource.includes('matches!(host.as_str(), "localhost" | "127.0.0.1")');
assert.equal(daemonWssFailClosed, true, "daemon WSS runtime boundary changed; update hosted readiness");
assert.equal(
  daemonSource.includes("std::net::TcpStream::connect"),
  true,
  "daemon relay client no longer looks like a plain TCP runtime; update hosted readiness",
);
assert.equal(
  pwaSource.includes('PWA_TRANSPORT_MODE_LIVE_LOOPBACK = "live-loopback"'),
  true,
  "PWA product default guard changed; update hosted readiness",
);

const requiredRunbookPhrases = [
  "## Hosted Production Gate",
  "Daemon runtime WSS client support",
  "A production relay service artifact and deployment recipe",
  "Verifier-key distribution or public-key ticket signing",
  "Payload confidentiality or an explicit relay-operator trust decision",
  "Hosted observability and retention policy evidence",
  "Hosted failure-mode evidence",
];
for (const phrase of requiredRunbookPhrases) {
  assert.equal(runbook.includes(phrase), true, `runbook missing hosted readiness phrase: ${phrase}`);
}

const blockers = [
  "daemon-wss-runtime-support",
  "production-relay-service-artifact",
  "verifier-key-distribution-or-public-key-ticket-signing",
  "payload-confidentiality-or-explicit-trust-decision",
  "hosted-observability-and-retention-policy-evidence",
  "hosted-failure-mode-evidence",
];

const evidence = {
  status: "blocked",
  generatedAt: new Date().toISOString(),
  objective: "Track hosted/WSS relay production readiness without promoting relay to product default",
  decision,
  gates: {
    pwaHostedSetup: "ready",
    daemonWssRuntime: "blocked",
    productionRelayArtifact: "blocked",
    verifierKeyDistribution: "blocked",
    payloadConfidentiality: "blocked",
    hostedObservability: "blocked",
    hostedFailureModeEvidence: "blocked",
  },
  blockers,
  nextLocalSlice: "daemon-wss-relay-runtime-support",
  guardrails: [
    "product-default-remains-live-loopback",
    "relay-remains-explicit-setup-debug-path",
    "public-hosted-relay-requires-wss",
    "relay-operator-trust-or-payload-confidentiality-must-be-resolved-before-user-selectable-relay",
  ],
  result: {
    pwaHostedSetup,
    daemonWssFailClosed,
  },
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_HOSTED_READINESS_BLOCKED ${evidencePath}`);
