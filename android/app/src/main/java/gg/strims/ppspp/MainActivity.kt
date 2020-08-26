package gg.strims.ppspp

import android.os.Bundle
import android.util.Log
import android.view.LayoutInflater
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import gg.strims.ppspp.proto.Api

class MainActivity : AppCompatActivity() {
    private val TAG = "MainActivity"
    private lateinit var recyclerView: RecyclerView
    private lateinit var viewAdapter: RecyclerView.Adapter<*>
    private lateinit var viewManager: RecyclerView.LayoutManager
    private val stdOut = mutableListOf<String>()


    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)


        viewManager = LinearLayoutManager(this)
        viewAdapter = LogListAdapter(stdOut)

        recyclerView = findViewById<RecyclerView>(R.id.stdOut).apply {
            setHasFixedSize(true)
            layoutManager = viewManager
            adapter = viewAdapter
        }

        val client = FrontendRPCClient(filesDir.canonicalPath)

        val createClientBtn = findViewById<Button>(R.id.createClientBtn)
        createClientBtn.setOnClickListener {
            client.handleCreateBootstrapClient()
        }

        val createProfileBtn = findViewById<Button>(R.id.createProfileBtn)
        createProfileBtn.setOnClickListener {
            val username = findViewById<EditText>(R.id.usernameField).text.toString()
            val password = findViewById<EditText>(R.id.passwordField).text.toString()
            client.handleCreateProfile(password, username)
        }

        val loginBtn = findViewById<Button>(R.id.loginBtn)
        loginBtn.setOnClickListener {
            val username = findViewById<EditText>(R.id.usernameField).text.toString()
            val password = findViewById<EditText>(R.id.passwordField).text.toString()
            client.handleLogin(password, username)
        }
    }

    class LogListAdapter(private val data: List<String>) :
        RecyclerView.Adapter<LogListAdapter.MyViewHolder>() {

        // Provide a reference to the views for each data item
        // Complex data items may need more than one view per item, and
        // you provide access to all the views for a data item in a view holder.
        // Each data item is just a string in this case that is shown in a TextView.
        class MyViewHolder(val textView: TextView) : RecyclerView.ViewHolder(textView)


        // Create new views (invoked by the layout manager)
        override fun onCreateViewHolder(
            parent: ViewGroup,
            viewType: Int
        ): MyViewHolder {
            // create a new view
            val textView = LayoutInflater.from(parent.context)
                .inflate(R.layout.log_output, parent, false) as TextView

            return MyViewHolder(textView)
        }

        // Replace the contents of a view (invoked by the layout manager)
        override fun onBindViewHolder(holder: MyViewHolder, position: Int) {
            // - get element from your dataset at this position
            // - replace the contents of the view with that element
            holder.textView.text = data[position]
        }

        // Return the size of your dataset (invoked by the layout manager)
        override fun getItemCount() = data.size
    }


    fun FrontendRPCClient.handleCreateProfile(pw: String, username: String): Any = try {
        val resp = this.createProfile(
            Api.CreateProfileRequest.newBuilder().setName(username).setPassword(pw).build()
        ).get()!!
        Log.i(TAG, "profile: ${resp.profile}")
        stdOut.add("profile: ${resp.profile}")
    } catch (e: Exception) {
        stdOut.add("creating profile failed: $e")
        Log.e(TAG, "creating profile failed: $e")
    } finally {
        viewAdapter.notifyItemInserted(stdOut.size - 1)
    }

    private fun FrontendRPCClient.handleLogin(pw: String, username: String): Any = try {
        val profilesResp = this.getProfiles(Api.GetProfilesRequest.newBuilder().build()).get()!!
        try {
            val profileResp = this.loadProfile(
                Api.LoadProfileRequest.newBuilder()
                    .setId(profilesResp.profilesList
                        .filter { p -> p.name == username }
                        .map { p -> p.id }
                        .first())
                    .setName(username).setPassword(pw).build()
            ).get()!!
            stdOut.add("profile: ${profileResp.profile}")
            Log.i(TAG, "logged in: ${profileResp.profile}")
        } catch (e: Exception) {
            stdOut.add("loading profile failed: $e")
            Log.e(TAG, "loading profile failed: $e")
        }
    } catch (e: Exception) {
        stdOut.add("loading profiles failed: $e")
        Log.e(TAG, "loading profiles failed: $e")
    } finally {
        viewAdapter.notifyItemInserted(stdOut.size - 1)
    }

    private fun FrontendRPCClient.handleCreateBootstrapClient(): Any = try {
        val resp = this.createBootstrapClient(
            Api.CreateBootstrapClientRequest.newBuilder().setWebsocketOptions(
                Api.BootstrapClientWebSocketOptions.newBuilder()
                    .setUrl("ws://localhost:8080/test-bootstrap").build()
            ).build()
        ).get()!!
        stdOut.add("bootstrap client: ${resp.bootstrapClient}")
        Log.i(TAG, "bootstrap client: ${resp.bootstrapClient}")
    } catch (e: Exception) {
        stdOut.add("creating bootstrap client failed: $e")
        Log.e(TAG, "creating bootstrap client failed", e)
    } finally {
        viewAdapter.notifyItemInserted(stdOut.size - 1)
    }

    private fun FrontendRPCClient.handleLoadInviteCert(): Any = try {
        val resp = this.createNetworkMembershipFromInvitation(
            Api.CreateNetworkMembershipFromInvitationRequest.newBuilder()
                .setInvitationB64("EoADCmYIARJA3+jPfL6RMfY8aLFZRDYdmzY5s8gsuEvzrLNOM+KQxDtU0VEHnhGkPOp8mryKzh5ISz1dpRr8xD2kcZMIZ+dNRhogVNFRB54RpDzqfJq8is4eSEs9XaUa/MQ9pHGTCGfnTUYSjwIKIFTRUQeeEaQ86nyavIrOHkhLPV2lGvzEPaRxkwhn501GEAEYBiCH8ob6BSiH56v6BTIQTKUyIcq6qpRJYxpU4CVm0zpAIcPsy2/eBc/FLAp62xJka2WVrWqa8JdnYscnOh80REVOPSQbJ5s1uXQRUqJ8hwUUCMw7rPRhP29ZTV8ZGTznCEKGAQogHVmKdL3JUzXvjh3BQ8tFqFCvzPp7Wxe4ak2yWbjSj/cQARgEINb+kfcFKNbMm5UGMhCGDUmvQDYLYehxX3XjVz/EOkAixCMT3+O7tBwyhTEid0bCtxNkpAN6FkrSHdOiIkAv4wWp/OJ3UzlWpYGaA01wO27gUIEb+")
                .build()
        ).get()!!
        Log.i(TAG, "membership: ${resp.membership}")
    } catch (e: Exception) {
        Log.e(TAG, "creating network failed: $e")
    }

    private fun FrontendRPCClient.handleCreateNetwork(): Any = try {
        val resp = this.createNetwork(Api.CreateNetworkRequest.newBuilder().setName("test").build())
            .get()!!
        Log.i(TAG, "network: ${resp.network}")
    } catch (e: Exception) {
        Log.e(TAG, "creating network failed: $e")
    }

    private fun FrontendRPCClient.handleStartVPN(): Any = try {
        val vpn = this.startVPN()
        vpn.delegate = { event: Api.NetworkEvent?, eventType: RPCEvent ->
            when (eventType) {
                RPCEvent.data -> Log.i(TAG, "vpn event: ${event!!}")
                RPCEvent.close -> Log.i(TAG, "vpn event stream closed")
                else -> Log.e(TAG, "vpn rpc error")
            }
        }
    } catch (e: Exception) {
        Log.e(TAG, "starting vpn failed: $e")
    }

    private fun rootCert(cert: Api.Certificate): Api.Certificate {
        var root = cert
        while (root.hasParent()) {
            root = root.parent
        }
        return root
    }

    private fun FrontendRPCClient.publishSwarm(id: Long): Any = try {
        val memberships = this.getNetworkMemberships().get()!!
        memberships.networkMembershipsList.map {
            this.publishSwarm(
                Api.PublishSwarmRequest.newBuilder().setId(it.id)
                    .setNetworkKey(rootCert(it.certificate).key)
                    .build()
            ).get()!!
        }
    } catch (e: Exception) {
        Log.e(TAG, "publishing swarm failed: $e")
    }

    private fun FrontendRPCClient.handleVideoSwarm(): Any = try {
        val client = this.openVideoClient()
        client.delegate = { event: Api.VideoClientEvent?, eventType: RPCEvent ->
            when (eventType) {
                RPCEvent.data -> {
                    event!!.bodyCase?.let {
                        when (it) {
                            Api.VideoClientEvent.BodyCase.OPEN -> {
                                Log.i(TAG, "open: ${event.open.id}")
                                this.publishSwarm(event.open.id)
                            }
                            Api.VideoClientEvent.BodyCase.DATA -> {
                                Log.i(TAG, "video data: ${event.data.data.count()}")
                            }
                            Api.VideoClientEvent.BodyCase.CLOSE -> Log.i(TAG, "close")
                            else -> Log.e(TAG, "vpn rpc error")
                        }
                    }
                }
                RPCEvent.close -> Log.i(TAG, "vpn event stream closed")
                else -> Log.e(TAG, "vpn rpc error")
            }
        }
    } catch (e: Exception) {
        Log.e(TAG, "joining video swarm failed: $e")
    }
}