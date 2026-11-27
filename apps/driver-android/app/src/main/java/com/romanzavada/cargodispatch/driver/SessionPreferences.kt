package com.romanzavada.cargodispatch.driver

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import com.romanzavada.cargodispatch.driver.core.SessionLink
import java.nio.charset.StandardCharsets
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class SessionPreferences(context: Context) {
    private val preferences =
        context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)

    private val keyStore: KeyStore by lazy {
        KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }
    }

    fun load(): SavedDriverSession? {
        loadSecure()?.let { return it }

        // Legacy r1-r5 migration path. The raw URL is used only to reconnect once;
        // MainActivity calls save() after the connection succeeds, which replaces it
        // with encrypted token persistence and removes this key.
        val legacyUrl = preferences.getString(KEY_LEGACY_URL, null)?.trim().orEmpty()
        if (legacyUrl.isEmpty()) return null
        return SavedDriverSession(
            driverUrl = legacyUrl,
            emulatorMode = preferences.getBoolean(KEY_EMULATOR, false),
        )
    }

    fun save(driverUrl: String, emulatorMode: Boolean): Boolean {
        val target = runCatching {
            // Persist the original host. Emulator loopback rewriting happens only
            // when SessionLink is used by the active client.
            SessionLink.parseDriverUrl(driverUrl, emulatorMode = false)
        }.getOrNull() ?: return false

        return runCatching {
            val encrypted = encrypt(target.token)
            preferences.edit()
                .putString(KEY_BASE_URL, target.baseUrl)
                .putString(KEY_TOKEN_CIPHERTEXT, encrypted.ciphertext)
                .putString(KEY_TOKEN_IV, encrypted.iv)
                .putBoolean(KEY_EMULATOR, emulatorMode)
                .remove(KEY_LEGACY_URL)
                .commit()
        }.getOrElse {
            // Never keep or fall back to a raw token when secure persistence fails.
            clearPersistedSession()
            false
        }
    }

    fun clear() {
        clearPersistedSession()
        runCatching {
            if (keyStore.containsAlias(KEY_ALIAS)) {
                keyStore.deleteEntry(KEY_ALIAS)
            }
        }
    }

    private fun loadSecure(): SavedDriverSession? {
        val baseUrl = preferences.getString(KEY_BASE_URL, null)?.trim().orEmpty()
        val ciphertext = preferences.getString(KEY_TOKEN_CIPHERTEXT, null)?.trim().orEmpty()
        val iv = preferences.getString(KEY_TOKEN_IV, null)?.trim().orEmpty()
        if (baseUrl.isEmpty() || ciphertext.isEmpty() || iv.isEmpty()) return null

        return runCatching {
            val key = keyStore.getKey(KEY_ALIAS, null) as? SecretKey
                ?: return@runCatching null
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(
                Cipher.DECRYPT_MODE,
                key,
                GCMParameterSpec(GCM_TAG_BITS, Base64.decode(iv, Base64.NO_WRAP)),
            )
            val token = String(
                cipher.doFinal(Base64.decode(ciphertext, Base64.NO_WRAP)),
                StandardCharsets.UTF_8,
            )
            SavedDriverSession(
                driverUrl = "${baseUrl.trimEnd('/')}/driver/$token",
                emulatorMode = preferences.getBoolean(KEY_EMULATOR, false),
            )
        }.getOrNull()
    }

    private fun encrypt(token: String): EncryptedToken {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey())
        return EncryptedToken(
            ciphertext = Base64.encodeToString(
                cipher.doFinal(token.toByteArray(StandardCharsets.UTF_8)),
                Base64.NO_WRAP,
            ),
            iv = Base64.encodeToString(cipher.iv, Base64.NO_WRAP),
        )
    }

    private fun getOrCreateKey(): SecretKey {
        (keyStore.getKey(KEY_ALIAS, null) as? SecretKey)?.let { return it }

        val generator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            ANDROID_KEY_STORE,
        )
        generator.init(
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .build(),
        )
        return generator.generateKey()
    }

    private fun clearPersistedSession() {
        preferences.edit()
            .remove(KEY_LEGACY_URL)
            .remove(KEY_BASE_URL)
            .remove(KEY_TOKEN_CIPHERTEXT)
            .remove(KEY_TOKEN_IV)
            .remove(KEY_EMULATOR)
            .commit()
    }

    private data class EncryptedToken(
        val ciphertext: String,
        val iv: String,
    )

    private companion object {
        const val PREFERENCES_NAME = "cargo_dispatch_driver_session"
        const val KEY_LEGACY_URL = "driver_url"
        const val KEY_BASE_URL = "base_url"
        const val KEY_TOKEN_CIPHERTEXT = "token_ciphertext"
        const val KEY_TOKEN_IV = "token_iv"
        const val KEY_EMULATOR = "emulator_mode"

        const val ANDROID_KEY_STORE = "AndroidKeyStore"
        const val KEY_ALIAS = "cargo_dispatch_driver_session_token"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val GCM_TAG_BITS = 128
    }
}

data class SavedDriverSession(
    val driverUrl: String,
    val emulatorMode: Boolean,
)
