// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.video.v1.VideoIngressSetConfigRequest in video/v1/ingress.proto
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

class VideoIngressSetConfigRequest(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.video.v1.VideoIngressConfig#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY
  )
  val config: VideoIngressConfig? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<VideoIngressSetConfigRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is VideoIngressSetConfigRequest) return false
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
    return result.joinToString(prefix = "VideoIngressSetConfigRequest{", separator = ", ", postfix =
        "}")
  }

  fun copy(config: VideoIngressConfig? = this.config, unknownFields: ByteString =
      this.unknownFields): VideoIngressSetConfigRequest = VideoIngressSetConfigRequest(config,
      unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<VideoIngressSetConfigRequest> = object :
        ProtoAdapter<VideoIngressSetConfigRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      VideoIngressSetConfigRequest::class, 
      "type.googleapis.com/strims.video.v1.VideoIngressSetConfigRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: VideoIngressSetConfigRequest): Int {
        var size = value.unknownFields.size
        if (value.config != null) size += VideoIngressConfig.ADAPTER.encodedSizeWithTag(1,
            value.config)
        return size
      }

      override fun encode(writer: ProtoWriter, value: VideoIngressSetConfigRequest) {
        if (value.config != null) VideoIngressConfig.ADAPTER.encodeWithTag(writer, 1, value.config)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): VideoIngressSetConfigRequest {
        var config: VideoIngressConfig? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> config = VideoIngressConfig.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return VideoIngressSetConfigRequest(
          config = config,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: VideoIngressSetConfigRequest): VideoIngressSetConfigRequest =
          value.copy(
        config = value.config?.let(VideoIngressConfig.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
