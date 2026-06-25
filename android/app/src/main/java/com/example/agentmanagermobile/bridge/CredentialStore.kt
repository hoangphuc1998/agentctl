package com.example.agentmanagermobile.bridge

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

interface BridgeCredentialStore {
  suspend fun load(): BridgeCredentials?
  suspend fun save(credentials: BridgeCredentials)
  suspend fun clear()
}

class EncryptedBridgeCredentialStore(context: Context) : BridgeCredentialStore {
  private val appContext = context.applicationContext

  override suspend fun load(): BridgeCredentials? =
    withContext(Dispatchers.IO) {
      val prefs = prefs()
      val baseUrl = prefs.getString(KEY_BASE_URL, null) ?: return@withContext null
      val deviceId = prefs.getString(KEY_DEVICE_ID, null) ?: return@withContext null
      val token = prefs.getString(KEY_TOKEN, null) ?: return@withContext null
      BridgeCredentials(baseUrl, deviceId, token)
    }

  override suspend fun save(credentials: BridgeCredentials) {
    withContext(Dispatchers.IO) {
      prefs()
        .edit()
        .putString(KEY_BASE_URL, credentials.baseUrl)
        .putString(KEY_DEVICE_ID, credentials.deviceId)
        .putString(KEY_TOKEN, credentials.deviceToken)
        .apply()
    }
  }

  override suspend fun clear() {
    withContext(Dispatchers.IO) { prefs().edit().clear().apply() }
  }

  private fun prefs() =
    EncryptedSharedPreferences.create(
      appContext,
      "agent_manager_mobile_bridge",
      MasterKey.Builder(appContext).setKeyScheme(MasterKey.KeyScheme.AES256_GCM).build(),
      EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
      EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
    )

  private companion object {
    const val KEY_BASE_URL = "base_url"
    const val KEY_DEVICE_ID = "device_id"
    const val KEY_TOKEN = "token"
  }
}
