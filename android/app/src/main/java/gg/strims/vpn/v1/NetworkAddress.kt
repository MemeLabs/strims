// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.vpn.v1.NetworkAddress in vpn/v1/vpn.proto
package gg.strims.vpn.v1

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import kotlin.Any
import kotlin.AssertionError
import kotlin.Boolean
import kotlin.Deprecated
import kotlin.DeprecationLevel
import kotlin.Int
import kotlin.Long
import kotlin.Nothing
import kotlin.String
import kotlin.hashCode
import kotlin.jvm.JvmField
import okio.ByteString

class NetworkAddress(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "hostId"
  )
  val host_id: ByteString = ByteString.EMPTY,
  @field:WireField(
    tag = 2,
    adapter = "com.squareup.wire.ProtoAdapter#UINT32",
    label = WireField.Label.OMIT_IDENTITY
  )
  val port: Int = 0,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<NetworkAddress, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is NetworkAddress) return false
    if (unknownFields != other.unknownFields) return false
    if (host_id != other.host_id) return false
    if (port != other.port) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + host_id.hashCode()
      result = result * 37 + port.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """host_id=$host_id"""
    result += """port=$port"""
    return result.joinToString(prefix = "NetworkAddress{", separator = ", ", postfix = "}")
  }

  fun copy(
    host_id: ByteString = this.host_id,
    port: Int = this.port,
    unknownFields: ByteString = this.unknownFields
  ): NetworkAddress = NetworkAddress(host_id, port, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<NetworkAddress> = object : ProtoAdapter<NetworkAddress>(
      FieldEncoding.LENGTH_DELIMITED, 
      NetworkAddress::class, 
      "type.googleapis.com/strims.vpn.v1.NetworkAddress", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: NetworkAddress): Int {
        var size = value.unknownFields.size
        if (value.host_id != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
            value.host_id)
        if (value.port != 0) size += ProtoAdapter.UINT32.encodedSizeWithTag(2, value.port)
        return size
      }

      override fun encode(writer: ProtoWriter, value: NetworkAddress) {
        if (value.host_id != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1,
            value.host_id)
        if (value.port != 0) ProtoAdapter.UINT32.encodeWithTag(writer, 2, value.port)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): NetworkAddress {
        var host_id: ByteString = ByteString.EMPTY
        var port: Int = 0
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> host_id = ProtoAdapter.BYTES.decode(reader)
            2 -> port = ProtoAdapter.UINT32.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return NetworkAddress(
          host_id = host_id,
          port = port,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: NetworkAddress): NetworkAddress = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
