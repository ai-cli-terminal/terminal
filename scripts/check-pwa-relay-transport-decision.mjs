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

function assertCommonBridgeEvidence(evidence, label) {
  assert.equal(evidence.status, "ok", `${label} evidence status mismatch`);
  assert.equal(evidence.result?.status, "ok", `${label} result status mismatch`);
  assert.equal(evidence.result?.companionMessageType, "approval_request", `${label} request mismatch`);
  assert.equal(evidence.result?.daemonReplyType, "approval_response", `${label} reply mismatch`);
  assert.equal(evidence.result?.duplicateRejected, true, `${label} duplicate sequence not rejected`);
  assert.equal(evidence.result?.expiredDropped, true, `${label} expired frame not dropped`);
  assert.equal(evidence.result?.finalHealth?.queuedFrames, 0, `${label} queued frames remain`);
  assert.equal(evidence.result?.finalHealth?.stats?.acceptedFrames, 3, `${label} accepted count mismatch`);
  assert.equal(evidence.result?.finalHealth?.stats?.deliveredFrames, 2, `${label} delivered count mismatch`);
  assert.equal(evidence.result?.finalHealth?.stats?.expiredFrames, 1, `${label} expired count mismatch`);
  assert.equal(evidence.result?.finalHealth?.stats?.rejectedFrames, 1, `${label} rejected count mismatch`);
  assertNoPayloadLeak(evidence.result?.daemonRoute, `${label} daemon`);
  assertNoPayloadLeak(evidence.result?.companionRoute, `${label} companion`);
}

function assertWebSocketBridgeEvidence(evidence) {
  assertCommonBridgeEvidence(evidence, "websocket");
  assert.equal(evidence.websocketUrl?.startsWith("ws://127.0.0.1:"), true, "websocket URL mismatch");
  assert.equal(evidence.result.finalHealth.sessions, 0, "websocket sessions remain");
  assert.equal(evidence.result.finalHealth.stats.openedConnections, 4, "websocket open count mismatch");
  assert.equal(evidence.result.finalHealth.stats.closedConnections, 4, "websocket close count mismatch");
}

function assertHttpBridgeEvidence(evidence) {
  assertCommonBridgeEvidence(evidence, "http");
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
      "WebSocket is browser-native full-duplex, so daemon and companion peers can receive pending frames without polling loops.",
      "HTTP polling remains useful as a simpler fallback or diagnostics harness, but it is not the first prototype substrate.",
      "The product default remains live-loopback until relay auth, deployment, UX, and daemon integration evidence exist.",
    ],
    nonGoals: [
      "No product transport switch.",
      "No hosted relay deployment.",
      "No relay auth token design.",
      "No operator-visible relay UX.",
    ],
  };
  await writeFile(decisionEvidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_TRANSPORT_DECISION_OK ${decisionEvidencePath}`);
}

await main();
