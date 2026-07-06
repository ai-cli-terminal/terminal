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
    val contentKind: WorkspaceDocumentContentKind,
    val preview: WorkspaceDocumentPreview?,
)

enum class WorkspaceDocumentContentKind {
    Text,
    BinaryOrUnsupported,
}

data class WorkspaceDocumentPreview(
    val text: String,
    val truncated: Boolean,
    val bytesRead: Int,
    val linesRead: Int,
)

data class OpenedWorkspaceDocument(
    val fileName: String,
    val bytes: Long,
    val contentKind: WorkspaceDocumentContentKind,
    val preview: WorkspaceDocumentPreview?,
)

data class ExportedWorkspaceDocument(
    val fileName: String,
    val bytes: Long,
)

internal data class WorkspaceDocumentExportSource(
    val file: File,
    val fileName: String,
    val bytes: Long,
)

data class WorkspaceDocumentCommand(
    val command: String,
    val fileName: String,
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

    val preview = previewWorkspaceDocument(destination)
    return ImportedWorkspaceDocument(
        fileName = destination.name,
        path = destination.path,
        bytes = bytes,
        contentKind = if (preview == null) {
            WorkspaceDocumentContentKind.BinaryOrUnsupported
        } else {
            WorkspaceDocumentContentKind.Text
        },
        preview = preview,
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

    val lines = if (decoded.isEmpty()) {
        emptyList()
    } else {
        decoded.lineSequence().take(maxLines + 1).toList()
    }
    val truncatedByLines = lines.size > maxLines
    val previewLines = if (truncatedByLines) lines.take(maxLines) else lines
    return WorkspaceDocumentPreview(
        text = previewLines.joinToString("\n"),
        truncated = bytes.size > maxBytes || truncatedByLines,
        bytesRead = minOf(bytes.size, maxBytes),
        linesRead = previewLines.size,
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
    return OpenedWorkspaceDocument(
        fileName = target.name,
        bytes = target.length(),
        contentKind = if (preview == null) {
            WorkspaceDocumentContentKind.BinaryOrUnsupported
        } else {
            WorkspaceDocumentContentKind.Text
        },
        preview = preview,
    )
}

internal fun prepareWorkspaceDocumentExport(
    path: String,
    state: ShellState,
): WorkspaceDocumentExportSource {
    val workspaceRoot = File(state.workspaceRoot).canonicalFile
    val target = File(path).canonicalFile
    check(target.path.startsWith(workspaceRoot.path + File.separator)) {
        "document is outside workspace"
    }
    require(target.isFile) { "document is not a file" }
    return WorkspaceDocumentExportSource(
        file = target,
        fileName = target.name,
        bytes = target.length(),
    )
}

fun exportWorkspaceDocument(
    context: Context,
    uri: Uri,
    path: String,
    state: ShellState,
): ExportedWorkspaceDocument {
    val source = prepareWorkspaceDocumentExport(path, state)
    context.contentResolver.openOutputStream(uri, "wt").use { output ->
        requireNotNull(output) { "unable to open export destination" }
        source.file.inputStream().use { input ->
            input.copyTo(output)
        }
    }
    return ExportedWorkspaceDocument(
        fileName = source.fileName,
        bytes = source.bytes,
    )
}

internal fun selectedWorkspaceDocumentListCommand(
    path: String,
    state: ShellState,
): WorkspaceDocumentCommand {
    val workspaceRoot = File(state.workspaceRoot).canonicalFile
    val cwd = File(state.cwd).canonicalFile
    check(cwd.path == workspaceRoot.path || cwd.path.startsWith(workspaceRoot.path + File.separator)) {
        "current directory is outside workspace"
    }
    val target = File(path).canonicalFile
    check(target.path.startsWith(workspaceRoot.path + File.separator)) {
        "document is outside workspace"
    }
    require(target.isFile) { "document is not a file" }
    val targetParent = requireNotNull(target.parentFile) {
        "document parent is unavailable"
    }.canonicalFile

    val relativeParent = cwd.toPath()
        .relativize(targetParent.toPath())
        .toString()
        .ifBlank { "." }
        .replace(File.separatorChar, '/')
    val parentLiteral = shellcoreStringLiteral(relativeParent)
    val nameLiteral = shellcoreStringLiteral(target.name)
    return WorkspaceDocumentCommand(
        command = "ls $parentLiteral | where name == $nameLiteral | first 1",
        fileName = target.name,
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

internal fun shellcoreStringLiteral(value: String): String {
    require(!value.contains('\n') && !value.contains('\r')) {
        "value cannot be represented as a single-line shellcore string"
    }
    return when {
        !value.contains('"') -> "\"$value\""
        !value.contains('\'') -> "'$value'"
        else -> error("value cannot be represented as a shellcore string")
    }
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
