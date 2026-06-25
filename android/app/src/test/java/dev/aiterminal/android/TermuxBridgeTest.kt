package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TermuxBridgeTest {
    @Test
    fun availabilityRequiresInstalledRuntime() {
        val availability = TermuxRunCommandContract.availability(
            installed = false,
            permissionGranted = true,
        )

        assertEquals(TermuxBridgeState.Unavailable, availability.state)
        assertEquals("external runtime unavailable", availability.message)
    }

    @Test
    fun availabilityRequiresRunCommandPermission() {
        val availability = TermuxRunCommandContract.availability(
            installed = true,
            permissionGranted = false,
        )

        assertEquals(TermuxBridgeState.PermissionRequired, availability.state)
        assertEquals("Termux bridge permission required", availability.message)
    }

    @Test
    fun availabilityAllowsProbeWhenRuntimeAndPermissionExist() {
        val availability = TermuxRunCommandContract.availability(
            installed = true,
            permissionGranted = true,
        )

        assertEquals(TermuxBridgeState.Installed, availability.state)
    }

    @Test
    fun decodeResultMapKeepsStdoutStderrAndExitCode() {
        val result = TermuxRunCommandContract.decodeResultMap(
            mapOf(
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT to "ASH_TERMUX_OK\n",
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_STDERR to "warn\n",
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_EXIT_CODE to 0,
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_ERR to -1,
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT_ORIGINAL_LENGTH to 15,
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_STDERR_ORIGINAL_LENGTH to 5,
            ),
        )

        assertTrue(result.ok)
        assertEquals("ASH_TERMUX_OK\n", result.stdout)
        assertEquals("warn\n", result.stderr)
        assertEquals(0, result.exitCode)
        assertEquals(15, result.stdoutOriginalLength)
        assertEquals(5, result.stderrOriginalLength)
    }

    @Test
    fun decodeResultMapReportsInternalError() {
        val result = TermuxRunCommandContract.decodeResultMap(
            mapOf(
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_ERR to 1,
                TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE_ERRMSG to "allow-external-apps disabled",
            ),
        )

        assertFalse(result.ok)
        assertEquals("allow-external-apps disabled", result.internalError)
    }

    @Test
    fun commandResultConvertsToShellEvalResult() {
        val state = ShellState(cwd = "/app/work", workspaceRoot = "/app")
        val shellResult = TermuxRunCommandContract.toShellEvalResult(
            TermuxCommandResult(stdout = "done\n", exitCode = 7),
            state,
        )

        assertFalse(shellResult.ok)
        assertEquals("done\n", shellResult.outputText)
        assertEquals("external command exited 7", shellResult.error)
        assertEquals(7, shellResult.state.exitCode)
    }

    @Test
    fun successfulCommandResultHasNoError() {
        val shellResult = TermuxRunCommandContract.toShellEvalResult(
            TermuxCommandResult(stdout = "ok\n", exitCode = 0),
            ShellState(),
        )

        assertTrue(shellResult.ok)
        assertEquals("ok\n", shellResult.outputText)
        assertNull(shellResult.error)
        assertEquals(0, shellResult.state.exitCode)
    }
}
