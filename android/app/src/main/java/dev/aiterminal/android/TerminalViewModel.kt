package dev.aiterminal.android

import android.content.Context
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import java.io.File

data class TranscriptEntry(
    val kind: EntryKind,
    val text: String,
)

enum class EntryKind {
    Command,
    Output,
    Error,
}

class TerminalViewModel(
    private val worker: ShellWorker,
    private val termuxBridge: TermuxBridge?,
    initialState: ShellState,
    private val externalAdapterFactory: ((File) -> ExternalShellStreamAdapter)? = null,
    initialTermuxStagingPath: String = DEFAULT_TERMUX_STAGING_PATH,
) : ViewModel() {
    val transcript = mutableStateListOf(
        TranscriptEntry(EntryKind.Output, "AI Terminal Android spike"),
        TranscriptEntry(EntryKind.Output, "Rust MobileShell JNI bridge ready"),
        TranscriptEntry(EntryKind.Output, "workspace ${initialState.workspaceState().rootName}"),
    )

    var input by mutableStateOf("[{size: 50} {size: 200}] | where size > 100")
        private set

    var sessionState by mutableStateOf(initialState)
        private set

    var isBusy by mutableStateOf(false)
        private set

    var termuxStatus by mutableStateOf(
        termuxBridge?.availability()
            ?: TermuxBridgeAvailability(TermuxBridgeState.Unavailable, "Termux bridge unavailable"),
    )
        private set

    var termuxStagingPath by mutableStateOf(initialTermuxStagingPath)
        private set

    private var activeRun: ShellRunHandle? = null

    fun updateInput(value: String) {
        input = value
    }

    fun updateTermuxStagingPath(value: String) {
        termuxStagingPath = value
    }

    fun useDefaultTermuxStagingPath() {
        termuxStagingPath = DEFAULT_TERMUX_STAGING_PATH
    }

    fun submit() {
        val command = input.trim()
        if (command.isEmpty() || isBusy) return

        transcript += TranscriptEntry(EntryKind.Command, command)
        input = ""
        isBusy = true

        activeRun = worker.submitStreaming(command, sessionState) { event ->
            when (event) {
                is ShellStreamEvent.Started -> Unit
                is ShellStreamEvent.Stdout -> transcript += TranscriptEntry(EntryKind.Output, event.text)
                is ShellStreamEvent.Stderr -> transcript += TranscriptEntry(EntryKind.Error, event.text)
                is ShellStreamEvent.Cancelled -> {
                    sessionState = event.state
                    transcript += TranscriptEntry(EntryKind.Error, "cancelled")
                    activeRun = null
                    isBusy = false
                }
                is ShellStreamEvent.Finished -> {
                    val result = event.result
                    sessionState = result.state
                    if (!result.ok && !result.error.isNullOrBlank()) {
                        transcript += TranscriptEntry(EntryKind.Error, result.error)
                    }
                    activeRun = null
                    isBusy = false
                }
            }
        }
    }

    fun cancelActiveRun() {
        activeRun?.cancel()
    }

    fun probeTermux() {
        if (isBusy) return
        val bridge = termuxBridge
        worker.externalCommandsEnabled = false
        if (bridge == null) {
            termuxStatus = TermuxBridgeAvailability(
                TermuxBridgeState.Unavailable,
                "Termux bridge unavailable",
            )
            transcript += TranscriptEntry(EntryKind.Error, termuxStatus.message)
            return
        }

        val availability = bridge.availability()
        termuxStatus = availability
        if (availability.state != TermuxBridgeState.Installed) {
            transcript += TranscriptEntry(EntryKind.Error, availability.message)
            return
        }

        transcript += TranscriptEntry(EntryKind.Command, "termux t0 smoke")
        isBusy = true
        termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.ProbeRunning, "Termux T0 smoke running")
        bridge.startT0Smoke { results ->
            results.forEach { smoke ->
                val kind = if (smoke.passed) EntryKind.Output else EntryKind.Error
                val status = if (smoke.passed) "ok" else "fail"
                transcript += TranscriptEntry(kind, "termux ${smoke.case.name}: $status ${smoke.message}")
            }

            if (results.isNotEmpty() && results.all { it.passed }) {
                termuxStatus = TermuxBridgeAvailability(
                    TermuxBridgeState.Installed,
                    "Termux T0 smoke ready; install helper for streamed external commands",
                )
            } else {
                val message = results.lastOrNull()?.message ?: "Termux T0 smoke failed"
                termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Installed, message)
            }
            isBusy = false
        }
    }

    fun verifyTermuxSharedStaging() {
        if (isBusy) return
        worker.externalCommandsEnabled = false
        val adapterFactory = externalAdapterFactory
        if (adapterFactory == null) {
            val message = "Termux shared staging adapter unavailable"
            termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Installed, message)
            transcript += TranscriptEntry(EntryKind.Error, message)
            return
        }

        val stagingRoot = validateSharedStagingPath(termuxStagingPath)
            .getOrElse { error ->
                val message = error.message ?: "invalid shared staging path"
                termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Installed, message)
                transcript += TranscriptEntry(EntryKind.Error, "termux staging: $message")
                return
            }

        val adapter = adapterFactory(stagingRoot)
        var sawMarker = false
        val stderrText = StringBuilder()
        transcript += TranscriptEntry(EntryKind.Command, "termux shared staging smoke")
        isBusy = true
        termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.ProbeRunning, "Termux shared staging smoke running")
        val smokeHandle = adapter.submitStreaming(
            SHARED_STAGING_SMOKE_COMMAND,
            ShellState(cwd = stagingRoot.absolutePath, workspaceRoot = stagingRoot.absolutePath),
        ) { event ->
            when (event) {
                is ShellStreamEvent.Started -> Unit
                is ShellStreamEvent.Stdout -> {
                    if (event.text.contains(SHARED_STAGING_SMOKE_MARKER)) {
                        sawMarker = true
                    }
                    transcript += TranscriptEntry(EntryKind.Output, event.text)
                }
                is ShellStreamEvent.Stderr -> {
                    stderrText.append(event.text)
                    transcript += TranscriptEntry(EntryKind.Error, event.text)
                }
                is ShellStreamEvent.Cancelled -> {
                    (adapter as? AutoCloseable)?.close()
                    activeRun = null
                    isBusy = false
                    termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Installed, "Termux shared staging smoke cancelled")
                    transcript += TranscriptEntry(EntryKind.Error, "termux staging: cancelled")
                }
                is ShellStreamEvent.Finished -> {
                    val result = event.result
                    activeRun = null
                    isBusy = false
                    if (result.ok && sawMarker) {
                        worker.replaceExternalAdapter(adapter)
                        worker.externalCommandsEnabled = true
                        termuxStatus = TermuxBridgeAvailability(
                            TermuxBridgeState.Ready,
                            "Termux shared staging ready: ${stagingRoot.name}",
                        )
                        transcript += TranscriptEntry(EntryKind.Output, "termux staging: ok")
                    } else {
                        (adapter as? AutoCloseable)?.close()
                        val message = sharedStagingFailureMessage(stderrText.toString(), result.error)
                        termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Installed, message)
                        transcript += TranscriptEntry(EntryKind.Error, "termux staging: $message")
                    }
                }
            }
        }
        activeRun = if (isBusy) smokeHandle else null
    }

    fun installTermuxHelper() {
        if (isBusy) return
        val bridge = termuxBridge
        if (bridge == null) {
            termuxStatus = TermuxBridgeAvailability(
                TermuxBridgeState.Unavailable,
                "Termux bridge unavailable",
            )
            transcript += TranscriptEntry(EntryKind.Error, termuxStatus.message)
            return
        }

        val availability = bridge.availability()
        termuxStatus = availability
        if (availability.state != TermuxBridgeState.Installed) {
            transcript += TranscriptEntry(EntryKind.Error, availability.message)
            return
        }

        transcript += TranscriptEntry(EntryKind.Command, "termux helper install")
        isBusy = true
        worker.externalCommandsEnabled = false
        termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.ProbeRunning, "Termux helper installing")
        bridge.startHelperBootstrap { result ->
            if (result.ok && result.stdout.contains(TermuxHelperBootstrapContract.SELF_TEST_MARKER)) {
                worker.externalCommandsEnabled = false
                termuxStatus = TermuxBridgeAvailability(
                    TermuxBridgeState.Installed,
                    "Termux helper installed; shared staging smoke required",
                )
                transcript += TranscriptEntry(
                    EntryKind.Output,
                    "termux helper: ok; shared staging smoke required",
                )
            } else {
                worker.externalCommandsEnabled = false
                val message = result.internalError
                    ?: result.stderr.trim().takeIf { it.isNotEmpty() }
                    ?: result.stdout.trim().takeIf { it.isNotEmpty() }
                    ?: "Termux helper install failed"
                termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Installed, message)
                transcript += TranscriptEntry(EntryKind.Error, "termux helper: $message")
            }
            isBusy = false
        }
    }

    fun importDocument(context: Context, uri: Uri) {
        val result = runCatching {
            importDocumentToWorkspace(context.applicationContext, uri, sessionState)
        }
        result
            .onSuccess { imported ->
                transcript += TranscriptEntry(
                    EntryKind.Output,
                    "imported ${imported.fileName} (${imported.bytes} bytes)",
                )
                val preview = imported.preview
                if (preview != null) {
                    val body = preview.text.ifEmpty { "(empty)" }
                    val marker = if (preview.truncated) "\n..." else ""
                    transcript += TranscriptEntry(
                        EntryKind.Output,
                        "preview ${imported.fileName}:\n$body$marker",
                    )
                }
            }
            .onFailure { error ->
                transcript += TranscriptEntry(
                    EntryKind.Error,
                    "import failed: ${error.message ?: error::class.java.simpleName}",
                )
            }
    }

    fun exportTranscript(context: Context, uri: Uri) {
        val snapshot = transcript.toList()
        val result = runCatching {
            exportTranscript(context.applicationContext, uri, snapshot)
        }
        result
            .onSuccess {
                transcript += TranscriptEntry(EntryKind.Output, "exported transcript")
            }
            .onFailure { error ->
                transcript += TranscriptEntry(
                    EntryKind.Error,
                    "export failed: ${error.message ?: error::class.java.simpleName}",
                )
            }
    }

    override fun onCleared() {
        worker.close()
    }

    private fun validateSharedStagingPath(path: String): Result<File> =
        runCatching {
            val trimmed = path.trim()
            require(trimmed.isNotEmpty()) { "shared staging path required" }
            val root = File(trimmed)
            require(root.mkdirs() || root.isDirectory) { "shared staging path is not a directory" }
            require(root.canWrite()) { "shared staging path is not writable by the app" }
            val probe = File(root, ".ash-app-write-test")
            probe.writeText("ok\n", Charsets.UTF_8)
            probe.delete()
            root.canonicalFile
        }

    private fun sharedStagingFailureMessage(stderr: String, fallback: String?): String {
        val lower = stderr.lowercase()
        return when {
            "permission denied" in lower || "not writable" in lower ->
                "Termux storage permission required for shared staging"
            stderr.isNotBlank() -> stderr.trim().lineSequence().lastOrNull().orEmpty()
            else -> fallback ?: "Termux shared staging smoke failed"
        }
    }

    companion object {
        const val DEFAULT_TERMUX_STAGING_PATH = "/sdcard/Download/ash-termux-bridge"
        private const val SHARED_STAGING_SMOKE_MARKER = "ASH_SHARED_STAGING_OK"
        private const val SHARED_STAGING_SMOKE_COMMAND = "sh -c \"echo $SHARED_STAGING_SMOKE_MARKER\""

        fun factory(context: Context): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T {
                    val workspace = ensureAppPrivateWorkspace(context.applicationContext)
                    val termuxBridge = AndroidTermuxBridge(context.applicationContext)
                    val initialState = ShellState(
                        cwd = workspace.cwdPath,
                        workspaceRoot = workspace.rootPath,
                    )
                    return TerminalViewModel(
                        worker = ShellWorker(
                            bridge = NativeShellBridge(),
                            externalAdapter = null,
                        ),
                        termuxBridge = termuxBridge,
                        initialState = initialState,
                        externalAdapterFactory = { bridgeRoot ->
                            TermuxHelperBridgeAdapter(
                                bridgeRoot = bridgeRoot,
                                launcher = AndroidTermuxHelperLauncher(context.applicationContext),
                                resultPoster = mainThreadPoster(),
                            )
                        },
                    ) as T
                }
            }
    }
}
