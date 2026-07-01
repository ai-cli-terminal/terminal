export const COMPANION_IDENTITY_KEY = "ai-terminal-companion-identity-v1";
export const COMPANION_IDENTITY_DB = "ai-terminal-companion-v1";
export const COMPANION_IDENTITY_STORE = "identity";
export const ACTIVE_IDENTITY_ID = "active";
export const LIVE_TRANSPORT_PROTOCOL_VERSION = 1;

export function decodePairPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  const payload = url.searchParams.get("payload");
  return payload || "";
}

export function decodeApprovalPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  return url.searchParams.get("approval") || url.searchParams.get("request") || "";
}

export function parsePairingInput(text, currentSearch = "") {
  const raw = (text || "").trim();
  let candidate = raw;
  if (!candidate && currentSearch) {
    candidate = decodePairPayloadFromUrl(`https://companion.local/${currentSearch}`);
  } else if (candidate.startsWith("aiterminal://pair?") || candidate.includes("?payload=")) {
    candidate = decodePairPayloadFromUrl(candidate);
  }
  if (!candidate) {
    throw new Error("payload 없음");
  }

  let payload;
  try {
    payload = JSON.parse(candidate);
  } catch {
    throw new Error("payload JSON 파싱 실패");
  }
  validatePairingPayload(payload);
  return payload;
}

export function validatePairingPayload(payload) {
  if (payload.protocol_version !== 1) {
    throw new Error("지원하지 않는 protocol_version");
  }
  if (!/^[0-9]{6}$/.test(payload.pairing_code || "")) {
    throw new Error("pairing_code 형식 오류");
  }
  if (!/^[0-9a-f]{64}$/i.test(payload.daemon_pubkey_hex || "")) {
    throw new Error("daemon_pubkey_hex 형식 오류");
  }
  if (typeof payload.transport_addr !== "string" || payload.transport_addr.length < 6) {
    throw new Error("transport_addr 형식 오류");
  }
  if (!Number.isSafeInteger(payload.expires_at_ms) || payload.expires_at_ms <= 0) {
    throw new Error("expires_at_ms 형식 오류");
  }
}

export function parseApprovalInput(text, currentSearch = "") {
  const raw = (text || "").trim();
  let candidate = raw;
  if (!candidate && currentSearch) {
    candidate = decodeApprovalPayloadFromUrl(`https://companion.local/${currentSearch}`);
  } else if (candidate.startsWith("aiterminal://approve?") || candidate.includes("?approval=")) {
    candidate = decodeApprovalPayloadFromUrl(candidate);
  }
  if (!candidate) {
    throw new Error("approval request 없음");
  }
  let request;
  try {
    request = JSON.parse(candidate);
  } catch {
    throw new Error("approval request JSON 파싱 실패");
  }
  validateApprovalRequest(request);
  return request;
}

export function validateApprovalRequest(request) {
  if (!Array.isArray(request.approval_id) || request.approval_id.length === 0) {
    throw new Error("approval_id 형식 오류");
  }
  if (!Array.isArray(request.nonce) || request.nonce.length !== 32) {
    throw new Error("nonce 형식 오류");
  }
  for (const byte of [...request.approval_id, ...request.nonce]) {
    if (!Number.isInteger(byte) || byte < 0 || byte > 255) {
      throw new Error("byte array 형식 오류");
    }
  }
  if (typeof request.command_masked !== "string" || request.command_masked.length === 0) {
    throw new Error("command_masked 형식 오류");
  }
  if (typeof request.context_hash !== "string" || request.context_hash.length === 0) {
    throw new Error("context_hash 형식 오류");
  }
  if (!Number.isSafeInteger(request.expires_at) || request.expires_at <= 0) {
    throw new Error("expires_at 형식 오류");
  }
  if (!Number.isSafeInteger(request.device_epoch) || request.device_epoch < 0) {
    throw new Error("device_epoch 형식 오류");
  }
}

export function commandForPairing(payload, device) {
  const deviceId = shellToken(device.deviceId || "");
  const noise = (device.noisePubkeyHex || "").trim();
  const approval = (device.approvalPubkeyHex || "").trim();
  if (!deviceId || !/^[0-9a-f]{64}$/i.test(noise) || !/^[0-9a-f]{64}$/i.test(approval)) {
    return "-";
  }
  return [
    "ai remote pair",
    `--device-id ${deviceId}`,
    `--code ${payload.pairing_code}`,
    `--noise-pubkey-hex ${noise}`,
    `--approval-pubkey-hex ${approval}`,
  ].join(" ");
}

export function commandForApprovalVerify(request, response, deviceId) {
  validateApprovalRequest(request);
  validateApprovalResponse(response);
  const id = shellToken(deviceId || "");
  if (!id) return "-";
  return [
    "ai remote approval-verify",
    `--device-id ${id}`,
    `--request-json ${shellToken(JSON.stringify(request))}`,
    `--response-json ${shellToken(approvalResponseJson(response))}`,
  ].join(" ");
}

export function liveHelloMessage(identity) {
  const deviceId = identity?.deviceId || "";
  const noisePubkeyHex = identity?.noisePubkeyHex || "";
  const approvalPubkeyHex = identity?.approvalPubkeyHex || "";
  if (
    !/^[A-Za-z0-9._:-]+$/.test(deviceId) ||
    !/^[0-9a-f]{64}$/i.test(noisePubkeyHex) ||
    !/^[0-9a-f]{64}$/i.test(approvalPubkeyHex)
  ) {
    throw new Error("companion identity 형식 오류");
  }
  return {
    type: "hello",
    protocol_version: LIVE_TRANSPORT_PROTOCOL_VERSION,
    device_id: deviceId,
    noise_pubkey_hex: noisePubkeyHex,
    approval_pubkey_hex: approvalPubkeyHex,
  };
}

export function liveApprovalRequestMessage(request) {
  validateApprovalRequest(request);
  return { type: "approval_request", request };
}

export function liveApprovalResponseMessage(response) {
  validateApprovalResponse(response);
  return { type: "approval_response", response };
}

export function livePingMessage(nonce) {
  if (typeof nonce !== "string" || nonce.length === 0 || nonce.length > 128) {
    throw new Error("heartbeat nonce 형식 오류");
  }
  return { type: "ping", nonce };
}

export function livePongMessage(nonce) {
  if (typeof nonce !== "string" || nonce.length === 0 || nonce.length > 128) {
    throw new Error("heartbeat nonce 형식 오류");
  }
  return { type: "pong", nonce };
}

export function liveErrorMessage(message) {
  if (typeof message !== "string" || message.trim().length === 0) {
    throw new Error("error message 형식 오류");
  }
  return { type: "error", message };
}

export function liveTransportJson(message) {
  validateLiveTransportMessage(message);
  return JSON.stringify(message);
}

export function liveEndpointUrls(baseUrl) {
  const root = new URL("/", baseUrl);
  const href = root.href.replace(/\/$/, "");
  return {
    baseUrl: href,
    healthUrl: `${href}/health`,
    eventsUrl: `${href}/events`,
    messageUrl: `${href}/message`,
  };
}

export function liveEventSourceUrl(baseUrl) {
  return liveEndpointUrls(baseUrl).eventsUrl;
}

export function liveApprovalRequestKey(request) {
  validateApprovalRequest(request);
  return `${request.approval_id.join(".")}:${request.nonce.join(".")}`;
}

export function liveApprovalQueueNext(queue, message, maxItems = 8) {
  validateLiveTransportMessage(message);
  const current = Array.isArray(queue) ? queue : [];
  if (message.type !== "approval_request") {
    return current.slice(0, maxItems);
  }
  const key = liveApprovalRequestKey(message.request);
  const withoutDuplicate = current.filter((item) => item.key !== key);
  return [{ key, request: message.request, receivedAtMs: Date.now() }, ...withoutDuplicate].slice(
    0,
    maxItems,
  );
}

export function liveMonitorInitialState(nowMs = Date.now()) {
  return {
    state: "Disconnected",
    endpoint: "",
    deviceId: "",
    pendingCount: 0,
    receivedCount: 0,
    sentCount: 0,
    approvedCount: 0,
    rejectedCount: 0,
    errorCount: 0,
    connectedAtMs: 0,
    lastHeartbeatAtMs: 0,
    lastResponseAtMs: 0,
    updatedAtMs: nowMs,
    history: [],
  };
}

export function liveMonitorNext(state, event, nowMs = Date.now(), maxHistory = 12) {
  const current = state || liveMonitorInitialState(nowMs);
  const next = { ...current, updatedAtMs: nowMs, history: [...(current.history || [])] };
  const type = event?.type || "unknown";
  const label = event?.label || type;
  if (type === "connected") {
    next.state = "Connected";
    next.endpoint = event.endpoint || next.endpoint;
    next.deviceId = event.deviceId || next.deviceId;
    next.connectedAtMs = nowMs;
  } else if (type === "disconnected") {
    next.state = "Disconnected";
    next.pendingCount = 0;
  } else if (type === "waiting") {
    next.state = "Waiting";
  } else if (type === "approval_request") {
    next.state = "Connected";
    next.pendingCount = Math.max(0, Number(event.pendingCount ?? next.pendingCount));
    next.receivedCount += 1;
  } else if (type === "approval_response") {
    next.state = "Connected";
    next.pendingCount = Math.max(0, Number(event.pendingCount ?? next.pendingCount));
    next.sentCount += 1;
    next.lastResponseAtMs = nowMs;
    if (event.approve === true) next.approvedCount += 1;
    if (event.approve === false) next.rejectedCount += 1;
  } else if (type === "ping" || type === "pong") {
    next.state = "Connected";
    next.lastHeartbeatAtMs = nowMs;
  } else if (type === "error") {
    next.errorCount += 1;
  }
  next.history = [{ type, label, atMs: nowMs }, ...next.history].slice(0, maxHistory);
  return next;
}

export function liveMessageRequest(message) {
  return {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: liveTransportJson(message),
  };
}

export async function postLiveTransportMessage(baseUrl, message, fetchImpl = globalThis.fetch) {
  if (typeof fetchImpl !== "function") {
    throw new Error("fetch 미지원");
  }
  const { messageUrl } = liveEndpointUrls(baseUrl);
  const response = await fetchImpl(messageUrl, liveMessageRequest(message));
  const reply = parseLiveTransportMessage(await response.text());
  if (!response.ok) {
    const err = new Error(reply.type === "error" ? reply.message : "live endpoint error");
    err.status = response.status;
    err.reply = reply;
    throw err;
  }
  return reply;
}

export function parseLiveTransportMessage(text) {
  let message;
  try {
    message = JSON.parse(text);
  } catch {
    throw new Error("live transport JSON 파싱 실패");
  }
  validateLiveTransportMessage(message);
  return message;
}

export function validateLiveTransportMessage(message) {
  switch (message?.type) {
    case "hello":
      if (message.protocol_version !== LIVE_TRANSPORT_PROTOCOL_VERSION) {
        throw new Error("지원하지 않는 live protocol_version");
      }
      liveHelloMessage({
        deviceId: message.device_id,
        noisePubkeyHex: message.noise_pubkey_hex,
        approvalPubkeyHex: message.approval_pubkey_hex,
      });
      return;
    case "approval_request":
      validateApprovalRequest(message.request);
      return;
    case "approval_response":
      validateApprovalResponse(message.response);
      return;
    case "ping":
    case "pong":
      if (typeof message.nonce !== "string" || message.nonce.length === 0 || message.nonce.length > 128) {
        throw new Error("heartbeat nonce 형식 오류");
      }
      return;
    case "error":
      if (typeof message.message !== "string" || message.message.trim().length === 0) {
        throw new Error("error message 형식 오류");
      }
      return;
    default:
      throw new Error("지원하지 않는 live transport message type");
  }
}

export async function generateCompanionIdentity(webCrypto = globalThis.crypto) {
  return (await generateCompanionKeyMaterial(webCrypto)).identity;
}

export async function generateCompanionKeyMaterial(webCrypto = globalThis.crypto) {
  if (!webCrypto?.subtle || typeof webCrypto.getRandomValues !== "function") {
    throw new Error("WebCrypto 미지원");
  }
  const noise = await webCrypto.subtle.generateKey({ name: "X25519" }, false, ["deriveBits"]);
  const approval = await webCrypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
  const random = new Uint8Array(4);
  webCrypto.getRandomValues(random);
  const identity = {
    deviceId: `web-${bytesToHex(random)}`,
    noisePubkeyHex: await publicKeyHex(webCrypto, noise.publicKey),
    approvalPubkeyHex: await publicKeyHex(webCrypto, approval.publicKey),
  };
  return {
    identity,
    keyMaterial: { noise, approval },
  };
}

export function saveCompanionIdentity(storage, identity) {
  if (!storage) return;
  storage.setItem(COMPANION_IDENTITY_KEY, JSON.stringify(identity));
}

export function loadCompanionIdentity(storage) {
  if (!storage) return null;
  const raw = storage.getItem(COMPANION_IDENTITY_KEY);
  if (!raw) return null;
  try {
    const identity = JSON.parse(raw);
    if (
      !/^[A-Za-z0-9._:-]+$/.test(identity.deviceId || "") ||
      !/^[0-9a-f]{64}$/i.test(identity.noisePubkeyHex || "") ||
      !/^[0-9a-f]{64}$/i.test(identity.approvalPubkeyHex || "")
    ) {
      return null;
    }
    return identity;
  } catch {
    return null;
  }
}

export async function saveCompanionKeyMaterial(indexedDb, identity, keyMaterial) {
  if (!indexedDb) {
    throw new Error("IndexedDB 미지원");
  }
  const db = await openIdentityDb(indexedDb);
  try {
    await idbPut(db, COMPANION_IDENTITY_STORE, {
      id: ACTIVE_IDENTITY_ID,
      identity,
      keyMaterial,
      createdAtMs: Date.now(),
    });
  } finally {
    db.close?.();
  }
}

export async function loadCompanionKeyMaterial(indexedDb) {
  if (!indexedDb) return null;
  const db = await openIdentityDb(indexedDb);
  try {
    const record = await idbGet(db, COMPANION_IDENTITY_STORE, ACTIVE_IDENTITY_ID);
    if (!record || !loadCompanionIdentity(memoryStorageFor(record.identity))) {
      return null;
    }
    if (!record.keyMaterial?.noise?.privateKey || !record.keyMaterial?.approval?.privateKey) {
      return null;
    }
    return record;
  } finally {
    db.close?.();
  }
}

export async function signApprovalBytes(bytes, keyMaterial, webCrypto = globalThis.crypto) {
  const input = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const signature = await webCrypto.subtle.sign(
    { name: "Ed25519" },
    keyMaterial.approval.privateKey,
    input,
  );
  return bytesToHex(new Uint8Array(signature));
}

export async function verifyApprovalBytes(bytes, signatureHex, keyMaterial, webCrypto = globalThis.crypto) {
  const input = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  return webCrypto.subtle.verify(
    { name: "Ed25519" },
    keyMaterial.approval.publicKey,
    hexToBytes(signatureHex),
    input,
  );
}

export function approvalSigningBytes(request, approve) {
  validateApprovalRequest(request);
  const out = new Uint8Array(request.approval_id.length + request.nonce.length + 1);
  out.set(request.approval_id, 0);
  out.set(request.nonce, request.approval_id.length);
  out[out.length - 1] = approve ? 1 : 0;
  return out;
}

export async function approvalResponseForRequest(
  request,
  approve,
  keyMaterial,
  webCrypto = globalThis.crypto,
) {
  if (!keyMaterial?.approval?.privateKey) {
    throw new Error("approval private key 없음");
  }
  const signature = await webCrypto.subtle.sign(
    { name: "Ed25519" },
    keyMaterial.approval.privateKey,
    approvalSigningBytes(request, approve),
  );
  return {
    approval_id: request.approval_id,
    nonce: request.nonce,
    approve,
    sig: Array.from(new Uint8Array(signature)),
  };
}

export function approvalResponseJson(response) {
  return JSON.stringify(response);
}

export function validateApprovalResponse(response) {
  if (!Array.isArray(response.approval_id) || response.approval_id.length === 0) {
    throw new Error("approval response id 형식 오류");
  }
  if (!Array.isArray(response.nonce) || response.nonce.length !== 32) {
    throw new Error("approval response nonce 형식 오류");
  }
  if (typeof response.approve !== "boolean") {
    throw new Error("approval response decision 형식 오류");
  }
  if (!Array.isArray(response.sig) || response.sig.length !== 64) {
    throw new Error("approval response sig 형식 오류");
  }
  for (const byte of [...response.approval_id, ...response.nonce, ...response.sig]) {
    if (!Number.isInteger(byte) || byte < 0 || byte > 255) {
      throw new Error("approval response byte array 형식 오류");
    }
  }
}

export async function deriveNoiseSharedSecretHex(peerPubkeyHex, keyMaterial, webCrypto = globalThis.crypto) {
  const peerPublicKey = await webCrypto.subtle.importKey(
    "raw",
    hexToBytes(peerPubkeyHex),
    { name: "X25519" },
    false,
    [],
  );
  const bits = await webCrypto.subtle.deriveBits(
    { name: "X25519", public: peerPublicKey },
    keyMaterial.noise.privateKey,
    256,
  );
  return bytesToHex(new Uint8Array(bits));
}

function applyIdentity(identity) {
  document.querySelector("#device-id").value = identity.deviceId;
  document.querySelector("#noise-pubkey").value = identity.noisePubkeyHex;
  document.querySelector("#approval-pubkey").value = identity.approvalPubkeyHex;
}

async function publicKeyHex(webCrypto, publicKey) {
  const raw = await webCrypto.subtle.exportKey("raw", publicKey);
  return bytesToHex(new Uint8Array(raw));
}

function bytesToHex(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function hexToBytes(hex) {
  if (!/^[0-9a-f]*$/i.test(hex) || hex.length % 2 !== 0) {
    throw new Error("hex 형식 오류");
  }
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    out[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

function openIdentityDb(indexedDb) {
  return new Promise((resolve, reject) => {
    const request = indexedDb.open(COMPANION_IDENTITY_DB, 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(COMPANION_IDENTITY_STORE)) {
        db.createObjectStore(COMPANION_IDENTITY_STORE, { keyPath: "id" });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error || new Error("IndexedDB open 실패"));
  });
}

function idbPut(db, storeName, value) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, "readwrite");
    tx.objectStore(storeName).put(value);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error || new Error("IndexedDB write 실패"));
    tx.onabort = () => reject(tx.error || new Error("IndexedDB write 중단"));
  });
}

function idbGet(db, storeName, key) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, "readonly");
    const request = tx.objectStore(storeName).get(key);
    request.onsuccess = () => resolve(request.result || null);
    request.onerror = () => reject(request.error || new Error("IndexedDB read 실패"));
  });
}

function memoryStorageFor(identity) {
  return {
    getItem: () => JSON.stringify(identity),
  };
}

function shellToken(value) {
  const clean = value.trim();
  if (!clean) return "";
  if (/^[A-Za-z0-9._:-]+$/.test(clean)) return clean;
  return `'${clean.replaceAll("'", "'\\''")}'`;
}

function formatExpiry(ms) {
  if (!Number.isSafeInteger(ms) || ms <= 0) return "-";
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString();
}

function formatMonitorTime(ms) {
  if (!Number.isSafeInteger(ms) || ms <= 0) return "-";
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleTimeString();
}

function renderPayload(payload) {
  document.querySelector("#pair-code").textContent = payload.pairing_code;
  document.querySelector("#pair-expires").textContent = formatExpiry(payload.expires_at_ms);
  document.querySelector("#pair-transport").textContent = payload.transport_addr;
  document.querySelector("#pair-key").textContent = payload.daemon_pubkey_hex;
  updateCommand(payload);
}

function renderApprovalRequest(request, source = "Manual") {
  document.querySelector("#approval-command").textContent = request.command_masked;
  document.querySelector("#approval-context").textContent = request.context_hash;
  document.querySelector("#approval-source").textContent = source;
}

function renderApprovalVerifyCommand(request, response) {
  document.querySelector("#approval-verify-command").textContent = commandForApprovalVerify(
    request,
    response,
    document.querySelector("#device-id").value,
  );
}

function updateCommand(payload) {
  const device = {
    deviceId: document.querySelector("#device-id").value,
    noisePubkeyHex: document.querySelector("#noise-pubkey").value,
    approvalPubkeyHex: document.querySelector("#approval-pubkey").value,
  };
  document.querySelector("#complete-command").textContent = commandForPairing(payload, device);
}

function setStatus(text, kind = "") {
  const el = document.querySelector("#pair-status");
  el.textContent = text;
  el.className = `status-line ${kind}`.trim();
}

function setLiveState(text, kind = "") {
  const el = document.querySelector("#live-state");
  el.textContent = text;
  el.className = kind;
}

function setLiveLastEvent(text) {
  document.querySelector("#live-last-event").textContent = text;
}

function renderLiveQueue(queue) {
  document.querySelector("#live-pending-count").textContent = String(queue.length);
  const list = document.querySelector("#live-approval-list");
  list.replaceChildren();
  if (queue.length === 0) {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "No live approvals";
    list.append(empty);
    return;
  }
  for (const item of queue) {
    const li = document.createElement("li");
    li.textContent = `${item.request.command_masked} | ${item.request.context_hash}`;
    list.append(li);
  }
}

function renderMonitor(monitor) {
  document.querySelector("#monitor-state").textContent = monitor.state;
  document.querySelector("#monitor-endpoint").textContent = monitor.endpoint || "-";
  document.querySelector("#monitor-device").textContent = monitor.deviceId || "-";
  document.querySelector("#monitor-pending").textContent = String(monitor.pendingCount);
  document.querySelector("#monitor-received").textContent = String(monitor.receivedCount);
  document.querySelector("#monitor-sent").textContent = String(monitor.sentCount);
  document.querySelector("#monitor-approved").textContent = String(monitor.approvedCount);
  document.querySelector("#monitor-rejected").textContent = String(monitor.rejectedCount);
  document.querySelector("#monitor-heartbeat").textContent = formatMonitorTime(monitor.lastHeartbeatAtMs);
  document.querySelector("#monitor-response").textContent = formatMonitorTime(monitor.lastResponseAtMs);

  const list = document.querySelector("#monitor-event-log");
  list.replaceChildren();
  if (!monitor.history.length) {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "No monitor events";
    list.append(empty);
    return;
  }
  for (const event of monitor.history) {
    const li = document.createElement("li");
    li.textContent = `${formatMonitorTime(event.atMs)} | ${event.label}`;
    list.append(li);
  }
}

function init() {
  const input = document.querySelector("#payload-input");
  const approvalInput = document.querySelector("#approval-input");
  const parse = document.querySelector("#parse-button");
  const clear = document.querySelector("#clear-button");
  const identity = document.querySelector("#identity-button");
  const approvalParse = document.querySelector("#approval-parse-button");
  const approveButton = document.querySelector("#approve-button");
  const rejectButton = document.querySelector("#reject-button");
  const copyResponse = document.querySelector("#copy-response-button");
  const copyVerify = document.querySelector("#copy-verify-button");
  const liveEndpointInput = document.querySelector("#live-endpoint");
  const liveConnectButton = document.querySelector("#live-connect-button");
  const liveDisconnectButton = document.querySelector("#live-disconnect-button");
  let activePayload = null;
  let activeApprovalRequest = null;
  let activeApprovalResponse = null;
  let activeKeyMaterial = null;
  let liveBaseUrl = "";
  let liveEventSource = null;
  let liveApprovalQueue = [];
  let liveMonitor = liveMonitorInitialState();
  renderMonitor(liveMonitor);

  function updateMonitor(event) {
    liveMonitor = liveMonitorNext(liveMonitor, event);
    renderMonitor(liveMonitor);
  }

  async function loadActiveIdentityAndKeys() {
    const savedIdentity = loadCompanionIdentity(window.localStorage);
    const record = await loadCompanionKeyMaterial(window.indexedDB);
    const identity = savedIdentity || record?.identity || null;
    if (!identity) {
      throw new Error("Companion identity 없음");
    }
    if (record?.identity?.deviceId === identity.deviceId) {
      activeKeyMaterial = record.keyMaterial;
    }
    if (!activeKeyMaterial) {
      throw new Error("approval private key 없음");
    }
    applyIdentity(identity);
    return identity;
  }

  function closeLiveEvents(stateText = "Disconnected") {
    liveEventSource?.close();
    liveEventSource = null;
    liveBaseUrl = "";
    setLiveState(stateText);
    liveConnectButton.disabled = false;
    liveDisconnectButton.disabled = true;
    updateMonitor({ type: stateText === "Waiting" ? "waiting" : "disconnected", label: stateText });
  }

  function handleLiveEventData(data) {
    const message = parseLiveTransportMessage(data);
    setLiveLastEvent(message.type);
    if (message.type === "approval_request") {
      liveApprovalQueue = liveApprovalQueueNext(liveApprovalQueue, message);
      activeApprovalRequest = message.request;
      activeApprovalResponse = null;
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      document.querySelector("#approval-response").textContent = "-";
      document.querySelector("#approval-verify-command").textContent = "-";
      renderApprovalRequest(activeApprovalRequest, "Live");
      renderLiveQueue(liveApprovalQueue);
      updateMonitor({
        type: "approval_request",
        label: `approval_request ${message.request.command_masked}`,
        pendingCount: liveApprovalQueue.length,
      });
      setStatus("Live 승인 요청 수신됨", "ok");
      return;
    }
    if (message.type === "ping") {
      setLiveState("Connected", "ok");
      updateMonitor({ type: "ping", label: `ping ${message.nonce}` });
    }
  }

  function openLiveEvents() {
    if (typeof window.EventSource !== "function") {
      throw new Error("EventSource 미지원");
    }
    liveEventSource?.close();
    liveEventSource = new window.EventSource(liveEventSourceUrl(liveBaseUrl));
    liveEventSource.onopen = () => setLiveState("Connected", "ok");
    liveEventSource.onmessage = (event) => {
      try {
        handleLiveEventData(event.data);
      } catch (err) {
        updateMonitor({ type: "error", label: err.message });
        setStatus(err.message, "error");
      }
    };
    liveEventSource.onerror = () => {
      if (liveBaseUrl) {
        setLiveState("Waiting");
        updateMonitor({ type: "waiting", label: "EventSource waiting" });
      }
    };
  }

  async function connectLive() {
    liveConnectButton.disabled = true;
    try {
      const urls = liveEndpointUrls(liveEndpointInput.value.trim());
      liveBaseUrl = urls.baseUrl;
      liveEndpointInput.value = liveBaseUrl;
      const identity = await loadActiveIdentityAndKeys();
      await postLiveTransportMessage(liveBaseUrl, liveHelloMessage(identity));
      openLiveEvents();
      setLiveState("Connected", "ok");
      setLiveLastEvent("hello");
      setStatus("Live companion 연결됨", "ok");
      updateMonitor({
        type: "connected",
        label: "hello",
        endpoint: liveBaseUrl,
        deviceId: identity.deviceId,
      });
      liveDisconnectButton.disabled = false;
    } catch (err) {
      closeLiveEvents("Disconnected");
      updateMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    } finally {
      liveConnectButton.disabled = Boolean(liveBaseUrl);
    }
  }

  function parseInput() {
    try {
      activePayload = parsePairingInput(input.value, window.location.search);
      input.value = JSON.stringify(activePayload, null, 2);
      renderPayload(activePayload);
      setStatus("페어링 payload 확인됨", "ok");
    } catch (err) {
      activePayload = null;
      setStatus(err.message, "error");
    }
  }

  parse.addEventListener("click", parseInput);
  clear.addEventListener("click", () => {
    input.value = "";
    activePayload = null;
    setStatus("페어링 payload 대기");
    renderPayload({
      pairing_code: "-",
      expires_at_ms: 0,
      transport_addr: "-",
      daemon_pubkey_hex: "-",
    });
  });
  for (const id of ["device-id", "noise-pubkey", "approval-pubkey"]) {
    document.querySelector(`#${id}`).addEventListener("input", () => {
      if (activePayload) updateCommand(activePayload);
      if (activeApprovalRequest && activeApprovalResponse) {
        renderApprovalVerifyCommand(activeApprovalRequest, activeApprovalResponse);
      }
    });
  }
  identity.addEventListener("click", async () => {
    identity.disabled = true;
    try {
      const generated = await generateCompanionKeyMaterial();
      await saveCompanionKeyMaterial(window.indexedDB, generated.identity, generated.keyMaterial);
      saveCompanionIdentity(window.localStorage, generated.identity);
      applyIdentity(generated.identity);
      activeKeyMaterial = generated.keyMaterial;
      if (activePayload) updateCommand(activePayload);
      setStatus("Companion identity 생성됨", "ok");
    } catch (err) {
      setStatus(err.message, "error");
    } finally {
      identity.disabled = false;
    }
  });
  function parseApprovalRequest() {
    try {
      activeApprovalRequest = parseApprovalInput(approvalInput.value);
      activeApprovalResponse = null;
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      renderApprovalRequest(activeApprovalRequest, "Manual");
      document.querySelector("#approval-verify-command").textContent = "-";
      setStatus("승인 요청 확인됨", "ok");
    } catch (err) {
      activeApprovalRequest = null;
      activeApprovalResponse = null;
      setStatus(err.message, "error");
    }
  }
  async function signApprovalDecision(approve) {
    try {
      if (!activeApprovalRequest) {
        activeApprovalRequest = parseApprovalInput(approvalInput.value);
        renderApprovalRequest(activeApprovalRequest);
      }
      if (!activeKeyMaterial) {
        const record = await loadCompanionKeyMaterial(window.indexedDB);
        activeKeyMaterial = record?.keyMaterial || null;
      }
      const response = await approvalResponseForRequest(activeApprovalRequest, approve, activeKeyMaterial);
      activeApprovalResponse = response;
      document.querySelector("#approval-response").textContent = approvalResponseJson(response);
      renderApprovalVerifyCommand(activeApprovalRequest, response);
      if (liveBaseUrl) {
        await postLiveTransportMessage(liveBaseUrl, liveApprovalResponseMessage(response));
        const sentKey = liveApprovalRequestKey(activeApprovalRequest);
        liveApprovalQueue = liveApprovalQueue.filter((item) => item.key !== sentKey);
        renderLiveQueue(liveApprovalQueue);
        setLiveLastEvent("approval_response");
        updateMonitor({
          type: "approval_response",
          label: approve ? "approval_response approve" : "approval_response reject",
          approve,
          pendingCount: liveApprovalQueue.length,
        });
        setStatus(approve ? "Live 승인 응답 전송됨" : "Live 거부 응답 전송됨", "ok");
      } else {
        setStatus(approve ? "승인 응답 서명됨" : "거부 응답 서명됨", "ok");
      }
    } catch (err) {
      updateMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    }
  }
  liveConnectButton.addEventListener("click", connectLive);
  liveDisconnectButton.addEventListener("click", () => {
    closeLiveEvents("Disconnected");
    setStatus("Live companion 연결 해제됨");
  });
  approvalParse.addEventListener("click", parseApprovalRequest);
  approveButton.addEventListener("click", () => signApprovalDecision(true));
  rejectButton.addEventListener("click", () => signApprovalDecision(false));
  copyResponse.addEventListener("click", async () => {
    const text = document.querySelector("#approval-response").textContent;
    if (!text || text === "-") {
      setStatus("복사할 승인 응답 없음", "error");
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      setStatus("승인 응답 복사됨", "ok");
    } catch {
      setStatus("클립보드 복사 실패", "error");
    }
  });
  copyVerify.addEventListener("click", async () => {
    const text = document.querySelector("#approval-verify-command").textContent;
    if (!text || text === "-") {
      setStatus("복사할 검증 명령 없음", "error");
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      setStatus("검증 명령 복사됨", "ok");
    } catch {
      setStatus("클립보드 복사 실패", "error");
    }
  });
  for (const tab of document.querySelectorAll(".tab")) {
    tab.addEventListener("click", () => {
      document.querySelectorAll(".tab").forEach((item) => item.classList.remove("active"));
      tab.classList.add("active");
      const mode = tab.dataset.mode;
      const target =
        mode === "approve"
          ? document.querySelector(".approval-section")
          : mode === "monitor"
            ? document.querySelector(".monitor-section")
            : document.querySelector("#detail-title");
      target?.scrollIntoView({ block: "start", behavior: "smooth" });
    });
  }
  const savedIdentity = loadCompanionIdentity(window.localStorage);
  if (savedIdentity) {
    loadCompanionKeyMaterial(window.indexedDB)
      .then((record) => {
        if (record?.identity?.deviceId === savedIdentity.deviceId) {
          applyIdentity(savedIdentity);
          activeKeyMaterial = record.keyMaterial;
          if (activePayload) updateCommand(activePayload);
          if (activeApprovalRequest && activeApprovalResponse) {
            renderApprovalVerifyCommand(activeApprovalRequest, activeApprovalResponse);
          }
          setStatus("Companion identity 복원됨", "ok");
        }
      })
      .catch(() => {});
  }
  if (window.location.search.includes("payload=")) {
    parseInput();
  }
  if (window.location.search.includes("approval=") || window.location.search.includes("request=")) {
    try {
      activeApprovalRequest = parseApprovalInput("", window.location.search);
      activeApprovalResponse = null;
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      renderApprovalRequest(activeApprovalRequest);
      document.querySelector("#approval-verify-command").textContent = "-";
      setStatus("승인 요청 확인됨", "ok");
    } catch (err) {
      setStatus(err.message, "error");
    }
  }
  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.register("./sw.js").catch(() => {});
  }
}

if (typeof document !== "undefined") {
  init();
}
