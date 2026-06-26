package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.file.Files

class WorkspaceDocumentsTest {
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
    fun previewWorkspaceDocumentSkipsBinaryContent() {
        val file = Files.createTempFile("workspace-preview", ".bin").toFile()
        try {
            file.writeBytes(byteArrayOf(0x41, 0x00, 0x42))

            assertNull(previewWorkspaceDocument(file))
        } finally {
            file.delete()
        }
    }
}
