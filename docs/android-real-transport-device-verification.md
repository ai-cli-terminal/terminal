# 실 transport 실기기 검증 (SM-F956N, openai)

이 슬라이스의 OkHttp JNI transport는 실기기에서만 실 openai 응답을 검증할 수 있다(CI 불가).

## 준비
1. DEBUG APK 빌드: `cd terminal/android && ./build-rust-jni.sh --profile release && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:assembleDebug`
2. 설치: `adb install -r app/build/outputs/apk/debug/app-debug.apk`
3. openai config 배치(실 키):
   `adb push ai-terminal-ai-config.json /sdcard/Download/ai-terminal-ai-config.json`
   내용: `{"provider":"openai","model":"gpt-4o-mini","openai_url":"https://api.openai.com","api_key":"sk-..."}`

## 확인 항목
- [ ] AI 토글 ON → 자연어 입력("큰 파일 찾아줘") → **openai 실 응답**(mock echo 아님).
- [ ] 응답이 제안으로만 표시되고 자동 실행되지 않음(§3-11).
- [ ] 네트워크 차단/키 오류 시 "unavailable"로 흡수, 셸은 계속 동작(§3-3).
- [ ] `adb logcat`에 api_key 평문 미노출(Debug redaction 확인).
- [ ] 세션 중 여러 줄 입력 시 핸들 재사용(재구성 로그 없음).

## 정리
- 검증 후 `adb shell rm /sdcard/Download/ai-terminal-ai-config.json`로 키 파일 제거.
