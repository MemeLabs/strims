// Code generated by Wire protocol buffer compiler, do not edit.
// Source: CAPeerRenewResponse in peer.proto
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

class CAPeerRenewResponse(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.Certificate#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY
  )
  val certificate: Certificate? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<CAPeerRenewResponse, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is CAPeerRenewResponse) return false
    if (unknownFields != other.unknownFields) return false
    if (certificate != other.certificate) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + certificate.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (certificate != null) result += """certificate=$certificate"""
    return result.joinToString(prefix = "CAPeerRenewResponse{", separator = ", ", postfix = "}")
  }

  fun copy(certificate: Certificate? = this.certificate, unknownFields: ByteString =
      this.unknownFields): CAPeerRenewResponse = CAPeerRenewResponse(certificate, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<CAPeerRenewResponse> = object : ProtoAdapter<CAPeerRenewResponse>(
      FieldEncoding.LENGTH_DELIMITED, 
      CAPeerRenewResponse::class, 
      "type.googleapis.com/CAPeerRenewResponse", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: CAPeerRenewResponse): Int {
        var size = value.unknownFields.size
        if (value.certificate != null) size += Certificate.ADAPTER.encodedSizeWithTag(1,
            value.certificate)
        return size
      }

      override fun encode(writer: ProtoWriter, value: CAPeerRenewResponse) {
        if (value.certificate != null) Certificate.ADAPTER.encodeWithTag(writer, 1,
            value.certificate)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): CAPeerRenewResponse {
        var certificate: Certificate? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> certificate = Certificate.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return CAPeerRenewResponse(
          certificate = certificate,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: CAPeerRenewResponse): CAPeerRenewResponse = value.copy(
        certificate = value.certificate?.let(Certificate.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}