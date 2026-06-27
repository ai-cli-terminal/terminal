package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class TermuxHelperJobFilesTest {
    @Rule
    @JvmField
    val temporaryFolder = TemporaryFolder()

    @Test
    fun jobFilesUseStableHelperNames() {
        val files = TermuxHelperJobFiles(temporaryFolder.newFolder("job-1"))

        assertEquals("request.json", files.requestFile.name)
        assertEquals("argv", files.argvDir.name)
        assertEquals("events.ndjson", files.eventsFile.name)
        assertEquals("cancel", files.cancelFile.name)
        assertEquals("exit.json", files.exitFile.name)
    }

    @Test
    fun pollerIgnoresMissingEventFile() {
        val files = TermuxHelperJobFiles(temporaryFolder.newFolder("job-1"))
        val poller = TermuxHelperEventFilePoller(files, input = "cmd", state = ShellState())

        assertTrue(poller.poll().isEmpty())
        assertFalse(poller.isTerminal)
    }

    @Test
    fun pollerReadsOnlyNewCompleteLinesInOrder() {
        val files = TermuxHelperJobFiles(temporaryFolder.newFolder("job-1"))
        val state = ShellState(cwd = "/work")
        val poller = TermuxHelperEventFilePoller(files, input = "cmd", state = state)

        files.eventsFile.appendText("""{"seq":1,"type":"started","pid":11}""" + "\n")
        files.eventsFile.appendText("""{"seq":2,"type":"stdout","text":"hello\n"}""" + "\n")

        val firstPoll = poller.poll()

        assertEquals(ShellStreamEvent.Started("cmd", state), firstPoll[0])
        assertEquals(ShellStreamEvent.Stdout("hello\n"), firstPoll[1])
        assertTrue(poller.poll().isEmpty())

        files.eventsFile.appendText("""{"seq":3,"type":"stderr","text":"warn\n"}""" + "\n")
        files.eventsFile.appendText("""{"seq":4,"type":"finished","exit_code":0}""" + "\n")

        val secondPoll = poller.poll()

        assertEquals(ShellStreamEvent.Stderr("warn\n"), secondPoll[0])
        assertTrue(secondPoll[1] is ShellStreamEvent.Finished)
        assertTrue(poller.isTerminal)
    }

    @Test
    fun pollerWaitsForPartialLineBeforeDecoding() {
        val files = TermuxHelperJobFiles(temporaryFolder.newFolder("job-1"))
        val poller = TermuxHelperEventFilePoller(files, input = "cmd", state = ShellState())

        files.eventsFile.writeText("""{"seq":1,"type":"stdout","text":"hel""")
        assertTrue(poller.poll().isEmpty())

        files.eventsFile.appendText("""lo\n"}""" + "\n")

        assertEquals(ShellStreamEvent.Stdout("hello\n"), poller.poll().single())
    }

    @Test
    fun pollerStopsAfterTerminalEvent() {
        val files = TermuxHelperJobFiles(temporaryFolder.newFolder("job-1"))
        val poller = TermuxHelperEventFilePoller(files, input = "cmd", state = ShellState())

        files.eventsFile.writeText("""{"seq":1,"type":"finished","exit_code":0}""" + "\n")
        assertTrue(poller.poll().single() is ShellStreamEvent.Finished)

        files.eventsFile.appendText("""{"seq":2,"type":"stdout","text":"late"}""" + "\n")
        assertTrue(poller.poll().isEmpty())
    }

    @Test
    fun pollerResetsOffsetWhenEventFileIsTruncated() {
        val files = TermuxHelperJobFiles(temporaryFolder.newFolder("job-1"))
        val poller = TermuxHelperEventFilePoller(files, input = "cmd", state = ShellState())

        files.eventsFile.writeText("""{"seq":1,"type":"stdout","text":"old"}""" + "\n")
        assertEquals(ShellStreamEvent.Stdout("old"), poller.poll().single())

        files.eventsFile.writeText("""{"type":"stdout","text":"n"}""" + "\n")

        assertEquals(ShellStreamEvent.Stdout("n"), poller.poll().single())
    }

    @Test
    fun cancelHandleWritesCancelFileOnce() {
        val files = TermuxHelperJobFiles(temporaryFolder.newFolder("job-1"))
        val handle = FileBackedShellRunHandle(files.cancelFile)

        assertFalse(handle.isCancelled)

        handle.cancel()
        handle.cancel()

        assertTrue(handle.isCancelled)
        assertTrue(files.cancelFile.isFile)
        assertEquals("user\n", files.cancelFile.readText())
    }
}
