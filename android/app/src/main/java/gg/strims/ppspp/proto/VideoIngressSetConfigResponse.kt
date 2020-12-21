// Code generated by Wire protocol buffer compiler, do not edit.
// Source: VideoIngressSetConfigResponse in video.proto
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

class VideoIngressSetConfigResponse(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.VideoIngressConfig#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY
  )
  val config: VideoIngressConfig? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<VideoIngressSetConfigResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is VideoIngressSetConfigResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (config != other.config) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + config.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (config != null) result += """config=$config"""
    return result.joinToString(prefix = "VideoIngressSetConfigResponse{", separator = ", ", postfix
        = "}")
  }

  fun copy(config: VideoIngressConfig? = this.config, unknownFields: ByteString =
      this.unknownFields): VideoIngressSetConfigResponse = VideoIngressSetConfigResponse(config,
      unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<VideoIngressSetConfigResponse> = object :
        ProtoAdapter<VideoIngressSetConfigResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      VideoIngressSetConfigResponse::class, 
      "type.googleapis.com/VideoIngressSetConfigResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: VideoIngressSetConfigResponse): Int {
        var size = value.unknownFields.size
        if (value.config != null) size += VideoIngressConfig.ADAPTER.encodedSizeWithTag(1,
            value.config)
        return size
      }

      override fun encode(writer: ProtoWriter, value: VideoIngressSetConfigResponse) {
        if (value.config != null) VideoIngressConfig.ADAPTER.encodeWithTag(writer, 1, value.config)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): VideoIngressSetConfigResponse {
        var config: VideoIngressConfig? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> config = VideoIngressConfig.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return VideoIngressSetConfigResponse(
          config = config,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: VideoIngressSetConfigResponse): VideoIngressSetConfigResponse =
          value.copy(
        config = value.config?.let(VideoIngressConfig.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
