export const COMPANION_IDENTITY_KEY = "ai-terminal-companion-identity-v1";
export const COMPANION_IDENTITY_DB = "ai-terminal-companion-v1";
export const COMPANION_IDENTITY_STORE = "identity";
export const ACTIVE_IDENTITY_ID = "active";
export const LIVE_TRANSPORT_PROTOCOL_VERSION = 1;
export const RELAY_TRANSPORT_PROTOCOL_VERSION = 1;
export const DEFAULT_RELAY_FRAME_TTL_MS = 30_000;
export const DEFAULT_RELAY_SESSION_TTL_MS = 5 * 60 * 1000;
export const RELAY_TICKET_MAC_ALG_HMAC_SHA256 = "hmac-sha256";
export const PWA_TRANSPORT_MODE_LIVE_LOOPBACK = "live-loopback";
export const PWA_TRANSPORT_MODE_RELAY = "relay";
export const PWA_RELAY_DEPLOYMENT_MODE_SELF_HOSTED = "self-hosted";
export const PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK = "private-network";
export const PWA_RELAY_DEPLOYMENT_MODE_MANAGED = "managed";
export const PWA_RELAY_DEPLOYMENT_MODES = Object.freeze([
  PWA_RELAY_DEPLOYMENT_MODE_SELF_HOSTED,
  PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK,
  PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
]);
export const PWA_RELAY_SELECTED_DEPLOYMENT_MODE = PWA_RELAY_DEPLOYMENT_MODE_SELF_HOSTED;
export const PWA_RELAY_DEPLOYMENT_DECISION = Object.freeze({
  selectedMode: PWA_RELAY_SELECTED_DEPLOYMENT_MODE,
  selectedSubstrate: "websocket",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  relayTransportReadiness: "planned",
  endpointPolicy: "wss-production-localhost-ws-development",
  ticketSecretOwner: "daemon",
  deferredModes: Object.freeze([
    PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK,
    PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  ]),
});
export const MAX_RELAY_SESSION_ID_LENGTH = 96;
export const MIN_RELAY_SESSION_TOKEN_LENGTH = 32;
export const MAX_RELAY_SESSION_TOKEN_LENGTH = 128;
export const MAX_RELAY_DEVICE_ID_LENGTH = 96;
export const MIN_RELAY_TICKET_HMAC_KEY_BYTES = 32;
export const MAX_RELAY_PAYLOAD_JSON_BYTES = 1 << 20;

export function decodePairPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  const payload = url.searchParams.get("payload");
  return payload || "";
}

export function decodeApprovalPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  return url.searchParams.get("approval") || url.searchParams.get("request") || "";
}

export function decodeRelaySetupPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  return url.searchParams.get("relaySetup") || url.searchParams.get("setup") || "";
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

export function parseRelayRuntimeSetupInput(text, currentSearch = "") {
  const raw = (text || "").trim();
  let candidate = raw;
  if (!candidate && currentSearch) {
    candidate = decodeRelaySetupPayloadFromUrl(`https://companion.local/${currentSearch}`);
  } else if (
    candidate.startsWith("aiterminal://relay?") ||
    candidate.includes("?relaySetup=") ||
    candidate.includes("?setup=")
  ) {
    candidate = decodeRelaySetupPayloadFromUrl(candidate);
  }
  if (!candidate) {
    throw new Error("relay setup 없음");
  }
  let setup;
  try {
    setup = JSON.parse(candidate);
  } catch {
    throw new Error("relay setup JSON 파싱 실패");
  }
  validateRelayRuntimeSetupMetadata(setup);
  return setup;
}

export function validateRelayRuntimeSetupMetadata(setup) {
  if (!setup || typeof setup !== "object" || Array.isArray(setup)) {
    throw new Error("relay setup 형식 오류");
  }
  rejectRelaySetupSecretFields(setup);
  if (setup.relayProtocolVersion !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay setup protocol_version");
  }
  if (setup.transportMode !== PWA_TRANSPORT_MODE_RELAY) {
    throw new Error("relay setup transportMode 형식 오류");
  }
  if (setup.deploymentMode !== PWA_RELAY_SELECTED_DEPLOYMENT_MODE) {
    throw new Error("relay setup deploymentMode 형식 오류");
  }
  if (!validRelayWebSocketEndpointUrl(setup.relayEndpointUrl)) {
    throw new Error("relay setup endpoint URL 형식 오류");
  }
  validateSignedRelaySessionTicketMetadata(setup.signedSessionTicket);
  const ticket = setup.signedSessionTicket.ticket;
  if (setup.daemonConnect?.peer !== "daemon") {
    throw new Error("relay setup daemonConnect peer 형식 오류");
  }
  if (setup.companionConnect?.peer !== "companion") {
    throw new Error("relay setup companionConnect peer 형식 오류");
  }
  validateRelaySessionConnect(ticket, setup.daemonConnect, ticket.issued_at_ms);
  validateRelaySessionConnect(ticket, setup.companionConnect, ticket.issued_at_ms);
  if (!validCompanionIdentity(setup.companionIdentity)) {
    throw new Error("relay setup companionIdentity 형식 오류");
  }
  if (
    setup.companionIdentity.deviceId !== ticket.companion_device_id ||
    setup.companionIdentity.noisePubkeyHex !== ticket.companion_noise_pubkey_hex ||
    setup.companionIdentity.approvalPubkeyHex !== ticket.companion_approval_pubkey_hex
  ) {
    throw new Error("relay setup companionIdentity mismatch");
  }
  if (typeof setup.operatorSetupText !== "string" || setup.operatorSetupText.trim().length < 12) {
    throw new Error("relay setup operatorSetupText 형식 오류");
  }
}

export function relayRuntimeSetupPreflight(setup, nowMs = Date.now()) {
  validateRelayRuntimeSetupMetadata(setup);
  return relayTransportUxPreflight(
    {
      transportMode: setup.transportMode,
      relayEndpointUrl: setup.relayEndpointUrl,
      signedSessionTicket: setup.signedSessionTicket,
      companionIdentity: setup.companionIdentity,
      deploymentMode: setup.deploymentMode,
      operatorSetupText: setup.operatorSetupText,
    },
    nowMs,
  );
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

export function validRelaySessionId(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_RELAY_SESSION_ID_LENGTH &&
    /^[A-Za-z0-9._:-]+$/.test(value)
  );
}

export function validRelaySender(value) {
  return value === "daemon" || value === "companion";
}

export function validRelaySessionToken(value) {
  return (
    typeof value === "string" &&
    value.length >= MIN_RELAY_SESSION_TOKEN_LENGTH &&
    value.length <= MAX_RELAY_SESSION_TOKEN_LENGTH &&
    /^[A-Za-z0-9._:~-]+$/.test(value)
  );
}

export function validRelayDeviceId(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_RELAY_DEVICE_ID_LENGTH &&
    /^[A-Za-z0-9._:-]+$/.test(value)
  );
}

function validRelayPubkeyHex(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/i.test(value);
}

export function createRelaySessionTicket({
  sessionId,
  sessionToken,
  issuedAtMs,
  expiresAtMs = issuedAtMs + DEFAULT_RELAY_SESSION_TTL_MS,
  daemonPubkeyHex,
  companionDeviceId,
  companionNoisePubkeyHex,
  companionApprovalPubkeyHex,
  transport = "websocket",
}) {
  const ticket = {
    relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
    transport,
    session_id: sessionId,
    session_token: sessionToken,
    issued_at_ms: issuedAtMs,
    expires_at_ms: expiresAtMs,
    daemon_pubkey_hex: daemonPubkeyHex,
    companion_device_id: companionDeviceId,
    companion_noise_pubkey_hex: companionNoisePubkeyHex,
    companion_approval_pubkey_hex: companionApprovalPubkeyHex,
  };
  validateRelaySessionTicket(ticket);
  return ticket;
}

export function validateRelaySessionTicket(ticket) {
  if (ticket?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay session protocol_version");
  }
  if (ticket.transport !== "websocket") {
    throw new Error("relay session transport 형식 오류");
  }
  if (!validRelaySessionId(ticket.session_id)) {
    throw new Error("relay session_id 형식 오류");
  }
  if (!validRelaySessionToken(ticket.session_token)) {
    throw new Error("relay session_token 형식 오류");
  }
  if (
    !Number.isSafeInteger(ticket.issued_at_ms) ||
    ticket.issued_at_ms <= 0 ||
    !Number.isSafeInteger(ticket.expires_at_ms) ||
    ticket.expires_at_ms <= ticket.issued_at_ms ||
    ticket.expires_at_ms - ticket.issued_at_ms > DEFAULT_RELAY_SESSION_TTL_MS
  ) {
    throw new Error("relay session expiry 형식 오류");
  }
  if (!validRelayPubkeyHex(ticket.daemon_pubkey_hex)) {
    throw new Error("relay daemon_pubkey_hex 형식 오류");
  }
  if (!validRelayDeviceId(ticket.companion_device_id)) {
    throw new Error("relay companion device_id 형식 오류");
  }
  if (!validRelayPubkeyHex(ticket.companion_noise_pubkey_hex)) {
    throw new Error("relay companion noise_pubkey_hex 형식 오류");
  }
  if (!validRelayPubkeyHex(ticket.companion_approval_pubkey_hex)) {
    throw new Error("relay companion approval_pubkey_hex 형식 오류");
  }
}

export function relaySessionExpiredAt(ticket, nowMs) {
  validateRelaySessionTicket(ticket);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("relay session now_ms 형식 오류");
  }
  return nowMs >= ticket.expires_at_ms;
}

export function relaySessionConnect(ticket, peer) {
  validateRelaySessionTicket(ticket);
  const connect =
    peer === "daemon"
      ? {
          relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
          session_id: ticket.session_id,
          peer,
          session_token: ticket.session_token,
          daemon_pubkey_hex: ticket.daemon_pubkey_hex,
        }
      : {
          relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
          session_id: ticket.session_id,
          peer,
          session_token: ticket.session_token,
          device_id: ticket.companion_device_id,
          noise_pubkey_hex: ticket.companion_noise_pubkey_hex,
          approval_pubkey_hex: ticket.companion_approval_pubkey_hex,
        };
  validateRelaySessionConnectMetadata(connect);
  return connect;
}

export function relaySessionConnectJson(connect) {
  validateRelaySessionConnectMetadata(connect);
  return JSON.stringify(connect);
}

export function validateRelaySessionConnect(ticket, connect, nowMs) {
  validateRelaySessionTicket(ticket);
  validateRelaySessionConnectMetadata(connect);
  if (relaySessionExpiredAt(ticket, nowMs)) {
    throw new Error("relay session expired");
  }
  if (connect.session_id !== ticket.session_id) {
    throw new Error("relay session_id mismatch");
  }
  if (connect.session_token !== ticket.session_token) {
    throw new Error("relay session_token mismatch");
  }
  if (connect.peer === "daemon") {
    if (connect.daemon_pubkey_hex !== ticket.daemon_pubkey_hex) {
      throw new Error("relay daemon pubkey mismatch");
    }
    return;
  }
  if (connect.device_id !== ticket.companion_device_id) {
    throw new Error("relay companion device_id mismatch");
  }
  if (connect.noise_pubkey_hex !== ticket.companion_noise_pubkey_hex) {
    throw new Error("relay companion noise pubkey mismatch");
  }
  if (connect.approval_pubkey_hex !== ticket.companion_approval_pubkey_hex) {
    throw new Error("relay companion approval pubkey mismatch");
  }
}

export function relaySessionTicketSigningPayload(ticket) {
  validateRelaySessionTicket(ticket);
  return [
    "ai-terminal-relay-ticket-v1",
    `relay_protocol_version=${ticket.relay_protocol_version}`,
    `transport=${ticket.transport}`,
    `session_id=${ticket.session_id}`,
    `session_token=${ticket.session_token}`,
    `issued_at_ms=${ticket.issued_at_ms}`,
    `expires_at_ms=${ticket.expires_at_ms}`,
    `daemon_pubkey_hex=${ticket.daemon_pubkey_hex}`,
    `companion_device_id=${ticket.companion_device_id}`,
    `companion_noise_pubkey_hex=${ticket.companion_noise_pubkey_hex}`,
    `companion_approval_pubkey_hex=${ticket.companion_approval_pubkey_hex}`,
    "",
  ].join("\n");
}

export async function relaySessionTicketHmacSha256Hex(
  ticket,
  secret,
  webCrypto = globalThis.crypto,
) {
  validateRelaySessionTicket(ticket);
  const secretBytes = relayTicketHmacKeyBytes(secret);
  const key = await webCrypto.subtle.importKey(
    "raw",
    secretBytes,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const payload = new TextEncoder().encode(relaySessionTicketSigningPayload(ticket));
  const mac = await webCrypto.subtle.sign("HMAC", key, payload);
  return bytesToHex(new Uint8Array(mac));
}

export async function createSignedRelaySessionTicket(
  ticket,
  secret,
  webCrypto = globalThis.crypto,
) {
  const signed = {
    ticket,
    mac_alg: RELAY_TICKET_MAC_ALG_HMAC_SHA256,
    mac_hex: await relaySessionTicketHmacSha256Hex(ticket, secret, webCrypto),
  };
  validateSignedRelaySessionTicketMetadata(signed);
  return signed;
}

export function validateSignedRelaySessionTicketMetadata(signed) {
  validateRelaySessionTicket(signed?.ticket);
  if (signed.mac_alg !== RELAY_TICKET_MAC_ALG_HMAC_SHA256) {
    throw new Error("relay ticket mac_alg 형식 오류");
  }
  if (typeof signed.mac_hex !== "string" || !/^[0-9a-f]{64}$/i.test(signed.mac_hex)) {
    throw new Error("relay ticket mac_hex 형식 오류");
  }
  if (signed.key_id !== undefined && !validRelayTicketKeyId(signed.key_id)) {
    throw new Error("relay ticket key_id 형식 오류");
  }
}

export async function validateSignedRelaySessionTicket(
  signed,
  secret,
  webCrypto = globalThis.crypto,
) {
  validateSignedRelaySessionTicketMetadata(signed);
  const expected = await relaySessionTicketHmacSha256Hex(signed.ticket, secret, webCrypto);
  if (!constantTimeHexEqual(signed.mac_hex, expected)) {
    throw new Error("relay ticket mac mismatch");
  }
  return signed.ticket;
}

export async function validateSignedRelaySessionConnect(
  signed,
  connect,
  nowMs,
  secret,
  webCrypto = globalThis.crypto,
) {
  const ticket = await validateSignedRelaySessionTicket(signed, secret, webCrypto);
  validateRelaySessionConnect(ticket, connect, nowMs);
}

export function relayDeploymentShapeDecision() {
  return {
    ...PWA_RELAY_DEPLOYMENT_DECISION,
    knownModes: [...PWA_RELAY_DEPLOYMENT_MODES],
    deferredModes: [...PWA_RELAY_DEPLOYMENT_DECISION.deferredModes],
    guardrails: [
      "product_default_remains_live_loopback",
      "relay_ui_requires_selected_self_hosted_mode",
      "production_endpoint_requires_wss",
      "localhost_ws_is_development_only",
      "ticket_hmac_secret_stays_daemon_owned",
    ],
  };
}

export function relayTransportUxPreflight(config = {}, nowMs = Date.now()) {
  const {
    transportMode = PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
    relayEndpointUrl = "",
    signedSessionTicket = null,
    companionIdentity = null,
    deploymentMode = "",
    operatorSetupText = "",
  } = config || {};
  const blockers = [];
  const addBlocker = (code) => {
    if (!blockers.includes(code)) {
      blockers.push(code);
    }
  };

  if (transportMode !== PWA_TRANSPORT_MODE_RELAY) {
    addBlocker("transport_mode_not_relay");
  }
  if (typeof relayEndpointUrl !== "string" || relayEndpointUrl.trim().length === 0) {
    addBlocker("relay_endpoint_url_missing");
  } else if (!validRelayWebSocketEndpointUrl(relayEndpointUrl)) {
    addBlocker("relay_endpoint_url_invalid");
  }
  if (typeof deploymentMode !== "string" || deploymentMode.trim().length === 0) {
    addBlocker("relay_deployment_mode_missing");
  } else if (!PWA_RELAY_DEPLOYMENT_MODES.includes(deploymentMode)) {
    addBlocker("relay_deployment_mode_invalid");
  } else if (deploymentMode !== PWA_RELAY_SELECTED_DEPLOYMENT_MODE) {
    addBlocker("relay_deployment_mode_not_selected");
  }
  if (typeof operatorSetupText !== "string" || operatorSetupText.trim().length < 12) {
    addBlocker("relay_operator_setup_text_missing");
  }

  const identityValid = validCompanionIdentity(companionIdentity);
  if (!identityValid) {
    addBlocker("companion_identity_missing");
  }

  let ticket = null;
  if (!signedSessionTicket) {
    addBlocker("relay_signed_ticket_missing");
  } else {
    try {
      validateSignedRelaySessionTicketMetadata(signedSessionTicket);
      ticket = signedSessionTicket.ticket;
      if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
        addBlocker("relay_now_ms_invalid");
      } else if (relaySessionExpiredAt(ticket, nowMs)) {
        addBlocker("relay_signed_ticket_expired");
      }
    } catch {
      addBlocker("relay_signed_ticket_invalid");
    }
  }

  if (ticket && identityValid) {
    if (
      ticket.companion_device_id !== companionIdentity.deviceId ||
      ticket.companion_noise_pubkey_hex !== companionIdentity.noisePubkeyHex ||
      ticket.companion_approval_pubkey_hex !== companionIdentity.approvalPubkeyHex
    ) {
      addBlocker("relay_ticket_identity_mismatch");
    }
  }

  const ready = blockers.length === 0;
  return {
    status: ready ? "ready" : "hidden",
    transportMode,
    deploymentMode: deploymentMode || "",
    relayVisible: ready,
    relayEnabled: ready,
    blockers,
  };
}

function validateRelaySessionConnectMetadata(connect) {
  if (connect?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay session protocol_version");
  }
  if (!validRelaySessionId(connect.session_id)) {
    throw new Error("relay session_id 형식 오류");
  }
  if (!validRelaySender(connect.peer)) {
    throw new Error("relay peer 형식 오류");
  }
  if (!validRelaySessionToken(connect.session_token)) {
    throw new Error("relay session_token 형식 오류");
  }
  if (connect.peer === "daemon") {
    if (!validRelayPubkeyHex(connect.daemon_pubkey_hex)) {
      throw new Error("relay daemon_pubkey_hex 형식 오류");
    }
    if (
      connect.device_id !== undefined ||
      connect.noise_pubkey_hex !== undefined ||
      connect.approval_pubkey_hex !== undefined
    ) {
      throw new Error("relay daemon connect companion field 오류");
    }
    return;
  }
  if (!validRelayDeviceId(connect.device_id)) {
    throw new Error("relay companion device_id 형식 오류");
  }
  if (!validRelayPubkeyHex(connect.noise_pubkey_hex)) {
    throw new Error("relay companion noise_pubkey_hex 형식 오류");
  }
  if (!validRelayPubkeyHex(connect.approval_pubkey_hex)) {
    throw new Error("relay companion approval_pubkey_hex 형식 오류");
  }
  if (connect.daemon_pubkey_hex !== undefined) {
    throw new Error("relay companion connect daemon field 오류");
  }
}

export function relayFrameFromLiveMessage(
  sessionId,
  sender,
  sequence,
  sentAtMs,
  expiresAtMs,
  message,
) {
  const frame = {
    relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
    session_id: sessionId,
    sender,
    sequence,
    sent_at_ms: sentAtMs,
    expires_at_ms: expiresAtMs,
    payload_json: liveTransportJson(message),
  };
  validateRelayFrame(frame);
  return frame;
}

export function relayFrameWithDefaultExpiry(sessionId, sender, sequence, sentAtMs, message) {
  if (!Number.isSafeInteger(sentAtMs) || sentAtMs <= 0) {
    throw new Error("relay sent_at_ms 형식 오류");
  }
  return relayFrameFromLiveMessage(
    sessionId,
    sender,
    sequence,
    sentAtMs,
    sentAtMs + DEFAULT_RELAY_FRAME_TTL_MS,
    message,
  );
}

export function relayFrameJson(frame) {
  validateRelayFrame(frame);
  return JSON.stringify(frame);
}

export function parseRelayFrame(text) {
  let frame;
  try {
    frame = JSON.parse(text);
  } catch {
    throw new Error("relay frame JSON 파싱 실패");
  }
  validateRelayFrame(frame);
  return frame;
}

export function relayFramePayloadMessage(frame) {
  validateRelayFrame(frame);
  return parseLiveTransportMessage(frame.payload_json);
}

export function relayFrameRouteEnvelope(frameOrText) {
  const frame =
    typeof frameOrText === "string"
      ? parseRelayFrame(frameOrText)
      : validateRelayFrame(frameOrText) || frameOrText;
  return {
    relay_protocol_version: frame.relay_protocol_version,
    session_id: frame.session_id,
    sender: frame.sender,
    sequence: frame.sequence,
    sent_at_ms: frame.sent_at_ms,
    expires_at_ms: frame.expires_at_ms,
    payload_json_bytes: new TextEncoder().encode(frame.payload_json).length,
  };
}

export function createRelayEndpoint(
  sessionId,
  sender,
  frameTtlMs = DEFAULT_RELAY_FRAME_TTL_MS,
) {
  const endpoint = {
    sessionId,
    sender,
    nextSequence: 1,
    frameTtlMs,
  };
  validateRelayEndpoint(endpoint);
  return endpoint;
}

export function validateRelayEndpoint(endpoint) {
  if (!validRelaySessionId(endpoint?.sessionId)) {
    throw new Error("relay endpoint sessionId 형식 오류");
  }
  if (!validRelaySender(endpoint.sender)) {
    throw new Error("relay endpoint sender 형식 오류");
  }
  if (!Number.isSafeInteger(endpoint.nextSequence) || endpoint.nextSequence <= 0) {
    throw new Error("relay endpoint nextSequence 형식 오류");
  }
  if (!Number.isSafeInteger(endpoint.frameTtlMs) || endpoint.frameTtlMs <= 0) {
    throw new Error("relay endpoint frameTtlMs 형식 오류");
  }
}

export function relayEndpointNextFrame(endpoint, message, nowMs = Date.now()) {
  validateRelayEndpoint(endpoint);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("relay sent_at_ms 형식 오류");
  }
  const frame = relayFrameFromLiveMessage(
    endpoint.sessionId,
    endpoint.sender,
    endpoint.nextSequence,
    nowMs,
    nowMs + endpoint.frameTtlMs,
    message,
  );
  if (endpoint.nextSequence >= Number.MAX_SAFE_INTEGER) {
    throw new Error("relay sequence overflow");
  }
  endpoint.nextSequence += 1;
  return frame;
}

export function relayEndpointAcceptFrame(endpoint, frameOrText, nowMs = Date.now()) {
  validateRelayEndpoint(endpoint);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("relay now_ms 형식 오류");
  }
  const frame =
    typeof frameOrText === "string"
      ? parseRelayFrame(frameOrText)
      : validateRelayFrame(frameOrText) || frameOrText;
  if (frame.session_id !== endpoint.sessionId) {
    throw new Error("relay session_id mismatch");
  }
  if (frame.sender === endpoint.sender) {
    throw new Error("relay sender matches endpoint");
  }
  if (nowMs >= frame.expires_at_ms) {
    return null;
  }
  return relayFramePayloadMessage(frame);
}

export function relayWebSocketConnectUrl(relayEndpointUrl, connect) {
  if (!validRelayWebSocketEndpointUrl(relayEndpointUrl)) {
    throw new Error("relay websocket endpoint URL 형식 오류");
  }
  validateRelaySessionConnectMetadata(connect);
  const url = new URL(relayEndpointUrl);
  url.searchParams.set("session_id", connect.session_id);
  url.searchParams.set("role", connect.peer);
  return url.toString();
}

export function relayEndpointLoopInitialState(connect, frameTtlMs = DEFAULT_RELAY_FRAME_TTL_MS) {
  validateRelaySessionConnectMetadata(connect);
  const endpoint = createRelayEndpoint(connect.session_id, connect.peer, frameTtlMs);
  const loop = {
    connect: { ...connect },
    connectJson: relaySessionConnectJson(connect),
    endpoint,
    webSocketUrl: "",
    connected: false,
    sentCount: 0,
    queuedCount: 0,
    receivedCount: 0,
    droppedCount: 0,
    errorCount: 0,
  };
  validateRelayEndpointLoop(loop);
  return loop;
}

export function relayCompanionEndpointLoopFromSetup(
  setup,
  frameTtlMs = DEFAULT_RELAY_FRAME_TTL_MS,
  nowMs = Date.now(),
) {
  const preflight = relayRuntimeSetupPreflight(setup, nowMs);
  if (!preflight.relayEnabled) {
    throw new Error(`relay setup not ready: ${preflight.blockers.join(",")}`);
  }
  const loop = relayEndpointLoopInitialState(setup.companionConnect, frameTtlMs);
  loop.webSocketUrl = relayWebSocketConnectUrl(setup.relayEndpointUrl, setup.companionConnect);
  loop.preflight = preflight;
  return loop;
}

export function relayEndpointLoopConnectJson(loop) {
  validateRelayEndpointLoop(loop);
  return relaySessionConnectJson(loop.connect);
}

export function relayEndpointLoopNextFrame(loop, message, nowMs = Date.now()) {
  validateRelayEndpointLoop(loop);
  const frame = relayEndpointNextFrame(loop.endpoint, message, nowMs);
  loop.sentCount += 1;
  return {
    kind: "frame",
    frame,
    frameJson: relayFrameJson(frame),
    route: relayFrameRouteEnvelope(frame),
  };
}

export function relayEndpointLoopAcceptSocketMessage(loop, socketMessage, nowMs = Date.now()) {
  validateRelayEndpointLoop(loop);
  const message = parseRelayWebSocketEnvelope(socketMessage);
  if (message.kind === "connected") {
    if (message.session_id !== loop.connect.session_id || message.peer !== loop.connect.peer) {
      throw new Error("relay websocket connected envelope mismatch");
    }
    loop.connected = true;
    return { kind: "connected", envelope: message };
  }
  if (message.kind === "queued") {
    validateRelayRouteEnvelopeMetadata(message.route);
    if (message.route.session_id !== loop.endpoint.sessionId || message.route.sender !== loop.endpoint.sender) {
      throw new Error("relay websocket queued envelope mismatch");
    }
    loop.queuedCount += 1;
    return { kind: "queued", route: message.route };
  }
  if (message.kind === "frame") {
    if (typeof message.frame_json !== "string" || message.frame_json.length === 0) {
      throw new Error("relay websocket frame_json 형식 오류");
    }
    const route = relayFrameRouteEnvelope(message.frame_json);
    if (route.session_id !== loop.endpoint.sessionId || route.sender === loop.endpoint.sender) {
      throw new Error("relay websocket frame envelope mismatch");
    }
    const liveMessage = relayEndpointAcceptFrame(loop.endpoint, message.frame_json, nowMs);
    if (liveMessage === null) {
      loop.droppedCount += 1;
      return { kind: "dropped", route, liveMessage: null };
    }
    loop.receivedCount += 1;
    return { kind: "live_message", route, liveMessage };
  }
  if (message.kind === "error") {
    loop.errorCount += 1;
    return {
      kind: "error",
      message: typeof message.message === "string" ? message.message : "relay websocket error",
      envelope: message,
    };
  }
  throw new Error("relay websocket envelope kind 형식 오류");
}

export function relayEndpointExchange(
  daemonEndpoint,
  companionEndpoint,
  daemonMessage,
  companionReply,
  nowMs = Date.now(),
) {
  validateRelayEndpoint(daemonEndpoint);
  validateRelayEndpoint(companionEndpoint);
  validateLiveTransportMessage(daemonMessage);
  validateLiveTransportMessage(companionReply);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0 || nowMs > Number.MAX_SAFE_INTEGER - 3) {
    throw new Error("relay exchange now_ms 형식 오류");
  }
  if (daemonEndpoint.sessionId !== companionEndpoint.sessionId) {
    throw new Error("relay exchange session_id mismatch");
  }
  if (daemonEndpoint.sender !== "daemon") {
    throw new Error("relay exchange daemon endpoint sender mismatch");
  }
  if (companionEndpoint.sender !== "companion") {
    throw new Error("relay exchange companion endpoint sender mismatch");
  }

  const daemonFrame = relayEndpointNextFrame(daemonEndpoint, daemonMessage, nowMs);
  const daemonFrameJson = relayFrameJson(daemonFrame);
  const companionMessage = relayEndpointAcceptFrame(companionEndpoint, daemonFrameJson, nowMs + 1);
  if (companionMessage === null) {
    throw new Error("relay exchange daemon frame expired");
  }

  const companionFrame = relayEndpointNextFrame(companionEndpoint, companionReply, nowMs + 2);
  const companionFrameJson = relayFrameJson(companionFrame);
  const daemonReply = relayEndpointAcceptFrame(daemonEndpoint, companionFrameJson, nowMs + 3);
  if (daemonReply === null) {
    throw new Error("relay exchange companion frame expired");
  }

  return {
    daemonFrame,
    daemonFrameJson,
    companionMessage,
    companionFrame,
    companionFrameJson,
    daemonReply,
  };
}

export function validateRelayFrame(frame) {
  if (frame?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay protocol_version");
  }
  if (!validRelaySessionId(frame.session_id)) {
    throw new Error("relay session_id 형식 오류");
  }
  if (!validRelaySender(frame.sender)) {
    throw new Error("relay sender 형식 오류");
  }
  if (!Number.isSafeInteger(frame.sequence) || frame.sequence <= 0) {
    throw new Error("relay sequence 형식 오류");
  }
  if (!Number.isSafeInteger(frame.sent_at_ms) || frame.sent_at_ms <= 0) {
    throw new Error("relay sent_at_ms 형식 오류");
  }
  if (!Number.isSafeInteger(frame.expires_at_ms) || frame.expires_at_ms <= frame.sent_at_ms) {
    throw new Error("relay expires_at_ms 형식 오류");
  }
  if (
    typeof frame.payload_json !== "string" ||
    frame.payload_json.length === 0 ||
    new TextEncoder().encode(frame.payload_json).length > MAX_RELAY_PAYLOAD_JSON_BYTES
  ) {
    throw new Error("relay payload_json 형식 오류");
  }
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

function relayTicketHmacKeyBytes(secret) {
  let bytes;
  if (typeof secret === "string") {
    bytes = new TextEncoder().encode(secret);
  } else if (secret instanceof ArrayBuffer) {
    bytes = new Uint8Array(secret);
  } else if (ArrayBuffer.isView(secret)) {
    bytes = new Uint8Array(secret.buffer, secret.byteOffset, secret.byteLength);
  } else if (Array.isArray(secret)) {
    bytes = Uint8Array.from(secret);
  } else {
    throw new Error("relay ticket hmac key 형식 오류");
  }
  if (bytes.byteLength < MIN_RELAY_TICKET_HMAC_KEY_BYTES) {
    throw new Error("relay ticket hmac key too short");
  }
  return bytes;
}

function validCompanionIdentity(identity) {
  return (
    /^[A-Za-z0-9._:-]+$/.test(identity?.deviceId || "") &&
    /^[0-9a-f]{64}$/i.test(identity?.noisePubkeyHex || "") &&
    /^[0-9a-f]{64}$/i.test(identity?.approvalPubkeyHex || "")
  );
}

function validRelayTicketKeyId(value) {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{1,64}$/.test(value);
}

function validRelayWebSocketEndpointUrl(value) {
  try {
    const url = new URL(value);
    if (url.protocol === "wss:") {
      return true;
    }
    return (
      url.protocol === "ws:" &&
      (url.hostname === "127.0.0.1" || url.hostname === "localhost" || url.hostname === "[::1]")
    );
  } catch {
    return false;
  }
}

function validateRelayRouteEnvelopeMetadata(route) {
  if (route?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay route protocol_version");
  }
  if (!validRelaySessionId(route.session_id)) {
    throw new Error("relay route session_id 형식 오류");
  }
  if (!validRelaySender(route.sender)) {
    throw new Error("relay route sender 형식 오류");
  }
  if (!Number.isSafeInteger(route.sequence) || route.sequence <= 0) {
    throw new Error("relay route sequence 형식 오류");
  }
  if (!Number.isSafeInteger(route.sent_at_ms) || route.sent_at_ms <= 0) {
    throw new Error("relay route sent_at_ms 형식 오류");
  }
  if (!Number.isSafeInteger(route.expires_at_ms) || route.expires_at_ms <= route.sent_at_ms) {
    throw new Error("relay route expires_at_ms 형식 오류");
  }
  if (!Number.isSafeInteger(route.payload_json_bytes) || route.payload_json_bytes <= 0) {
    throw new Error("relay route payload_json_bytes 형식 오류");
  }
}

function validateRelayEndpointLoop(loop) {
  validateRelaySessionConnectMetadata(loop?.connect);
  validateRelayEndpoint(loop?.endpoint);
  if (loop.connect.session_id !== loop.endpoint.sessionId || loop.connect.peer !== loop.endpoint.sender) {
    throw new Error("relay endpoint loop connect mismatch");
  }
  for (const key of ["sentCount", "queuedCount", "receivedCount", "droppedCount", "errorCount"]) {
    if (!Number.isSafeInteger(loop[key]) || loop[key] < 0) {
      throw new Error(`relay endpoint loop ${key} 형식 오류`);
    }
  }
  if (typeof loop.connected !== "boolean") {
    throw new Error("relay endpoint loop connected 형식 오류");
  }
}

function parseRelayWebSocketEnvelope(socketMessage) {
  let message = socketMessage;
  if (typeof socketMessage === "string") {
    try {
      message = JSON.parse(socketMessage);
    } catch {
      throw new Error("relay websocket envelope JSON 파싱 실패");
    }
  }
  if (!message || typeof message !== "object" || Array.isArray(message)) {
    throw new Error("relay websocket envelope 형식 오류");
  }
  if (typeof message.kind !== "string" || message.kind.length === 0) {
    throw new Error("relay websocket envelope kind 형식 오류");
  }
  return message;
}

function rejectRelaySetupSecretFields(value, path = "$", depth = 0) {
  if (value === null || typeof value !== "object" || depth > 16) {
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => rejectRelaySetupSecretFields(item, `${path}[${index}]`, depth + 1));
    return;
  }
  for (const [key, nested] of Object.entries(value)) {
    const normalized = key.replaceAll("_", "").toLowerCase();
    if (normalized === "secret" || normalized === "hmacsha256keys") {
      throw new Error(`relay setup secret field not allowed: ${path}.${key}`);
    }
    rejectRelaySetupSecretFields(nested, `${path}.${key}`, depth + 1);
  }
}

function constantTimeHexEqual(left, right) {
  if (
    typeof left !== "string" ||
    typeof right !== "string" ||
    !/^[0-9a-f]+$/i.test(left) ||
    !/^[0-9a-f]+$/i.test(right) ||
    left.length !== right.length
  ) {
    return false;
  }
  let diff = 0;
  const a = left.toLowerCase();
  const b = right.toLowerCase();
  for (let i = 0; i < a.length; i += 1) {
    diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return diff === 0;
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

function renderRelayQueue(queue) {
  document.querySelector("#relay-pending-count").textContent = String(queue.length);
  const list = document.querySelector("#relay-approval-list");
  list.replaceChildren();
  if (queue.length === 0) {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "No relay approvals";
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

function relayBlockerText(code) {
  return (
    {
      transport_mode_not_relay: "transport mode is not relay",
      relay_endpoint_url_missing: "relay endpoint URL missing",
      relay_endpoint_url_invalid: "relay endpoint URL invalid",
      relay_deployment_mode_missing: "deployment mode missing",
      relay_deployment_mode_invalid: "deployment mode invalid",
      relay_deployment_mode_not_selected: "deployment mode is not self-hosted",
      relay_operator_setup_text_missing: "operator setup text missing",
      companion_identity_missing: "companion identity missing",
      relay_signed_ticket_missing: "signed relay ticket missing",
      relay_signed_ticket_invalid: "signed relay ticket invalid",
      relay_signed_ticket_expired: "signed relay ticket expired",
      relay_ticket_identity_mismatch: "ticket and companion identity mismatch",
      relay_now_ms_invalid: "local clock invalid",
    }[code] || code
  );
}

function renderRelayBlockers(blockers, emptyText) {
  const list = document.querySelector("#relay-blocker-list");
  list.replaceChildren();
  const items = blockers.length ? blockers.map(relayBlockerText) : [emptyText];
  for (const text of items) {
    const li = document.createElement("li");
    li.className = blockers.length ? "blocked" : "ready";
    li.textContent = text;
    list.append(li);
  }
}

function setRelayState(text, kind = "") {
  const el = document.querySelector("#relay-state");
  el.textContent = text;
  el.className = kind;
}

function setRelayConnectionState(text, kind = "") {
  const el = document.querySelector("#relay-connection-state");
  el.textContent = text;
  el.className = kind;
}

function setRelayLastEvent(text) {
  document.querySelector("#relay-last-event").textContent = text;
}

function renderRelayRuntime(monitor) {
  setRelayConnectionState(monitor.state, monitor.state === "Connected" ? "ok" : "");
  document.querySelector("#relay-pending-count").textContent = String(monitor.pendingCount);
  document.querySelector("#relay-received-count").textContent = String(monitor.receivedCount);
  document.querySelector("#relay-sent-count").textContent = String(monitor.sentCount);
  document.querySelector("#relay-approved-count").textContent = String(monitor.approvedCount);
  document.querySelector("#relay-rejected-count").textContent = String(monitor.rejectedCount);
}

function renderRelaySetup(setup = null, preflight = null) {
  const ticket = setup?.signedSessionTicket?.ticket || null;
  const keyId = setup?.signedSessionTicket?.key_id || "-";
  const blockers = preflight?.blockers || [];
  const ready = Boolean(setup && preflight?.status === "ready" && blockers.length === 0);

  setRelayState(setup ? (ready ? "Ready" : "Blocked") : "No setup", setup ? (ready ? "ok" : "error") : "");
  document.querySelector("#relay-default-mode").textContent = PWA_TRANSPORT_MODE_LIVE_LOOPBACK;
  document.querySelector("#relay-endpoint").textContent = setup?.relayEndpointUrl || "-";
  document.querySelector("#relay-deployment").textContent = setup?.deploymentMode || "-";
  document.querySelector("#relay-device").textContent = setup?.companionIdentity?.deviceId || "-";
  document.querySelector("#relay-session").textContent = ticket?.session_id || "-";
  document.querySelector("#relay-expires").textContent = formatExpiry(ticket?.expires_at_ms || 0);
  document.querySelector("#relay-ticket-key").textContent = keyId;
  document.querySelector("#relay-companion-connect").textContent = setup
    ? relaySessionConnectJson(setup.companionConnect)
    : "-";
  document.querySelector("#relay-daemon-connect").textContent = setup
    ? relaySessionConnectJson(setup.daemonConnect)
    : "-";
  renderRelayBlockers(blockers, setup ? "Relay setup ready" : "No relay setup loaded");
}

function renderRelaySetupError(message) {
  renderRelaySetup();
  setRelayState("Invalid", "error");
  renderRelayBlockers([message], "");
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
  const relaySetupInput = document.querySelector("#relay-setup-input");
  const relaySetupLoadButton = document.querySelector("#relay-setup-load-button");
  const relaySetupClearButton = document.querySelector("#relay-setup-clear-button");
  const relayConnectButton = document.querySelector("#relay-connect-button");
  const relayDisconnectButton = document.querySelector("#relay-disconnect-button");
  let activePayload = null;
  let activeApprovalRequest = null;
  let activeApprovalResponse = null;
  let activeApprovalTransport = "manual";
  let activeRelaySetup = null;
  let activeRelayLoop = null;
  let activeKeyMaterial = null;
  let liveBaseUrl = "";
  let liveEventSource = null;
  let liveApprovalQueue = [];
  let relaySocket = null;
  let relayApprovalQueue = [];
  let liveMonitor = liveMonitorInitialState();
  let relayMonitor = liveMonitorInitialState();
  renderMonitor(liveMonitor);
  renderRelaySetup();
  renderRelayQueue(relayApprovalQueue);
  renderRelayRuntime(relayMonitor);

  function updateMonitor(event) {
    liveMonitor = liveMonitorNext(liveMonitor, event);
    renderMonitor(liveMonitor);
  }

  function updateRelayMonitor(event) {
    relayMonitor = liveMonitorNext(relayMonitor, event);
    renderRelayRuntime(relayMonitor);
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

  function closeRelaySocket(stateText = "Disconnected") {
    if (relaySocket) {
      const socket = relaySocket;
      relaySocket = null;
      try {
        if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
          socket.close();
        }
      } catch {
        // Best-effort UI cleanup only.
      }
    }
    activeRelayLoop = null;
    relayConnectButton.disabled = !(activeRelaySetup && relayRuntimeSetupPreflight(activeRelaySetup).relayEnabled);
    relayDisconnectButton.disabled = true;
    setRelayConnectionState(stateText);
    updateRelayMonitor({ type: stateText === "Waiting" ? "waiting" : "disconnected", label: stateText });
  }

  function relaySetupMatchesIdentity(setup, identity) {
    return (
      setup?.companionIdentity?.deviceId === identity?.deviceId &&
      setup?.companionIdentity?.noisePubkeyHex === identity?.noisePubkeyHex &&
      setup?.companionIdentity?.approvalPubkeyHex === identity?.approvalPubkeyHex
    );
  }

  function handleLiveEventData(data) {
    const message = parseLiveTransportMessage(data);
    setLiveLastEvent(message.type);
    if (message.type === "approval_request") {
      liveApprovalQueue = liveApprovalQueueNext(liveApprovalQueue, message);
      activeApprovalRequest = message.request;
      activeApprovalResponse = null;
      activeApprovalTransport = "live";
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

  function handleRelayLiveMessage(message) {
    setRelayLastEvent(message.type);
    if (message.type === "approval_request") {
      relayApprovalQueue = liveApprovalQueueNext(relayApprovalQueue, message);
      activeApprovalRequest = message.request;
      activeApprovalResponse = null;
      activeApprovalTransport = "relay";
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      document.querySelector("#approval-response").textContent = "-";
      document.querySelector("#approval-verify-command").textContent = "-";
      renderApprovalRequest(activeApprovalRequest, "Relay");
      renderRelayQueue(relayApprovalQueue);
      updateRelayMonitor({
        type: "approval_request",
        label: `relay approval_request ${message.request.command_masked}`,
        pendingCount: relayApprovalQueue.length,
      });
      setStatus("Relay 승인 요청 수신됨", "ok");
      return;
    }
    if (message.type === "ping") {
      setRelayConnectionState("Connected", "ok");
      updateRelayMonitor({ type: "ping", label: `relay ping ${message.nonce}` });
    }
  }

  function handleRelaySocketMessage(data) {
    if (!activeRelayLoop) {
      throw new Error("relay endpoint loop 없음");
    }
    const result = relayEndpointLoopAcceptSocketMessage(activeRelayLoop, data);
    if (result.kind === "connected") {
      setRelayConnectionState("Connected", "ok");
      setRelayLastEvent("connected");
      relayConnectButton.disabled = true;
      relayDisconnectButton.disabled = false;
      updateRelayMonitor({
        type: "connected",
        label: "relay connected",
        endpoint: activeRelaySetup?.relayEndpointUrl || "",
        deviceId: activeRelaySetup?.companionIdentity?.deviceId || "",
      });
      setStatus("Relay companion 연결됨", "ok");
      return;
    }
    if (result.kind === "queued") {
      setRelayLastEvent("queued");
      return;
    }
    if (result.kind === "live_message") {
      handleRelayLiveMessage(result.liveMessage);
      return;
    }
    if (result.kind === "dropped") {
      setRelayLastEvent("dropped");
      updateRelayMonitor({ type: "error", label: "relay frame dropped" });
      return;
    }
    if (result.kind === "error") {
      throw new Error(result.message);
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

  async function connectRelay() {
    relayConnectButton.disabled = true;
    try {
      if (!activeRelaySetup) {
        activeRelaySetup = parseRelayRuntimeSetupInput(relaySetupInput.value);
        relaySetupInput.value = JSON.stringify(activeRelaySetup, null, 2);
      }
      const preflight = relayRuntimeSetupPreflight(activeRelaySetup);
      renderRelaySetup(activeRelaySetup, preflight);
      if (!preflight.relayEnabled) {
        throw new Error(`relay setup blocked: ${preflight.blockers.join(",")}`);
      }
      const identity = await loadActiveIdentityAndKeys();
      if (!relaySetupMatchesIdentity(activeRelaySetup, identity)) {
        throw new Error("relay setup companion identity mismatch");
      }
      activeRelayLoop = relayCompanionEndpointLoopFromSetup(activeRelaySetup);
      const socket = new WebSocket(activeRelayLoop.webSocketUrl);
      relaySocket = socket;
      setRelayConnectionState("Connecting");
      setRelayLastEvent("connecting");
      socket.addEventListener("open", () => {
        try {
          socket.send(relayEndpointLoopConnectJson(activeRelayLoop));
        } catch (err) {
          setStatus(err.message, "error");
          closeRelaySocket("Disconnected");
        }
      });
      socket.addEventListener("message", (event) => {
        try {
          handleRelaySocketMessage(event.data);
        } catch (err) {
          updateRelayMonitor({ type: "error", label: err.message });
          setStatus(err.message, "error");
        }
      });
      socket.addEventListener("error", () => {
        updateRelayMonitor({ type: "error", label: "relay websocket error" });
        setStatus("Relay websocket 오류", "error");
      });
      socket.addEventListener("close", () => {
        if (relaySocket === socket) {
          relaySocket = null;
          activeRelayLoop = null;
          relayConnectButton.disabled = !(activeRelaySetup && relayRuntimeSetupPreflight(activeRelaySetup).relayEnabled);
          relayDisconnectButton.disabled = true;
          setRelayConnectionState("Disconnected");
          setRelayLastEvent("closed");
        }
      });
    } catch (err) {
      closeRelaySocket("Disconnected");
      updateRelayMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    } finally {
      relayConnectButton.disabled = Boolean(relaySocket);
    }
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

  function loadRelaySetup() {
    try {
      closeRelaySocket("Disconnected");
      relayApprovalQueue = [];
      relayMonitor = liveMonitorInitialState();
      activeRelaySetup = parseRelayRuntimeSetupInput(relaySetupInput.value);
      relaySetupInput.value = JSON.stringify(activeRelaySetup, null, 2);
      const preflight = relayRuntimeSetupPreflight(activeRelaySetup);
      renderRelaySetup(activeRelaySetup, preflight);
      renderRelayQueue(relayApprovalQueue);
      renderRelayRuntime(relayMonitor);
      relayConnectButton.disabled = !preflight.relayEnabled;
      setStatus(preflight.relayEnabled ? "Relay setup 확인됨" : "Relay setup blocked", preflight.relayEnabled ? "ok" : "error");
    } catch (err) {
      activeRelaySetup = null;
      relayConnectButton.disabled = true;
      renderRelaySetupError(err.message);
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
  relaySetupLoadButton.addEventListener("click", loadRelaySetup);
  relaySetupClearButton.addEventListener("click", () => {
    closeRelaySocket("Disconnected");
    relaySetupInput.value = "";
    activeRelaySetup = null;
    relayApprovalQueue = [];
    relayMonitor = liveMonitorInitialState();
    renderRelaySetup();
    renderRelayQueue(relayApprovalQueue);
    renderRelayRuntime(relayMonitor);
    setStatus("Relay setup 대기");
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
      activeApprovalTransport = "manual";
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
      if (
        activeApprovalTransport === "relay" &&
        relaySocket?.readyState === WebSocket.OPEN &&
        activeRelayLoop?.connected
      ) {
        const out = relayEndpointLoopNextFrame(activeRelayLoop, liveApprovalResponseMessage(response));
        relaySocket.send(out.frameJson);
        const sentKey = liveApprovalRequestKey(activeApprovalRequest);
        relayApprovalQueue = relayApprovalQueue.filter((item) => item.key !== sentKey);
        renderRelayQueue(relayApprovalQueue);
        setRelayLastEvent("approval_response");
        updateRelayMonitor({
          type: "approval_response",
          label: approve ? "relay approval_response approve" : "relay approval_response reject",
          approve,
          pendingCount: relayApprovalQueue.length,
        });
        setStatus(approve ? "Relay 승인 응답 전송됨" : "Relay 거부 응답 전송됨", "ok");
      } else if (activeApprovalTransport === "live" && liveBaseUrl) {
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
      updateRelayMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    }
  }
  liveConnectButton.addEventListener("click", connectLive);
  liveDisconnectButton.addEventListener("click", () => {
    closeLiveEvents("Disconnected");
    setStatus("Live companion 연결 해제됨");
  });
  relayConnectButton.addEventListener("click", connectRelay);
  relayDisconnectButton.addEventListener("click", () => {
    closeRelaySocket("Disconnected");
    setStatus("Relay companion 연결 해제됨");
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
            : mode === "relay"
              ? document.querySelector(".relay-section")
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
