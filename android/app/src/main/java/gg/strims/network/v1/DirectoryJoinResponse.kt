// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.network.v1.DirectoryJoinResponse in network/v1/directory.proto
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

class DirectoryJoinResponse(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<DirectoryJoinResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is DirectoryJoinResponse) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "DirectoryJoinResponse{}"

  fun copy(unknownFields: ByteString = this.unknownFields): DirectoryJoinResponse =
      DirectoryJoinResponse(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<DirectoryJoinResponse> = object : ProtoAdapter<DirectoryJoinResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      DirectoryJoinResponse::class, 
      "type.googleapis.com/strims.network.v1.DirectoryJoinResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: DirectoryJoinResponse): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: DirectoryJoinResponse) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): DirectoryJoinResponse {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return DirectoryJoinResponse(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: DirectoryJoinResponse): DirectoryJoinResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}