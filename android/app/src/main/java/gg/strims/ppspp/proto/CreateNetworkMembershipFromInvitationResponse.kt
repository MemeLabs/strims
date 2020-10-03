// Code generated by Wire protocol buffer compiler, do not edit.
// Source: CreateNetworkMembershipFromInvitationResponse in profile.proto
package gg.strims.ppspp.proto

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

class CreateNetworkMembershipFromInvitationResponse(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.NetworkMembership#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY
  )
  val membership: NetworkMembership? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<CreateNetworkMembershipFromInvitationResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is CreateNetworkMembershipFromInvitationResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (membership != other.membership) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + membership.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (membership != null) result += """membership=$membership"""
    return result.joinToString(prefix = "CreateNetworkMembershipFromInvitationResponse{", separator
        = ", ", postfix = "}")
  }

  fun copy(membership: NetworkMembership? = this.membership, unknownFields: ByteString =
      this.unknownFields): CreateNetworkMembershipFromInvitationResponse =
      CreateNetworkMembershipFromInvitationResponse(membership, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<CreateNetworkMembershipFromInvitationResponse> = object :
        ProtoAdapter<CreateNetworkMembershipFromInvitationResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      CreateNetworkMembershipFromInvitationResponse::class, 
      "type.googleapis.com/CreateNetworkMembershipFromInvitationResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: CreateNetworkMembershipFromInvitationResponse): Int {
        var size = value.unknownFields.size
        if (value.membership != null) size += NetworkMembership.ADAPTER.encodedSizeWithTag(1,
            value.membership)
        return size
      }

      override fun encode(writer: ProtoWriter,
          value: CreateNetworkMembershipFromInvitationResponse) {
        if (value.membership != null) NetworkMembership.ADAPTER.encodeWithTag(writer, 1,
            value.membership)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): CreateNetworkMembershipFromInvitationResponse {
        var membership: NetworkMembership? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> membership = NetworkMembership.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return CreateNetworkMembershipFromInvitationResponse(
          membership = membership,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: CreateNetworkMembershipFromInvitationResponse):
          CreateNetworkMembershipFromInvitationResponse = value.copy(
        membership = value.membership?.let(NetworkMembership.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}