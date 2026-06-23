package dev.aiterminal.android

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

class FakeShellBridge : ShellBridge {
    override fun evalLine(input: String, state: ShellState): ShellEvalResult {
        val trimmed = input.trim()
        return when {
            trimmed.isEmpty() -> ok("", "null", state)
            trimmed.startsWith("let ") -> {
                val parts = trimmed.removePrefix("let ").split("=", limit = 2)
                if (parts.size != 2) {
                    err("expected: let name = value", state)
                } else {
                    val name = parts[0].trim()
                    val value = parts[1].trim()
                    ok("", "null", state.copy(varsJson = """{"$name":$value}"""))
                }
            }
            trimmed == "[{size: 50} {size: 200}] | where size > 100" ->
                ok("size\n200", """[{"size":200}]""", state)
            trimmed.endsWith("| length") ->
                ok("1", "1", state)
            else -> err("external execution disabled: ${trimmed.substringBefore(' ')}", state)
        }
    }

    private fun ok(text: String, json: String, state: ShellState) =
        ShellEvalResult(ok = true, outputText = text, outputJson = json, error = null, state = state)

    private fun err(message: String, state: ShellState) =
        ShellEvalResult(ok = false, outputText = "", outputJson = "null", error = message, state = state)
}
