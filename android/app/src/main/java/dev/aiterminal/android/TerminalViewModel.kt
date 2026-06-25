package dev.aiterminal.android

import android.content.Context
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider

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

    fun updateInput(value: String) {
        input = value
    }

    fun submit() {
        val command = input.trim()
        if (command.isEmpty() || isBusy) return

        transcript += TranscriptEntry(EntryKind.Command, command)
        input = ""
        isBusy = true

        worker.submit(command, sessionState) { result ->
            sessionState = result.state
            if (result.ok) {
                if (result.outputText.isNotBlank()) {
                    transcript += TranscriptEntry(EntryKind.Output, result.outputText)
                }
            } else {
                transcript += TranscriptEntry(EntryKind.Error, result.error ?: "unknown error")
            }
            isBusy = false
        }
    }

    fun probeTermux() {
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
                termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Ready, "Termux T0 smoke ready")
            } else {
                val message = results.lastOrNull()?.message ?: "Termux T0 smoke failed"
                termuxStatus = TermuxBridgeAvailability(TermuxBridgeState.Installed, message)
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

    companion object {
        fun factory(context: Context): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T {
                    val workspace = ensureAppPrivateWorkspace(context.applicationContext)
                    val initialState = ShellState(
                        cwd = workspace.cwdPath,
                        workspaceRoot = workspace.rootPath,
                    )
                    return TerminalViewModel(
                        worker = ShellWorker(NativeShellBridge()),
                        termuxBridge = AndroidTermuxBridge(context.applicationContext),
                        initialState = initialState,
                    ) as T
                }
            }
    }
}
