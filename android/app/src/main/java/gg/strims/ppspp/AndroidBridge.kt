package gg.strims.ppspp

import bridge.AndroidSide
import bridge.Bridge
import bridge.GoSide
import java.lang.Exception

class AndroidBridge(location: String) : AndroidSide {
    public var onData: (_: ByteArray?) -> Unit = {_: ByteArray? -> }
    public var g : GoSide = Bridge.newGoSide(this, location)
    public var error: Throwable? = null

    fun write(b: ByteArray) {
        this.error?.let { throw it }
        g.write(b)!!
    }

    override fun emitError(msg: String?) {
        print("error: " + msg!!)
    }

    override fun emitData(b: ByteArray?) {
        this.onData(b)
    }
}
