package dev.aiterminal.android

import java.io.File
import java.io.RandomAccessFile
import java.util.concurrent.atomic.AtomicBoolean

data class TermuxHelperJobFiles(
    val jobDir: File,
) {
    val requestFile: File
        get() = File(jobDir, "request.json")
    val eventsFile: File
        get() = File(jobDir, "events.ndjson")
    val cancelFile: File
        get() = File(jobDir, "cancel")
    val exitFile: File
        get() = File(jobDir, "exit.json")
}

class TermuxHelperEventFilePoller(
    private val files: TermuxHelperJobFiles,
    private val input: String,
    private val state: ShellState,
    private val maxReadBytes: Int = DEFAULT_MAX_READ_BYTES,
) {
    private var offset = 0L
    private var partialLine = ""
    private var terminalSeen = false

    val isTerminal: Boolean
        get() = terminalSeen

    fun poll(): List<ShellStreamEvent> {
        if (terminalSeen) return emptyList()

        val chunk = readNewChunk() ?: return emptyList()
        if (chunk.isEmpty()) return emptyList()

        val completedLines = consumeCompletedLines(chunk)
        if (completedLines.isEmpty()) return emptyList()

        val events = mutableListOf<ShellStreamEvent>()
        for (line in completedLines) {
            if (line.isBlank()) continue

            val event = TermuxHelperProtocol.decodeEventLine(line, input, state)
            events += event
            if (event is ShellStreamEvent.Finished || event is ShellStreamEvent.Cancelled) {
                terminalSeen = true
                break
            }
        }
        return events
    }

    private fun readNewChunk(): String? {
        val eventsFile = files.eventsFile
        if (!eventsFile.isFile) return null

        return RandomAccessFile(eventsFile, "r").use { file ->
            val length = file.length()
            if (length < offset) {
                offset = 0L
                partialLine = ""
            }
            if (length == offset) return@use ""

            val bytesToRead = minOf(length - offset, maxReadBytes.toLong()).toInt()
            val bytes = ByteArray(bytesToRead)
            file.seek(offset)
            val bytesRead = file.read(bytes)
            if (bytesRead <= 0) {
                ""
            } else {
                offset += bytesRead
                String(bytes, 0, bytesRead, Charsets.UTF_8)
            }
        }
    }

    private fun consumeCompletedLines(chunk: String): List<String> {
        val text = partialLine + chunk
        val completed = mutableListOf<String>()
        var lineStart = 0

        for (index in text.indices) {
            if (text[index] == '\n') {
                val rawLine = text.substring(lineStart, index)
                completed += rawLine.removeSuffix("\r")
                lineStart = index + 1
            }
        }

        partialLine = text.substring(lineStart)
        return completed
    }

    private companion object {
        const val DEFAULT_MAX_READ_BYTES = 256 * 1024
    }
}

class FileBackedShellRunHandle(
    private val cancelFile: File,
    private val reason: String = "user",
) : ShellRunHandle {
    private val cancelled = AtomicBoolean(false)

    override val isCancelled: Boolean
        get() = cancelled.get()

    override fun cancel() {
        if (!cancelled.compareAndSet(false, true)) return

        runCatching {
            cancelFile.parentFile?.mkdirs()
            cancelFile.writeText("$reason\n", Charsets.UTF_8)
        }
    }
}
