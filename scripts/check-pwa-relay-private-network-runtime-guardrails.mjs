import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  relayPrivateNetworkRuntimeSetupPreflight,
  relaySessionConnect,
  validateRelayPrivateNetworkRuntimeSetupMetadata,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-private-network-runtime-guardrails",
);
const evidencePath =
  process.env.RA_PWA_RELAY_PRIVATE_NETWORK_RUNTIME_GUARDRAILS_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-private-network-runtime-guardrails.json");

const identity = {
  deviceId: "web-private-runtime",
  noisePubkeyHex: "b".repeat(64),
  approvalPubkeyHex: "c".repeat(64),
};
const ticket = createRelaySessionTicket({
  sessionId: "relay-private-network-runtime",
  sessionToken: "token_relay_private_network_runtime_123456",
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

const privateNetworkSetup = {
  relayProtocolVersion: 1,
  transportMode: "relay",
  deploymentMode: "private-network",
  privateNetworkName: "tailnet-dev",
  relayEndpointUrl: "wss://relay.tailnet.example/relay",
  signedSessionTicket: signedTicket,
  daemonConnect: relaySessionConnect(ticket, "daemon"),
  companionConnect: relaySessionConnect(ticket, "companion"),
  companionIdentity: identity,
  operatorSetupText: "Private-network relay tailnet-dev endpoint is ready.",
};

assert.doesNotThrow(() => validateRelayPrivateNetworkRuntimeSetupMetadata(privateNetworkSetup));
const ready = relayPrivateNetworkRuntimeSetupPreflight(privateNetworkSetup, 1500);
assert.equal(ready.status, "ready");
assert.equal(ready.contractReady, true);
assert.equal(ready.relayVisible, false);
assert.deepEqual(ready.blockers, []);

assert.throws(
  () =>
    relayPrivateNetworkRuntimeSetupPreflight({
      ...privateNetworkSetup,
      relayEndpointUrl: "ws://relay.example.test/relay",
    }),
  /endpoint URL/,
);
assert.throws(
  () =>
    validateRelayPrivateNetworkRuntimeSetupMetadata({
      ...privateNetworkSetup,
      privateNetworkName: "bad name",
    }),
  /privateNetworkName/,
);

const mainSource = await readFile(path.join(repoRoot, "src", "main.rs"), "utf8");
const transportSource = await readFile(path.join(repoRoot, "src", "remote_transport.rs"), "utf8");

for (const marker of [
  "relay_deployment_mode",
  "private_network_name",
  "resolve_relay_deployment_selection",
  "valid_relay_websocket_endpoint_url",
]) {
  assert.ok(mainSource.includes(marker), `missing main.rs marker ${marker}`);
}
for (const marker of [
  "COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK",
  "valid_relay_private_network_name",
  "private_network_name",
  "relay_private_network_runtime_setup_emits_guarded_contract",
]) {
  assert.ok(transportSource.includes(marker), `missing remote_transport.rs marker ${marker}`);
}

const evidence = {
  status: "ok",
  generatedAt: new Date().toISOString(),
  objective: "Verify private-network relay runtime guardrails and PWA setup preflight boundary",
  runtimeSetup: {
    deploymentMode: privateNetworkSetup.deploymentMode,
    privateNetworkName: privateNetworkSetup.privateNetworkName,
    relayEndpointUrl: privateNetworkSetup.relayEndpointUrl,
    operatorSetupText: privateNetworkSetup.operatorSetupText,
    signedTicketKeyId: privateNetworkSetup.signedSessionTicket.key_id || null,
  },
  result: {
    ready,
    publicWsBlocked: true,
    badPrivateNetworkNameBlocked: true,
    daemonModeParsingMarkers: true,
    setupJsonEmissionBoundaryMarkers: true,
  },
  rustVerificationSelectors: [
    "cargo test --features remote relay_private_network_runtime_setup_emits_guarded_contract",
    "cargo test --features remote daemon_transport_selection_validates_relay_inputs",
    "cargo test --features remote cli_parses_remote_daemon_relay_transport",
    "cargo test --features remote cli_parses_remote_relay_setup",
  ],
  nextLocalSlice: "managed-relay-active-session-and-byte-quota-smoke",
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_PRIVATE_NETWORK_RUNTIME_GUARDRAILS_OK ${evidencePath}`);
