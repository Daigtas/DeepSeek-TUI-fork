package com.deepseek.tui.connection

import android.content.Context
import android.util.Base64
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import net.schmizz.sshj.SSHClient
import net.schmizz.sshj.transport.verification.HostKeyVerifier
import java.io.File
import java.security.MessageDigest
import java.security.PublicKey
import java.util.concurrent.TimeUnit

/**
 * Manages the SSH connection to the deepseek server.
 *
 * Supports password and private key authentication, custom host key
 * verification with known_hosts persistence, and detailed connection
 * logging for debugging.
 */
class SshTunnelManager(private val context: Context) {

    enum class TunnelState {
        DISCONNECTED,
        CONNECTING,
        CONNECTED,
        ERROR,
        HOST_KEY_UNKNOWN  // waiting for user to accept/reject host key
    }

    private val _state = MutableStateFlow(TunnelState.DISCONNECTED)
    val state: StateFlow<TunnelState> = _state.asStateFlow()

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage.asStateFlow()

    private val _latencyMs = MutableStateFlow<Long?>(null)
    val latencyMs: StateFlow<Long?> = _latencyMs.asStateFlow()

    private val _reconnectCount = MutableStateFlow(0)
    val reconnectCount: StateFlow<Int> = _reconnectCount.asStateFlow()

    private val _logMessages = MutableStateFlow<List<String>>(emptyList())
    val logMessages: StateFlow<List<String>> = _logMessages.asStateFlow()

    private val _pendingHostKey = MutableStateFlow<PendingHostKey?>(null)
    val pendingHostKey: StateFlow<PendingHostKey?> = _pendingHostKey.asStateFlow()

    private var sshClient: SSHClient? = null
    private var healthCheckJob: Job? = null
    private var reconnectJob: Job? = null
    private var pendingHost: String? = null

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val configRepo = com.deepseek.tui.data.prefs.ConfigRepository(context)
    private val keyStore = KeyStoreManager(context)

    private fun log(msg: String) {
        _logMessages.value = _logMessages.value + "[${java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.US).format(java.util.Date())}] $msg"
    }

    fun clearLogs() {
        _logMessages.value = emptyList()
    }

    data class PendingHostKey(
        val host: String,
        val fingerprint: String,
        val keyType: String
    )

    fun acceptHostKey() {
        val host = pendingHost ?: return
        val pk = _pendingHostKey.value ?: return
        configRepo.saveHostKeyFingerprint(host, pk.fingerprint)
        log("Host key accepted and saved to known_hosts")
        _pendingHostKey.value = null
        pendingHost = null
        _state.value = TunnelState.DISCONNECTED
    }

    fun rejectHostKey() {
        _pendingHostKey.value = null
        pendingHost = null
        _state.value = TunnelState.DISCONNECTED
        _errorMessage.value = "Host key rejected by user"
    }

    /**
     * Establish SSH connection with password or key auth, run post-login
     * commands, and verify the daemon is reachable.
     */
    suspend fun connect(): Result<Unit> = withContext(Dispatchers.IO) {
        if (_state.value == TunnelState.CONNECTED) return@withContext Result.success(Unit)

        _state.value = TunnelState.CONNECTING
        _errorMessage.value = null
        clearLogs()

        try {
            val config = configRepo.loadConfig()
            log("Connecting to ${config.host}:${config.port} as ${config.user}...")

            val client = SSHClient()

            // Custom host key verifier: check known_hosts, prompt user for unknown
            val hostVerifier = createHostKeyVerifier(config.host)
            client.addHostKeyVerifier(hostVerifier)

            try {
                client.connect(config.host, config.port)
                log("TCP connection established to ${config.host}:${config.port}")
            } catch (e: Exception) {
                log("TCP connection failed: ${e.message}")
                _state.value = TunnelState.ERROR
                _errorMessage.value = "Cannot reach ${config.host}:${config.port} — ${e.message}"
                return@withContext Result.failure(e)
            }

            // Try private key auth first, fall back to password
            val keyBytes = keyStore.getSshKeyBytes()
            if (keyBytes != null) {
                val tmpKeyFile = File(context.cacheDir, "deepseek_ssh_key_tmp")
                tmpKeyFile.writeBytes(keyBytes)
                tmpKeyFile.deleteOnExit()

                try {
                    client.authPublickey(config.user, client.loadKeys(tmpKeyFile.absolutePath))
                    log("Authenticated with private key")
                } catch (e: Exception) {
                    log("Key auth failed: ${e.message}")
                    if (!config.password.isNullOrBlank()) {
                        log("Falling back to password auth...")
                        client.authPassword(config.user, config.password)
                        log("Authenticated with password")
                    } else {
                        throw e
                    }
                }
            } else if (!config.password.isNullOrBlank()) {
                client.authPassword(config.user, config.password)
                log("Authenticated with password")
            } else {
                _state.value = TunnelState.ERROR
                _errorMessage.value = "No SSH key or password configured"
                log("ERROR: No authentication method available")
                client.close()
                return@withContext Result.failure(IllegalStateException("No auth method"))
            }

            // Execute post-login commands
            log("Running post-login commands...")
            for ((i, cmd) in config.postLoginCommands.withIndex()) {
                val session = client.startSession()
                try {
                    log("  [$i] $cmd")
                    val exec = session.exec(cmd)
                    exec.join(10, TimeUnit.SECONDS)
                    val exitCode = exec.exitStatus
                    if (exitCode != null && exitCode != 0) {
                        val errStream = exec.errorStream
                        val errText = errStream?.bufferedReader()?.readText() ?: ""
                        log("  [$i] FAILED (exit=$exitCode): $errText")
                        session.close()
                        _state.value = TunnelState.ERROR
                        _errorMessage.value = "Post-login command failed: $cmd (exit=$exitCode)"
                        client.close()
                        return@withContext Result.failure(
                            IllegalStateException("Post-login command failed: $cmd (exit=$exitCode)")
                        )
                    }
                    log("  [$i] OK")
                } finally {
                    session.close()
                }
            }

            sshClient = client
            _state.value = TunnelState.CONNECTED
            _reconnectCount.value = 0
            log("Connected successfully to ${config.host}:${config.port}")

            startHealthChecks(config)
            Result.success(Unit)
        } catch (e: Exception) {
            log("Connection failed: ${e.message}")
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
        log("Disconnected")
    }

    private fun cleanup() {
        try { sshClient?.close() } catch (_: Exception) {}
        sshClient = null
        _latencyMs.value = null
    }

    private fun createHostKeyVerifier(host: String): HostKeyVerifier {
        return object : HostKeyVerifier {
            override fun findExistingAlgorithms(hostname: String, port: Int): List<String>? = null
            override fun verify(hostname: String, port: Int, key: PublicKey): Boolean {
                val fingerprint = computeFingerprint(key)
                val keyType = key.algorithm

                // Check known_hosts
                val savedFingerprint = configRepo.getHostKeyFingerprint(host)
                if (savedFingerprint != null) {
                    if (savedFingerprint == fingerprint) {
                        log("Host key verified (known)")
                        return true
                    } else {
                        log("WARNING: Host key mismatch for $host!")
                        log("  Expected: $savedFingerprint")
                        log("  Got:      $fingerprint")
                        return false // key changed — possible MITM
                    }
                } else {
                    // Unknown host — show UI prompt, reject for now
                    log("Unknown host key for $host")
                    log("  Fingerprint: $fingerprint")
                    log("  Key type: $keyType")
                    log("Prompting user to accept/reject...")

                    _pendingHostKey.value = PendingHostKey(
                        host = host,
                        fingerprint = fingerprint,
                        keyType = keyType
                    )
                    pendingHost = host
                    _state.value = TunnelState.HOST_KEY_UNKNOWN
                    return false // Reject for now — user must accept then retry Connect
                }
            }
        }
    }

    private fun computeFingerprint(key: PublicKey): String {
        val digest = MessageDigest.getInstance("SHA-256").digest(key.encoded)
        return "SHA256:" + Base64.encodeToString(digest, Base64.NO_WRAP)
    }

    private fun scheduleReconnect() {
        reconnectJob?.cancel()
        reconnectJob = scope.launch {
            val config = configRepo.loadConfig()
            var attempt = _reconnectCount.value + 1
            _reconnectCount.value = attempt
            log("Scheduling reconnect (attempt $attempt)...")

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
                        log("Health check failed $consecutiveFailures times — reconnecting")
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

    fun destroy() {
        disconnect()
        scope.cancel()
    }

}
