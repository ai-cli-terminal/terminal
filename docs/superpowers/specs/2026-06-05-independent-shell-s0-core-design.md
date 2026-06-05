# 독립 AI-네이티브 셸 — 피벗 + S0 셸 코어 MVP 설계

> **작성일**: 2026-06-05 · 브레인스토밍 산출.
> **결정**: 프로젝트를 "bash 위 AI 보조 레이어"에서 **자체 명령 언어를 가진 독립 구조화 셸**(PowerShell/Nushell 계열)로 재정의한다. 기존 안전/AI 서브시스템은 `ai_terminal` 라이브러리로 재사용한다.
> **근거**: 원안(독립 셸)과 현 구조(`document/docs/00-overview-architecture.md` §1·§3 "셸 호환성 우선, AI는 보조자, bash 래핑")의 괴리를 사용자 확인으로 정렬 → 방향 A(독립 셸 재정의) 채택.
> **상태**: 계획. 본 스펙은 **S0(셸 코어 MVP)**를 구현 가능 수준으로 명세한다. S1~S4는 개요만.

---

## 0. 피벗 개요 + 단계 분해

### 0.1 정체성
Nushell 계열 **독립 구조화 셸** + AI/안전이 1급 빌트인. bash/zsh를 감싸지 않는다 — **이 셸이 곧 셸**이며, 외부 리눅스 바이너리는 직접 spawn한다. 명령은 텍스트가 아니라 **구조화 데이터(레코드/테이블)**를 파이프라인으로 흘린다.

### 0.2 재사용 경계
- ♻️ **lib로 재사용**(명령-비의존): `risk`·`mask`·`policy`·`preview`/`diff`/`sandbox`·`undo`·`usage`·`store`(audit)·`gateway`+provider(`ollama`/`openai`/`http`)·`intent`/`verify`·원격승인 크립토(`remote`/`approval`/`gate`/`session`/`daemon`)·`pty`. 기존 `src/lib.rs`(`ai_terminal`)가 그대로 제공.
- ⚠️ **레거시 분리**(독립 셸엔 불필요): `shell.rs`의 bash/zsh hook 주입, `ai shell`의 bash 래핑(`wrapper.rs` probe), hook 기반 컨텍스트 동기화(독립 셸은 컨텍스트를 네이티브 보유).
- 기존 `ai` 바이너리(서브커맨드 도구)는 당분간 유지. 새 셸은 **새 바이너리**.

### 0.3 단계 (각 단계 = 그 자체로 쓸 수 있는 산출물)
| 단계 | 내용 |
|------|------|
| **S0 — 셸 코어 MVP**(본 스펙) | 값 모델 + 렉서 + 파서 + 평가기 + 핵심 빌트인 7종 + 외부 실행 + 최소 REPL |
| S1 — 구조화 데이터 명령 | 연산자/불리언식 + `where`/`each`/`sort-by`/`select`/`range` + 텍스트→구조화 파서(`from json` 등) + 외부 stdout 캡처 |
| S2 — 셸 에르고노믹스 | 제어흐름(if/for) · 문자열 보간 · 리다이렉트 · 라인에디터/히스토리/보완 · config |
| S3 — AI/안전 빌트인 | `ai_terminal` lib 결선: 외부 실행 전 risk 게이트 · `ai`(NL→cmd) · mask · preview/undo · policy 프로파일 |
| S4 — 고급 | 원격 승인(기존 RA 재배치) · 잡 컨트롤 · 스킬/MCP · 플러그인 |

---

## 1. S0 — 셸 코어 MVP

### 1.1 값 모델 (`value.rs`)
구조화 파이프라인의 최소 충분 집합:
```
Value =
  | Nothing
  | Bool(bool)
  | Int(i64)
  | Float(f64)
  | String(String)
  | List(Vec<Value>)
  | Record(OrderedMap<String, Value>)   // 순서 보존(자체 경량 구현)
```
- **테이블 = `List`(원소가 모두 `Record`)**. 별도 타입 아님; 포매터가 표로 렌더.
- 순서 보존 레코드: 자체 경량 순서맵(`Vec<(String,Value)>` 래퍼)로 구현 — 새 의존성(indexmap) 회피, C-free 유지.

### 1.2 문법 (S0 범위) (`lexer.rs`, `ast.rs`, `parser.rs`)
지원:
- **파이프라인**: `expr | cmd | cmd` — 좌측 값이 우측 명령의 입력(`$in`).
- **명령 호출**: `name arg…` — 인자는 표현식. 빌트인 아니면 외부 명령.
- **리터럴**: 정수, 실수, `"..."`/`'...'`(보간 없음), `true`/`false`, `null`, 리스트 `[a b c]`(공백/쉼표 구분), 레코드 `{k: v, k2: v2}`.
- **변수**: `let name = expr` 바인딩, `$name` 참조. `$in`=파이프라인 입력.
- **구분자**: 개행 또는 `;`. 주석 `# …`.

문법(개념 EBNF):
```
program   = (pipeline (NEWLINE | ';'))*
statement = 'let' IDENT '=' pipeline | pipeline
pipeline  = command ('|' command)*
command   = WORD expr*                 # WORD=빌트인명 또는 외부명
expr      = literal | '$' IDENT | list | record | '(' pipeline ')'
list      = '[' (expr (','? expr)*)? ']'
record    = '{' (IDENT ':' expr (','? )?)* '}'
literal   = INT | FLOAT | STRING | 'true' | 'false' | 'null'
```
**S0 제외**: 연산자(`+ - == > and …`)·불리언식·`where`/`each`/`sort`(→S1), 제어흐름·문자열 보간·리다이렉트(→S2). `where`는 조건식이 필요하므로 S1; S0는 연산자 없는 `get`/`first`/`length`로 구조화 파이프라인을 증명한다.

### 1.3 빌트인 (S0 최소 7종) (`builtins.rs`)
| 빌트인 | 시그니처 | 동작 |
|--------|----------|------|
| `print`/`echo` | `print <value>` | 인자(또는 `$in`)를 포매터로 출력 → `Nothing` |
| `cd` | `cd [path]` | 셸 cwd 변경(기본=홈). 경로 없거나 부재 시 오류. ls·외부 spawn cwd에 반영 |
| `exit` | `exit [code=0]` | REPL 종료(코드 반환) |
| `ls` | `ls [path=.]` | 디렉터리 항목을 `{name:String, type:String, size:Int}` 레코드 리스트(=테이블)로 생성. type∈{file,dir,symlink} |
| `get` | `get <field>` | `$in`이 Record면 필드값; 테이블(Record 리스트)이면 컬럼(List). 없는 필드=오류 |
| `first` | `first [n=1]` | `$in` 리스트의 앞 n개(리스트 반환; n=1이어도 리스트). 리스트 아니면 오류 |
| `length` | `length` | `$in` 리스트 항목 수(Int). 리스트 아니면 오류 |

### 1.4 외부 실행 (`external.rs`)
- 빌트인 아닌 명령 이름 → PATH에서 외부 바이너리를 셸 cwd·현재 env로 spawn.
- 인자: 각 인자 표현식 평가 → 문자열로 강제.
- **stdio 상속(inherited)**: stdout/stderr가 터미널로 라이브 통과 → **대화형 프로그램(vim/git/less) 정상 동작**. 반환 = `Nothing` + 종료코드(비0이면 안내 표시, REPL은 지속).
- **S0 제외**: 외부 stdout을 값으로 캡처(`git status | ...`)·구조화 파싱·외부에 `$in` 주입 → 텍스트↔구조화 브리지가 필요하므로 S1.
- 미존재 명령: "command not found: <name>" 오류(REPL 지속).

### 1.5 엔진/스코프 (`engine.rs`)
- `Engine { cwd: PathBuf, vars: OrderedMap<String, Value>, env }`.
- `eval_pipeline`: 좌→우로 `Value`를 흘린다. 각 단계 입력 `$in`(첫 단계는 `Nothing` 또는 좌측 expr 값). 명령은 빌트인 레지스트리 디스패치 또는 외부.
- `eval_line(src: &str, engine: &mut Engine) -> Result<Value>`: 렉스→파스→평가. **이 함수가 테스트 진입점**(REPL과 분리).
- 최상위 결과가 `Nothing`이 아니면 포매터로 자동 출력.

### 1.6 포매터 (`format.rs`)
- 스칼라(Bool/Int/Float/String/Nothing) → 한 줄 문자열.
- List(스칼라) → 인덱스 2열 표. Record → key/value 2열 표. 테이블(Record 리스트) → index + 헤더 정렬 표.
- MVP는 정렬 ASCII 표(박스 드로잉은 후속 미감).

### 1.7 REPL (`repl.rs`)
- 프롬프트: cwd(홈은 `~`로 축약) + 구분 기호.
- 입력: **std stdin `read_line`**(MVP — 라인에디터/히스토리/보완은 S2).
- `eval_line` → 결과 포맷 출력. 파스/런타임/외부 오류는 명확히 출력하고 루프 지속(크래시 금지).
- EOF(Ctrl-D) → 종료. Ctrl-C → 현재 입력 취소(베스트-에포트).
- **AI/안전 게이트 없음**(S3) — S0는 명령을 바로 실행.

### 1.8 파일 구조 (순수 Rust, C-free, 새 의존성 0)
```
src/shellcore/mod.rs       — 모듈 루트 + 재노출
src/shellcore/value.rs     — Value + OrderedMap + 헬퍼
src/shellcore/lexer.rs     — Token + 토큰화
src/shellcore/ast.rs       — Pipeline/Command/Expr/Stmt
src/shellcore/parser.rs    — 토큰 → AST
src/shellcore/engine.rs    — Engine/scope + eval_pipeline + eval_line
src/shellcore/builtins.rs  — 빌트인 레지스트리 + 7종 구현
src/shellcore/external.rs  — 외부 spawn(stdio 상속)
src/shellcore/format.rs    — 값 렌더
src/shellcore/repl.rs      — REPL 루프
src/lib.rs                 — `pub mod shellcore;`
src/bin/ash.rs             — `fn main(){ shellcore::repl::run() }`
Cargo.toml                 — [[bin]] name="ash" path="src/bin/ash.rs"
```
바이너리명 `ash`(AI SHell)는 가칭 — 확정 시 변경.

### 1.9 테스트 (TDD)
- **순수 단위**: lexer(파이프/리터럴/리스트/레코드/let/$var/주석/외부명 토큰화), parser(AST: 파이프라인·명령+인자·let·리스트·레코드), engine(파이프라인 스레딩·let/$var·`get`/`first`/`length` 인메모리 테이블), value/format(렌더).
- **통합**: `ls`(temp dir에서 name/type/size), external(`echo`/`true` spawn 종료코드 — unix), `eval_line` 엔드투엔드(`"ls | get name | first 3"` 류, temp dir).
- REPL 루프(I/O)는 비대상 — 로직은 `eval_line`로 분리해 검증.
- 검증: default 빌드(C-free) `cargo test`, fmt/clippy `-D warnings` clean. (lib feature 매트릭스 무회귀.)

### 1.10 S0 비목표 (명시)
연산자/`where`/`each`/`sort`(S1) · 외부 stdout→구조화 캡처(S1) · 제어흐름/문자열보간/리다이렉트/라인에디터/히스토리/보완/config(S2) · AI/안전 게이트(S3) · 잡 컨트롤/플러그인/스킬/MCP/원격(S4) · 박스드로잉 표 미감.

---

## 2. 의존성·시퀀싱

```
S0(코어: lex→parse→eval→value→external→repl)
  └─► S1(연산자+데이터명령+텍스트브리지) ─► S2(에르고노믹스) ─► S3(AI/안전 lib 결선) ─► S4(고급)
```
- S0는 기존 lib에 거의 비의존(값/언어 코어). AI/안전 결선은 S3에서.
- 기존 Phase 3 로드맵(R0/RA/P3, "bash 위 보조" 전제)은 본 피벗으로 **재정렬 대상**: R0 릴리즈 도구는 새 `ash` 바이너리에 재사용, RA/원격승인은 S4로 재배치, P3-1~3(조직정책/감사/MCP)은 셸 위 기능으로 후속 재평가. (별도 정리.)
