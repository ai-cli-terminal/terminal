package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Test

class WorkspaceDocumentsTest {
    @Test
    fun sanitizeWorkspaceFileNameKeepsImportInsideWorkspace() {
        assertEquals("passwd", sanitizeWorkspaceFileName("../../etc/passwd"))
        assertEquals("notes.txt", sanitizeWorkspaceFileName("notes.txt"))
        assertEquals("hello_world.md", sanitizeWorkspaceFileName("hello world.md"))
        assertEquals("imported-document", sanitizeWorkspaceFileName("../.."))
    }
}
