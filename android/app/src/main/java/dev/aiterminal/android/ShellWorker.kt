package dev.aiterminal.android

import android.os.Handler
import android.os.Looper
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicReference

fun interface ResultPoster {
    fun post(block: () -> Unit)
}

class ShellWorker(
    private val bridge: ShellBridge,
    externalAdapter: ExternalShellStreamAdapter? = null,
    private val executor: ExecutorService = Executors.newSingleThreadExecutor(),
    private val resultPoster: ResultPoster = mainThreadPoster(),
) {
    private val externalAdapterRef = AtomicReference(externalAdapter)

    @Volatile
    var externalCommandsEnabled: Boolean = false

    // aiHandle은 executor 스레드에서만 접근(직렬화 → race 없음).
    // currentAiConfig는 @Volatile로 동기 저장 — 호출 스레드가 setter 직후 즉시
    // readback해야 한다(TerminalViewModel.toggleAi가 worker.aiConfig를 동기 설정 후
    // 곧바로 읽는 계약). 핸들 create/destroy만 executor로 직렬화한다.
    private var aiHandle: Long = 0L

    @Volatile
    private var currentAiConfig: ShellAiConfig? = null

    /** 설정되면 pure eval이 AI 보조 경로로 간다. null이면 기존 그대로. */
    var aiConfig: ShellAiConfig?
        get() = currentAiConfig
        set(value) {
            currentAiConfig = value // 동기 저장(즉시 readback)
            executor.execute { reconfigureAiHandle(value) } // 핸들 조작만 직렬화
        }

    // executor 스레드 전용. 기존 핸들 파괴 후 새 config로 재생성.
    private fun reconfigureAiHandle(next: ShellAiConfig?) {
        if (aiHandle != 0L) {
            bridge.destroyAi(aiHandle)
            aiHandle = 0L
        }
        if (next != null) {
            aiHandle = bridge.createAi(next)
        }
    }

    fun submit(input: String, state: ShellState, onResult: (ShellEvalResult) -> Unit) {
        submitStreaming(input, state) { event ->
            if (event is ShellStreamEvent.Finished) {
                onResult(event.result)
            }
        }
    }

    fun submitStreaming(
        input: String,
        state: ShellState,
        eventSink: ShellEventSink,
    ): ShellRunHandle {
        val handle = SwitchingShellRunHandle()
        resultPoster.post { eventSink.onEvent(ShellStreamEvent.Started(input, state)) }
        executor.execute {
            val result = runCatching {
                val handle = aiHandle
                val config = currentAiConfig
                when {
                    // 지속 핸들 우선(이 슬라이스 목표: 실기기 openai 지속 핸들 경로)
                    handle != 0L -> bridge.evalLineAiHandle(handle, input, state)
                    // handle 미지원(createAi가 0 반환)인데 config는 설정됨 →
                    // legacy per-call evalLineAi 폴백(하위호환·fail-soft; Task 6 createAi
                    // doc "미지원 구현은 0(호출측 폴백)" 계약).
                    config != null -> bridge.evalLineAi(input, state, config)
                    else -> bridge.evalLine(input, state)
                }
            }
                .getOrElse { error ->
                    ShellEvalResult(
                        ok = false,
                        outputText = "",
                        outputJson = "null",
                        error = error.message ?: error::class.java.simpleName,
                        state = state,
                    )
                }
            val adapter = externalAdapterRef.get()
            if (!handle.isCancelled && externalCommandsEnabled && adapter?.canHandle(input, result) == true) {
                val externalHandle = adapter.submitStreaming(input, state, eventSink)
                handle.switchTo(externalHandle)
                return@execute
            }
            resultPoster.post {
                if (handle.isCancelled) {
                    eventSink.onEvent(ShellStreamEvent.Cancelled(result.state))
                } else {
                    if (result.ok && result.outputText.isNotBlank()) {
                        eventSink.onEvent(ShellStreamEvent.Stdout(result.outputText))
                    } else if (!result.ok && !result.error.isNullOrBlank()) {
                        eventSink.onEvent(ShellStreamEvent.Stderr(result.error))
                    }
                    eventSink.onEvent(ShellStreamEvent.Finished(result))
                }
            }
        }
        return handle
    }

    fun replaceExternalAdapter(next: ExternalShellStreamAdapter?) {
        val previous = externalAdapterRef.getAndSet(next)
        if (previous !== next) {
            (previous as? AutoCloseable)?.close()
        }
    }

    fun close() {
        executor.execute {
            if (aiHandle != 0L) {
                bridge.destroyAi(aiHandle)
                aiHandle = 0L
            }
        }
        executor.shutdown()
        try { executor.awaitTermination(1, java.util.concurrent.TimeUnit.SECONDS) } catch (_: InterruptedException) {}
        (externalAdapterRef.getAndSet(null) as? AutoCloseable)?.close()
    }
}

private class SwitchingShellRunHandle : ShellRunHandle {
    private val cancelled = java.util.concurrent.atomic.AtomicBoolean(false)
    private val delegate = java.util.concurrent.atomic.AtomicReference<ShellRunHandle?>()

    override val isCancelled: Boolean
        get() = cancelled.get()

    override fun cancel() {
        cancelled.set(true)
        delegate.get()?.cancel()
    }

    fun switchTo(next: ShellRunHandle) {
        delegate.set(next)
        if (cancelled.get()) {
            next.cancel()
        }
    }
}

fun mainThreadPoster(): ResultPoster {
    val handler = Handler(Looper.getMainLooper())
    return ResultPoster { block -> handler.post(block) }
}
