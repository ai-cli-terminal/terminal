import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  relayTransportUxPreflight,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-ux-preflight");
const evidencePath =
  process.env.RA_PWA_RELAY_UX_PREFLIGHT_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-ux-preflight.json");

const identity = {
  deviceId: "web-1234abcd",
  noisePubkeyHex: "b".repeat(64),
  approvalPubkeyHex: "c".repeat(64),
};
const ticket = createRelaySessionTicket({
  sessionId: "relay-ux-preflight-session",
  sessionToken: "token_relay_ux_preflight_1234567890abcdef",
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

const defaultHidden = relayTransportUxPreflight({}, 1500);
assert.equal(defaultHidden.status, "hidden");
assert.equal(defaultHidden.relayVisible, false);
assert.ok(defaultHidden.blockers.includes("transport_mode_not_relay"));
assert.ok(defaultHidden.blockers.includes("relay_endpoint_url_missing"));
assert.ok(defaultHidden.blockers.includes("relay_signed_ticket_missing"));

const missingRelayInputs = relayTransportUxPreflight({ transportMode: "relay" }, 1500);
assert.equal(missingRelayInputs.status, "hidden");
assert.ok(missingRelayInputs.blockers.includes("relay_endpoint_url_missing"));
assert.ok(missingRelayInputs.blockers.includes("relay_deployment_mode_missing"));
assert.ok(missingRelayInputs.blockers.includes("relay_operator_setup_text_missing"));
assert.ok(missingRelayInputs.blockers.includes("companion_identity_missing"));
assert.ok(missingRelayInputs.blockers.includes("relay_signed_ticket_missing"));

const readyRelay = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    deploymentMode: "managed",
    operatorSetupText: "Managed relay setup is ready.",
  },
  1500,
);
assert.equal(readyRelay.status, "ready");
assert.equal(readyRelay.relayVisible, true);
assert.equal(readyRelay.relayEnabled, true);
assert.deepEqual(readyRelay.blockers, []);

const expiredRelay = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedTicket,
    companionIdentity: identity,
    deploymentMode: "managed",
    operatorSetupText: "Managed relay setup is ready.",
  },
  2000,
);
assert.equal(expiredRelay.status, "hidden");
assert.ok(expiredRelay.blockers.includes("relay_signed_ticket_expired"));

const mismatchRelay = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "https://relay.example.test/session",
    signedSessionTicket: signedTicket,
    companionIdentity: { ...identity, approvalPubkeyHex: "d".repeat(64) },
    deploymentMode: "experimental",
    operatorSetupText: "short",
  },
  1500,
);
assert.equal(mismatchRelay.status, "hidden");
assert.ok(mismatchRelay.blockers.includes("relay_endpoint_url_invalid"));
assert.ok(mismatchRelay.blockers.includes("relay_deployment_mode_invalid"));
assert.ok(mismatchRelay.blockers.includes("relay_operator_setup_text_missing"));
assert.ok(mismatchRelay.blockers.includes("relay_ticket_identity_mismatch"));

await mkdir(artifactRoot, { recursive: true });
const evidence = {
  status: "ok",
  generatedAt: new Date().toISOString(),
  objective: "Verify PWA relay transport UX remains hidden until all relay readiness inputs exist",
  result: {
    defaultHidden,
    missingRelayInputs,
    readyRelay,
    expiredRelay,
    mismatchRelay,
  },
};
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_UX_PREFLIGHT_OK ${evidencePath}`);
