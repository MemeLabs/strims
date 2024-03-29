// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.profile.v1.ListProfilesRequest in profile/v1/profile.proto
package gg.strims.profile.v1

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

class ListProfilesRequest(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<ListProfilesRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is ListProfilesRequest) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "ListProfilesRequest{}"

  fun copy(unknownFields: ByteString = this.unknownFields): ListProfilesRequest =
      ListProfilesRequest(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<ListProfilesRequest> = object : ProtoAdapter<ListProfilesRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      ListProfilesRequest::class, 
      "type.googleapis.com/strims.profile.v1.ListProfilesRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: ListProfilesRequest): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: ListProfilesRequest) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): ListProfilesRequest {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return ListProfilesRequest(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: ListProfilesRequest): ListProfilesRequest = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
