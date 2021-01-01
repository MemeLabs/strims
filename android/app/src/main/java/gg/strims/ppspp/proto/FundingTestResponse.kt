// Code generated by Wire protocol buffer compiler, do not edit.
// Source: FundingTestResponse in funding.proto
package gg.strims.ppspp.proto

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import com.squareup.wire.internal.sanitize
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

class FundingTestResponse(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY
  )
  val message: String = "",
  unknownFields: ByteString = ByteString.EMPTY
) : Message<FundingTestResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is FundingTestResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (message != other.message) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + message.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """message=${sanitize(message)}"""
    return result.joinToString(prefix = "FundingTestResponse{", separator = ", ", postfix = "}")
  }

  fun copy(message: String = this.message, unknownFields: ByteString = this.unknownFields):
      FundingTestResponse = FundingTestResponse(message, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<FundingTestResponse> = object : ProtoAdapter<FundingTestResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      FundingTestResponse::class, 
      "type.googleapis.com/FundingTestResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: FundingTestResponse): Int {
        var size = value.unknownFields.size
        if (value.message != "") size += ProtoAdapter.STRING.encodedSizeWithTag(1, value.message)
        return size
      }

      override fun encode(writer: ProtoWriter, value: FundingTestResponse) {
        if (value.message != "") ProtoAdapter.STRING.encodeWithTag(writer, 1, value.message)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): FundingTestResponse {
        var message: String = ""
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> message = ProtoAdapter.STRING.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return FundingTestResponse(
          message = message,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: FundingTestResponse): FundingTestResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
