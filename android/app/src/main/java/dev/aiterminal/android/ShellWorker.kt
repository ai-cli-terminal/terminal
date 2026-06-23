package dev.aiterminal.android

import android.os.Handler
import android.os.Looper
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

class ShellWorker(
    private val bridge: ShellBridge,
    private val executor: ExecutorService = Executors.newSingleThreadExecutor(),
    private val mainHandler: Handler = Handler(Looper.getMainLooper()),
) {
    fun submit(input: String, state: ShellState, onResult: (ShellEvalResult) -> Unit) {
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
            mainHandler.post { onResult(result) }
        }
    }

    fun close() {
        executor.shutdownNow()
    }
}
