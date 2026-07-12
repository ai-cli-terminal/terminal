package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

class TerminalViewModelAiTest {
    private class PostedQueueFixture {
        val posted = ArrayBlockingQueue<() -> Unit>(16)

        fun worker(bridge: ShellBridge): ShellWorker =
            ShellWorker(
                bridge = bridge,
                executor = Executors.newSingleThreadExecutor(),
                resultPoster = ResultPoster { block -> posted.put(block) },
            )

        fun drain(viewModel: TerminalViewModel) {
            while (viewModel.isBusy) {
                val block = posted.poll(2, TimeUnit.SECONDS) ?: break
                block.invoke()
            }
            assertFalse("viewmodel should settle after drain", viewModel.isBusy)
        }
    }

    private fun suggestionBridge(suggestion: AiSuggestion) = object : ShellBridge {
        override fun evalLine(input: String, state: ShellState): ShellEvalResult =
            error("AI가 켜지면 evalLineAi 경로를 써야 한다")

        override fun evalLineAi(
            input: String,
            state: ShellState,
            aiConfig: ShellAiConfig,
        ): ShellEvalResult =
            ShellEvalResult(
                ok = true,
                outputText = "",
                outputJson = "null",
                error = null,
                state = state,
                ai = suggestion,
            )
    }

    @Test
    fun toggleAiWiresWorkerConfigAndAnnounces() {
        val fixture = PostedQueueFixture()
        val worker = fixture.worker(suggestionBridge(AiSuggestion("answered", "x")))
        val viewModel = TerminalViewModel(worker, null, ShellState())

        assertFalse(viewModel.aiEnabled)
        assertNull(worker.aiConfig)

        viewModel.toggleAi()

        assertTrue(viewModel.aiEnabled)
        assertEquals(ShellAiConfig(), worker.aiConfig)
        assertEquals(EntryKind.Output, viewModel.transcript.last().kind)

        viewModel.toggleAi()

        assertFalse(viewModel.aiEnabled)
        assertNull(worker.aiConfig)

        worker.close()
    }

    @Test
    fun answeredSuggestionRendersAsAiSuggestionEntry() {
        val fixture = PostedQueueFixture()
        val worker = fixture.worker(suggestionBridge(AiSuggestion("answered", "du -sh *")))
        val viewModel = TerminalViewModel(worker, null, ShellState())

        viewModel.toggleAi()
        viewModel.updateInput("큰 파일 찾아줘")
        viewModel.submit()
        fixture.drain(viewModel)

        val entry = viewModel.transcript.last()
        assertEquals(EntryKind.AiSuggestion, entry.kind)
        assertEquals("du -sh *", entry.text)

        worker.close()
    }

    @Test
    fun blockedSuggestionRendersAsError() {
        val fixture = PostedQueueFixture()
        val worker = fixture.worker(suggestionBridge(AiSuggestion("blocked", "masking failed")))
        val viewModel = TerminalViewModel(worker, null, ShellState())

        viewModel.toggleAi()
        viewModel.updateInput("secret 보내줘")
        viewModel.submit()
        fixture.drain(viewModel)

        val entry = viewModel.transcript.last()
        assertEquals(EntryKind.Error, entry.kind)
        assertEquals("ai blocked: masking failed", entry.text)

        worker.close()
    }

    @Test
    fun parseAiConfigReadsOpenaiFromJson() {
        val json = """{"provider":"openai","model":"gpt-4o-mini","openai_url":"https://api.openai.com","api_key":"sk-x"}"""
        val cfg = parseAiConfigJson(json)
        assertEquals("openai", cfg.provider)
        assertEquals("gpt-4o-mini", cfg.model)
        assertEquals("sk-x", cfg.apiKey)
    }

    @Test
    fun parseAiConfigFallsBackToMockOnBadJson() {
        assertEquals("mock", parseAiConfigJson("{not-json").provider)
        assertEquals(ShellAiConfig(), parseAiConfigJson("{not-json"))
    }
}
