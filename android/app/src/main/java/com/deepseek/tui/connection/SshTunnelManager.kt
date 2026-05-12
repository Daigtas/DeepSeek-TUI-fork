package com.deepseek.tui.connection

import android.content.Context
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import net.schmizz.sshj.SSHClient
import net.schmizz.sshj.transport.verification.PromiscuousVerifier
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * Manages the SSH connection to the deepseek server.
 *
 * Strategy: SSH is used for authentication and daemon lifecycle
 * (post-login commands, daemon start/stop). The API client connects
 * directly via HTTPS to deepseek.boottify.com or via a separately
 * configured SSH tunnel.
 */
class SshTunnelManager(private val context: Context) {

    enum class TunnelState {
        DISCONNECTED,
        CONNECTING,
        CONNECTED,
        ERROR
    }

    private val _state = MutableStateFlow(TunnelState.DISCONNECTED)
    val state: StateFlow<TunnelState> = _state.asStateFlow()

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage.asStateFlow()

    private val _latencyMs = MutableStateFlow<Long?>(null)
    val latencyMs: StateFlow<Long?> = _latencyMs.asStateFlow()

    private val _reconnectCount = MutableStateFlow(0)
    val reconnectCount: StateFlow<Int> = _reconnectCount.asStateFlow()

    private var sshClient: SSHClient? = null
    private var healthCheckJob: Job? = null
    private var reconnectJob: Job? = null

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val configRepo = com.deepseek.tui.data.prefs.ConfigRepository(context)
    private val keyStore = KeyStoreManager(context)

    /**
     * Establish SSH connection, run post-login commands,
     * and verify the daemon is reachable.
     */
    suspend fun connect(): Result<Unit> = withContext(Dispatchers.IO) {
        if (_state.value == TunnelState.CONNECTED) return@withContext Result.success(Unit)

        _state.value = TunnelState.CONNECTING
        _errorMessage.value = null

        try {
            val config = configRepo.loadConfig()
            val keyBytes = keyStore.getSshKeyBytes()
                ?: return@withContext Result.failure(IllegalStateException("No SSH key imported"))

            // Write PEM key to temp file for SSHJ
            val tmpKeyFile = File(context.cacheDir, "deepseek_ssh_key_tmp")
            tmpKeyFile.writeBytes(keyBytes)
            tmpKeyFile.deleteOnExit()

            // Connect and authenticate
            val client = SSHClient()
            client.addHostKeyVerifier(PromiscuousVerifier())
            client.connect(config.host, config.port)
            client.authPublickey(config.user, client.loadKeys(tmpKeyFile.absolutePath))

            // Execute post-login commands
            for (cmd in config.postLoginCommands) {
                val session = client.startSession()
                try {
                    val exec = session.exec(cmd)
                    exec.join(10, TimeUnit.SECONDS)
                    if (exec.exitStatus != null && exec.exitStatus != 0) {
                        return@withContext Result.failure(
                            IllegalStateException("Post-login command failed: $cmd (exit=${exec.exitStatus})")
                        )
                    }
                } finally {
                    session.close()
                }
            }

            sshClient = client
            _state.value = TunnelState.CONNECTED
            _reconnectCount.value = 0

            startHealthChecks(config)
            Result.success(Unit)
        } catch (e: Exception) {
            _state.value = TunnelState.ERROR
            _errorMessage.value = e.message ?: "Unknown connection error"
            cleanup()
            Result.failure(e)
        }
    }

    fun disconnect() {
        healthCheckJob?.cancel()
        reconnectJob?.cancel()
        cleanup()
        _state.value = TunnelState.DISCONNECTED
    }

    private fun scheduleReconnect() {
        reconnectJob?.cancel()
        reconnectJob = scope.launch {
            val config = configRepo.loadConfig()
            var attempt = _reconnectCount.value + 1
            _reconnectCount.value = attempt

            while (isActive && _state.value != TunnelState.CONNECTED) {
                val delayMs = minOf(
                    (Math.pow(2.0, attempt.toDouble()) * 1000).toLong(),
                    config.reconnectMaxBackoffSec * 1000L
                )
                delay(delayMs)
                attempt++
                _reconnectCount.value = attempt

                if (connect().isSuccess) break
            }
        }
    }

    private fun startHealthChecks(config: com.deepseek.tui.data.prefs.ConnectionConfig) {
        healthCheckJob?.cancel()
        healthCheckJob = scope.launch {
            var consecutiveFailures = 0
            while (isActive && _state.value == TunnelState.CONNECTED) {
                delay(config.healthCheckIntervalSec * 1000L)
                val ok = pingHealth()
                if (ok) {
                    consecutiveFailures = 0
                } else {
                    consecutiveFailures++
                    if (consecutiveFailures >= 3) {
                        _state.value = TunnelState.ERROR
                        _errorMessage.value = "Health check failed — reconnecting"
                        scheduleReconnect()
                        return@launch
                    }
                }
            }
        }
    }

    private suspend fun pingHealth(): Boolean = withContext(Dispatchers.IO) {
        try {
            val config = configRepo.loadConfig()
            val url = java.net.URL("${config.directHttpsUrl}/healthz")
            val start = System.currentTimeMillis()
            val conn = url.openConnection() as javax.net.ssl.HttpsURLConnection
            conn.connectTimeout = 3000
            conn.readTimeout = 3000
            conn.requestMethod = "GET"
            val code = conn.responseCode
            val elapsed = System.currentTimeMillis() - start
            conn.disconnect()
            _latencyMs.value = elapsed
            code == 200
        } catch (e: Exception) {
            false
        }
    }

    private fun cleanup() {
        try { sshClient?.close() } catch (_: Exception) {}
        sshClient = null
        _latencyMs.value = null
    }

    fun destroy() {
        disconnect()
        scope.cancel()
    }
}
