// Code generated by Wire protocol buffer compiler, do not edit.
// Source: CreateNetworkInvitationRequest in profile.proto
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

class CreateNetworkInvitationRequest(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.Key#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "signingKey"
  )
  val signing_key: Key? = null,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.ppspp.proto.Certificate#ADAPTER",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "signingCert"
  )
  val signing_cert: Certificate? = null,
  @field:WireField(
    tag = 3,
    adapter = "com.squareup.wire.ProtoAdapter#STRING",
    label = WireField.Label.OMIT_IDENTITY,
    jsonName = "networkName"
  )
  val network_name: String = "",
  unknownFields: ByteString = ByteString.EMPTY
) : Message<CreateNetworkInvitationRequest, Nothing>(ADAPTER, unknownFields) {
  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is CreateNetworkInvitationRequest) return false
    if (unknownFields != other.unknownFields) return false
    if (signing_key != other.signing_key) return false
    if (signing_cert != other.signing_cert) return false
    if (network_name != other.network_name) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + signing_key.hashCode()
      result = result * 37 + signing_cert.hashCode()
      result = result * 37 + network_name.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (signing_key != null) result += """signing_key=$signing_key"""
    if (signing_cert != null) result += """signing_cert=$signing_cert"""
    result += """network_name=${sanitize(network_name)}"""
    return result.joinToString(prefix = "CreateNetworkInvitationRequest{", separator = ", ", postfix
        = "}")
  }

  fun copy(
    signing_key: Key? = this.signing_key,
    signing_cert: Certificate? = this.signing_cert,
    network_name: String = this.network_name,
    unknownFields: ByteString = this.unknownFields
  ): CreateNetworkInvitationRequest = CreateNetworkInvitationRequest(signing_key, signing_cert,
      network_name, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<CreateNetworkInvitationRequest> = object :
        ProtoAdapter<CreateNetworkInvitationRequest>(
      FieldEncoding.LENGTH_DELIMITED, 
      CreateNetworkInvitationRequest::class, 
      "type.googleapis.com/CreateNetworkInvitationRequest", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: CreateNetworkInvitationRequest): Int {
        var size = value.unknownFields.size
        if (value.signing_key != null) size += Key.ADAPTER.encodedSizeWithTag(1, value.signing_key)
        if (value.signing_cert != null) size += Certificate.ADAPTER.encodedSizeWithTag(2,
            value.signing_cert)
        if (value.network_name != "") size += ProtoAdapter.STRING.encodedSizeWithTag(3,
            value.network_name)
        return size
      }

      override fun encode(writer: ProtoWriter, value: CreateNetworkInvitationRequest) {
        if (value.signing_key != null) Key.ADAPTER.encodeWithTag(writer, 1, value.signing_key)
        if (value.signing_cert != null) Certificate.ADAPTER.encodeWithTag(writer, 2,
            value.signing_cert)
        if (value.network_name != "") ProtoAdapter.STRING.encodeWithTag(writer, 3,
            value.network_name)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): CreateNetworkInvitationRequest {
        var signing_key: Key? = null
        var signing_cert: Certificate? = null
        var network_name: String = ""
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> signing_key = Key.ADAPTER.decode(reader)
            2 -> signing_cert = Certificate.ADAPTER.decode(reader)
            3 -> network_name = ProtoAdapter.STRING.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return CreateNetworkInvitationRequest(
          signing_key = signing_key,
          signing_cert = signing_cert,
          network_name = network_name,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: CreateNetworkInvitationRequest): CreateNetworkInvitationRequest =
          value.copy(
        signing_key = value.signing_key?.let(Key.ADAPTER::redact),
        signing_cert = value.signing_cert?.let(Certificate.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }
}
