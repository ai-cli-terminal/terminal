package dev.aiterminal.android

import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject

data class TermuxHelperRequest(
    val argv: List<String>,
    val cwd: String,
    val env: Map<String, String> = emptyMap(),
    val timeoutMs: Long? = null,
)

object TermuxHelperProtocol {
    fun encodeRequest(request: TermuxHelperRequest): String {
        require(request.argv.isNotEmpty()) { "argv must not be empty" }

        val json = JSONObject()
        json.put("argv", JSONArray(request.argv))
        json.put("cwd", request.cwd)
        json.put("env", JSONObject(request.env))
        json.put("timeout_ms", request.timeoutMs ?: JSONObject.NULL)
        return json.toString()
    }

    fun decodeEventLine(
        line: String,
        input: String,
        state: ShellState,
    ): ShellStreamEvent {
        val json = parseEvent(line)
        return when (val type = json.optString("type")) {
            "started" -> ShellStreamEvent.Started(input, state)
            "stdout" -> ShellStreamEvent.Stdout(json.optString("text", ""))
            "stderr" -> ShellStreamEvent.Stderr(json.optString("text", ""))
            "finished" -> {
                val exitCode = json.optInt("exit_code", 0)
                ShellStreamEvent.Finished(
                    ShellEvalResult(
                        ok = exitCode == 0,
                        outputText = "",
                        outputJson = "null",
                        error = if (exitCode == 0) null else "external command exited $exitCode",
                        state = state.copy(exitCode = exitCode),
                    ),
                )
            }
            "cancelled" -> ShellStreamEvent.Cancelled(state)
            else -> ShellStreamEvent.Stderr("unknown Termux helper event: $type")
        }
    }

    private fun parseEvent(line: String): JSONObject =
        try {
            JSONObject(line)
        } catch (error: JSONException) {
            JSONObject()
                .put("type", "stderr")
                .put("text", "invalid Termux helper event: ${error.message}")
        }
}
