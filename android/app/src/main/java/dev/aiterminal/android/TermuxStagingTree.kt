package dev.aiterminal.android

import java.net.URI
import java.net.URLDecoder
import java.nio.charset.StandardCharsets

internal fun resolveSharedStagingPathFromTreeUri(uriString: String): Result<String> =
    runCatching {
        val uri = URI(uriString)
        require(uri.scheme == "content") { "shared staging picker returned a non-content URI" }
        require(uri.authority == EXTERNAL_STORAGE_DOCUMENTS_AUTHORITY) {
            "selected directory is not Android shared storage"
        }

        val encodedTreeId = uri.rawPath
            ?.trim('/')
            ?.split('/')
            ?.let { segments ->
                val treeIndex = segments.indexOf("tree")
                if (treeIndex >= 0 && treeIndex + 1 < segments.size) {
                    segments[treeIndex + 1]
                } else {
                    null
                }
            }
        require(!encodedTreeId.isNullOrBlank()) { "selected directory is not a storage tree" }

        val treeId = URLDecoder.decode(encodedTreeId, StandardCharsets.UTF_8.name())
        resolveExternalStorageTreeDocumentId(treeId)
    }

private fun resolveExternalStorageTreeDocumentId(treeId: String): String {
    val volume = treeId.substringBefore(':', missingDelimiterValue = treeId)
    val relativePath = treeId.substringAfter(':', missingDelimiterValue = "")
    require(volume == "primary") {
        "selected directory is not on primary shared storage"
    }

    val cleanSegments = relativePath
        .split('/')
        .filter { it.isNotBlank() }
    require(cleanSegments.none { it == "." || it == ".." }) {
        "selected directory path is not safe"
    }

    return if (cleanSegments.isEmpty()) {
        PRIMARY_SHARED_STORAGE_ROOT
    } else {
        PRIMARY_SHARED_STORAGE_ROOT + "/" + cleanSegments.joinToString("/")
    }
}

private const val EXTERNAL_STORAGE_DOCUMENTS_AUTHORITY = "com.android.externalstorage.documents"
private const val PRIMARY_SHARED_STORAGE_ROOT = "/sdcard"
