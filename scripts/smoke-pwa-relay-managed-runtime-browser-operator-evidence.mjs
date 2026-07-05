import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import {
  relayManagedRuntimeBrowserOperatorEvidence,
  relayManagedRuntimePwaExposureGate,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-browser-operator-evidence",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_BROWSER_OPERATOR_EVIDENCE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-browser-operator-evidence.json",
  );
const desktopScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-browser-operator-evidence.png",
);
const mobileScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-browser-operator-evidence-mobile.png",
);

let staticServer = null;
let browser = null;

function browserExecutablePath() {
  const candidates = [
    process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE,
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
    "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
    path.join(os.homedir(), "AppData", "Local", "Google", "Chrome", "Application", "chrome.exe"),
    path.join(os.homedir(), "AppData", "Local", "Microsoft", "Edge", "Application", "msedge.exe"),
  ].filter(Boolean);
  for (const candidate of candidates) {
    if (os.platform() === "win32" && existsSync(candidate)) {
      return candidate;
    }
  }
  return undefined;
}

async function startStaticServer() {
  const contentTypes = new Map([
    [".html", "text/html; charset=utf-8"],
    [".mjs", "text/javascript; charset=utf-8"],
    [".js", "text/javascript; charset=utf-8"],
    [".css", "text/css; charset=utf-8"],
    [".svg", "image/svg+xml"],
    [".webmanifest", "application/manifest+json; charset=utf-8"],
  ]);
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url || "/", "http://127.0.0.1/");
      const relative = url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname.slice(1));
      const filePath = path.resolve(pwaDir, relative);
      if (!filePath.startsWith(pwaDir + path.sep)) {
        res.writeHead(403);
        res.end("forbidden");
        return;
      }
      const body = await readFile(filePath);
      res.writeHead(200, {
        "content-type": contentTypes.get(path.extname(filePath)) || "application/octet-stream",
      });
      res.end(body);
    } catch {
      res.writeHead(404);
      res.end("not found");
    }
  });
  await new Promise((resolve, reject) => {
    server.on("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  return { server, url: `http://127.0.0.1:${address.port}` };
}

function closeServer(server) {
  return new Promise((resolve) => {
    if (!server) {
      resolve();
      return;
    }
    server.close(() => resolve());
  });
}

async function loadManagedRelayPanel(page, url) {
  await page.goto(`${url}/index.html`, { waitUntil: "networkidle" });
  await page.click('[data-mode="relay"]');
  await page.waitForFunction(
    () => document.querySelector("#relay-managed-state")?.textContent === "Explicit opt-in ready",
    null,
    { timeout: 15000 },
  );
}

async function managedRelayVisibleState(page) {
  return page.evaluate(() => ({
    title: document.querySelector("#relay-managed-title")?.textContent || "",
    state: document.querySelector("#relay-managed-state")?.textContent || "",
    productDefault: document.querySelector("#relay-managed-default-mode")?.textContent || "",
    exposure: document.querySelector("#relay-managed-exposure")?.textContent || "",
    endpointMode: document.querySelector("#relay-managed-endpoint-mode")?.textContent || "",
    publicBind: document.querySelector("#relay-managed-public-bind")?.textContent || "",
    autoStart: document.querySelector("#relay-managed-auto-start")?.textContent || "",
    rollback: document.querySelector("#relay-managed-rollback")?.textContent || "",
    next: document.querySelector("#relay-managed-next")?.textContent || "",
    evidence: Array.from(
      document.querySelectorAll("#relay-managed-evidence-list li"),
      (item) => item.textContent || "",
    ),
    copy: document.querySelector("#relay-managed-copy")?.textContent || "",
    bodyText: document.body.textContent || "",
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
}

function assertManagedRelayVisibleState(state, summary) {
  const expected = summary.browserEvidence.expectedVisibleText;
  assert.equal(state.title, expected.title);
  assert.equal(state.state, expected.state);
  assert.equal(state.productDefault, expected.productDefault);
  assert.equal(state.exposure, expected.exposure);
  assert.equal(state.endpointMode, expected.endpointMode);
  assert.equal(state.publicBind, expected.publicBind);
  assert.equal(state.autoStart, expected.autoStart);
  assert.equal(state.rollback, expected.rollback);
  assert.equal(state.next, expected.next);
  assert.deepEqual(state.evidence, ["Managed relay explicit opt-in exposure ready"]);
  for (const requiredCopy of summary.operatorEvidence.operatorCopyIncludes) {
    assert.ok(state.copy.includes(requiredCopy), `managed relay copy missing: ${requiredCopy}`);
  }
  for (const prohibited of summary.prohibitedVisibleTokens) {
    assert.equal(
      state.bodyText.includes(prohibited),
      false,
      `managed relay visible body leaked ${prohibited}`,
    );
  }
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const exposureGate = relayManagedRuntimePwaExposureGate();
  const summary = relayManagedRuntimeBrowserOperatorEvidence();
  assert.equal(exposureGate.readiness, "exposure-gate");
  assert.equal(summary.readiness, "browser-operator-evidence");
  assert.equal(summary.productDefault, "live-loopback");
  assert.equal(summary.pwaExposure, "explicit-opt-in");
  assert.equal(summary.endpointAutoStart, false);
  assert.equal(summary.publicBind, false);

  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });

  const desktopViewport = summary.browserEvidence.requiredViewports.find(
    ({ name }) => name === "desktop",
  );
  const page = await browser.newPage({
    viewport: { width: desktopViewport.width, height: desktopViewport.height },
  });
  await loadManagedRelayPanel(page, staticInfo.url);
  await page.screenshot({ path: desktopScreenshotPath, fullPage: true });
  const desktop = await managedRelayVisibleState(page);
  assertManagedRelayVisibleState(desktop, summary);

  const mobileViewport = summary.browserEvidence.requiredViewports.find(
    ({ name }) => name === "mobile",
  );
  const mobilePage = await browser.newPage({
    viewport: { width: mobileViewport.width, height: mobileViewport.height },
  });
  await loadManagedRelayPanel(mobilePage, staticInfo.url);
  await mobilePage.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobile = await managedRelayVisibleState(mobilePage);
  assertManagedRelayVisibleState(mobile, summary);
  assert.ok(
    mobile.scrollWidth <= mobile.innerWidth,
    `mobile layout overflowed: ${mobile.scrollWidth} > ${mobile.innerWidth}`,
  );

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Capture managed relay PWA browser/operator evidence for explicit opt-in setup copy",
    pwaUrl: staticInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      desktop: desktopScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    exposureGate: {
      readiness: exposureGate.readiness,
      pwaExposure: exposureGate.pwaExposure,
      endpointMode: exposureGate.endpointMode,
      endpointAutoStart: exposureGate.endpointAutoStart,
      publicBind: exposureGate.publicBind,
      nextLocalSlice: exposureGate.nextLocalSlice,
    },
    summary: {
      readiness: summary.readiness,
      implementationStatus: summary.implementationStatus,
      selectedRuntime: summary.selectedRuntime,
      pwaExposureDecision: summary.pwaExposureDecision,
      endpointAutoStart: summary.endpointAutoStart,
      publicBind: summary.publicBind,
      nextLocalSlice: summary.nextLocalSlice,
    },
    desktop: {
      ...desktop,
      bodyText: undefined,
    },
    mobile: {
      ...mobile,
      bodyText: undefined,
    },
    evidenceChecks: summary.evidenceChecks,
    guardrails: summary.guardrails,
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_BROWSER_OPERATOR_EVIDENCE_OK ${evidencePath}`);
}

try {
  await main();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(staticServer);
}
