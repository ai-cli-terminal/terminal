# Gateway 시맨틱 캐시 2차 조회 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**목표:** `Gateway::ask`가 exact 캐시 미스 후 `SemanticCache`를 2차 조회하고(히트 시 exact로 승격), 응답 출처(`CacheSource`)를 `ai ask`/`ai dispatch`까지 표시한다.

**Architecture:** `cache.rs`에 `CacheSource` enum 추가. `Gateway`에 `Mutex<SemanticCache>` 필드를 두고 `ask`에서 exact→semantic 순으로 조회, 시맨틱 히트는 exact에 승격 저장, 백엔드 응답은 양쪽에 저장한다. `source`를 `GatewayOutcome::Answered`→(responder)→`AiOutcome::Answered`로 흘려 CLI가 캐시 배지를 표시한다. in-memory 유지, threshold/TTL 하드코딩(exact 패턴과 일치).

**기술 스택:** Rust, tokio, 기존 `cache.rs`(Jaccard `similarity`/`SemanticCache`)·`gateway.rs`.

설계 정본: `docs/superpowers/specs/2026-06-03-gateway-semantic-cache-design.md`

빌드/검증 래퍼(WSL): `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
**주의: 종료코드는 `echo $?`로 못 잼(이 하니스에서 항상 0). `cmd && echo OK || echo FAIL`로만 확인.** 테스트는 result 라인으로 판정.

---

### 작업 1: `CacheSource` + Gateway 시맨틱 2차 조회 + `ai ask` 배지

**Files:**
- Modify: `src/cache.rs`(enum 추가), `src/gateway.rs`(필드+ask+source+테스트), `src/responder.rs`(컴파일 유지), `src/main.rs`(`ai ask` 배지 + 헬퍼)

- [ ] **단계 1: `CacheSource` enum 추가 (`src/cache.rs`)**

`src/cache.rs` 상단 `use` 아래, `ResponseCache` 정의 앞에 추가:

```rust
/// 응답 출처(캐시 계층 식별 — telemetry/표시용).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    /// 백엔드가 실제 생성.
    Backend,
    /// exact 캐시 히트.
    Exact,
    /// 시맨틱 캐시 히트(근사).
    Semantic,
}
```

- [ ] **단계 2: Gateway 시맨틱 히트/승격 테스트 작성 (실패 확인용) (`src/gateway.rs`)**

`gateway.rs`의 `mod tests` 안에 추가:

```rust
    #[tokio::test]
    async fn semantic_hit_then_promotes_to_exact() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Counting(Arc<AtomicUsize>);
        impl LlmBackend for Counting {
            fn generate<'a>(&'a self, prompt: &'a str) -> GenerateFuture<'a> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move { Ok(format!("r:{prompt}")) })
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let cap = crate::provider::Provider::mock().models[0].clone();
        let gw = Gateway::new(Box::new(Counting(calls.clone())), cap);

        // 1) 최초 → 백엔드 생성
        let o1 = gw.ask("alpha beta gamma", "").await.unwrap();
        assert!(
            matches!(o1, GatewayOutcome::Answered { source: CacheSource::Backend, .. }),
            "{o1:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // 2) 단어 재배열 → exact 미스, 시맨틱 히트(유사도 1.0) → 백엔드 호출 없음
        let o2 = gw.ask("gamma alpha beta", "").await.unwrap();
        assert!(
            matches!(o2, GatewayOutcome::Answered { source: CacheSource::Semantic, .. }),
            "{o2:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1, "semantic hit must not call backend");

        // 3) 같은 재배열 재요청 → 승격된 exact 히트
        let o3 = gw.ask("gamma alpha beta", "").await.unwrap();
        assert!(
            matches!(o3, GatewayOutcome::Answered { source: CacheSource::Exact, .. }),
            "{o3:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn source_is_backend_then_exact_on_repeat() {
        let gw = Gateway::mock();
        let a = gw.ask("repeat me", "").await.unwrap();
        assert!(matches!(a, GatewayOutcome::Answered { source: CacheSource::Backend, .. }), "{a:?}");
        let b = gw.ask("repeat me", "").await.unwrap();
        assert!(matches!(b, GatewayOutcome::Answered { source: CacheSource::Exact, .. }), "{b:?}");
    }
```

이 테스트는 `CacheSource`를 `gateway.rs`에서 참조하므로 `mod tests`에 `use crate::cache::CacheSource;`가 필요하다 — 다음 Step에서 본문 `use`로 들여온다.

- [ ] **단계 3: 테스트 실패 확인**

실행: `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test -p ai-terminal --lib gateway 2>&1 | tail -20'`
기대: 컴파일 에러(또는 FAIL) — `GatewayOutcome::Answered`에 `source` 필드 없음 / `CacheSource` 미해결.

- [ ] **단계 4: Gateway 구현 (`src/gateway.rs`)**

(a) `use` 교체: 기존 `use crate::cache::ResponseCache;` →
```rust
use crate::cache::{CacheSource, ResponseCache, SemanticCache};
```

(b) `GatewayOutcome::Answered`에 `source` 필드 추가:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayOutcome {
    Answered {
        text: String,
        input_tokens: usize,
        output_tokens: usize,
        source: CacheSource,
    },
    /// 마스킹 실패 등으로 원격 전송 차단.
    Blocked(String),
}
```

(c) `Gateway` 구조체에 `semantic` 필드 추가:
```rust
pub struct Gateway {
    backend: Box<dyn LlmBackend>,
    cap: ModelCapability,
    masker: Masker,
    cache: Mutex<ResponseCache>,
    semantic: Mutex<SemanticCache>,
}
```

(d) `new`에서 초기화(exact TTL 24h와 동일, 임계값 0.85):
```rust
    pub fn new(backend: Box<dyn LlmBackend>, cap: ModelCapability) -> Gateway {
        Gateway {
            backend,
            cap,
            masker: Masker::baseline(),
            cache: Mutex::new(ResponseCache::new(86_400)),
            semantic: Mutex::new(SemanticCache::new(86_400, 0.85)),
        }
    }
```

(e) `ask`의 캐시 단계(기존 "3) 캐시 조회" 블록부터 함수 끝의 `Ok(GatewayOutcome::Answered {...})`까지)를 다음으로 교체:
```rust
        // 3) exact 캐시 조회(§29.6) — 히트 시 백엔드 생략. 값 복제로 락 즉시 해제.
        let key = ResponseCache::key(&sent);
        let now = now_ms();
        let cached = self
            .cache
            .lock()
            .expect("cache mutex poisoned")
            .get(&key, now)
            .map(|s| s.to_string());
        if let Some(text) = cached {
            return Ok(Self::answered(text, input_tokens, CacheSource::Exact));
        }

        // 3b) 시맨틱 2차 조회 — 임계값 이상 유사 시 히트, 그 답을 exact 캐시에 승격 저장.
        let similar = self
            .semantic
            .lock()
            .expect("semantic cache mutex poisoned")
            .get_similar(&sent, now)
            .map(|s| s.to_string());
        if let Some(text) = similar {
            self.cache
                .lock()
                .expect("cache mutex poisoned")
                .put(key, text.clone(), now);
            return Ok(Self::answered(text, input_tokens, CacheSource::Semantic));
        }

        // 4) 백엔드 생성 + exact·semantic 양쪽 저장.
        let text = self.backend.generate(&sent).await?;
        self.cache
            .lock()
            .expect("cache mutex poisoned")
            .put(key, text.clone(), now);
        self.semantic
            .lock()
            .expect("semantic cache mutex poisoned")
            .put(sent, text.clone(), now);
        Ok(Self::answered(text, input_tokens, CacheSource::Backend))
```

(f) `ask` 함수 바로 아래(같은 `impl Gateway` 블록 안)에 출력 토큰 산출 + 생성 헬퍼 추가:
```rust
    /// 응답 텍스트로 `Answered`를 구성한다(출력 토큰 추정 공통화).
    fn answered(text: String, input_tokens: usize, source: CacheSource) -> GatewayOutcome {
        let output_tokens = tokenwin::estimate_tokens(&text);
        GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
            source,
        }
    }
```

(g) `mod tests` 최상단에 `use crate::cache::CacheSource;` 추가(테스트가 `CacheSource` 참조).

- [ ] **단계 5: responder.rs 컴파일 유지 (source는 아직 미전파) (`src/responder.rs`)**

`finish`의 `GatewayOutcome::Answered` 패턴이 비-`..` 구조분해라 필드 추가로 깨진다. `source`를 바인딩하되 이 Task에선 무시(다음 Task에서 AiOutcome로 전파):
```rust
        Ok(GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
            source: _,
        }) => {
            sink.write(&text);
            AiOutcome::Answered {
                text,
                input_tokens,
                output_tokens,
            }
        }
```
그리고 `finish_answered_writes_to_sink` 테스트의 `GatewayOutcome::Answered {...}` 생성에 `source` 추가(테스트 `mod tests`에 `use crate::cache::CacheSource;` 추가):
```rust
            Ok(GatewayOutcome::Answered {
                text: "hello".into(),
                input_tokens: 3,
                output_tokens: 4,
                source: CacheSource::Backend,
            }),
```

- [ ] **단계 6: `ai ask` 배지 + `cache_badge` 헬퍼 (`src/main.rs`)**

(a) `ai ask` Answered arm(약 743행)의 비-`..` 구조분해에 `source` 추가하고 배지 출력:
```rust
                Ok(gateway::GatewayOutcome::Answered {
                    text,
                    input_tokens,
                    output_tokens,
                    source,
                }) => {
                    println!("{text}");
                    println!(
                        "(tokens ~ in:{input_tokens} out:{output_tokens}){}",
                        cache_badge(source)
                    );
                    #[cfg(feature = "storage")]
                    if let Ok(store) = ai_terminal::store::Store::open_default() {
                        let _ = store.record_usage(
                            "mock",
                            "mock-model",
                            input_tokens as i64,
                            output_tokens as i64,
                            0,
                            0.0,
                            None,
                        );
                    }
                }
```

(b) `cache_badge` 헬퍼를 `main.rs`에 추가(예: `run_doctor` 앞 적당한 위치):
```rust
/// 캐시 출처 배지(Backend는 무배지). `ai ask`·`ai dispatch` 공용.
fn cache_badge(source: ai_terminal::cache::CacheSource) -> &'static str {
    use ai_terminal::cache::CacheSource;
    match source {
        CacheSource::Backend => "",
        CacheSource::Exact => " [cache: exact]",
        CacheSource::Semantic => " [cache: semantic ~근사]",
    }
}
```

(c) `mod tests`에 헬퍼 단위 테스트 추가:
```rust
    #[test]
    fn cache_badge_labels() {
        use ai_terminal::cache::CacheSource;
        assert_eq!(cache_badge(CacheSource::Backend), "");
        assert!(cache_badge(CacheSource::Exact).contains("exact"));
        assert!(cache_badge(CacheSource::Semantic).contains("semantic"));
    }
```

- [ ] **단계 7: 빌드·clippy·fmt·테스트(기본 + storage)**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | grep -E "test result|error" | grep -v "0 passed"'
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features storage -- -D warnings && cargo test --features storage 2>&1 | grep -E "test result|error" | grep -v "0 passed"'
```
기대: clippy/fmt clean, 모든 테스트 PASS(신규 gateway 2개 + cache_badge 1개 포함).

- [ ] **단계 8: 커밋**

```
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add src/cache.rs src/gateway.rs src/responder.rs src/main.rs && git commit -m "feat(gateway): semantic cache secondary lookup with promotion and CacheSource

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### 작업 2: source를 `AiOutcome`까지 전파 + `ai dispatch` 배지

**Files:**
- Modify: `src/dispatch.rs`(AiOutcome 필드+테스트), `src/responder.rs`(finish 전파+테스트), `src/main.rs`(`ai dispatch` 배지)

- [ ] **단계 1: `AiOutcome::Answered`에 `source` 추가 (`src/dispatch.rs`)**

(a) 파일 상단 `use`에 추가: `use crate::cache::CacheSource;`

(b) `AiOutcome::Answered`에 필드 추가:
```rust
pub enum AiOutcome {
    /// 응답 성공(text는 sink에도 기록됨).
    Answered {
        text: String,
        input_tokens: usize,
        output_tokens: usize,
        source: CacheSource,
    },
    /// 마스킹 fail-closed 등으로 원격 전송 차단.
    Blocked(String),
    /// 장애·타임아웃·취소(§3-3: 셸을 막지 않음).
    Unavailable(String),
}
```

(c) `mod tests`의 `MockAi::respond`가 반환하는 `AiOutcome::Answered`에 `source: CacheSource::Backend` 추가:
```rust
            Ok(AiOutcome::Answered {
                text: self.answer.clone(),
                input_tokens: 1,
                output_tokens: 2,
                source: CacheSource::Backend,
            })
```
(`mod tests`에 `use crate::cache::CacheSource;` 추가.)

(d) `natural_language_routes_to_ai` 테스트의 정확 일치 비교에 `source` 추가:
```rust
        assert_eq!(
            out,
            Handled::Ai(AiOutcome::Answered {
                text: "answer-text".into(),
                input_tokens: 1,
                output_tokens: 2,
                source: CacheSource::Backend,
            })
        );
```
(`ai_inline_routes_to_ai`는 `AiOutcome::Answered { .. }` 매칭이라 변경 불필요.)

- [ ] **단계 2: `finish`가 source 전파 (`src/responder.rs`)**

`finish`의 패턴에서 `source: _`를 `source`로 바꾸고 AiOutcome로 전달:
```rust
        Ok(GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
            source,
        }) => {
            sink.write(&text);
            AiOutcome::Answered {
                text,
                input_tokens,
                output_tokens,
                source,
            }
        }
```
`finish_answered_writes_to_sink` 테스트의 기대 `AiOutcome::Answered {...}`에 `source: CacheSource::Backend` 추가(입력 GatewayOutcome도 Backend이므로 일치):
```rust
        assert_eq!(
            out,
            AiOutcome::Answered {
                text: "hello".into(),
                input_tokens: 3,
                output_tokens: 4,
                source: CacheSource::Backend,
            }
        );
```

- [ ] **단계 3: `ai dispatch` 배지 (`src/main.rs`)**

`run_dispatch`의 `Handled::Ai(AiOutcome::Answered { .. })` arm을 `source` 바인딩 + 배지 출력으로 수정:
```rust
        Handled::Ai(AiOutcome::Answered {
            input_tokens,
            output_tokens,
            source,
            ..
        }) => {
            #[cfg(feature = "storage")]
            if let Ok(store) = ai_terminal::store::Store::open_default() {
                let _ = store.record_usage(
                    "mock",
                    "mock-model",
                    input_tokens as i64,
                    output_tokens as i64,
                    0,
                    0.0,
                    None,
                );
            }
            println!(
                "\n(tokens ~ in:{input_tokens} out:{output_tokens}){}",
                cache_badge(source)
            );
            Ok(())
        }
```
(`cache_badge`는 작업 1에서 추가됨. `ui.rs`의 `render_output`은 `AiOutcome::Answered { .. }` 매칭이라 변경 불필요.)

- [ ] **단계 4: 빌드·clippy·fmt·테스트(기본 + storage)**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test 2>&1 | grep -E "test result|error" | grep -v "0 passed"'
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features storage -- -D warnings && cargo test --features storage 2>&1 | grep -E "test result|error" | grep -v "0 passed"'
```
기대: clippy/fmt clean, 모든 테스트 PASS.

- [ ] **단계 5: 커밋**

```
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add src/dispatch.rs src/responder.rs src/main.rs && git commit -m "feat(gateway): propagate CacheSource through AiOutcome and show cache badge in dispatch

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### 작업 3: e2e 확인 + 문서 갱신

**Files:**
- Modify: `docs/TASK.md`, `docs/HISTORY.md`

- [ ] **단계 1: e2e — `ai ask` 동일 프롬프트 2회 시 exact 배지**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build 2>&1 | tail -1; BIN=$HOME/targets/ai-terminal/debug/ai; $BIN ask "hello world test" 2>&1 | tail -2; echo "--- 2nd (same process? no — separate, cache is in-memory per process) ---"'
```
참고: 캐시는 프로세스 내 in-memory라 별도 프로세스 호출 간에는 공유되지 않는다. 따라서 단위 테스트(작업 1)가 캐시 동작의 정본 검증이며, e2e는 `ai ask`가 배지 포함 정상 출력되는지(회귀 없음)만 확인한다. 기대: 답변 + `(tokens ~ ...)` 라인 정상 출력(첫 호출이므로 무배지).

- [ ] **단계 2: `docs/TASK.md` 갱신**

"P2 나머지" 섹션의 `- gateway에 시맨틱 캐시 2차 조회 결합` 줄을 완료 표기로 교체:
```
- [x] gateway 시맨틱 캐시 2차 조회 결합 (2026-06-03): exact 미스 → `SemanticCache::get_similar`(임계값 0.85) 2차 조회, 히트 시 exact 승격. `CacheSource`(Backend/Exact/Semantic) 플래그를 `ai ask`/`ai dispatch` 배지로 표시. 설계/계획: `docs/superpowers/{specs,plans}/2026-06-03-gateway-semantic-cache*`
```

- [ ] **단계 3: `docs/HISTORY.md` 엔트리 추가**

먼저 `docs/HISTORY.md` 최신 엔트리 형식을 확인한 뒤, 최상단(newest-first)에 추가:
```markdown
## 2026-06-03 — Gateway 시맨틱 캐시 2차 조회

- **gateway**(`gateway.rs`): `ask`가 exact 캐시 미스 후 `SemanticCache::get_similar`(TTL 24h, Jaccard 임계값 0.85)를 2차 조회. 시맨틱 히트는 그 답을 exact 캐시에 승격 저장(다음 동일 프롬프트는 exact 히트). 백엔드 응답은 exact+semantic 양쪽 저장. 시맨틱 키도 마스킹된 텍스트(RULES §2).
- **cache source 플래그**(`cache.rs`): `CacheSource { Backend, Exact, Semantic }`를 `GatewayOutcome::Answered`→`AiOutcome::Answered`로 전파. `ai ask`/`ai dispatch`가 캐시 히트 시 배지(`[cache: exact]`/`[cache: semantic ~근사]`) 표시.
- 검증: gateway 단위 테스트(시맨틱 히트→exact 승격, source 계층 반영), `cache_badge` 라벨 테스트. clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-gateway-semantic-cache-design.md`, `docs/superpowers/plans/2026-06-03-gateway-semantic-cache.md`.
```
(HISTORY.md 기존 스타일에 맞게 보정.)

- [ ] **단계 4: 커밋**

```
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add docs/TASK.md docs/HISTORY.md && git commit -m "docs: record gateway semantic cache secondary lookup

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

## 완료 기준 (완료 기준)

- `Gateway::ask`가 exact 미스 → 시맨틱 2차 조회 → 히트 시 exact 승격 → 둘 다 미스면 백엔드(양쪽 저장).
- `CacheSource`가 `ai ask`/`ai dispatch`에서 배지로 표시(Backend 무배지).
- 단위 테스트: 시맨틱 히트(백엔드 1회)·exact 승격·source 계층·배지 라벨.
- clippy/fmt clean, 기본 + storage 전체 테스트 PASS.
- 문서(TASK/HISTORY) 갱신.
- 비목표(config 노출, 임베딩, 영속화, TUI 배지)는 미포함.
