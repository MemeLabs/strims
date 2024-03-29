// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.network.v1.DirectoryListingSnippet in network/v1/directory.proto
package gg.strims.network.v1

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import com.squareup.wire.internal.immutableCopyOf
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
import kotlin.collections.List
import kotlin.hashCode
import kotlin.jvm.JvmField
import okio.ByteString

class DirectoryListingSnippet(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY
  )
  val title: String = "",
  @field:WireField(
    tag = 2,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY
  )
  val description: String = "",
  tags: List<String> = emptyList(),
  unknownFields: ByteString = ByteString.EMPTY
) : Message<DirectoryListingSnippet, Nothing>(ADAPTER, unknownFields) {
  @field:WireField(
    tag = 3,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.REPEATED
  )
  val tags: List<String> = immutableCopyOf("tags", tags)

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is DirectoryListingSnippet) return false
    if (unknownFields != other.unknownFields) return false
    if (title != other.title) return false
    if (description != other.description) return false
    if (tags != other.tags) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + title.hashCode()
      result = result * 37 + description.hashCode()
      result = result * 37 + tags.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """title=${sanitize(title)}"""
    result += """description=${sanitize(description)}"""
    if (tags.isNotEmpty()) result += """tags=${sanitize(tags)}"""
    return result.joinToString(prefix = "DirectoryListingSnippet{", separator = ", ", postfix = "}")
  }

  fun copy(
    title: String = this.title,
    description: String = this.description,
    tags: List<String> = this.tags,
    unknownFields: ByteString = this.unknownFields
  ): DirectoryListingSnippet = DirectoryListingSnippet(title, description, tags, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<DirectoryListingSnippet> = object :
        ProtoAdapter<DirectoryListingSnippet>(
      FieldEncoding.LENGTH_DELIMITED, 
      DirectoryListingSnippet::class, 
      "type.googleapis.com/strims.network.v1.DirectoryListingSnippet", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: DirectoryListingSnippet): Int {
        var size = value.unknownFields.size
        if (value.title != "") size += ProtoAdapter.STRING.encodedSizeWithTag(1, value.title)
        if (value.description != "") size += ProtoAdapter.STRING.encodedSizeWithTag(2,
            value.description)
        size += ProtoAdapter.STRING.asRepeated().encodedSizeWithTag(3, value.tags)
        return size
      }

      override fun encode(writer: ProtoWriter, value: DirectoryListingSnippet) {
        if (value.title != "") ProtoAdapter.STRING.encodeWithTag(writer, 1, value.title)
        if (value.description != "") ProtoAdapter.STRING.encodeWithTag(writer, 2, value.description)
        ProtoAdapter.STRING.asRepeated().encodeWithTag(writer, 3, value.tags)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): DirectoryListingSnippet {
        var title: String = ""
        var description: String = ""
        val tags = mutableListOf<String>()
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> title = ProtoAdapter.STRING.decode(reader)
            2 -> description = ProtoAdapter.STRING.decode(reader)
            3 -> tags.add(ProtoAdapter.STRING.decode(reader))
            else -> reader.readUnknownField(tag)
          }
        }
        return DirectoryListingSnippet(
          title = title,
          description = description,
          tags = tags,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: DirectoryListingSnippet): DirectoryListingSnippet = value.copy(
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
