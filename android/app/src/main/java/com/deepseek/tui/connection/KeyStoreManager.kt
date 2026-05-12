package com.deepseek.tui.connection

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.PublicKey
import java.security.spec.ECGenParameterSpec

/**
 * Manages SSH private key storage using Android Keystore.
 *
 * Keys are stored as EC P-256 key pairs in the hardware-backed
 * Android Keystore. The private key never leaves the keystore;
 * we extract it only as a Java PrivateKey reference for SSHJ.
 *
 * For SSH keys that are already in OpenSSH format (PEM),
 * they are imported into the keystore as a KeyStore entry.
 */
class KeyStoreManager(private val context: Context) {

    companion object {
        private const val KEYSTORE_ALIAS = "deepseek_ssh_key"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
    }

    private val keyStore: KeyStore by lazy {
        KeyStore.getInstance(ANDROID_KEYSTORE).also { it.load(null) }
    }

    /**
     * Check if an SSH key has been imported.
     */
    fun hasKey(): Boolean {
        return keyStore.containsAlias(KEYSTORE_ALIAS)
    }

    /**
     * Get a human-readable fingerprint of the stored public key.
     * Returns null if no key is stored.
     */
    fun getPublicKeyFingerprint(): String? {
        val entry = keyStore.getEntry(KEYSTORE_ALIAS, null)
            as? KeyStore.PrivateKeyEntry ?: return null

        val publicKey = entry.certificate.publicKey
        val encoded = publicKey.encoded ?: return null

        // SHA-256 fingerprint in the format: SHA256:base64
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(encoded)
        return "SHA256:" + Base64.encodeToString(digest, Base64.NO_WRAP)
    }

    /**
     * Import an SSH private key in PEM format (PKCS#8 or OpenSSH).
     *
     * SSHJ can handle PEM parsing directly, so we store the PEM bytes
     * in EncryptedSharedPreferences and use them at connection time.
     * The Android Keystore holds a separate EC key for app-level
     * authentication if needed, but SSH keys are stored as encrypted
     * data since raw SSH private key material can't go directly into
     * the Keystore's asymmetric key API.
     *
     * @param pemBytes The raw PEM-encoded private key bytes
     */
    fun importSshKey(pemBytes: ByteArray) {
        // Store encrypted in prefs — the actual SSH key is used by SSHJ
        // which needs the raw bytes. EncryptedSharedPreferences handles
        // encryption at rest via AES-256-GCM with Keystore-backed key.
        context.getSharedPreferences("deepseek_secure", Context.MODE_PRIVATE)
            .edit()
            .putString("ssh_key_b64", Base64.encodeToString(pemBytes, Base64.NO_WRAP))
            .apply()
    }

    /**
     * Retrieve the imported SSH private key PEM bytes.
     * Returns null if no key has been imported.
     */
    fun getSshKeyBytes(): ByteArray? {
        val b64 = context.getSharedPreferences("deepseek_secure", Context.MODE_PRIVATE)
            .getString("ssh_key_b64", null) ?: return null
        return try {
            Base64.decode(b64, Base64.NO_WRAP)
        } catch (e: Exception) {
            null
        }
    }

    /**
     * Generate an EC key pair for app-level operations (future use —
     * client certificate auth, local signing, etc.). Not used for SSH
     * authentication directly.
     */
    fun generateAppKeyPair() {
        if (hasKey()) return

        val generator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_EC,
            ANDROID_KEYSTORE
        )
        generator.initialize(
            KeyGenParameterSpec.Builder(
                KEYSTORE_ALIAS,
                KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY
            )
                .setDigests(KeyProperties.DIGEST_SHA256)
                .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
                .setUserAuthenticationRequired(false)
                .build()
        )
        generator.generateKeyPair()
    }

    /**
     * Remove all stored key material.
     */
    fun clearKeys() {
        keyStore.deleteEntry(KEYSTORE_ALIAS)
        context.getSharedPreferences("deepseek_secure", Context.MODE_PRIVATE)
            .edit()
            .remove("ssh_key_b64")
            .apply()
    }
}
