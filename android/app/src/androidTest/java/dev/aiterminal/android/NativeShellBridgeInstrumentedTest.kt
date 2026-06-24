package dev.aiterminal.android

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeShellBridgeInstrumentedTest {
    @Test
    fun nativeBridgeLoadsLibraryAndEvaluatesPureShellcore() {
        val bridge = NativeShellBridge()
        val result = bridge.evalLine(
            "[{size: 50} {size: 200}] | where size > 100",
            shellState(),
        )

        assertTrue(result.error ?: "expected native eval ok", result.ok)
        assertNull(result.error)
        assertEquals("[{\"size\":200}]", result.outputJson)
        assertTrue(result.outputText.contains("200"))
        assertFalse(result.outputText.contains("50"))
    }

    @Test
    fun nativeBridgePersistsSessionStateAcrossCalls() {
        val bridge = NativeShellBridge()
        val first = bridge.evalLine("let limit = 100", shellState())
        assertTrue(first.error ?: "expected let ok", first.ok)

        val second = bridge.evalLine("[{size: 200}] | where size > \$limit | length", first.state)

        assertTrue(second.error ?: "expected stateful eval ok", second.ok)
        assertEquals("1", second.outputJson)
        assertEquals("1", second.outputText.trim())
    }

    private fun shellState(): ShellState {
        val root = "/data/data/dev.aiterminal.android/files/ash-workspace"
        return ShellState(cwd = root, workspaceRoot = root)
    }
}
