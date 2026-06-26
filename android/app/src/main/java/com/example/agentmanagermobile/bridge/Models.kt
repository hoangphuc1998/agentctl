package com.example.agentmanagermobile.bridge

import kotlinx.serialization.Serializable

@Serializable
data class BridgeCredentials(
  val baseUrl: String,
  val deviceId: String,
  val deviceToken: String,
)

@Serializable
data class PairedDeviceCredentials(
  val baseUrl: String,
  val deviceId: String,
  val deviceToken: String,
)

@Serializable
data class PairClaimRequest(
  val code: String,
  val deviceName: String,
)

@Serializable
data class PairClaimResponse(
  val id: String,
  val name: String,
  val token: String,
)

@Serializable
data class DashboardState(
  val repos: List<RepoNode>,
  val selectedRunId: String? = null,
  val activeCount: Int = 0,
  val attentionCount: Int = 0,
  val staleCount: Int = 0,
  val restorableCount: Int = 0,
  val activeRepoPath: String? = null,
)

@Serializable
data class RepoNode(
  val repoName: String,
  val repoPath: String,
  val runs: List<RunView>,
)

@Serializable
data class RunView(
  val id: String,
  val repoPath: String,
  val repoName: String,
  val tag: String,
  val runName: String,
  val agent: String,
  val lifecycle: String,
  val observedState: String,
  val detectionSource: String,
  val branch: String,
  val baseRef: String,
  val worktreePath: String,
  val restorable: Boolean = false,
  val createdAt: Long,
  val updatedAt: Long,
)
