package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.BlockingQueue
import java.util.concurrent.CopyOnWriteArrayList
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
        val recorder = StreamEventRecorder()

        val handle = worker.submitStreaming("echo", ShellState(), recorder)

        assertFalse(handle.isCancelled)
        drainPostedUntilTerminal(posted, recorder)

        val events = recorder.snapshot()
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
        val recorder = StreamEventRecorder()

        val handle = worker.submitStreaming("slow", ShellState(), recorder)
        // Drain the Started block before cancelling so it is recorded first,
        // mirroring the real delivery order.
        posted.take().invoke()
        assertTrue(bridgeEntered.await(2, TimeUnit.SECONDS))
        handle.cancel()
        releaseBridge.countDown()
        drainPostedUntilTerminal(posted, recorder)

        val events = recorder.snapshot()
        assertTrue(handle.isCancelled)
        assertEquals(ShellStreamEvent.Started("slow", ShellState()), events[0])
        assertTrue(events[1] is ShellStreamEvent.Cancelled)
        assertEquals(2, events.size)

        worker.close()
    }

    @Test
    fun externalDisabledResultRoutesToEnabledExternalAdapter() {
        // Gate the bridge so the test thread records Started (delivered via the
        // posted queue) before the external adapter emits Stdout/Finished
        // directly on the executor thread, making the event order deterministic.
        val releaseBridge = CountDownLatch(1)
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult {
                assertTrue(releaseBridge.await(2, TimeUnit.SECONDS))
                return ShellEvalResult(
                    ok = false,
                    outputText = "",
                    outputJson = "null",
                    error = "external execution disabled: echo",
                    state = state,
                )
            }
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
        val recorder = StreamEventRecorder()

        worker.submitStreaming("echo hi", ShellState(), recorder)
        // Started is enqueued before the executor runs; record it first, then
        // release the bridge so the adapter's events are appended afterwards.
        posted.take().invoke()
        releaseBridge.countDown()
        recorder.awaitTerminal()

        val events = recorder.snapshot()
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
        val recorder = StreamEventRecorder()

        worker.submitStreaming("echo hi", ShellState(), recorder)
        drainPostedUntilTerminal(posted, recorder)

        val events = recorder.snapshot()
        assertFalse(externalCalled.get())
        assertEquals(ShellStreamEvent.Stderr("external execution disabled: echo"), events[1])
        assertTrue(events[2] is ShellStreamEvent.Finished)

        worker.close()
    }
}

/**
 * Thread-safe [ShellEventSink] for tests. Events may be appended from the test
 * thread (posted-block callbacks) and from the worker/executor thread (external
 * adapters that emit directly), so collection must tolerate concurrent writes.
 *
 * A [CopyOnWriteArrayList] guarantees each append is atomic and that [snapshot]
 * never observes a partially-written list, while the [terminal] latch (tripped on
 * the first [ShellStreamEvent.Finished]/[ShellStreamEvent.Cancelled]) establishes
 * a happens-before edge: a caller that returns from [awaitTerminal] is guaranteed
 * to see every event recorded up to and including the terminal one.
 */
private class StreamEventRecorder : ShellEventSink {
    private val events = CopyOnWriteArrayList<ShellStreamEvent>()
    private val terminal = CountDownLatch(1)

    override fun onEvent(event: ShellStreamEvent) {
        events += event
        if (event is ShellStreamEvent.Finished || event is ShellStreamEvent.Cancelled) {
            terminal.countDown()
        }
    }

    /** True until the terminal (Finished/Cancelled) event has been observed. */
    fun isPending(): Boolean = terminal.count > 0L

    /** Block until the terminal event is observed, failing the test on timeout. */
    fun awaitTerminal() {
        assertTrue(
            "terminal stream event was not observed within timeout",
            terminal.await(2, TimeUnit.SECONDS),
        )
    }

    /** Immutable copy of the events recorded so far, safe to index into. */
    fun snapshot(): List<ShellStreamEvent> = events.toList()
}

/**
 * Invoke posted blocks until [recorder] observes its terminal event, then await
 * that terminal event. Uses a bounded [BlockingQueue.poll] rather than a fixed
 * number of `take()` calls so the drain works whether the terminal event arrives
 * via the posted queue (in-process delivery) or directly from the executor thread
 * (external adapters) — and never blocks forever waiting for a block that will
 * not come.
 */
private fun drainPostedUntilTerminal(
    posted: BlockingQueue<() -> Unit>,
    recorder: StreamEventRecorder,
) {
    while (recorder.isPending()) {
        val block = posted.poll(2, TimeUnit.SECONDS) ?: break
        block.invoke()
    }
    recorder.awaitTerminal()
}
