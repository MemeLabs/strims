// Code generated by Wire protocol buffer compiler, do not edit.
// Source: VideoIngressShareDeleteChannelResponse in video.proto
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

class VideoIngressShareDeleteChannelResponse(
  unknownFields: ByteString = ByteString.EMPTY
) : Message<VideoIngressShareDeleteChannelResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is VideoIngressShareDeleteChannelResponse) return false
    if (unknownFields != other.unknownFields) return false
    return true
  }

  override fun hashCode(): Int = unknownFields.hashCode()

  override fun toString(): String = "VideoIngressShareDeleteChannelResponse{}"

  fun copy(unknownFields: ByteString = this.unknownFields): VideoIngressShareDeleteChannelResponse =
      VideoIngressShareDeleteChannelResponse(unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<VideoIngressShareDeleteChannelResponse> = object :
        ProtoAdapter<VideoIngressShareDeleteChannelResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      VideoIngressShareDeleteChannelResponse::class, 
      "type.googleapis.com/VideoIngressShareDeleteChannelResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: VideoIngressShareDeleteChannelResponse): Int {
        var size = value.unknownFields.size
        return size
      }

      override fun encode(writer: ProtoWriter, value: VideoIngressShareDeleteChannelResponse) {
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): VideoIngressShareDeleteChannelResponse {
        val unknownFields = reader.forEachTag(reader::readUnknownField)
        return VideoIngressShareDeleteChannelResponse(
          unknownFields = unknownFields
        )
      }

      override fun redact(value: VideoIngressShareDeleteChannelResponse):
          VideoIngressShareDeleteChannelResponse = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
