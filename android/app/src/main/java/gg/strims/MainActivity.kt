package gg.strims

import android.app.Application
import android.net.Uri
import android.os.Bundle
import android.util.Log
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.Text
import androidx.compose.foundation.layout.*
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
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.ui.tooling.preview.Preview
import com.google.android.exoplayer2.SimpleExoPlayer
import com.google.android.exoplayer2.source.hls.HlsMediaSource
import com.google.android.exoplayer2.ui.PlayerView
import com.google.android.exoplayer2.upstream.DataSource
import com.google.android.exoplayer2.upstream.DefaultDataSourceFactory
import com.google.android.exoplayer2.util.Util
import gg.strims.ui.PasswordState
import gg.strims.ui.UsernameState
import gg.strims.api.chat.v1.*
import gg.strims.rpc.*
import gg.strims.api.network.v1.*
import gg.strims.api.video.v1.*
import gg.strims.api.vpn.v1.*
import gg.strims.api.type.*
import gg.strims.api.network.v1.bootstrap.*
import gg.strims.api.profile.v1.*
import gg.strims.ui.Theme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import okio.ByteString
import okio.ByteString.Companion.decodeBase64


private const val TAG = "strims"

class ProfileViewModel(filepath: String): ViewModel() {
    var isSignedIn: MutableLiveData<Boolean> = MutableLiveData(false)
    var profile: MutableLiveData<Profile> = MutableLiveData(null)

    private var client = ProfileClient(filepath)

    fun createProfile(username: String, password: String) {
        viewModelScope.launch {
            try {
                profile.setValue(
                    client.create(
                        CreateProfileRequest(name = username, password = password)
                    ).profile
                )
                isSignedIn.setValue(true)
                Log.i(TAG, "profile: ${profile.value}")
            } catch (e: Exception) {
                Log.e(TAG, "creating profile failed: $e")
            }
        }
    }

    fun login(username: String, password: String) {
        viewModelScope.launch {
            try {
                val profiles = client.list().profiles
                profile.setValue(client.load(
                    LoadProfileRequest(
                        id = (profiles
                            .filter { p -> p.name == username }
                            .map { p -> p.id }
                            .first()),
                        name = username, password = password)
                ).profile)
                Log.i(TAG, "profile: ${profile.value}")
                isSignedIn.setValue(true)
                return@launch
            } catch (e: Exception) {
                Log.e(TAG, "creating profile failed: $e")
            }
        }
    }
}

// TODO: break this down
class MainViewModel(filepath: String): ViewModel() {
    private var addr = "10.0.2.2:8082"
    var videoUrl: MutableState<String> = mutableStateOf("")
    var inSwarm: MutableLiveData<Boolean> = MutableLiveData(false)
    private var bootstrap = BootstrapClient(filepath)
    private var video = VideoClient(filepath)
    private var network = NetworkClient(filepath)

    fun createBootstrapClient() {
        viewModelScope.launch {
            try {
                bootstrap.createClient(
                    CreateBootstrapClientRequest(
                        websocket_options = BootstrapClientWebSocketOptions(
                            url = "ws://$addr/test-bootstrap",
                            insecure_skip_verify_tls = true,
                        )
                    )
                )
                return@launch
            } catch (e: Exception) {
                Log.e(TAG, "failed to create bootstrap client: $e")
            }
        }
    }

    fun loadInviteCert(cert: String) {
        viewModelScope.launch {
            try {
                network.createFromInvitation(CreateNetworkFromInvitationRequest(invitation_b64 = cert))
            } catch (e: Exception) {
                Log.e(TAG, "creating network membership from invite failed: $e")
            }
            return@launch
        }
    }

    fun createNetwork() {
        viewModelScope.launch {
            try {
                network.create(CreateNetworkRequest(name = "test"))
            } catch (e: Exception) {
                Log.e(TAG, "failed to create network: $e")
            }
        }
    }

    private fun publishSwarm(swarmID: Long) {
        viewModelScope.launch {
            try {
//                val memberships = client.getNetworkMemberships().network_memberships
//                video.publishSwarm(
//                    PublishSwarmRequest(id = swarmID, network_key = rootCert(it.certificate!!).key)
//                )
            } catch (e: Exception) {
                Log.e(TAG, "failed to publish swarm($swarmID): $e")
            }
        }
    }

    private fun startHLSEgress(video_id: Long) {
        viewModelScope.launch {
            try {
                val resp = video.startHLSEgress(HLSEgressOpenStreamRequest(video_id = video_id))
                // TODO: pause until first chunk is loaded
                runBlocking { delay(5000) }
                videoUrl.value = resp.url
                Log.i(TAG, videoUrl.value)
            } catch (e: Exception) {
                Log.e(TAG, "failed to start HLS egress: $e")
            }
        }
    }

    fun startVPN() {
        viewModelScope.launch {
            try {
                val vpn = network.startVPN(StartVPNRequest(enable_bootstrap_publishing = false))
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
                Log.e(TAG, "start vpn failed: $e")
            }
        }
    }

    fun joinVideoSwarm(swarmKey: ByteString) {
        viewModelScope.launch {
            val videoClient = video.openClient(
                OpenVideoClientRequest(
                    swarm_key = swarmKey,
                )
            )
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
                            (event.data != null) -> Log.i(
                                TAG,
                                "video data: ${event.data.data.size}"
                            )
                            (event.close != null) -> Log.i(TAG, "close")
                            else -> Log.e(TAG, "vpn rpc error")
                        }
                    }
                    RPCEvent.close -> Log.i(TAG, "vpn event stream closed")
                    else -> Log.e(TAG, "vpn rpc error")
                }
            }
            inSwarm.setValue(true)
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
    val viewModel: ProfileViewModel = viewModel()
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
                onLogin = {
                    viewModel.login(
                        username = usernameState.text,
                        password = passwordState.text
                    )
                },
                onCreate = {
                    viewModel.createProfile(
                        username = usernameState.text,
                        password = passwordState.text
                    )
                },
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
        MockButton("Load invite cert") { viewModel.loadInviteCert("EoADCmYIARJA2Ya1yMkBA9TAmwNYL1A/hK9UV835MNas/DWQ1Tqi9DtJ2j219XLJ6OQQWAU5bit/BNNAo7md2mBESeVEgymMnxogSdo9tfVyyejkEFgFOW4rfwTTQKO5ndpgREnlRIMpjJ8SjwIKIEnaPbX1csno5BBYBTluK38E00CjuZ3aYERJ5USDKYyfEAEYBiDiwvn7BSjit578BTIQecq7nNLRMgfQ8SCCKcn1HjpApdw7mtkQ8I5BfWQ1bTlpXUjX7StJRcRAztx7bXtr04ZccByN60VVZs+zk2AIjec0snKQkO03fOn4HefQ1DcSB0KGAQogSWR/I7EoulBAP/ZjOojV+7Jrw9Vyod6iQoCnEiROlLYQARgEIPbB+fsFKPaPg5oGMhDv3gMxOgR3YiZn6Bnoi48NOkAVE+uslKOCFrG27Lk3W+2samt8BBFkokezWLfH884ztSXKYxVaiA6wiCsSNNKc4DNZYy4fO8PFflSwQG8ADVAKIgR0ZXN0") }
        MockButton("Create network") { viewModel.createNetwork() }
        MockButton("Start vpn") { viewModel.startVPN() }
        MockButton("Join video swarm") { viewModel.joinVideoSwarm("0uJfwk6ks1OwZaokGtXDnkEfeBWQjdESbqqGIIq1fjI=".decodeBase64()!!) }
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

@Composable
fun HomeScreen() {
    val mainViewModel: MainViewModel = viewModel()
    val inSwarm = mainViewModel.inSwarm.observeAsState()

    MockButtons()
    if (inSwarm.value!!) {
        VideoPlayer()
    }
}

@Composable
fun MainScreen() {
    val profileViewModel: ProfileViewModel = viewModel()
    val isSignedIn = profileViewModel.isSignedIn.observeAsState()

    Column(Modifier.padding(2.dp)) {
        if (!isSignedIn.value!!) {
            Login()
        } else {
            HomeScreen()
        }
    }
}

@Suppress("UNCHECKED_CAST")
class ProfileViewModelFactory(private val someString: String): ViewModelProvider.NewInstanceFactory() {
    override fun <T : ViewModel?> create(modelClass: Class<T>): T = ProfileViewModel(someString) as T
}

@Suppress("UNCHECKED_CAST")
class MainViewModelFactory(private val someString: String): ViewModelProvider.NewInstanceFactory() {
    override fun <T : ViewModel?> create(modelClass: Class<T>): T = MainViewModel(someString) as T
}

class MainActivity : AppCompatActivity() {
    private val mvm: MainViewModel by viewModels { MainViewModelFactory(filesDir.canonicalPath) }
    private val pvm: ProfileViewModel by viewModels { ProfileViewModelFactory(filesDir.canonicalPath) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            Theme {
                // A surface container using the 'background' color from the theme
                Surface(color = MaterialTheme.colors.background) {
                    MainScreen()
                }
            }
        }
    }
}

@Composable
fun Greeting(name: String) {
    Text(text = "Hello $name!")
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    Theme {
        Greeting("Android")
    }
}
