package dev.aiterminal.android

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
) : ViewModel() {
    val transcript = mutableStateListOf(
        TranscriptEntry(EntryKind.Output, "AI Terminal Android spike"),
        TranscriptEntry(EntryKind.Output, "Rust MobileShell JNI bridge ready"),
    )

    var input by mutableStateOf("[{size: 50} {size: 200}] | where size > 100")
        private set

    var sessionState by mutableStateOf(ShellState())
        private set

    var isBusy by mutableStateOf(false)
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

    override fun onCleared() {
        worker.close()
    }

    companion object {
        fun factory(): ViewModelProvider.Factory =
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : ViewModel> create(modelClass: Class<T>): T {
                    return TerminalViewModel(ShellWorker(NativeShellBridge())) as T
                }
            }
    }
}
