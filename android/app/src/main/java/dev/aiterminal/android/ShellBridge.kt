package dev.aiterminal.android

import org.json.JSONException
import org.json.JSONArray
import org.json.JSONObject

data class ShellState(
    val cwd: String = "/app",
    val varsJson: String = "{}",
    val exitCode: Int? = null,
)

data class ShellEvalResult(
    val ok: Boolean,
    val outputText: String,
    val outputJson: String,
    val error: String?,
    val state: ShellState,
)

interface ShellBridge {
    fun evalLine(input: String, state: ShellState): ShellEvalResult
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

    private external fun nativeEvalLine(input: String, stateJson: String): String

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
    encoded.put("cwd", state.cwd)
    encoded.put("vars", parseJsonObjectOrEmpty(state.varsJson))
    encoded.put("exit_code", state.exitCode ?: JSONObject.NULL)
    return encoded.toString()
}

private fun decodeResult(raw: String, fallbackState: ShellState): ShellEvalResult {
    return try {
        val json = JSONObject(raw)
        val stateJson = json.optJSONObject("state")
        val nextState = if (stateJson == null) fallbackState else decodeState(stateJson)

        ShellEvalResult(
            ok = json.optBoolean("ok", false),
            outputText = json.optString("output_text", ""),
            outputJson = jsonValueToString(json.opt("output_json")),
            error = if (json.isNull("error")) null else json.optString("error"),
            state = nextState,
        )
    } catch (error: JSONException) {
        err("native shell returned invalid JSON: ${error.message}", fallbackState)
    }
}

private fun decodeState(json: JSONObject): ShellState {
    return ShellState(
        cwd = json.optString("cwd", "/app"),
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
