import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const runbookPath = path.join(repoRoot, "docs", "relay-self-hosted-runbook.md");
const deployRecipePath = path.join(repoRoot, "docs", "relay-self-hosted-deploy.md");
const relayServicePath = path.join(repoRoot, "scripts", "relay-self-hosted-service.mjs");
const daemonPath = path.join(repoRoot, "src", "daemon.rs");
const pwaPath = path.join(repoRoot, "pwa", "app.mjs");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-deployment-runbook");
const evidencePath =
  process.env.RA_PWA_RELAY_DEPLOYMENT_RUNBOOK_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-deployment-runbook.json");

const runbook = await readFile(runbookPath, "utf8");
const deployRecipe = await readFile(deployRecipePath, "utf8");
const relayService = await readFile(relayServicePath, "utf8");
const daemonSource = await readFile(daemonPath, "utf8");
const pwaSource = await readFile(pwaPath, "utf8");

const requiredSections = [
  "## Current Readiness",
  "## Architecture",
  "## Endpoint Policy",
  "## Relay Service Contract",
  "## Secret And Key Handling",
  "## Relay Operator Trust Decision",
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
  "npm run relay:self-hosted",
  "npm run smoke:pwa-relay-service-artifact",
  "ai remote daemon --device-id <device-id> --transport relay --relay-endpoint-url",
  "npm run smoke:pwa-relay-approve-reject-evidence",
  "npm run smoke:pwa-relay-websocket-bridge",
  "npm run check:pwa-relay-transport-decision",
  "npm run check:pwa-relay-deployment-decision",
  "Daemon runtime WSS client support is available in `remote,tls` builds",
  "Ed25519 public-key ticket verification",
  "explicit self-hosted relay-operator trust decision",
  "aggregate-only observability",
  "Retention policy",
  "Relay itself is not production-ready",
];

for (const section of requiredSections) {
  assert.ok(runbook.includes(section), `runbook missing section ${section}`);
}

for (const phrase of requiredPhrases) {
  assert.ok(runbook.includes(phrase), `runbook missing phrase ${phrase}`);
}

assert.ok(
  daemonSource.includes("ParsedWsScheme::Wss") &&
    daemonSource.includes('url.strip_prefix("wss://")') &&
    daemonSource.includes('url.strip_prefix("ws://")') &&
    daemonSource.includes('matches!(host.as_str(), "localhost" | "127.0.0.1")') &&
    daemonSource.includes("relay_tls_stream"),
  "daemon WSS/localhost runtime boundary changed; update relay deployment runbook",
);
assert.ok(
  pwaSource.includes('PWA_TRANSPORT_MODE_LIVE_LOOPBACK = "live-loopback"'),
  "PWA product default guard changed; update relay deployment runbook",
);
assert.ok(
  deployRecipe.includes("AI_TERMINAL_RELAY_ED25519_PUBLIC_KEY_HEX") &&
    deployRecipe.includes("wss://relay.example.test/relay") &&
    deployRecipe.includes("npm run smoke:pwa-relay-service-artifact"),
  "relay deploy recipe missing required config or smoke guidance",
);
assert.ok(
  relayService.includes("createRelayService") &&
    relayService.includes('url.pathname === "/health"') &&
    relayService.includes('url.pathname === "/sessions"') &&
    relayService.includes('url.pathname !== "/relay"') &&
    relayService.includes("RELAY_TICKET_MAC_ALG_ED25519") &&
    relayService.includes("AI_TERMINAL_RELAY_ED25519_PUBLIC_KEY_HEX") &&
    relayService.includes("OBSERVABILITY_RETENTION_POLICY") &&
    relayService.includes("OBSERVABILITY_ERROR_CLASSES") &&
    relayService.includes("payloadJson") &&
    relayService.includes("verifierKeys"),
  "relay service artifact missing expected contract markers",
);

const evidence = {
  status: "ok",
  generatedAt: new Date().toISOString(),
  objective: "Verify the self-hosted relay deployment runbook covers required operator and production-readiness boundaries",
  runbookPath,
  checks: {
    requiredSections,
    requiredPhrases,
    daemonWssRuntimeReadyWithTls: true,
    relayServiceArtifactReady: true,
    productDefaultLiveLoopback: true,
  },
  readiness: {
    localStaging: "ready",
    hostedProduction: "blocked",
    verifierKeyDistribution: "ready-with-ed25519-public-verifier-keys",
    payloadConfidentiality: "ready-with-explicit-relay-operator-trust-decision",
    hostedObservability: "ready-with-aggregate-health-and-retention-policy-evidence",
    blockers: [
      "hosted-failure-mode-evidence",
    ],
  },
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_DEPLOYMENT_RUNBOOK_OK ${evidencePath}`);
