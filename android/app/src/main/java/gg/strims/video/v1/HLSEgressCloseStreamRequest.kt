// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.video.v1.HLSEgressCloseStreamRequest in video/v1/hls_egress.proto
package gg.strims.video.v1

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

class HLSEgressCloseStreamRequest(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "transferId"
  )
  val transfer_id: ByteString = ByteString.EMPTY,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<HLSEgressCloseStreamRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is HLSEgressCloseStreamRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (transfer_id != other.transfer_id) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + transfer_id.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """transfer_id=$transfer_id"""
    return result.joinToString(prefix = "HLSEgressCloseStreamRequest{", separator = ", ", postfix =
        "}")
  }

  fun copy(transfer_id: ByteString = this.transfer_id, unknownFields: ByteString =
      this.unknownFields): HLSEgressCloseStreamRequest = HLSEgressCloseStreamRequest(transfer_id,
      unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<HLSEgressCloseStreamRequest> = object :
        ProtoAdapter<HLSEgressCloseStreamRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      HLSEgressCloseStreamRequest::class, 
      "type.googleapis.com/strims.video.v1.HLSEgressCloseStreamRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: HLSEgressCloseStreamRequest): Int {
        var size = value.unknownFields.size
        if (value.transfer_id != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
            value.transfer_id)
        return size
      }

      override fun encode(writer: ProtoWriter, value: HLSEgressCloseStreamRequest) {
        if (value.transfer_id != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1,
            value.transfer_id)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): HLSEgressCloseStreamRequest {
        var transfer_id: ByteString = ByteString.EMPTY
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> transfer_id = ProtoAdapter.BYTES.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return HLSEgressCloseStreamRequest(
          transfer_id = transfer_id,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: HLSEgressCloseStreamRequest): HLSEgressCloseStreamRequest =
          value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
