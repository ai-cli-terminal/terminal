import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-transport-decision");
const decisionEvidencePath =
  process.env.RA_PWA_RELAY_TRANSPORT_DECISION_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-transport-decision.json");
const httpEvidencePath = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-http-bridge",
  "ra-pwa-relay-http-bridge.json",
);
const websocketEvidencePath = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-websocket-bridge",
  "ra-pwa-relay-websocket-bridge.json",
);

const decision = {
  selectedPrototypeSubstrate: "websocket",
  fallbackCandidate: "http-polling",
  productDefault: "live-loopback",
  transportModeReadiness: "planned",
};

async function runNodeScript(scriptName) {
  await new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [path.join(scriptDir, scriptName)], {
      cwd: repoRoot,
      stdio: "inherit",
      windowsHide: true,
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${scriptName} exited with ${code}`));
    });
  });
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

function assertNoPayloadLeak(route, label) {
  assert.equal(typeof route, "object", `${label} route missing`);
  assert.equal("payload_json" in route, false, `${label} leaked payload_json`);
  assert.equal(route.relay_protocol_version, 1, `${label} protocol version mismatch`);
  assert.equal(typeof route.session_id, "string", `${label} session id missing`);
  assert.equal(typeof route.payload_json_bytes, "number", `${label} payload byte length missing`);
  assert.ok(route.payload_json_bytes > 0, `${label} payload byte length empty`);
}

function assertCommonBridgeEvidence(evidence, label, expectedCounts) {
  assert.equal(evidence.status, "ok", `${label} evidence status mismatch`);
  assert.equal(evidence.result?.status, "ok", `${label} result status mismatch`);
  assert.equal(evidence.result?.companionMessageType, "approval_request", `${label} request mismatch`);
  assert.equal(evidence.result?.daemonReplyType, "approval_response", `${label} reply mismatch`);
  assert.equal(evidence.result?.duplicateRejected, true, `${label} duplicate sequence not rejected`);
  assert.equal(evidence.result?.expiredDropped, true, `${label} expired frame not dropped`);
  assert.equal(evidence.result?.finalHealth?.queuedFrames, 0, `${label} queued frames remain`);
  assert.equal(
    evidence.result?.finalHealth?.stats?.acceptedFrames,
    expectedCounts.acceptedFrames,
    `${label} accepted count mismatch`,
  );
  assert.equal(
    evidence.result?.finalHealth?.stats?.deliveredFrames,
    expectedCounts.deliveredFrames,
    `${label} delivered count mismatch`,
  );
  assert.equal(
    evidence.result?.finalHealth?.stats?.expiredFrames,
    expectedCounts.expiredFrames,
    `${label} expired count mismatch`,
  );
  assert.equal(
    evidence.result?.finalHealth?.stats?.rejectedFrames,
    expectedCounts.rejectedFrames,
    `${label} rejected count mismatch`,
  );
  assertNoPayloadLeak(evidence.result?.daemonRoute, `${label} daemon`);
  assertNoPayloadLeak(evidence.result?.companionRoute, `${label} companion`);
}

function assertWebSocketBridgeEvidence(evidence) {
  assertCommonBridgeEvidence(evidence, "websocket", {
    acceptedFrames: 5,
    deliveredFrames: 4,
    expiredFrames: 1,
    rejectedFrames: 1,
  });
  assert.equal(evidence.websocketUrl?.startsWith("ws://127.0.0.1:"), true, "websocket URL mismatch");
  assert.equal(evidence.sessionUrl?.startsWith("http://127.0.0.1:"), true, "websocket session URL mismatch");
  assert.equal(evidence.result.daemonConnected, true, "websocket daemon connect not authenticated");
  assert.equal(evidence.result.companionConnected, true, "websocket companion connect not authenticated");
  assert.equal(
    evidence.result.expiredTicketConnectRejected,
    true,
    "websocket expired ticket connect not rejected",
  );
  assert.equal(evidence.result.rotationReconnected, true, "websocket rotation reconnect failed");
  assert.equal(
    evidence.result.rotationReconnectDelivered,
    true,
    "websocket rotation reconnect delivery failed",
  );
  assert.equal(evidence.result.rotationOldTokenRejected, true, "websocket old token was not rejected");
  assert.equal(
    evidence.result.rotationOldFrameIsolated,
    true,
    "websocket old session frame was not isolated",
  );
  assert.equal(evidence.result.unsignedTicketRejected, true, "websocket unsigned ticket not rejected");
  assert.equal(evidence.result.badMacTicketRejected, true, "websocket bad mac ticket not rejected");
  assert.equal(
    evidence.result.unauthenticatedFrameRejected,
    true,
    "websocket unauthenticated frame not rejected",
  );
  assert.equal(evidence.result.badTokenRejected, true, "websocket bad token not rejected");
  assert.equal(evidence.result.finalHealth.sessions, 0, "websocket sessions remain");
  assert.equal(evidence.result.finalHealth.stats.acceptedConnects, 8, "websocket accepted connect count mismatch");
  assert.equal(evidence.result.finalHealth.stats.rejectedConnects, 4, "websocket rejected connect count mismatch");
  assert.equal(evidence.result.finalHealth.stats.registeredTickets, 7, "websocket ticket count mismatch");
  assert.equal(evidence.result.finalHealth.stats.rejectedTickets, 2, "websocket ticket reject count mismatch");
  assert.equal(evidence.result.finalHealth.stats.openedConnections, 12, "websocket open count mismatch");
  assert.equal(evidence.result.finalHealth.stats.closedConnections, 12, "websocket close count mismatch");
  assertNoPayloadLeak(evidence.result.rotationOldRoute, "websocket rotation old");
  assertNoPayloadLeak(evidence.result.rotationNewRoute, "websocket rotation new");
}

function assertHttpBridgeEvidence(evidence) {
  assertCommonBridgeEvidence(evidence, "http", {
    acceptedFrames: 3,
    deliveredFrames: 2,
    expiredFrames: 1,
    rejectedFrames: 1,
  });
  assert.equal(evidence.bridgeUrl?.startsWith("http://127.0.0.1:"), true, "http bridge URL mismatch");
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  if (process.env.RA_PWA_RELAY_TRANSPORT_DECISION_SKIP_REFRESH !== "1") {
    await runNodeScript("smoke-pwa-relay-http-bridge.mjs");
    await runNodeScript("smoke-pwa-relay-websocket-bridge.mjs");
  }

  const httpEvidence = await readJson(httpEvidencePath);
  const websocketEvidence = await readJson(websocketEvidencePath);
  assertHttpBridgeEvidence(httpEvidence);
  assertWebSocketBridgeEvidence(websocketEvidence);

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Select the first RA/PWA relay prototype substrate from local bridge evidence",
    decision,
    requiredEvidence: {
      httpBridge: {
        path: httpEvidencePath,
        status: httpEvidence.status,
        objective: httpEvidence.objective,
      },
      websocketBridge: {
        path: websocketEvidencePath,
        status: websocketEvidence.status,
        objective: websocketEvidence.objective,
      },
    },
    rationale: [
      "WebSocket preserves the relay frame invariants already proven by the HTTP bridge smoke.",
      "The local WebSocket smoke now requires a signed ticket plus connect handshake before routing frames.",
      "The WebSocket smoke also proves expired-ticket connect rejection, reconnect with a rotated session token, and stale-session isolation.",
      "WebSocket is browser-native full-duplex, so daemon and companion peers can receive pending frames without polling loops.",
      "HTTP polling remains useful as a simpler fallback or diagnostics harness, but it is not the first prototype substrate.",
      "The product default remains live-loopback until hosted relay deployment, UX, and daemon integration evidence exist.",
    ],
    nonGoals: [
      "No product transport switch.",
      "No hosted relay deployment.",
      "No hosted relay runtime or persistent secret store.",
      "No operator-visible relay UX.",
    ],
  };
  await writeFile(decisionEvidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_TRANSPORT_DECISION_OK ${decisionEvidencePath}`);
}

await main();
