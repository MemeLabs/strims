// Code generated by Wire protocol buffer compiler, do not edit.
// Source: BootstrapDHTRequest in api.proto
package gg.strims.ppspp.proto

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import com.squareup.wire.internal.immutableCopyOf
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
import kotlin.collections.List
import kotlin.jvm.JvmField
import okio.ByteString

class BootstrapDHTRequest(
  transport_uris: List<String> = emptyList(),
  unknownFields: ByteString = ByteString.EMPTY
) : Message<BootstrapDHTRequest, Nothing>(ADAPTER, unknownFields) {
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.REPEATED,
    jsonName = "transportUris"
  )
  val transport_uris: List<String> = immutableCopyOf("transport_uris", transport_uris)

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is BootstrapDHTRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (transport_uris != other.transport_uris) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + transport_uris.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (transport_uris.isNotEmpty()) result += """transport_uris=${sanitize(transport_uris)}"""
    return result.joinToString(prefix = "BootstrapDHTRequest{", separator = ", ", postfix = "}")
  }

  fun copy(transport_uris: List<String> = this.transport_uris, unknownFields: ByteString =
      this.unknownFields): BootstrapDHTRequest = BootstrapDHTRequest(transport_uris, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<BootstrapDHTRequest> = object : ProtoAdapter<BootstrapDHTRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      BootstrapDHTRequest::class, 
      "type.googleapis.com/BootstrapDHTRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: BootstrapDHTRequest): Int {
        var size = value.unknownFields.size
        size += ProtoAdapter.STRING.asRepeated().encodedSizeWithTag(1, value.transport_uris)
        return size
      }

      override fun encode(writer: ProtoWriter, value: BootstrapDHTRequest) {
        ProtoAdapter.STRING.asRepeated().encodeWithTag(writer, 1, value.transport_uris)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): BootstrapDHTRequest {
        val transport_uris = mutableListOf<String>()
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> transport_uris.add(ProtoAdapter.STRING.decode(reader))
            else -> reader.readUnknownField(tag)
          }
        }
        return BootstrapDHTRequest(
          transport_uris = transport_uris,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: BootstrapDHTRequest): BootstrapDHTRequest = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
