package com.example.agentmanagermobile.bridge

import kotlinx.coroutines.flow.StateFlow

interface BridgeClient {
  suspend fun claimPairing(baseUrl: String, code: String, deviceName: String): PairedDeviceCredentials
  suspend fun dashboard(credentials: BridgeCredentials): DashboardState
  suspend fun resumeRun(credentials: BridgeCredentials, runId: String)
  fun terminalStream(credentials: BridgeCredentials): TerminalStream
}

interface TerminalStream {
  val output: StateFlow<String>
  suspend fun attach(runId: String, cols: Int, rows: Int)
  suspend fun input(data: String)
  suspend fun resize(cols: Int, rows: Int)
  suspend fun close()
}
