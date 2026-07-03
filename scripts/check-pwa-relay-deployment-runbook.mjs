import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const runbookPath = path.join(repoRoot, "docs", "relay-self-hosted-runbook.md");
const daemonPath = path.join(repoRoot, "src", "daemon.rs");
const pwaPath = path.join(repoRoot, "pwa", "app.mjs");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-deployment-runbook");
const evidencePath =
  process.env.RA_PWA_RELAY_DEPLOYMENT_RUNBOOK_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-deployment-runbook.json");

const runbook = await readFile(runbookPath, "utf8");
const daemonSource = await readFile(daemonPath, "utf8");
const pwaSource = await readFile(pwaPath, "utf8");

const requiredSections = [
  "## Current Readiness",
  "## Architecture",
  "## Endpoint Policy",
  "## Relay Service Contract",
  "## Secret And Key Handling",
  "## Local Staging Procedure",
  "## Manual Staging Procedure",
  "## Hosted Production Gate",
  "## Observability",
  "## Failure-Mode Evidence",
  "## Rollback",
  "## Completion Criteria",
];

const requiredPhrases = [
  "Product default remains `live-loopback`",
  "wss://",
  "ws://127.0.0.1",
  "GET /health",
  "POST /sessions",
  "GET /relay?session_id=<id>&role=<daemon|companion>",
  "ai remote daemon --device-id <device-id> --transport relay --relay-endpoint-url",
  "npm run smoke:pwa-relay-approve-reject-evidence",
  "npm run smoke:pwa-relay-websocket-bridge",
  "npm run check:pwa-relay-transport-decision",
  "npm run check:pwa-relay-deployment-decision",
  "Daemon runtime WSS client support",
  "Verifier-key distribution or public-key ticket signing",
  "Payload confidentiality",
  "Relay itself is not production-ready",
];

for (const section of requiredSections) {
  assert.ok(runbook.includes(section), `runbook missing section ${section}`);
}

for (const phrase of requiredPhrases) {
  assert.ok(runbook.includes(phrase), `runbook missing phrase ${phrase}`);
}

assert.ok(
  daemonSource.includes('url.starts_with("wss://")') &&
    daemonSource.includes('url.strip_prefix("ws://")') &&
    daemonSource.includes('matches!(host.as_str(), "localhost" | "127.0.0.1")'),
  "daemon WSS/localhost runtime boundary changed; update relay deployment runbook",
);
assert.ok(
  pwaSource.includes('PWA_TRANSPORT_MODE_LIVE_LOOPBACK = "live-loopback"'),
  "PWA product default guard changed; update relay deployment runbook",
);

const evidence = {
  status: "ok",
  generatedAt: new Date().toISOString(),
  objective: "Verify the self-hosted relay deployment runbook covers required operator and production-readiness boundaries",
  runbookPath,
  checks: {
    requiredSections,
    requiredPhrases,
    daemonWssRuntimeBlocked: true,
    productDefaultLiveLoopback: true,
  },
  readiness: {
    localStaging: "ready",
    hostedProduction: "blocked",
    blockers: [
      "daemon-wss-runtime-support",
      "production-relay-service-artifact",
      "verifier-key-distribution-or-public-key-ticket-signing",
      "payload-confidentiality-or-explicit-trust-decision",
      "hosted-observability-and-failure-mode-evidence",
    ],
  },
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_DEPLOYMENT_RUNBOOK_OK ${evidencePath}`);
