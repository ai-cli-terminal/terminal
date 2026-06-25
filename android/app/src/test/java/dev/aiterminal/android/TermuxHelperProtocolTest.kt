package dev.aiterminal.android

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TermuxHelperProtocolTest {
    @Test
    fun encodeRequestUsesArgvArrayAndExplicitCwdEnvAndTimeout() {
        val raw = TermuxHelperProtocol.encodeRequest(
            TermuxHelperRequest(
                argv = listOf("grep", "-n", "needle", "file.txt"),
                cwd = "bridge://workspace",
                env = mapOf("TERM" to "xterm-256color"),
                timeoutMs = 30_000,
            ),
        )

        val json = JSONObject(raw)
        val argv = json.getJSONArray("argv")
        assertEquals("grep", argv.getString(0))
        assertEquals("-n", argv.getString(1))
        assertEquals("needle", argv.getString(2))
        assertEquals("file.txt", argv.getString(3))
        assertEquals("bridge://workspace", json.getString("cwd"))
        assertEquals("xterm-256color", json.getJSONObject("env").getString("TERM"))
        assertEquals(30_000, json.getLong("timeout_ms"))
    }

    @Test
    fun encodeRequestRejectsEmptyArgv() {
        val error = runCatching {
            TermuxHelperProtocol.encodeRequest(TermuxHelperRequest(argv = emptyList(), cwd = "."))
        }.exceptionOrNull()

        assertTrue(error is IllegalArgumentException)
        assertEquals("argv must not be empty", error?.message)
    }

    @Test
    fun decodeEventLineMapsStartedStdoutStderrAndCancelled() {
        val state = ShellState(cwd = "/work")

        val started = TermuxHelperProtocol.decodeEventLine(
            """{"seq":1,"type":"started","job_id":"j1","pid":123}""",
            input = "grep needle",
            state = state,
        )
        val stdout = TermuxHelperProtocol.decodeEventLine(
            """{"seq":2,"type":"stdout","text":"hello\n"}""",
            input = "grep needle",
            state = state,
        )
        val stderr = TermuxHelperProtocol.decodeEventLine(
            """{"seq":3,"type":"stderr","text":"warn\n"}""",
            input = "grep needle",
            state = state,
        )
        val cancelled = TermuxHelperProtocol.decodeEventLine(
            """{"seq":4,"type":"cancelled","reason":"user"}""",
            input = "grep needle",
            state = state,
        )

        assertEquals(ShellStreamEvent.Started("grep needle", state), started)
        assertEquals(ShellStreamEvent.Stdout("hello\n"), stdout)
        assertEquals(ShellStreamEvent.Stderr("warn\n"), stderr)
        assertEquals(ShellStreamEvent.Cancelled(state), cancelled)
    }

    @Test
    fun decodeFinishedEventPreservesExitCode() {
        val event = TermuxHelperProtocol.decodeEventLine(
            """{"seq":4,"type":"finished","exit_code":7}""",
            input = "false",
            state = ShellState(cwd = "/work"),
        )

        assertTrue(event is ShellStreamEvent.Finished)
        val result = (event as ShellStreamEvent.Finished).result
        assertFalse(result.ok)
        assertEquals(7, result.state.exitCode)
        assertEquals("external command exited 7", result.error)
    }

    @Test
    fun decodeSuccessfulFinishedEventHasNoError() {
        val event = TermuxHelperProtocol.decodeEventLine(
            """{"seq":4,"type":"finished","exit_code":0}""",
            input = "true",
            state = ShellState(cwd = "/work"),
        )

        assertTrue(event is ShellStreamEvent.Finished)
        val result = (event as ShellStreamEvent.Finished).result
        assertTrue(result.ok)
        assertEquals(0, result.state.exitCode)
        assertNull(result.error)
    }
}
