package gg.strims.ppspp

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.Text
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.setContent
import androidx.compose.ui.unit.dp
import androidx.ui.tooling.preview.Preview
import gg.strims.ppspp.rpc.FrontendRPCClient
import gg.strims.ppspp.ui.PpsppTheme

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val client = FrontendRPCClient(filesDir.canonicalPath)
        setContent {
            PpsppTheme {
                // A surface container using the 'background' color from the theme
                Surface(color = MaterialTheme.colors.background) {
                    MockButtons(client)
                }
            }
        }
    }
}

@Composable
fun Greeting(name: String) {
    Text(text = "Hello $name!")
}

@Composable
fun MockButtons(client: FrontendRPCClient) {
    Column(Modifier.padding(16.dp).fillMaxWidth()) {
        MockButton("Create mock profile") { client.handleCreateProfile() }
        MockButton("Login") { client.handleLogin() }
        MockButton("Create bootstrap client") { client.handleCreateBootstrapClient() }
        MockButton("Load invite cert") { client.handleLoadInviteCert() }
        MockButton("Create network") { client.handleCreateNetwork() }
        MockButton("Start vpn") { client.handleStartVPN() }
        MockButton("Join video swarm") { client.handleJoinVideoSwarm() }
    }
}

@Composable
fun MockButton(text: String, func: () -> Unit = {}) {
    Button(onClick = func) {
        Text(text)
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    PpsppTheme {
        Greeting("Android")
    }
}