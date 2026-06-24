package dev.aiterminal.android

import java.util.concurrent.atomic.AtomicBoolean

sealed class ShellStreamEvent {
    data class Started(
        val input: String,
        val state: ShellState,
    ) : ShellStreamEvent()

    data class Stdout(
        val text: String,
    ) : ShellStreamEvent()

    data class Stderr(
        val text: String,
    ) : ShellStreamEvent()

    data class Finished(
        val result: ShellEvalResult,
    ) : ShellStreamEvent()

    data class Cancelled(
        val state: ShellState,
    ) : ShellStreamEvent()
}

fun interface ShellEventSink {
    fun onEvent(event: ShellStreamEvent)
}

interface ShellRunHandle {
    val isCancelled: Boolean
    fun cancel()
}

class AtomicShellRunHandle : ShellRunHandle {
    private val cancelled = AtomicBoolean(false)

    override val isCancelled: Boolean
        get() = cancelled.get()

    override fun cancel() {
        cancelled.set(true)
    }
}
