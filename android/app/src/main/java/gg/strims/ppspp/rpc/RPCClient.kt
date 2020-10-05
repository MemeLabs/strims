package gg.strims.ppspp.rpc

import android.util.Log
import com.squareup.wire.*
import gg.strims.ppspp.proto.Call
import gg.strims.ppspp.proto.Cancel
import gg.strims.ppspp.proto.Close
import gg.strims.ppspp.proto.Error
import okio.buffer
import okio.sink
import okio.source
import java.io.ByteArrayOutputStream
import java.util.*
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.util.concurrent.Semaphore
import kotlin.concurrent.timerTask

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

    private var nextCallID: Long = 0
    var callbacks: MutableMap<Long, (Call) -> Unit> = mutableMapOf()
    private var g: AndroidBridge = AndroidBridge(filepath)

    init {
        this.g.onData = { b: ByteArray? -> this.handleCallback(b) }
    }

    private fun handleCallback(b: ByteArray?) {
        val stream = b!!.inputStream()
        try {
            val s2 = stream.source().buffer()
            val pr = ProtoReader(s2)
            pr.readVarint64()
            val call = Call.ADAPTER.decode(pr)
            s2.close()
            this.callbacks[call.parent_id]?.let {
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
        val callID = this.getNextCallID()
        val stream = RPCResponseStream<R> {
            this.callbacks.remove(callID)
            try {
                this.call(
                    cancelMethod,
                    Cancel(),
                    this.getNextCallID(),
                    callID
                )
            } catch (e: Exception) {
                Log.e(TAG, "failed to send call", e)
            }
        }

        Log.i("ree", "creating callback")
        this.callbacks[callID] = {
            Log.i("REE", "in callback")
            val unpacked = it.argument?.unpackOrNull(ProtoAdapter.get(R::class.java))
            if (unpacked != null) {
                stream.delegate(unpacked, RPCEvent.data)
            }
            val close = it.argument?.unpackOrNull(Close.ADAPTER)
            if (close != null) {
                callbacks.remove(callID)
                stream.delegate(null, RPCEvent.close)
            }
            val error = it.argument?.unpackOrNull(Error.ADAPTER)
            if (error != null) {
                callbacks.remove(callID)
                stream.delegate(null, RPCEvent.responseError)
            }
            if (unpacked == null && close == null && error == null) {
                callbacks.remove(callID)
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

    inline fun <T : Message<T, *>, reified R : Message<R, *>> callUnary(method: String, arg: T): Future<R> {
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
            Log.i("ree", "creating callback")
            callbacks[callId] = {
                Log.i("REE", "in callback")
                callbacks.remove(callId)
                timer.cancel()
                result = it.argument?.unpackOrNull(ProtoAdapter.get(R::class.java))
                Log.i("ree", result.toString())
                if (result == null) {
                    ex = RPCClientError(it.argument?.unpackOrNull(Error.ADAPTER)?.message)
                }
                if (ex == null) {
                    ex = RPCClientError("unexpected response type ${it.argument?.typeUrl}")
                }

                Log.i("ree", ex.toString())

                s.release()
            }
            Log.i("ree", "created callback")

            // call method
            try {
                Log.i("ree", "executing call")
                call(method, arg, callId)
                Log.i("ree", "executed call")
            } catch (e: Exception) {
                Log.e("ree", "got error", e)
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
