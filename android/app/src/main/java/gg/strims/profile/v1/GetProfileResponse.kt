// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.profile.v1.GetProfileResponse in profile/v1/profile.proto
package gg.strims.profile.v1

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

class GetProfileResponse(
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.profile.v1.Profile#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY
  )
  val profile: Profile? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<GetProfileResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is GetProfileResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (profile != other.profile) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + profile.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (profile != null) result += """profile=$profile"""
    return result.joinToString(prefix = "GetProfileResponse{", separator = ", ", postfix = "}")
  }

  fun copy(profile: Profile? = this.profile, unknownFields: ByteString = this.unknownFields):
      GetProfileResponse = GetProfileResponse(profile, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<GetProfileResponse> = object : ProtoAdapter<GetProfileResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      GetProfileResponse::class, 
      "type.googleapis.com/strims.profile.v1.GetProfileResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: GetProfileResponse): Int {
        var size = value.unknownFields.size
        if (value.profile != null) size += Profile.ADAPTER.encodedSizeWithTag(2, value.profile)
        return size
      }

      override fun encode(writer: ProtoWriter, value: GetProfileResponse) {
        if (value.profile != null) Profile.ADAPTER.encodeWithTag(writer, 2, value.profile)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): GetProfileResponse {
        var profile: Profile? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            2 -> profile = Profile.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return GetProfileResponse(
          profile = profile,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: GetProfileResponse): GetProfileResponse = value.copy(
        profile = value.profile?.let(Profile.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
