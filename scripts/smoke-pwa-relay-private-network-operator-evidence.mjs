import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createRelayEndpoint,
  livePingMessage,
  livePongMessage,
  relayDeploymentShapeDecision,
  relayEndpointExchange,
  relayPrivateNetworkRuntimeSetupPreflight,
  relayWebSocketConnectUrl,
  validateRelayPrivateNetworkRuntimeSetupMetadata,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-private-network-operator-evidence");
const evidencePath =
  process.env.RA_PWA_RELAY_PRIVATE_NETWORK_OPERATOR_EVIDENCE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-private-network-operator-evidence.json");
const transcriptPath = path.join(
  artifactRoot,
  "ra-pwa-relay-private-network-operator-transcript.txt",
);
const setupJsonPath = path.join(artifactRoot, "private-network-relay-setup.json");
const wslRunRoot = `/tmp/ra-pwa-relay-private-network-operator-${Date.now()}`;
const wslConfigHome = `${wslRunRoot}/config`;
const wslDataHome = `${wslRunRoot}/data`;
const wslCargoTargetDir = `${wslRunRoot}/target`;
const wslAiBin = `${wslCargoTargetDir}/debug/ai`;
const wslRepoRoot = toWslPath(repoRoot);
const transcript = [];

const identity = {
  deviceId: "web-private-operator",
  noisePubkeyHex: "b".repeat(64),
  approvalPubkeyHex: "c".repeat(64),
};
const privateNetworkName = "tailnet-dev";
const privateNetworkEndpoint = "ws://127.0.0.1:49152/relay";

function log(line) {
  const text = `[${new Date().toISOString()}] ${line}`;
  transcript.push(text);
  console.log(text);
}

function toWslPath(winPath) {
  const normalized = path.resolve(winPath).replaceAll("\\", "/");
  const match = /^([A-Za-z]):(\/.*)$/.exec(normalized);
  if (!match) {
    throw new Error(`cannot convert Windows path to WSL path: ${winPath}`);
  }
  return `/mnt/${match[1].toLowerCase()}${match[2]}`;
}

function bashQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function wslScript(command) {
  return [
    "set -euo pipefail",
    "source ~/.cargo/env",
    `export CARGO_TARGET_DIR=${bashQuote(wslCargoTargetDir)}`,
    `mkdir -p ${bashQuote(wslConfigHome)} ${bashQuote(wslDataHome)} ${bashQuote(wslCargoTargetDir)}`,
    `export XDG_CONFIG_HOME=${bashQuote(wslConfigHome)}`,
    `export XDG_DATA_HOME=${bashQuote(wslDataHome)}`,
    `cd ${bashQuote(wslRepoRoot)}`,
    command,
  ].join("; ");
}

function runWsl(command, label, timeoutMs = 120000) {
  return new Promise((resolve, reject) => {
    const child = spawn("wsl.exe", ["--", "bash", "-lc", wslScript(command)], {
      cwd: repoRoot,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      for (const line of chunk.split(/\r?\n/).filter(Boolean)) {
        log(`${label} stdout: ${line}`);
      }
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
      for (const line of chunk.split(/\r?\n/).filter(Boolean)) {
        log(`${label} stderr: ${line}`);
      }
    });
    child.on("error", (err) => {
      clearTimeout(timer);
      reject(err);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      const result = { code, stdout, stderr };
      if (code === 0) {
        resolve(result);
      } else {
        const err = new Error(`${label} exited ${code}`);
        err.result = result;
        reject(err);
      }
    });
  });
}

async function runWslFailure(command, label, timeoutMs = 120000) {
  try {
    const result = await runWsl(command, label, timeoutMs);
    throw new Error(`${label} unexpectedly succeeded: ${result.stdout}`);
  } catch (err) {
    if (err.result && err.result.code !== 0) {
      return err.result;
    }
    throw err;
  }
}

function parseLine(output, key) {
  const re = new RegExp(`^${key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*:\\s*(.+)$`, "m");
  const match = re.exec(output);
  if (!match) {
    throw new Error(`missing output key ${key}`);
  }
  return match[1].trim();
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  log(`repoRoot=${repoRoot}`);
  log(`wslRunRoot=${wslRunRoot}`);
  log("building remote ai binary");
  await runWsl("cargo build --quiet --features remote --bin ai", "cargo-build", 180000);

  const pairStart = await runWsl(
    `${bashQuote(wslAiBin)} remote pair --ttl-seconds 600 --pwa-url http://127.0.0.1/private-network-operator`,
    "pair-start",
  );
  const pairingCode = parseLine(pairStart.stdout, "code");

  await runWsl(
    [
      `${bashQuote(wslAiBin)} remote pair`,
      `--device-id ${bashQuote(identity.deviceId)}`,
      `--code ${bashQuote(pairingCode)}`,
      `--noise-pubkey-hex ${bashQuote(identity.noisePubkeyHex)}`,
      `--approval-pubkey-hex ${bashQuote(identity.approvalPubkeyHex)}`,
    ].join(" "),
    "pair-complete",
  );

  const setupResult = await runWsl(
    [
      `${bashQuote(wslAiBin)} remote relay-setup`,
      `--relay-endpoint-url ${bashQuote(privateNetworkEndpoint)}`,
      "--relay-deployment-mode private-network",
      `--private-network-name ${bashQuote(privateNetworkName)}`,
      `--device-id ${bashQuote(identity.deviceId)}`,
      "--ttl-seconds 300",
    ].join(" "),
    "private-network-relay-setup",
  );
  const relaySetupJson = parseLine(setupResult.stdout, "relay_setup_json");
  const relaySetup = JSON.parse(relaySetupJson);
  await writeFile(setupJsonPath, `${JSON.stringify(relaySetup, null, 2)}\n`, "utf8");

  assert.equal(relaySetup.transportMode, "relay");
  assert.equal(relaySetup.deploymentMode, "private-network");
  assert.equal(relaySetup.privateNetworkName, privateNetworkName);
  assert.equal(relaySetup.relayEndpointUrl, privateNetworkEndpoint);
  assert.equal(relaySetup.companionIdentity.deviceId, identity.deviceId);
  assert.equal(relaySetup.companionIdentity.noisePubkeyHex, identity.noisePubkeyHex);
  assert.equal(relaySetup.companionIdentity.approvalPubkeyHex, identity.approvalPubkeyHex);
  validateRelayPrivateNetworkRuntimeSetupMetadata(relaySetup);

  const nowMs = relaySetup.signedSessionTicket.ticket.issued_at_ms + 1;
  const preflight = relayPrivateNetworkRuntimeSetupPreflight(relaySetup, nowMs);
  assert.equal(preflight.status, "ready");
  assert.equal(preflight.contractReady, true);
  assert.equal(preflight.relayVisible, false);
  assert.deepEqual(preflight.blockers, []);

  const daemonConnectUrl = relayWebSocketConnectUrl(relaySetup.relayEndpointUrl, relaySetup.daemonConnect);
  const companionConnectUrl = relayWebSocketConnectUrl(
    relaySetup.relayEndpointUrl,
    relaySetup.companionConnect,
  );
  const daemonEndpoint = createRelayEndpoint(relaySetup.daemonConnect.session_id, "daemon");
  const companionEndpoint = createRelayEndpoint(relaySetup.companionConnect.session_id, "companion");
  const exchange = relayEndpointExchange(
    daemonEndpoint,
    companionEndpoint,
    livePingMessage("private-network-operator-ping"),
    livePongMessage("private-network-operator-pong"),
    nowMs + 10,
  );
  assert.deepEqual(exchange.companionMessage, livePingMessage("private-network-operator-ping"));
  assert.deepEqual(exchange.daemonReply, livePongMessage("private-network-operator-pong"));

  const publicWsFailure = await runWslFailure(
    [
      `${bashQuote(wslAiBin)} remote relay-setup`,
      "--relay-endpoint-url ws://relay.example.test/relay",
      "--relay-deployment-mode private-network",
      `--private-network-name ${bashQuote(privateNetworkName)}`,
      `--device-id ${bashQuote(identity.deviceId)}`,
      "--ttl-seconds 300",
    ].join(" "),
    "private-network-public-ws-blocked",
  );
  assert.match(
    `${publicWsFailure.stdout}\n${publicWsFailure.stderr}`,
    /wss:\/\/ or localhost ws:\/\//,
  );

  const decision = relayDeploymentShapeDecision();
  assert.equal(decision.productDefault, "live-loopback");
  assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);

  const evidence = {
    status: "passed",
    timestamp: new Date().toISOString(),
    repoRoot,
    evidencePath,
    transcriptPath,
    setupJsonPath,
    wslRunRoot,
    productDefault: decision.productDefault,
    managedRelay: "deferred",
    operatorSetup: {
      command: "ai remote relay-setup --relay-deployment-mode private-network",
      deploymentMode: relaySetup.deploymentMode,
      privateNetworkName: relaySetup.privateNetworkName,
      relayEndpointUrl: relaySetup.relayEndpointUrl,
      deviceId: relaySetup.companionIdentity.deviceId,
      operatorSetupText: relaySetup.operatorSetupText,
      ticketKeyId: relaySetup.signedSessionTicket.key_id || null,
      expiresAtMs: relaySetup.signedSessionTicket.ticket.expires_at_ms,
    },
    pwaImportPreflight: preflight,
    relayBridgeRoundtrip: {
      daemonConnectUrl,
      companionConnectUrl,
      daemonFrameRoute: exchange.daemonFrame ? {
        sessionId: exchange.daemonFrame.session_id,
        sender: exchange.daemonFrame.sender,
        sequence: exchange.daemonFrame.sequence,
      } : null,
      companionFrameRoute: exchange.companionFrame ? {
        sessionId: exchange.companionFrame.session_id,
        sender: exchange.companionFrame.sender,
        sequence: exchange.companionFrame.sequence,
      } : null,
      daemonReceived: exchange.daemonReply,
      companionReceived: exchange.companionMessage,
    },
    publicWsBlocked: {
      exitCode: publicWsFailure.code,
      stderrTail: publicWsFailure.stderr.split(/\r?\n/).filter(Boolean).slice(-10),
    },
    nextLocalSlice: "managed-relay-revocation-and-rotation-propagation-smoke",
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
  await writeFile(transcriptPath, `${transcript.join("\n")}\n`, "utf8");
  console.log(`RA_PWA_RELAY_PRIVATE_NETWORK_OPERATOR_EVIDENCE_OK ${evidencePath}`);
}

main().catch(async (err) => {
  const evidence = {
    status: "failed",
    timestamp: new Date().toISOString(),
    repoRoot,
    evidencePath,
    transcriptPath,
    error: err?.stack || String(err),
  };
  await mkdir(artifactRoot, { recursive: true }).catch(() => {});
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8").catch(() => {});
  await writeFile(transcriptPath, `${transcript.join("\n")}\n`, "utf8").catch(() => {});
  console.error(`RA_PWA_RELAY_PRIVATE_NETWORK_OPERATOR_EVIDENCE_FAILED ${evidencePath}`);
  console.error(err);
  process.exitCode = 1;
});
