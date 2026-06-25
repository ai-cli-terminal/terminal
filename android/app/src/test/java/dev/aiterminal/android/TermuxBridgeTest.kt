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
    fun resultBundleKeyMatchesTermuxServiceContract() {
        assertEquals("result", TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE)
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

    @Test
    fun t0SmokeCasesCoverEchoPwdStderrAndNonZero() {
        assertEquals(
            listOf("echo", "pwd", "stderr", "non-zero"),
            TermuxRunCommandContract.T0_SMOKE_CASES.map { it.name },
        )
    }

    @Test
    fun t0SmokeEvaluationAcceptsExpectedPwdStderrAndNonZeroResults() {
        val cases = TermuxRunCommandContract.T0_SMOKE_CASES.associateBy { it.name }

        val pwd = TermuxRunCommandContract.evaluateT0Smoke(
            cases.getValue("pwd"),
            TermuxCommandResult(stdout = "/data/data/com.termux/files/home\n", exitCode = 0),
        )
        val stderr = TermuxRunCommandContract.evaluateT0Smoke(
            cases.getValue("stderr"),
            TermuxCommandResult(stderr = "ERR\n", exitCode = 2),
        )
        val nonZero = TermuxRunCommandContract.evaluateT0Smoke(
            cases.getValue("non-zero"),
            TermuxCommandResult(exitCode = 7),
        )

        assertTrue(pwd.passed)
        assertEquals("/data/data/com.termux/files/home", pwd.message)
        assertTrue(stderr.passed)
        assertEquals("ERR", stderr.message)
        assertTrue(nonZero.passed)
        assertEquals("exit 7", nonZero.message)
    }

    @Test
    fun t0SmokeEvaluationRejectsUnexpectedExitCode() {
        val case = TermuxRunCommandContract.T0_SMOKE_CASES.first { it.name == "non-zero" }
        val result = TermuxRunCommandContract.evaluateT0Smoke(
            case,
            TermuxCommandResult(stdout = "wrong\n", exitCode = 0),
        )

        assertFalse(result.passed)
        assertEquals("exit 0, stdout=wrong", result.message)
    }
}
