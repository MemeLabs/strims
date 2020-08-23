package gg.strims.ppspp

import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import android.widget.TextView

class MainActivity : AppCompatActivity() {


    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        val bridge = AndroidBridge(filesDir.canonicalPath)
        val bytes = ByteArray(1)
        bytes[0] = 1
        bridge.write(bytes)
        val textView = findViewById<TextView>(R.id.helloText)
        textView.text = "done writing"
    }

}