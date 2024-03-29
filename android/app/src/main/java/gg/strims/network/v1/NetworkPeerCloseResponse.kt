// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.network.v1.NetworkPeerCloseResponse in network/v1/peer.proto
package gg.strims.network.v1

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import kotlin.Any
import kotlin.AssertionError
import kotlin.Boolean
import kotlin.Deprecated
import kotlin.DeprecationLevel
import kotlin.Int
import kotlin.Long
import kotlin.Nothing
import kotlin.String
import kotlin.jvm.JvmField
import okio.ByteString

class NetworkPeerCloseResponse(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<NetworkPeerCloseResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is NetworkPeerCloseResponse) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "NetworkPeerCloseResponse{}"

  fun copy(unknownFields: ByteString = this.unknownFields): NetworkPeerCloseResponse =
      NetworkPeerCloseResponse(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<NetworkPeerCloseResponse> = object :
        ProtoAdapter<NetworkPeerCloseResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      NetworkPeerCloseResponse::class, 
      "type.googleapis.com/strims.network.v1.NetworkPeerCloseResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: NetworkPeerCloseResponse): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: NetworkPeerCloseResponse) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): NetworkPeerCloseResponse {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return NetworkPeerCloseResponse(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: NetworkPeerCloseResponse): NetworkPeerCloseResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
