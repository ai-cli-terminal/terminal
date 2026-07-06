package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.Rule
import java.nio.file.Files

class WorkspaceDocumentsTest {
    @Rule
    @JvmField
    val temporaryFolder = TemporaryFolder()

    @Test
    fun sanitizeWorkspaceFileNameKeepsImportInsideWorkspace() {
        assertEquals("passwd", sanitizeWorkspaceFileName("../../etc/passwd"))
        assertEquals("notes.txt", sanitizeWorkspaceFileName("notes.txt"))
        assertEquals("hello_world.md", sanitizeWorkspaceFileName("hello world.md"))
        assertEquals("imported-document", sanitizeWorkspaceFileName("../.."))
    }

    @Test
    fun previewWorkspaceDocumentReturnsTextPreview() {
        val file = Files.createTempFile("workspace-preview", ".txt").toFile()
        try {
            file.writeText("alpha\nbeta\ncharlie")

            val preview = requireNotNull(previewWorkspaceDocument(file))

            assertEquals("alpha\nbeta\ncharlie", preview.text)
            assertFalse(preview.truncated)
            assertEquals(3, preview.linesRead)
            assertEquals("alpha\nbeta\ncharlie".toByteArray(Charsets.UTF_8).size, preview.bytesRead)
        } finally {
            file.delete()
        }
    }

    @Test
    fun previewWorkspaceDocumentTruncatesLongText() {
        val file = Files.createTempFile("workspace-preview", ".txt").toFile()
        try {
            file.writeText("one\ntwo\nthree\nfour")

            val preview = requireNotNull(previewWorkspaceDocument(file, maxBytes = 1024, maxLines = 2))

            assertEquals("one\ntwo", preview.text)
            assertTrue(preview.truncated)
        } finally {
            file.delete()
        }
    }

    @Test
    fun previewWorkspaceDocumentReportsEmptyTextWithoutPhantomLine() {
        val file = Files.createTempFile("workspace-preview", ".txt").toFile()
        try {
            file.writeText("")

            val preview = requireNotNull(previewWorkspaceDocument(file))

            assertEquals("", preview.text)
            assertEquals(0, preview.linesRead)
            assertEquals(0, preview.bytesRead)
            assertFalse(preview.truncated)
        } finally {
            file.delete()
        }
    }

    @Test
    fun previewWorkspaceDocumentSkipsBinaryContent() {
        val file = Files.createTempFile("workspace-preview", ".bin").toFile()
        try {
            file.writeBytes(byteArrayOf(0x41, 0x00, 0x42))

            assertNull(previewWorkspaceDocument(file))
        } finally {
            file.delete()
        }
    }

    @Test
    fun openWorkspaceDocumentReadOnlyReturnsTextInsideWorkspace() {
        val root = temporaryFolder.newFolder("workspace")
        val file = root.resolve("notes.txt")
        file.writeText("alpha\nbeta")

        val opened = openWorkspaceDocumentReadOnly(
            file.absolutePath,
            ShellState(cwd = root.absolutePath, workspaceRoot = root.absolutePath),
        )

        assertEquals("notes.txt", opened.fileName)
        assertEquals(WorkspaceDocumentContentKind.Text, opened.contentKind)
        assertEquals(file.length(), opened.bytes)
        val preview = requireNotNull(opened.preview)
        assertEquals("alpha\nbeta", preview.text)
        assertFalse(preview.truncated)
    }

    @Test
    fun openWorkspaceDocumentReadOnlyRejectsPathOutsideWorkspace() {
        val root = temporaryFolder.newFolder("workspace")
        val outside = temporaryFolder.newFile("outside.txt")

        val result = runCatching {
            openWorkspaceDocumentReadOnly(
                outside.absolutePath,
                ShellState(cwd = root.absolutePath, workspaceRoot = root.absolutePath),
            )
        }

        assertTrue(result.isFailure)
        assertEquals("document is outside workspace", result.exceptionOrNull()?.message)
    }

    @Test
    fun openWorkspaceDocumentReadOnlySummarizesBinaryContent() {
        val root = temporaryFolder.newFolder("workspace")
        val file = root.resolve("image.bin")
        file.writeBytes(byteArrayOf(0x41, 0x00, 0x42))

        val opened = openWorkspaceDocumentReadOnly(
            file.absolutePath,
            ShellState(cwd = root.absolutePath, workspaceRoot = root.absolutePath),
        )

        assertEquals("image.bin", opened.fileName)
        assertEquals(WorkspaceDocumentContentKind.BinaryOrUnsupported, opened.contentKind)
        assertEquals(3, opened.bytes)
        assertNull(opened.preview)
    }

    @Test
    fun prepareWorkspaceDocumentExportReturnsFileInsideWorkspace() {
        val root = temporaryFolder.newFolder("workspace")
        val file = root.resolve("notes.txt")
        file.writeText("alpha")

        val source = prepareWorkspaceDocumentExport(
            file.absolutePath,
            ShellState(cwd = root.absolutePath, workspaceRoot = root.absolutePath),
        )

        assertEquals(file.canonicalFile, source.file)
        assertEquals("notes.txt", source.fileName)
        assertEquals(file.length(), source.bytes)
    }

    @Test
    fun prepareWorkspaceDocumentExportRejectsPathOutsideWorkspace() {
        val root = temporaryFolder.newFolder("workspace")
        val outside = temporaryFolder.newFile("outside.txt")

        val result = runCatching {
            prepareWorkspaceDocumentExport(
                outside.absolutePath,
                ShellState(cwd = root.absolutePath, workspaceRoot = root.absolutePath),
            )
        }

        assertTrue(result.isFailure)
        assertEquals("document is outside workspace", result.exceptionOrNull()?.message)
    }

    @Test
    fun prepareWorkspaceDocumentExportRejectsDirectory() {
        val root = temporaryFolder.newFolder("workspace")
        val directory = root.resolve("nested")
        assertTrue(directory.mkdirs())

        val result = runCatching {
            prepareWorkspaceDocumentExport(
                directory.absolutePath,
                ShellState(cwd = root.absolutePath, workspaceRoot = root.absolutePath),
            )
        }

        assertTrue(result.isFailure)
        assertEquals("document is not a file", result.exceptionOrNull()?.message)
    }

    @Test
    fun selectedWorkspaceDocumentListCommandUsesRelativeWorkspacePath() {
        val root = temporaryFolder.newFolder("workspace")
        val file = root.resolve("notes.txt")
        file.writeText("alpha")

        val prepared = selectedWorkspaceDocumentListCommand(
            file.absolutePath,
            ShellState(cwd = root.absolutePath, workspaceRoot = root.absolutePath),
        )

        assertEquals("notes.txt", prepared.fileName)
        assertEquals("""ls "." | where name == "notes.txt" | first 1""", prepared.command)
    }

    @Test
    fun selectedWorkspaceDocumentListCommandWorksFromNestedCwdWithoutAbsolutePath() {
        val root = temporaryFolder.newFolder("workspace")
        val child = root.resolve("child")
        assertTrue(child.mkdirs())
        val file = root.resolve("notes.txt")
        file.writeText("alpha")

        val prepared = selectedWorkspaceDocumentListCommand(
            file.absolutePath,
            ShellState(cwd = child.absolutePath, workspaceRoot = root.absolutePath),
        )

        assertEquals("""ls ".." | where name == "notes.txt" | first 1""", prepared.command)
        assertFalse(prepared.command.contains(root.absolutePath))
    }

    @Test
    fun selectedWorkspaceDocumentListCommandRejectsOutsideWorkspaceCwd() {
        val root = temporaryFolder.newFolder("workspace")
        val outsideCwd = temporaryFolder.newFolder("outside-cwd")
        val file = root.resolve("notes.txt")
        file.writeText("alpha")

        val result = runCatching {
            selectedWorkspaceDocumentListCommand(
                file.absolutePath,
                ShellState(cwd = outsideCwd.absolutePath, workspaceRoot = root.absolutePath),
            )
        }

        assertTrue(result.isFailure)
        assertEquals("current directory is outside workspace", result.exceptionOrNull()?.message)
    }

    @Test
    fun shellcoreStringLiteralUsesSingleQuotesWhenNeeded() {
        assertEquals("'quote\"inside'", shellcoreStringLiteral("quote\"inside"))
    }

    @Test
    fun shellcoreStringLiteralRejectsMultilineValues() {
        val result = runCatching {
            shellcoreStringLiteral("bad\nvalue")
        }

        assertTrue(result.isFailure)
        assertEquals(
            "value cannot be represented as a single-line shellcore string",
            result.exceptionOrNull()?.message,
        )
    }
}
