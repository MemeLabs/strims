package gg.strims.ppspp

import android.os.Bundle
import android.util.Log
import androidx.appcompat.app.AppCompatActivity
import gg.strims.ppspp.proto.Api

class MainActivity : AppCompatActivity() {
    private val TAG = "strims"

    fun FrontendRPCClient.handleCreateProfile(): Any = try {
        val resp = this.createProfile(
            Api.CreateProfileRequest.newBuilder().setName("test").setPassword("test").build()
        ).get()!!
        Log.i(TAG, "profile: ${resp.profile}")
    } catch (e: RPCClientError) {
        Log.e(TAG, "creating profile failed: $e")
    }

    private fun FrontendRPCClient.handleLogin(): Any = try {
        val profilesResp = this.getProfiles(Api.GetProfilesRequest.newBuilder().build()).get()!!
        try {
            val profileResp = this.loadProfile(
                Api.LoadProfileRequest.newBuilder().setId(profilesResp.profilesList[0].id)
                    .setName("test").setPassword("test").build()
            ).get()!!
            Log.i(TAG, "profile: ${profileResp.profile}")
        } catch (e: RPCClientError) {
            Log.e(TAG, "loading profile failed: $e")
        }
    } catch (e: RPCClientError) {
        Log.e(TAG, "loading profiles failed: $e")
    }

    private fun FrontendRPCClient.handleCreateBootstrapClient(): Any = try {
        val resp = this.createBootstrapClient(
            Api.CreateBootstrapClientRequest.newBuilder().setWebsocketOptions(
                Api.BootstrapClientWebSocketOptions.newBuilder()
                    .setUrl("ws://localhost:8080/test-bootstrap").build()
            ).build()
        ).get()!!
        Log.i(TAG, "bootstrap client: ${resp.bootstrapClient}")
    } catch (e: RPCClientError) {
        Log.e(TAG, "creating bootstrap client failed: $e")
    }

    private fun FrontendRPCClient.handleLoadInviteCert(): Any = try {
        val resp = this.createNetworkMembershipFromInvitation(
            Api.CreateNetworkMembershipFromInvitationRequest.newBuilder()
                .setInvitationB64("EoADCmYIARJA3+jPfL6RMfY8aLFZRDYdmzY5s8gsuEvzrLNOM+KQxDtU0VEHnhGkPOp8mryKzh5ISz1dpRr8xD2kcZMIZ+dNRhogVNFRB54RpDzqfJq8is4eSEs9XaUa/MQ9pHGTCGfnTUYSjwIKIFTRUQeeEaQ86nyavIrOHkhLPV2lGvzEPaRxkwhn501GEAEYBiCH8ob6BSiH56v6BTIQTKUyIcq6qpRJYxpU4CVm0zpAIcPsy2/eBc/FLAp62xJka2WVrWqa8JdnYscnOh80REVOPSQbJ5s1uXQRUqJ8hwUUCMw7rPRhP29ZTV8ZGTznCEKGAQogHVmKdL3JUzXvjh3BQ8tFqFCvzPp7Wxe4ak2yWbjSj/cQARgEINb+kfcFKNbMm5UGMhCGDUmvQDYLYehxX3XjVz/EOkAixCMT3+O7tBwyhTEid0bCtxNkpAN6FkrSHdOiIkAv4wWp/OJ3UzlWpYGaA01wO27gUIEb+")
                .build()
        ).get()!!
        Log.i(TAG, "membership: ${resp.membership}")
    } catch (e: RPCClientError) {
        Log.e(TAG, "creating network failed: $e")
    }

    private fun FrontendRPCClient.handleCreateNetwork(): Any = try {
        val resp = this.createNetwork(Api.CreateNetworkRequest.newBuilder().setName("test").build())
            .get()!!
        Log.i(TAG, "network: ${resp.network}")
    } catch (e: RPCClientError) {
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
    } catch (e: RPCClientError) {
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
    } catch (e: RPCClientError) {
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
    } catch (e: RPCClientError) {
        Log.e(TAG, "joining video swarm failed: $e")
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val client = FrontendRPCClient(filesDir.canonicalPath)
        client.handleLogin()
    }
}