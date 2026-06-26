package dev.aiterminal.android

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(AndroidJUnit4::class)
class TermuxHelperRealDeviceSmokeTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext

    @Test
    fun helperBootstrapAndEventFileSmokes() {
        assumeRealDeviceSmokeEnabled()
        val bridge = AndroidTermuxBridge(context)
        val availability = bridge.availability()
        assertEquals(availability.message, TermuxBridgeState.Installed, availability.state)

        val bootstrap = runProbe { callback ->
            bridge.startHelperBootstrap(callback)
        }
        assertTrue(bootstrap.stderr + bootstrap.internalError, bootstrap.ok)
        assertTrue(bootstrap.stdout, bootstrap.stdout.contains(TermuxHelperBootstrapContract.SELF_TEST_MARKER))

        val stdoutEvents = runHelperJob(
            argv = listOf(
                "sh",
                "-c",
                "i=1; while [ \$i -le 5 ]; do echo OUT-\$i; i=\$((i+1)); sleep 0.1; done",
            ),
        )
        assertTrue(stdoutEvents.any { it is ShellStreamEvent.Stdout && it.text.contains("OUT-5") })
        assertTrue(stdoutEvents.last() is ShellStreamEvent.Finished)
        assertTrue((stdoutEvents.last() as ShellStreamEvent.Finished).result.ok)

        val stderrEvents = runHelperJob(
            argv = listOf("sh", "-c", "echo ERR-SMOKE >&2; exit 7"),
        )
        assertTrue(stderrEvents.any { it is ShellStreamEvent.Stderr && it.text.contains("ERR-SMOKE") })
        val stderrFinished = stderrEvents.last() as ShellStreamEvent.Finished
        assertFalse(stderrFinished.result.ok)
        assertEquals(7, stderrFinished.result.state.exitCode)

        val largeEvents = runHelperJob(
            argv = listOf(
                "sh",
                "-c",
                "i=1; while [ \$i -le 1500 ]; do echo LARGE-\$i; i=\$((i+1)); done",
            ),
            timeoutSeconds = 20,
        )
        val largeText = largeEvents
            .filterIsInstance<ShellStreamEvent.Stdout>()
            .joinToString(separator = "") { it.text }
        assertTrue(largeText.length > 10_000)
        assertTrue(largeText.contains("LARGE-1500"))
        assertTrue(largeEvents.last() is ShellStreamEvent.Finished)
    }

    @Test
    fun helperCancelSmoke() {
        assumeRealDeviceSmokeEnabled()
        val bridge = AndroidTermuxBridge(context)
        val bootstrap = runProbe { callback ->
            bridge.startHelperBootstrap(callback)
        }
        assertTrue(bootstrap.stderr + bootstrap.internalError, bootstrap.ok)

        val files = newJobFiles()
        files.jobDir.mkdirs()
        files.eventsFile.writeText("", Charsets.UTF_8)
        writeFallbackArgvFiles(files, listOf("sh", "-c", "while true; do echo TICK; sleep 0.2; done"))
        files.requestFile.writeText(
            TermuxHelperProtocol.encodeRequest(
                TermuxHelperRequest(
                    argv = listOf("sh", "-c", "while true; do echo TICK; sleep 0.2; done"),
                    cwd = files.jobDir.absolutePath,
                ),
            ),
            Charsets.UTF_8,
        )

        val launcher = AndroidTermuxHelperLauncher(context)
        val handle = FileBackedShellRunHandle(files.cancelFile)
        val launchHandle = launcher.startHelper(files.jobDir) {}
        val poller = TermuxHelperEventFilePoller(files, "cancel-smoke", ShellState(cwd = files.jobDir.absolutePath))
        val events = mutableListOf<ShellStreamEvent>()

        assertTrue(waitUntil(timeoutSeconds = 10) {
            events += poller.poll()
            events.any { it is ShellStreamEvent.Stdout && it.text.contains("TICK") }
        })

        handle.cancel()
        assertTrue(waitUntil(timeoutSeconds = 10) {
            events += poller.poll()
            events.any { it is ShellStreamEvent.Cancelled }
        })
        launchHandle.cancel()

        assertTrue(files.cancelFile.isFile)
        assertTrue(events.any { it is ShellStreamEvent.Cancelled })
    }

    private fun runHelperJob(
        argv: List<String>,
        timeoutSeconds: Long = 10,
    ): List<ShellStreamEvent> {
        val files = newJobFiles()
        files.jobDir.mkdirs()
        files.eventsFile.writeText("", Charsets.UTF_8)
        writeFallbackArgvFiles(files, argv)
        files.requestFile.writeText(
            TermuxHelperProtocol.encodeRequest(
                TermuxHelperRequest(
                    argv = argv,
                    cwd = files.jobDir.absolutePath,
                    env = mapOf("TERM" to "xterm-256color"),
                ),
            ),
            Charsets.UTF_8,
        )

        val launcher = AndroidTermuxHelperLauncher(context)
        val launchFailure = mutableListOf<TermuxCommandResult>()
        val launchHandle = launcher.startHelper(files.jobDir) { result ->
            if (!result.ok) {
                launchFailure += result
            }
        }
        val poller = TermuxHelperEventFilePoller(files, argv.joinToString(" "), ShellState(cwd = files.jobDir.absolutePath))
        val events = mutableListOf<ShellStreamEvent>()

        val completed = waitUntil(timeoutSeconds) {
            events += poller.poll()
            events.any { it is ShellStreamEvent.Finished || it is ShellStreamEvent.Cancelled }
        }
        launchHandle.cancel()

        assertTrue(
            "helper job did not complete; launchFailure=$launchFailure events=$events files=${files.jobDir}",
            completed,
        )
        assertTrue("expected shared bridge events file at ${files.eventsFile}", files.eventsFile.isFile)
        return events
    }

    private fun writeFallbackArgvFiles(files: TermuxHelperJobFiles, argv: List<String>) {
        files.argvDir.mkdirs()
        argv.forEachIndexed { index, arg ->
            File(files.argvDir, index.toString().padStart(4, '0'))
                .writeText(arg, Charsets.UTF_8)
        }
    }

    private fun runProbe(start: (TermuxProbeCallback) -> ShellRunHandle): TermuxCommandResult {
        val latch = CountDownLatch(1)
        var result: TermuxCommandResult? = null
        val handle = start(
            TermuxProbeCallback {
                result = it
                latch.countDown()
            },
        )
        assertTrue(latch.await(30, TimeUnit.SECONDS))
        handle.cancel()
        return requireNotNull(result)
    }

    private fun waitUntil(timeoutSeconds: Long, condition: () -> Boolean): Boolean {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(timeoutSeconds)
        while (System.nanoTime() < deadline) {
            if (condition()) return true
            Thread.sleep(100)
        }
        return condition()
    }

    private fun newJobFiles(): TermuxHelperJobFiles {
        val bridgeRoot = manualSharedStagingDir()
        return TermuxHelperJobFiles(
            File(File(bridgeRoot, "instrumentation-jobs"), "job-${UUID.randomUUID()}"),
        )
    }

    private fun assumeRealDeviceSmokeEnabled() {
        val args = InstrumentationRegistry.getArguments()
        assumeTrue(
            "manual Termux real-device smoke; run with -e termuxRealDeviceSmoke true",
            args.getString("termuxRealDeviceSmoke") == "true",
        )
        assumeTrue(
            "manual Termux real-device smoke requires -e termuxBridgeStagingDir <shared-dir>",
            !args.getString("termuxBridgeStagingDir").isNullOrBlank(),
        )
    }

    private fun manualSharedStagingDir(): File {
        val args = InstrumentationRegistry.getArguments()
        val path = requireNotNull(args.getString("termuxBridgeStagingDir")) {
            "termuxBridgeStagingDir is required"
        }
        val dir = File(path)
        assertTrue("shared staging dir must be writable by the app: $dir", dir.mkdirs() || dir.isDirectory)
        assertTrue("shared staging dir must be writable by the app: $dir", dir.canWrite())
        return dir
    }
}
