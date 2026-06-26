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
        executor.shutdownNow()
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
