// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.network.v1.BrokerProxySendKeysRequest in network/v1/broker_proxy.proto
package gg.strims.network.v1

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import com.squareup.wire.internal.immutableCopyOf
import kotlin.Any
import kotlin.AssertionError
import kotlin.Boolean
import kotlin.Deprecated
import kotlin.DeprecationLevel
import kotlin.Int
import kotlin.Long
import kotlin.Nothing
import kotlin.String
import kotlin.collections.List
import kotlin.hashCode
import kotlin.jvm.JvmField
import okio.ByteString

class BrokerProxySendKeysRequest(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#UINT64",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "proxyId"
  )
  val proxy_id: Long = 0L,
  keys: List<ByteString> = emptyList(),
  unknownFields: ByteString = ByteString.EMPTY
) : Message<BrokerProxySendKeysRequest, Nothing>(ADAPTER, unknownFields) {
  @field:WireField(
    tag = 2,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.REPEATED
  )
  val keys: List<ByteString> = immutableCopyOf("keys", keys)

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is BrokerProxySendKeysRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (proxy_id != other.proxy_id) return false
    if (keys != other.keys) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + proxy_id.hashCode()
      result = result * 37 + keys.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """proxy_id=$proxy_id"""
    if (keys.isNotEmpty()) result += """keys=$keys"""
    return result.joinToString(prefix = "BrokerProxySendKeysRequest{", separator = ", ", postfix =
        "}")
  }

  fun copy(
    proxy_id: Long = this.proxy_id,
    keys: List<ByteString> = this.keys,
    unknownFields: ByteString = this.unknownFields
  ): BrokerProxySendKeysRequest = BrokerProxySendKeysRequest(proxy_id, keys, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<BrokerProxySendKeysRequest> = object :
        ProtoAdapter<BrokerProxySendKeysRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      BrokerProxySendKeysRequest::class, 
      "type.googleapis.com/strims.network.v1.BrokerProxySendKeysRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: BrokerProxySendKeysRequest): Int {
        var size = value.unknownFields.size
        if (value.proxy_id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1, value.proxy_id)
        size += ProtoAdapter.BYTES.asRepeated().encodedSizeWithTag(2, value.keys)
        return size
      }

      override fun encode(writer: ProtoWriter, value: BrokerProxySendKeysRequest) {
        if (value.proxy_id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.proxy_id)
        ProtoAdapter.BYTES.asRepeated().encodeWithTag(writer, 2, value.keys)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): BrokerProxySendKeysRequest {
        var proxy_id: Long = 0L
        val keys = mutableListOf<ByteString>()
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> proxy_id = ProtoAdapter.UINT64.decode(reader)
            2 -> keys.add(ProtoAdapter.BYTES.decode(reader))
            else -> reader.readUnknownField(tag)
          }
        }
        return BrokerProxySendKeysRequest(
          proxy_id = proxy_id,
          keys = keys,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: BrokerProxySendKeysRequest): BrokerProxySendKeysRequest =
          value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}