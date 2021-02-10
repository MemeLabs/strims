package gg.strims.rpc

import android.util.Log
import com.squareup.wire.*
import gg.strims.ppspp.proto.Call
import gg.strims.ppspp.proto.Cancel
import gg.strims.ppspp.proto.Close
import gg.strims.ppspp.proto.Error
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import okio.buffer
import okio.sink
import okio.source
import java.io.ByteArrayOutputStream
import java.util.*
import java.util.concurrent.*
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ConcurrentMap
import kotlin.concurrent.timerTask
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.coroutines.suspendCoroutine

class RPCClientError(message: String?) : Exception(message)

data class RPCEvent(val rawValue: Int) : BitSet() {
    companion object {
        val data: RPCEvent = RPCEvent(rawValue = 1 shl 0)
        val close: RPCEvent = RPCEvent(rawValue = 1 shl 1)
        val requestError: RPCEvent = RPCEvent(rawValue = 1 shl 2)
        val responseError: RPCEvent = RPCEvent(rawValue = 1 shl 3)
    }
}

class RPCResponseStream<T : Message<*, *>>(val close: () -> Unit) {
    var delegate: (T?, RPCEvent) -> Unit = { _: T?, _: RPCEvent -> }
}

open class RPCClient(filepath: String) {
    companion object {
        const val TAG = "RPCClient"

        // const val callbackMethod = "_CALLBACK"
        const val cancelMethod = "_CANCEL"
    }

    val executor: ExecutorService = Executors.newCachedThreadPool()

    private var nextCallID: AtomicLong = AtomicLong(0)
    var callbacks: ConcurrentMap<Long, (Call) -> Unit> = ConcurrentHashMap()
    private var g: AndroidBridge = AndroidBridge(filepath)

    init {
        this.g.onData = { b: ByteArray? -> this.handleCallback(b) }
    }

    private fun handleCallback(b: ByteArray?) {
        val stream = b!!.inputStream()
        try {
            val s2 = stream.source().buffer()
            // discard length prefix
            while (!s2.exhausted() && s2.readByte().toInt() and 0x80 != 0) {}
            val call = Call.ADAPTER.decode(s2)
            s2.close()
            Log.d(TAG, "decoded call ${call.toString()}")
            this.callbacks[call.parent_id]?.let {
                it(call)
            } ?: throw RPCClientError("missing callback")
        } catch (e: Exception) {
            Log.e(TAG, "could not parse message", e)
        }
    }

    fun getNextCallID(): Long = this.nextCallID.incrementAndGet()

    fun typeName(typeUrl: String?): String? = typeUrl?.substringAfter("/")

    fun <T : Message<T, *>> call(method: String, arg: T, callID: Long, parentID: Long = 0) {
        val packed = AnyMessage.pack(arg)

        val call = Call(parent_id = parentID, id = callID, method = method, argument = packed)
        val stream = ByteArrayOutputStream()
        val s2 = stream.sink().buffer()
        val pw = ProtoWriter(s2)
        Log.i("test", "encoded size is ${arg.adapter.encodedSize(arg)}")
        pw.writeVarint64(Call.ADAPTER.encodedSize(call).toLong())
        call.encode(s2)
        s2.close()

        this.g.write(stream.toByteArray())
    }

    fun <T : Message<T, *>> call(method: String, arg: T) {
        this.call(method, arg, this.getNextCallID())
    }

    // inline required to get refined type information, see https://stackoverflow.com/a/46870546/5698680
    inline fun <T : Message<T, *>, reified R : Message<R, *>> callStreaming(
        method: String,
        arg: T
    ): RPCResponseStream<R> {
        val callID = getNextCallID()
        val stream = RPCResponseStream<R> {
            callbacks.remove(callID)
            GlobalScope.launch {
                try {
                    call(
                        cancelMethod,
                        Cancel(),
                        getNextCallID(),
                        callID
                    )
                } catch (e: Exception) {
                    Log.e(TAG, "failed to send call", e)
                }
            }
        }

        Log.i(TAG, "creating callback $callID")
        callbacks[callID] = callback@ {
            Log.i(TAG, "in callback")

            val adapter = ProtoAdapter.get(R::class.java)
            try {
                when (typeName(it.argument?.typeUrl)) {
                    typeName(adapter.typeUrl) -> {
                        stream.delegate(adapter.decode(it.argument?.value!!), RPCEvent.data)
                        return@callback
                    }
                    typeName(Close.ADAPTER.typeUrl) -> {
                        stream.delegate(null, RPCEvent.close)
                    }
                    else -> {
                        stream.delegate(null, RPCEvent.responseError)
                    }
                }
            } catch (e: Exception) {
                stream.delegate(null, RPCEvent.responseError)
            }

            Log.d(TAG, "removing callback $callID")
            callbacks.remove(callID)
        }

        GlobalScope.launch {
            try {
                call(method, arg, callID)
            } catch (e: Exception) {
                callbacks.remove(callID)
                stream.delegate(null, RPCEvent.requestError)
                throw e
            }
        }

        return stream
    }

    suspend inline fun <T : Message<T, *>, reified R : Message<R, *>> callUnary(method: String, arg: T): R = suspendCoroutine { cont ->
        val callId = getNextCallID()

        // set timeout
        val timer = Timer()
        timer.schedule(timerTask {
            callbacks.remove(callId)
            cont.resumeWithException(RPCClientError("call timeout"))
        }, 5L * 1000) // five seconds

        // prepare callback
        Log.i(TAG, "creating callback")
        callbacks[callId] = {
            Log.i(TAG, "in callback")
            callbacks.remove(callId)
            timer.cancel()

            val adapter = ProtoAdapter.get(R::class.java)
            try {
                when (typeName(it.argument?.typeUrl)) {
                    typeName(adapter.typeUrl) -> {
                        cont.resume(adapter.decode(it.argument?.value!!))
                    }
                    typeName(Error.ADAPTER.typeUrl) -> {
                        cont.resumeWithException(RPCClientError(Error.ADAPTER.decode(it.argument?.value!!).message))
                    }
                    else -> {
                        cont.resumeWithException(RPCClientError("unexpected response type ${it.argument?.typeUrl}"))
                    }
                }
            } catch (e: Exception) {
                cont.resumeWithException(e)
            }
        }

        // call method
        try {
            Log.i(TAG, "executing call")
            call(method, arg, callId)
            Log.i(TAG, "executed call")
        } catch (e: Exception) {
            Log.e(TAG, "got error", e)
            callbacks.remove(callId)
            throw e
        }
    }
}
