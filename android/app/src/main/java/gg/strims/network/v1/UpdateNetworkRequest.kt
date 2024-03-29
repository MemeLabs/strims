// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.network.v1.UpdateNetworkRequest in network/v1/network.proto
package gg.strims.network.v1

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

class UpdateNetworkRequest(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#UINT64",
    label = WireField.Label.OMIT_IDENTITY
  )
  val id: Long = 0L,
  @field:WireField(
    tag = 2,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY
  )
  val name: String = "",
  unknownFields: ByteString = ByteString.EMPTY
) : Message<UpdateNetworkRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is UpdateNetworkRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (id != other.id) return false
    if (name != other.name) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + id.hashCode()
      result = result * 37 + name.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """id=$id"""
    result += """name=${sanitize(name)}"""
    return result.joinToString(prefix = "UpdateNetworkRequest{", separator = ", ", postfix = "}")
  }

  fun copy(
    id: Long = this.id,
    name: String = this.name,
    unknownFields: ByteString = this.unknownFields
  ): UpdateNetworkRequest = UpdateNetworkRequest(id, name, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<UpdateNetworkRequest> = object : ProtoAdapter<UpdateNetworkRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      UpdateNetworkRequest::class, 
      "type.googleapis.com/strims.network.v1.UpdateNetworkRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: UpdateNetworkRequest): Int {
        var size = value.unknownFields.size
        if (value.id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1, value.id)
        if (value.name != "") size += ProtoAdapter.STRING.encodedSizeWithTag(2, value.name)
        return size
      }

      override fun encode(writer: ProtoWriter, value: UpdateNetworkRequest) {
        if (value.id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.id)
        if (value.name != "") ProtoAdapter.STRING.encodeWithTag(writer, 2, value.name)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): UpdateNetworkRequest {
        var id: Long = 0L
        var name: String = ""
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> id = ProtoAdapter.UINT64.decode(reader)
            2 -> name = ProtoAdapter.STRING.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return UpdateNetworkRequest(
          id = id,
          name = name,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: UpdateNetworkRequest): UpdateNetworkRequest = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
