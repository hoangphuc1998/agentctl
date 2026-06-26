package com.example.agentmanagermobile.bridge

import android.webkit.CookieManager
import java.io.IOException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.withContext
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.Call
import okhttp3.Callback
import okhttp3.Cookie
import okhttp3.CookieJar
import okhttp3.HttpUrl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener

class OkHttpBridgeClient(
  private val client: OkHttpClient =
    OkHttpClient.Builder()
      .cookieJar(WebViewCookieJar())
      .build(),
  private val json: Json = Json { ignoreUnknownKeys = true },
) : BridgeClient {
  override suspend fun claimPairing(baseUrl: String, code: String, deviceName: String): PairedDeviceCredentials {
    val requestBody =
      json
        .encodeToString(PairClaimRequest.serializer(), PairClaimRequest(code, deviceName))
        .toRequestBody(JSON_MEDIA_TYPE)
    val request =
      Request.Builder()
        .url("${baseUrl.trimEnd('/')}/api/mobile/v1/pair/claim")
        .post(requestBody)
        .build()
    val response = execute(request)
    val paired = json.decodeFromString(PairClaimResponse.serializer(), response)
    return PairedDeviceCredentials(baseUrl.trimEnd('/'), paired.id, paired.token)
  }

  override suspend fun dashboard(credentials: BridgeCredentials): DashboardState {
    val request =
      authenticatedRequest(credentials, "/api/mobile/v1/dashboard")
        .get()
        .build()
    return json.decodeFromString(DashboardState.serializer(), execute(request))
  }

  override suspend fun resumeRun(credentials: BridgeCredentials, runId: String) {
    val request =
      authenticatedRequest(credentials, "/api/mobile/v1/runs/$runId/resume")
        .post("".toRequestBody(JSON_MEDIA_TYPE))
        .build()
    execute(request)
  }

  override fun terminalStream(credentials: BridgeCredentials): TerminalStream {
    return OkHttpTerminalStream(client, json, credentials)
  }

  private suspend fun execute(request: Request): String =
    withContext(Dispatchers.IO) {
      client.newCall(request).execute().use { response ->
        if (!response.isSuccessful) {
          throw IOException("Bridge request failed: HTTP ${response.code}")
        }
        response.body?.string() ?: ""
      }
    }
}

class OkHttpTerminalStream(
  private val client: OkHttpClient,
  private val json: Json,
  private val credentials: BridgeCredentials,
) : TerminalStream {
  private val _output = MutableStateFlow("")
  override val output: StateFlow<String> = _output
  private var webSocket: WebSocket? = null
  private var terminalId: String? = null

  override suspend fun attach(runId: String, cols: Int, rows: Int) {
    val request = authenticatedRequest(credentials, "/api/mobile/v1/stream")
      .url(credentials.baseUrl.trimEnd('/').replaceFirst("https://", "wss://") + "/api/mobile/v1/stream")
      .build()
    webSocket =
      client.newWebSocket(
        request,
        object : WebSocketListener() {
          override fun onMessage(webSocket: WebSocket, text: String) {
            handleMessage(text)
          }
        },
      )
    webSocket?.send("""{"type":"attachTerminal","runId":"$runId","cols":$cols,"rows":$rows}""")
  }

  override suspend fun input(data: String) {
    val id = terminalId ?: return
    webSocket?.send(jsonObject("terminalInput", "terminalId" to id, "data" to data))
  }

  override suspend fun resize(cols: Int, rows: Int) {
    val id = terminalId ?: return
    webSocket?.send("""{"type":"terminalResize","terminalId":"$id","cols":$cols,"rows":$rows}""")
  }

  override suspend fun close() {
    terminalId?.let { webSocket?.send(jsonObject("detachTerminal", "terminalId" to it)) }
    webSocket?.close(1000, "closed")
    webSocket = null
    terminalId = null
  }

  private fun handleMessage(text: String) {
    val element = json.parseToJsonElement(text).jsonObject
    when (element["type"]?.jsonPrimitive?.contentOrNull) {
      "terminalAttached" -> terminalId = element["terminalId"]?.jsonPrimitive?.contentOrNull
      "terminalSnapshot", "terminalOutput" -> {
        val data = element["data"]?.jsonPrimitive?.contentOrNull.orEmpty()
        _output.value += data
      }
    }
  }
}

class WebViewCookieJar : CookieJar {
  private val cookieManager = CookieManager.getInstance()

  override fun saveFromResponse(url: HttpUrl, cookies: List<Cookie>) {
    cookies.forEach { cookie -> cookieManager.setCookie(url.toString(), cookie.toString()) }
    cookieManager.flush()
  }

  override fun loadForRequest(url: HttpUrl): List<Cookie> {
    return cookieManager
      .getCookie(url.toString())
      ?.split(";")
      ?.mapNotNull { Cookie.parse(url, it.trim()) }
      .orEmpty()
  }
}

private val JSON_MEDIA_TYPE = "application/json; charset=utf-8".toMediaType()

private fun authenticatedRequest(credentials: BridgeCredentials, path: String): Request.Builder {
  return Request.Builder()
    .url(credentials.baseUrl.trimEnd('/') + path)
    .header("Authorization", "Bearer ${credentials.deviceToken}")
    .header("X-Agent-Manager-Device", credentials.deviceId)
}

private fun jsonObject(type: String, vararg fields: Pair<String, String>): String {
  val encodedFields = fields.joinToString(",") { (key, value) ->
    """"$key":${Json.encodeToString(String.serializer(), value)}"""
  }
  return """{"type":"$type",$encodedFields}"""
}
