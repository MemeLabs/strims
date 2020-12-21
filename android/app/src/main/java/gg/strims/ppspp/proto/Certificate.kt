// Code generated by Wire protocol buffer compiler, do not edit.
// Source: Certificate in profile.proto
package gg.strims.ppspp.proto

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

class Certificate(
  @field:WireField(
    tag = 1,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY
  )
  val key: ByteString = ByteString.EMPTY,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.ppspp.proto.KeyType#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "keyType"
  )
  val key_type: KeyType = KeyType.KEY_TYPE_UNDEFINED,
  @field:WireField(
    tag = 3,
    adapter = "com.squareup.wire.ProtoAdapter#UINT32",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "keyUsage"
  )
  val key_usage: Int = 0,
  @field:WireField(
    tag = 4,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY
  )
  val subject: String = "",
  @field:WireField(
    tag = 5,
    adapter = "com.squareup.wire.ProtoAdapter#UINT64",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "notBefore"
  )
  val not_before: Long = 0L,
  @field:WireField(
    tag = 6,
    adapter = "com.squareup.wire.ProtoAdapter#UINT64",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "notAfter"
  )
  val not_after: Long = 0L,
  @field:WireField(
    tag = 7,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "serialNumber"
  )
  val serial_number: ByteString = ByteString.EMPTY,
  @field:WireField(
    tag = 8,
    adapter = "com.squareup.wire.ProtoAdapter#BYTES",
    label = WireField.Label.OMIT_IDENTITY
  )
  val signature: ByteString = ByteString.EMPTY,
  @field:WireField(
    tag = 9,
    adapter = "gg.strims.ppspp.proto.Certificate#ADAPTER"
  )
  val parent: Certificate? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<Certificate, Nothing>(ADAPTER, unknownFields) {
  init {
  }

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is Certificate) return false
    if (unknownFields != other.unknownFields) return false
    if (key != other.key) return false
    if (key_type != other.key_type) return false
    if (key_usage != other.key_usage) return false
    if (subject != other.subject) return false
    if (not_before != other.not_before) return false
    if (not_after != other.not_after) return false
    if (serial_number != other.serial_number) return false
    if (signature != other.signature) return false
    if (parent != other.parent) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + key.hashCode()
      result = result * 37 + key_type.hashCode()
      result = result * 37 + key_usage.hashCode()
      result = result * 37 + subject.hashCode()
      result = result * 37 + not_before.hashCode()
      result = result * 37 + not_after.hashCode()
      result = result * 37 + serial_number.hashCode()
      result = result * 37 + signature.hashCode()
      result = result * 37 + parent.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    result += """key=$key"""
    result += """key_type=$key_type"""
    result += """key_usage=$key_usage"""
    result += """subject=${sanitize(subject)}"""
    result += """not_before=$not_before"""
    result += """not_after=$not_after"""
    result += """serial_number=$serial_number"""
    result += """signature=$signature"""
    if (parent != null) result += """parent=$parent"""
    return result.joinToString(prefix = "Certificate{", separator = ", ", postfix = "}")
  }

  fun copy(
    key: ByteString = this.key,
    key_type: KeyType = this.key_type,
    key_usage: Int = this.key_usage,
    subject: String = this.subject,
    not_before: Long = this.not_before,
    not_after: Long = this.not_after,
    serial_number: ByteString = this.serial_number,
    signature: ByteString = this.signature,
    parent: Certificate? = this.parent,
    unknownFields: ByteString = this.unknownFields
  ): Certificate = Certificate(key, key_type, key_usage, subject, not_before, not_after,
      serial_number, signature, parent, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<Certificate> = object : ProtoAdapter<Certificate>(
      FieldEncoding.LENGTH_DELIMITED, 
      Certificate::class, 
      "type.googleapis.com/Certificate", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: Certificate): Int {
        var size = value.unknownFields.size
        if (value.key != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
            value.key)
        if (value.key_type != KeyType.KEY_TYPE_UNDEFINED) size +=
            KeyType.ADAPTER.encodedSizeWithTag(2, value.key_type)
        if (value.key_usage != 0) size += ProtoAdapter.UINT32.encodedSizeWithTag(3, value.key_usage)
        if (value.subject != "") size += ProtoAdapter.STRING.encodedSizeWithTag(4, value.subject)
        if (value.not_before != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(5,
            value.not_before)
        if (value.not_after != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(6,
            value.not_after)
        if (value.serial_number != ByteString.EMPTY) size +=
            ProtoAdapter.BYTES.encodedSizeWithTag(7, value.serial_number)
        if (value.signature != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(8,
            value.signature)
        size += Certificate.ADAPTER.encodedSizeWithTag(9, value.parent)
        return size
      }

      override fun encode(writer: ProtoWriter, value: Certificate) {
        if (value.key != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1, value.key)
        if (value.key_type != KeyType.KEY_TYPE_UNDEFINED) KeyType.ADAPTER.encodeWithTag(writer, 2,
            value.key_type)
        if (value.key_usage != 0) ProtoAdapter.UINT32.encodeWithTag(writer, 3, value.key_usage)
        if (value.subject != "") ProtoAdapter.STRING.encodeWithTag(writer, 4, value.subject)
        if (value.not_before != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 5, value.not_before)
        if (value.not_after != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 6, value.not_after)
        if (value.serial_number != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 7,
            value.serial_number)
        if (value.signature != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 8,
            value.signature)
        Certificate.ADAPTER.encodeWithTag(writer, 9, value.parent)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): Certificate {
        var key: ByteString = ByteString.EMPTY
        var key_type: KeyType = KeyType.KEY_TYPE_UNDEFINED
        var key_usage: Int = 0
        var subject: String = ""
        var not_before: Long = 0L
        var not_after: Long = 0L
        var serial_number: ByteString = ByteString.EMPTY
        var signature: ByteString = ByteString.EMPTY
        var parent: Certificate? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> key = ProtoAdapter.BYTES.decode(reader)
            2 -> try {
              key_type = KeyType.ADAPTER.decode(reader)
            } catch (e: ProtoAdapter.EnumConstantNotFoundException) {
              reader.addUnknownField(tag, FieldEncoding.VARINT, e.value.toLong())
            }
            3 -> key_usage = ProtoAdapter.UINT32.decode(reader)
            4 -> subject = ProtoAdapter.STRING.decode(reader)
            5 -> not_before = ProtoAdapter.UINT64.decode(reader)
            6 -> not_after = ProtoAdapter.UINT64.decode(reader)
            7 -> serial_number = ProtoAdapter.BYTES.decode(reader)
            8 -> signature = ProtoAdapter.BYTES.decode(reader)
            9 -> parent = Certificate.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return Certificate(
          key = key,
          key_type = key_type,
          key_usage = key_usage,
          subject = subject,
          not_before = not_before,
          not_after = not_after,
          serial_number = serial_number,
          signature = signature,
          parent = parent,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: Certificate): Certificate = value.copy(
        parent = value.parent?.let(Certificate.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
