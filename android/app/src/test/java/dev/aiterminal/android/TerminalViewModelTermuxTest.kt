package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class TerminalViewModelTermuxTest {
    @Rule
    @JvmField
    val temporaryFolder = TemporaryFolder()

    @Test
    fun sharedStagingPathDefaultsToVerifiedDownloadLocation() {
        val worker = testWorker()
        val viewModel = TerminalViewModel(worker, installedBridge(), ShellState(cwd = "/work"))

        assertEquals(TerminalViewModel.DEFAULT_TERMUX_STAGING_PATH, viewModel.termuxStagingPath)

        viewModel.updateTermuxStagingPath("/tmp/custom")
        viewModel.useDefaultTermuxStagingPath()

        assertEquals(TerminalViewModel.DEFAULT_TERMUX_STAGING_PATH, viewModel.termuxStagingPath)

        worker.close()
    }

    @Test
    fun t0SmokeAndHelperBootstrapDoNotEnableExternalCommandsUntilStagingSmokePasses() {
        val worker = testWorker()
        val bridge = FakeTermuxBridge(
            t0Results = TermuxRunCommandContract.T0_SMOKE_CASES.map { smokeCase ->
                TermuxSmokeResult(
                    case = smokeCase,
                    commandResult = TermuxCommandResult(stdout = "ok", exitCode = 0),
                    passed = true,
                    message = "ok",
                )
            },
            bootstrapResult = TermuxCommandResult(
                stdout = "${TermuxHelperBootstrapContract.SELF_TEST_MARKER}\n",
                exitCode = 0,
            ),
        )
        val viewModel = TerminalViewModel(worker, bridge, ShellState(cwd = "/work"))

        viewModel.probeTermux()

        assertFalse(worker.externalCommandsEnabled)
        assertEquals(TermuxBridgeState.Installed, viewModel.termuxStatus.state)

        viewModel.installTermuxHelper()

        assertFalse(worker.externalCommandsEnabled)
        assertEquals(TermuxBridgeState.Installed, viewModel.termuxStatus.state)
        assertEquals("Termux helper installed; shared staging smoke required", viewModel.termuxStatus.message)

        worker.close()
    }

    @Test
    fun sharedStagingSmokeEnablesExternalAdapterOnlyAfterMarker() {
        val worker = externalDisabledWorker()
        val stagingRoot = temporaryFolder.newFolder("shared-staging")
        val adapter = FakeExternalAdapter(
            events = listOf(
                ShellStreamEvent.Stdout("ASH_SHARED_STAGING_OK\n"),
                ShellStreamEvent.Finished(ShellEvalResult(true, "", "null", null, ShellState())),
            ),
        )
        val viewModel = TerminalViewModel(
            worker = worker,
            termuxBridge = installedBridge(),
            initialState = ShellState(cwd = "/work"),
            externalAdapterFactory = { root -> adapter.apply { bridgeRoot = root } },
        )

        viewModel.updateTermuxStagingPath(stagingRoot.absolutePath)
        viewModel.verifyTermuxSharedStaging()

        assertTrue(worker.externalCommandsEnabled)
        assertEquals(TermuxBridgeState.Ready, viewModel.termuxStatus.state)
        assertEquals(stagingRoot.canonicalFile, adapter.bridgeRoot)

        val events = mutableListOf<ShellStreamEvent>()
        val finished = CountDownLatch(1)
        worker.submitStreaming("echo hi", ShellState()) { event ->
            events += event
            if (event is ShellStreamEvent.Finished) {
                finished.countDown()
            }
        }

        assertTrue(finished.await(2, TimeUnit.SECONDS))
        assertTrue(adapter.submittedAfterSmoke.get())
        assertTrue(events.any { it is ShellStreamEvent.Stdout && it.text.contains("ASH_SHARED_STAGING_OK") })

        worker.close()
    }

    @Test
    fun sharedStagingSmokeFailureKeepsExternalCommandsDisabled() {
        val worker = externalDisabledWorker()
        val stagingRoot = temporaryFolder.newFolder("shared-staging")
        val adapter = FakeExternalAdapter(
            events = listOf(
                ShellStreamEvent.Stderr("nope\n"),
                ShellStreamEvent.Finished(
                    ShellEvalResult(false, "", "null", "Termux shared staging smoke failed", ShellState()),
                ),
            ),
        )
        val viewModel = TerminalViewModel(
            worker = worker,
            termuxBridge = installedBridge(),
            initialState = ShellState(cwd = "/work"),
            externalAdapterFactory = { root -> adapter.apply { bridgeRoot = root } },
        )

        viewModel.updateTermuxStagingPath(stagingRoot.absolutePath)
        viewModel.verifyTermuxSharedStaging()

        assertFalse(worker.externalCommandsEnabled)
        assertEquals(TermuxBridgeState.Installed, viewModel.termuxStatus.state)
        assertTrue(adapter.closed.get())

        worker.close()
    }

    @Test
    fun sharedStagingSmokePermissionFailureShowsStoragePermissionMessage() {
        val worker = externalDisabledWorker()
        val stagingRoot = temporaryFolder.newFolder("shared-staging")
        val adapter = FakeExternalAdapter(
            events = listOf(
                ShellStreamEvent.Stderr("mkdir: cannot create directory '/sdcard': Permission denied\n"),
                ShellStreamEvent.Finished(
                    ShellEvalResult(false, "", "null", "external command exited 73", ShellState()),
                ),
            ),
        )
        val viewModel = TerminalViewModel(
            worker = worker,
            termuxBridge = installedBridge(),
            initialState = ShellState(cwd = "/work"),
            externalAdapterFactory = { root -> adapter.apply { bridgeRoot = root } },
        )

        viewModel.updateTermuxStagingPath(stagingRoot.absolutePath)
        viewModel.verifyTermuxSharedStaging()

        assertFalse(worker.externalCommandsEnabled)
        assertEquals("Termux storage permission required for shared staging", viewModel.termuxStatus.message)

        worker.close()
    }

    @Test
    fun failedHelperBootstrapKeepsExternalCommandsDisabled() {
        val worker = testWorker()
        val bridge = FakeTermuxBridge(
            t0Results = emptyList(),
            bootstrapResult = TermuxCommandResult(stderr = "python3 required\n", exitCode = 86),
        )
        val viewModel = TerminalViewModel(worker, bridge, ShellState(cwd = "/work"))

        viewModel.installTermuxHelper()

        assertFalse(worker.externalCommandsEnabled)
        assertEquals(TermuxBridgeState.Installed, viewModel.termuxStatus.state)
        assertEquals("python3 required", viewModel.termuxStatus.message)

        worker.close()
    }

    private fun testWorker(): ShellWorker =
        ShellWorker(
            bridge = object : ShellBridge {
                override fun evalLine(input: String, state: ShellState): ShellEvalResult =
                    ShellEvalResult(true, "", "null", null, state)
            },
            executor = Executors.newSingleThreadExecutor(),
            resultPoster = ResultPoster { it() },
        )

    private fun externalDisabledWorker(): ShellWorker =
        ShellWorker(
            bridge = object : ShellBridge {
                override fun evalLine(input: String, state: ShellState): ShellEvalResult =
                    ShellEvalResult(
                        ok = false,
                        outputText = "",
                        outputJson = "null",
                        error = "external execution disabled: ${input.substringBefore(' ')}",
                        state = state,
                    )
            },
            executor = Executors.newSingleThreadExecutor(),
            resultPoster = ResultPoster { it() },
        )

    private fun installedBridge(): TermuxBridge =
        FakeTermuxBridge(
            t0Results = emptyList(),
            bootstrapResult = TermuxCommandResult(stdout = "ASH_TERMUX_HELPER_OK\n", exitCode = 0),
        )

    private class FakeExternalAdapter(
        private val events: List<ShellStreamEvent>,
    ) : ExternalShellStreamAdapter, AutoCloseable {
        var bridgeRoot: File? = null
        val closed = AtomicBoolean(false)
        val submittedAfterSmoke = AtomicBoolean(false)

        override fun canHandle(input: String, pureResult: ShellEvalResult): Boolean = true

        override fun submitStreaming(
            input: String,
            state: ShellState,
            eventSink: ShellEventSink,
        ): ShellRunHandle {
            if (input != "sh -c \"echo ASH_SHARED_STAGING_OK\"") {
                submittedAfterSmoke.set(true)
            }
            events.forEach(eventSink::onEvent)
            return AtomicShellRunHandle()
        }

        override fun close() {
            closed.set(true)
        }
    }

    private class FakeTermuxBridge(
        private val t0Results: List<TermuxSmokeResult>,
        private val bootstrapResult: TermuxCommandResult,
    ) : TermuxBridge {
        override fun availability(): TermuxBridgeAvailability =
            TermuxBridgeAvailability(TermuxBridgeState.Installed, "Termux runtime installed; probe required")

        override fun startEchoProbe(callback: TermuxProbeCallback): ShellRunHandle {
            callback.onProbeResult(TermuxCommandResult(stdout = "ASH_TERMUX_OK\n", exitCode = 0))
            return AtomicShellRunHandle()
        }

        override fun startT0Smoke(callback: TermuxSmokeCallback): ShellRunHandle {
            callback.onSmokeComplete(t0Results)
            return AtomicShellRunHandle()
        }

        override fun startHelperBootstrap(callback: TermuxProbeCallback): ShellRunHandle {
            callback.onProbeResult(bootstrapResult)
            return AtomicShellRunHandle()
        }
    }
}
