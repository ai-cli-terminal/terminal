package dev.aiterminal.android

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import java.io.File
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets

data class ImportedWorkspaceDocument(
    val fileName: String,
    val path: String,
    val bytes: Long,
    val preview: WorkspaceDocumentPreview?,
)

data class WorkspaceDocumentPreview(
    val text: String,
    val truncated: Boolean,
)

data class OpenedWorkspaceDocument(
    val fileName: String,
    val preview: WorkspaceDocumentPreview,
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
        preview = previewWorkspaceDocument(destination),
    )
}

internal fun previewWorkspaceDocument(
    file: File,
    maxBytes: Int = 4 * 1024,
    maxLines: Int = 80,
): WorkspaceDocumentPreview? {
    require(maxBytes > 0) { "maxBytes must be positive" }
    require(maxLines > 0) { "maxLines must be positive" }

    val buffer = ByteArray(maxBytes + 1)
    var total = 0
    file.inputStream().use { input ->
        while (total < buffer.size) {
            val read = input.read(buffer, total, buffer.size - total)
            if (read == -1) break
            total += read
        }
    }
    val bytes = buffer.copyOf(total)
    if (bytes.any { it.toInt() == 0 }) {
        return null
    }

    val decoder = StandardCharsets.UTF_8.newDecoder()
        .onMalformedInput(CodingErrorAction.REPORT)
        .onUnmappableCharacter(CodingErrorAction.REPORT)
    val decoded = runCatching {
        decoder.decode(ByteBuffer.wrap(bytes.copyOf(minOf(bytes.size, maxBytes)))).toString()
    }.getOrNull() ?: return null

    val lines = decoded.lineSequence().take(maxLines + 1).toList()
    val truncatedByLines = lines.size > maxLines
    val previewLines = if (truncatedByLines) lines.take(maxLines) else lines
    return WorkspaceDocumentPreview(
        text = previewLines.joinToString("\n"),
        truncated = bytes.size > maxBytes || truncatedByLines,
    )
}

internal fun openWorkspaceDocumentReadOnly(
    path: String,
    state: ShellState,
    maxBytes: Int = 16 * 1024,
    maxLines: Int = 240,
): OpenedWorkspaceDocument {
    val workspaceRoot = File(state.workspaceRoot).canonicalFile
    val target = File(path).canonicalFile
    check(target.path.startsWith(workspaceRoot.path + File.separator)) {
        "document is outside workspace"
    }
    require(target.isFile) { "document is not a file" }
    val preview = previewWorkspaceDocument(target, maxBytes = maxBytes, maxLines = maxLines)
        ?: throw IllegalArgumentException("document is binary or not UTF-8 text")
    return OpenedWorkspaceDocument(fileName = target.name, preview = preview)
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
