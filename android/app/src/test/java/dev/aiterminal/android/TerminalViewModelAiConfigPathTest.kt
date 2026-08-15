package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.io.File
import java.nio.file.Files

class TerminalViewModelAiConfigPathTest {
    private fun tempDir(): File = Files.createTempDirectory("ai-config-path").toFile()

    private fun writeConfig(dir: File, name: String): File {
        val file = File(dir, name)
        file.writeText("""{"provider":"openai"}""")
        return file
    }

    @Test
    fun picks_first_readable_candidate() {
        val dir = tempDir()
        val appPrivate = writeConfig(dir, "app-private.json")
        val legacy = writeConfig(dir, "legacy.json")

        assertEquals(
            appPrivate,
            TerminalViewModel.pickReadableAiConfigFile(listOf(appPrivate, legacy)),
        )
    }

    @Test
    fun falls_back_to_later_candidate_when_earlier_is_missing() {
        val dir = tempDir()
        val missing = File(dir, "not-created.json")
        val legacy = writeConfig(dir, "legacy.json")

        assertEquals(
            legacy,
            TerminalViewModel.pickReadableAiConfigFile(listOf(missing, legacy)),
        )
    }

    @Test
    fun returns_null_when_no_candidate_is_readable() {
        val dir = tempDir()
        val missing = File(dir, "not-created.json")

        assertNull(TerminalViewModel.pickReadableAiConfigFile(listOf(missing)))
        assertNull(TerminalViewModel.pickReadableAiConfigFile(emptyList()))
    }

    @Test
    fun ignores_directory_candidates() {
        val dir = tempDir()
        val directory = File(dir, "ai-terminal-ai-config.json").also { it.mkdirs() }
        val legacy = writeConfig(dir, "legacy.json")

        assertEquals(
            legacy,
            TerminalViewModel.pickReadableAiConfigFile(listOf(directory, legacy)),
        )
    }
}
