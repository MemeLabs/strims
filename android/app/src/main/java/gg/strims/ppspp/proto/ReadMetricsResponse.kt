// Code generated by Wire protocol buffer compiler, do not edit.
// Source: ReadMetricsResponse in api.proto
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

class ReadMetricsResponse(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY
  )
  val data: ByteString = ByteString.EMPTY,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<ReadMetricsResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is ReadMetricsResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (data != other.data) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + data.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """data=$data"""
    return result.joinToString(prefix = "ReadMetricsResponse{", separator = ", ", postfix = "}")
  }

  fun copy(data: ByteString = this.data, unknownFields: ByteString = this.unknownFields):
      ReadMetricsResponse = ReadMetricsResponse(data, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<ReadMetricsResponse> = object : ProtoAdapter<ReadMetricsResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      ReadMetricsResponse::class, 
      "type.googleapis.com/ReadMetricsResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: ReadMetricsResponse): Int {
        var size = value.unknownFields.size
        if (value.data != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
            value.data)
        return size
      }

      override fun encode(writer: ProtoWriter, value: ReadMetricsResponse) {
        if (value.data != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1, value.data)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): ReadMetricsResponse {
        var data: ByteString = ByteString.EMPTY
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> data = ProtoAdapter.BYTES.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return ReadMetricsResponse(
          data = data,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: ReadMetricsResponse): ReadMetricsResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
