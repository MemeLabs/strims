// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.network.v1.bootstrap.UpdateBootstrapClientResponse in network/v1/bootstrap/bootstrap.proto
package gg.strims.network.v1.bootstrap

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

class UpdateBootstrapClientResponse(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.network.v1.bootstrap.BootstrapClient#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "bootstrapClient"
  )
  val bootstrap_client: BootstrapClient? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<UpdateBootstrapClientResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is UpdateBootstrapClientResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (bootstrap_client != other.bootstrap_client) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + bootstrap_client.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (bootstrap_client != null) result += """bootstrap_client=$bootstrap_client"""
    return result.joinToString(prefix = "UpdateBootstrapClientResponse{", separator = ", ", postfix
        = "}")
  }

  fun copy(bootstrap_client: BootstrapClient? = this.bootstrap_client, unknownFields: ByteString =
      this.unknownFields): UpdateBootstrapClientResponse =
      UpdateBootstrapClientResponse(bootstrap_client, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<UpdateBootstrapClientResponse> = object :
        ProtoAdapter<UpdateBootstrapClientResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      UpdateBootstrapClientResponse::class, 
      "type.googleapis.com/strims.network.v1.bootstrap.UpdateBootstrapClientResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: UpdateBootstrapClientResponse): Int {
        var size = value.unknownFields.size
        if (value.bootstrap_client != null) size += BootstrapClient.ADAPTER.encodedSizeWithTag(1,
            value.bootstrap_client)
        return size
      }

      override fun encode(writer: ProtoWriter, value: UpdateBootstrapClientResponse) {
        if (value.bootstrap_client != null) BootstrapClient.ADAPTER.encodeWithTag(writer, 1,
            value.bootstrap_client)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): UpdateBootstrapClientResponse {
        var bootstrap_client: BootstrapClient? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> bootstrap_client = BootstrapClient.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return UpdateBootstrapClientResponse(
          bootstrap_client = bootstrap_client,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: UpdateBootstrapClientResponse): UpdateBootstrapClientResponse =
          value.copy(
        bootstrap_client = value.bootstrap_client?.let(BootstrapClient.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
