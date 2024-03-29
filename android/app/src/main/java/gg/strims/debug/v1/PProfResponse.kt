// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.debug.v1.PProfResponse in debug/v1/debug.proto
package gg.strims.debug.v1

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

class PProfResponse(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY
  )
  val name: String = "",
  @field:WireField(
    tag = 2,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY
  )
  val data: ByteString = ByteString.EMPTY,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<PProfResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is PProfResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (name != other.name) return false
    if (data != other.data) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + name.hashCode()
      result = result * 37 + data.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """name=${sanitize(name)}"""
    result += """data=$data"""
    return result.joinToString(prefix = "PProfResponse{", separator = ", ", postfix = "}")
  }

  fun copy(
    name: String = this.name,
    data: ByteString = this.data,
    unknownFields: ByteString = this.unknownFields
  ): PProfResponse = PProfResponse(name, data, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<PProfResponse> = object : ProtoAdapter<PProfResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      PProfResponse::class, 
      "type.googleapis.com/strims.debug.v1.PProfResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: PProfResponse): Int {
        var size = value.unknownFields.size
        if (value.name != "") size += ProtoAdapter.STRING.encodedSizeWithTag(1, value.name)
        if (value.data != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(2,
            value.data)
        return size
      }

      override fun encode(writer: ProtoWriter, value: PProfResponse) {
        if (value.name != "") ProtoAdapter.STRING.encodeWithTag(writer, 1, value.name)
        if (value.data != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 2, value.data)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): PProfResponse {
        var name: String = ""
        var data: ByteString = ByteString.EMPTY
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> name = ProtoAdapter.STRING.decode(reader)
            2 -> data = ProtoAdapter.BYTES.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return PProfResponse(
          name = name,
          data = data,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: PProfResponse): PProfResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
