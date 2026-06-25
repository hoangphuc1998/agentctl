package com.example.agentmanagermobile.ui.main

import com.example.agentmanagermobile.bridge.BridgeClient
import com.example.agentmanagermobile.bridge.BridgeCredentials
import com.example.agentmanagermobile.bridge.BridgeCredentialStore
import com.example.agentmanagermobile.bridge.DashboardState
import com.example.agentmanagermobile.bridge.PairedDeviceCredentials
import com.example.agentmanagermobile.bridge.RepoNode
import com.example.agentmanagermobile.bridge.RunView
import com.example.agentmanagermobile.bridge.TerminalStream
import junit.framework.TestCase.assertEquals
import junit.framework.TestCase.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class MainScreenViewModelTest {
  @Test
  fun loadsDashboardWhenCredentialsExist() = runTest {
    val client = FakeBridgeClient()
    val store = FakeCredentialStore(
      BridgeCredentials(
        baseUrl = "https://linhmon.1vn.app",
        deviceId = "device-1",
        deviceToken = "token-1",
      ),
    )
    val viewModel = MainScreenViewModel(client, store, StandardTestDispatcher(testScheduler))

    viewModel.refresh()
    advanceUntilIdle()

    val state = viewModel.uiState.value as MainScreenUiState.Ready
    assertEquals("login-flow", state.selectedRun?.runName)
    assertEquals("https://linhmon.1vn.app", state.baseUrl)
  }

  @Test
  fun pairingClaimsCodeAndPersistsCredentials() = runTest {
    val client = FakeBridgeClient()
    val store = FakeCredentialStore(null)
    val viewModel = MainScreenViewModel(client, store, StandardTestDispatcher(testScheduler))

    viewModel.updatePairingBaseUrl("https://linhmon.1vn.app")
    viewModel.updatePairingCode("ABCD1234")
    viewModel.claimPairing("Pixel 9")
    advanceUntilIdle()

    assertEquals("device-2", store.savedCredentials?.deviceId)
    val state = viewModel.uiState.value as MainScreenUiState.Ready
    assertEquals("login-flow", state.selectedRun?.runName)
  }

  @Test
  fun sendInstructionWritesTextAndEnterToTerminalStream() = runTest {
    val client = FakeBridgeClient()
    val store = FakeCredentialStore(
      BridgeCredentials(
        baseUrl = "https://linhmon.1vn.app",
        deviceId = "device-1",
        deviceToken = "token-1",
      ),
    )
    val viewModel = MainScreenViewModel(client, store, StandardTestDispatcher(testScheduler))
    viewModel.refresh()
    advanceUntilIdle()

    viewModel.sendInstruction("check status")
    advanceUntilIdle()

    assertEquals(listOf("check status\n"), client.stream.sentInput)
  }

  @Test
  fun terminalOutputUpdatesReadyState() = runTest {
    val client = FakeBridgeClient()
    val store = FakeCredentialStore(
      BridgeCredentials(
        baseUrl = "https://linhmon.1vn.app",
        deviceId = "device-1",
        deviceToken = "token-1",
      ),
    )
    val viewModel = MainScreenViewModel(client, store, StandardTestDispatcher(testScheduler))
    viewModel.refresh()
    advanceUntilIdle()

    client.stream.output.value = "agent is ready"
    advanceUntilIdle()

    val state = viewModel.uiState.value as MainScreenUiState.Ready
    assertEquals("agent is ready", state.terminalOutput)
  }

  @Test
  fun resumeSelectedRunCallsBridgeAndRefreshesDashboard() = runTest {
    val client = FakeBridgeClient(sampleDashboard(restorable = true))
    val store = FakeCredentialStore(
      BridgeCredentials(
        baseUrl = "https://linhmon.1vn.app",
        deviceId = "device-1",
        deviceToken = "token-1",
      ),
    )
    val viewModel = MainScreenViewModel(client, store, StandardTestDispatcher(testScheduler))
    viewModel.refresh()
    advanceUntilIdle()

    viewModel.resumeSelectedRun()
    advanceUntilIdle()

    assertEquals(listOf("run-1"), client.resumedRunIds)
    val state = viewModel.uiState.value as MainScreenUiState.Ready
    assertTrue(state.selectedRun?.restorable == true)
    assertEquals("run-1", state.selectedRun?.id)
  }
}

private class FakeBridgeClient(private val dashboard: DashboardState = sampleDashboard()) : BridgeClient {
  val stream = FakeTerminalStream()
  val resumedRunIds = mutableListOf<String>()

  override suspend fun claimPairing(baseUrl: String, code: String, deviceName: String): PairedDeviceCredentials {
    return PairedDeviceCredentials(
      baseUrl = baseUrl,
      deviceId = "device-2",
      deviceToken = "token-2",
    )
  }

  override suspend fun dashboard(credentials: BridgeCredentials): DashboardState = dashboard

  override suspend fun resumeRun(credentials: BridgeCredentials, runId: String) {
    resumedRunIds += runId
  }

  override fun terminalStream(credentials: BridgeCredentials): TerminalStream = stream
}

private class FakeTerminalStream : TerminalStream {
  val sentInput = mutableListOf<String>()
  override val output = MutableStateFlow("")
  override suspend fun attach(runId: String, cols: Int, rows: Int) = Unit
  override suspend fun input(data: String) {
    sentInput += data
  }

  override suspend fun resize(cols: Int, rows: Int) = Unit
  override suspend fun close() = Unit
}

private class FakeCredentialStore(initial: BridgeCredentials?) : BridgeCredentialStore {
  private var credentials = initial
  var savedCredentials: BridgeCredentials? = initial

  override suspend fun load(): BridgeCredentials? = credentials

  override suspend fun save(credentials: BridgeCredentials) {
    this.credentials = credentials
    savedCredentials = credentials
  }

  override suspend fun clear() {
    credentials = null
    savedCredentials = null
  }
}

private fun sampleDashboard(restorable: Boolean = false): DashboardState {
  val run = RunView(
    id = "run-1",
    repoPath = "/repo/agent-manager",
    repoName = "agent-manager",
    tag = "default",
    runName = "login-flow",
    agent = "codex",
    lifecycle = "active",
    observedState = "running",
    detectionSource = "tmux",
    branch = "login-flow",
    baseRef = "HEAD",
    worktreePath = "/repo/worktrees/login-flow",
    restorable = restorable,
    createdAt = 1,
    updatedAt = 2,
  )
  return DashboardState(
    repos = listOf(RepoNode("agent-manager", "/repo/agent-manager", listOf(run))),
    selectedRunId = "run-1",
    activeCount = 1,
    attentionCount = 0,
    staleCount = 0,
    restorableCount = 0,
    activeRepoPath = "/repo/agent-manager",
  )
}
