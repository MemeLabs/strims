// Code generated by Wire protocol buffer compiler, do not edit.
// Source: DirectoryPartResponse in directory.proto
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

class DirectoryPartResponse(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<DirectoryPartResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is DirectoryPartResponse) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "DirectoryPartResponse{}"

  fun copy(unknownFields: ByteString = this.unknownFields): DirectoryPartResponse =
      DirectoryPartResponse(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<DirectoryPartResponse> = object : ProtoAdapter<DirectoryPartResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      DirectoryPartResponse::class, 
      "type.googleapis.com/DirectoryPartResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: DirectoryPartResponse): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: DirectoryPartResponse) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): DirectoryPartResponse {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return DirectoryPartResponse(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: DirectoryPartResponse): DirectoryPartResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
