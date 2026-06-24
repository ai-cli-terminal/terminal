package dev.aiterminal.android

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import java.io.File

data class ImportedWorkspaceDocument(
    val fileName: String,
    val path: String,
    val bytes: Long,
)

fun importDocumentToWorkspace(
    context: Context,
    uri: Uri,
    state: ShellState,
): ImportedWorkspaceDocument {
    val workspaceRoot = File(state.workspaceRoot).canonicalFile
    workspaceRoot.mkdirs()

    val displayName = queryDisplayName(context, uri) ?: "imported-document"
    val fileName = uniqueWorkspaceFile(workspaceRoot, sanitizeWorkspaceFileName(displayName))
    val destination = File(workspaceRoot, fileName).canonicalFile
    check(destination.path.startsWith(workspaceRoot.path + File.separator)) {
        "import target escaped workspace root"
    }

    val bytes = context.contentResolver.openInputStream(uri).use { input ->
        requireNotNull(input) { "unable to open selected document" }
        destination.outputStream().use { output ->
            input.copyTo(output)
        }
    }

    return ImportedWorkspaceDocument(
        fileName = destination.name,
        path = destination.path,
        bytes = bytes,
    )
}

fun exportTranscript(
    context: Context,
    uri: Uri,
    entries: List<TranscriptEntry>,
) {
    val body = buildString {
        for (entry in entries) {
            val prefix = when (entry.kind) {
                EntryKind.Command -> "> "
                EntryKind.Output -> ""
                EntryKind.Error -> "error: "
            }
            append(prefix).append(entry.text).append('\n')
        }
    }
    context.contentResolver.openOutputStream(uri, "wt").use { output ->
        requireNotNull(output) { "unable to open export destination" }
        output.write(body.toByteArray(Charsets.UTF_8))
    }
}

internal fun sanitizeWorkspaceFileName(name: String): String {
    val cleaned = name
        .substringAfterLast('/')
        .substringAfterLast('\\')
        .replace(Regex("[^A-Za-z0-9._-]"), "_")
        .trim('.', '_')
    return cleaned.ifBlank { "imported-document" }.take(96)
}

private fun uniqueWorkspaceFile(root: File, baseName: String): String {
    val stem = baseName.substringBeforeLast('.', baseName)
    val ext = baseName.substringAfterLast('.', "")
        .takeIf { it.isNotEmpty() && it != baseName }
        ?.let { ".$it" }
        ?: ""

    var candidate = baseName
    var index = 1
    while (File(root, candidate).exists()) {
        candidate = "$stem-$index$ext"
        index += 1
    }
    return candidate
}

private fun queryDisplayName(context: Context, uri: Uri): String? {
    val projection = arrayOf(OpenableColumns.DISPLAY_NAME)
    return context.contentResolver.query(uri, projection, null, null, null).use { cursor ->
        if (cursor == null || !cursor.moveToFirst()) {
            null
        } else {
            val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index >= 0) cursor.getString(index) else null
        }
    }
}
