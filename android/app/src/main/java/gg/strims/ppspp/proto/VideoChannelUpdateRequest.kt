// Code generated by Wire protocol buffer compiler, do not edit.
// Source: VideoChannelUpdateRequest in video.proto
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

class VideoChannelUpdateRequest(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#UINT64",
    label = WireField.Label.OMIT_IDENTITY
  )
  val id: Long = 0L,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.ppspp.proto.DirectoryListingSnippet#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "directoryListingSnippet"
  )
  val directory_listing_snippet: DirectoryListingSnippet? = null,
  @field:WireField(
    tag = 3,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "networkKey"
  )
  val network_key: ByteString = ByteString.EMPTY,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<VideoChannelUpdateRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is VideoChannelUpdateRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (id != other.id) return false
    if (directory_listing_snippet != other.directory_listing_snippet) return false
    if (network_key != other.network_key) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + id.hashCode()
      result = result * 37 + directory_listing_snippet.hashCode()
      result = result * 37 + network_key.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """id=$id"""
    if (directory_listing_snippet != null) result +=
        """directory_listing_snippet=$directory_listing_snippet"""
    result += """network_key=$network_key"""
    return result.joinToString(prefix = "VideoChannelUpdateRequest{", separator = ", ", postfix =
        "}")
  }

  fun copy(
    id: Long = this.id,
    directory_listing_snippet: DirectoryListingSnippet? = this.directory_listing_snippet,
    network_key: ByteString = this.network_key,
    unknownFields: ByteString = this.unknownFields
  ): VideoChannelUpdateRequest = VideoChannelUpdateRequest(id, directory_listing_snippet,
      network_key, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<VideoChannelUpdateRequest> = object :
        ProtoAdapter<VideoChannelUpdateRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      VideoChannelUpdateRequest::class, 
      "type.googleapis.com/VideoChannelUpdateRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: VideoChannelUpdateRequest): Int {
        var size = value.unknownFields.size
        if (value.id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1, value.id)
        if (value.directory_listing_snippet != null) size +=
            DirectoryListingSnippet.ADAPTER.encodedSizeWithTag(2, value.directory_listing_snippet)
        if (value.network_key != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(3,
            value.network_key)
        return size
      }

      override fun encode(writer: ProtoWriter, value: VideoChannelUpdateRequest) {
        if (value.id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.id)
        if (value.directory_listing_snippet != null)
            DirectoryListingSnippet.ADAPTER.encodeWithTag(writer, 2,
            value.directory_listing_snippet)
        if (value.network_key != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 3,
            value.network_key)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): VideoChannelUpdateRequest {
        var id: Long = 0L
        var directory_listing_snippet: DirectoryListingSnippet? = null
        var network_key: ByteString = ByteString.EMPTY
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> id = ProtoAdapter.UINT64.decode(reader)
            2 -> directory_listing_snippet = DirectoryListingSnippet.ADAPTER.decode(reader)
            3 -> network_key = ProtoAdapter.BYTES.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return VideoChannelUpdateRequest(
          id = id,
          directory_listing_snippet = directory_listing_snippet,
          network_key = network_key,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: VideoChannelUpdateRequest): VideoChannelUpdateRequest = value.copy(
        directory_listing_snippet =
            value.directory_listing_snippet?.let(DirectoryListingSnippet.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
