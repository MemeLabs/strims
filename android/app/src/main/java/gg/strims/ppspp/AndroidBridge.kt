package gg.strims.ppspp

import android.content.Context
import bridge.AndroidSide
import bridge.Bridge
import bridge.GoSide

class AndroidBridge(location: String) : AndroidSide {
    var goSide : GoSide = Bridge.newGoSide(this, location)

    fun write(b: ByteArray) {
        goSide.write(b)
    }

    override fun emitData(b: ByteArray?) {
        TODO("Not yet implemented")
    }

    override fun emitError(msg: String?) {
        TODO("Not yet implemented")
    }
}