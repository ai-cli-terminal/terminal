package dev.aiterminal.android

import org.junit.Assert.assertTrue
import org.junit.Test

class TermuxHelperBootstrapContractTest {
    @Test
    fun installScriptWritesHelperAndRunsSelfTest() {
        val script = TermuxHelperBootstrapContract.installScript()

        assertTrue(script.contains("mkdir -p \"\$HOME/.ash-termux-bridge\""))
        assertTrue(script.contains("cat > \"\$HOME/.ash-termux-bridge/helper.sh\""))
        assertTrue(script.contains("chmod 700 \"\$HOME/.ash-termux-bridge/helper.sh\""))
        assertTrue(script.contains("\"\$HOME/.ash-termux-bridge/helper.sh\" self-test"))
        assertTrue(script.contains(TermuxHelperBootstrapContract.SELF_TEST_MARKER))
    }

    @Test
    fun helperScriptUsesJobFilesAndCancelContract() {
        val script = TermuxHelperBootstrapContract.installScript()

        assertTrue(script.contains("request.json"))
        assertTrue(script.contains("events.ndjson"))
        assertTrue(script.contains("cancel"))
        assertTrue(script.contains("\"type\": \"cancelled\""))
    }
}
