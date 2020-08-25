package gg.strims.ppspp

import java.io.ByteArrayOutputStream
import java.util.*
import java.util.concurrent.Future
import kotlin.Exception

import com.google.protobuf.*

class RPCClientError(message: String): Exception(message)

data class RPCEvent(val rawValue: Int): BitSet() {
    companion object {
        val data: RPCEvent = RPCEvent(rawValue = 1 shl 0)
        val close: RPCEvent = RPCEvent(rawValue = 1 shl 1)
        val requestError: RPCEvent = RPCEvent(rawValue = 1 shl 2)
        val responseError: RPCEvent = RPCEvent(rawValue = 1 shl 3)
    }
}

class RPCResponseStream<T: Message>(close: () -> Unit) {
    public val close: () -> Unit = close
    public var delegate: (T?, RPCEvent) -> Unit = { _: T?, _: RPCEvent -> }
}

class RPCClient {
    companion object {
        private val callbackMethod = "_CALLBACK"
        private val cancelMethod = "_CANCEL"
        private val anyURLPrefix = "strims.gg/"
    }
    private var nextCallID: ULong = 0u
    private var callbacks: MutableMap<ULong, (PBCall) -> Unit> = mutableMapOf()
    private var g: AndroidBridge = AndroidBridge("")

    constructor() {
        this.g.onData = {b: ByteArray? -> this.handleCallback(b)}
    }

    private fun handleCallback(b: ByteArray?) {
        val stream = b!!.inputStream()
        try {
            // parse call
            val call = TODO()
            this.callbacks[call]?.let {
                it(call)
            } ?: throw RPCClientError("missing callback")
        }
        catch (e: Exception) {
            print("error: $e")
        }
    }

    private fun getNextCallID(): ULong {
        this.nextCallID += 1u
        return this.nextCallID
    }

    private fun <T: Message>call(method: String, arg: T, callID: ULong, parentID: ULong = 0u) {
        val arg = TODO()
        val call = TODO()

        val stream = ByteArrayOutputStream()

        stream.close()

        this.g.write(stream.toByteArray())
    }

    public fun <T: Message, R: Message>call(method: String, arg: T) {
        this.call(method, arg, this.getNextCallID())
    }

    public fun <T: Message, R: Message>callStreaming(method: String, arg: T): RPCResponseStream<R> {
        val callID = this.getNextCallID()
        val stream = RPCResponseStream<R> {
            this.callbacks.remove(callID)
            try {
                this.call(RPCClient.cancelMethod, PBCancel(), this.getNextCallID(), callID)
            } catch (e: Exception) {
            }
        }

        this.callbacks[callID]?.let {
            try {

            } catch(e: Exception) {
                stream.delegate(null, RPCEvent.responseError)
            }
        }

        try {
            this.call(method, arg, callID)
        } catch(e: Exception) {
            this.callbacks.remove(callID)
            stream.delegate(null, RPCEvent.requestError)
            throw e
        }

        return stream
    }

    public fun <T: Message, R: Message>callUnary(method: String, arg: T): Future<R> {
        TODO("learn futures")
    }
}

