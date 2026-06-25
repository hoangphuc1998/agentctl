package com.example.agentmanagermobile.ui.main

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.agentmanagermobile.bridge.BridgeClient
import com.example.agentmanagermobile.bridge.BridgeCredentialStore
import com.example.agentmanagermobile.bridge.BridgeCredentials
import com.example.agentmanagermobile.bridge.DashboardState
import com.example.agentmanagermobile.bridge.RunView
import com.example.agentmanagermobile.bridge.TerminalStream
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class MainScreenViewModel(
  private val bridgeClient: BridgeClient,
  private val credentialStore: BridgeCredentialStore,
  private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
  private val _uiState = MutableStateFlow<MainScreenUiState>(
    MainScreenUiState.Pairing(baseUrl = DEFAULT_BRIDGE_URL),
  )
  val uiState: StateFlow<MainScreenUiState> = _uiState

  private var credentials: BridgeCredentials? = null
  private var terminalStream: TerminalStream? = null
  private var terminalOutputJob: Job? = null

  init {
    refresh()
  }

  fun refresh() {
    viewModelScope.launch(dispatcher) {
      val stored = credentialStore.load()
      if (stored == null) {
        _uiState.value = MainScreenUiState.Pairing(baseUrl = DEFAULT_BRIDGE_URL)
        return@launch
      }
      credentials = stored
      loadDashboard(stored)
    }
  }

  fun updatePairingBaseUrl(value: String) {
    _uiState.update { state ->
      when (state) {
        is MainScreenUiState.Pairing -> state.copy(baseUrl = value)
        else -> MainScreenUiState.Pairing(baseUrl = value)
      }
    }
  }

  fun updatePairingCode(value: String) {
    _uiState.update { state ->
      when (state) {
        is MainScreenUiState.Pairing -> state.copy(code = value.trim())
        else -> MainScreenUiState.Pairing(baseUrl = DEFAULT_BRIDGE_URL, code = value.trim())
      }
    }
  }

  fun claimPairing(deviceName: String) {
    val state = _uiState.value as? MainScreenUiState.Pairing ?: return
    viewModelScope.launch(dispatcher) {
      _uiState.value = state.copy(working = true, error = null)
      runCatching {
          bridgeClient.claimPairing(state.baseUrl.trimEnd('/'), state.code, deviceName)
        }
        .onSuccess { paired ->
          val next = BridgeCredentials(paired.baseUrl, paired.deviceId, paired.deviceToken)
          credentialStore.save(next)
          credentials = next
          loadDashboard(next)
        }
        .onFailure { err ->
          _uiState.value = state.copy(working = false, error = err.message ?: "Pairing failed")
        }
    }
  }

  fun selectRun(runId: String) {
    val state = _uiState.value as? MainScreenUiState.Ready ?: return
    _uiState.value = state.copy(selectedRunId = runId)
    attachSelectedTerminal()
  }

  fun sendInstruction(text: String) {
    if (text.isBlank()) return
    viewModelScope.launch(dispatcher) {
      terminalStream?.input(text.trimEnd() + "\n")
    }
  }

  fun resumeSelectedRun() {
    val state = _uiState.value as? MainScreenUiState.Ready ?: return
    val run = state.selectedRun ?: return
    if (!run.restorable) return
    val activeCredentials = credentials ?: return
    viewModelScope.launch(dispatcher) {
      runCatching { bridgeClient.resumeRun(activeCredentials, run.id) }
        .onSuccess { loadDashboard(activeCredentials, preferredSelectedRunId = run.id) }
        .onFailure { err ->
          _uiState.update { current ->
            if (current is MainScreenUiState.Ready) {
              current.copy(error = err.message ?: "Resume failed")
            } else {
              current
            }
          }
        }
    }
  }

  fun signOut() {
    viewModelScope.launch(dispatcher) {
      terminalOutputJob?.cancel()
      terminalOutputJob = null
      terminalStream?.close()
      terminalStream = null
      credentials = null
      credentialStore.clear()
      _uiState.value = MainScreenUiState.Pairing(baseUrl = DEFAULT_BRIDGE_URL)
    }
  }

  private suspend fun loadDashboard(
    credentials: BridgeCredentials,
    preferredSelectedRunId: String? = null,
  ) {
    _uiState.value = MainScreenUiState.Loading
    runCatching { bridgeClient.dashboard(credentials) }
      .onSuccess { dashboard ->
        val selectedRunId =
          preferredSelectedRunId
            ?: dashboard.selectedRunId
            ?: dashboard.repos.firstOrNull()?.runs?.firstOrNull()?.id
        terminalOutputJob?.cancel()
        terminalStream?.close()
        val stream = bridgeClient.terminalStream(credentials)
        terminalStream = stream
        _uiState.value =
          MainScreenUiState.Ready(
            baseUrl = credentials.baseUrl,
            dashboard = dashboard,
            selectedRunId = selectedRunId,
            terminalOutput = stream.output.value,
          )
        terminalOutputJob =
          viewModelScope.launch(dispatcher) {
            stream.output.collect { output ->
              _uiState.update { state ->
                if (state is MainScreenUiState.Ready) state.copy(terminalOutput = output) else state
              }
            }
          }
        attachSelectedTerminal()
      }
      .onFailure { err ->
        _uiState.value = MainScreenUiState.Error(err.message ?: "Dashboard unavailable")
      }
  }

  private fun attachSelectedTerminal() {
    val state = _uiState.value as? MainScreenUiState.Ready ?: return
    val run = state.selectedRun ?: return
    viewModelScope.launch(dispatcher) {
      terminalStream?.attach(run.id, cols = 96, rows = 28)
    }
  }

  companion object {
    const val DEFAULT_BRIDGE_URL = "https://linhmon.1vn.app"
  }
}

sealed interface MainScreenUiState {
  data object Loading : MainScreenUiState

  data class Pairing(
    val baseUrl: String,
    val code: String = "",
    val working: Boolean = false,
    val error: String? = null,
  ) : MainScreenUiState

  data class Error(val message: String) : MainScreenUiState

  data class Ready(
    val baseUrl: String,
    val dashboard: DashboardState,
    val selectedRunId: String?,
    val terminalOutput: String = "",
    val error: String? = null,
  ) : MainScreenUiState {
    val runs: List<RunView> = dashboard.repos.flatMap { it.runs }
    val selectedRun: RunView? = runs.firstOrNull { it.id == selectedRunId } ?: runs.firstOrNull()
  }
}
