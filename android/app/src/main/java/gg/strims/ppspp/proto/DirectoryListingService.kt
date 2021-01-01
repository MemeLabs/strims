// Code generated by Wire protocol buffer compiler, do not edit.
// Source: DirectoryListingService in directory.proto
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

class DirectoryListingService(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY
  )
  val type: String = "",
  unknownFields: ByteString = ByteString.EMPTY
) : Message<DirectoryListingService, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is DirectoryListingService) return false
    if (unknownFields != other.unknownFields) return false
    if (type != other.type) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + type.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """type=${sanitize(type)}"""
    return result.joinToString(prefix = "DirectoryListingService{", separator = ", ", postfix = "}")
  }

  fun copy(type: String = this.type, unknownFields: ByteString = this.unknownFields):
      DirectoryListingService = DirectoryListingService(type, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<DirectoryListingService> = object :
        ProtoAdapter<DirectoryListingService>(
      FieldEncoding.LENGTH_DELIMITED, 
      DirectoryListingService::class, 
      "type.googleapis.com/DirectoryListingService", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: DirectoryListingService): Int {
        var size = value.unknownFields.size
        if (value.type != "") size += ProtoAdapter.STRING.encodedSizeWithTag(1, value.type)
        return size
      }

      override fun encode(writer: ProtoWriter, value: DirectoryListingService) {
        if (value.type != "") ProtoAdapter.STRING.encodeWithTag(writer, 1, value.type)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): DirectoryListingService {
        var type: String = ""
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> type = ProtoAdapter.STRING.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return DirectoryListingService(
          type = type,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: DirectoryListingService): DirectoryListingService = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}