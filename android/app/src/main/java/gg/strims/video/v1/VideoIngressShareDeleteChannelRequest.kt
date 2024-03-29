// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.video.v1.VideoIngressShareDeleteChannelRequest in video/v1/ingress.proto
package gg.strims.video.v1

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

class VideoIngressShareDeleteChannelRequest(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<VideoIngressShareDeleteChannelRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is VideoIngressShareDeleteChannelRequest) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "VideoIngressShareDeleteChannelRequest{}"

  fun copy(unknownFields: ByteString = this.unknownFields): VideoIngressShareDeleteChannelRequest =
      VideoIngressShareDeleteChannelRequest(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<VideoIngressShareDeleteChannelRequest> = object :
        ProtoAdapter<VideoIngressShareDeleteChannelRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      VideoIngressShareDeleteChannelRequest::class, 
      "type.googleapis.com/strims.video.v1.VideoIngressShareDeleteChannelRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: VideoIngressShareDeleteChannelRequest): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: VideoIngressShareDeleteChannelRequest) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): VideoIngressShareDeleteChannelRequest {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return VideoIngressShareDeleteChannelRequest(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: VideoIngressShareDeleteChannelRequest):
          VideoIngressShareDeleteChannelRequest = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
