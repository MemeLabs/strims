// Code generated by Wire protocol buffer compiler, do not edit.
// Source: BootstrapDHTResponse in api.proto
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

class BootstrapDHTResponse(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<BootstrapDHTResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is BootstrapDHTResponse) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "BootstrapDHTResponse{}"

  fun copy(unknownFields: ByteString = this.unknownFields): BootstrapDHTResponse =
      BootstrapDHTResponse(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<BootstrapDHTResponse> = object : ProtoAdapter<BootstrapDHTResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      BootstrapDHTResponse::class, 
      "type.googleapis.com/BootstrapDHTResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: BootstrapDHTResponse): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: BootstrapDHTResponse) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): BootstrapDHTResponse {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return BootstrapDHTResponse(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: BootstrapDHTResponse): BootstrapDHTResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}