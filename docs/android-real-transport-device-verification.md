# 실 transport 실기기 검증 (SM-F956N, openai)

이 슬라이스의 OkHttp JNI transport는 실기기에서만 실 openai 응답을 검증할 수 있다(CI 불가).

## config 경로 (2026-08-15 정정)

DEBUG 빌드는 다음 순서로 config를 찾는다.

1. `/sdcard/Android/data/dev.aiterminal.android/files/ai-terminal-ai-config.json` — **앱 전용 외부 디렉터리**
2. `/sdcard/Download/ai-terminal-ai-config.json` — 레거시 폴백

targetSdk 35 scoped storage 에서는 앱이 스토리지 권한 없이 `/sdcard/Download` 를 읽을 수 없다
(이 앱의 매니페스트에는 스토리지 권한 선언이 없다). 그래서 레거시 경로만 쓰면 `file.canRead()` 가
항상 false 가 되어 **조용히 mock 으로 폴백**한다. 2026-08-15 실기기 검증에서 이 문제가 드러나
앱 전용 외부 디렉터리를 우선 경로로 추가했다. 이 위치는 다른 앱에서 보이지 않으므로 키 노출
면에서도 공유 Download 폴더보다 안전하다.

## 준비

1. JNI 빌드(4 ABI): `bash android/build-rust-jni.sh --profile release`
   - Windows 호스트에서도 동작한다(호스트 NDK 툴체인 자동 탐지).
   - `ANDROID_HOME` 을 SDK 경로로 지정한다. 예: `ANDROID_HOME=/c/Users/<user>/AppData/Local/Android/Sdk`
2. APK 빌드: `gradlew :app:assembleDebug` (`ANDROID_HOME`, JDK 21 필요)
3. 설치: `adb install -r android/app/build/outputs/apk/debug/app-debug.apk`
4. 앱을 한 번 실행해 앱 전용 외부 디렉터리를 만든다.
5. config 배치(실 키):
   `adb push ai-terminal-ai-config.json /sdcard/Android/data/dev.aiterminal.android/files/ai-terminal-ai-config.json`
   내용: `{"provider":"openai","model":"gpt-4o-mini","openai_url":"https://api.openai.com","api_key":"sk-..."}`
6. 앱을 재시작하고 AI 토글을 켠다. 전사에 `AI assist enabled (openai provider; ...)` 가 떠야 한다.
   `mock provider` 로 뜨면 config 를 읽지 못한 것이다. 다음으로 확인한다:
   `adb shell "run-as dev.aiterminal.android sh -c 'test -r <경로> && echo READABLE || echo NOT_READABLE'"`

## 입력 라우팅 주의

자연어 판정은 `src/intent.rs` 의 분류기를 탄다. `list files sorted by size` 처럼 명령처럼 보이는
입력은 **셸로 라우팅**되어 AI 를 타지 않는다(`external execution disabled: list`). 물음표로 끝나는
문장이나 한국어 자연어를 쓴다. 예: `how do I list files by size?`, `큰 파일 찾아줘`

## 확인 항목

- [ ] AI 토글 ON → 자연어 입력 → **openai 실 응답**(mock echo 아님).
- [x] 응답이 제안으로만 표시되고 자동 실행되지 않음(§3-11).
- [x] 네트워크 차단/키 오류 시 "unavailable"로 흡수, 셸은 계속 동작(§3-3).
- [x] `adb logcat`에 api_key 평문 미노출(Debug redaction 확인).
- [ ] 세션 중 여러 줄 입력 시 핸들 재사용(재구성 로그 없음).

## 2026-08-15 검증 결과

`SM-F956N`(Android 16)에서 실행했다.

**검증된 것**

- 앱 전용 외부 디렉터리 config 적재 → `openai provider` 활성(전사 메시지로 확인).
- JNI → OkHttp → `https://api.openai.com/v1/chat/completions` 실제 요청 도달, 실제 HTTP 응답 수신.
- §3-11: AI 경로에서 어떤 명령도 자동 실행되지 않음.
- §3-3 fail-soft: `error: ai unavailable ...` 표시 후 곧바로
  `[{size: 50} {size: 200}] | where size > 100` 이 정상 실행되어 `200` 반환.
- api_key 평문이 `adb logcat` 에 나타나지 않음(키 접두사·`sk-` 패턴 모두 미검출).

**미검증(외부 사유)**

- **실 openai 성공 응답**과 **핸들 재사용**은 확인하지 못했다. 사용한 키가
  `HTTP 429 insufficient_quota`(quota 소진)를 반환해 성공 경로에 진입할 수 없었다. 앱 결함이 아니라
  키의 결제/quota 문제이며, PC 에서 동일 키로 같은 엔드포인트를 호출해 같은 429 를 재현해 확인했다.
- quota 가 있는 키를 넣으면 위 두 항목만 다시 보면 된다. 나머지 경로는 이미 실기기에서 통과했다.

## 정리

- 검증 후 키 파일 제거:
  `adb shell rm /sdcard/Android/data/dev.aiterminal.android/files/ai-terminal-ai-config.json`
- PC 원본도 함께 지운다. 키를 shell history·문서·evidence·스크린샷에 남기지 않는다.
