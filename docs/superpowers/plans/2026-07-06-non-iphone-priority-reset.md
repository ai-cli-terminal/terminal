# 2026-07-06 Non-iPhone Priority Reset

## Purpose

현재 host에서는 iPhone/iOS 구현, iOS/Xcode project 생성, TestFlight build evidence를
진행할 수 없다. PM-4 iOS/iPadOS track은 policy boundary, common mobile JSON bridge,
Rust C ABI surface까지 준비된 상태로 보존하고, 실제 SwiftUI REPL scaffold는
macOS/Xcode/iOS project 환경이 생길 때까지 TODO 대기열로 넘긴다.

## Decision

- Do not select iOS TestFlight SwiftUI `shellcore` REPL scaffold as the next
  active local slice on this host.
- Keep the iOS promise constrained: self-contained `shellcore`,
  app-private/document-picker workspace, policy-safe pure/builtin command subset,
  no Linux terminal/package-manager/userland claim.
- Continue release follow-up external evidence first when the required external
  operators/environments are available.
- If external blockers are not available, use only non-iPhone local work.

## Active Non-iPhone Priority

| Priority | Work | Done when |
|---|---|---|
| P1 external | Windows MSI evidence | Native Rust/MSVC/WiX host records successful MSI build, generated MSI, and SHA256 evidence. |
| P1 external | Android signing secrets | GitHub repository secret names match the workflow `AI_TERMINAL_ANDROID_*` references without recording secret values. |
| P1 external | F-Droid build/buildserver evidence | Evidence records `dev.aiterminal.android`, versionName `0.3.4`, versionCode `304`, successful result, and APK/buildserver artifact marker. |
| P2 local | Product packaging/docs | `ai`/`ash` role-name-version policy is documented, README platform support is split into current distribution vs target matrix, and the `document/` v3.3 -> `terminal/` pivot migration note exists. |
| P2 local | Mobile/PWA identity and copy separation | RA device identity is explicitly separate from mobile terminal body identity, and user-facing copy says "Mobile ash app = local terminal" plus "PWA companion = approve/pair/monitor/demo". |

## TODO Until iOS Host Exists

- TestFlight SwiftUI `shellcore` REPL scaffold.
- iOS app-private/document-picker workspace implementation evidence.
- iOS unknown/external command fail-closed runtime evidence.
- TestFlight beta review notes and build evidence.

## Verification

Use these gates for the active non-iPhone state:

```powershell
npm run check:release-followup
npm run export:release-followup-evidence-packet
npm run check:pwa-relay-managed-runtime-operator-setup-production-closeout
npm run check:pwa-relay-next-mode-planning
```
