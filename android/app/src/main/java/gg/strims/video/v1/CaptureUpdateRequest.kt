// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.video.v1.CaptureUpdateRequest in video/v1/capture.proto
package gg.strims.video.v1

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import gg.strims.network.v1.DirectoryListingSnippet
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

class CaptureUpdateRequest(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY
  )
  val id: ByteString = ByteString.EMPTY,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.network.v1.DirectoryListingSnippet#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "directorySnippet"
  )
  val directory_snippet: DirectoryListingSnippet? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<CaptureUpdateRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is CaptureUpdateRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (id != other.id) return false
    if (directory_snippet != other.directory_snippet) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + id.hashCode()
      result = result * 37 + directory_snippet.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """id=$id"""
    if (directory_snippet != null) result += """directory_snippet=$directory_snippet"""
    return result.joinToString(prefix = "CaptureUpdateRequest{", separator = ", ", postfix = "}")
  }

  fun copy(
    id: ByteString = this.id,
    directory_snippet: DirectoryListingSnippet? = this.directory_snippet,
    unknownFields: ByteString = this.unknownFields
  ): CaptureUpdateRequest = CaptureUpdateRequest(id, directory_snippet, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<CaptureUpdateRequest> = object : ProtoAdapter<CaptureUpdateRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      CaptureUpdateRequest::class, 
      "type.googleapis.com/strims.video.v1.CaptureUpdateRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: CaptureUpdateRequest): Int {
        var size = value.unknownFields.size
        if (value.id != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1, value.id)
        if (value.directory_snippet != null) size +=
            DirectoryListingSnippet.ADAPTER.encodedSizeWithTag(2, value.directory_snippet)
        return size
      }

      override fun encode(writer: ProtoWriter, value: CaptureUpdateRequest) {
        if (value.id != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1, value.id)
        if (value.directory_snippet != null) DirectoryListingSnippet.ADAPTER.encodeWithTag(writer,
            2, value.directory_snippet)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): CaptureUpdateRequest {
        var id: ByteString = ByteString.EMPTY
        var directory_snippet: DirectoryListingSnippet? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> id = ProtoAdapter.BYTES.decode(reader)
            2 -> directory_snippet = DirectoryListingSnippet.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return CaptureUpdateRequest(
          id = id,
          directory_snippet = directory_snippet,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: CaptureUpdateRequest): CaptureUpdateRequest = value.copy(
        directory_snippet = value.directory_snippet?.let(DirectoryListingSnippet.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
