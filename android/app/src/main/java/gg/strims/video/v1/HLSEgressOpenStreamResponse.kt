// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.video.v1.HLSEgressOpenStreamResponse in video/v1/hls_egress.proto
package gg.strims.video.v1

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

class HLSEgressOpenStreamResponse(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "playlistUrl"
  )
  val playlist_url: String = "",
  unknownFields: ByteString = ByteString.EMPTY
) : Message<HLSEgressOpenStreamResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is HLSEgressOpenStreamResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (playlist_url != other.playlist_url) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + playlist_url.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """playlist_url=${sanitize(playlist_url)}"""
    return result.joinToString(prefix = "HLSEgressOpenStreamResponse{", separator = ", ", postfix =
        "}")
  }

  fun copy(playlist_url: String = this.playlist_url, unknownFields: ByteString =
      this.unknownFields): HLSEgressOpenStreamResponse = HLSEgressOpenStreamResponse(playlist_url,
      unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<HLSEgressOpenStreamResponse> = object :
        ProtoAdapter<HLSEgressOpenStreamResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      HLSEgressOpenStreamResponse::class, 
      "type.googleapis.com/strims.video.v1.HLSEgressOpenStreamResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: HLSEgressOpenStreamResponse): Int {
        var size = value.unknownFields.size
        if (value.playlist_url != "") size += ProtoAdapter.STRING.encodedSizeWithTag(1,
            value.playlist_url)
        return size
      }

      override fun encode(writer: ProtoWriter, value: HLSEgressOpenStreamResponse) {
        if (value.playlist_url != "") ProtoAdapter.STRING.encodeWithTag(writer, 1,
            value.playlist_url)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): HLSEgressOpenStreamResponse {
        var playlist_url: String = ""
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> playlist_url = ProtoAdapter.STRING.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return HLSEgressOpenStreamResponse(
          playlist_url = playlist_url,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: HLSEgressOpenStreamResponse): HLSEgressOpenStreamResponse =
          value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}