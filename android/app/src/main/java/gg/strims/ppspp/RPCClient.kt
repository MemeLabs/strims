package gg.strims.ppspp

import android.util.Log
import com.google.protobuf.Message
import gg.strims.ppspp.proto.Rpc
import java.io.ByteArrayOutputStream
import java.util.*
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.util.concurrent.Semaphore
import kotlin.concurrent.timerTask

class RPCClientError(message: String) : Exception(message)

data class RPCEvent(val rawValue: Int) : BitSet() {
    companion object {
        val data: RPCEvent = RPCEvent(rawValue = 1 shl 0)
        val close: RPCEvent = RPCEvent(rawValue = 1 shl 1)
        val requestError: RPCEvent = RPCEvent(rawValue = 1 shl 2)
        val responseError: RPCEvent = RPCEvent(rawValue = 1 shl 3)
    }
}

class RPCResponseStream<T : Message>(val close: () -> Unit) {
    var delegate: (T?, RPCEvent) -> Unit = { _: T?, _: RPCEvent -> }
}

open class RPCClient(filepath: String) {
    companion object {
        const val TAG = "RPCClient"

        // const val callbackMethod = "_CALLBACK"
        const val cancelMethod = "_CANCEL"
        const val anyURLPrefix = "strims.gg/"
    }

    val executor: ExecutorService = Executors.newCachedThreadPool()

    private var nextCallID: Long = 0
    var callbacks: MutableMap<Long, (Rpc.Call) -> Unit> = mutableMapOf()
    private var g: AndroidBridge = AndroidBridge(filepath)

    init {
        this.g.onData = { b: ByteArray? -> this.handleCallback(b) }
    }

    private fun handleCallback(b: ByteArray?) {
        val stream = b!!.inputStream()
        try {
            // parse call
            val call = Rpc.Call.parseDelimitedFrom(stream)
            this.callbacks[call.parentId]?.let {
                it(call)
            } ?: throw RPCClientError("missing callback")
        } catch (e: Exception) {
            Log.e(TAG, "could not parse message", e)
        }
    }

    fun getNextCallID(): Long {
        this.nextCallID += 1
        return this.nextCallID
    }

    fun <T : Message> call(method: String, arg: T, callID: Long, parentID: Long = 0) {
        val packed = com.google.protobuf.Any.pack(arg, anyURLPrefix)

        val call = Rpc.Call.newBuilder()
            .setParentId(parentID)
            .setId(callID)
            .setArgument(packed)
            .setMethod(method)
            .build()

        val stream = ByteArrayOutputStream()
        call.writeDelimitedTo(stream)
        stream.close()

        this.g.write(stream.toByteArray())
    }

    fun <T : Message> call(method: String, arg: T) {
        this.call(method, arg, this.getNextCallID())
    }

    // inline required to get refined type information, see https://stackoverflow.com/a/46870546/5698680
    inline fun <T : Message, reified R : Message> callStreaming(
        method: String,
        arg: T
    ): RPCResponseStream<R> {
        val callID = this.getNextCallID()
        val stream = RPCResponseStream<R> {
            this.callbacks.remove(callID)
            try {
                this.call(
                    cancelMethod,
                    Rpc.Cancel.getDefaultInstance(),
                    this.getNextCallID(),
                    callID
                )
            } catch (e: Exception) {
                Log.e(TAG, "failed to send call", e)
            }
        }

        this.callbacks[callID] = {
            try {
                when {
                    it.argument.`is`(R::class.java) -> {
                        val unpacked = it.argument.unpack(R::class.java)
                        stream.delegate(unpacked, RPCEvent.data)
                    }
                    it.argument.`is`(Rpc.Close::class.java) -> {
                        callbacks.remove(callID)
                        stream.delegate(null, RPCEvent.close)
                    }
                    it.argument.`is`(Rpc.Error::class.java) -> {
                        callbacks.remove(callID)
                        stream.delegate(null, RPCEvent.responseError)
                    }
                    else -> { // error
                        callbacks.remove(callID)
                        stream.delegate(null, RPCEvent.responseError)
                    }
                }
            } catch (e: Exception) {
                stream.delegate(null, RPCEvent.responseError)
            }
        }

        try {
            this.call(method, arg, callID)
        } catch (e: Exception) {
            this.callbacks.remove(callID)
            stream.delegate(null, RPCEvent.requestError)
            throw e
        }

        return stream
    }

    inline fun <T : Message, reified R : Message> callUnary(method: String, arg: T): Future<R> {
        val callId = getNextCallID()
        return executor.submit<R> {
            // set timeout
            val timer = Timer()
            val s = Semaphore(1)
            var ex: Throwable? = null
            timer.schedule(timerTask {
                callbacks.remove(callId)
                ex = RPCClientError("call timeout")
                s.release()
            }, 5L * 1000) // five seconds


            // prepare callback
            var result: R? = null
            s.acquire()
            callbacks[callId] = {
                callbacks.remove(callId)
                timer.cancel()
                when {
                    it.argument.`is`(R::class.java) -> {
                        result = it.argument.unpack(R::class.java)
                        Log.i(TAG, result.toString())
                    }
                    it.argument.`is`(Rpc.Error::class.java) -> {
                        val error = it.argument.unpack(Rpc.Error::class.java)
                        ex = RPCClientError(error.message)
                    }
                    else -> {
                        ex = RPCClientError("unexpected response type ${it.argument.typeUrl}")
                    }
                }
                s.release()
            }

            // call method
            try {
                call(method, arg, callId)
            } catch (e: Exception) {
                callbacks.remove(callId)
                throw e
            }

            // wait for response
            s.acquire()
            if (ex != null) {
                throw ex as Throwable
            }
            return@submit result
        }
    }
}
