package dev.aiterminal.android

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

class TermuxHelperBridgeAdapterTest {
    @Rule
    @JvmField
    val temporaryFolder = TemporaryFolder()

    @Test
    fun parseExternalArgvKeepsQuotedArgumentsAndRejectsShellOperators() {
        assertEquals(
            listOf("grep", "-n", "hello world", "file.txt"),
            parseExternalArgv("""grep -n "hello world" file.txt"""),
        )
        assertEquals(null, parseExternalArgv("ls | get name"))
        assertEquals(null, parseExternalArgv("echo hi; rm x"))
        assertEquals(null, parseExternalArgv("""echo "unterminated"""))
    }

    @Test
    fun adapterWritesRequestStartsHelperAndStreamsEvents() {
        val bridgeRoot = temporaryFolder.newFolder("bridge")
        val scheduler = Executors.newSingleThreadScheduledExecutor()
        val launchedJobDir = AtomicReference<File>()
        val adapter = TermuxHelperBridgeAdapter(
            bridgeRoot = bridgeRoot,
            launcher = TermuxHelperLauncher { jobDir, _ ->
                launchedJobDir.set(jobDir)
                TermuxHelperJobFiles(jobDir).eventsFile.appendText(
                    """
                    {"seq":1,"type":"started","pid":1}
                    {"seq":2,"type":"stdout","text":"hello\n"}
                    {"seq":3,"type":"finished","exit_code":0}
                    """.trimIndent() + "\n",
                )
                AtomicShellRunHandle()
            },
            resultPoster = ResultPoster { it() },
            scheduler = scheduler,
            pollIntervalMs = 10,
        )
        val terminal = CountDownLatch(1)
        val events = mutableListOf<ShellStreamEvent>()

        adapter.submitStreaming("echo hello", ShellState(cwd = "/work")) { event ->
            events += event
            if (event is ShellStreamEvent.Finished) terminal.countDown()
        }

        assertTrue(terminal.await(2, TimeUnit.SECONDS))
        scheduler.shutdownNow()

        val request = JSONObject(TermuxHelperJobFiles(launchedJobDir.get()).requestFile.readText())
        assertEquals("echo", request.getJSONArray("argv").getString(0))
        assertEquals("hello", request.getJSONArray("argv").getString(1))
        assertEquals("/work", request.getString("cwd"))
        assertEquals("echo", TermuxHelperJobFiles(launchedJobDir.get()).argvDir.resolve("0000").readText())
        assertEquals("hello", TermuxHelperJobFiles(launchedJobDir.get()).argvDir.resolve("0001").readText())
        assertEquals(ShellStreamEvent.Stdout("hello\n"), events[0])
        assertTrue(events[1] is ShellStreamEvent.Finished)
    }

    @Test
    fun adapterCancelWritesCancelMarkerAndCancelsLauncherHandle() {
        val bridgeRoot = temporaryFolder.newFolder("bridge")
        val scheduler = Executors.newSingleThreadScheduledExecutor()
        val launchedJobDir = AtomicReference<File>()
        val launcherHandle = AtomicShellRunHandle()
        val adapter = TermuxHelperBridgeAdapter(
            bridgeRoot = bridgeRoot,
            launcher = TermuxHelperLauncher { jobDir, _ ->
                launchedJobDir.set(jobDir)
                launcherHandle
            },
            resultPoster = ResultPoster { it() },
            scheduler = scheduler,
            pollIntervalMs = 10,
        )

        val handle = adapter.submitStreaming("sleep 10", ShellState()) {}
        handle.cancel()

        scheduler.shutdownNow()
        val files = TermuxHelperJobFiles(launchedJobDir.get())
        assertTrue(handle.isCancelled)
        assertTrue(launcherHandle.isCancelled)
        assertEquals("user\n", files.cancelFile.readText())
    }

    @Test
    fun adapterReportsHelperLaunchFailureWhenNoEventsArrive() {
        val scheduler = Executors.newSingleThreadScheduledExecutor()
        val adapter = TermuxHelperBridgeAdapter(
            bridgeRoot = temporaryFolder.newFolder("bridge"),
            launcher = TermuxHelperLauncher { _, callback ->
                callback.onProbeResult(TermuxCommandResult(stderr = "missing helper\n", exitCode = 127))
                AtomicShellRunHandle()
            },
            resultPoster = ResultPoster { it() },
            scheduler = scheduler,
            pollIntervalMs = 10,
        )
        val events = mutableListOf<ShellStreamEvent>()

        adapter.submitStreaming("missing", ShellState()) { events += it }
        scheduler.shutdownNow()

        assertEquals(ShellStreamEvent.Stderr("missing helper"), events[0])
        assertTrue(events[1] is ShellStreamEvent.Finished)
        assertFalse((events[1] as ShellStreamEvent.Finished).result.ok)
    }
}
