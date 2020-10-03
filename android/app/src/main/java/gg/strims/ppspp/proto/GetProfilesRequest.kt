// Code generated by Wire protocol buffer compiler, do not edit.
// Source: GetProfilesRequest in profile.proto
package gg.strims.ppspp.proto

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

class GetProfilesRequest(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<GetProfilesRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is GetProfilesRequest) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "GetProfilesRequest{}"

  fun copy(unknownFields: ByteString = this.unknownFields): GetProfilesRequest =
      GetProfilesRequest(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<GetProfilesRequest> = object : ProtoAdapter<GetProfilesRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      GetProfilesRequest::class, 
      "type.googleapis.com/GetProfilesRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: GetProfilesRequest): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: GetProfilesRequest) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): GetProfilesRequest {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return GetProfilesRequest(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: GetProfilesRequest): GetProfilesRequest = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}