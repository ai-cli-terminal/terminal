# INSTALL — 플랫폼별 설치 안내

> 기준 버전: v0.2.4 이상. 릴리즈는 `ai`와 독립 셸 `ash`를 함께 제공한다.

## 1. 선택 기준

Windows 사용자는 두 경로 중 하나를 선택한다.

| 경로 | 실행되는 바이너리 | 선택 기준 |
|---|---|---|
| Windows native | `ai.exe`, `ash.exe` | PowerShell/cmd, `.exe/.cmd/.bat/.ps1`, Windows PATH/PATHEXT와 함께 쓰려는 경우 |
| WSL | Linux용 `ai`, `ash` | Linux 유저랜드, bash/zsh hook, POSIX 도구, Linux/WSL 검증 경로가 필요한 경우 |

두 경로는 서로 다른 런타임이다. Windows native `ash.exe`는 PowerShell 호환 셸이 아니라 `ash` 문법 위에서 Windows 실행 대상을 호출한다. WSL의 `ash`는 distro 안의 Linux 바이너리이며 Windows native PATH/PATHEXT adapter를 사용하지 않는다.

## 2. Linux 또는 WSL

WSL에서는 먼저 원하는 distro 안의 셸을 열고 아래 명령을 실행한다.

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | bash
ai --version
ash
```

특정 버전을 고정하려면:

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | AI_VERSION=v0.2.4 bash
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
$env:AI_VERSION = 'v0.2.4'
irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
```

설치 스크립트는 `ai-windows-x86_64.exe`와 `ash-windows-x86_64.exe`를 내려받아 설치한다. PATH 변경은 새 터미널 창부터 반영될 수 있다.

Windows native 경로의 의미:

- `ash.exe`는 `.exe`를 직접 실행한다.
- `.cmd`/`.bat`는 `cmd.exe /d /c`를 통해 실행한다.
- `.ps1`은 PowerShell 실행 대상으로 실행한다.
- Windows ConPTY 동작은 CI에서 `cmd.exe` interactive smoke로 검증한다.
- bash/zsh hook은 Windows native 범위가 아니며, `ai doctor`는 wrapper fallback을 안내한다.

## 4. 소스 빌드

C-free 개발 빌드:

```bash
cargo build --release --features remote
```

SQLite 저장소와 TLS까지 포함하려면 C 툴체인이 필요하다.

```bash
cargo build --release --features "storage tls remote"
```

Windows 릴리즈 빌드는 CI에서 `storage remote` 조합으로 만든다. `tls`는 `ring`/`nasm` 요구 때문에 기본 Windows 릴리즈 조합에서 제외한다.

## 5. 검증

Linux/WSL:

```bash
cargo test --all-targets
printf '[{size: 50} {size: 200}] | where size > 100\nexit\n' | cargo run --bin ash
```

Windows native:

```powershell
pwsh scripts/smoke.ps1
```

릴리즈 파일을 직접 내려받는 경우 같은 이름의 `.sha256` 파일로 체크섬을 확인한다. 서명 검증은 Phase 3 트러스트 채널에서 도입할 예정이다.
