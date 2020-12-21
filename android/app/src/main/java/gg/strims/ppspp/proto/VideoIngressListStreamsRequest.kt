// Code generated by Wire protocol buffer compiler, do not edit.
// Source: VideoIngressListStreamsRequest in video.proto
package gg.strims.ppspp.proto

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import kotlin.Any
import kotlin.AssertionError
import kotlin.Boolean
import kotlin.Deprecated
import kotlin.DeprecationLevel
import kotlin.Int
import kotlin.Long
import kotlin.Nothing
import kotlin.String
import kotlin.jvm.JvmField
import okio.ByteString

class VideoIngressListStreamsRequest(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<VideoIngressListStreamsRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is VideoIngressListStreamsRequest) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "VideoIngressListStreamsRequest{}"

  fun copy(unknownFields: ByteString = this.unknownFields): VideoIngressListStreamsRequest =
      VideoIngressListStreamsRequest(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<VideoIngressListStreamsRequest> = object :
        ProtoAdapter<VideoIngressListStreamsRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      VideoIngressListStreamsRequest::class, 
      "type.googleapis.com/VideoIngressListStreamsRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: VideoIngressListStreamsRequest): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: VideoIngressListStreamsRequest) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): VideoIngressListStreamsRequest {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return VideoIngressListStreamsRequest(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: VideoIngressListStreamsRequest): VideoIngressListStreamsRequest =
          value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
