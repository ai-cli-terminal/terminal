package dev.aiterminal.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp

class MainActivity : ComponentActivity() {
    private val viewModel: TerminalViewModel by viewModels { TerminalViewModel.factory() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                TerminalScreen(viewModel)
            }
        }
    }
}

@Composable
fun TerminalScreen(viewModel: TerminalViewModel) {
    Surface(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            SessionStatus(viewModel.sessionState, viewModel.isBusy)
            Transcript(
                entries = viewModel.transcript,
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
            )
            CommandInput(
                value = viewModel.input,
                busy = viewModel.isBusy,
                onValueChange = viewModel::updateInput,
                onSubmit = viewModel::submit,
            )
        }
    }
}

@Composable
private fun SessionStatus(state: ShellState, busy: Boolean) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column {
            Text("AI Terminal", style = MaterialTheme.typography.titleMedium)
            Text("cwd ${state.cwd}", style = MaterialTheme.typography.bodySmall)
        }
        Text(
            if (busy) "running" else "ready",
            color = if (busy) Color(0xFF9A5B00) else Color(0xFF116D38),
            style = MaterialTheme.typography.labelLarge,
        )
    }
}

@Composable
private fun Transcript(entries: List<TranscriptEntry>, modifier: Modifier = Modifier) {
    val listState = rememberLazyListState()
    LaunchedEffect(entries.size) {
        if (entries.isNotEmpty()) {
            listState.animateScrollToItem(entries.lastIndex)
        }
    }

    LazyColumn(
        state = listState,
        modifier = modifier
            .background(Color(0xFF101418))
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(entries) { entry ->
            val color = when (entry.kind) {
                EntryKind.Command -> Color(0xFF9CCAFF)
                EntryKind.Output -> Color(0xFFE6EDF3)
                EntryKind.Error -> Color(0xFFFFB4AB)
            }
            val prefix = when (entry.kind) {
                EntryKind.Command -> "> "
                EntryKind.Output -> ""
                EntryKind.Error -> "error: "
            }
            Text(
                text = prefix + entry.text,
                color = color,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.bodyMedium,
            )
        }
    }
}

@Composable
private fun CommandInput(
    value: String,
    busy: Boolean,
    onValueChange: (String) -> Unit,
    onSubmit: () -> Unit,
) {
    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = value,
                onValueChange = onValueChange,
                modifier = Modifier.weight(1f),
                enabled = !busy,
                singleLine = true,
                label = { Text("ash") },
            )
            Button(
                onClick = onSubmit,
                enabled = !busy && value.isNotBlank(),
            ) {
                Text("Run")
            }
        }
        Spacer(Modifier.height(4.dp))
        Text(
            "shellcore-only; external commands are blocked",
            style = MaterialTheme.typography.bodySmall,
            color = Color(0xFF536471),
        )
    }
}
