package com.example.agentmanagermobile.ui.main

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithText
import org.junit.Rule
import org.junit.Test

/** UI tests for [com.example.agentmanagermobile.ui.main.MainScreen]. */
class MainScreenTest {

  @get:Rule val composeTestRule = createAndroidComposeRule<ComponentActivity>()

  @Test
  fun pairingScreenShowsXTunnelEntryPoint() {
    composeTestRule.setContent {
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

    composeTestRule.onNodeWithText("xTunnel URL").assertExists()
    composeTestRule.onNodeWithText("Pair Android").assertExists()
  }
}
