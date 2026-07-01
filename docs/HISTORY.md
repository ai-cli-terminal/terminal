# HISTORY — 변경 / 결정 로그

> **정본**: 설계 결정의 권위 기록은 `../document/`(특히 `00-overview-architecture.md` §0.2 불일치 해소, `03_프로젝트_아키텍처_정의서.md` ADR, `05-roadmap-enhancements-decisions.md` §30 결정안).
> 본 문서는 **구현 repo(`terminal/`)의 변경·결정 타임라인**이다. 최신 항목이 위로 온다.

---

## 2026-07-01 — Handoff cleanup, release checks, RA listener reliability

- **RA/PWA transport mode decision**: Added `docs/superpowers/plans/2026-07-01-ra-pwa-transport-mode-decision.md` and fixed the product default as `live-loopback`. `ai remote daemon` now prints `PWA transport mode : live-loopback`; the browser/operator evidence smoke asserts that mode and records `transportMode` in its evidence JSON. Native `device.sock` remains an internal/test substrate and future fallback candidate, not a user-facing transport flag.
- **Repo hygiene**: Removed a mistakenly created trailing-space directory under the repo root (`terminal\ `) that contained only an `artifacts/fdroid-dry-run/tools` directory skeleton and caused `git status` to warn with `could not open directory ' /'`.
- **Install docs refresh**: Updated `README.md` and `docs/INSTALL.md` from stale v0.3.0/v0.2.4 examples to the v0.3.3 release state. The docs now distinguish Windows GUI assets (`ai-terminal-windows-*.zip`, `AI.Terminal_*_x64-setup.exe`) from CLI/runtime assets (`ai-windows-x86_64.exe`, `ash-windows-x86_64.exe`) and keep pinned install examples on `v0.3.3`.
- **Release follow-up checks**: Confirmed the public `v0.3.2` GitHub Release note already carries a superseded-by-v0.3.3 banner. The public `v0.3.3` release body is currently empty, so adding human-readable release notes remains a content follow-up rather than a tag/asset fix.
- **Windows MSI preflight rerun**: Re-ran `scripts/smoke-msi-preflight.ps1`; it still records `MSI_PREFLIGHT_BLOCKED` at `artifacts/msi-preflight/msi-preflight-evidence.json` because this host lacks Windows-native Rust/Cargo, MSVC `cl`/`link`/`rc`, and WiX/WiX Toolset.
- **Android/F-Droid preflight rerun**: Re-ran `:app:verifyFdroidReleaseInputs`, `android/smoke-fdroid-metadata.ps1`, `android/smoke-github-signing-secrets.ps1 -UseThrowawayKeystore`, and `android/smoke-fdroid-release-activation.ps1 -Commit 792e0eef5280788bb45c1cab0731a0443eac1a30`; all passed. Evidence lives under `artifacts/fdroid-dry-run/`, `artifacts/android-github-signing-preflight/`, and `artifacts/fdroid-activation-smoke/`. `gh secret list` returned no repository secrets, so real Android release signing secrets still need registration/verification.
- **RA listener reliability fix**: Changed the queue-backed `device.sock` listener to use per-request response channels plus accept timeouts. This prevents a timed-out approval request from leaving a stale blocked accept/read path that can poison the next request. Regression coverage: `device_listener_timeout_does_not_poison_next_request`.
- **RA/PWA next-work plan + multi-device floor**: Added `docs/superpowers/plans/2026-07-01-ra-pwa-live-companion-next.md` as the live next-work document for RA/PWA companion completion. Implemented the first floor slices: `DeviceRegistry::select_device`, `ai remote daemon --device-id <id>` for explicit approval target selection, and `ai remote devices` for listing registered public device records. One-device behavior remains default-compatible; zero or multiple devices without explicit selection fail closed with an actionable message.
- **RA/PWA live transport contract slice**: Added `docs/superpowers/plans/2026-07-01-ra-pwa-live-transport-contract.md` and implemented the shared Rust/PWA JSON envelope for future live companion traffic: `hello`, `approval_request`, `approval_response`, `ping`, `pong`, and `error`. Rust helpers in `session.rs` and PWA helpers in `pwa/app.mjs` now validate the same protocol version, device id, public key, approval payload, heartbeat, and malformed-message boundaries. Follow-up slices added the local browser endpoint, backend approval bridge, and PWA live UX; the remaining gap is end-to-end evidence.
- **RA/PWA live loopback endpoint slice**: Added `docs/superpowers/plans/2026-07-01-ra-pwa-live-loopback-endpoint.md` and implemented `daemon::spawn_companion_live_endpoint`, a dependency-free `127.0.0.1:<ephemeral>` HTTP/SSE endpoint for browser companions. `ai remote daemon` now prints the live base URL plus `/message` and `/events` endpoints. `GET /health` returns endpoint metadata, `GET /events` emits a typed SSE `ping`, and `POST /message` accepts shared `CompanionTransportMsg` JSON. `ping` returns `pong`; `hello` is checked against the selected/registered device id and public keys; malformed JSON, unknown routes, rejected hello, and incomplete request bodies return typed `error` envelopes with fail-closed HTTP status codes. PWA helpers now normalize live endpoint URLs and POST typed messages with `fetch`.
- **RA/PWA live approval bridge slice**: Added `docs/superpowers/plans/2026-07-01-ra-pwa-live-approval-bridge.md`. The live endpoint now owns the `DeviceListenerHandle` used by `serve_with_remote`, so `ai remote daemon` routes eligible High opt-in gate approvals through the browser loopback path by default. A valid `hello` marks the selected device connected, `GET /events` emits pending `approval_request` messages, and `POST /message` with a matching `approval_response` wakes the original gate waiter before the existing `finish_remote_gate_response` validation/nonce/replay path. Mismatched responses fail closed with HTTP 409 and leave the pending request intact. The native `device.sock` substrate remains in code/tests for fallback or future transport-mode decisions, but it is no longer the default daemon path after this slice.
- **RA/PWA PWA live UX slice**: Added `docs/superpowers/plans/2026-07-01-ra-pwa-live-pwa-ux.md` and connected the static PWA to the live daemon endpoint. The page now accepts the printed live endpoint URL, sends `hello` with the stored/generated companion identity, opens `EventSource` on `/events`, renders incoming `approval_request` messages as the active pending approval, and sends signed approve/reject decisions back through `postLiveTransportMessage`. The manual paste/sign/copy path remains available when no live endpoint is connected.
- **RA/PWA live evidence harness slice**: Added `docs/superpowers/plans/2026-07-01-ra-pwa-live-e2e-evidence.md` and `scripts/smoke-pwa-live-approval.ps1`. The smoke records `artifacts/ra-pwa-live-evidence/ra-pwa-live-evidence.json`, runs `node pwa/app.test.mjs`, verifies the static PWA live endpoint controls/selectors, and runs targeted WSL Rust tests for `companion_live` endpoint/bridge behavior. This is repeatable P4a evidence; full browser/operator daemon evidence remains P4b.
- **RA/PWA validation refresh**: `node pwa/app.test.mjs`, `cargo test --features remote companion_transport`, `cargo fmt --all -- --check`, full `cargo test --features remote`, `cargo clippy --features remote -- -D warnings`, and `git diff --check` passed. Rustfmt still emits the existing stable-channel warnings for unstable config keys.
- **RA/PWA endpoint validation refresh**: `node pwa/app.test.mjs`, targeted `cargo test --features remote companion_live_endpoint`, `cargo fmt --all -- --check`, full `cargo test --features remote`, and `cargo clippy --features remote -- -D warnings` passed.
- **RA/PWA bridge validation refresh**: `node pwa/app.test.mjs`, targeted `cargo test --features remote companion_live_bridge`, targeted `cargo test --features remote companion_live_endpoint`, `cargo fmt --all -- --check`, `cargo clippy --features remote -- -D warnings`, and full `cargo test --features remote` passed.
- **RA/PWA PWA UX validation refresh**: `node pwa/app.test.mjs` passed with coverage for live endpoint URL normalization, EventSource URL selection, live approval queue dedupe, and live response POST helpers.
- **RA/PWA P4a evidence refresh**: `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-approval.ps1` passed with `RA_PWA_LIVE_EVIDENCE_OK`; evidence file: `artifacts/ra-pwa-live-evidence/ra-pwa-live-evidence.json`.
- **RA/PWA P4b documentation and preflight**: Added `docs/TROUBLESHOOTING.md`,
  `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`,
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-p4b-browser-evidence.md`, and
  `scripts/smoke-pwa-live-browser-preflight.ps1`. The preflight wrote
  `RA_PWA_LIVE_BROWSER_PREFLIGHT_BLOCKED` at
  `artifacts/ra-pwa-live-browser-preflight/ra-pwa-live-browser-preflight.json`.
  Required files, Node, `node pwa/app.test.mjs`, PWA live browser surface, WSL Rust toolchain,
  and the P4a live harness all passed; the remaining blocker is browser evidence capture
  availability (`playwright` package missing and no Edge/Chrome/Chromium command on `PATH`).
- **RA/PWA P4b browser capture unblock**: Added
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-browser-capture-unblock.md`, taught
  `scripts/smoke-pwa-live-browser-preflight.ps1` to find Chrome/Edge in common install paths,
  added root Node dev tooling (`package.json`/`package-lock.json`) with Playwright, and ignored
  root `node_modules/`. Re-running the preflight now records
  `RA_PWA_LIVE_BROWSER_PREFLIGHT_READY` at
  `artifacts/ra-pwa-live-browser-preflight/ra-pwa-live-browser-preflight.json`.
  The next remaining P4b step is the actual daemon + browser/PWA approve/reject evidence run.
- **RA/PWA P4b browser/operator evidence**: Added
  `scripts/smoke-pwa-live-browser-evidence.mjs` and npm script
  `smoke:pwa-live-browser-evidence`. The smoke builds the remote `ai` binary in WSL, serves the
  static PWA locally, drives installed Chrome through Playwright, creates a disposable PWA identity,
  completes `ai remote pair` in isolated `XDG_CONFIG_HOME`/`XDG_DATA_HOME`, starts
  `ai remote daemon --device-id <id>`, connects the PWA to the printed live endpoint, arms High
  approval, and captures both approve and reject gate decisions. Result:
  `RA_PWA_LIVE_BROWSER_EVIDENCE_OK`; evidence:
  `artifacts/ra-pwa-live-browser-evidence/ra-pwa-live-browser-evidence.json`; screenshots and
  transcript are stored in the same artifact directory. Approve returned exit `0`; reject returned
  exit `1` with daemon rejection text.
- **RA/PWA monitoring view**: Added
  `docs/superpowers/plans/2026-07-01-ra-pwa-monitoring-view.md` and enabled the PWA `Monitor`
  tab. The PWA now tracks live connection state, endpoint, device id, pending approvals,
  request/response counts, approve/reject counts, heartbeat/response timestamps, and recent live
  event history through a deterministic monitor reducer covered by `node pwa/app.test.mjs`.
  `npm run smoke:pwa-live-browser-evidence` now asserts monitor counters during the existing
  approve/reject browser smoke and records a `monitor` snapshot in
  `artifacts/ra-pwa-live-browser-evidence/ra-pwa-live-browser-evidence.json`
  (`received=2`, `sent=2`, `approved=1`, `rejected=1`).

## 2026-06-30 — Windows follow-up, review cleanup, RA listener/device registry

- **Windows MSI preflight**: Added `scripts/smoke-msi-preflight.ps1`. On this host it records `MSI_PREFLIGHT_BLOCKED` at `artifacts/msi-preflight/msi-preflight-evidence.json` because Windows-native MSI prerequisites are missing: `cargo`, `rustc`, MSVC `cl`/`link`/`rc`, and WiX/WiX Toolset. Node/NPM are present. No MSI build was attempted.
- **Explorer/ShellOpen GUI evidence**: Extended `scripts/smoke-gui.ps1` with `-LaunchMode ShellOpen` and optional `-OpenExplorerSelection`. Because ShellOpen cannot inject per-process smoke environment variables, this mode is limited to launch/window/`ash.exe` child/resize/cleanup evidence. Run result: `GUI_SMOKE_OK artifacts/explorer-shell-open-smoke/gui-shell-open-evidence.json`; screenshots were written beside it. This is Windows Shell open-verb evidence with Explorer selection, not a human-recorded literal double-click video.
- **Review cleanup**: Moved redirect parsing helpers from `pipeline.rs` into shared `cmdparse.rs`, leaving preview/pipeline on the same command parsing primitive. `command_executed` audit payloads now include masked `command`, `source`, and `exit`, matching the non-Ran audit payload shape. `SemanticCache`/`ResponseCache` capacity bounds were already present.
- **RA-1 substrate start**: Added `session::run_daemon_listener_once` and `session::run_device_connect` for Unix sockets. New `approval_roundtrip_over_unix_listener` proves a real `UnixListener` path can host `run_daemon_request` and complete Noise handshake + signed approval validation. This is RA-1 substrate only; daemon process wiring, pairing, persistent device records, and gate-flow approval are still next.
- **RA-1 daemon-owned listener step**: Added `daemon::device_socket_path` and `daemon::serve_device_once`, so the daemon module now owns the `device.sock` endpoint and hosts `session::run_daemon_request` for one real device connection. New `daemon::tests::device_listener_once_roundtrip` covers bind → device connect → Noise handshake → signed approval validation. This still stops before persistent daemon task wiring, pairing, registered device lookup, and gate-flow approval.
- **RA-1 repeated listener skeleton**: Added `daemon::serve_device_loop`, a daemon-owned repeat accept loop that takes a pending approval request supplier and response handler. New `daemon::tests::device_listener_loop_handles_multiple_connections` proves two sequential device connections can be served over the same `device.sock`, producing Approved then Rejected signed responses. The supplier/handler are the seam where RA-3 gate-flow pending approvals and wakeups will attach.
- **RA-1 daemon process wiring**: Added `daemon::spawn_device_listener`, a queue-backed background `device.sock` listener. `ai remote daemon` now starts this listener when built with `--features remote`; the listener binds at startup and waits for future gate-flow requests. New `daemon::tests::spawned_device_listener_handles_queued_request` proves queued approval request → device connect → response channel works end-to-end.
- **RA-2 device registry start**: Added `device_registry.rs` behind the `remote` feature. It persists registered devices at `config_dir()/remote-devices.json`, stores both Noise static pubkey and Ed25519 approval pubkey, rejects duplicate id/Noise key/approval key, exposes a single-device helper for unambiguous early flows, and validates signed approval responses through `approval::validate` using the registered device record. This is the storage/verification substrate only; `ai remote pair` CLI/QR, daemon key persistence, and gate-flow response handling are still next.
- **RA-2 pairing CLI/key persistence + PWA payload**: Added `pairing.rs` behind the `remote` feature. `remote-daemon-key.json` now persists the daemon Noise static key; `ai remote daemon` reuses that key instead of generating a fresh ephemeral key on every start. `ai remote pair` starts a single pending pairing with a 6-digit code, daemon public key hex, TTL, and `remote-pairing.json`; it now also prints versioned `pair_payload_json` and `aiterminal://pair?payload=...` for QR/PWA handoff. Supplying `--device-id`, `--code`, `--noise-pubkey-hex`, and `--approval-pubkey-hex` completes pairing by registering the device and clearing the pending pairing. Concurrent unexpired pairing is rejected. QR rendering/PWA transport UX and gate-flow pending approval handling are still next.
- **RA-3 gate-flow orchestration/runtime wiring**: Added `daemon::plan_remote_gate`, `finish_remote_gate_response`, `remote_timeout_reply`, and `decide_with_remote_listener`. Armed Low/Medium stays local allow, Critical stays local block, High without opt-in stays opt-in block, and High with opt-in now produces a registered-device approval plan instead of immediate allow. The response path consumes nonce, validates the signed response against `remote-devices.json`, maps Approved to allow, Rejected/invalid/replay/timeout to fail-closed block, and has a real queue-backed `device.sock` listener roundtrip test. `serve_with_remote`/`DaemonRuntime` now wire this into `handle_conn`; `ai remote daemon` loads the persisted registry and sends eligible High opt-in gate requests through the device listener.
- **RA-4 context hash/request-origin wiring**: Added `context::RemoteContextSnapshot`, `RemoteContextOrigin`, `remote_context_snapshot`, and `remote_context_hash`. The hash covers canonical cwd, shell/user/host/git branch, allowlisted env with hash-only PATH, and command target paths resolved via realpath where possible. `ai __gate` now sends filtered shell-origin cwd/env in the gate IPC request; the remote daemon uses that origin when issuing High opt-in approval requests and recomputes the hash after the device response before `approval::validate`, so cwd/env/target drift is fail-closed through the existing ContextDrift path. Legacy gate JSON without context remains accepted. Remaining gaps are QR/PWA pairing UX, real companion approval UI, and broader multi-device selection.
- **RA-5 PWA pair/approval surface start**: Added a static `pwa/` companion shell with manifest, service worker, icon asset, responsive pair screen, payload URL/manual JSON parsing, payload validation, WebCrypto X25519/Ed25519 identity generation, local identity restore, and `ai remote pair --device-id ...` complete-command generation. The PWA now stores non-extractable private CryptoKeys in IndexedDB and only restores an identity when matching private key material is present. It can also parse an `ApprovalRequestMsg` JSON or `aiterminal://approve?approval=...` / `?approval=...` URL payload, render masked command/context details, generate signed approve/reject `ApprovalResponseMsg` JSON with the same Ed25519 signing preimage as Rust (`approval_id || nonce || decision`), copy the signed response, and generate a registry-based `ai remote approval-verify --device-id ...` command for manual terminal handoff. `ai remote pair` now renders a terminal QR for the `aiterminal://pair?...` payload URL, and `--pwa-url <url>` emits `pwa_pair_url` plus a PWA-opening QR that appends the same payload to the supplied companion URL. `ai remote approval-url --request-json ...` now emits canonical `approval_request_json`, `aiterminal://approve?...`, terminal QR, and optional `pwa_approval_url`/`pwa_approval_qr` for companion handoff. `ai remote approval-verify --request-json ... --response-json ... --device-id ...` rebuilds the pending approval from the request JSON and verifies the PWA response against `remote-devices.json` through Rust `approval::validate`; `--approval-pubkey-hex ...` remains for direct smoke/debug verification. Browser Noise transport and phone end-to-end evidence remain next.
- **검증**: `cargo fmt --all -- --check` green (rustfmt emitted existing stable-channel warnings for unstable config keys), targeted tests green: `cargo test cmdparse`, `cargo test backup_targets_picks_up_redirect_overwrite`, `cargo test ran_payload_matches_audit_shape_and_masks_command`, `cargo test --features remote approval_roundtrip_over_unix_listener`, `cargo test --features remote approval_request_url_and_qr_are_pwa_ready`, `cargo test --features remote pwa_response_json_validates_after_request_rebuild`, `cargo test --features remote device_listener_once_roundtrip`, `cargo test --features remote device_listener_loop_handles_multiple_connections`, `cargo test --features remote spawned_device_listener_handles_queued_request`, `cargo test --features remote device_registry`, `cargo test --features remote pairing`, `cargo test --features remote cli_parses_remote_pair_start_and_complete`, `cargo test --features remote cli_parses_remote_approval_url`, `cargo test --features remote cli_parses_remote_approval_verify`, `cargo test --features remote cli_parses_remote_approval_verify_device_id`, `cargo test --features remote remote_gate`, `cargo test --features remote remote_context`, `cargo test --features remote gate_request_accepts_legacy_json_without_context`. CLI smoke with isolated `XDG_CONFIG_HOME` confirmed `pair_payload_json`, `pair_url`, `pair_qr`, `pwa_pair_url`, and `pwa_pair_qr` output; `ai remote approval-url --request-json ... --pwa-url http://127.0.0.1:8787/index.html` smoke confirmed `approval_url`, `approval_qr`, `pwa_approval_url`, and `pwa_approval_qr` output; WebCrypto-generated response smoke confirmed direct `ai remote approval-verify ... --approval-pubkey-hex ...` prints `approval_outcome : Approved`; isolated registry smoke confirmed `ai remote approval-verify ... --device-id phone-1` loads `remote-devices.json` and prints `device_id : phone-1` plus `approval_outcome : Approved`. PWA companion parser/crypto smoke passed with `node pwa/app.test.mjs`, including non-extractable private keys, Ed25519 sign/verify, X25519 shared-secret derivation, approval request URL parsing/validation, signed approve/reject response generation, and verify-command generation. Full remote gate also passed: `cargo clippy --features remote -- -D warnings` and `cargo test --features remote` (360 lib tests + CLI/integration/e2e/version/doc tests). Static HTTP serving was smoke-checked for `index.html`, `app.mjs`, `manifest.webmanifest`, `sw.js`, `icon.svg`, and `styles.css`. PowerShell parser check passed for `scripts/smoke-gui.ps1` and `scripts/smoke-msi-preflight.ps1`.

## 2026-06-29 — v0.3.3 release asset smoke

- **Release asset verification**: Downloaded the public `v0.3.3` Windows GUI assets from GitHub Release into `artifacts/release-v0.3.3-smoke` and verified the uploaded `.sha256` files. Portable zip SHA256 is `d46fdbf9a5e3557d40ea7d46184212b42513575e518cf886cbba720d93e70f24`; NSIS installer SHA256 is `89054f320280eb87336b769b4f621adc33ce798819f78ea2b1f92439b0b987cc`.
- **Portable GUI smoke**: Extracted `ai-terminal-windows-x86_64-pc-windows-msvc.zip` and ran `scripts/smoke-gui.ps1` against the extracted package. Result: `GUI_SMOKE_OK`. Evidence: `artifacts/release-v0.3.3-smoke/zip-extracted/ai-terminal-windows-x86_64-pc-windows-msvc/gui-smoke-evidence.json`. The smoke covered package checksums, visible `ai-terminal.exe`, bundled `ash.exe` child, no external terminal descendant, command transcript, resize, Ctrl-C recovery, Ctrl-D exit, frontend selection/copy/paste/scrollback, and GUI-internal AI routing/safety/storage-audit.
- **NSIS smoke refresh**: The first installer smoke exposed stale local harness behavior: `scripts/smoke-nsis.ps1` still required `WebView2Loader.dll`, but the MSVC/Tauri release installer works without a separate loader DLL. Updated the script to treat `WebView2Loader.dll` as optional and record optional missing files in evidence.
- **NSIS release smoke**: Re-ran `scripts/smoke-nsis.ps1` against the downloaded `AI.Terminal_0.3.3_x64-setup.exe`. Result: `NSIS_SMOKE_OK`. Evidence: `artifacts/nsis-install-smoke/nsis-smoke-evidence.json`. Silent install and uninstall both exited `0`; installed GUI smoke passed; installed files were `ai-terminal.exe`, `ash.exe`, `ai.exe`, and `uninstall.exe`; optional missing file was `WebView2Loader.dll`.
- **Handoff refresh**: Rewrote `docs/HANDOFF.md` around the actual v0.3.3 release state, replacing stale v0.3.0/v0.3.1 guidance and recording the real release asset smoke paths and next work.

## 2026-06-28 — Windows NSIS smoke refresh

- **MSI 재확인**: WSL cross-host Tauri bundle은 NSIS artifact를 생성하지만 MSI는 다시 `ignoring msi`로 처리된다. Windows-native Tauri build는 `cargo metadata ... program not found`에서 멈춘다. 현재 Windows PATH에는 Node만 있고 Rust/Cargo, MSVC `cl/link/rc`, WiX가 없다. MSI는 Windows-native Rust+MSVC+WiX packaging host에서 재검토해야 한다.
- **NSIS smoke 재확인**: `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-nsis.ps1` green. 최신 installer SHA256은 `e206a1ebc3c2a59b87a2a317455472ee5b0eff45e53c0f79b5d9a176c7fe289e`, evidence는 `artifacts/nsis-install-smoke/nsis-smoke-evidence.json`.
- **Smoke harness 수정**: Windows SQLite `ai-terminal.db-shm`이 range lock 상태일 때 binary evidence scan이 실패하지 않도록 `scripts/smoke-gui.ps1`가 잠긴 파일 read/open `IOException`을 skip 처리한다. DB 본체와 WAL evidence scan은 계속 수행한다.
- **GUI visual smoke evidence**: portable GUI smoke를 재실행하고 캡처 이미지를 직접 확인했다. `docs/superpowers/plans/2026-06-28-windows-gui-visual-smoke-evidence.md`에 `AI Terminal` 창, `ash` prompt/output, resize, Ctrl-C recovery, Ctrl-D exit, frontend UX, AI/gate/storage/audit evidence를 정리했다. 엄밀한 Explorer double-click gesture evidence는 별도 수동 단계로 남긴다.

## 2026-06-28 — Android shared staging picker decision

- **F-Droid reproducibility prep**: Android release versioning now derives from root `VERSION`; `versionCode` is computed as `major * 10000 + minor * 100 + patch` (`0.3.1` → `301`). The F-Droid/direct APK candidate stays a universal full-ABI APK. Added `:app:verifyFdroidReleaseInputs` to verify metadata, repository license files, and the matching changelog before packaging, plus `android/fastlane/metadata/android/en-US/changelogs/301.txt`; release workflow now runs this gate. Added `LICENSE-MIT` and `LICENSE-APACHE` to match `Cargo.toml`'s `MIT OR Apache-2.0`. Latest unsigned APK hash is refreshed by the Android release gate; details: `docs/superpowers/specs/2026-06-28-android-fdroid-reproducibility-prep.md`.
- **Android signing smoke**: `android/smoke-release-signing.ps1` validates the env-based Gradle signing path with a throwaway local keystore under `artifacts/android-signing-smoke`, then runs `apksigner verify` on `app-release.apk`. Latest smoke produced a signed APK of 8,950,386 bytes, SHA256 `245844e4cc684c24868158be6edbb8443a7e9b310054f668cf2274dfa0da492f`; `apksigner` reports v2 verification and 1 signer. This proves signing wiring without committing or exposing real release secrets.
- **F-Droid screenshots**: Captured Fastlane phone screenshots from the `Medium_Phone` emulator after installing and launching the debug APK: `android/fastlane/metadata/android/en-US/images/phoneScreenshots/01-home.png` and `02-run-result.png`. The second capture includes the default shellcore command result (`# size` / `200`) verified through UI hierarchy. `:app:verifyFdroidReleaseInputs` now requires at least two non-empty phone screenshot PNGs.
- **F-Droid submission dry-run prep**: Added fdroiddata-style draft metadata at `android/fdroiddata/metadata/dev.aiterminal.android.yml` plus `android/fdroid-version.properties`. The version mirror lets F-Droid regex-based update checks read `versionName=0.3.1` / `versionCode=301` without running Gradle or recomputing semver logic. The draft build block is intentionally disabled until a release tag/commit is selected for fdroiddata submission. `:app:verifyFdroidReleaseInputs` checks the mirror and draft metadata content. `android/smoke-fdroid-metadata.ps1` now reproduces the local fdroidserver 2.4.5 dry-run; it passed `fdroid lint dev.aiterminal.android` and `fdroid rewritemeta dev.aiterminal.android` with no source diff. Only `apksigner not found` remains as a dry-run environment warning. Details: `docs/superpowers/specs/2026-06-28-android-fdroid-submission-dry-run.md`.
- **F-Droid activation preflight**: Added `android/smoke-fdroid-release-activation.ps1`. Given a full release commit hash, it dry-runs the final fdroiddata activation step by removing the temporary `disable`, replacing `TODO_NEXT_ANDROID_RELEASE_COMMIT`, running `fdroid rewritemeta`, and linting the canonical activated metadata copy. Source metadata is unchanged unless `-Apply` is passed. The script passed with local `HEAD` as a stand-in release commit.
- **GitHub Android signing secrets preflight**: Added `android/smoke-github-signing-secrets.ps1` to validate the exact base64 keystore path used by `.github/workflows/release.yml`. `-UseThrowawayKeystore` creates a local throwaway keystore, converts it to secret-shaped base64, decodes it back, runs Gradle release signing, and verifies the APK with `apksigner --print-certs` without writing passwords/base64 to evidence. Throwaway APK hash and certificate fingerprint are run-specific and are written to `artifacts/android-github-signing-preflight/android-github-signing-preflight-evidence.json`.
- **구현**: Android Termux shared staging UX는 path input을 유지하고 `OpenDocumentTree` 기반 `Pick` 버튼을 보조로 추가했다. Picker가 `com.android.externalstorage.documents`의 `primary:` tree URI를 반환하면 `/sdcard/...` 경로로 매핑해 기존 app-write validation + helper smoke gate에 태운다.
- **경계**: SAF URI 자체를 Termux helper execution backend로 쓰지 않는다. T1 helper는 shared filesystem directory를 job root로 요구하므로, non-primary volume이나 traversal-like tree path는 거부하고 manual path input을 계속 허용한다.
- **Imported file UX**: `Open Last` action을 추가해 가장 최근 import된 workspace 파일을 read-only로 다시 열고 transcript에 더 큰 bounded UTF-8 preview를 표시한다. Reopen은 canonical path가 workspace root 아래인지 재검증하고 binary/non-UTF-8 content를 거부한다.
- **Android 배포 경로 결정**: Android는 direct APK/GitHub Release → F-Droid 준비를 우선한다. Termux-enabled build의 Google Play 제출은 Device and Network Abuse / runtime command execution 정책 검토 후로 보류하며, 필요하면 core-only/reduced bridge flavor로 분리한다. 결정 기록은 `docs/superpowers/specs/2026-06-28-android-distribution-route.md`.
- **Release packaging probe**: WSL용 `android/build-rust-jni.sh`를 추가했다. NDK r28c Linux prebuilt를 user-local에 풀고 `bash android/build-rust-jni.sh --profile release`로 4개 ABI `libai_terminal.so`를 staging한 뒤, `.\gradlew.bat :app:assembleRelease :app:verifyNativeLibraries` green. 산출물은 unsigned `android/app/build/outputs/apk/release/app-release-unsigned.apk`(8,942,198 bytes, SHA256 `9082168B795B319CFDA3DD8AD565576674E8B48942BA76814EE540765C25DA48`). Release signing은 `AI_TERMINAL_ANDROID_KEYSTORE*` env vars가 있을 때만 켜지도록 Gradle에 연결했다.
- **Release workflow APK asset**: tag-triggered `.github/workflows/release.yml`에 Android job을 추가했다. Ubuntu에서 NDK/JNI release build, Android JVM test, `assembleRelease`, `verifyNativeLibraries`를 실행하고 signed `ai-terminal-android-universal.apk` 또는 unsigned `ai-terminal-android-universal-unsigned.apk`와 SHA256을 GitHub Release에 업로드한다. Signing은 `AI_TERMINAL_ANDROID_KEYSTORE_BASE64` 등 GitHub secrets가 모두 있을 때만 활성화된다.
- **Android toolchain pin 정리**: CI/release Android job은 시스템 Gradle 대신 checked-in `android/gradlew`를 사용하고, Android API 35 / build-tools 35.0.0 / NDK r28c(`28.2.13676358`)로 고정했다.
- **Android release metadata**: F-Droid/fastlane 스타일 metadata 초안을 `android/fastlane/metadata/android/en-US`에 추가했다. 설명 문구는 app-private shellcore, document import/export, opt-in Termux bridge로 제한하고 full Linux terminal로 과장하지 않는다.
- **테스트**: `TerminalViewModelTermuxTest`가 primary URI 매핑, non-primary storage 거부, traversal 거부를 고정한다. `WorkspaceDocumentsTest`가 read-only reopen, workspace escape 거부, binary 거부를 고정한다. 로컬 `ANDROID_HOME=$env:LOCALAPPDATA\Android\Sdk; ANDROID_SDK_ROOT=$ANDROID_HOME; .\gradlew.bat :app:testDebugUnitTest` green.

## 2026-06-28 — Windows GUI packaging verification refresh

- **Repository gate 재확인**: `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"`, `cargo test`, `cargo check --lib --target aarch64-linux-android` 모두 green.
- **Desktop/Tauri gate 재확인**: `desktop`에서 `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-gnu`, `npm run tauri -- build --target x86_64-pc-windows-gnu --no-bundle --ci --config src-tauri/tauri.windows.conf.example.json` 통과.
- **Portable/NSIS smoke 재확인**: portable package 재생성 후 `scripts/smoke-gui.ps1` green. NSIS installer 재생성 후 `scripts/smoke-nsis.ps1` green. 최신 installer SHA256은 `c90253b6b2f08a097114094ef84240c304ca8fe6c5aa73e7e1b9e194d1d776bd`, evidence는 `artifacts/nsis-install-smoke/nsis-smoke-evidence.json`.
- **GUI input/output evidence 보강**: `scripts/smoke-gui.ps1`가 창 크기 안정화 후 대상 `ai-terminal.exe` 윈도우를 PNG로 캡처하고, smoke command `print AI_TERMINAL_GUI_SMOKE_OK`를 앱 내부 Tauri→PTY write 경로로 주입한 뒤 transcript에서 `AI_TERMINAL_GUI_SMOKE_OK` 출력과 `error:` 부재를 검증한다. portable/installed screenshot에서 `AI Terminal` 창 내부 prompt/input/output/`running` 상태바를 확인했다. `scripts/smoke-nsis.ps1`는 screenshot/transcript를 설치 디렉터리 밖 `artifacts/nsis-install-smoke/`에 저장하고, installer 직후 1.5초 settle 대기를 둬 첫 실행 close flake를 제거했다.
- **GUI resize evidence 보강**: `scripts/smoke-gui.ps1`가 smoke command 통과 후 `SetWindowPos`로 실제 `ai-terminal.exe` 창 크기를 변경하고, resize 전/목표/후 bounds와 `gui-smoke-resize-screenshot.png`를 evidence에 남긴다. portable/installed resize screenshots에서 입력/출력과 `running` 상태가 유지됨을 확인했다.
- **GUI Ctrl-D evidence 보강**: `AI_TERMINAL_GUI_SMOKE_CTRL_D_DELAY_MS` smoke hook을 추가해 command/resize 통과 후 GUI 세션에 종료 입력을 주입하고, `ash.exe` child가 종료되는지 검증한다. `gui-smoke-ctrl-d-screenshot.png` / `installed-gui-smoke-ctrl-d-screenshot.png`와 `ctrlD.ashExited=true`가 evidence에 기록된다.
- **GUI Ctrl-C evidence 보강**: `AI_TERMINAL_GUI_SMOKE_CTRL_C_DELAY_MS` smoke hook을 추가해 GUI 세션에 pending 입력을 주입한 뒤 Ctrl-C로 취소하고, 같은 `ash.exe` child가 살아 있는 상태에서 복구 명령 `print AI_TERMINAL_GUI_SMOKE_CTRL_C_OK`가 실행되는지 검증한다. `gui-smoke-ctrl-c-screenshot.png` / `installed-gui-smoke-ctrl-c-screenshot.png`와 `ctrlC.recovered=true`, `ctrlC.ashStillRunning=true`가 evidence에 기록된다.
- **GUI frontend UX evidence 보강**: `AI_TERMINAL_GUI_SMOKE_FRONTEND_EVIDENCE` smoke hook을 추가해 xterm selection/copy/paste/scrollback을 앱 내부에서 검증한다. Frontend smoke는 marker text 선택, copy event clipboard round-trip, paste event를 통한 `print AI_TERMINAL_GUI_SMOKE_PASTE_OK` 실행, 120라인 scrollback retention/scroll position 변화를 확인하고 `gui-smoke-frontend-evidence.json` / `installed-gui-smoke-frontend-evidence.json` 및 screenshot을 evidence에 기록한다.
- **GUI ash integration evidence 보강**: `AI_TERMINAL_GUI_SMOKE_ASH_INTEGRATION_*` smoke hook을 추가해 GUI PTY 내부에서 mock AI routing(`ai AI_TERMINAL_GUI_SMOKE_AI_ROUTE`), Critical safety gate 차단(`rm -rf /`), 외부 명령 실행(`cmd.exe /c echo AI_TERMINAL_GUI_SMOKE_EXTERNAL_OK`)을 순차 검증한다. Smoke는 격리 `XDG_CONFIG_HOME`/`XDG_DATA_HOME`에 mock provider config와 SQLite DB를 만들고, transcript에서 `aiRouted=true`/`safetyGateBlocked=true`/`externalCommandRan=true`, DB 파일에서 `usagePersisted=true`/`commandHistoryPersisted=true`/`auditBlockedPersisted=true`를 확인해 `gui-smoke-ash-integration-evidence.json` / `installed-gui-smoke-ash-integration-evidence.json`에 기록한다.

## 2026-06-27 — Windows GUI terminal pivot

- **방향 정정**: Windows 완료 기준을 `ash.exe`를 Windows Terminal/PowerShell/Git Bash에서 수동 검증하는 모델에서, 자체 창을 가진 독립 GUI 터미널 `ai-terminal.exe`로 전환했다.
- **새 정본**: `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`. 기존 `ash.exe` S1~S7은 폐기하지 않고 GUI 앱 내부 PTY/ConPTY child runtime으로 재사용한다.
- **완료 기준 변경**: Windows 완료는 `ai-terminal.exe` 더블클릭 후 외부 터미널 창 없이 앱 내부에서 `ash` prompt/input/output/resize/Ctrl-C/Ctrl-D/AI/safety gate/storage/cleanup이 통과해야 한다.
- **기존 수동 검증 문서의 지위**: `docs/superpowers/plans/2026-06-27-windows-ash-manual-verification.md`는 GUI MVP 완료 조건이 아니라 `ash` runtime regression evidence로만 유지한다.
- **G1 scaffold**: `desktop/` Tauri v2 + xterm.js 앱을 추가했다. 첫 화면은 terminal surface이며, Rust backend가 `portable-pty`로 `ash.exe`/`ash`를 child runtime으로 띄우고 xterm `onData`/resize/event bridge와 연결한다. Windows packaging과 실제 GUI smoke는 후속이다.
- **G2 hardening/package prep**: 종료 후 Restart UX, `terminal_kill_all` child cleanup, sidecar path policy, user-local WSL MinGW/WebKitGTK toolchain, Windows GNU `ai-terminal.exe` release build, `ash.exe`/`ai.exe` sidecar staging, portable package + SHA256 manifest를 추가했다. `scripts/smoke-gui.ps1`는 Windows에서 portable package checksum, GUI main window, `ash.exe` child spawn, unexpected external shell descendant absence, close cleanup evidence를 기록한다. 실제 Windows GUI 수동 smoke와 installer packaging은 후속이다.
- **Windows GUI automated smoke pass**: Tauri ACL capability/permission 파일을 추가해 `event.listen` 및 custom terminal commands(`terminal_open/write/resize/kill/kill_all`)를 허용했다. 이후 Windows host에서 portable package smoke가 green: main window 생성, `ash.exe` child spawn, 예상 외부 shell descendant 없음, close 후 child cleanup, checksum evidence 기록. 이후 입출력/resize/Ctrl-C/Ctrl-D와 installer smoke evidence는 2026-06-28 항목에서 보강됐다.
- **NSIS installer artifact**: WSL cross-host에서 Tauri NSIS bundling을 통과시켰다. `desktop/scripts/setup-wsl-nsis.sh`가 `nsis`/`proot`/`libtalloc2`를 user-local apt extraction으로 준비하고, `makensis` wrapper가 NSIS share를 `/usr/share/nsis`로 bind해 Tauri bundler를 통과시킨다. 산출물은 `AI Terminal_0.1.0_x64-setup.exe`와 `.sha256`이며, MSI는 Linux cross-host에서 ignored 처리됐다. installer 실행/설치/제거 smoke는 후속이다.
- **NSIS installer smoke pass**: `scripts/smoke-nsis.ps1`를 추가해 silent install을 repo-local `artifacts\nsis-install-smoke\AITerminal`에 수행하고, 설치 파일(`ai-terminal.exe`, `ash.exe`, `ai.exe`, `WebView2Loader.dll`, `uninstall.exe`) 존재 확인, installed GUI smoke(`smoke-gui.ps1 -SkipChecksums`), silent uninstall, 설치 디렉터리/shortcut/uninstall registry cleanup을 자동 검증했다. Evidence는 `artifacts/nsis-install-smoke/nsis-smoke-evidence.json`.

## 2026-06-27 — Windows ash manual verification partial pass, Windows-only checks blocked

- **부분 검증 완료**: Windows native 환경은 아니지만 Linux `ash` 실행 경로에서 격리 config/data(`XDG_CONFIG_HOME`, `XDG_DATA_HOME`)로 config fail-soft, mock AI routing, Ollama 미실행 fail-soft, OpenAI no-key fail-soft, Critical gate 차단, High-risk 비대화형 확인 거부, storage `usage_events`/`commands`/`audit_events` 기록을 확인했다.
- **저장소 evidence**: `usage_events`에 mock AI 1건, `commands`에 `source="ash"` 실행 기록, `audit_events`에 `command_executed`/`command_blocked`/`command_declined`가 기록됐다. High-risk `rm -rf /tmp/...` 거부 후 대상 파일이 남아 있음을 확인했다.
- **Repository gate**: `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"`, `cargo test`, `cargo check --lib --target aarch64-linux-android` 모두 green.
- **블로커**: 현재 실행 환경에 `powershell.exe`/`cmd.exe`/`wsl.exe`와 Windows `ash.exe`가 없고, 제공 PTY가 reedline cursor-position query(`ESC[6n`)에 응답하지 않아 TTY line editor 검증을 완료할 수 없었다. 따라서 PM-1 `Windows 완료 검증`은 아직 `[x]`로 올리지 않는다.

## 2026-06-27 — PR #26 AI usage and Windows ash CI green

- **PR**: #26 `[codex] record AI usage from ask, dispatch, and ash` is open as draft, merge-clean, and green on all required CI checks as of 2026-06-27 07:17 UTC: `fmt · clippy · test`, `cargo audit (supply chain)`, `android JNI packaging`, `windows build + self-contained check`.
- **구현**: `ai ask`, `ai dispatch`, and ash AI routing now share the AI usage recording helper so provider/model metadata, token estimates, cache/local zero-cost behavior, OpenAI estimated cost, and ash budget snapshot handling are recorded consistently.
- **상태**: automated implementation verification is complete for PR #26. The remaining blocker before Windows completion is real Windows/TTY manual verification: reedline editing, history recall/persistence/filtering, config fail-soft, natural-language AI routing, Ollama fail-soft/response behavior, safety gate/audit, and Git Bash/MSYS `AI_TERMINAL_WINDOWS_PROFILE=msys` execution.

## 2026-06-26 — Android imported document preview

- **구현**: Android `Import`가 선택한 document를 app-private workspace로 복사한 뒤 UTF-8 텍스트 preview를 transcript에 바로 남긴다. Preview는 4KiB/80라인 상한에서 잘림 marker를 표시하고, NUL byte나 UTF-8 decode 실패가 있는 binary-like content는 preview를 건너뛴다.
- **경계**: 여전히 user-selected document tree를 mount하지 않는다. Import는 명시 복사 경로이고, 파일 열람 UX의 다음 후보는 read-only builtin 또는 structured table reader다.
- **테스트**: `WorkspaceDocumentsTest`가 텍스트 preview, line truncation, binary skip을 고정한다. 로컬 `gradle -p android :app:testDebugUnitTest`와 `git diff --check` 통과.

## 2026-06-26 — Termux T1 helper bootstrap and capability gate

- **구현**: `TermuxHelperBootstrapContract`를 추가해 Termux `RUN_COMMAND`로 `~/.ash-termux-bridge/helper.sh`를 설치하고 `self-test` marker를 돌려받는 bootstrap script를 생성한다. helper는 `request.json`을 읽고 child process stdout/stderr를 `events.ndjson`에 쓰며, `cancel` marker를 감지하면 process group에 interrupt/terminate/kill 순서로 중단을 시도한다.
- **구현**: Compose UI에 `Install Helper` action을 추가했다. `Probe Termux`는 T0 final-result smoke만 통과시키고 external adapter를 켜지 않는다. `Install Helper`의 self-test가 `ASH_TERMUX_HELPER_OK`를 반환해도, shared staging smoke가 아직 없으면 `ShellWorker.externalCommandsEnabled`는 계속 꺼진다.
- **구현**: user-selected shared staging path 입력과 `Verify` action을 추가했다. 앱이 해당 path에 쓸 수 있고 helper event-file smoke가 `ASH_SHARED_STAGING_OK`를 반환한 뒤에만 `ShellWorker`에 T1 adapter를 동적으로 연결하고 `externalCommandsEnabled=true`로 전환한다.
- **구현**: helper가 `python3` 없이도 동작하도록 shell fallback을 추가했다. 앱은 `request.json`과 함께 `argv/0000...` fallback files를 쓰고, helper는 shared storage에서 FIFO 대신 regular stdout/stderr log polling + batched `awk` NDJSON 변환으로 stream/cancel을 처리한다.
- **수정**: 제품 factory에서 app external-files 기반 T1 adapter 연결을 제거했다. T1 adapter는 SAF 또는 user-selected shared staging smoke가 통과하기 전까지 제품 경로에 연결하지 않는다.
- **경계**: 앱은 Termux storage permission이나 package install을 대신하지 않는다. Real-device smoke에서 app external-files bridge root는 Termux가 `Permission denied`로 job dir을 만들 수 없어 shared staging 설계 변경이 필요함을 확인했다.
- **실기기 검증**: `SM_F956N / R3CX60P3R5K`에서 Termux storage permission grant 후 `/sdcard/Download/ash-termux-bridge` shared staging으로 helper bootstrap, long-running stdout, stderr/non-zero, large output, cancel smoke가 통과했다. 중간 실패로 확인한 내용: Termux storage permission 없이는 `/sdcard` write가 실패하고, `events.ndjson`를 앱이 먼저 생성하지 않으면 앱 read가 EACCES로 실패하며, shared storage는 FIFO를 지원하지 않아 log polling fallback이 필요하다.
- **테스트**: `TermuxHelperBootstrapContractTest`가 install script/self-test/helper job 계약 문자열을 고정하고, `TerminalViewModelTermuxTest`가 T0/helper bootstrap만으로 external command가 켜지지 않는 gating과 shared staging smoke 성공/실패 게이트를 검증한다. `TermuxHelperRealDeviceSmokeTest`는 manual instrumentation smoke로 추가했고 기본 CI 실행에서는 `termuxRealDeviceSmoke=true`와 `termuxBridgeStagingDir=<shared-dir>` 없이는 skip된다.

## 2026-06-26 — Termux T1 helper-backed stream/cancel adapter

- **구현**: `TermuxHelperBridgeAdapter`를 추가해 external execution disabled 결과를 helper-backed T1 경로로 재시도할 수 있게 했다. 앱은 shared bridge root 아래 job directory를 만들고 `request.json`을 argv 기반으로 쓴 뒤, Termux `RUN_COMMAND`로 `~/.ash-termux-bridge/helper.sh run <job-dir>`를 시작한다.
- **구현**: adapter는 `events.ndjson`를 scheduled polling으로 읽어 `Stdout`/`Stderr`/`Finished`/`Cancelled`를 UI stream에 전달하고, helper가 event를 쓰기 전에 실패하면 PendingIntent result를 error `Finished`로 변환한다. helper `started` event는 worker가 이미 보낸 `Started`와 중복되지 않도록 숨긴다.
- **구현**: `ShellWorker`에 opt-in external stream adapter fallback을 붙였다. Shared staging smoke가 성공한 뒤에만 external fallback을 켜야 하며, 단일 argv command만 T1로 보낸다. `|`, `;`, redirection 같은 shell operator는 shell string 합성을 피하기 위해 거부한다.
- **구현**: Compose 입력줄에 실행 중 `Cancel` 버튼을 추가하고, ViewModel이 active `ShellRunHandle`을 추적해 T1 cancel marker를 쓸 수 있게 했다.
- **경계**: SAF directory picker, real-device long-running/cancel/large-output smoke는 아직 남아 있다. 현재 Android 기본 경계는 계속 app-private workspace이며, T1 bridge root는 제품 경로에 자동 연결하지 않는다.
- **테스트**: `TermuxHelperBridgeAdapterTest`가 argv parser, request file 생성, helper start, event streaming, launch failure fallback, cancel marker/launcher cancel을 검증한다. `ShellWorkerTest`가 opt-in fallback routing과 disabled 상태 보존을 검증한다. 로컬 `gradle -p android :app:testDebugUnitTest`와 `gradle -p android :app:assembleDebug` 통과.

## 2026-06-26 — Termux T1 helper protocol substrate

- **구현**: `TermuxHelperProtocol`을 추가해 T1 helper request를 argv 배열 기반 JSON으로 만들고, helper `events.ndjson`의 `started`/`stdout`/`stderr`/`finished`/`cancelled` line을 `ShellStreamEvent`로 변환하는 순수 Kotlin 계약을 고정했다.
- **구현**: `TermuxHelperEventFilePoller`를 추가해 `events.ndjson`를 offset 기반으로 polling하고, partial line은 newline까지 보류하며, `Finished`/`Cancelled` terminal event 이후 추가 output을 무시한다. 파일 truncate/rotation 시 offset을 reset한다.
- **구현**: `FileBackedShellRunHandle.cancel()`은 shared job directory의 `cancel` file에 user cancel marker를 쓰는 idempotent handle이다.
- **경계**: 아직 실제 helper 설치, SAF/shared staging workspace, scheduled polling integration은 붙이지 않았다. 이번 slice는 T1 adapter가 UI stream/cancel 계약에 맞게 들어올 수 있는 file-level substrate다.
- **테스트**: `TermuxHelperProtocolTest`가 request JSON, empty argv rejection, stream event mapping, non-zero exit final result를 검증한다. `TermuxHelperJobFilesTest`가 stable helper filenames, missing file, incremental line polling, partial line buffering, terminal stop, truncate reset, cancel file write를 검증한다. JVM unit test에서 Android `org.json` mock stub 대신 실제 JSON 구현을 쓰도록 test-only `org.json:json` dependency를 추가했다.

## 2026-06-26 — Termux T0 real-device smoke

- **구현**: `Probe Termux`를 echo 단일 probe에서 T0 smoke suite로 확장했다. `echo`, `pwd`, stderr capture, non-zero exit code를 순차 실행하고 transcript에 각 케이스 결과를 남긴다.
- **실기기 검증**: `SM_F956N / R3CX60P3R5K`에서 Termux v0.118.3, `allow-external-apps=true`, `com.termux.permission.RUN_COMMAND` grant 상태로 `termux echo`, `pwd`, `stderr`, `non-zero` 모두 통과했다. 당시 상태바는 `external / opt-in`, `Termux T0 smoke ready`로 전환됐지만, 이후 T0/helper self-test만으로는 external adapter를 켜지 않도록 capability gate를 강화했다.
- **수정**: Termux v0.118.3 result bundle key를 `"result"`로 맞추고, Android O+에서 `RunCommandService` 시작 시 `startForegroundService`를 사용하도록 분기했다.
- **테스트**: `gradle -p android :app:testDebugUnitTest`와 `gradle -p android :app:assembleDebug` 통과.

## 2026-06-25 — Termux T0 `RUN_COMMAND` probe substrate

- **구현**: Android manifest에 Termux package visibility와 `com.termux.permission.RUN_COMMAND`를 추가하고, `AndroidTermuxBridge`가 설치/권한 상태를 판정하게 했다. `Probe Termux` UI는 T0 echo probe를 Termux `RUN_COMMAND` service로 보내고, 앱의 result service가 PendingIntent 결과를 받아 transcript와 capability 상태를 갱신한다.
- **경계**: 이 구현은 T0 completion-result probe다. 실제 incremental stream/cancel은 아직 T1 helper 범위이며, app-private workspace를 Termux에 직접 노출하지 않는다.
- **테스트**: `TermuxBridgeTest`가 availability 판정, Termux result bundle decoding, non-zero exit conversion을 고정한다. 로컬 `gradle -p android :app:testDebugUnitTest`와 `gradle -p android :app:assembleDebug` 통과.

## 2026-06-25 — PM-3F Termux-compatible opt-in bridge design

- **결정**: Termux-compatible bridge는 Android MVP 기본값이 아니라 explicit opt-in adapter다. Android 기본 약속은 계속 `shellcore-only`다.
- **설계**: bridge를 T0 `RUN_COMMAND` completion probe와 T1 `ash-termux-helper` protocol로 나눴다. T0는 설치/권한/setup과 final stdout/stderr/exit만 검증하고, 실제 incremental stream/cancel/workspace staging은 T1 helper가 job id, NDJSON event log, cancel token을 관리할 때만 ready로 표시한다.
- **문서**: `docs/superpowers/specs/2026-06-25-termux-compatible-opt-in-bridge-design.md`를 추가하고 PM workflow, TASK, Android README, README가 다음 작업을 T0 probe와 T1 helper 구현으로 가리키게 했다.

## 2026-06-24 — PM-3E Android external command strategy

- **결정**: Android MVP는 계속 `shellcore-only`로 유지한다. 다음 구현 후보는 Termux-compatible opt-in bridge이며, Termux/user-installed runtime을 같은 process의 PATH처럼 직접 실행하거나 mount하지 않는다.
- **보류**: bundled minimal userland는 4 ABI 패키징, CVE update, 라이선스 고지, binary provenance, 배포 크기와 정책 리뷰 책임 때문에 지금 구현하지 않는다.
- **문서**: `docs/superpowers/specs/2026-06-24-android-external-command-strategy.md`를 추가하고 PM workflow, TASK, HANDOFF, Android README가 다음 작업을 Termux-compatible bridge design spike로 가리키게 했다.

## 2026-06-24 — Android stream/cancel contract

- **계약**: `ShellStreamEvent`(`Started`, `Stdout`, `Stderr`, `Finished`, `Cancelled`)와 `ShellRunHandle`을 추가했다. 기존 complete-result `NativeShellBridge.evalLine`은 `ShellWorker.submitStreaming`에서 stream event sequence로 어댑트된다.
- **취소 의미**: 현재 shellcore-only path는 JNI 호출 자체를 강제 중단하지 않고, completion 전에 cancel되면 final result를 UI에 적용하지 않으며 `Cancelled`를 post한다. future PTY/userland adapter는 같은 계약 위에서 실제 interrupt/timeout을 구현해야 한다.
- **문서/테스트**: `docs/superpowers/specs/2026-06-24-android-stream-cancel-contract.md`를 추가했고, JVM tests로 `Started -> Stdout -> Finished`와 `Started -> Cancelled` event ordering을 고정했다.

## 2026-06-24 — Android document picker import/export

- **구현**: Compose 화면에 `Import`/`Export` 액션을 추가하고, Android Storage Access Framework의 `OpenDocument`/`CreateDocument` launcher로 연결했다. Import는 선택한 document stream을 app-private `ash-workspace` 안으로 복사하고, Export는 transcript를 사용자가 선택한 URI에 UTF-8 텍스트로 쓴다.
- **경계**: user-selected document tree를 직접 mount하지 않는다. 파일명은 basename + allowlist 문자로 sanitize하고, 복사 destination이 canonical app-private workspace root 밖으로 나가면 거부한다. 따라서 workspace root 밖 파일은 명시 import/export 복사 경로로만 다룬다.
- **테스트**: `WorkspaceDocumentsTest`로 filename sanitization을 고정했다. 로컬 `gradle -p android :app:testDebugUnitTest :app:assembleDebug` 통과.

## 2026-06-24 — NativeShellBridge instrumentation smoke 추가

- **구현**: `androidTest` runner/dependencies를 추가하고 `NativeShellBridgeInstrumentedTest`를 작성했다. 테스트는 실제 APK 환경에서 `System.loadLibrary("ai_terminal")`가 성공해야 통과하며, `NativeShellBridge`가 Rust `MobileShell`로 구조화 pipeline을 평가하고 session state를 다음 호출에 보존하는지 검증한다.
- **CI**: Android JNI packaging job에 x86_64 emulator step을 추가해 `gradle -p android :app:connectedDebugAndroidTest`를 실행한다. 이 job은 먼저 4 ABI `.so`를 생성하므로 emulator APK에는 `x86_64/libai_terminal.so`가 포함된다.
- **로컬 검증**: 이 Windows 세션에는 Rust Android `.so`와 emulator 실행 환경이 없어 connected test는 실행하지 못했다. 대신 `gradle -p android :app:assembleDebugAndroidTest`로 instrumentation APK 컴파일을 확인했다.

## 2026-06-24 — ShellWorker JVM behavior test 고정

- **구현**: `ShellWorker`의 result posting 경계를 `ResultPoster` 인터페이스로 분리했다. production 기본값은 `Handler(Looper.getMainLooper())`를 유지하고, JVM test에서는 fake poster를 주입해 callback 실행 시점을 직접 제어한다.
- **테스트**: `ShellWorkerTest`를 추가해 `ShellBridge.evalLine`이 single worker executor thread에서 실행되고, result callback은 poster를 통해 나중에 실행되는 계약을 검증한다. bridge exception은 `ShellEvalResult(ok=false)`로 변환되고 기존 `ShellState`를 유지하는 것도 고정했다.
- **CI**: Android JNI packaging job에 `gradle -p android :app:testDebugUnitTest`를 추가해 worker 계약을 매 PR에서 확인한다.

## 2026-06-24 — Android Rust JNI 전체 ABI/CI 패키징 경로

- **구현**: `android/build-rust-jni.ps1`의 기본 target을 `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android` 전체로 확장했다. 스크립트는 Windows/Linux/macOS NDK host tag를 감지하고, rustup이 있으면 필요한 Android Rust target을 자동 추가한다.
- **패키징 검증**: Android Gradle에 `:app:verifyNativeLibraries` task를 추가해 `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64` 각각의 `libai_terminal.so` 존재를 확인한다. 일반 `assembleDebug`는 native library가 없어도 개발용 fallback 오류 UX를 유지한다.
- **CI**: GitHub Actions에 `android JNI packaging` job을 추가했다. Ubuntu runner에서 Android SDK/NDK를 설치하고, Rust JNI `.so` 4개 ABI를 빌드한 뒤 Gradle verify task와 debug APK assemble을 실행한다.

## 2026-06-24 — PM-3D Android workspace boundary와 cwd 표시 연결

- **구현**: Android 앱 시작 시 `Context.filesDir/ash-workspace`를 app-private workspace root로 생성하고, `ShellState.cwd`/`workspaceRoot`를 이 경로로 초기화한다. JNI state JSON도 `workspace_root`를 포함해 Rust `MobileShell` state와 왕복한다.
- **안전 경계**: `shellcore::Engine`에 optional workspace root를 추가하고, `cd`/`ls`가 root 밖 경로를 거부하게 했다. 따라서 Android `shellcore-only` 단계에서도 외부 process spawn뿐 아니라 filesystem builtin의 host 파일 접근 범위가 app-private workspace 안으로 제한된다.
- **UI/문서**: Android 상태바는 좁은 화면에 맞춰 전체 경로 대신 workspace/cwd basename과 `core / private` capability만 보여준다. `docs/TASK.md`, PM workflow, Android README는 PM-3D 완료 범위와 남은 document picker/import-export 구현을 반영했다.

## 2026-06-23 — PM-3 Android JNI bridge로 `MobileShell` 연결

- **구현**: Rust library crate를 `cdylib`로도 빌드하게 하고, `src/mobile_jni.rs`에 JNI export `NativeShellBridge.nativeEvalLine(input, stateJson)`을 추가했다. Kotlin `NativeShellBridge`는 JSON state를 넘기고 `MobileEvalResult` JSON을 받아 `ShellEvalResult`로 복원한다.
- **제거**: `FakeShellBridge`를 삭제하고 `TerminalViewModel` factory가 실제 native bridge를 사용하게 바꿨다. native `.so`가 아직 패키징되지 않은 개발 환경에서는 첫 명령 제출 시 transcript에 로드 오류를 표시한다.
- **도구화**: `android/build-rust-jni.ps1`을 추가해 NDK linker로 Android Rust target을 빌드하고 `app/src/main/jniLibs/<abi>/libai_terminal.so`로 복사하는 경로를 문서화했다. 전체 ABI/CI 자동화는 후속 작업으로 남긴다.

## 2026-06-23 — PM-3 Android Compose UI + worker thread spike

- **구현**: `android/`에 최소 Kotlin/Compose skeleton을 추가했다. 화면은 session status, transcript, command input, run button으로 구성된다. `TerminalViewModel`이 UI state를 소유하고, `ShellWorker`가 single-thread executor에서 `ShellBridge.evalLine`을 호출한 뒤 main thread로 결과를 post한다.
- **경계**: `ShellBridge`는 후속 JNI/UniFFI binding이 구현할 인터페이스다. 이번 slice는 `FakeShellBridge`로 UI/worker 흐름만 검증 가능한 형태를 만들었고, 실제 Rust `MobileShell` 연결은 다음 slice로 남겼다.
- **결정**: shellcore-only 단계는 별도 Android process가 아니라 thread worker로 시작한다. long-running native execution, PTY, userland가 붙는 시점에 process 분리를 재평가한다.

## 2026-06-23 — PM-3 Android local terminal core boundary 착수

- **배경**: Android 목표는 승인 PWA가 아니라 온디바이스 로컬 터미널이다. 다만 첫 slice에서 완전 Linux 터미널을 약속하면 userland·권한·worker·패키징 검증 전 제품 약속이 과해진다.
- **결정**: 앱 shell 기본값은 Kotlin/Compose + Rust core binding이다. 외부 명령 전략은 첫 단계에서 `shellcore-only`를 채택하고, Termux 호환·bundled minimal userland는 후속 spike로 비교한다.
- **구현**: `src/mobile.rs`에 `MobileShell` pure core boundary를 추가했다. `MobileShell`은 `Engine::pure()`를 사용해 process spawn을 차단하고, `eval_line` 결과를 `output_json`/`output_text`/`error`/updated `state`로 반환한다. panic은 FFI 경계를 넘지 않도록 문자열 오류로 격리한다.
- **문서**: `docs/superpowers/specs/2026-06-23-android-local-terminal-spike.md`를 추가하고 TASK/workflow/README를 PM-3 진입 상태로 갱신했다.

## 2026-06-23 — PM-2 Git Bash/MSYS profile 계약 정의

- **배경**: Git Bash/MSYS에서 Windows native `ash.exe`를 실행할 수 있지만, MSYS path conversion과 POSIX userland를 Windows native PATH/PATHEXT adapter와 암묵적으로 섞으면 명령 탐색·quoting·PTY 의미가 불명확해진다.
- **결정**: 기본 profile은 항상 `native`다. MSYS bridge는 `AI_TERMINAL_WINDOWS_PROFILE=msys`로 명시 opt-in할 때만 선택하며, `MSYSTEM` 또는 `MSYSTEM_PREFIX`가 있는 환경에서만 유효하다.
- **구현**: `shellcore::msys` 순수 profile selection 계약과 테스트를 추가했다. 문서는 path conversion과 POSIX tool discovery가 bridge profile에서만 수행된다고 명시한다. 실제 MSYS bridge runner와 smoke는 후속 구현 대상이다.

## 2026-06-23 — PM-2B Windows ConPTY smoke와 설치 문서 분리

- **배경**: Windows native `ash.exe`가 외부 실행 adapter뿐 아니라 실제 terminal transport(ConPTY) 위에서도 interactive program과 왕복할 수 있어야 했다. 또한 Windows native와 WSL 설치 경로를 한 문단에 섞으면 사용자가 서로 다른 런타임을 혼동할 수 있었다.
- **구현**: Windows 전용 `pty` 테스트가 `cmd.exe`를 `portable-pty` ConPTY 세션으로 띄우고 `CONPTY_OK` marker round-trip을 확인한다. Windows CI와 `scripts/smoke.ps1`은 이 테스트를 실행한다. `guardrails::Platform::Windows`와 `Windows ConPTY` capability를 추가해 `ai doctor --guardrails`가 Windows를 명시적으로 표시한다.
- **문서**: `docs/INSTALL.md`를 추가해 Linux/WSL과 Windows native 설치·검증 경로를 분리했다. README, TASK, PM workflow, 플랫폼 실행 계약도 PM-2B 완료 상태에 맞춰 갱신했다.

## 2026-06-23 — PM-2A Windows `ash.exe` exit code smoke

- **배경**: Windows adapter가 `.cmd/.bat`와 `.ps1`를 별도 host(`cmd.exe`, PowerShell)로 실행하므로, 성공 출력뿐 아니라 non-zero exit code가 `ash` 사용자에게 정확히 보이는지 확인해야 했다.
- **구현**: Windows CI와 `scripts/smoke.ps1`에 `ash-fail.cmd`(`exit /b 7`)와 `ash-fail.ps1`(`exit 9`) smoke를 추가했다. `ash` REPL은 외부 명령 실패 후 계속 유지하되 `[command: exit N]` 형식으로 종료 코드를 출력하므로, smoke는 이 표시를 검증한다.
- **검증**: PowerShell regex parse와 `git diff --check`, WSL Rust gate에서 확인한다. 실제 Windows runner 검증은 main CI의 `windows build + self-contained check`에서 수행한다.

## 2026-06-23 — PM-2C `ash` 릴리즈 asset 동시 배포

- **결정**: `ai`와 `ash`는 같은 release 안에 별도 바이너리 asset으로 둔다. 패키지 하나로 묶지 않아도 checksum 검증과 수동 다운로드가 단순하고, 기존 `ai` 설치 경로와 독립 `ash` 제품화를 동시에 유지할 수 있다.
- **구현**: Release workflow가 Linux/Windows에서 `--bins`로 빌드하고 `ai-*`, `ash-*`, 각 `.sha256`을 업로드한다. `install.sh`/`install.ps1`은 새 릴리즈에서 `ai`와 `ash`를 함께 설치하고, 이전 릴리즈처럼 `ash` asset이 없으면 경고 후 `ai`만 설치한다.
- **검증**: PowerShell smoke regex 자체 검증과 `git diff --check` 통과. 릴리즈 검증은 v0.2.4 tag workflow에서 Linux/Windows asset 업로드로 확인한다.

## 2026-06-23 — PM-2C Windows `ash.exe` smoke 추가

- **구현**: Windows CI build를 `--bins`로 바꿔 `ai.exe`와 `ash.exe`를 함께 빌드하고, `ash.exe` smoke를 추가했다. 로컬 `scripts/smoke.ps1`도 같은 범위로 확장했다.
- **검증 범위**: 구조화 셸 baseline(`[ {size: 50} {size: 200} ] | where size > 100`)이 `size 200`만 남기는지 확인하고, 임시 디렉터리의 `.cmd`와 `.ps1` 스크립트를 `ash`에서 실행해 Windows adapter 경로가 실제 runner에서 살아있는지 확인한다.
- **남은 일**: release artifact에 `ash.exe`를 포함할지 결정해야 한다. ConPTY interactive smoke는 PM-2B에서 별도 진행한다.

## 2026-06-23 — PM-2A Windows `ash.exe` 실행 해석 1차

- **배경**: Windows native `ash.exe`는 PowerShell 문법 호환 셸이 아니라 `ash` 문법 위에서 Windows 실행 대상을 호출하는 로컬 터미널이어야 한다. 따라서 `.exe`, `.cmd/.bat`, `.ps1`을 같은 문자열 처리로 섞으면 quoting·exit code·사용자 기대가 깨진다.
- **구현**: `shellcore::winexec` 순수 모듈을 추가해 PATH/PATHEXT 탐색과 invocation kind를 분리했다. `.exe/.com/기타`는 direct spawn, `.cmd/.bat`는 `cmd.exe /d /c`, `.ps1`는 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File`로 분류한다. Windows 빌드의 `DesktopRunner`는 이 해석과 spawn plan을 사용하고, Linux/WSL은 기존 direct spawn 경로를 유지한다.
- **검증**: Linux/WSL에서도 돌아가는 순수 단위 테스트로 PATHEXT 정규화, cwd 우선순위, PATH 탐색 순서, 상대 경로 + PATHEXT, `.ps1`/`.cmd` 분류를 고정했다. Spawn-plan 테스트로 공백/따옴표/백슬래시가 있는 argv가 shell 문자열로 합쳐지지 않고 별도 인자로 보존되는 것도 고정했다.
- **남은 일**: 실제 Windows runner에서 exit code 보존을 확인해야 한다. ConPTY interactive 동작은 PM-2B 범위다.

## 2026-06-23 — PM-1 shellcore 플랫폼 실행 경계

- **배경**: 플랫폼 피벗 이후 `shellcore`는 데스크톱 `ash`뿐 아니라 Android/iOS/PWA 임베딩 경로에서도 써야 한다. 기존 `engine.rs`는 빌트인 아닌 명령을 곧바로 `external::run`으로 보내 `std::process::Command` spawn 경계가 pure evaluator와 섞여 있었다.
- **구현**: `shellcore::external::ExternalRunner` 경계를 추가했다. 기본 `Engine::new()`은 기존 desktop process runner를 쓰고, `Engine::pure()`은 `DisabledRunner`로 외부 명령을 PATH lookup 전 차단한다. capability flags(`can_spawn`, `has_pty`, `has_conpty`, `has_userland`, `can_write_workspace`, `can_network`)도 runner에서 노출한다.
- **문서/테스트**: `docs/superpowers/specs/2026-06-23-platform-execution-contract.md`에 command resolution, argv, cwd/workspace, env, stream/exit, 플랫폼 capability 표를 기록했다. pure `shellcore` 테스트는 spawn 없이 `where`/`length`를 검증하고, integration smoke는 `ash` REPL에서 `size 200` 행만 남는 baseline을 고정한다.
- **남은 일**: filesystem builtin(`cd`/`ls`)은 아직 host filesystem을 직접 쓴다. Android/iOS/PWA는 PM-3/PM-4에서 workspace/filesystem adapter를 별도로 정해야 한다. Windows native `ash.exe` adapter와 ConPTY smoke는 PM-2 범위다.

## 2026-06-04 — FU-4 / M1 (slice 4a): Noise 세션 전송 substrate (실제 소켓)

- **배경**: s3는 Noise 왕복을 인메모리 버퍼로 검증. 실제 데몬↔폰은 스트림(소켓/Tailscale)을 거치며 가변 길이 Noise 메시지에 framing이 필요. 이 슬라이스는 framing + 역할 함수로 **실제 스트림 위 완주**를 증명(substrate 교체점).
- **`session.rs` 확장(`remote` feature)**: `send_frame`/`recv_frame`(제네릭 `Read`/`Write`, `[u32 BE len][payload]`, 1 MiB 상한 DoS 가드) + `run_device`(initiator: handshake→요청 수신·서명→응답)·`run_daemon_request`(responder: handshake→요청 송신→응답 수신). 검증(consume+validate)은 호출자(데몬).
- **검증**: `frame_roundtrip_and_size_guard`(무손실 + 과대 길이 거부) + **`approval_roundtrip_over_unix_socket`**(실제 `UnixStream::pair`, device 스레드↔daemon 스레드 handshake+승인 왕복 → `validate`=Approved/Rejected). default 230(코어 불변)/`--features "storage tls remote"` 263 green, clippy clean.
- **제외(M1 후속)**: 실제 데몬 프로세스의 디바이스 연결 리스너·페어링/등록·게이트 플로우 결선(armed High opt-in → 승인 트리거)·PWA·relay(M2). 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-transport-design.md`.

## 2026-06-04 — FU-4 / M1 (slice 3): Noise 세션 승인 왕복 (크립토+검증 end-to-end)

- **배경**: M0.5(크립토)·M1s2(검증)가 따로 검증됨 → 이 슬라이스가 둘을 **실제 Noise 암호문 위에서** 잇는다. "데몬 암호화 송신 → 디바이스 복호·서명 → 데몬 복호·검증" 한 바퀴를 메모리 내 Noise 채널로 증명(전송 substrate·PWA만 남김).
- **`session.rs`(`remote` feature)**: 와이어 메시지 `ApprovalRequestMsg`/`ApprovalResponseMsg`(serde_json; `[u8;N]` serde 한계로 `Vec<u8>` + `try_into` 길이 검증) + `encode`/`decode` + 변환(`from_pending`/`to_signed`) + `device_respond`(모의 디바이스 Ed25519 서명, PWA의 in-repo 대역).
- **검증(3)**: `wire_roundtrip`(직렬화 무손실), **`end_to_end_approve_over_noise`**(XX handshake → 암호화 요청 → 서명 응답 → 복호 → `NonceStore.consume`+`approval::validate` = Approved, replay 차단), `end_to_end_reject_over_noise`(Rejected). default 230(코어 불변)/`--features "storage tls remote"` 261 green, clippy clean.
- **제외(M1 후속)**: 실제 소켓/Tailscale 전송에 실어 보내기(데몬↔폰)·페어링/QR·디바이스 등록 영속화·context_hash 산출(§31.10)·데몬 게이트 플로우 승인 결선·PWA·relay(M2). 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-noise-session-design.md`.

## 2026-06-04 — FU-4 / M1 (slice 2): 승인 검증 상태머신 + nonce 저장소 (보안-핵심)

- **배경**: TEST-PLAN ship 게이트 = "revoke / replay / TOCTOU 거부 세 음성 케이스 + 서명 검증 필수". 폰/네트워크 독립적으로 단위 검증 가능 → 전송보다 먼저 구현해 위험 최소화.
- **`approval.rs`(`remote` feature)**: `validate(pending, device, now, ctx_hash, resp) -> ApprovalOutcome{Approved|Rejected|Invalid(reason)}` 순수 검증 — 순서: approval_id/nonce 매칭 → 만료 → revoke(device_epoch 단조) → 서명(`remote::verify_approval` Ed25519) → TOCTOU(context_hash 재검증) → 결정. `NonceStore`(register/consume 1회용 = replay 차단/prune). `gen_nonce`(getrandom, C-free).
- **검증(9 단위, ship 게이트 음성 케이스)**: happy approve/reject, **replay**(NonceStore 2회 consume false), **expired**, **revoke**(stale epoch), **bad signature**(다른키), **TOCTOU**(context drift), id/nonce mismatch. `remote.rs` sign/verify 연동.
- `getrandom` 의존을 `remote` feature에 추가(순수 Rust, C-free). default 230(remote 미포함, 코어 불변)/`--features "storage tls remote"` 258 green, clippy clean.
- **제외(M1 후속)**: Noise 전송으로 요청/응답 실제 송수신·페어링/디바이스 등록 영속화·context_hash 산출(§31.10)·데몬 게이트 플로우에 승인 결선·PWA. 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-approval-validation-design.md`.

## 2026-06-04 — FU-4 / M1 (slice 1): 로컬 게이트 데몬 + hook 결선

- **배경**: 원격 승인 한 바퀴 = "armed + 게이트 명령 → (블로킹 IPC) → Host 데몬 → 결정 → 실행/차단". M0는 `ai __gate`가 로컬 결정만 했다. 이 슬라이스는 결정 지점을 **데몬으로 옮길 수 있는 IPC 경로**를 깐다(향후 폰 왕복이 끼어들 seam).
- **로컬 IPC(hook↔데몬, 신뢰 경계 내부 — phone Noise와 다른 채널)**: `daemon.rs`(unix) — `tokio::net::UnixListener` 소켓(`<config_dir>/gate.sock`), 개행 구분 JSON(`GateRequest{command}`→`GateReply{decision,reason}`). `serve`(tokio accept 루프)·`query`(동기 std 클라이언트)·`decide_with`/`decide_request`(`gate::decide_gate` shared-core, §30-13).
- **`ai __gate` 결선 + fail-safe**: armed 아님 → 즉시 통과(hot-path). armed → 데몬 질의. **데몬 도달 불가 → 로컬 `decide_gate` 폴백**(데몬 다운은 보안 경계 아닌 자기-가드레일, 셸 비중단 + 로컬 보호 유지).
- **`ai remote daemon`**: 포그라운드 데몬(tokio, Ctrl-C 종료), stale 소켓 정리.
- **검증**: 단위(`decide_with` §30-13)·통합(`serve↔query` 실제 왕복, tokio 런타임)·CLI 파싱 + **e2e**(arm→데몬 기동→`armed_critical=BLOCK via=DAEMON`·`armed_safe=ALLOW`→데몬 종료→`daemon_down_critical=BLOCK via=LOCAL`). default(unix) 230 / `--features "storage tls remote"` 249 test green, clippy clean.
- **제외(M1 후속)**: phone Noise 왕복·페어링/QR·PWA·컨텍스트 스냅샷 데몬 보유·nonce 소비·context_hash·revoke·TTL/heartbeat·데몬 백그라운드화. 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-daemon-design.md`.

## 2026-06-04 — FU-4 / M0.5: 원격 승인 와이어 프로토콜 + 크립토 코어 (검증 기반)

- **배경**: DESIGN M0.5 = 와이어 프로토콜을 "제대로" 정의(Noise 검증 패턴, AKE 직접 안 굴림; primitive X25519+ChaCha20-Poly1305+Ed25519).
- **크립토 스택 확정(context7 `/mcginty/snow` 검증)**: 핸드셰이크/transport = **snow** `Noise_XX_25519_ChaChaPoly_BLAKE2s` **default resolver(순수 Rust, `ring`/C 불필요)**, 승인 서명 = **ed25519-dalek**(순수 Rust). 전부 C-free → storage/tls와 달리 C 의존 없음. 코어 경량화 위해 `remote` feature 게이트.
- **와이어 프로토콜 스펙**: `docs/superpowers/specs/2026-06-04-remote-approval-m05-wire-protocol-design.md` — Noise_XX 상호인증, TOFU 페어링(QR=daemon pubkey 앵커+pairing_code), 앱 레이어 메시지(ApprovalRequest/Response, approval_id·nonce·expires_at·context_hash·device_epoch), 보안 불변식(replay 1회용 nonce·TOCTOU context_hash 재검증·revoke device_epoch·서명 바인딩·§30-13 경계 shared-core·fail-closed), framing([u32 BE len][json]).
- **크립토 코어 구현(`remote.rs`, `--features remote`)**: `generate_static_keypair`(X25519), `approval_signing_bytes`(approval_id‖nonce‖decision 바인딩), `sign_approval`/`verify_approval`(Ed25519). 검증: **XX 핸드셰이크 상호인증(양측 static key 학습) + transport 암복호 왕복 + 변조 암호문 거부**, **Ed25519 서명/위조·다른키·다른결정 거부**.
- **검증**: default 227(remote 미포함, 코어 불변) / `--features "storage tls remote"` 246 / clippy clean. **C-free 확인**(snow default resolver, ring 없이 컴파일).
- **제외(M1+)**: 데몬 프로세스·Unix/TCP 소켓 서버·armed↔원격 왕복 결선·페어링 CLI/QR 생성·PWA·relay(M2)·TTL/heartbeat/viz·nonce 저장소·context_hash 산출. 본 단계는 프로토콜 정의 + 크립토 코어까지.

## 2026-06-04 — FU-4 / M0: 원격 승인 셸 인터셉트 제어점 (검증 기반)

- **배경**: 원격 승인 빌드(`../document/planning/builds/remote-approval/`)의 전 리뷰 만장일치 결론 = "인터셉트 제어점이 최대 feasibility 위험, 크립토 전에 먼저 증명". Codex: "preexec 반환값 차단은 비이식적".
- **검증 우선(spike, WSL)**: 추정 아님 — bash `extdebug`+`DEBUG` trap(비0 반환=실행 취소)·zsh ZLE `accept-line` 위젯으로 대화형 차단 실증(preexec는 차단 불가 확인), Unix소켓 데몬 왕복 0.117ms, 비-armed hot-path 0.02ms, fail-closed. 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m0-intercept-design.md`.
- **구현(M0, in-repo)**:
  - `gate.rs`(신규): `decide_gate(cmd, armed, allow_high)` 순수 결정(§30-13: Low/Medium 통과·High 기본차단/opt-in 통과·Critical 항상 차단) + armed 상태 파일(`<config_dir>/armed`, parse/render/load/arm/disarm).
  - `ai __gate "<cmd>"`(내부): armed 읽어 결정 → exit 0=통과/비0=차단(셸 hook이 실행 취소). 경로 접근 실패=fail-closed.
  - `ai remote arm [--allow-high] / disarm / status`.
  - `shell.rs`: bash hook을 단일 DEBUG trap에 인터셉트 병합(+`shopt -s extdebug`+재진입 가드), zsh hook에 ZLE 위젯 추가. armed 파일 존재 시에만 `ai __gate` 호출(hot-path: 파일 stat).
- **위험도 경계(§30-13)**: balanced/paranoid 무관하게 원격 승인 경계 강제. **위협모델**: "자기 자신용 가드레일"(대화형 셸 전용, 직접 binary·PATH 변조·데몬 kill로 우회 가능), advisory best-effort.
- **검증**: gate 6 단위 + CLI 파싱 + `bash -n`/`zsh -n` 문법 + **대화형 e2e**(실제 hook+ai 바이너리로 bash/zsh armed 차단·통과, 파일시스템 확인). 등급 픽스처는 `ai risk`로 실측 보정(rm -rf <abs>=High 55, rm -rf /=Critical).
- **제외(M1+)**: 크립토(Noise/X25519/AEAD/Ed25519)·데몬(tokio)·Unix소켓 서버·페어링/QR·PWA·원격 왕복·TTL/heartbeat/viz·replay/nonce·revoke·TOCTOU. 계획: `docs/superpowers/plans/2026-06-04-remote-approval-m0-intercept.md`.

## 2026-06-04 — FU-3 후속: readline이 probe 마커 가로채 무한 행(hang) 수정

- **배경/증상**: FU-3 핸드오프에서 "재확인 보류"였던 `persistent_session_keeps_cwd_and_probe_reports_it`이 실제로 `cargo test`(병렬) 전체를 무한 행시켰다. `ai shell`(`run_persistent_shell`)도 동일하게 첫 명령에서 멈추는 **프로덕션 결함**.
- **근본 원인**: `PROBE`(U+001F = Ctrl-_)는 bash readline의 `undo` 키바인딩이다. 라인 에디터가 켜진 인터랙티브 셸에 `\x1f`를 입력하면 readline이 편집 명령으로 가로채 명령줄을 파괴 → probe가 출력에 전혀 도달하지 못함 → `PtySession::read_chunk`(블로킹)가 영원히 대기. (WSL `python3 pty.fork()`로 마커수 0 vs 4 재현·확증)
- **수정**: `wrapper::session_shell_args(shell)` — bash는 `--noediting`으로 spawn해 라인 에디터를 끄고 마커를 리터럴로 통과(사용자 rc는 유지해 별칭/PATH 보존). `main.rs run_persistent_shell`·wrapper 테스트가 헬퍼 공유. `PtySession::killer()` 추가 + 테스트 5s 워치독으로 회귀 시 무한 행 대신 fail-fast.
- 검증: `persistent_session_keeps_cwd_and_probe_reports_it` 통과(0.06s), 전체 `cargo test` 219, `--features "storage tls"` 236 통과, clippy clean.

## 2026-06-04 — FU-3 영속 PTY 셸 런처 (probe cwd 동기화, 바운디드 MVP, §30-1/§7.4)

- **배경**: `ai exec`/`ai tui`는 명령마다 새 PTY를 써 `cd` 등 built-in 상태가 유지되지 않는다. §30-1 Native Wrapper의 핵심은 영속 셸 + 실행 후 probe로 상태 동기화(§7.4).
- **범위(사용자 확정 바운디드 MVP)**: 전체 입력 인터셉트·분류(§30-1도 "부담 큼" 명시)는 제외 — 라인 게이트는 기존 `ai exec`/`ai tui`. cwd probe 동기화 핵심만.
- **wrapper**(`wrapper.rs` 신규): `PROBE`(U+001F) 마커. `probe_command(cmd)`(실행 후 `\x1f$PWD\x1f` 방출), `parse_probe_cwds`(닫힌 마커쌍만 추출, dangling 무시), `strip_probes`(표시용 마커 제거) — 모두 순수·단위 테스트.
- **`ai shell`**(`main.rs` `run_persistent_shell`): 단일 `PtySession`(bash) 재사용 라인 REPL → `cd`가 다음 명령에 유지(영속성). 각 명령을 `probe_command`로 감싸 실행, 출력에서 cwd 파싱해 `sync_wrapper_cwd`(storage: `update_session_cwd`/`record_context_snapshot` type=`wrapper_probe`). TTY 루프라 단위 테스트 비대상.
- 검증: wrapper 순수 6 단위 + Windows default+storage 전체 green(209/226), clippy/fmt clean. **WSL 영속성 e2e**(`persistent_session_keeps_cwd_and_probe_reports_it`, unix-only)는 cargo 락 경합으로 핸드오프 시점 재확인 보류 — PtySession 경로는 WI-3/WI-5에서 검증됨.
- 설계: `docs/superpowers/specs/2026-06-04-persistent-shell-probe-design.md`(상위: `plans/2026-06-04-phase2-followups.md` FU-3).

## 2026-06-04 — FU-2 실행형 preview 샌드박스 (tmpdir 백엔드, §31.5/§31.11)

- **배경**: W9 안전 preview는 실행 없는 diff(cp/mv)·content-at-risk(rm)만. `sed -i`·포매터 등 실행 필요 in-place 편집은 "보류" 안내만 했다(§31.5 diff 미충족).
- **sandbox**(`sandbox.rs` 신규): tmpdir 백엔드 — 대상을 임시 복사 → 명령의 경로 토큰을 임시본으로 치환(`rewrite_path`) → temp cwd에서 실행 → 원본 vs 임시본 `diff::unified_diff`. **원본 절대 미수정.** `in_place_targets`(기존 일반 파일만, 플래그/스크립트/`=` 제외)·`rewrite_path`(순수)·`preview_in_place`(오케스트레이션). 크기 상한 64KiB, temp는 `Drop` 정리.
- **preview 연결**(`preview.rs`): `render_temp_copy`의 in-place 분기가 보류 대신 `sandbox::preview_in_place` 호출(빈 결과/실패 시 보류로 강등). `ai preview` 출력에 실제 diff.
- **플랫폼 가드**: 샌드박스 실행은 POSIX 셸·경로 의존 → **Unix(WSL/Linux) 한정**(`cfg!(unix)`), Windows 네이티브는 보류로 강등(셸/경로 차이로 인한 원본 오염 원천 차단).
- **위협/완화**: 실행은 `is_in_place_edit` 분류된 알려진 집합만, temp 복사본 대상, cwd=tempdir → 부수효과 최소. bubblewrap/gVisor 격리는 후속(§31.11).
- 검증: 단위(`rewrite_path`·`in_place_targets`, Windows), **WSL e2e**(`sed -i` 원본 미수정 + foo→FOO diff). default+storage 전체 통과(Win 205/222·WSL), clippy/fmt clean.
- 설계: `docs/superpowers/specs/2026-06-04-sandbox-preview-design.md`(상위: `plans/2026-06-04-phase2-followups.md` FU-2).

## 2026-06-04 — FU-1 리팩터 부채: 캐시 용량 상한 + `cmdparse` 공용화

- **캐시 LRU**(`cache.rs`): `ResponseCache`(HashMap)·`SemanticCache`(Vec) 둘 다 무한 증가(line 111 TODO)였다 → `DEFAULT_CACHE_CAPACITY=1024` + `with_capacity`. `put` 시 용량 초과면 가장 오래된(삽입 시각 최소) 항목 축출(Semantic은 만료 정리 후 앞쪽 제거). 장기 세션 메모리·선형 탐색 비용 제어.
- **`cmdparse` 공용화**(`cmdparse.rs` 신규): `program_token`이 preview·pipeline에 중복, 래퍼 스킵(`sudo|doas|env|nohup|nice`+`VAR=`)이 verify/risk/preview/pipeline에 흩어져 있었다 → `is_wrapper_token`/`is_env_assignment`/`program_token`/`args_after_program` 단일 진실원. preview(`program_token`·`path_args`·`extract_targets`)·pipeline(`program_token`·`candidate_paths`)·verify(`extract_program`)가 위임. 동작 보존 리팩터.
- 검증: cmdparse 단위(래퍼/환경 스킵), 캐시 축출 단위(용량 초과 시 oldest 축출·기존 키 갱신 무축출). default+storage 전체 통과(기존 테스트 무회귀), clippy/fmt clean.
- 계획: `docs/superpowers/plans/2026-06-04-phase2-followups.md` FU-1.

## 2026-06-04 — WI-5 TUI mid-exec 중단 + 라이브 스트리밍 (Phase 1 실사용 갭, §5/§31.5/§16.2)

- **배경**: TUI(`ui::run`)는 Submit 시 `dispatch::run`을 동기 블로킹 실행해 (1) 장기 명령 출력이 라이브로 안 보이고 (2) 실행 중 중단 불가했다. CLI `run_in_pty_streaming`은 프로세스 전역 ctrl_c라 raw-mode TUI(Ctrl+C=KeyEvent)에 부적합.
- **pty**(`pty.rs`): `run_in_pty_streaming_cancellable(shell, cmd, cancel: Arc<AtomicBool>, on_chunk)` 추가 — `child.clone_killer()`로 워처 스레드가 `cancel`을 20ms 폴링해 kill(출력 없는 silent 명령도 중단). 취소 시 130, 아니면 자식 exit.
- **ui**(`ui.rs`): Submit을 메인에서 `dispatch::dispatch`로 분류 → **셸만 `std::thread::scope` 워커**에서 `pipeline::execute`(게이트+취소 실행) 수행, `ChannelSink`로 청크를 메인에 송신. 메인 루프가 `try_recv`로 라이브 표시 + `event::poll(20ms)`로 Esc/Ctrl+C 중단 요청. **AI는 메인 스레드 동기**(GatewayResponder Send 비보장 회피, 요청 타임아웃이 상한). `render_shell_tail`로 완료 후 상태 꼬리만 append(라이브 출력 이중표시 방지).
- **위협/완화**: hook·셸 비중단 유지(워커 패닉/실패는 안내 후 루프 지속). 중단은 자식 프로세스 kill(고아 없음). AI 차단/타임아웃은 기존 경로 유지.
- 검증: pty 단위(취소→130 즉시 kill·정상→출력+exit, WSL), `render_shell_tail` 단위(Ran0/130/N·Blocked·Declined·BackupRefused). WSL 전체(lib 221) 통과, Windows default+storage 통과, clippy/fmt clean. TUI 루프 자체는 기존 `run`과 동일하게 단위 테스트 비대상.
- 설계: `docs/superpowers/specs/2026-06-04-tui-mid-exec-cancel-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-5).

## 2026-06-04 — WI-4 Native Wrapper fallback: 통합 모드 감지·표시 (Phase 1 실사용 갭, §30-1/§29.1)

- **배경**: §30-1 확정은 "Hook 기본 + Native Wrapper fallback"이나, hook 가용성 감지나 fallback 인지가 전혀 없었다.
- **조사**: `ai exec`/`ai dispatch`의 `Ran` 경로는 이미 `record_exec`로 명령+cwd+exit를 `commands`에 기록한다 → **wrapper 모드의 데이터 수집은 이미 기능**. 별도 wrapper 기록 신설은 중복(over-design)이라 추가하지 않음.
- **shell**(`shell.rs`): `ConfiguredMode{Hook,Wrapper,Auto}` + `IntegrationMode{Hook,Wrapper}` + `resolve_integration_mode`(Auto→hook 활성이면 Hook, 아니면 Wrapper). `hook_active(env_get)`가 `AI_TERMINAL_HOOK` 마커 확인(DI 테스트). bash/zsh hook이 `export AI_TERMINAL_HOOK=1` 설정.
- **doctor**(`main.rs`): `ai doctor`가 유효 통합 모드(hook 활성 / wrapper fallback) 표시 + wrapper 시 `ai exec` 사용·`ai init shell` 설치 안내.
- **범위 결정**: 영속 PTY 셸 런처(프롬프트 파싱·probe 동기화)는 무겁고 `ai tui`/WI-5와 중복 → Phase 2 이연. 심층 hook-health(활성 셸 수 등)는 T-RA5 이연.
- 검증: 단위(모드 해석 4케이스·hook_active·마커 export), `bash -n`/`zsh -n`(WSL 24 tests), doctor 양 모드 스모크(Windows: 마커 유무로 hook/wrapper 전환). default+storage 전체 통과, clippy/fmt clean.
- 설계: `docs/superpowers/specs/2026-06-04-wrapper-fallback-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-4).

## 2026-06-04 — WI-3 bash cwd hook 연동 (chpwd 에뮬레이션, Phase 1 실사용 갭, §31.1/§31.10)

- **배경**: zsh는 native `chpwd`로 디렉터리 변경 시 세션 cwd·git branch를 갱신하나, bash는 native chpwd가 없어 `cd`/`git switch` 후 컨텍스트가 갱신되지 않았다. bash `precmd` 핸들러는 `exit=`만 처리하고 받은 `cwd=`를 무시했다.
- **shell**(`shell.rs` BASH_HOOK): 셸 변수 `__ai_last_pwd`로 직전 PWD 보관. precmd에서 `$PWD != $__ai_last_pwd`이면 변수 갱신 후 `ai __hook chpwd "cwd=$PWD"` 호출(zsh와 동일 이벤트로 수렴 → `record_hook_chpwd` 핸들러 재사용, Rust 측 무변경). 초기값 빈 문자열 → 첫 prompt에서 초기 cwd 1회 기록.
- **불변식 유지**: `command -v ai` 가드 + `>/dev/null 2>&1 || true`, precmd가 `local __ai_ec=$?` 캡처 후 `return $__ai_ec`로 종료 코드 보존(chpwd 에뮬레이션은 그 사이에서 수행).
- 검증: 단위(BASH_HOOK가 `__ai_last_pwd`·`__hook chpwd`·`return $__ai_ec` 포함), `bash -n` 문법(WSL), **WSL e2e**: hook source 후 `cd` 2회→세션 cwd가 마지막 디렉터리로 갱신·context_snapshots에 chpwd 2건. default+storage 전체 통과, clippy/fmt clean.
- 설계: `docs/superpowers/specs/2026-06-04-bash-cwd-hook-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-3).

## 2026-06-04 — WI-2 `.env`/민감 경로 컨텍스트 제외 가드 (Phase 1 실사용 갭, §31.8)

- **배경**: `mask::is_sensitive_path`는 있으나 컨텍스트 경계에서 미사용. 현재 `context::gather`는 파일 본문을 수집하지 않지만, Phase 2 파일 본문 수집기 추가 시 `.env`/`.pem` 본문이 원격 AI로 유출될 면이 열린다(§31.8 미보장).
- **context**(`context.rs`): `allow_file_in_context(path)`(민감 경로면 false) + `filter_context_paths(paths)`(민감 경로 제거·순서 보존) 추가. 패턴은 `mask::is_sensitive_path` 단일 진실원에 위임.
- **계약**: 향후 파일 본문 수집기는 원격 전송 전 반드시 이 게이트를 통과 → 경로 게이트(1차) + 본문 마스킹(기존, 2차)의 이중 방어. fail-closed.
- **위협/완화**: `.env`/`*.pem`/`*.key`/`id_rsa`/`credentials` 본문의 원격 노출 차단. 경로 기준 결정적 제외.
- 검증: 단위(민감 경로 제외·일반 소스 포함·필터 순서 보존). default+storage 전체 통과, clippy/fmt clean.
- 설계: `docs/superpowers/specs/2026-06-04-context-sensitive-path-guard-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-2).

## 2026-06-04 — WI-1 Gateway 예산 게이트 + estimated 비용 (Phase 1 실사용 갭, §31.7)

- **배경**: `gateway::ask`가 백엔드(원격 AI) 호출 전 예산을 평가하지 않았고, `ai ask`는 비용을 `0.0`으로 하드코딩해 지출이 누적되지 않았다(§31.7 미충족). `usage::evaluate`는 순수 함수로 존재했으나 미연결.
- **usage**(`usage.rs`): `estimate_cost(input, output)` 추가 — per-token 단가 테이블로 비용 추정, 항상 `CostSource::Estimated`(provider 미보고 표시).
- **gateway**(`gateway.rs`): `BudgetSnapshot{spent_usd, cfg}` + `Gateway::with_budget(spent, cfg)`(주입식 — 게이트웨이는 storage 비의존). `ask`에서 **exact·semantic 캐시 미스 이후·`backend.generate()` 직전**에 `evaluate` 평가 → Block 임계 시 `Blocked("예산 초과 …")`. 캐시 히트·로컬 결과는 원격 비용이 없어 위에서 이미 통과(예산 무관).
- **cli**(`main.rs`): `ai ask`가 storage 시 `total_cost(None)`를 읽어 `with_budget` 주입. 응답 비용은 *원격 호출 시에만*(캐시 히트·ollama 로컬=$0) `estimate_cost`로 기록(0.0 하드코딩 제거)+`(cost ~ $X estimated)` 배지.
- **위협/완화**: 예산 초과 시 원격 전송 차단(fail-closed 비용 통제). 캐시/로컬은 비용 0이라 차단되지 않아 가용성 보존. 게이트는 *원격 호출 직전*에만 평가해 캐시된 답의 가용성을 해치지 않음.
- 검증: usage 단위(estimate_cost 양수·estimated·스케일), gateway 단위(초과→차단·백엔드 미호출, 캐시 히트 바이패스, 정상), storage 통합테스트(지출 $2 초과→`ask` 차단). default+storage 전체 통과, clippy/fmt clean. `ai ask` 런타임 배지 확인.
- 설계/계획: `docs/superpowers/specs/2026-06-04-gateway-budget-gate-design.md`, `docs/superpowers/plans/2026-06-04-gateway-budget-gate.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-1).

## 2026-06-03 — W9 안전(실행 없는) 실제 미리보기: unified diff + content-at-risk

- **diff**(`diff.rs` 신규): 순수 LCS 라인 unified diff(`unified_diff`, 외부 의존성 없음).
- **preview**(`preview.rs`): `render_preview`/`PreviewRender` 추가 — cp/mv 덮어쓰기(dst 기존)는 read-only로 진짜 unified diff, rm/shred/unlink·`> file` truncate는 content-at-risk(행·바이트·head). sed -i/perl -i/formatter 등 실행 필요 diff는 보류(샌드박스 후속). 크기 상한(diff 64KiB/risk 1MiB)·비UTF8 lossy·미존재/디렉터리 안전 처리. **대상 파일 절대 미수정**(e2e 확인).
- **cli**(`main.rs`): `ai preview`가 실제 diff/content-at-risk 출력(기존 분류 메시지는 Info로 유지).
- 검증: diff 단위(추가/삭제/변경/엣지), preview 단위(temp 파일: cp diff·rm risk·redirect·sed 보류·미존재), WSL e2e(dst 미수정 확인). clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-safe-preview-render-design.md`, `docs/superpowers/plans/2026-06-03-safe-preview-render.md`.

## 2026-06-03 — PTY 출력 스트리밍 + CLI Ctrl+C 중단 (W2 완료)

- **pty**(`pty.rs`): `run_in_pty_streaming` 추가 — 리더 스레드가 PTY를 블로킹 read해 bounded `tokio::mpsc`(cap 64)로 보내고(backpressure), current-thread 런타임이 `select!{ recv, ctrl_c }`로 청크를 `on_chunk`에 흘리며 Ctrl+C 시 자식 kill·버퍼 드레인·exit 130. 기존 `run_in_pty`/`PtySession` 유지.
- **pipeline**(`pipeline.rs`): `PtyExecutor::run`을 `run_in_pty_streaming(..|c| sink.write(c))`로 제자리 교체 → `ai exec`/`ai dispatch`/TUI 3경로가 라이브 스트리밍·CLI 중단 자동 적용. 트레이트 시그니처 불변.
- 검증: pty 단위 테스트(스트리밍 누적·종료코드 전파), WSL e2e(printf 라이브 출력·exit 전파·`sleep` SIGINT 즉시 중단). clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-pty-streaming-cancel-design.md`, `docs/superpowers/plans/2026-06-03-pty-streaming-cancel.md`.

## 2026-06-03 — gateway 시맨틱 캐시 2차 조회 결합

- **gateway**(`gateway.rs`): `ask`가 exact 캐시 미스 후 `SemanticCache::get_similar`(TTL 24h, Jaccard 임계값 0.85) 2차 조회. 시맨틱 히트는 그 답을 exact 캐시에 승격 저장(다음 동일 프롬프트는 exact 히트). 백엔드 응답은 exact+semantic 양쪽 저장(await 후 시각으로 TTL 기록). 시맨틱 키도 마스킹된 텍스트(RULES §2).
- **cache source 플래그**(`cache.rs`): `CacheSource { Backend, Exact, Semantic }`를 `GatewayOutcome::Answered`→`AiOutcome::Answered`로 전파. `ai ask`/`ai dispatch`가 캐시 히트 시 배지(`[cache: exact]`/`[cache: semantic ~근사]`) 표시.
- 검증: gateway 단위 테스트(시맨틱 히트→exact 승격, source 계층), `cache_badge` 라벨 테스트. clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-gateway-semantic-cache-design.md`, `docs/superpowers/plans/2026-06-03-gateway-semantic-cache.md`.

## 2026-06-03 — 비-Ran 명령 결과 audit 기록 (run_exec/run_dispatch, storage feature)

- **CLI**(`main.rs`): `shell_outcome_audit`(순수 매퍼 — 비-Ran `ExecOutcome`→`Option<AuditRecord>` 변환, 단위 테스트 가능) + `finish_shell_outcome`(공용 발산 헬퍼 — audit 기록 + 안내 후 `process::exit`) 추출. `run_exec`/`run_dispatch` Shell arm이 이 헬퍼를 공유하도록 중복 제거. `pipeline.rs`는 storage-free 유지(기록은 호출측).
- **audit 기록**: `Blocked`→`command_blocked`(Critical), `Declined`→`command_declined`(High 등 실제 등급 재산출), `BackupRefused`→`command_backup_refused`(해당 등급) — 마스킹된 명령(`mask::Masker::baseline().mask(...)`) 포함, `serde_json` payload(`command`/`source`/`factors`|`reason`). `Ran`은 기존 `record_exec`/`command_executed` 경로 유지(변경 없음).
- **storage 게이팅**: `record_outcome_audit`가 `#[cfg(feature = "storage")]` 게이트 안에서만 활성화 — 기본 빌드 C-free 유지.
- 검증: 단위 테스트 5개(Ran→None, 각 비-Ran 타입/level, BackupRefused reason, 마스킹 무유출). WSL e2e — `rm -rf /` → `('command_blocked','Critical','{…"command"…}')` 행 확인; `sudo systemctl restart nginx` + `n` 입력 → `('command_declined','High')` 행 확인. clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-audit-non-ran-outcomes-design.md`, `docs/superpowers/plans/2026-06-03-audit-non-ran-outcomes.md`.

## 2026-06-03 — Shell/Ai 단일 디스패처 통합

- **dispatch**(`dispatch.rs`): `run` 오케스트레이터 추가 — 입력 intent를 판정해 셸 경로(위험도→정책→preview→백업→실행 `pipeline`)와 AI 경로(주입된 `AiResponder`)로 라우팅한다. 셸/AI 양쪽 진입점을 하나로 일원화.
- **GatewayResponder**(lib 모듈, 신규): async `Gateway`를 동기 디스패처에 연결하는 브리지 — 내부 런타임에서 `ask_cancellable`을 구동해 타임아웃 + Ctrl+C 취소를 적용하고 동기 `AiResponder` 인터페이스로 노출. AI 경로 실패는 셸을 깨지 않음(graceful).
- **TUI**(`ui.rs`): Submit(Enter)를 `pipeline` 직접 호출 대신 `dispatch::run`을 거치도록 재배선 — 자연어 질의가 이제 AI 경로로 라우팅된다(명령은 셸 경로 유지).
- **CLI**(`main.rs`): `ai dispatch "<input>"` 원샷 명령 추가 — 디스패처를 직접 호출(셸/AI 자동 라우팅). audit 기록은 source를 "dispatch"와 "exec"로 구분.
- 설계/계획: `docs/superpowers/specs/2026-06-03-unified-dispatcher-design.md`, `docs/superpowers/plans/2026-06-03-unified-dispatcher.md`.
- 검증: 전체 테스트 default/storage/`storage tls` 모두 통과(0 failed; storage tls 합산 217), WSL e2e(셸 `echo` exit 0 / AI mock echo `(tokens ~ in:.. out:..)` exit 0 / `rm -rf /` Critical 차단 exit 1), fmt·clippy(`storage tls`, `-D warnings`) clean.

## 2026-06-03 — 그룹 C 백로그: 리다이렉트 인식 백업 대상 (W10 보완)

- **pipeline**(`pipeline.rs`): `strip_redirect_op`/`redirect_targets` 추가 — 셸 리다이렉트(`>f`/`>>f`/`N>f`/`&>f`/`> f`) 대상을 추출. `backup_targets`가 (삭제/덮어쓰기 프로그램 인자 ∪ 리다이렉트 대상)을 dedup 후 기존 일반 파일만 백업. `command.contains('>')` 거친 트리거 제거 → 붙은 `>out.txt`도 정확히 백업.
- **이유**: 기존엔 `echo x >out.txt`의 대상이 `is_file(">out.txt")`로 걸러져 덮어쓰기 전 백업이 안 됨(조용한 갭). 리뷰 LOW 보완.
- **한계**: 공백 분리된 인용 내 `>`(`echo "a > b"`)는 여전히 오인 가능하나 `is_file` 필터로 무해. 완전 정확성은 shell-words 토크나이저 영역 — 이연.
- 검증: TDD(strip_redirect_op/redirect_targets 단위 + backup_targets 통합 4), WSL e2e(`echo > f` 덮어쓰기→백업→`undo last` 복구). pipeline 11 + 전체 통과, clippy(default+storage)·fmt clean.

## 2026-06-03 — 그룹 C: 중앙 실행 파이프라인 (W10/W11/W2 키스톤)

- **pipeline**(`pipeline.rs`, 신규): `execute`가 위험도→정책(Block/Confirm)→preview→undo 백업(W10 자동 트리거, Refused 시 실행 중단)→실행→결과를 묶는다. I/O는 `Executor`/`Confirmer`/`OutputSink` 트레이트로 주입(PTY 없이 단위 테스트). `PtyExecutor`가 `run_in_pty` 래핑 — 청크 sink 모양이 W2 스트리밍을 수용(후속에 impl 교체).
- **CLI**(`main.rs`): `ai exec "<cmd>" [--yes] [--profile]` — stdin y/N 확인(`--yes`로 생략, Block은 우회 불가), 종료코드 전파. storage 시 명령+종료코드+audit 기록.
- **TUI**(`ui.rs`): Enter가 `run_in_pty` 직접 호출 대신 `pipeline::execute`를 거친다. 이번 증분은 위험(확인 필요) 명령을 거부+안내, Allow 명령은 실행.
- **백업 범위**: 삭제(rm/unlink/shred)·덮어쓰기/in-place(sed -i, `>`, cp/mv/tee/touch)의 기존 일반 파일만. 권한 변경(chmod/chown)은 내용 백업 무의미로 제외(한계 고지). W11은 셸 경로 토큰비용 없음 → AI 경로 기존 기록 재사용.
- 검증: TDD(pipeline 7: Allow/Block/Declined/Confirmed/백업생성/백업거부중단/종료코드), `ai exec` WSL e2e(rm 백업→undo 복구, `rm -rf /` 차단 exit 1). storage/default 통과, clippy(default+storage)·fmt clean.
- **후속**: W2 실제 async 스트리밍, W9 실제 diff, Shell/Ai 단일 dispatcher 통합, TUI 인라인 확인 모달.

## 2026-06-03 — 그룹 C 2b: HTTPS(TLS) transport (`tls` feature, Phase 2)

- **http**(`http.rs`): scheme 인식 `parse_url`(http/https) + `host_header`(기본 포트 생략) + 요청/응답 헬퍼 추출. `TcpTransport`가 스킴에 따라 평문/TLS로 분기.
- **TLS**(`#[cfg(feature = "tls")]`): `tokio-rustls`(ring) + `webpki-roots`로 `post_json_tls` — `RootCertStore` + `ClientConfig::builder_with_provider(ring)` + `TlsConnector`. tls 미빌드 시 https는 명확히 거부(조용한 실패 금지).
- **Cargo.toml**: `tls` feature + optional `tokio-rustls`(default-features off, ring/logging/tls12) + `webpki-roots`. rustls crypto provider가 C 툴체인을 요구하므로 `storage`처럼 게이트 → **기본 빌드 C-free 유지**.
- **CI**: `--features tls` clippy + `storage tls` build 추가. **README**: feature 빌드 안내.
- 검증: 단위(parse_url http/https·host_header·build_request), **실제 TLS e2e**(`ai ask --backend ollama --ollama-url https://postman-echo.com/post` → TLS 핸드셰이크+HTTPS 왕복 성공으로 JSON 수신; tls 없는 빌드는 거부). tls/default/storage 모두 144 통과, 양쪽 clippy clean.

## 2026-06-03 — 그룹 C 2a: 진짜 async transport + AI 경로 async 전환 (Phase 2)

- **http**(`http.rs`): `HttpTransport`를 async 트레이트(AFIT)로, `TcpTransport`를 `tokio::net::TcpStream` 기반 **비동기 평문 HTTP/1.1**로 전환. 진짜 async I/O라 상위에서 future drop(타임아웃/취소) 시 연결도 함께 취소(고아 호출 없음).
- **gateway**(`gateway.rs`): `LlmBackend`를 dyn 호환 async(박싱 future `GenerateFuture`)로, `Gateway::ask`를 async로. `ask_cancellable`이 `spawn_blocking` 없이 `run_cancellable(self.ask(...))`로 단순화 — #5의 워커-스레드 우회 제거.
- **backends**(`ollama.rs`/`openai.rs`): async generate(transport await). **Send 바운드 제거**(current-thread `block_on` 구동이라 불필요) → 새 의존성 0, C-free 유지.
- **main**(`main.rs`): ask 핸들러가 async gateway를 직접 await(Arc 불필요).
- 검증: 테스트 async 전환(gateway/ollama/openai #[tokio::test]), **로컬 mock HTTP 서버 e2e**(`ai ask --backend ollama` → tokio async TCP 실연결·응답 파싱). storage 158 / default 141 통과, fmt·clippy clean.
- **다음(2b)**: `tls` feature 게이트로 `tokio-rustls`(ring) + `webpki-roots` → HTTPS 클라우드 provider. 기본 빌드는 C-free 유지(rustls crypto provider가 C 툴체인 요구하므로 게이트).

## 2026-06-03 — 그룹 C 착수: AI 게이트웨이 타임아웃/취소 결합 (Phase 2, §16.2)

- **gateway**(`gateway.rs`): `Gateway` 스레드 안전화(`RefCell→Mutex`, `LlmBackend: Send+Sync`) + `ask_cancellable`(async) 추가 — 동기 `ask`를 `spawn_blocking`으로 옮겨 `aitask::run_cancellable`(타임아웃 + Ctrl+C)로 감싼다. 캐시 락은 백엔드 호출 전 해제.
- **http**(`http.rs`): `HttpTransport: Send+Sync`(transport를 워커 스레드로 이동 가능하게).
- **main**(`main.rs`): `ai ask`가 current-thread tokio 런타임에서 `ask_cancellable` 실행 + `cancel_on_ctrl_c`. 실패·타임아웃·취소 모두 graceful 고지(exit 0, §16.2).
- 검증: TDD(gateway: 느린 백엔드 타임아웃 / 정상 응답 통과 / 캐시; mock transport `RefCell→Mutex`), `ai ask` e2e(echo + 마스킹 유지). storage 158 / default 141 통과, fmt·clippy clean.
- **한계/다음**: 동기 백엔드는 타임아웃 시 호출자만 제어 복귀(고아 호출은 백그라운드 종료). 진짜 async transport(tokio TcpStream/TLS)·gateway 시맨틱 캐시 2차 조회는 다음 증분.

## 2026-06-03 — hook chpwd → cwd + git branch 컨텍스트 (M1/W3, §31.10)

- **store**(`store.rs`): `record_context_snapshot`(context_snapshots INSERT) + `latest_context`(최근 스냅샷 조회) + `update_session_cwd`(세션 cwd 갱신). `NewContext`/`ContextRow`.
- **main**(`main.rs`): `ai __hook chpwd cwd=<path>` 처리 → 세션 cwd 갱신 + 해당 경로의 git branch(`context::git_branch`)를 context_snapshot으로 기록(best-effort, 셸 비중단).
- 검증: TDD(store 2: 스냅샷 record/latest, session cwd update), WSL e2e(git 레포 → `(chpwd, …/terminal, master)`, 비-git → branch None, sessions.cwd 갱신; python3 sqlite로 확인). storage 156 / default 139 통과, fmt·clippy clean.
- **범위**: zsh는 `chpwd` hook을 발생시킴. bash는 native chpwd가 없어(precmd `cwd` 보유) bash용 cwd/branch 연동은 후속.

## 2026-06-03 — 마스킹 고엔트로피 휴리스틱 (M2/W7, §31.8)

- **mask**(`mask.rs`): 명명 규칙(AWS/GitHub 등)이 놓친 generic secret을 Shannon 엔트로피로 탐지·마스킹. named 규칙 적용 후 → validation 전 후처리 패스로 `[HIGH_ENTROPY_REDACTED]` 치환.
- 판정(`is_high_entropy_secret`): 길이 ≥20 + 엔트로피 ≥4.0 bits/char + 영문·숫자 혼합 + `_REDACTED` 플레이스홀더 제외. 후보 문자셋 `[A-Za-z0-9_=+-]`(점·슬래시·콜론 제외)로 경로/URL/도메인/버전 오탐 회피.
- 차단(block)이 아니라 마스킹(redact) — 마스킹 자체가 안전 조치이고, 해시 등 비밀이 아닌 고엔트로피 문자열 과차단을 피함(보수적 over-mask 허용).
- 검증: TDD(고엔트로피 마스킹 / 자연어·경로·저엔트로피 비마스킹 / guards 3종), 합성 토큰은 선형 순열로 결정성 확보(리터럴 시크릿 회피). `ai mask` e2e 확인. storage 154 / default 139 통과, fmt·clippy clean.

## 2026-06-03 — hook precmd 종료코드 + last-error 분석 (M1/W3 + M3/W12)

- **store**(`store.rs`): `update_last_exit(session, exit)` — `preexec`에서 `exit_code=NULL`로 기록된 직전 명령에 `precmd`의 실제 종료 코드를 채움(미정 1건만 갱신). `last_error(session)` — 가장 최근 실패(exit≠0) 명령 조회. `OptionalExtension` 사용.
- **main**(`main.rs`): `ai __hook precmd exit=<n>` 처리 → `update_last_exit`(best-effort, 셸 비중단). `ai explain --last-error`가 저장소의 직전 실패 명령을 꺼내 분석(`command`를 Optional로 변경, storage 미빌드 시 명확한 안내).
- 셸 hook 스크립트는 이미 `precmd`에 `exit=$?`를 전달 중이었음 → Rust 쪽 처리만 추가하면 연결 완성.
- 검증: TDD(store 단위 4 + CLI 파싱 1), WSL e2e(`frobnicate` exit=127 → `explain --last-error`가 "명령을 찾을 수 없습니다" + 제안; 성공-only는 "실패 명령 없음"). storage 151+22+4 / default 136+22+4 통과, fmt·clippy(default+storage) clean.
- **TASK 정정**: W6 `ai policy set` 영속·W7 전화/카드/여권 패턴은 이미 구현됨을 반영(문서가 stale했음).

## 2026-06-02 — Phase 2 후속: Semantic Index + Tool Use Planner (P2-11~12)

- **Semantic File Index**(`index.rs`): `FileIndex::build/search`(무시 디렉터리·대용량 제외 키워드 인덱스/랭킹). `ai index`.
- **Tool Use Planner**(`planner.rs`): `plan` 규칙 기반 명령 단계(복합 다단계/무매칭 AI 위임). `ai plan`.
- 환경: Windows `target/`가 3.8GB로 디스크 가득참 → `cargo clean` 후 재빌드(기본 feature로 검증, storage는 WSL/CI).
- 검증: Windows 기본 157개 통과, clippy/fmt clean. (storage 포함은 WSL에서 확인.)
- 남은 P2: async aitask 결합·HTTPS TLS·시맨틱 캐시 gateway 결합·데몬.

## 2026-06-02 — Phase 2 우선순위 진행: dispatcher/verify/skill/semcache/mcp (P2-6~10)

- **Hybrid dispatcher**(`dispatch.rs`): intent→Shell{risk,decision}/Ai/Empty. `ai route`.
- **Verification Agent**(`verify_agent.rs`): 환각+위험도+정책+secret 종합 Verdict. `ai verify`.
- **스킬 관리(§26)**(`skill.rs`): SKILL.md discover/parse/match. `ai skill`.
- **시맨틱 캐시**(`cache.rs`): Jaccard 유사도 `SemanticCache`.
- **MCP 관리(§27)**(`mcp.rs`): mcp.json 파싱 + mutate 도구 컨센트 판정. `ai mcp`.
- 검증: Windows 161개·Linux 동등, clippy(default+storage)/fmt clean. 커밋 5개 분리.
- 남은 P2: Tool Use Planner(AI 의존), async aitask 결합, HTTPS TLS, Semantic Index, 데몬.

## 2026-06-02 — Phase 2 진행: Intent/Cache/Ollama/OpenAI (P2-2~5)

- **P2-2 Intent**(`intent.rs`): `classify`(Shell/AiQuery/AiInline/Empty), `ai classify`.
- **P2-3 Cache**(`cache.rs`): TTL 정확 캐시 + Gateway 연동(히트 시 백엔드 생략, counting 테스트).
- **P2-4 Ollama**(`http.rs`+`ollama.rs`): `HttpTransport` 주입(+`TcpTransport` 무의존 평문 HTTP) + `OllamaBackend`(/api/generate, mock 테스트). `ai ask --backend ollama`.
- **P2-5 OpenAI**(`openai.rs`): bearer 인증 transport + `/v1/chat/completions` + `OpenAiBackend`($OPENAI_API_KEY). `ai ask --backend openai`.
- AI 백엔드 실패는 친절 고지 후 정상 종료(§3-3, exit 0). serde_json 추가.
- 검증: Windows 141개·Linux 동등 테스트 통과, clippy(default+storage) clean, fmt clean. 커밋 분리(intent/cache/ollama/openai).

## 2026-06-02 — Phase 2 착수: AI Model Gateway (P2-1)

- `src/gateway.rs` (TDD): `LlmBackend` 트레이트 + `EchoBackend`(mock), `Gateway::ask` 파이프라인 — prompt+context → **마스킹**(secret 치환/private key 차단 fail-closed) → 토큰 윈도(한도 초과 시 truncate) → 백엔드 → 토큰 추정.
- `ai ask "<prompt>"` CLI: 컨텍스트(cwd) 포함, 토큰 표시, storage feature 시 usage 자동 기록. echo 백엔드로 "secret이 백엔드 도달 전 마스킹됨" 검증.
- 이전 MVP 모듈(mask/tokenwin/provider/usage/context)을 AI 경로로 결합 — Phase 2의 토대.
- 검증: Windows 123개·(WSL 동등) 테스트 통과, clippy(default+storage) clean, fmt clean.
- 후속: 실제 provider HTTP 어댑터·로컬 LLM(Ollama), aitask 타임아웃/취소를 async 백엔드에 결합, Intent Classifier 등.

## 2026-06-02 — M4 구현 + MVP 진입 (context/guardrails/provider/호환성, W13~W16)

- **W13 context**(`src/context.rs`): `SessionContext`/`gather`/`is_context_changing`/`filter_env_var`(allowlist+denylist+PATH hash, secret 미저장)/`needs_refresh`/`git_branch`(.git/HEAD). `ai context`.
- **W14 guardrails**(`src/guardrails.rs`): `detect`(Linux/WSL/macOS/Other)·`baseline`·`capabilities` 매트릭스·`dynamic_monitoring_limited`. `ai doctor --guardrails` 리팩터링.
- **W15 provider**(`src/provider.rs`, `src/tokenwin.rs`): capability map + fallback(token_source/cost_source/use_streaming) + mock, `estimate_tokens`/`chunk`/`fits`.
- **W16 호환성+진입**: `tests/integration.rs`(결정성 50회·Critical 차단 100%·마스킹 무유출), `docs/MVP-ENTRY.md`(§31.12 9영역 + §31.13 확정값).
- 검증: 단계별 TDD, Windows 118개(94 lib + 20 bin + 4 integration)·Linux 동등, 양쪽 clippy(default+storage) clean, fmt clean. 커밋 W13~W16 분리.
- **M1~M4 로컬 결정성 핵심 완료.** provider 의존 원격 경로는 Phase 2(Model Gateway).

## 2026-06-02 — M3 구현 (preview/undo/usage/explain, W9~W12)

- **W9 preview**(`src/preview.rs`): `classify_preview`(dry-run 제안 / in-place→temp diff / 삭제·권한→대상목록 / 외부상태→불가 / 읽기→불필요), `ai preview`.
- **W10 undo**(`src/undo.rs`): `create_backup`(상한 enforcement→Refused) / `restore` / `latest`, `ai undo last`.
- **W11 usage**(`src/usage.rs` + store): `BudgetConfig`/`evaluate`(80% 경고/100% 차단), `record_usage`/`total_cost`, `ai usage`.
- **W12 explain**(`src/explain.rs`): 규칙 기반 에러 분석(not found/permission/no such file/generic), `ai explain`.
- 검증: 단계별 TDD, Windows 100개·Linux 104개 테스트 통과, 양쪽 clippy(default+storage) clean, fmt clean. 커밋 W9~W12 분리.
- M3 핵심 완료. 실행 파이프라인 자동 연동(백업 트리거·usage 자동기록·last-error stderr 캡처)은 provider/실행 연동 후속.

## 2026-06-02 — AI 요청 타임아웃 + Ctrl+C 취소 (M2/W8, §13·§16.2)

- `src/aitask.rs` 추가 (TDD, tokio): `Timeouts::defaults`(5/15/60/180s), `run_cancellable`(작업/타임아웃/취소 3-way select), `RequestError`(TimedOut/Cancelled/Failed), `cancel_on_ctrl_c`(SIGINT→취소).
- 실패·타임아웃·취소는 모두 `Err` 반환 → **AI 장애가 셸을 막지 않음**(Graceful Recovery, §16.2). tokio `sync` feature 추가.
- 검증: Windows 77개·Linux 81개 테스트 통과(async 테스트 포함), 양쪽 clippy clean, fmt clean.
- W8 완료 → M2 핵심(위험도·정책·마스킹·환각검증·타임아웃) 모듈 구현 완료. 실제 provider end-to-end는 Phase 2.

## 2026-06-02 — M1 잔여 항목 마무리 (5종, TDD + 커밋별 정리)

순차 진행한 M1 마무리 작업:
1. **마스킹 패턴 확장**(§31.8): 전화(KR)/신용카드/여권 추가, IP 오탐 방지.
2. **환각 검증 게이트**(§29.2, `src/verify.rs`): 바이너리 존재 검증(sudo/env/VAR= 건너뜀, 빌트인 인식, 경로/PATHEXT), `ai risk`에 binary 상태 표시.
3. **config 영속화**(§31.3, `src/config.rs`): 활성 프로파일을 `~/.config/ai-terminal/active_profile`에 저장. `ai policy set`, show/risk/tui는 활성 프로파일 사용.
4. **locks 레지스트리 + audit**(§31.2): `store`에 register/lock_owner/release/`reclaim_if_stale`(audit)/`record_audit`. 파일 락(lock.rs)과 함께 2층 구조 완성.
5. **TUI↔PTY 연결**(§5): TUI Enter 제출 → `pty::run_in_pty` 실행 → `append_output`로 히스토리 표시.

- 검증: Windows 72개·Linux 76개 테스트 통과, 양쪽 clippy(`--features storage`) clean, fmt clean.

## 2026-06-02 — 파일 락 + stale 정리 + DB 동시성 (M1/W4 잔여, §31.2)

- `src/lock.rs` 추가 (TDD): advisory 파일 락(`create_new` 원자적 상호배제), 락 파일에 pid/timestamp 기록, `LockGuard` RAII 해제. stale 판정(TTL 초과 / Linux는 `/proc` PID 부재) → 제거 → 재시도(§31.2).
- `store`: `integrity_ok`(`PRAGMA integrity_check`) 추가. **동시성 테스트**: 같은 파일 DB에 두 연결이 교대 write(30건) 후 무손상·integrity=ok 검증 → M1 완료 기준 "동시 터미널 무손상"(WAL+busy_timeout) 충족.
- 검증: Windows 58개·Linux 62개 테스트 통과, 양쪽 clippy clean, fmt clean.
- 후속: `locks` 테이블 heartbeat 레지스트리 + stale audit 기록(진단/복구 고도화).

## 2026-06-02 — Secret/PII 마스킹 (M1/W7, §31.8)

- `src/mask.rs` 추가 (TDD, regex): `Masker::baseline()` 규칙 테이블(Secret: private_key_block(hard block)/AWS/GitHub/Slack/Bearer/Authorization/Password, PII: email/kr_rrn/ipv4), `mask()`가 Secret→PII 순 적용 후 validation scan.
- fail-closed: private key block 감지 또는 validation 재매치 시 `blocked=true`(원격 전송 차단). 원문 secret 미잔존 검증 테스트.
- `is_sensitive_path`(.env/.pem/.key/id_rsa), CLI `ai mask "<text>"`(leading-dash 허용).
- authorization 치환문이 자기 패턴에 재매치되어 오탐 차단 → 치환문을 `[AUTHORIZATION_REDACTED]`로 수정.
- 검증: Windows 54개·(WSL 동일) 테스트 통과, clippy clean, fmt clean.

> 다음 단계: 2층 파일 락 + stale 정리(W4 잔여, M1 완료 기준).

## 2026-06-02 — TUI 렌더링 착수 (M1/W2, §5)

- `src/ui.rs` 추가 (TDD): `UiState`(입력 편집/submit/히스토리), `current_risk`(실시간 위험도), `handle_key`(Char/Backspace/Enter/Esc→Action), `render`(상태바 profile·cwd / 히스토리 / 입력+위험도).
- `ratatui::TestBackend`로 헤드리스 렌더 검증(상태바 profile, 입력 위험 등급 표시 확인). `run` 이벤트 루프(crossterm raw mode + alt screen, Esc/Ctrl-C 종료)는 TTY 필요로 단위 테스트 제외.
- CLI: `ai tui [--profile]`.
- 검증: Windows 45개·Linux 49개 테스트 통과, 양쪽 clippy clean, fmt clean.

> 다음 단계: Secret/PII 마스킹(W7, §31.8).

## 2026-06-02 — SQLite 스토리지 + PTY 인터랙티브 (M1/W4·W2, §31.2)

- `src/store.rs` 추가 (TDD, `storage` feature/rusqlite): `Store`(open/open_in_memory/open_default), §31.2 7테이블 스키마 + WAL/PRAGMA, CRUD(create/get_or_create session, record_command w/ 위험도, recent_commands, count), FK 강제, `data_dir`(XDG/HOME).
- e2e 배선: `ai __hook preexec`가 명령을 위험도와 함께 `sess-default`에 기록(best-effort, 재진입 가드) → `ai history`로 표시. 셸 hook → risk → SQLite → 조회 전 구간 동작. (storage feature, 기본 빌드는 C-free 유지.)
- `src/pty.rs` 확장: `PtySession`(spawn/write_input/read_chunk/kill) — 인터랙티브 입출력 프리미티브. WSL에서 `cat` echo 라운드트립 검증.
- SQL 다중행 리터럴의 `\` 줄잇기가 식별자를 붙여(`risk_scoreFROM`) 버그 유발 → 일반 개행으로 수정.
- 검증: Windows 40개(lib 27 + bin 13)·Linux 44개(lib 31 incl pty 3 + bin 13) 테스트 통과, 양쪽 clippy(`--features storage`) clean, fmt clean. PtySession은 Windows(ConPTY) 컴파일 확인.

> 다음 단계: 2층 파일 락 + stale 정리(W4 잔여, M1 완료 기준) 또는 TUI 렌더링(ratatui, W2 잔여) 또는 마스킹(W7).

## 2026-06-02 — 셸 Hook 생성/설치 UX (M1/W3, §31.1)

- `src/shell.rs` 추가 (TDD, 2 cycle): `Shell`(bash/zsh, 경로 파싱), `hook_script`(preexec/precmd/chpwd, `command -v ai` 가드 + 에러 무시), `rc_block`(마커 래핑 가드 블록), `is_installed`/`apply_install`(idempotent)/`apply_uninstall`(블록만 제거)/`unified_diff`(공통 prefix/suffix).
- CLI: `ai shell-hook <bash|zsh>`, `ai init shell [--shell --rc --dry-run --diff --uninstall]`, 내부 `ai __hook`(hide, no-op). 순수 `plan_init_shell`로 파일 I/O와 분리해 테스트.
- WSL 검증: 생성 hook이 `bash -n`/`zsh -n` 문법 통과, rc 라운드트립(install→`bash -n` OK→uninstall이 사용자 라인 정확 복원).
- §31.1 수용 기준 충족: `--dry-run`/`--diff` 미수정, `--uninstall` 블록만 제거, hook 실패가 셸 중단 안 함. (cd/exit/git 실제 기록은 W4 스토리지 연동 후 — 현재 `__hook` no-op로 wiring만.)
- 검증: Windows 34개(lib 21 + bin 13)·Linux 37개(lib 24 + bin 13) 테스트 통과, clippy clean, fmt clean.

> 다음 단계: SQLite 스토리지(W4, §31.2) — `ai-terminal.db` + 락. 정책 `set` 영속화·hook 상태 기록의 선행조건.

## 2026-06-02 — PTY 터미널 코어 착수 (M1/W2, WSL 검증)

- `src/pty.rs` 추가 (TDD): `run_in_pty(shell, command) -> PtyOutput{output, exit_code}` — portable-pty로 PTY를 열고 `shell -c command` 실행, 출력/종료코드 수집.
- 테스트는 `#[cfg(all(test, unix))]` — 실제 bash spawn이 필요해 **WSL(Ubuntu-Dev)** 에서 검증(`echo` 출력 포함, 종료코드 3 전파).
- 환경: WSL에 Linux Rust 툴체인 설치(rustup), 빌드는 `CARGO_TARGET_DIR=~/targets/ai-terminal`로 분리(/mnt/c 느림·Windows 산출물 충돌 회피). 소스는 `/mnt/c/...` 공유.
- 검증: Linux 21개(lib 14 incl pty 2 + bin 7)·Windows 19개(unix 테스트 제외) 통과, 양쪽 clippy clean, fmt clean. pty 모듈은 Windows(ConPTY)에서도 컴파일.

> 다음 단계: PTY 인터랙티브 세션 + 입출력 렌더링(W2 잔여) 또는 셸 Hook 생성/설치 UX(W3, §31.1).

## 2026-06-02 — 정책 엔진 + 프로파일 선구현 (W6, §31.3·§31.4)

- `src/policy.rs` 추가 (TDD): `PolicyProfile`(balanced 기본 / paranoid) 전체 필드(§31.3 권위값), `Decision`(Allow/Confirm/StrongConfirm/Block), `decide(level)` 액션 매핑(§31.4).
- 매핑: Critical→Block(두 프로파일), High→StrongConfirm(balanced)/Block(paranoid), Medium→Confirm, Low→Allow(balanced)/Confirm(paranoid).
- 위험 등급을 로컬 `risk::assess`에서 받으므로 "로컬 정책 우선"(§31.4)이 구조적으로 보장됨.
- CLI: `ai policy show [--profile]`, `ai risk --profile <p>`(결정 표시 추가). 미지원 프로파일은 명확히 오류.
- `set`(영속 변경)은 config 저장 모듈(W4) 구현 후로 보류.
- 검증: lib 12 + bin 7 = 19 테스트 통과, clippy clean, fmt clean.

> 다음 단계: WSL에서 M1 PTY/Hook 착수.

## 2026-06-02 — 위험도 스코어링 엔진 선구현 (W5, §31.4)

- `src/lib.rs` 라이브러리 크레이트 착수 + `src/risk.rs` 위험도 엔진 추가 (TDD, red-green-refactor).
- 0~100 rule-based 스코어링: 명령 유형 점수 → (액션 존재 시) 경로 가중치 최댓값 → 완화 요소. 등급 매핑 Low/Medium/High/Critical(§31.4).
- 결정성 보장(순수 함수). §31.4 "예시 분류" golden set 테스트로 고정: `ls -al`=Low … `rm -rf /`/`dd …=/dev/sda`=Critical, `chmod -R 777 .`/`curl|sh`/`sudo systemctl restart`=High.
- 순수 read-only 명령은 경로 가중치 미적용(`cat /etc/hostname`이 High로 오분류되지 않도록).
- `ai risk "<command>"` CLI 추가 — 점수·등급·요인(factor) 분해 출력(감사/설명용, RULES §2).
- 검증: lib 6 + bin 4 = 10 테스트 통과, clippy `-D warnings` clean, fmt clean.
- **순서 결정**: PTY(W2)·셸 Hook(W3)은 Linux 전용이라 Windows 개발 머신에서 검증이 어려워, 크로스플랫폼·결정성 보안 핵심인 위험도 엔진(W5)을 먼저 구현. 정책 엔진(W6)이 이 엔진에 의존한다.

> 다음 단계: 정책 엔진 + 프로파일(W6, §31.3) — balanced/paranoid에서 위험 등급별 액션(Critical 차단 등) 매핑. 또는 WSL 환경에서 M1 PTY/Hook 착수.

## 2026-06-02 — 구현 repo 부트스트랩 (M0)

- `../document/` 설계 정본(v3.3) 검토 완료.
- `docs/` working-set 5종 작성: PRD · TASK · WORKFLOW · HISTORY · RULES (한국어 압축형, 설계 repo §번호 참조).
- 기술 스택 확정: **Rust** (설계 1순위). ratatui · crossterm · tokio · portable-pty · serde/toml · clap · tracing · rusqlite.
- Rust 개발 환경 구성: `Cargo.toml` · `rust-toolchain.toml`(stable + rustfmt/clippy) · `rustfmt.toml` · `.editorconfig` · `.gitignore` · `config.toml.example` · `.github/workflows/ci.yml`.
- `ai` CLI 최소 골격(`src/main.rs`): clap 기반 `--version` / `doctor` 서브커맨드 (스켈레톤).
- `cargo build` / `cargo test` 검증 (개발 머신: Windows 11). Linux 전용 동작(PTY·샌드박스)은 추후 `#[cfg(target_os)]` 분기 + Linux CI에서 검증.

> 다음 단계: `docs/TASK.md` M1(W1) — Rust 워크스페이스/크레이트 구성 확정 및 5계층 아키텍처 합의.

---

## 채택된 핵심 설계 결정 (요약 — 정본은 설계 repo §0.2 / §30)

부트스트랩 시점에 확정되어 구현이 따르는 결정들. 상세 근거·대안은 정본 참조.

| 결정 | 채택안 | 정본 |
|---|---|---|
| 셸 통합 | **Hook 기반 기본 + Native Wrapper fallback** (rc 자동 수정 금지) | §29.1, §30-1, §31.1 |
| 저장 아키텍처 | **데몬 없음** — SQLite WAL `ai-terminal.db` + 파일 락 + stale cleanup | §30-2, §31.2 |
| 위험도 스케일 | **0~100 rule-based** (소가산 안 폐기), 로컬 정책 우선, AI는 보조 | §31.4 |
| 저장 DB 통일 | `history.db` → **`ai-terminal.db` 단일 스키마** | §0.2, §15.2 |
| 마스킹 | Secret/PII 기본 ON, **마스킹 실패 시 원격 AI 차단(fail-closed)** | §31.8 |
| 정책 프로파일 | **balanced(기본) + paranoid** 필수, poweruser/dev는 P2 | §31.3 |
| 자가 치유 | 자동 *분석/제안* 허용, 자동 *실행* 항상 금지 | §16.3 |
| 로컬 LLM | Phase 2로 이연 | §30-3 |
| 기술 스택 | **Rust** 1순위 (Go 대안) | §24.1 |

---

<!-- 새 항목 추가 시 이 위에 날짜 역순으로 기록. 형식:
## YYYY-MM-DD — <제목> (마일스톤)
- 변경/결정 요약 (왜 중심). 보안 관련은 위협/완화 명시.
-->
