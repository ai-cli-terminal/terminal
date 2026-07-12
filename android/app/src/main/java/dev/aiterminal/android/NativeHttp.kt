package dev.aiterminal.android

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.util.concurrent.TimeUnit

/**
 * Rust JniHttpTransport가 JNI로 호출하는 동기 HTTP 포스트.
 * Android 시스템 TLS를 사용한다(Rust rustls 크로스컴파일 회피).
 * 실패는 예외로 던지고 Rust가 Err로 흡수한다(fail-soft, §3-3).
 */
object NativeHttp {
    private val JSON = "application/json; charset=utf-8".toMediaType()

    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .callTimeout(75, TimeUnit.SECONDS) // 전체 호출 상한(동기 취소 대체)
        .build()

    @JvmStatic
    fun postJson(url: String, body: String, bearer: String?): String {
        val builder = Request.Builder().url(url).post(body.toRequestBody(JSON))
        if (bearer != null) builder.header("Authorization", "Bearer $bearer")
        client.newCall(builder.build()).execute().use { response ->
            val text = response.body?.string() ?: ""
            if (!response.isSuccessful) {
                throw java.io.IOException("HTTP ${response.code}: $text")
            }
            return text
        }
    }
}
