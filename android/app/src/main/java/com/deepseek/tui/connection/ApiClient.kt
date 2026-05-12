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

    suspend fun delete(path: String): Result<String> {
        return try {
            val request = Request.Builder()
                .url("$baseUrl$path")
                .delete()
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

    // ── Health ───────────────────────────────────────────────────────────

    suspend fun healthCheck(): Result<String> = get("/healthz")

    // ── Daemon status / lifecycle ────────────────────────────────────────

    suspend fun daemonStatus(): Result<String> = get("/daemon/status")

    suspend fun daemonResume(): Result<String> = get("/daemon/resume")

    suspend fun daemonProgress(): Result<String> = get("/daemon/progress")

    /** POST /daemon/detach — detach from the daemon (empty body). */
    suspend fun daemonDetach(): Result<String> =
        post("/daemon/detach", "{}")

    /** POST /daemon/attach — attach to the daemon (empty body). */
    suspend fun daemonAttach(): Result<String> =
        post("/daemon/attach", "{}")

    /** POST /daemon/checkpoint — save a hive checkpoint. */
    suspend fun daemonCheckpoint(): Result<String> =
        post("/daemon/checkpoint", "{}")

    // ── Swarm ────────────────────────────────────────────────────────────

    suspend fun swarmAgents(): Result<String> = get("/swarm/agents")

    /** POST /swarm/spawn — launch a new swarm agent. */
    suspend fun swarmSpawn(role: String, name: String, prompt: String): Result<String> {
        val body = """{"role":"$role","name":"$name","prompt":"$prompt"}"""
        return post("/swarm/spawn", body)
    }

    /** POST /swarm/spawn — raw JSON body variant. */
    suspend fun swarmSpawnRaw(jsonBody: String): Result<String> =
        post("/swarm/spawn", jsonBody)

    // ── Hive ─────────────────────────────────────────────────────────────

    suspend fun hiveSummary(): Result<String> = get("/hive/summary")

    /** GET /hive/query/{key} — look up a hive entry by key. */
    suspend fun hiveQuery(key: String): Result<String> =
        get("/hive/query/${key}")

    /** POST /hive/inject — insert or update a hive entry (value is JSON-encoded). */
    suspend fun hiveInject(key: String, value: String): Result<String> {
        val escapedValue = value
            .replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
        val body = """{"key":"$key","value":"$escapedValue"}"""
        return post("/hive/inject", body)
    }

    /** POST /hive/inject — raw JSON body variant. */
    suspend fun hiveInjectRaw(jsonBody: String): Result<String> =
        post("/hive/inject", jsonBody)

    /** GET /hive/snapshot — full hive snapshot. */
    suspend fun hiveSnapshot(): Result<String> = get("/hive/snapshot")

    // ── Sessions ─────────────────────────────────────────────────────────

    /** GET /sessions — list all sessions. */
    suspend fun sessionList(): Result<String> = get("/sessions")

    /** GET /sessions/{id} — read one session. */
    suspend fun sessionRead(id: String): Result<String> = get("/sessions/$id")

    /** DELETE /sessions/{id} — delete a session. */
    suspend fun sessionDelete(id: String): Result<String> = delete("/sessions/$id")

    /** POST /sessions/{id}/export — export a session archive. */
    suspend fun sessionExport(id: String): Result<String> =
        post("/sessions/$id/export", "{}")

    /** POST /sessions/import — import a session archive. */
    suspend fun sessionImport(archivePath: String, overwrite: Boolean = false): Result<String> {
        val body = """{"archive_path":"$archivePath","overwrite":$overwrite}"""
        return post("/sessions/import", body)
    }

    // ── App config ───────────────────────────────────────────────────────

    /** POST /app with method=get — read a config value. */
    suspend fun appGet(key: String): Result<String> {
        val body = """{"method":"get","key":"$key"}"""
        return post("/app", body)
    }

    /** POST /app with method=set — write a config value. */
    suspend fun appSet(key: String, value: String): Result<String> {
        val body = """{"method":"set","key":"$key","value":"$value"}"""
        return post("/app", body)
    }

    /** POST /app with method=unset — remove a config key. */
    suspend fun appUnset(key: String): Result<String> {
        val body = """{"method":"unset","key":"$key"}"""
        return post("/app", body)
    }

    /** POST /app with method=list — list all config values. */
    suspend fun appList(): Result<String> =
        post("/app", """{"method":"list"}""")

    // ── Generic endpoint helpers ─────────────────────────────────────────

    /** GET /jobs — list active jobs. */
    suspend fun getJobs(): Result<String> = get("/jobs")

    /** POST /thread — create or interact with a thread. */
    suspend fun postThread(jsonBody: String): Result<String> =
        post("/thread", jsonBody)

    /** POST /app — raw app endpoint (capabilities, config ops, models, etc.). */
    suspend fun postApp(jsonBody: String): Result<String> =
        post("/app", jsonBody)

    /** POST /prompt — send a prompt to the daemon. */
    suspend fun postPrompt(jsonBody: String): Result<String> =
        post("/prompt", jsonBody)

    /** POST /tool — invoke a tool through the daemon. */
    suspend fun postTool(jsonBody: String): Result<String> =
        post("/tool", jsonBody)

    /** POST /mcp/startup — trigger MCP startup sequence. */
    suspend fun mcpStartup(): Result<String> =
        post("/mcp/startup", "{}")

    // ── Lifecycle ────────────────────────────────────────────────────────

    fun close() {
        okHttpClient.dispatcher.executorService.shutdown()
        okHttpClient.connectionPool.evictAll()
    }
}
