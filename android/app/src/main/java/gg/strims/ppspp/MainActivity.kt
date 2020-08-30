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
import gg.strims.ppspp.proto.Api.*
import gg.strims.ppspp.rpc.FrontendRPCClient
import gg.strims.ppspp.rpc.RPCEvent
import gg.strims.ppspp.ui.PpsppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking

private const val TAG = "ppspp"

class MainActivity : AppCompatActivity() {
    private var host = "10.0.2.2:0"
    private var addr = "10.0.2.2:8080"
    private var videoUrl: MutableState<String> = mutableStateOf("")
    var isSignedIn: MutableState<Boolean> = mutableStateOf(false)
    var inSwarm: MutableState<Boolean> = mutableStateOf(false)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        var client = FrontendRPCClient(filesDir.canonicalPath)
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
                CreateProfileRequest.newBuilder().setName(username).setPassword(password)
                    .build()
            ).get()!!
            Log.i(TAG, "profile: ${resp.profile}")
        } catch (e: Exception) {
            Log.e(TAG, "creating profile failed: $e")
        }

    fun FrontendRPCClient.handleLogin(username: String, password: String): Any = try {
        val profilesResp = this.getProfiles(GetProfilesRequest.newBuilder().build()).get()!!
        try {
            val profileResp = this.loadProfile(
                LoadProfileRequest.newBuilder()
                    .setId(profilesResp.profilesList
                        .filter { p -> p.name == username }
                        .map { p -> p.id }
                        .first())
                    .setName(username).setPassword(password).build()
            ).get()!!
            Log.i(TAG, "logged in: ${profileResp.profile}")
        } catch (e: Exception) {
            Log.e(TAG, "loading profile failed: $e")
        }
    } catch (e: Exception) {
        Log.e(TAG, "loading profiles failed: $e")
    }

    private fun FrontendRPCClient.handleCreateBootstrapClient(): Any = try {
        val resp = this.createBootstrapClient(
            CreateBootstrapClientRequest.newBuilder().setWebsocketOptions(
                BootstrapClientWebSocketOptions.newBuilder()
                    .setUrl("ws://$addr/test-bootstrap").setInsecureSkipVerifyTls(true)
                    .build()
            ).build()
        ).get()!!
        Log.i(TAG, "bootstrap client: ${resp.bootstrapClient}")
    } catch (e: Exception) {
        Log.e(TAG, "creating bootstrap client failed", e)
    }

    private fun FrontendRPCClient.handleLoadInviteCert(cert: String): Any = try {
        val resp = this.createNetworkMembershipFromInvitation(
            CreateNetworkMembershipFromInvitationRequest.newBuilder()
                .setInvitationB64(cert)
                .build()
        ).get()!!
        Log.i(TAG, "membership: ${resp.membership}")
    } catch (e: Exception) {
        Log.e(TAG, "creating network failed: $e")
    }

    private fun FrontendRPCClient.handleCreateNetwork(): Any = try {
        val resp = this.createNetwork(CreateNetworkRequest.newBuilder().setName("test").build())
            .get()!!
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
        while (root.hasParent()) {
            root = root.parent
        }
        return root
    }

    private fun FrontendRPCClient.publishSwarm() = try {
        val memberships = this.getNetworkMemberships().get()!!
        memberships.networkMembershipsList.map {
            this.publishSwarm(
                PublishSwarmRequest.newBuilder().setId(it.id)
                    .setNetworkKey(rootCert(it.certificate).key)
                    .build()
            ).get()!!
        }
    } catch (e: Exception) {
        Log.e(TAG, "publishing swarm failed: $e")
    }

    fun FrontendRPCClient.startHLSEgress(id: Long) = try {
        val resp =
            this.startHLSEgress(
                StartHLSEgressRequest.newBuilder().setVideoId(id).build()
            ).get()!!

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
        val client = this.openVideoClient()
        client.delegate = { event: VideoClientEvent?, eventType: RPCEvent ->
            Log.i(TAG, eventType.toString())
            when (eventType) {
                RPCEvent.data -> {
                    event!!.bodyCase?.let {
                        when (it) {
                            VideoClientEvent.BodyCase.OPEN -> {
                                Log.i(TAG, "open: ${event.open.id}")
                                this.publishSwarm(
                                    PublishSwarmRequest.newBuilder().setId(event.open.id)
                                        .build()
                                )
                                // this.startHLSEgress(event.open.id)
                            }
                            VideoClientEvent.BodyCase.DATA -> {
                                Log.i(TAG, "video data: ${event.data.data.count()}")
                            }
                            VideoClientEvent.BodyCase.CLOSE -> Log.i(TAG, "close")
                            else -> Log.e(TAG, "vpn rpc error")
                        }
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
                usernameState.text = getRandomString(10)
            }
            val passwordState = remember { PasswordState() }
            if (passwordState.text.isEmpty()) {
                passwordState.text = getRandomString(10)
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
            MockButton("Load invite cert") { client.handleLoadInviteCert("") }
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