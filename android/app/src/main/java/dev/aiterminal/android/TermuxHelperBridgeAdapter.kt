package dev.aiterminal.android

import java.io.File
import java.util.Locale
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

interface ExternalShellStreamAdapter {
    fun canHandle(input: String, pureResult: ShellEvalResult): Boolean
    fun submitStreaming(
        input: String,
        state: ShellState,
        eventSink: ShellEventSink,
    ): ShellRunHandle
}

fun interface TermuxHelperLauncher {
    fun startHelper(jobDir: File, callback: TermuxProbeCallback): ShellRunHandle
}

class TermuxHelperBridgeAdapter(
    private val bridgeRoot: File,
    private val launcher: TermuxHelperLauncher,
    private val resultPoster: ResultPoster,
    private val scheduler: ScheduledExecutorService = Executors.newSingleThreadScheduledExecutor(),
    private val pollIntervalMs: Long = DEFAULT_POLL_INTERVAL_MS,
) : ExternalShellStreamAdapter, AutoCloseable {
    override fun canHandle(input: String, pureResult: ShellEvalResult): Boolean =
        pureResult.error?.contains(EXTERNAL_DISABLED_ERROR) == true &&
            parseExternalArgv(input) != null

    override fun submitStreaming(
        input: String,
        state: ShellState,
        eventSink: ShellEventSink,
    ): ShellRunHandle {
        val argv = parseExternalArgv(input)
        if (argv == null) {
            resultPoster.post {
                eventSink.onEvent(ShellStreamEvent.Stderr("external command is not a single argv command"))
                eventSink.onEvent(ShellStreamEvent.Finished(errorResult(state, "external command is not a single argv command")))
            }
            return AtomicShellRunHandle().also { it.cancel() }
        }

        val files = TermuxHelperJobFiles(newJobDir())
        val fileHandle = FileBackedShellRunHandle(files.cancelFile)
        val started = runCatching {
            files.jobDir.mkdirs()
            files.eventsFile.writeText("", Charsets.UTF_8)
            writeFallbackArgvFiles(files, argv)
            files.requestFile.writeText(
                TermuxHelperProtocol.encodeRequest(
                    TermuxHelperRequest(
                        argv = argv,
                        cwd = state.cwd,
                        env = DEFAULT_ENV,
                    ),
                ),
                Charsets.UTF_8,
            )
        }
        if (started.isFailure) {
            val message = started.exceptionOrNull()?.message ?: "failed to create Termux helper request"
            resultPoster.post {
                eventSink.onEvent(ShellStreamEvent.Stderr(message))
                eventSink.onEvent(ShellStreamEvent.Finished(errorResult(state, message)))
            }
            return AtomicShellRunHandle().also { it.cancel() }
        }

        val terminalDelivered = AtomicBoolean(false)
        val poller = TermuxHelperEventFilePoller(files, input, state)
        val pollFuture = AtomicReference<ScheduledFuture<*>?>()
        val launcherHandle = AtomicReference<ShellRunHandle?>()

        val handle = object : ShellRunHandle {
            override val isCancelled: Boolean
                get() = fileHandle.isCancelled

            override fun cancel() {
                fileHandle.cancel()
                launcherHandle.get()?.cancel()
            }
        }

        fun deliver(events: List<ShellStreamEvent>) {
            if (events.isEmpty()) return
            val visibleEvents = events.filterNot { it is ShellStreamEvent.Started }
            if (visibleEvents.isEmpty()) return

            if (visibleEvents.any { it is ShellStreamEvent.Finished || it is ShellStreamEvent.Cancelled }) {
                terminalDelivered.set(true)
                pollFuture.get()?.cancel(false)
            }
            resultPoster.post {
                visibleEvents.forEach(eventSink::onEvent)
            }
        }

        val scheduled = scheduler.scheduleWithFixedDelay(
            {
                runCatching {
                    deliver(poller.poll())
                }.onFailure { error ->
                    if (terminalDelivered.compareAndSet(false, true)) {
                        pollFuture.get()?.cancel(false)
                        val message = error.message ?: error::class.java.simpleName
                        resultPoster.post {
                            eventSink.onEvent(ShellStreamEvent.Stderr(message))
                            eventSink.onEvent(ShellStreamEvent.Finished(errorResult(state, message)))
                        }
                    }
                }
            },
            0,
            pollIntervalMs,
            TimeUnit.MILLISECONDS,
        )
        pollFuture.set(scheduled)

        val launched = launcher.startHelper(files.jobDir) { result ->
            if (terminalDelivered.get()) return@startHelper

            val maybeEvents = poller.poll()
            if (maybeEvents.isNotEmpty()) {
                deliver(maybeEvents)
                return@startHelper
            }

            if (!result.ok && terminalDelivered.compareAndSet(false, true)) {
                scheduled.cancel(false)
                val message = result.internalError
                    ?: result.stderr.trim().takeIf { it.isNotEmpty() }
                    ?: "Termux bridge helper failed"
                resultPoster.post {
                    eventSink.onEvent(ShellStreamEvent.Stderr(message))
                    eventSink.onEvent(ShellStreamEvent.Finished(errorResult(state, message, result.exitCode)))
                }
            }
        }
        launcherHandle.set(launched)

        if (fileHandle.isCancelled) {
            launched.cancel()
        }
        return handle
    }

    override fun close() {
        scheduler.shutdownNow()
    }

    private fun newJobDir(): File =
        File(File(bridgeRoot, "jobs"), "job-${UUID.randomUUID()}")

    private fun writeFallbackArgvFiles(files: TermuxHelperJobFiles, argv: List<String>) {
        files.argvDir.mkdirs()
        argv.forEachIndexed { index, arg ->
            File(files.argvDir, index.toString().padStart(4, '0'))
                .writeText(arg, Charsets.UTF_8)
        }
    }

    private fun errorResult(
        state: ShellState,
        message: String,
        exitCode: Int? = null,
    ): ShellEvalResult =
        ShellEvalResult(
            ok = false,
            outputText = "",
            outputJson = "null",
            error = message,
            state = state.copy(exitCode = exitCode),
        )

    companion object {
        private const val EXTERNAL_DISABLED_ERROR = "external execution disabled"
        private const val DEFAULT_POLL_INTERVAL_MS = 100L
        private val DEFAULT_ENV = mapOf("TERM" to "xterm-256color")
    }
}

fun parseExternalArgv(input: String): List<String>? {
    val trimmed = input.trim()
    if (trimmed.isEmpty()) return null
    if (trimmed.any { it in "|;&<>()" }) return null

    val args = mutableListOf<String>()
    val current = StringBuilder()
    var quote: Char? = null
    var escaping = false

    for (char in trimmed) {
        when {
            escaping -> {
                current.append(char)
                escaping = false
            }
            char == '\\' -> escaping = true
            quote != null -> {
                if (char == quote) {
                    quote = null
                } else {
                    current.append(char)
                }
            }
            char == '"' || char == '\'' -> quote = char
            char.isWhitespace() -> {
                if (current.isNotEmpty()) {
                    args += current.toString()
                    current.clear()
                }
            }
            else -> current.append(char)
        }
    }

    if (escaping || quote != null) return null
    if (current.isNotEmpty()) args += current.toString()
    if (args.isEmpty()) return null

    val command = args.first().lowercase(Locale.US)
    if (command == "cd" || command == "let") return null
    return args
}
