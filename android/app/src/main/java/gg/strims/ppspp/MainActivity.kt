package gg.strims.ppspp

import android.net.Uri
import android.os.Bundle
import android.util.Log
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.Text
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.layout.Row
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.OutlinedTextField
import androidx.compose.material.Surface
import androidx.compose.runtime.*
import androidx.compose.runtime.livedata.observeAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ContextAmbient
import androidx.compose.ui.platform.setContent
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.viewinterop.viewModel
import androidx.lifecycle.MutableLiveData
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.ui.tooling.preview.Preview
import com.google.android.exoplayer2.SimpleExoPlayer
import com.google.android.exoplayer2.source.hls.HlsMediaSource
import com.google.android.exoplayer2.ui.PlayerView
import com.google.android.exoplayer2.upstream.DefaultDataSourceFactory
import com.google.android.exoplayer2.util.Util
import gg.strims.ppspp.profile.PasswordState
import gg.strims.ppspp.profile.UsernameState
import gg.strims.ppspp.proto.*
import gg.strims.ppspp.rpc.FrontendRPCClient
import gg.strims.ppspp.rpc.RPCEvent
import gg.strims.ppspp.ui.PpsppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

private const val TAG = "ppspp"

// TODO: break this down
class MainViewModel: ViewModel() {
    private var addr = "10.0.2.2:8082"
    var videoUrl: MutableState<String> = mutableStateOf("")
    var isSignedIn: MutableLiveData<Boolean> = MutableLiveData(false)
    var inSwarm: MutableLiveData<Boolean> = MutableLiveData(false)
    private var client: FrontendRPCClient? = null

    fun setup(path: String) {
        client = FrontendRPCClient(filepath = path)
    }

    fun createProfile(username: String, password: String): Profile? {
        if (client == null) return null
        Log.i(TAG, "here")
        var profile: Profile? = null
        viewModelScope.launch {
            try {
                profile  = client!!.createProfile(
                    CreateProfileRequest(name = username, password = password)
                ).profile
                Log.i(TAG, "profile: $profile")
                isSignedIn.setValue(true)
            } catch (e: Exception) {
                Log.e(TAG, "creating profile failed: $e")
            }
        }
        return profile
    }

    fun login(username: String, password: String): Profile? {
        var profile: Profile? = null
        viewModelScope.launch {
            try {
                val profiles = client!!.getProfiles(GetProfilesRequest()).profiles
                profile = client!!.loadProfile(
                    LoadProfileRequest(
                        id = (profiles
                            .filter { p -> p.name == username }
                            .map { p -> p.id }
                            .first()),
                        name = username, password = password)
                ).profile
                Log.i(TAG, "profile: $profile")
                isSignedIn.setValue(true)
                return@launch
            } catch (e: Exception) {
                Log.e(TAG, "creating profile failed: $e")
            }
        }
        return profile
    }

    fun createBootstrapClient(): BootstrapClient? {
        var bootstrapClient: BootstrapClient? = null
        viewModelScope.launch {
            try {
                bootstrapClient = client!!.createBootstrapClient(
                    CreateBootstrapClientRequest(
                        websocket_options = BootstrapClientWebSocketOptions(
                            url = "ws://$addr/test-bootstrap",
                            insecure_skip_verify_tls = true,
                        )
                    )
                ).bootstrap_client
                return@launch
            } catch(e: Exception) {
                Log.e(TAG, "failed to create bootstrap client: $e")
            }
        }
        return bootstrapClient
    }

    fun loadInviteCert(cert: String): NetworkMembership? {
        var networkMembership: NetworkMembership? = null
        viewModelScope.launch {
            try {
                networkMembership = client!!.createNetworkMembershipFromInvitation(
                    CreateNetworkMembershipFromInvitationRequest(invitation_b64 = cert)
                ).membership
            } catch (e: Exception) {
                Log.e(TAG, "creating network membership from invite failed: $e")
            }
            return@launch
        }
        return networkMembership
    }

    fun createNetwork(): Network? {
        var network: Network? = null
        viewModelScope.launch {
            try {
                network = client!!.createNetwork(CreateNetworkRequest(name = "test")).network
            } catch (e: Exception) {
                Log.e(TAG, "failed to create network: $e")
            }
        }
        return network
    }

    private fun publishSwarm(swarmID: Long) {
        viewModelScope.launch {
            try {
                val memberships = client!!.getNetworkMemberships().network_memberships
                memberships.map {
                    client!!.publishSwarm(
                        PublishSwarmRequest(id = swarmID, network_key = rootCert(it.certificate!!).key)
                    )
                }
            } catch(e: Exception) {
                Log.e(TAG, "failed to publish swarm: $e")
            }
        }
    }

    private fun startHLSEgress(video_id: Long) {
        viewModelScope.launch {
            try {
                val resp =  client!!.startHLSEgress(StartHLSEgressRequest(video_id = video_id))
                // TODO: pause until first chunk is loaded
                runBlocking { delay(5000) }
                videoUrl.value = resp.url
                Log.i(TAG, videoUrl.value)
            } catch(e: Exception) {
                Log.e(TAG, "failed to start HLS egress: $e")
            }
        }
    }

    fun startVPN() {
        viewModelScope.launch {
            try {
                val vpn = client!!.startVPN(StartVPNRequest(enable_bootstrap_publishing = false))
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
            } catch(e: Exception) {
                Log.e(TAG, "start vpn failed: $e")
            }
        }
    }

    fun joinVideoSwarm() {
        viewModelScope.launch {
            val videoClient = client!!.openVideoClient(VideoClientOpenRequest())
            videoClient.delegate = { event: VideoClientEvent?, eventType: RPCEvent ->
                Log.i(TAG, eventType.toString())
                when (eventType) {
                    RPCEvent.data -> {
                        when {
                            (event!!.open != null) -> {
                                Log.i(TAG, "open: ${event.open!!.id}")
                                publishSwarm(event.open.id)
                                startHLSEgress(event.open.id)
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
        }
    }

    private fun rootCert(cert: Certificate): Certificate {
        var root = cert
        while(root.parent != null) {
            root = root.parent!!
        }
        return root
    }
}

@Composable
fun Login() {
    val viewModel: MainViewModel = viewModel()
    val usernameState = remember { UsernameState() }
    val passwordState = remember { PasswordState() }

    if (usernameState.text.isEmpty()) {
        usernameState.text = "majora"
    }
    if (passwordState.text.isEmpty()) {
        passwordState.text = "autumn"
    }

    Column(
        modifier = Modifier.fillMaxWidth().fillMaxHeight(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        loginInput(
            usernameState = usernameState,
            passwordState = passwordState,
        )
        Spacer(Modifier.padding(10.dp))
        Row {
            loginButtons(
                onLogin = { viewModel.login(username = usernameState.text, password = passwordState.text) } ,
                onCreate = { viewModel.createProfile(username = usernameState.text, password = passwordState.text) },
            )
        }
    }
}

@Composable
fun loginInput(usernameState: UsernameState, passwordState: PasswordState) {
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
fun MockButton(text: String, func: () -> Unit = {}) {
    Button(onClick = func) {
        Text(text)
    }
}

@Composable
fun loginButtons(
    onLogin: () -> Unit,
    onCreate: () -> Unit,
) {
    MockButton("Create profile", func = onCreate)
    Spacer(modifier = Modifier.padding(2.dp))
    MockButton("Login", func = onLogin)
}

@Composable
fun MockButtons() {
    val viewModel: MainViewModel = viewModel()
    Column(Modifier.padding(16.dp).fillMaxWidth()) {
        MockButton("Create bootstrap client") { viewModel.createBootstrapClient() }
        MockButton("Load invite cert") { viewModel.loadInviteCert("EoADCmYIARJAV8Wik5atNRhHG6q3pLsjG/lBtLLHzqTx0DAM5ZRcM3YQ22BWfAKSO1yTWG1eS2DxK/bVtW9N9xfx3FqXDa0hkhogENtgVnwCkjtck1htXktg8Sv21bVvTfcX8dxalw2tIZISjwIKIBDbYFZ8ApI7XJNYbV5LYPEr9tW1b033F/HcWpcNrSGSEAEYBiCY+av6BSiY7tD6BTIQeKqBCFl2lUO7SkUeGazEijpACq5jjd+OquKU2o8wPvy6ICyyYYpCFKScYx78ofZGq4uMSRf3q2DNsv4ckHp6dpSVIXIN8Y5MOvT4OWBl6YprDUKGAQogsCzueUWDAn2eWw99uFRr8YND7wwiY48Yske2MtCQCh4QARgEIPv4q/oFKPvGtZgGMhDYxi/f9+1ac7iQbWZRkDJXOkAc20513mQ2AJCaJde7+ox/oI4vn9vqZVTUSJQvSK3q+QlkYVwnUD9bwenolByGpkO3Yd7B5Nz8X76irNAbX4IGIgR0ZXN0") }
        MockButton("Create network") { viewModel.createNetwork() }
        MockButton("Start vpn") { viewModel.startVPN() }
        MockButton("Join video swarm") { viewModel.joinVideoSwarm() }
    }
}

@Composable
fun VideoPlayer() {
    val viewModel: MainViewModel = viewModel()
    val uri by viewModel.videoUrl
    val context = ContextAmbient.current
    val exoPlayer = remember {
        SimpleExoPlayer.Builder(context).build()
    }
    val dataSourceFactory = DefaultDataSourceFactory(context, Util.getUserAgent(context, context.packageName))

    Log.i(TAG, uri)
    exoPlayer.prepare(HlsMediaSource.Factory(dataSourceFactory).createMediaSource(Uri.parse(uri)))
    AndroidView({ ctx ->
        PlayerView(ctx).apply {
            player = exoPlayer
            exoPlayer.playWhenReady = true
        }
    })
}

@Composable
fun MainScreen() {
    val viewModel: MainViewModel = viewModel()
    val isSignedIn = viewModel.isSignedIn.observeAsState()
    val inSwarm = viewModel.inSwarm.observeAsState()

    Column(Modifier.padding(2.dp)) {
        if (!isSignedIn.value!!) {
            Login()
        } else {
            if (inSwarm.value!!) {
                VideoPlayer()
            }
            MockButtons()
        }
    }
}

class MainActivity : AppCompatActivity() {
    private var host = "10.0.2.2:0"
    private val vm: MainViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        vm.setup(filesDir.canonicalPath)
        setContent {
            PpsppTheme {
                // A surface container using the 'background' color from the theme
                Surface(color = MaterialTheme.colors.background) {
                    MainScreen()
                }
            }
        }
    }
}

/*
            MockButton("Create bootstrap client") { client.handleCreateBootstrapClient() }
    }

    }*/

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