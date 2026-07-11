package dev.aiterminal.android

import org.json.JSONException
import org.json.JSONArray
import org.json.JSONObject

data class ShellState(
    val cwd: String = "/app",
    val workspaceRoot: String = "/app",
    val varsJson: String = "{}",
    val exitCode: Int? = null,
)

data class ShellAiConfig(
    val provider: String = "mock",
    val model: String = "default",
    val openaiUrl: String = "https://api.openai.com",
    val apiKey: String? = null,
)

data class AiSuggestion(
    val kind: String,
    val text: String,
)

data class ShellEvalResult(
    val ok: Boolean,
    val outputText: String,
    val outputJson: String,
    val error: String?,
    val state: ShellState,
    val ai: AiSuggestion? = null,
)

interface ShellBridge {
    fun evalLine(input: String, state: ShellState): ShellEvalResult

    fun evalLineAi(input: String, state: ShellState, aiConfig: ShellAiConfig): ShellEvalResult =
        evalLine(input, state)

    /** 지속 AI 핸들 생성. 미지원 구현은 0(호출측 폴백). */
    fun createAi(config: ShellAiConfig): Long = 0L

    /** 지속 핸들로 평가. 미지원 구현은 기존 evalLine으로 폴백. */
    fun evalLineAiHandle(handle: Long, input: String, state: ShellState): ShellEvalResult =
        evalLine(input, state)

    /** 지속 핸들 해제. 미지원 구현은 no-op. */
    fun destroyAi(handle: Long) {}
}

class NativeShellBridge : ShellBridge {
    override fun evalLine(input: String, state: ShellState): ShellEvalResult {
        return try {
            loadNativeLibrary()
            decodeResult(nativeEvalLine(input, encodeState(state)), state)
        } catch (error: UnsatisfiedLinkError) {
            err("native shell library not loaded: ${error.message}", state)
        } catch (error: RuntimeException) {
            err("native shell bridge failed: ${error.message}", state)
        }
    }

    override fun evalLineAi(
        input: String,
        state: ShellState,
        aiConfig: ShellAiConfig,
    ): ShellEvalResult {
        return try {
            loadNativeLibrary()
            decodeResult(nativeEvalLineAi(input, encodeState(state), encodeAiConfig(aiConfig)), state)
        } catch (error: UnsatisfiedLinkError) {
            err("native shell library not loaded: ${error.message}", state)
        } catch (error: RuntimeException) {
            err("native shell bridge failed: ${error.message}", state)
        }
    }

    override fun createAi(config: ShellAiConfig): Long {
        return try {
            loadNativeLibrary()
            nativeCreateAi(encodeAiConfig(config))
        } catch (error: UnsatisfiedLinkError) {
            0L
        } catch (error: RuntimeException) {
            0L
        }
    }

    override fun evalLineAiHandle(handle: Long, input: String, state: ShellState): ShellEvalResult {
        return try {
            loadNativeLibrary()
            decodeResult(nativeEvalLineAiHandle(handle, input, encodeState(state)), state)
        } catch (error: UnsatisfiedLinkError) {
            err("native shell library not loaded: ${error.message}", state)
        } catch (error: RuntimeException) {
            err("native shell bridge failed: ${error.message}", state)
        }
    }

    override fun destroyAi(handle: Long) {
        try {
            loadNativeLibrary()
            if (handle != 0L) nativeDestroyAi(handle)
        } catch (error: UnsatisfiedLinkError) {
            // 라이브러리 미로드면 해제할 것도 없음
        } catch (error: RuntimeException) {
            // 해제 실패는 무시(best-effort)
        }
    }

    private external fun nativeEvalLine(input: String, stateJson: String): String

    private external fun nativeEvalLineAi(input: String, stateJson: String, aiConfigJson: String): String

    private external fun nativeCreateAi(aiConfigJson: String): Long
    private external fun nativeEvalLineAiHandle(handle: Long, input: String, stateJson: String): String
    private external fun nativeDestroyAi(handle: Long)

    companion object {
        @Volatile
        private var loaded = false

        private fun loadNativeLibrary() {
            if (loaded) return
            synchronized(this) {
                if (!loaded) {
                    System.loadLibrary("ai_terminal")
                    loaded = true
                }
            }
        }
    }
}

private fun encodeState(state: ShellState): String {
    val encoded = JSONObject()
    encoded.put("workspace_root", state.workspaceRoot)
    encoded.put("cwd", state.cwd)
    encoded.put("vars", parseJsonObjectOrEmpty(state.varsJson))
    encoded.put("exit_code", state.exitCode ?: JSONObject.NULL)
    return encoded.toString()
}

internal fun encodeAiConfig(config: ShellAiConfig): String {
    val encoded = JSONObject()
    encoded.put("provider", config.provider)
    encoded.put("model", config.model)
    encoded.put("openai_url", config.openaiUrl)
    encoded.put("api_key", config.apiKey ?: JSONObject.NULL)
    return encoded.toString()
}

internal fun decodeResult(raw: String, fallbackState: ShellState): ShellEvalResult {
    return try {
        val json = JSONObject(raw)
        val stateJson = json.optJSONObject("state")
        val nextState = if (stateJson == null) fallbackState else decodeState(stateJson, fallbackState)

        ShellEvalResult(
            ok = json.optBoolean("ok", false),
            outputText = json.optString("output_text", ""),
            outputJson = jsonValueToString(json.opt("output_json")),
            error = if (json.isNull("error")) null else json.optString("error"),
            state = nextState,
            ai = decodeAiSuggestion(json.optJSONObject("ai")),
        )
    } catch (error: JSONException) {
        err("native shell returned invalid JSON: ${error.message}", fallbackState)
    }
}

private fun decodeAiSuggestion(json: JSONObject?): AiSuggestion? {
    if (json == null) return null
    return AiSuggestion(
        kind = json.optString("kind", "unavailable"),
        text = json.optString("text", ""),
    )
}

private fun decodeState(json: JSONObject, fallbackState: ShellState): ShellState {
    return ShellState(
        cwd = json.optString("cwd", fallbackState.cwd),
        workspaceRoot = json.optString("workspace_root", fallbackState.workspaceRoot),
        varsJson = jsonValueToString(json.opt("vars") ?: JSONObject()),
        exitCode = if (json.isNull("exit_code")) null else json.optInt("exit_code"),
    )
}

private fun parseJsonObjectOrEmpty(raw: String): JSONObject {
    return try {
        JSONObject(raw)
    } catch (_: JSONException) {
        JSONObject()
    }
}

private fun err(message: String, state: ShellState) =
    ShellEvalResult(ok = false, outputText = "", outputJson = "null", error = message, state = state)

private fun jsonValueToString(value: Any?): String =
    when (value) {
        null, JSONObject.NULL -> "null"
        is JSONObject -> value.toString()
        is JSONArray -> value.toString()
        is String -> JSONObject.quote(value)
        else -> value.toString()
    }
