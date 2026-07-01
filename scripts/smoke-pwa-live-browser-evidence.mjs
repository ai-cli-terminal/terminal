import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-live-browser-evidence");
const evidencePath =
  process.env.RA_PWA_LIVE_BROWSER_EVIDENCE_PATH ||
  path.join(artifactRoot, "ra-pwa-live-browser-evidence.json");
const transcriptPath = path.join(artifactRoot, "ra-pwa-live-browser-transcript.txt");
const userDataDir = path.join(artifactRoot, "browser-profile");
const wslRunRoot = `/tmp/ra-pwa-live-browser-evidence-${Date.now()}`;
const wslConfigHome = `${wslRunRoot}/config`;
const wslDataHome = `${wslRunRoot}/data`;
const wslRepoRoot = toWslPath(repoRoot);
const transcript = [];
let daemon = null;
let browserContext = null;
let staticServer = null;

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
    `mkdir -p ${bashQuote(wslConfigHome)} ${bashQuote(wslDataHome)}`,
    `export XDG_CONFIG_HOME=${bashQuote(wslConfigHome)}`,
    `export XDG_DATA_HOME=${bashQuote(wslDataHome)}`,
    `cd ${bashQuote(wslRepoRoot)}`,
    command,
  ].join("; ");
}

function spawnWsl(command, label) {
  const child = spawn("wsl.exe", ["--", "bash", "-lc", wslScript(command)], {
    cwd: repoRoot,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  child.capturedStdout = "";
  child.capturedStderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    child.capturedStdout += chunk;
    for (const line of chunk.split(/\r?\n/).filter(Boolean)) {
      log(`${label} stdout: ${line}`);
    }
  });
  child.stderr.on("data", (chunk) => {
    child.capturedStderr += chunk;
    for (const line of chunk.split(/\r?\n/).filter(Boolean)) {
      log(`${label} stderr: ${line}`);
    }
  });
  return child;
}

function runWsl(command, label, timeoutMs = 120000) {
  return new Promise((resolve, reject) => {
    const child = spawnWsl(command, label);
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
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

function waitForChildExit(child, label, timeoutMs = 30000) {
  return new Promise((resolve, reject) => {
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", (err) => {
      clearTimeout(timer);
      reject(err);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code, stdout, stderr });
    });
  });
}

function waitForOutput(child, label, pattern, timeoutMs = 30000) {
  return new Promise((resolve, reject) => {
    let buffer = child.capturedStdout || "";
    const existing = pattern.exec(buffer);
    if (existing) {
      resolve(existing);
      return;
    }
    const timer = setTimeout(() => {
      reject(new Error(`${label} did not emit ${pattern} within ${timeoutMs}ms`));
    }, timeoutMs);
    const onData = (chunk) => {
      buffer += chunk;
      const match = pattern.exec(buffer);
      if (match) {
        clearTimeout(timer);
        child.stdout.off("data", onData);
        resolve(match);
      }
    };
    child.stdout.on("data", onData);
    child.on("error", (err) => {
      clearTimeout(timer);
      reject(err);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      reject(new Error(`${label} exited before emitting ${pattern}; code=${code}`));
    });
  });
}

async function startStaticServer() {
  const pwaDir = path.join(repoRoot, "pwa");
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
    try {
      if (candidate && os.platform() === "win32" && requireExists(candidate)) {
        return candidate;
      }
    } catch {
      // Ignore and continue to Playwright-managed browser fallback.
    }
  }
  return undefined;
}

function requireExists(filePath) {
  return existsSync(filePath);
}

function parseLine(output, key) {
  const re = new RegExp(`^${key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*:\\s*(.+)$`, "m");
  const match = re.exec(output);
  if (!match) {
    throw new Error(`missing output key ${key}`);
  }
  return match[1].trim();
}

async function clickAndWaitStatus(page, selector, expected) {
  await page.click(selector);
  await page.waitForFunction(
    (text) => document.querySelector("#pair-status")?.textContent.includes(text),
    expected,
    { timeout: 15000 },
  );
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  log(`repoRoot=${repoRoot}`);
  log(`wslRunRoot=${wslRunRoot}`);

  log("building remote ai binary");
  await runWsl("cargo build --quiet --features remote --bin ai", "cargo-build", 180000);

  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const pwaUrl = staticInfo.url;
  log(`pwaUrl=${pwaUrl}`);

  const executablePath = browserExecutablePath();
  log(`browserExecutablePath=${executablePath || "(playwright default)"}`);
  browserContext = await chromium.launchPersistentContext(userDataDir, {
    headless: true,
    executablePath,
    viewport: { width: 1280, height: 960 },
  });
  const page = browserContext.pages()[0] || (await browserContext.newPage());
  await page.goto(`${pwaUrl}/index.html`, { waitUntil: "networkidle" });

  await clickAndWaitStatus(page, "#identity-button", "Companion identity");
  const identity = await page.evaluate(() => ({
    deviceId: document.querySelector("#device-id").value,
    noisePubkeyHex: document.querySelector("#noise-pubkey").value,
    approvalPubkeyHex: document.querySelector("#approval-pubkey").value,
  }));
  log(`deviceId=${identity.deviceId}`);

  const pairStart = await runWsl(
    `./target/debug/ai remote pair --ttl-seconds 600 --pwa-url ${bashQuote(`${pwaUrl}/index.html`)}`,
    "pair-start",
  );
  const pairPayloadJson = parseLine(pairStart.stdout, "pair_payload_json");
  const pairingCode = parseLine(pairStart.stdout, "code");
  await page.fill("#payload-input", pairPayloadJson);
  await clickAndWaitStatus(page, "#parse-button", "payload");

  await runWsl(
    [
      "./target/debug/ai remote pair",
      `--device-id ${bashQuote(identity.deviceId)}`,
      `--code ${bashQuote(pairingCode)}`,
      `--noise-pubkey-hex ${bashQuote(identity.noisePubkeyHex)}`,
      `--approval-pubkey-hex ${bashQuote(identity.approvalPubkeyHex)}`,
    ].join(" "),
    "pair-complete",
  );
  const devices = await runWsl("./target/debug/ai remote devices", "remote-devices");
  if (!devices.stdout.includes(identity.deviceId)) {
    throw new Error("registered device not found in remote devices output");
  }

  daemon = spawnWsl(
    `./target/debug/ai remote daemon --device-id ${bashQuote(identity.deviceId)}`,
    "daemon",
  );
  const transportModeMatch = await waitForOutput(
    daemon,
    "daemon-transport-mode",
    /PWA transport mode\s*:\s*([a-z0-9-]+)/,
    30000,
  );
  const transportMode = transportModeMatch[1];
  if (transportMode !== "live-loopback") {
    throw new Error(`unexpected daemon transport mode: ${transportMode}`);
  }
  log(`transportMode=${transportMode}`);
  const endpointMatch = await waitForOutput(
    daemon,
    "daemon",
    /PWA live endpoint\s*:\s*(http:\/\/127\.0\.0\.1:\d+)/,
    30000,
  );
  const liveEndpoint = endpointMatch[1];
  log(`liveEndpoint=${liveEndpoint}`);

  await page.fill("#live-endpoint", liveEndpoint);
  await page.click("#live-connect-button");
  await page.waitForFunction(
    () => document.querySelector("#live-state")?.textContent === "Connected",
    null,
    { timeout: 15000 },
  );
  await page.waitForFunction(
    () => document.querySelector("#monitor-state")?.textContent === "Connected",
    null,
    { timeout: 15000 },
  );
  const connectedScreenshot = path.join(artifactRoot, "pwa-live-connected.png");
  await page.screenshot({ path: connectedScreenshot, fullPage: true });

  await runWsl("./target/debug/ai remote arm --allow-high", "remote-arm");

  const approveGate = spawnWsl("./target/debug/ai __gate rm -rf build", "gate-approve");
  const approveExit = waitForChildExit(approveGate, "gate-approve", 30000);
  await page.waitForFunction(
    () => document.querySelector("#live-pending-count")?.textContent === "1",
    null,
    { timeout: 15000 },
  );
  const approvePendingScreenshot = path.join(artifactRoot, "pwa-live-approve-pending.png");
  await page.screenshot({ path: approvePendingScreenshot, fullPage: true });
  await page.click("#approve-button");
  await page.waitForFunction(
    () => document.querySelector("#pair-status")?.textContent.includes("Live 승인 응답"),
    null,
    { timeout: 15000 },
  );
  const approveResult = await approveExit;
  if (approveResult.code !== 0) {
    throw new Error(`approve gate expected exit 0, got ${approveResult.code}`);
  }

  const rejectGate = spawnWsl("./target/debug/ai __gate rm -rf build", "gate-reject");
  const rejectExit = waitForChildExit(rejectGate, "gate-reject", 30000);
  await page.waitForFunction(
    () => document.querySelector("#live-pending-count")?.textContent === "1",
    null,
    { timeout: 15000 },
  );
  const rejectPendingScreenshot = path.join(artifactRoot, "pwa-live-reject-pending.png");
  await page.screenshot({ path: rejectPendingScreenshot, fullPage: true });
  await page.click("#reject-button");
  await page.waitForFunction(
    () => document.querySelector("#pair-status")?.textContent.includes("Live 거부 응답"),
    null,
    { timeout: 15000 },
  );
  const rejectResult = await rejectExit;
  if (rejectResult.code === 0) {
    throw new Error("reject gate expected non-zero exit");
  }
  await page.waitForFunction(
    () =>
      document.querySelector("#monitor-approved")?.textContent === "1" &&
      document.querySelector("#monitor-rejected")?.textContent === "1",
    null,
    { timeout: 15000 },
  );

  const finalScreenshot = path.join(artifactRoot, "pwa-live-final.png");
  await page.screenshot({ path: finalScreenshot, fullPage: true });
  const monitor = await page.evaluate(() => ({
    state: document.querySelector("#monitor-state")?.textContent || "",
    pending: document.querySelector("#monitor-pending")?.textContent || "",
    received: document.querySelector("#monitor-received")?.textContent || "",
    sent: document.querySelector("#monitor-sent")?.textContent || "",
    approved: document.querySelector("#monitor-approved")?.textContent || "",
    rejected: document.querySelector("#monitor-rejected")?.textContent || "",
    history: Array.from(document.querySelectorAll("#monitor-event-log li"), (item) => item.textContent || ""),
  }));

  const evidence = {
    status: "passed",
    timestamp: new Date().toISOString(),
    repoRoot,
    evidencePath,
    transcriptPath,
    pwaUrl,
    liveEndpoint,
    transportMode,
    deviceId: identity.deviceId,
    wslRunRoot,
    screenshots: {
      connected: connectedScreenshot,
      approvePending: approvePendingScreenshot,
      rejectPending: rejectPendingScreenshot,
      final: finalScreenshot,
    },
    monitor,
    approve: {
      command: "ai __gate rm -rf build",
      exitCode: approveResult.code,
      stdoutTail: approveResult.stdout.split(/\r?\n/).filter(Boolean).slice(-20),
      stderrTail: approveResult.stderr.split(/\r?\n/).filter(Boolean).slice(-20),
    },
    reject: {
      command: "ai __gate rm -rf build",
      exitCode: rejectResult.code,
      stdoutTail: rejectResult.stdout.split(/\r?\n/).filter(Boolean).slice(-20),
      stderrTail: rejectResult.stderr.split(/\r?\n/).filter(Boolean).slice(-20),
    },
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
  await writeFile(transcriptPath, `${transcript.join("\n")}\n`, "utf8");
  console.log(`RA_PWA_LIVE_BROWSER_EVIDENCE_OK ${evidencePath}`);
}

async function cleanup() {
  if (browserContext) {
    await browserContext.close().catch(() => {});
  }
  if (daemon) {
    daemon.kill();
  }
  if (staticServer) {
    await new Promise((resolve) => staticServer.close(resolve)).catch(() => {});
  }
  if (transcript.length > 0) {
    await mkdir(artifactRoot, { recursive: true }).catch(() => {});
    await writeFile(transcriptPath, `${transcript.join("\n")}\n`, "utf8").catch(() => {});
  }
}

main()
  .catch(async (err) => {
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
    console.error(`RA_PWA_LIVE_BROWSER_EVIDENCE_FAILED ${evidencePath}`);
    console.error(err);
    process.exitCode = 1;
  })
  .finally(cleanup);
