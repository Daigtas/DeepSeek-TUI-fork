package com.deepseek.tui.connection

import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * HTTP + WebSocket client for the deepseek daemon API.
 *
 * Connects to the forwarded local port (default :18787) or
 * directly to deepseek.boottify.com if configured.
 */
class ApiClient {

    private val JSON = "application/json; charset=utf-8".toMediaType()

    private val okHttpClient: OkHttpClient by lazy {
        OkHttpClient.Builder()
            .connectTimeout(10, TimeUnit.SECONDS)
            .readTimeout(60, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)
            .pingInterval(30, TimeUnit.SECONDS)
            .build()
    }

    /**
     * Base URL for API calls — tunneled local port or direct HTTPS.
     */
    private var baseUrl: String = "https://deepseek.boottify.com"

    fun setBaseUrl(url: String) {
        baseUrl = url.trimEnd('/')
    }

    // ── HTTP methods ────────────────────────────────────────────────────

    suspend fun get(path: String): Result<String> {
        return try {
            val request = Request.Builder()
                .url("$baseUrl$path")
                .get()
                .build()
            val response = okHttpClient.newCall(request).execute()
            Result.success(response.body?.string() ?: "")
        } catch (e: IOException) {
            Result.failure(e)
        }
    }

    suspend fun post(path: String, jsonBody: String): Result<String> {
        return try {
            val body = jsonBody.toRequestBody(JSON)
            val request = Request.Builder()
                .url("$baseUrl$path")
                .post(body)
                .build()
            val response = okHttpClient.newCall(request).execute()
            Result.success(response.body?.string() ?: "")
        } catch (e: IOException) {
            Result.failure(e)
        }
    }

    // ── WebSocket ───────────────────────────────────────────────────────

    fun newWebSocket(
        path: String,
        listener: WebSocketListener
    ): WebSocket {
        val request = Request.Builder()
            .url("${baseUrl.replace("http", "ws")}$path")
            .build()
        return okHttpClient.newWebSocket(request, listener)
    }

    // ── Daemon-specific endpoints ───────────────────────────────────────

    suspend fun healthCheck(): Result<String> = get("/healthz")

    suspend fun daemonStatus(): Result<String> = get("/daemon/status")

    suspend fun daemonResume(): Result<String> = get("/daemon/resume")

    suspend fun daemonProgress(): Result<String> = get("/daemon/progress")

    suspend fun swarmAgents(): Result<String> = get("/swarm/agents")

    suspend fun hiveSummary(): Result<String> = get("/hive/summary")

    fun close() {
        okHttpClient.dispatcher.executorService.shutdown()
        okHttpClient.connectionPool.evictAll()
    }
}
