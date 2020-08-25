package gg.strims.ppspp

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import gg.strims.ppspp.proto.Api

class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val client = FrontendRPCClient(filesDir.canonicalPath)
        val resp = client.createProfile(
            Api.CreateProfileRequest.newBuilder().setName("test").setPassword("majora").build()
        ).get()!!

        print(resp.profile.toString())
    }
}