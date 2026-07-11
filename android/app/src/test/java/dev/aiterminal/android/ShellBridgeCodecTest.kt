package dev.aiterminal.android

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ShellBridgeCodecTest {
    @Test
    fun decodeResultParsesAiSuggestion() {
        val raw = """
            {"ok":true,"output_json":null,"output_text":"","error":null,
             "ai":{"kind":"answered","text":"ls -al"},
             "state":{"workspace_root":"/app","cwd":"/app","vars":{},"exit_code":null}}
        """.trimIndent()

        val result = decodeResult(raw, ShellState())

        assertEquals(AiSuggestion(kind = "answered", text = "ls -al"), result.ai)
        assertEquals(true, result.ok)
    }

    @Test
    fun decodeResultWithoutAiFieldIsNull() {
        val raw = """
            {"ok":true,"output_json":null,"output_text":"x","error":null,
             "state":{"workspace_root":"/app","cwd":"/app","vars":{},"exit_code":null}}
        """.trimIndent()

        val result = decodeResult(raw, ShellState())

        assertNull(result.ai)
    }

    @Test
    fun encodeAiConfigProducesRustContract() {
        val json = JSONObject(
            encodeAiConfig(ShellAiConfig(provider = "openai", apiKey = "sk-1")),
        )

        assertEquals("openai", json.getString("provider"))
        assertEquals("default", json.getString("model"))
        assertEquals("https://api.openai.com", json.getString("openai_url"))
        assertEquals("sk-1", json.getString("api_key"))
    }

    @Test
    fun encodeAiConfigNullApiKeyIsJsonNull() {
        val json = JSONObject(encodeAiConfig(ShellAiConfig()))

        assertEquals("mock", json.getString("provider"))
        assertEquals(true, json.isNull("api_key"))
    }

    @Test
    fun defaultBridgeHandleMethodsAreNoOpFallback() {
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult =
                ShellEvalResult(ok = true, outputText = "ran:$input", outputJson = "null", error = null, state = state)
        }
        val state = ShellState()
        // default createAi=0, destroyAi no-op(throw 없음), evalLineAiHandle→evalLine 위임
        assertEquals(0L, bridge.createAi(ShellAiConfig()))
        bridge.destroyAi(0L)
        val result = bridge.evalLineAiHandle(0L, "x", state)
        assertEquals("ran:x", result.outputText)
    }
}
