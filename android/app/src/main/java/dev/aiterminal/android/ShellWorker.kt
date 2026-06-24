package dev.aiterminal.android

import android.os.Handler
import android.os.Looper
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

fun interface ResultPoster {
    fun post(block: () -> Unit)
}

class ShellWorker(
    private val bridge: ShellBridge,
    private val executor: ExecutorService = Executors.newSingleThreadExecutor(),
    private val resultPoster: ResultPoster = mainThreadPoster(),
) {
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
        val handle = AtomicShellRunHandle()
        resultPoster.post { eventSink.onEvent(ShellStreamEvent.Started(input, state)) }
        executor.execute {
            val result = runCatching { bridge.evalLine(input, state) }
                .getOrElse { error ->
                    ShellEvalResult(
                        ok = false,
                        outputText = "",
                        outputJson = "null",
                        error = error.message ?: error::class.java.simpleName,
                        state = state,
                    )
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

    fun close() {
        executor.shutdownNow()
    }
}

private fun mainThreadPoster(): ResultPoster {
    val handler = Handler(Looper.getMainLooper())
    return ResultPoster { block -> handler.post(block) }
}
