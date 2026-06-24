package dev.aiterminal.android

import android.content.Context
import java.io.File

data class WorkspaceState(
    val rootPath: String,
    val cwdPath: String,
) {
    val rootName: String = basename(rootPath)
    val cwdName: String = basename(cwdPath)
    val isAtRoot: Boolean = rootPath == cwdPath
}

fun ensureAppPrivateWorkspace(context: Context): WorkspaceState {
    val root = File(context.filesDir, "ash-workspace")
    if (!root.exists()) {
        root.mkdirs()
    }
    val rootPath = root.canonicalPath
    return WorkspaceState(rootPath = rootPath, cwdPath = rootPath)
}

fun ShellState.workspaceState(): WorkspaceState =
    WorkspaceState(rootPath = workspaceRoot, cwdPath = cwd)

private fun basename(path: String): String {
    val name = File(path).name
    return name.ifBlank { path }
}
