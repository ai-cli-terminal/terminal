# INSTALL — 플랫폼별 설치 안내

> 기준 버전: v0.3.4. 릴리즈는 Windows GUI `ai-terminal.exe` 자산과 CLI helper `ai`, 독립 셸 `ash`를 함께 제공한다.

## 1. 선택 기준

Windows 사용자는 먼저 실행 형태를 선택한다.

| 경로 | 실행되는 바이너리 | 선택 기준 |
|---|---|---|
| Windows GUI | `ai-terminal.exe` | 독립 창으로 뜨는 GUI 터미널이 필요한 경우 |
| Windows native | `ai.exe`, `ash.exe` | PowerShell/cmd, `.exe/.cmd/.bat/.ps1`, Windows PATH/PATHEXT와 함께 쓰려는 경우 |
| WSL | Linux용 `ai`, `ash` | Linux 유저랜드, bash/zsh hook, POSIX 도구, Linux/WSL 검증 경로가 필요한 경우 |

Windows GUI 릴리즈 자산은 `ai-terminal-windows-*.zip` 또는 `AI Terminal_*_x64-setup.exe`다. zip을 받는 경우 압축을 풀고 `ai-terminal.exe`를 실행한다. `ai-windows-x86_64.exe`는 GUI 앱이 아니라 CLI helper이므로 더블클릭용 실행파일이 아니다.

각 경로는 서로 다른 런타임이다. Windows native `ash.exe`는 PowerShell 호환 셸이 아니라 `ash` 문법 위에서 Windows 실행 대상을 호출한다. WSL의 `ash`는 distro 안의 Linux 바이너리이며 Windows native PATH/PATHEXT adapter를 사용하지 않는다.

Git Bash/MSYS는 세 번째 설치물이 아니라 Windows native 설치 위의 선택 profile이다. 기본값은 native `ash.exe` 동작이며, MSYS POSIX path/userland bridge는 명시 opt-in profile로만 다룬다.

## 2. Linux 또는 WSL

WSL에서는 먼저 원하는 distro 안의 셸을 열고 아래 명령을 실행한다.

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | bash
ai --version
ash
```

특정 버전을 고정하려면:

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | AI_VERSION=v0.3.4 bash
```

설치 후 PATH 안내가 나오면 셸을 다시 시작한다. Linux/WSL 경로는 bash/zsh hook과 POSIX PTY 검증 경로에 적합하다.

## 3. Windows Native

PowerShell에서 실행한다.

```powershell
irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
ai --version
ash
```

특정 버전을 고정하려면:

```powershell
$env:AI_VERSION = 'v0.3.4'
irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
```

설치 스크립트는 CLI helper `ai-windows-x86_64.exe`와 runtime shell `ash-windows-x86_64.exe`를 내려받아 설치한다. PATH 변경은 새 터미널 창부터 반영될 수 있다.

더블클릭 가능한 독립 GUI 터미널은 설치 스크립트가 아니라 릴리즈 자산에서 받는다. portable zip(`ai-terminal-windows-*.zip`)을 풀어 `ai-terminal.exe`를 실행하거나, NSIS installer(`AI.Terminal_*_x64-setup.exe`)를 실행한다. `ai-windows-x86_64.exe`는 GUI 앱이 아니라 CLI helper다.

## 4. Signed Binary Manifest Enforcement

설치 스크립트는 릴리스가 `binary-manifest.json`과 `binary-manifest.manifest.json`을 제공하면 함께 내려받는다. 기본 공개 설치는 기존 `.sha256` checksum 검증을 유지한다. 조직 배포나 업데이트에서 signed manifest를 강제하려면 다음을 함께 제공한다.

Linux/WSL:

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | \
  AI_TERMINAL_ORG_TRUST_ANCHOR=/path/to/org-root.json \
  AI_REQUIRE_SIGNED_MANIFEST=1 \
  AI_MANIFEST_VERIFIER=/path/to/trusted/ai \
  bash
```

Windows PowerShell:

```powershell
$env:AI_TERMINAL_ORG_TRUST_ANCHOR = 'C:\path\to\org-root.json'
$env:AI_REQUIRE_SIGNED_MANIFEST = '1'
$env:AI_MANIFEST_VERIFIER = 'C:\path\to\trusted\ai.exe'
irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
```

업데이트 경로에서는 `AI_MANIFEST_VERIFIER`를 생략해도 설치 디렉터리 또는 PATH의 기존 trust-enabled `ai`를 verifier로 사용할 수 있다. fresh install strict mode에서는 아직 신뢰된 `ai`가 없으므로 외부 verifier와 조직 trust anchor가 필요하다. 스크립트는 새로 내려받은 `ai`로 자기 자신을 검증하지 않는다.

서명 검증이 성공하면 설치 디렉터리에 `.ai-terminal-release-manifest-version`을 기록하고 이후 verified install에서 더 낮은 manifest version을 차단한다. 운영자가 별도 floor를 강제해야 하면 `AI_MIN_MANIFEST_VERSION=<n>`을 지정한다.

## 5. Windows Native Runtime Boundary

- `ash.exe`는 `.exe`를 직접 실행한다.
- `.cmd`/`.bat`는 `cmd.exe /d /c`를 통해 실행한다.
- `.ps1`은 PowerShell 실행 대상으로 실행한다.
- Windows ConPTY 동작은 CI에서 `cmd.exe` interactive smoke로 검증한다.
- bash/zsh hook은 Windows native 범위가 아니며, `ai doctor`는 wrapper fallback을 안내한다.

## 6. Git Bash/MSYS Profile

Git Bash나 MSYS2 터미널에서 `ash.exe`를 실행해도 기본 profile은 Windows native다.

```bash
ash
```

기본 profile의 의미:

- Windows PATH/PATHEXT를 따른다.
- `.cmd/.bat`는 `cmd.exe`, `.ps1`은 PowerShell host로 실행한다.
- `/usr/bin`, `/mingw64/bin`, `/c/Users/...` 같은 POSIX/MSYS path를 자동 변환하지 않는다.

MSYS bridge profile은 명시 opt-in으로만 켠다.

```bash
export AI_TERMINAL_WINDOWS_PROFILE=msys
ash
```

bridge profile은 `MSYSTEM` 또는 `MSYSTEM_PREFIX`가 있는 Git Bash/MSYS 환경에서만 유효하다. 이 profile은 MSYS path conversion과 POSIX tool discovery를 명시적으로 다루며, Windows native `.cmd/.ps1` adapter와 암묵적으로 섞지 않는다.

## 7. 소스 빌드

C-free 개발 빌드:

```bash
cargo build --release --features "remote trust"
```

SQLite 저장소와 TLS까지 포함하려면 C 툴체인이 필요하다.

```bash
cargo build --release --features "storage tls remote trust"
```

Windows 릴리즈 빌드는 CI에서 `storage remote trust` 조합으로 만든다. `tls`는 `ring`/`nasm` 요구 때문에 기본 Windows 릴리즈 조합에서 제외한다.

## 8. 검증

Linux/WSL:

```bash
cargo test --all-targets
printf '[{size: 50} {size: 200}] | where size > 100\nexit\n' | cargo run --bin ash
```

Windows native:

```powershell
pwsh scripts/smoke.ps1
```

릴리즈 파일을 직접 내려받는 경우 같은 이름의 `.sha256` 파일로 체크섬을 확인한다. `trust` feature 빌드는 조직 서명 `binary-manifest.json`을 `ai release manifest status/verify`로 검증할 수 있다. release workflow는 `AI_TERMINAL_RELEASE_SIGNING_KEY_HEX`/`AI_TERMINAL_RELEASE_KEY_ID` secret이 구성된 태그 릴리스에서 signed binary manifest asset을 함께 업로드할 수 있다.
