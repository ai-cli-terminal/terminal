import assert from "node:assert/strict";
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
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-approve-reject-evidence");
const evidencePath =
  process.env.RA_PWA_RELAY_APPROVE_REJECT_EVIDENCE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-approve-reject-evidence.json");
const transcriptPath = path.join(artifactRoot, "ra-pwa-relay-approve-reject-transcript.txt");
const bridgeScriptPath = path.join(artifactRoot, "relay-bridge.py");
const userDataDir = path.join(artifactRoot, "browser-profile");
const wslRunRoot = `/tmp/ra-pwa-relay-approve-reject-${Date.now()}`;
const wslConfigHome = `${wslRunRoot}/config`;
const wslDataHome = `${wslRunRoot}/data`;
const wslCargoTargetDir = `${wslRunRoot}/target`;
const wslAiBin = `${wslCargoTargetDir}/debug/ai`;
const wslRepoRoot = toWslPath(repoRoot);
const transcript = [];

let bridge = null;
let daemon = null;
let browserContext = null;
let staticServer = null;

const PYTHON_RELAY_BRIDGE = String.raw`
import base64
import hashlib
import json
import re
import struct
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
MAX_MESSAGE_BYTES = (1 << 20) + 4096
tickets = {}
sessions = {}
lock = threading.RLock()
stats = {
    "acceptedFrames": 0,
    "deliveredFrames": 0,
    "expiredFrames": 0,
    "rejectedFrames": 0,
    "acceptedConnects": 0,
    "rejectedConnects": 0,
    "registeredTickets": 0,
    "rejectedTickets": 0,
    "openedConnections": 0,
    "closedConnections": 0,
}

def valid_id(value, max_len=96):
    return isinstance(value, str) and 0 < len(value) <= max_len and re.match(r"^[A-Za-z0-9._:-]+$", value)

def valid_token(value):
    return isinstance(value, str) and 32 <= len(value) <= 128 and re.match(r"^[A-Za-z0-9._:~-]+$", value)

def valid_hex64(value):
    return isinstance(value, str) and re.match(r"^[0-9a-fA-F]{64}$", value)

def validate_ticket(signed):
    ticket = signed.get("ticket") if isinstance(signed, dict) else None
    if not isinstance(ticket, dict):
        raise ValueError("relay ticket missing")
    if ticket.get("relay_protocol_version") != 1:
        raise ValueError("unsupported relay session protocol version")
    if ticket.get("transport") != "websocket":
        raise ValueError("relay session transport format error")
    if not valid_id(ticket.get("session_id")):
        raise ValueError("relay session_id format error")
    if not valid_token(ticket.get("session_token")):
        raise ValueError("relay session_token format error")
    now = int(time.time() * 1000)
    if not isinstance(ticket.get("issued_at_ms"), int) or not isinstance(ticket.get("expires_at_ms"), int):
        raise ValueError("relay session expiry format error")
    if ticket["expires_at_ms"] <= ticket["issued_at_ms"] or now >= ticket["expires_at_ms"]:
        raise ValueError("relay session expired")
    if not valid_hex64(ticket.get("daemon_pubkey_hex")):
        raise ValueError("relay daemon_pubkey_hex format error")
    if not valid_id(ticket.get("companion_device_id")):
        raise ValueError("relay companion device_id format error")
    if not valid_hex64(ticket.get("companion_noise_pubkey_hex")):
        raise ValueError("relay companion noise_pubkey_hex format error")
    if not valid_hex64(ticket.get("companion_approval_pubkey_hex")):
        raise ValueError("relay companion approval_pubkey_hex format error")
    if signed.get("mac_alg") != "hmac-sha256":
        raise ValueError("relay ticket mac_alg format error")
    if not isinstance(signed.get("mac_hex"), str) or not re.match(r"^[0-9a-fA-F]{64}$", signed.get("mac_hex")):
        raise ValueError("relay ticket mac_hex format error")
    return ticket

def validate_connect(connect, ticket):
    if connect.get("relay_protocol_version") != 1:
        raise ValueError("unsupported relay session protocol version")
    if connect.get("session_id") != ticket["session_id"]:
        raise ValueError("relay session_id mismatch")
    if connect.get("session_token") != ticket["session_token"]:
        raise ValueError("relay session_token mismatch")
    peer = connect.get("peer")
    if peer not in ("daemon", "companion"):
        raise ValueError("relay peer format error")
    if int(time.time() * 1000) >= ticket["expires_at_ms"]:
        raise ValueError("relay session expired")
    if peer == "daemon":
        if connect.get("daemon_pubkey_hex") != ticket["daemon_pubkey_hex"]:
            raise ValueError("relay daemon pubkey mismatch")
    else:
        if connect.get("device_id") != ticket["companion_device_id"]:
            raise ValueError("relay companion device_id mismatch")
        if connect.get("noise_pubkey_hex") != ticket["companion_noise_pubkey_hex"]:
            raise ValueError("relay companion noise pubkey mismatch")
        if connect.get("approval_pubkey_hex") != ticket["companion_approval_pubkey_hex"]:
            raise ValueError("relay companion approval pubkey mismatch")

def validate_frame(frame):
    if frame.get("relay_protocol_version") != 1:
        raise ValueError("unsupported relay protocol version")
    if not valid_id(frame.get("session_id")):
        raise ValueError("relay session_id format error")
    if frame.get("sender") not in ("daemon", "companion"):
        raise ValueError("relay sender format error")
    for key in ("sequence", "sent_at_ms", "expires_at_ms"):
        if not isinstance(frame.get(key), int) or frame[key] <= 0:
            raise ValueError("relay frame number format error")
    payload = frame.get("payload_json")
    if not isinstance(payload, str) or not payload or len(payload.encode("utf8")) > (1 << 20):
        raise ValueError("relay payload_json size error")
    return {
        "relay_protocol_version": frame["relay_protocol_version"],
        "session_id": frame["session_id"],
        "sender": frame["sender"],
        "sequence": frame["sequence"],
        "sent_at_ms": frame["sent_at_ms"],
        "expires_at_ms": frame["expires_at_ms"],
        "payload_json_bytes": len(payload.encode("utf8")),
    }

def new_session():
    return {
        "daemonToCompanion": [],
        "companionToDaemon": [],
        "lastDaemonSequence": 0,
        "lastCompanionSequence": 0,
        "daemonConnections": set(),
        "companionConnections": set(),
    }

def get_session(session_id):
    if session_id not in sessions:
        sessions[session_id] = new_session()
    return sessions[session_id]

def connection_set(session, role):
    return session["daemonConnections"] if role == "daemon" else session["companionConnections"]

def recipient(sender):
    return "companion" if sender == "daemon" else "daemon"

def queue_for(session, role):
    return session["daemonToCompanion"] if role == "companion" else session["companionToDaemon"]

def sender_queue(session, sender):
    return session["daemonToCompanion"] if sender == "daemon" else session["companionToDaemon"]

def last_key(sender):
    return "lastDaemonSequence" if sender == "daemon" else "lastCompanionSequence"

def queued_frames():
    return sum(len(s["daemonToCompanion"]) + len(s["companionToDaemon"]) for s in sessions.values())

def send_ws(sock, payload, opcode=1):
    if isinstance(payload, str):
        payload = payload.encode("utf8")
    header = bytes([0x80 | opcode])
    if len(payload) < 126:
        header += bytes([len(payload)])
    elif len(payload) <= 0xFFFF:
        header += bytes([126]) + struct.pack("!H", len(payload))
    else:
        header += bytes([127]) + struct.pack("!Q", len(payload))
    sock.sendall(header + payload)

def send_json(conn, body):
    send_ws(conn.sock, json.dumps(body, separators=(",", ":")))

def recv_exact(sock, n):
    data = b""
    while len(data) < n:
        chunk = sock.recv(n - len(data))
        if not chunk:
            return None
        data += chunk
    return data

def read_frame(sock):
    header = recv_exact(sock, 2)
    if not header:
        return None
    first, second = header
    opcode = first & 0x0F
    masked = bool(second & 0x80)
    length = second & 0x7F
    if length == 126:
        length = struct.unpack("!H", recv_exact(sock, 2))[0]
    elif length == 127:
        length = struct.unpack("!Q", recv_exact(sock, 8))[0]
    if length > MAX_MESSAGE_BYTES:
        raise ValueError("websocket message too large")
    mask = recv_exact(sock, 4) if masked else b""
    payload = recv_exact(sock, length) or b""
    if masked:
        payload = bytes(byte ^ mask[i % 4] for i, byte in enumerate(payload))
    return opcode, payload

class Conn:
    def __init__(self, sock, session_id, role):
        self.sock = sock
        self.session_id = session_id
        self.role = role
        self.authenticated = False

def flush(session_id, role):
    session = sessions.get(session_id)
    if not session:
        return
    conns = list(connection_set(session, role))
    if not conns:
        return
    queue = queue_for(session, role)
    now = int(time.time() * 1000)
    while queue:
        item = queue.pop(0)
        if now >= item["route"]["expires_at_ms"]:
            stats["expiredFrames"] += 1
            continue
        for conn in conns:
            send_json(conn, {"kind": "frame", "route": item["route"], "frame_json": item["frameJson"]})
        stats["deliveredFrames"] += 1

def authenticate(conn, text):
    try:
        connect = json.loads(text)
        ticket = tickets.get(conn.session_id)
        if not ticket:
            raise ValueError("relay session ticket missing")
        if connect.get("peer") != conn.role:
            raise ValueError("relay websocket peer mismatch")
        validate_connect(connect, ticket)
        conn.authenticated = True
        session = get_session(conn.session_id)
        connection_set(session, conn.role).add(conn)
        stats["acceptedConnects"] += 1
        send_json(conn, {"kind": "connected", "session_id": conn.session_id, "peer": conn.role})
        flush(conn.session_id, conn.role)
    except Exception as exc:
        stats["rejectedConnects"] += 1
        send_json(conn, {"kind": "error", "message": str(exc)})
        raise

def route(conn, text):
    try:
        frame = json.loads(text)
        route = validate_frame(frame)
        if route["session_id"] != conn.session_id:
            raise ValueError("relay websocket session mismatch")
        if route["sender"] != conn.role:
            raise ValueError("relay websocket sender mismatch")
        session = get_session(conn.session_id)
        key = last_key(route["sender"])
        if route["sequence"] <= session[key]:
            raise ValueError("relay sequence must increase for sender")
        session[key] = route["sequence"]
        sender_queue(session, route["sender"]).append({"route": route, "frameJson": text})
        stats["acceptedFrames"] += 1
        send_json(conn, {"kind": "queued", "route": route})
        flush(route["session_id"], recipient(route["sender"]))
    except Exception as exc:
        stats["rejectedFrames"] += 1
        send_json(conn, {"kind": "error", "message": str(exc)})

def websocket_loop(sock, session_id, role):
    conn = Conn(sock, session_id, role)
    with lock:
        stats["openedConnections"] += 1
    try:
        while True:
            frame = read_frame(sock)
            if frame is None:
                break
            opcode, payload = frame
            if opcode == 8:
                break
            if opcode == 9:
                send_ws(sock, payload, 10)
                continue
            if opcode != 1:
                raise ValueError("unsupported websocket opcode")
            text = payload.decode("utf8")
            with lock:
                if not conn.authenticated:
                    authenticate(conn, text)
                else:
                    route(conn, text)
    finally:
        with lock:
            session = sessions.get(session_id)
            if session:
                connection_set(session, role).discard(conn)
            stats["closedConnections"] += 1
        try:
            sock.close()
        except Exception:
            pass

class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        return

    def write_json(self, status, body):
        data = json.dumps(body, separators=(",", ":")).encode("utf8")
        self.send_response(status)
        self.send_header("content-type", "application/json; charset=utf-8")
        self.send_header("access-control-allow-origin", "*")
        self.send_header("access-control-allow-methods", "GET, POST, OPTIONS")
        self.send_header("access-control-allow-headers", "content-type")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("access-control-allow-origin", "*")
        self.send_header("access-control-allow-methods", "GET, POST, OPTIONS")
        self.send_header("access-control-allow-headers", "content-type")
        self.end_headers()

    def do_POST(self):
        parsed = urlparse(self.path)
        if parsed.path != "/sessions":
            self.write_json(404, {"status": "error", "message": "not found"})
            return
        try:
            size = int(self.headers.get("content-length", "0"))
            signed = json.loads(self.rfile.read(size).decode("utf8"))
            ticket = validate_ticket(signed)
            with lock:
                tickets[ticket["session_id"]] = ticket
                stats["registeredTickets"] += 1
            self.write_json(201, {"status": "registered", "session_id": ticket["session_id"]})
        except Exception as exc:
            with lock:
                stats["rejectedTickets"] += 1
            self.write_json(400, {"status": "error", "message": str(exc)})

    def do_GET(self):
        parsed = urlparse(self.path)
        if parsed.path == "/health":
            with lock:
                body = {"status": "ok", "sessions": len(sessions), "tickets": len(tickets), "queuedFrames": queued_frames(), "stats": stats}
            self.write_json(200, body)
            return
        if parsed.path != "/relay":
            self.write_json(404, {"status": "error", "message": "not found"})
            return
        query = parse_qs(parsed.query)
        session_id = (query.get("session_id") or [""])[0]
        role = (query.get("role") or [""])[0]
        key = self.headers.get("sec-websocket-key", "")
        if not valid_id(session_id) or role not in ("daemon", "companion") or not key:
            self.send_response(400)
            self.end_headers()
            return
        accept = base64.b64encode(hashlib.sha1((key + GUID).encode("ascii")).digest()).decode("ascii")
        self.connection.sendall((
            "HTTP/1.1 101 Switching Protocols\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            "Sec-WebSocket-Accept: " + accept + "\r\n\r\n"
        ).encode("ascii"))
        self.close_connection = True
        websocket_loop(self.connection, session_id, role)

server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
port = server.server_address[1]
print("RA_PWA_RELAY_BRIDGE_READY " + json.dumps({
    "httpUrl": "http://127.0.0.1:%d" % port,
    "healthUrl": "http://127.0.0.1:%d/health" % port,
    "sessionUrl": "http://127.0.0.1:%d/sessions" % port,
    "websocketUrl": "ws://127.0.0.1:%d/relay" % port,
}), flush=True)
server.serve_forever()
`;

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
    if (os.platform() === "win32" && existsSync(candidate)) {
      return candidate;
    }
  }
  return undefined;
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
  await writeFile(bridgeScriptPath, PYTHON_RELAY_BRIDGE, "utf8");
  log(`repoRoot=${repoRoot}`);
  log(`wslRunRoot=${wslRunRoot}`);

  bridge = spawnWsl(`python3 ${bashQuote(toWslPath(bridgeScriptPath))}`, "relay-bridge");
  const bridgeReady = await waitForOutput(
    bridge,
    "relay-bridge",
    /RA_PWA_RELAY_BRIDGE_READY (\{.+\})/,
    30000,
  );
  const bridgeInfo = JSON.parse(bridgeReady[1]);
  log(`relayWebSocket=${bridgeInfo.websocketUrl}`);

  log("building remote ai binary");
  await runWsl("cargo build --quiet --features remote --bin ai", "cargo-build", 180000);

  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const pwaUrl = staticInfo.url;
  const executablePath = browserExecutablePath();
  browserContext = await chromium.launchPersistentContext(userDataDir, {
    headless: true,
    executablePath,
    viewport: { width: 1280, height: 1040 },
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
    `${bashQuote(wslAiBin)} remote pair --ttl-seconds 600 --pwa-url ${bashQuote(`${pwaUrl}/index.html`)}`,
    "pair-start",
  );
  const pairPayloadJson = parseLine(pairStart.stdout, "pair_payload_json");
  const pairingCode = parseLine(pairStart.stdout, "code");
  await page.fill("#payload-input", pairPayloadJson);
  await clickAndWaitStatus(page, "#parse-button", "payload");

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

  daemon = spawnWsl(
    [
      `${bashQuote(wslAiBin)} remote daemon`,
      `--device-id ${bashQuote(identity.deviceId)}`,
      "--transport relay",
      `--relay-endpoint-url ${bashQuote(bridgeInfo.websocketUrl)}`,
      "--relay-ttl-seconds 300",
    ].join(" "),
    "daemon",
  );
  await waitForOutput(daemon, "daemon-relay-runtime", /PWA relay runtime\s*:\s*enabled/, 30000);
  const setupMatch = await waitForOutput(
    daemon,
    "daemon-relay-setup",
    /PWA relay setup json:\s*(\{.+\})/,
    30000,
  );
  const relaySetupJson = setupMatch[1];
  const relaySetup = JSON.parse(relaySetupJson);
  assert.equal(relaySetup.relayEndpointUrl, bridgeInfo.websocketUrl);
  assert.equal(relaySetup.companionIdentity.deviceId, identity.deviceId);

  await page.click('[data-mode="relay"]');
  await page.fill("#relay-setup-input", relaySetupJson);
  await page.click("#relay-setup-load-button");
  await page.waitForFunction(
    () => document.querySelector("#relay-state")?.textContent === "Ready",
    null,
    { timeout: 15000 },
  );
  await page.click("#relay-connect-button");
  await page.waitForFunction(
    () => document.querySelector("#relay-connection-state")?.textContent === "Connected",
    null,
    { timeout: 15000 },
  );
  const connectedScreenshot = path.join(artifactRoot, "pwa-relay-connected.png");
  await page.screenshot({ path: connectedScreenshot, fullPage: true });

  await runWsl(`${bashQuote(wslAiBin)} remote arm --allow-high`, "remote-arm");

  const approveGate = spawnWsl(`${bashQuote(wslAiBin)} __gate rm -rf build`, "gate-approve");
  const approveExit = waitForChildExit(approveGate, "gate-approve", 30000);
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-pending-count")?.textContent === "1" &&
      document.querySelector("#approval-source")?.textContent === "Relay",
    null,
    { timeout: 15000 },
  );
  const approvePendingScreenshot = path.join(artifactRoot, "pwa-relay-approve-pending.png");
  await page.screenshot({ path: approvePendingScreenshot, fullPage: true });
  await page.click("#approve-button");
  await page.waitForFunction(
    () => document.querySelector("#pair-status")?.textContent.includes("Relay 승인 응답"),
    null,
    { timeout: 15000 },
  );
  const approveResult = await approveExit;
  if (approveResult.code !== 0) {
    throw new Error(`approve gate expected exit 0, got ${approveResult.code}`);
  }

  const rejectGate = spawnWsl(`${bashQuote(wslAiBin)} __gate rm -rf build`, "gate-reject");
  const rejectExit = waitForChildExit(rejectGate, "gate-reject", 30000);
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-pending-count")?.textContent === "1" &&
      document.querySelector("#approval-source")?.textContent === "Relay",
    null,
    { timeout: 15000 },
  );
  const rejectPendingScreenshot = path.join(artifactRoot, "pwa-relay-reject-pending.png");
  await page.screenshot({ path: rejectPendingScreenshot, fullPage: true });
  await page.click("#reject-button");
  await page.waitForFunction(
    () => document.querySelector("#pair-status")?.textContent.includes("Relay 거부 응답"),
    null,
    { timeout: 15000 },
  );
  const rejectResult = await rejectExit;
  if (rejectResult.code === 0) {
    throw new Error("reject gate expected non-zero exit");
  }
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-pending-count")?.textContent === "0" &&
      document.querySelector("#relay-received-count")?.textContent === "2" &&
      document.querySelector("#relay-sent-count")?.textContent === "2" &&
      document.querySelector("#relay-approved-count")?.textContent === "1" &&
      document.querySelector("#relay-rejected-count")?.textContent === "1",
    null,
    { timeout: 15000 },
  );

  const finalScreenshot = path.join(artifactRoot, "pwa-relay-final.png");
  await page.screenshot({ path: finalScreenshot, fullPage: true });
  const relayMonitor = await page.evaluate(() => ({
    connection: document.querySelector("#relay-connection-state")?.textContent || "",
    lastEvent: document.querySelector("#relay-last-event")?.textContent || "",
    pending: document.querySelector("#relay-pending-count")?.textContent || "",
    received: document.querySelector("#relay-received-count")?.textContent || "",
    sent: document.querySelector("#relay-sent-count")?.textContent || "",
    approved: document.querySelector("#relay-approved-count")?.textContent || "",
    rejected: document.querySelector("#relay-rejected-count")?.textContent || "",
    queue: Array.from(document.querySelectorAll("#relay-approval-list li"), (item) => item.textContent || ""),
  }));

  const health = await fetch(bridgeInfo.healthUrl).then((response) => response.json());
  const evidence = {
    status: "passed",
    timestamp: new Date().toISOString(),
    repoRoot,
    evidencePath,
    transcriptPath,
    pwaUrl,
    relayEndpoint: bridgeInfo.websocketUrl,
    transportMode: "relay",
    deviceId: identity.deviceId,
    browserExecutablePath: executablePath || "playwright-default",
    wslRunRoot,
    screenshots: {
      connected: connectedScreenshot,
      approvePending: approvePendingScreenshot,
      rejectPending: rejectPendingScreenshot,
      final: finalScreenshot,
    },
    relayMonitor,
    bridgeHealth: health,
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
  console.log(`RA_PWA_RELAY_APPROVE_REJECT_EVIDENCE_OK ${evidencePath}`);
}

async function cleanup() {
  if (browserContext) {
    await browserContext.close().catch(() => {});
  }
  if (daemon) {
    daemon.kill();
  }
  if (bridge) {
    bridge.kill();
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
    console.error(`RA_PWA_RELAY_APPROVE_REJECT_EVIDENCE_FAILED ${evidencePath}`);
    console.error(err);
    process.exitCode = 1;
  })
  .finally(cleanup);
