// Code generated by Wire protocol buffer compiler, do not edit.
// Source: JoinSwarmRequest in video.proto
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

class JoinSwarmRequest(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "swarmUri"
  )
  val swarm_uri: String = "",
  unknownFields: ByteString = ByteString.EMPTY
) : Message<JoinSwarmRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is JoinSwarmRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (swarm_uri != other.swarm_uri) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + swarm_uri.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """swarm_uri=${sanitize(swarm_uri)}"""
    return result.joinToString(prefix = "JoinSwarmRequest{", separator = ", ", postfix = "}")
  }

  fun copy(swarm_uri: String = this.swarm_uri, unknownFields: ByteString = this.unknownFields):
      JoinSwarmRequest = JoinSwarmRequest(swarm_uri, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<JoinSwarmRequest> = object : ProtoAdapter<JoinSwarmRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      JoinSwarmRequest::class, 
      "type.googleapis.com/JoinSwarmRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: JoinSwarmRequest): Int {
        var size = value.unknownFields.size
        if (value.swarm_uri != "") size += ProtoAdapter.STRING.encodedSizeWithTag(1,
            value.swarm_uri)
        return size
      }

      override fun encode(writer: ProtoWriter, value: JoinSwarmRequest) {
        if (value.swarm_uri != "") ProtoAdapter.STRING.encodeWithTag(writer, 1, value.swarm_uri)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): JoinSwarmRequest {
        var swarm_uri: String = ""
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> swarm_uri = ProtoAdapter.STRING.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return JoinSwarmRequest(
          swarm_uri = swarm_uri,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: JoinSwarmRequest): JoinSwarmRequest = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
