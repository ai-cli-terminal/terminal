package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

class ShellWorkerTest {
    @Test
    fun submitEvaluatesOnWorkerThreadAndPostsResult() {
        val bridgeThreadName = AtomicReference<String>()
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult {
                bridgeThreadName.set(Thread.currentThread().name)
                return ShellEvalResult(
                    ok = true,
                    outputText = "ok:$input",
                    outputJson = "\"ok\"",
                    error = null,
                    state = state.copy(exitCode = 0),
                )
            }
        }

        val executor = Executors.newSingleThreadExecutor { task ->
            Thread(task, "shell-worker-test")
        }
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )

        val callbackCalled = AtomicBoolean(false)
        val callbackResult = AtomicReference<ShellEvalResult>()

        worker.submit("length", ShellState()) { result ->
            callbackCalled.set(true)
            callbackResult.set(result)
        }

        posted.take().invoke()
        posted.take().invoke()

        assertEquals("shell-worker-test", bridgeThreadName.get())
        assertTrue(callbackCalled.get())
        assertEquals("ok:length", callbackResult.get().outputText)
        assertEquals(0, callbackResult.get().state.exitCode)

        worker.close()
    }

    @Test
    fun submitConvertsBridgeFailureToErrorResult() {
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult {
                error("boom")
            }
        }
        val executor = Executors.newSingleThreadExecutor()
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )
        val callbackResult = AtomicReference<ShellEvalResult>()

        worker.submit("broken", ShellState(cwd = "/app/work")) { result ->
            callbackResult.set(result)
        }

        posted.take().invoke()
        posted.take().invoke()

        val result = callbackResult.get()
        assertFalse(result.ok)
        assertEquals("", result.outputText)
        assertEquals("boom", result.error)
        assertEquals("/app/work", result.state.cwd)

        worker.close()
    }

    @Test
    fun submitStreamingEmitsStartedStdoutAndFinishedInOrder() {
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult {
                return ShellEvalResult(
                    ok = true,
                    outputText = "streamed",
                    outputJson = "\"streamed\"",
                    error = null,
                    state = state.copy(exitCode = 0),
                )
            }
        }
        val executor = Executors.newSingleThreadExecutor()
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )
        val events = mutableListOf<ShellStreamEvent>()

        val handle = worker.submitStreaming("echo", ShellState()) { event ->
            events += event
        }

        assertFalse(handle.isCancelled)
        posted.take().invoke()
        posted.take().invoke()

        assertEquals(ShellStreamEvent.Started("echo", ShellState()), events[0])
        assertEquals(ShellStreamEvent.Stdout("streamed"), events[1])
        assertTrue(events[2] is ShellStreamEvent.Finished)

        worker.close()
    }

    @Test
    fun cancelBeforeCompletionSuppressesFinalResult() {
        val bridgeEntered = CountDownLatch(1)
        val releaseBridge = CountDownLatch(1)
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult {
                bridgeEntered.countDown()
                assertTrue(releaseBridge.await(2, TimeUnit.SECONDS))
                return ShellEvalResult(
                    ok = true,
                    outputText = "late",
                    outputJson = "\"late\"",
                    error = null,
                    state = state,
                )
            }
        }
        val executor = Executors.newSingleThreadExecutor()
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )
        val events = mutableListOf<ShellStreamEvent>()

        val handle = worker.submitStreaming("slow", ShellState()) { event ->
            events += event
        }
        posted.take().invoke()
        assertTrue(bridgeEntered.await(2, TimeUnit.SECONDS))
        handle.cancel()
        releaseBridge.countDown()
        posted.take().invoke()

        assertTrue(handle.isCancelled)
        assertEquals(ShellStreamEvent.Started("slow", ShellState()), events[0])
        assertTrue(events[1] is ShellStreamEvent.Cancelled)
        assertEquals(2, events.size)

        worker.close()
    }

    @Test
    fun externalDisabledResultRoutesToEnabledExternalAdapter() {
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult =
                ShellEvalResult(
                    ok = false,
                    outputText = "",
                    outputJson = "null",
                    error = "external execution disabled: echo",
                    state = state,
                )
        }
        val externalCalled = AtomicBoolean(false)
        val external = object : ExternalShellStreamAdapter {
            override fun canHandle(input: String, pureResult: ShellEvalResult): Boolean = true

            override fun submitStreaming(
                input: String,
                state: ShellState,
                eventSink: ShellEventSink,
            ): ShellRunHandle {
                externalCalled.set(true)
                eventSink.onEvent(ShellStreamEvent.Stdout("from-termux"))
                eventSink.onEvent(
                    ShellStreamEvent.Finished(
                        ShellEvalResult(true, "", "null", null, state.copy(exitCode = 0)),
                    ),
                )
                return AtomicShellRunHandle()
            }
        }
        val executor = Executors.newSingleThreadExecutor()
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            externalAdapter = external,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )
        worker.externalCommandsEnabled = true
        val externalFinished = CountDownLatch(1)
        val events = mutableListOf<ShellStreamEvent>()

        worker.submitStreaming("echo hi", ShellState()) { event ->
            events += event
            if (event is ShellStreamEvent.Finished) externalFinished.countDown()
        }
        posted.take().invoke()
        assertTrue(externalFinished.await(2, TimeUnit.SECONDS))

        assertTrue(externalCalled.get())
        assertEquals(ShellStreamEvent.Started("echo hi", ShellState()), events[0])
        assertEquals(ShellStreamEvent.Stdout("from-termux"), events[1])
        assertTrue(events[2] is ShellStreamEvent.Finished)

        worker.close()
    }

    @Test
    fun externalDisabledResultDoesNotRouteWhenExternalIsDisabled() {
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult =
                ShellEvalResult(
                    ok = false,
                    outputText = "",
                    outputJson = "null",
                    error = "external execution disabled: echo",
                    state = state,
                )
        }
        val externalCalled = AtomicBoolean(false)
        val external = object : ExternalShellStreamAdapter {
            override fun canHandle(input: String, pureResult: ShellEvalResult): Boolean = true
            override fun submitStreaming(input: String, state: ShellState, eventSink: ShellEventSink): ShellRunHandle {
                externalCalled.set(true)
                return AtomicShellRunHandle()
            }
        }
        val executor = Executors.newSingleThreadExecutor()
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            externalAdapter = external,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )
        val events = mutableListOf<ShellStreamEvent>()

        worker.submitStreaming("echo hi", ShellState()) { events += it }
        posted.take().invoke()
        posted.take().invoke()

        assertFalse(externalCalled.get())
        assertEquals(ShellStreamEvent.Stderr("external execution disabled: echo"), events[1])
        assertTrue(events[2] is ShellStreamEvent.Finished)

        worker.close()
    }
}
