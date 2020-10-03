// Code generated by Wire protocol buffer compiler, do not edit.
// Source: GetNetworksResponse in profile.proto
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

class GetNetworksResponse(
  networks: List<Network> = emptyList(),
  unknownFields: ByteString = ByteString.EMPTY
) : Message<GetNetworksResponse, Nothing>(ADAPTER, unknownFields) {
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.Network#ADAPTER",
    label = WireField.Label.REPEATED
  )
  val networks: List<Network> = immutableCopyOf("networks", networks)

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is GetNetworksResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (networks != other.networks) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + networks.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (networks.isNotEmpty()) result += """networks=$networks"""
    return result.joinToString(prefix = "GetNetworksResponse{", separator = ", ", postfix = "}")
  }

  fun copy(networks: List<Network> = this.networks, unknownFields: ByteString = this.unknownFields):
      GetNetworksResponse = GetNetworksResponse(networks, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<GetNetworksResponse> = object : ProtoAdapter<GetNetworksResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      GetNetworksResponse::class, 
      "type.googleapis.com/GetNetworksResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: GetNetworksResponse): Int {
        var size = value.unknownFields.size
        size += Network.ADAPTER.asRepeated().encodedSizeWithTag(1, value.networks)
        return size
      }

      override fun encode(writer: ProtoWriter, value: GetNetworksResponse) {
        Network.ADAPTER.asRepeated().encodeWithTag(writer, 1, value.networks)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): GetNetworksResponse {
        val networks = mutableListOf<Network>()
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> networks.add(Network.ADAPTER.decode(reader))
            else -> reader.readUnknownField(tag)
          }
        }
        return GetNetworksResponse(
          networks = networks,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: GetNetworksResponse): GetNetworksResponse = value.copy(
        networks = value.networks.redactElements(Network.ADAPTER),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}