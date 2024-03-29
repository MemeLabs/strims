// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.chat.v1.UpdateChatServerRequest in chat/v1/chat.proto
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

class UpdateChatServerRequest(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#UINT64",
    label = WireField.Label.OMIT_IDENTITY
  )
  val id: Long = 0L,
  @field:WireField(
    tag = 2,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "networkKey"
  )
  val network_key: ByteString = ByteString.EMPTY,
  @field:WireField(
    tag = 3,
    adapter = "gg.strims.ppspp.proto.ChatRoom#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "serverKey"
  )
  val server_key: ChatRoom? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<UpdateChatServerRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is UpdateChatServerRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (id != other.id) return false
    if (network_key != other.network_key) return false
    if (server_key != other.server_key) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + id.hashCode()
      result = result * 37 + network_key.hashCode()
      result = result * 37 + server_key.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """id=$id"""
    result += """network_key=$network_key"""
    if (server_key != null) result += """server_key=$server_key"""
    return result.joinToString(prefix = "UpdateChatServerRequest{", separator = ", ", postfix = "}")
  }

  fun copy(
    id: Long = this.id,
    network_key: ByteString = this.network_key,
    server_key: ChatRoom? = this.server_key,
    unknownFields: ByteString = this.unknownFields
  ): UpdateChatServerRequest = UpdateChatServerRequest(id, network_key, server_key, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<UpdateChatServerRequest> = object :
        ProtoAdapter<UpdateChatServerRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      UpdateChatServerRequest::class, 
      "type.googleapis.com/strims.chat.v1.UpdateChatServerRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: UpdateChatServerRequest): Int {
        var size = value.unknownFields.size
        if (value.id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1, value.id)
        if (value.network_key != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(2,
            value.network_key)
        if (value.server_key != null) size += ChatRoom.ADAPTER.encodedSizeWithTag(3,
            value.server_key)
        return size
      }

      override fun encode(writer: ProtoWriter, value: UpdateChatServerRequest) {
        if (value.id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.id)
        if (value.network_key != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 2,
            value.network_key)
        if (value.server_key != null) ChatRoom.ADAPTER.encodeWithTag(writer, 3, value.server_key)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): UpdateChatServerRequest {
        var id: Long = 0L
        var network_key: ByteString = ByteString.EMPTY
        var server_key: ChatRoom? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> id = ProtoAdapter.UINT64.decode(reader)
            2 -> network_key = ProtoAdapter.BYTES.decode(reader)
            3 -> server_key = ChatRoom.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return UpdateChatServerRequest(
          id = id,
          network_key = network_key,
          server_key = server_key,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: UpdateChatServerRequest): UpdateChatServerRequest = value.copy(
        server_key = value.server_key?.let(ChatRoom.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
