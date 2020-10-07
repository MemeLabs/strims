package gg.strims.ppspp

import android.net.Uri
import android.os.Bundle
import android.util.Log
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.Text
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.OutlinedTextField
import androidx.compose.material.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ContextAmbient
import androidx.compose.ui.platform.setContent
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.ui.tooling.preview.Preview
import com.google.android.exoplayer2.SimpleExoPlayer
import com.google.android.exoplayer2.source.hls.HlsMediaSource
import com.google.android.exoplayer2.ui.PlayerView
import com.google.android.exoplayer2.upstream.DataSource
import com.google.android.exoplayer2.upstream.DefaultDataSourceFactory
import com.google.android.exoplayer2.util.Util
import gg.strims.ppspp.profile.PasswordState
import gg.strims.ppspp.profile.UsernameState
import gg.strims.ppspp.proto.*
import gg.strims.ppspp.rpc.FrontendRPCClient
import gg.strims.ppspp.rpc.RPCEvent
import gg.strims.ppspp.ui.PpsppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking

private const val TAG = "ppspp"

class MainActivity : AppCompatActivity() {
    private var host = "10.0.2.2:0"
    private var addr = "10.0.2.2:8082"
    private var videoUrl: MutableState<String> = mutableStateOf("")
    var isSignedIn: MutableState<Boolean> = mutableStateOf(false)
    var inSwarm: MutableState<Boolean> = mutableStateOf(false)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val client = FrontendRPCClient(filesDir.canonicalPath)
        setContent {
            PpsppTheme {
                // A surface container using the 'background' color from the theme
                Surface(color = MaterialTheme.colors.background) {
                    Column(Modifier.padding(2.dp)) {
                        if (!isSignedIn.value) {
                            profile(client)
                        } else {
                            if (inSwarm.value) {
                                VideoPlayer(uri = videoUrl.value)
                            }
                            MockButtons(client)
                        }
                    }
                }
            }
        }
    }

    private fun getRandomString(length: Int): String {
        val allowedChars = ('A'..'Z') + ('a'..'z')
        return (1..length)
            .map { allowedChars.random() }
            .joinToString("")
    }

    private fun FrontendRPCClient.handleCreateProfile(username: String, password: String): Any =
        try {
            val resp = this.createProfile(
                CreateProfileRequest(name = username, password = password)
            )
            Log.i(TAG, "profile: ${resp?.withoutUnknownFields()}")
        } catch (e: Exception) {
            Log.e(TAG, "creating profile failed: $e")
        }

    fun FrontendRPCClient.handleLogin(username: String, password: String): Any = try {
        val profilesResp = this.getProfiles(GetProfilesRequest())!!.withoutUnknownFields()
        try {
            val profileResp = this.loadProfile(
                LoadProfileRequest(
                    id = (profilesResp.profiles
                        .filter { p -> p.name == username }
                        .map { p -> p.id }
                        .first()),
                    name = username, password = password)
            )!!.withoutUnknownFields()
            Log.i(TAG, "logged in: ${profileResp.profile}")
        } catch (e: Exception) {
            Log.e(TAG, "loading profile failed: $e")
        }
    } catch (e: Exception) {
        Log.e(TAG, "loading profiles failed: $e")
    }

    private fun FrontendRPCClient.handleCreateBootstrapClient(): Any = try {
        val resp = this.createBootstrapClient(
            CreateBootstrapClientRequest(
                websocket_options = BootstrapClientWebSocketOptions(
                    url = "ws://$addr/test-bootstrap",
                    insecure_skip_verify_tls = true
                )
            )
        )!!.withoutUnknownFields()
        Log.i(TAG, "bootstrap client: ${resp.bootstrap_client}")
    } catch (e: Exception) {
        Log.e(TAG, "creating bootstrap client failed", e)
    }

    private fun FrontendRPCClient.handleLoadInviteCert(cert: String): Any = try {
        val resp = this.createNetworkMembershipFromInvitation(
            CreateNetworkMembershipFromInvitationRequest(invitation_b64 = cert)
        )!!.withoutUnknownFields()
        Log.i(TAG, "membership: ${resp.membership}")
    } catch (e: Exception) {
        Log.e(TAG, "creating network failed: $e")
    }

    private fun FrontendRPCClient.handleCreateNetwork(): Any = try {
        val resp = this.createNetwork(CreateNetworkRequest(name = "test"))!!.withoutUnknownFields()
        Log.i(TAG, "network: ${resp.network}")
    } catch (e: Exception) {
        Log.e(TAG, "creating network failed: $e")
    }

    private fun FrontendRPCClient.handleStartVPN(): Any = try {
        val vpn = this.startVPN()
        vpn.delegate = { event: NetworkEvent?, eventType: RPCEvent ->
            when (eventType) {
                RPCEvent.data -> Log.i(TAG, "vpn event: ${event!!}")
                RPCEvent.close -> Log.i(TAG, "vpn event stream closed")
                RPCEvent.requestError -> Log.i(TAG, "vpn request error")
                RPCEvent.responseError -> Log.i(TAG, "vpn response error")
                else -> Log.e(TAG, "vpn rpc error: $eventType")
            }
        }
        Log.i(TAG, "started vpn")
    } catch (e: Exception) {
        Log.e(TAG, "starting vpn failed: $e")
    }

    private fun rootCert(cert: Certificate): Certificate {
        var root = cert
        while (root.parent != null) {
            root = root.parent!!
        }
        return root
    }

    private fun FrontendRPCClient.publishSwarm(id: Long) = try {
        val memberships = this.getNetworkMemberships()!!.withoutUnknownFields()
        memberships.network_memberships.map {
            this.publishSwarm(
                PublishSwarmRequest(id = id, network_key = rootCert(it.certificate!!).key)
            )!!.withoutUnknownFields()
        }
    } catch (e: Exception) {
        Log.e(TAG, "publishing swarm failed: $e")
    }

    fun FrontendRPCClient.startHLSEgress(id: Long) = try {
        val resp = this.startHLSEgress(StartHLSEgressRequest(video_id = id))!!.withoutUnknownFields()

        // TODO: we need to pause until the first chunk loads
        runBlocking {
            delay(5000)
        }
        Log.i(TAG, resp.url)
        videoUrl.value = resp.url
    } catch (e: Exception) {
        Log.e(TAG, "starting hls egress failed: $e")
    }

    private fun FrontendRPCClient.handleJoinVideoSwarm(): Any = try {
        val client =
            this.openVideoClient(VideoClientOpenRequest())
        client.delegate = { event: VideoClientEvent?, eventType: RPCEvent ->
            Log.i(TAG, eventType.toString())
            when (eventType) {
                RPCEvent.data -> {

                    when {
                        (event!!.open != null) -> {
                            Log.i(TAG, "open: ${event.open?.id}")
                            this.publishSwarm(event.open!!.id) // why do I have to assert not null again?
                            this.startHLSEgress(event.open.id)
                        }
                        (event.data != null) -> Log.i(TAG, "video data: ${event.data.data.size}")
                        (event.close != null) -> Log.i(TAG, "close")
                        else -> Log.e(TAG, "vpn rpc error")
                    }
                }
                RPCEvent.close -> Log.i(TAG, "vpn event stream closed")
                else -> Log.e(TAG, "vpn rpc error")
            }
        }
        Log.i(TAG, "video client opened")
        inSwarm.value = true
    } catch (e: Exception) {
        Log.e(TAG, "joining video swarm failed: $e")
    }


    @Composable
    fun profile(client: FrontendRPCClient) {
        Column(Modifier.padding(4.dp)) {
            val usernameState = remember { UsernameState() }
            if (usernameState.text.isEmpty()) {
                usernameState.text = "majora"
            }
            val passwordState = remember { PasswordState() }
            if (passwordState.text.isEmpty()) {
                passwordState.text = "autumn"
            }
            profileInput(usernameState, passwordState)
            Row {
                Button({
                    client.handleCreateProfile(
                        usernameState.text,
                        passwordState.text
                    )
                    isSignedIn.value = true
                }) { Text("Create profile") }
                Button({
                    client.handleLogin(
                        usernameState.text,
                        passwordState.text,
                    )
                    isSignedIn.value = true
                }) { Text("Login") }
            }
        }
    }

    @Composable
    fun MockButtons(client: FrontendRPCClient) {
        Column(Modifier.padding(16.dp).fillMaxWidth()) {
            MockButton("Create bootstrap client") { client.handleCreateBootstrapClient() }
            MockButton("Load invite cert") { client.handleLoadInviteCert("EoADCmYIARJAV8Wik5atNRhHG6q3pLsjG/lBtLLHzqTx0DAM5ZRcM3YQ22BWfAKSO1yTWG1eS2DxK/bVtW9N9xfx3FqXDa0hkhogENtgVnwCkjtck1htXktg8Sv21bVvTfcX8dxalw2tIZISjwIKIBDbYFZ8ApI7XJNYbV5LYPEr9tW1b033F/HcWpcNrSGSEAEYBiCY+av6BSiY7tD6BTIQeKqBCFl2lUO7SkUeGazEijpACq5jjd+OquKU2o8wPvy6ICyyYYpCFKScYx78ofZGq4uMSRf3q2DNsv4ckHp6dpSVIXIN8Y5MOvT4OWBl6YprDUKGAQogsCzueUWDAn2eWw99uFRr8YND7wwiY48Yske2MtCQCh4QARgEIPv4q/oFKPvGtZgGMhDYxi/f9+1ac7iQbWZRkDJXOkAc20513mQ2AJCaJde7+ox/oI4vn9vqZVTUSJQvSK3q+QlkYVwnUD9bwenolByGpkO3Yd7B5Nz8X76irNAbX4IGIgR0ZXN0") }
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

    @Composable
    fun profileInput(
        usernameState: UsernameState = remember { UsernameState() },
        passwordState: PasswordState = remember { PasswordState() }
    ) {
        OutlinedTextField(
            value = usernameState.text,
            onValueChange = { value -> usernameState.text = value },
            label = { Text("Username") }
        )
        OutlinedTextField(
            value = passwordState.text,
            onValueChange = { value -> passwordState.text = value },
            label = { Text("Password") }
        )
    }

    @Composable
    fun VideoPlayer(uri: String) {
        val context = ContextAmbient.current

        val exoPlayer = remember {
            SimpleExoPlayer.Builder(context).build()
        }
        Log.i(TAG, uri)

        val dataSourceFactory: DataSource.Factory =
            DefaultDataSourceFactory(context, Util.getUserAgent(context, context.packageName))
        val source =
            HlsMediaSource.Factory(dataSourceFactory).createMediaSource(Uri.parse(uri))
        exoPlayer.prepare(source)

        AndroidView({ ctx ->
            PlayerView(ctx).apply {
                player = exoPlayer
                exoPlayer.playWhenReady = true
            }
        })
    }
}

@Composable
fun Greeting(name: String) {
    Text(text = "Hello $name!")
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    PpsppTheme {
        Greeting("Android")
    }
}