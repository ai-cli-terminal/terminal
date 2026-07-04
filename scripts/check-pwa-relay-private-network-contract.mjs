import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  relayDeploymentShapeDecision,
  relayPrivateNetworkSetupContract,
  relayPrivateNetworkSetupPreflight,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-private-network-contract");
const evidencePath =
  process.env.RA_PWA_RELAY_PRIVATE_NETWORK_CONTRACT_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-private-network-contract.json");

const identity = {
  deviceId: "web-private1",
  noisePubkeyHex: "b".repeat(64),
  approvalPubkeyHex: "c".repeat(64),
};
const ticket = createRelaySessionTicket({
  sessionId: "relay-private-network-contract",
  sessionToken: "token_relay_private_network_contract_123456",
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

const deploymentDecision = relayDeploymentShapeDecision();
assert.equal(deploymentDecision.productDefault, "live-loopback");
assert.ok(deploymentDecision.deferredModes.includes("private-network"));

const contract = relayPrivateNetworkSetupContract();
assert.equal(contract.deploymentMode, "private-network");
assert.equal(contract.productDefault, "live-loopback");
assert.equal(contract.selectedRuntime, "deferred");
assert.equal(contract.managedRelay, "deferred");
assert.ok(contract.requiredSetupFields.includes("privateNetworkName"));
assert.ok(contract.guardrails.includes("public_ws_blocked"));

const readyWss = relayPrivateNetworkSetupPreflight(
  {
    transportMode: "relay",
    deploymentMode: "private-network",
    relayEndpointUrl: "wss://relay.tailnet.example/relay",
    privateNetworkName: "tailnet-dev",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    operatorSetupText: "Private-network relay setup is ready.",
  },
  1500,
);
assert.equal(readyWss.status, "ready");
assert.equal(readyWss.contractReady, true);
assert.equal(readyWss.relayVisible, false);
assert.deepEqual(readyWss.blockers, []);

const readyLocalhost = relayPrivateNetworkSetupPreflight(
  {
    transportMode: "relay",
    deploymentMode: "private-network",
    relayEndpointUrl: "ws://127.0.0.1:8080/relay",
    privateNetworkName: "local-dev",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    operatorSetupText: "Private-network localhost setup is ready.",
  },
  1500,
);
assert.equal(readyLocalhost.status, "ready");

const publicWsBlocked = relayPrivateNetworkSetupPreflight(
  {
    transportMode: "relay",
    deploymentMode: "private-network",
    relayEndpointUrl: "ws://relay.example.test/relay",
    privateNetworkName: "tailnet-dev",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    operatorSetupText: "Private-network relay setup is ready.",
  },
  1500,
);
assert.ok(publicWsBlocked.blockers.includes("relay_endpoint_url_invalid"));

const missingInputs = relayPrivateNetworkSetupPreflight({ transportMode: "relay" }, 1500);
assert.ok(missingInputs.blockers.includes("private_network_deployment_mode_required"));
assert.ok(missingInputs.blockers.includes("private_network_name_invalid"));
assert.ok(missingInputs.blockers.includes("relay_endpoint_url_missing"));

const evidence = {
  status: "ok",
  generatedAt: new Date().toISOString(),
  objective: "Verify the private-network relay setup contract without changing live-loopback default",
  contract,
  result: {
    readyWss,
    readyLocalhost,
    publicWsBlocked,
    missingInputs,
  },
  nextLocalSlice: contract.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_PRIVATE_NETWORK_CONTRACT_OK ${evidencePath}`);
