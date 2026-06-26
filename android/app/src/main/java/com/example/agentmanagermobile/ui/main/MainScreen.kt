package com.example.agentmanagermobile.ui.main

import android.annotation.SuppressLint
import android.webkit.CookieManager
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.NavKey
import com.example.agentmanagermobile.bridge.EncryptedBridgeCredentialStore
import com.example.agentmanagermobile.bridge.OkHttpBridgeClient
import com.example.agentmanagermobile.bridge.RunView
import com.example.agentmanagermobile.theme.AgentManagerMobileTheme
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

@Composable
fun MainScreen(
  onItemClick: (NavKey) -> Unit,
  modifier: Modifier = Modifier,
  providedViewModel: MainScreenViewModel? = null,
) {
  val context = LocalContext.current.applicationContext
  val viewModel =
    providedViewModel
      ?: viewModel {
      MainScreenViewModel(
        OkHttpBridgeClient(),
        EncryptedBridgeCredentialStore(context),
      )
    }
  val state by viewModel.uiState.collectAsStateWithLifecycle()
  AgentManagerMobileApp(
    state = state,
    onBaseUrlChange = viewModel::updatePairingBaseUrl,
    onCodeChange = viewModel::updatePairingCode,
    onPair = { viewModel.claimPairing(android.os.Build.MODEL ?: "Android") },
    onRefresh = viewModel::refresh,
    onSelectRun = viewModel::selectRun,
    onResumeRun = viewModel::resumeSelectedRun,
    onSendInstruction = viewModel::sendInstruction,
    onSignOut = viewModel::signOut,
    modifier = modifier,
  )
}

@Composable
internal fun AgentManagerMobileApp(
  state: MainScreenUiState,
  onBaseUrlChange: (String) -> Unit,
  onCodeChange: (String) -> Unit,
  onPair: () -> Unit,
  onRefresh: () -> Unit,
  onSelectRun: (String) -> Unit,
  onResumeRun: () -> Unit,
  onSendInstruction: (String) -> Unit,
  onSignOut: () -> Unit,
  modifier: Modifier = Modifier,
) {
  Surface(modifier = modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
    when (state) {
      MainScreenUiState.Loading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        CircularProgressIndicator()
      }
      is MainScreenUiState.Pairing -> PairingScreen(state, onBaseUrlChange, onCodeChange, onPair)
      is MainScreenUiState.Error -> ErrorScreen(state.message, onRefresh)
      is MainScreenUiState.Ready ->
        DashboardScreen(
          state = state,
          onRefresh = onRefresh,
          onSelectRun = onSelectRun,
          onResumeRun = onResumeRun,
          onSendInstruction = onSendInstruction,
          onSignOut = onSignOut,
        )
    }
  }
}

@Composable
private fun PairingScreen(
  state: MainScreenUiState.Pairing,
  onBaseUrlChange: (String) -> Unit,
  onCodeChange: (String) -> Unit,
  onPair: () -> Unit,
) {
  Column(
    Modifier
      .fillMaxSize()
      .padding(20.dp),
    verticalArrangement = Arrangement.spacedBy(14.dp),
  ) {
    Text("Agent Manager", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
    Text("Sign in through xTunnel, then enter the pairing code from the desktop Mobile Bridge panel.")
    Text(
      "The sign-in panel must show bridge health JSON before pairing. If it shows the AI Hay landing page, finish xTunnel sign-in there first.",
      style = MaterialTheme.typography.bodySmall,
      color = MaterialTheme.colorScheme.secondary,
    )
    OutlinedTextField(
      value = state.baseUrl,
      onValueChange = onBaseUrlChange,
      label = { Text("xTunnel URL") },
      singleLine = true,
      modifier = Modifier.fillMaxWidth(),
    )
    XTunnelLoginWebView(state.baseUrl, Modifier.fillMaxWidth().weight(1f))
    OutlinedTextField(
      value = state.code,
      onValueChange = onCodeChange,
      label = { Text("Pairing code") },
      singleLine = true,
      modifier = Modifier.fillMaxWidth(),
    )
    Button(onClick = onPair, enabled = !state.working && state.code.isNotBlank()) {
      Text(if (state.working) "Pairing..." else "Pair Android")
    }
    state.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
  }
}

@Composable
private fun DashboardScreen(
  state: MainScreenUiState.Ready,
  onRefresh: () -> Unit,
  onSelectRun: (String) -> Unit,
  onResumeRun: () -> Unit,
  onSendInstruction: (String) -> Unit,
  onSignOut: () -> Unit,
) {
  var composer by remember { mutableStateOf("") }
  Row(Modifier.fillMaxSize()) {
    Column(
      Modifier
        .width(180.dp)
        .fillMaxHeight()
        .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.42f))
        .padding(12.dp),
      verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
      Text("Runs", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
      StatusLine("${state.dashboard.activeCount} active", "${state.dashboard.attentionCount} attention")
      LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.weight(1f)) {
        items(state.runs, key = { it.id }) { run ->
          RunRow(run, selected = run.id == state.selectedRun?.id, onClick = { onSelectRun(run.id) })
        }
      }
      OutlinedButton(onClick = onRefresh, modifier = Modifier.fillMaxWidth()) { Text("Refresh") }
      OutlinedButton(onClick = onSignOut, modifier = Modifier.fillMaxWidth()) { Text("Disconnect") }
    }
    Column(Modifier.weight(1f).fillMaxHeight().padding(12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
      state.selectedRun?.let { run ->
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
          Text(run.runName, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold, modifier = Modifier.weight(1f))
          if (run.restorable) {
            Button(onClick = onResumeRun) { Text("Resume") }
          }
        }
        StatusLine(run.repoName, "${run.agent} · ${run.observedState}")
      }
      if (state.dashboard.attentionCount > 0) {
        Text(
          "${state.dashboard.attentionCount} run${if (state.dashboard.attentionCount == 1) "" else "s"} need attention",
          color = MaterialTheme.colorScheme.error,
          style = MaterialTheme.typography.bodyMedium,
        )
      }
      state.error?.let {
        Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
      }
      TerminalWebView(state.terminalOutput, Modifier.weight(1f).fillMaxWidth())
      Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        OutlinedTextField(
          value = composer,
          onValueChange = { composer = it },
          label = { Text("Instruction") },
          modifier = Modifier.weight(1f),
        )
        Button(
          onClick = {
            onSendInstruction(composer)
            composer = ""
          },
          enabled = composer.isNotBlank(),
        ) {
          Text("Send")
        }
      }
    }
  }
}

@Composable
private fun RunRow(run: RunView, selected: Boolean, onClick: () -> Unit) {
  val color =
    if (selected) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surface
  Column(
    Modifier
      .fillMaxWidth()
      .clickable(onClick = onClick)
      .background(color, RoundedCornerShape(8.dp))
      .padding(10.dp),
  ) {
    Text(run.runName, fontWeight = FontWeight.SemiBold)
    Text("${run.repoName} · ${run.observedState}", style = MaterialTheme.typography.bodySmall)
  }
}

@Composable
private fun StatusLine(left: String, right: String) {
  Row(verticalAlignment = Alignment.CenterVertically) {
    Text(left, style = MaterialTheme.typography.bodySmall)
    Spacer(Modifier.width(8.dp))
    Text(right, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.secondary)
  }
}

@Composable
private fun ErrorScreen(message: String, onRefresh: () -> Unit) {
  Column(
    Modifier.fillMaxSize().padding(20.dp),
    verticalArrangement = Arrangement.Center,
    horizontalAlignment = Alignment.CenterHorizontally,
  ) {
    Text(message, color = MaterialTheme.colorScheme.error)
    Spacer(Modifier.height(12.dp))
    Button(onClick = onRefresh) { Text("Retry") }
  }
}

@SuppressLint("SetJavaScriptEnabled")
@Composable
private fun XTunnelLoginWebView(url: String, modifier: Modifier = Modifier) {
  val loginUrl = xtunnelLoginUrl(url)
  AndroidView(
    modifier = modifier,
    factory = { context ->
      val cookieManager = CookieManager.getInstance()
      WebView(context).apply {
        cookieManager.setAcceptCookie(true)
        cookieManager.setAcceptThirdPartyCookies(this, true)
        settings.javaScriptEnabled = true
        settings.domStorageEnabled = true
        webViewClient =
          object : WebViewClient() {
            override fun onPageFinished(view: WebView, url: String) {
              cookieManager.flush()
            }
          }
        tag = loginUrl
        loadUrl(loginUrl)
      }
    },
    update = { view ->
      if (view.tag != loginUrl) {
        view.tag = loginUrl
        view.loadUrl(loginUrl)
      }
    },
  )
}

internal fun xtunnelLoginUrl(baseUrl: String): String {
  return baseUrl.trimEnd('/') + "/api/mobile/v1/health"
}

@SuppressLint("SetJavaScriptEnabled")
@Composable
private fun TerminalWebView(output: String, modifier: Modifier = Modifier) {
  AndroidView(
    modifier = modifier.background(Color.Black, RoundedCornerShape(8.dp)),
    factory = { context ->
      WebView(context).apply {
        settings.javaScriptEnabled = true
        loadUrl("file:///android_asset/terminal.html")
      }
    },
    update = { view ->
      val json = Json.encodeToString(String.serializer(), output)
      view.evaluateJavascript("window.agentTerminalSet($json)", null)
    },
  )
}

@Preview(showBackground = true, widthDp = 420)
@Composable
fun MainScreenPreview() {
  AgentManagerMobileTheme {
    AgentManagerMobileApp(
      state = MainScreenUiState.Pairing(MainScreenViewModel.DEFAULT_BRIDGE_URL),
      onBaseUrlChange = {},
      onCodeChange = {},
      onPair = {},
      onRefresh = {},
      onSelectRun = {},
      onResumeRun = {},
      onSendInstruction = {},
      onSignOut = {},
    )
  }
}
