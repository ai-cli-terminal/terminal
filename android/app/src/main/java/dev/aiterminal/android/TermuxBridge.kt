package dev.aiterminal.android

import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.IBinder
import android.os.Build
import android.os.Bundle
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger

enum class TermuxBridgeState {
    Unavailable,
    PermissionRequired,
    Installed,
    ProbeRunning,
    Ready,
}

data class TermuxBridgeAvailability(
    val state: TermuxBridgeState,
    val message: String,
)

data class TermuxCommandResult(
    val stdout: String = "",
    val stderr: String = "",
    val exitCode: Int? = null,
    val internalError: String? = null,
    val stdoutOriginalLength: Int? = null,
    val stderrOriginalLength: Int? = null,
) {
    val ok: Boolean
        get() = internalError.isNullOrBlank() && exitCode == 0
}

data class TermuxSmokeCase(
    val name: String,
    val script: String,
)

data class TermuxSmokeResult(
    val case: TermuxSmokeCase,
    val commandResult: TermuxCommandResult,
    val passed: Boolean,
    val message: String,
)

object TermuxRunCommandContract {
    const val TERMUX_PACKAGE = "com.termux"
    const val RUN_COMMAND_SERVICE = "com.termux.app.RunCommandService"
    const val ACTION_RUN_COMMAND = "com.termux.RUN_COMMAND"
    const val PERMISSION_RUN_COMMAND = "com.termux.permission.RUN_COMMAND"

    const val EXTRA_COMMAND_PATH = "com.termux.RUN_COMMAND_PATH"
    const val EXTRA_ARGUMENTS = "com.termux.RUN_COMMAND_ARGUMENTS"
    const val EXTRA_WORKDIR = "com.termux.RUN_COMMAND_WORKDIR"
    const val EXTRA_BACKGROUND = "com.termux.RUN_COMMAND_BACKGROUND"
    const val EXTRA_PENDING_INTENT = "com.termux.RUN_COMMAND_PENDING_INTENT"
    const val EXTRA_COMMAND_LABEL = "com.termux.RUN_COMMAND_LABEL"
    const val EXTRA_COMMAND_DESCRIPTION = "com.termux.RUN_COMMAND_DESCRIPTION"

    const val EXTRA_PLUGIN_RESULT_BUNDLE = "result"
    const val EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT = "stdout"
    const val EXTRA_PLUGIN_RESULT_BUNDLE_STDERR = "stderr"
    const val EXTRA_PLUGIN_RESULT_BUNDLE_EXIT_CODE = "exitCode"
    const val EXTRA_PLUGIN_RESULT_BUNDLE_ERR = "err"
    const val EXTRA_PLUGIN_RESULT_BUNDLE_ERRMSG = "errmsg"
    const val EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT_ORIGINAL_LENGTH = "stdout_original_length"
    const val EXTRA_PLUGIN_RESULT_BUNDLE_STDERR_ORIGINAL_LENGTH = "stderr_original_length"

    const val EXTRA_EXECUTION_ID = "dev.aiterminal.android.TERMUX_EXECUTION_ID"

    val T0_SMOKE_CASES = listOf(
        TermuxSmokeCase("echo", "echo ASH_TERMUX_OK"),
        TermuxSmokeCase("pwd", "pwd"),
        TermuxSmokeCase("stderr", "echo ERR >&2; exit 2"),
        TermuxSmokeCase("non-zero", "exit 7"),
    )

    fun availability(installed: Boolean, permissionGranted: Boolean): TermuxBridgeAvailability =
        when {
            !installed -> TermuxBridgeAvailability(
                TermuxBridgeState.Unavailable,
                "external runtime unavailable",
            )
            !permissionGranted -> TermuxBridgeAvailability(
                TermuxBridgeState.PermissionRequired,
                "Termux bridge permission required",
            )
            else -> TermuxBridgeAvailability(
                TermuxBridgeState.Installed,
                "Termux runtime installed; probe required",
            )
        }

    fun decodeResultMap(values: Map<String, Any?>): TermuxCommandResult =
        TermuxCommandResult(
            stdout = values[EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT] as? String ?: "",
            stderr = values[EXTRA_PLUGIN_RESULT_BUNDLE_STDERR] as? String ?: "",
            exitCode = values[EXTRA_PLUGIN_RESULT_BUNDLE_EXIT_CODE] as? Int,
            internalError = decodeInternalError(values),
            stdoutOriginalLength = values[EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT_ORIGINAL_LENGTH] as? Int,
            stderrOriginalLength = values[EXTRA_PLUGIN_RESULT_BUNDLE_STDERR_ORIGINAL_LENGTH] as? Int,
        )

    fun toShellEvalResult(result: TermuxCommandResult, state: ShellState): ShellEvalResult {
        val error = when {
            !result.internalError.isNullOrBlank() -> result.internalError
            result.exitCode != null && result.exitCode != 0 -> "external command exited ${result.exitCode}"
            else -> null
        }
        return ShellEvalResult(
            ok = error == null,
            outputText = result.stdout,
            outputJson = "null",
            error = error,
            state = state.copy(exitCode = result.exitCode),
        )
    }

    fun evaluateT0Smoke(case: TermuxSmokeCase, result: TermuxCommandResult): TermuxSmokeResult {
        val internalError = result.internalError
        if (!internalError.isNullOrBlank()) {
            return TermuxSmokeResult(case, result, passed = false, message = internalError)
        }

        val passed = when (case.name) {
            "echo" -> result.exitCode == 0 && result.stdout.contains("ASH_TERMUX_OK")
            "pwd" -> result.exitCode == 0 && result.stdout.trim().isNotEmpty()
            "stderr" -> result.exitCode == 2 && result.stderr.contains("ERR")
            "non-zero" -> result.exitCode == 7
            else -> result.exitCode == 0
        }
        val message = if (passed) {
            when (case.name) {
                "pwd" -> result.stdout.trim()
                "stderr" -> result.stderr.trim().ifBlank { "stderr captured" }
                "non-zero" -> "exit ${result.exitCode}"
                else -> result.stdout.trim().ifBlank { "exit ${result.exitCode}" }
            }
        } else {
            val details = listOfNotNull(
                result.exitCode?.let { "exit $it" },
                result.stdout.trim().takeIf { it.isNotEmpty() }?.let { "stdout=$it" },
                result.stderr.trim().takeIf { it.isNotEmpty() }?.let { "stderr=$it" },
            ).joinToString(", ")
            details.ifBlank { "unexpected Termux result" }
        }
        return TermuxSmokeResult(case, result, passed, message)
    }

    private fun decodeInternalError(values: Map<String, Any?>): String? {
        val err = values[EXTRA_PLUGIN_RESULT_BUNDLE_ERR]
        val errMsg = values[EXTRA_PLUGIN_RESULT_BUNDLE_ERRMSG] as? String
        return when {
            err == null || err == -1 || err == "-1" -> errMsg?.takeIf { it.isNotBlank() }
            !errMsg.isNullOrBlank() -> errMsg
            else -> "Termux internal error: $err"
        }
    }
}

fun interface TermuxProbeCallback {
    fun onProbeResult(result: TermuxCommandResult)
}

fun interface TermuxSmokeCallback {
    fun onSmokeComplete(results: List<TermuxSmokeResult>)
}

interface TermuxBridge {
    fun availability(): TermuxBridgeAvailability
    fun startEchoProbe(callback: TermuxProbeCallback): ShellRunHandle
    fun startT0Smoke(callback: TermuxSmokeCallback): ShellRunHandle
}

class AndroidTermuxBridge(
    private val context: Context,
) : TermuxBridge {
    override fun availability(): TermuxBridgeAvailability {
        val installed = isPackageInstalled(context, TermuxRunCommandContract.TERMUX_PACKAGE)
        val permissionGranted = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            context.checkSelfPermission(TermuxRunCommandContract.PERMISSION_RUN_COMMAND) ==
                PackageManager.PERMISSION_GRANTED
        } else {
            context.packageManager.checkPermission(
                TermuxRunCommandContract.PERMISSION_RUN_COMMAND,
                context.packageName,
            ) == PackageManager.PERMISSION_GRANTED
        }
        return TermuxRunCommandContract.availability(installed, permissionGranted)
    }

    override fun startEchoProbe(callback: TermuxProbeCallback): ShellRunHandle {
        return startCommand(
            script = "echo ASH_TERMUX_OK",
            label = "AI Terminal probe",
            description = "Checks whether AI Terminal can receive Termux command results.",
            callback = callback,
        )
    }

    override fun startT0Smoke(callback: TermuxSmokeCallback): ShellRunHandle {
        val availability = availability()
        if (availability.state != TermuxBridgeState.Installed) {
            val case = TermuxRunCommandContract.T0_SMOKE_CASES.first()
            callback.onSmokeComplete(
                listOf(
                    TermuxSmokeResult(
                        case = case,
                        commandResult = TermuxCommandResult(internalError = availability.message),
                        passed = false,
                        message = availability.message,
                    ),
                ),
            )
            return AtomicShellRunHandle().also { it.cancel() }
        }

        val handle = AtomicShellRunHandle()
        val results = mutableListOf<TermuxSmokeResult>()

        fun runCase(index: Int) {
            if (handle.isCancelled) return
            val case = TermuxRunCommandContract.T0_SMOKE_CASES.getOrNull(index)
            if (case == null) {
                callback.onSmokeComplete(results.toList())
                return
            }

            startCommand(
                script = case.script,
                label = "AI Terminal ${case.name} smoke",
                description = "Runs the ${case.name} Termux T0 smoke case.",
            ) { result ->
                if (handle.isCancelled) return@startCommand
                val smokeResult = TermuxRunCommandContract.evaluateT0Smoke(case, result)
                results += smokeResult
                if (smokeResult.passed) {
                    runCase(index + 1)
                } else {
                    callback.onSmokeComplete(results.toList())
                }
            }
        }

        runCase(0)
        return handle
    }

    private fun startCommand(
        script: String,
        label: String,
        description: String,
        callback: TermuxProbeCallback,
    ): ShellRunHandle {
        val availability = availability()
        if (availability.state != TermuxBridgeState.Installed) {
            callback.onProbeResult(TermuxCommandResult(internalError = availability.message))
            return AtomicShellRunHandle().also { it.cancel() }
        }

        val handle = AtomicShellRunHandle()
        val executionId = TermuxProbeRegistry.register { result ->
            if (!handle.isCancelled) {
                callback.onProbeResult(result)
            }
        }
        val pendingIntent = buildResultPendingIntent(context, executionId)
        val intent = buildRunCommandIntent(
            script = script,
            executionId = executionId,
            pendingIntent = pendingIntent,
            label = label,
            description = description,
        )
        runCatching {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }
            .onFailure { error ->
                TermuxProbeRegistry.complete(
                    executionId,
                    TermuxCommandResult(
                        internalError = error.message ?: error::class.java.simpleName,
                    ),
                )
            }
        return object : ShellRunHandle {
            override val isCancelled: Boolean
                get() = handle.isCancelled

            override fun cancel() {
                handle.cancel()
                TermuxProbeRegistry.cancel(executionId)
            }
        }
    }

    private fun buildRunCommandIntent(
        script: String,
        executionId: Int,
        pendingIntent: PendingIntent,
        label: String,
        description: String,
    ): Intent =
        Intent().apply {
            setClassName(
                TermuxRunCommandContract.TERMUX_PACKAGE,
                TermuxRunCommandContract.RUN_COMMAND_SERVICE,
            )
            action = TermuxRunCommandContract.ACTION_RUN_COMMAND
            putExtra(TermuxRunCommandContract.EXTRA_COMMAND_PATH, TERMUX_SH)
            putExtra(TermuxRunCommandContract.EXTRA_ARGUMENTS, arrayOf("-c", script))
            putExtra(TermuxRunCommandContract.EXTRA_BACKGROUND, true)
            putExtra(TermuxRunCommandContract.EXTRA_PENDING_INTENT, pendingIntent)
            putExtra(TermuxRunCommandContract.EXTRA_COMMAND_LABEL, label)
            putExtra(TermuxRunCommandContract.EXTRA_COMMAND_DESCRIPTION, description)
            putExtra(TermuxRunCommandContract.EXTRA_EXECUTION_ID, executionId)
        }

    private fun buildResultPendingIntent(context: Context, executionId: Int): PendingIntent {
        val intent = Intent(context, TermuxRunCommandResultService::class.java)
            .putExtra(TermuxRunCommandContract.EXTRA_EXECUTION_ID, executionId)
        val flags = PendingIntent.FLAG_ONE_SHOT or pendingIntentMutableFlag()
        return PendingIntent.getService(context, executionId, intent, flags)
    }

    private fun pendingIntentMutableFlag(): Int =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0

    private fun isPackageInstalled(context: Context, packageName: String): Boolean =
        runCatching {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                context.packageManager.getPackageInfo(
                    packageName,
                    PackageManager.PackageInfoFlags.of(0),
                )
            } else {
                @Suppress("DEPRECATION")
                context.packageManager.getPackageInfo(packageName, 0)
            }
        }.isSuccess

    private companion object {
        const val TERMUX_SH = "/data/data/com.termux/files/usr/bin/sh"
    }
}

class TermuxRunCommandResultService : Service() {
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent != null) {
            val executionId = intent.getIntExtra(TermuxRunCommandContract.EXTRA_EXECUTION_ID, -1)
            if (executionId >= 0) {
                TermuxProbeRegistry.complete(executionId, decodeIntentResult(intent))
            }
        }
        stopSelf(startId)
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    @Suppress("DEPRECATION")
    private fun Bundle.toResultMap(): Map<String, Any?> =
        keySet().associateWith { key -> get(key) }

    private fun decodeIntentResult(intent: Intent): TermuxCommandResult {
        val bundle = intent
            .getBundleExtra(TermuxRunCommandContract.EXTRA_PLUGIN_RESULT_BUNDLE)
            ?: return TermuxCommandResult(internalError = "Termux result bundle missing")
        return TermuxRunCommandContract.decodeResultMap(bundle.toResultMap())
    }
}

private object TermuxProbeRegistry {
    private val nextId = AtomicInteger(1)
    private val callbacks = ConcurrentHashMap<Int, TermuxProbeCallback>()

    fun register(callback: TermuxProbeCallback): Int {
        val id = nextId.getAndIncrement()
        callbacks[id] = callback
        return id
    }

    fun complete(id: Int, result: TermuxCommandResult) {
        callbacks.remove(id)?.onProbeResult(result)
    }

    fun cancel(id: Int) {
        callbacks.remove(id)
    }
}
