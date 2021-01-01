// Code generated by Wire protocol buffer compiler, do not edit.
// Source: ListBootstrapPeersResponse in vpn.proto
package gg.strims.ppspp.proto

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import com.squareup.wire.internal.immutableCopyOf
import com.squareup.wire.internal.redactElements
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
import kotlin.jvm.JvmField
import okio.ByteString

class ListBootstrapPeersResponse(
  peers: List<BootstrapPeer> = emptyList(),
  unknownFields: ByteString = ByteString.EMPTY
) : Message<ListBootstrapPeersResponse, Nothing>(ADAPTER, unknownFields) {
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.BootstrapPeer#ADAPTER",
    label = WireField.Label.REPEATED
  )
  val peers: List<BootstrapPeer> = immutableCopyOf("peers", peers)

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is ListBootstrapPeersResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (peers != other.peers) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + peers.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (peers.isNotEmpty()) result += """peers=$peers"""
    return result.joinToString(prefix = "ListBootstrapPeersResponse{", separator = ", ", postfix =
        "}")
  }

  fun copy(peers: List<BootstrapPeer> = this.peers, unknownFields: ByteString = this.unknownFields):
      ListBootstrapPeersResponse = ListBootstrapPeersResponse(peers, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<ListBootstrapPeersResponse> = object :
        ProtoAdapter<ListBootstrapPeersResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      ListBootstrapPeersResponse::class, 
      "type.googleapis.com/ListBootstrapPeersResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: ListBootstrapPeersResponse): Int {
        var size = value.unknownFields.size
        size += BootstrapPeer.ADAPTER.asRepeated().encodedSizeWithTag(1, value.peers)
        return size
      }

      override fun encode(writer: ProtoWriter, value: ListBootstrapPeersResponse) {
        BootstrapPeer.ADAPTER.asRepeated().encodeWithTag(writer, 1, value.peers)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): ListBootstrapPeersResponse {
        val peers = mutableListOf<BootstrapPeer>()
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> peers.add(BootstrapPeer.ADAPTER.decode(reader))
            else -> reader.readUnknownField(tag)
          }
        }
        return ListBootstrapPeersResponse(
          peers = peers,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: ListBootstrapPeersResponse): ListBootstrapPeersResponse =
          value.copy(
        peers = value.peers.redactElements(BootstrapPeer.ADAPTER),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}