// Code generated by Wire protocol buffer compiler, do not edit.
// Source: NetworkEvent in vpn.proto
package gg.strims.ppspp.proto

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import com.squareup.wire.internal.countNonNull
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

class NetworkEvent(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.NetworkEvent${'$'}NetworkOpen#ADAPTER",
    jsonName = "networkOpen"
  )
  val network_open: NetworkOpen? = null,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.ppspp.proto.NetworkEvent${'$'}NetworkClose#ADAPTER",
    jsonName = "networkClose"
  )
  val network_close: NetworkClose? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<NetworkEvent, Nothing>(ADAPTER, unknownFields) {
  init {
    require(countNonNull(network_open, network_close) <= 1) {
      "At most one of network_open, network_close may be non-null"
    }
  }

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is NetworkEvent) return false
    if (unknownFields != other.unknownFields) return false
    if (network_open != other.network_open) return false
    if (network_close != other.network_close) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + network_open.hashCode()
      result = result * 37 + network_close.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (network_open != null) result += """network_open=$network_open"""
    if (network_close != null) result += """network_close=$network_close"""
    return result.joinToString(prefix = "NetworkEvent{", separator = ", ", postfix = "}")
  }

  fun copy(
    network_open: NetworkOpen? = this.network_open,
    network_close: NetworkClose? = this.network_close,
    unknownFields: ByteString = this.unknownFields
  ): NetworkEvent = NetworkEvent(network_open, network_close, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<NetworkEvent> = object : ProtoAdapter<NetworkEvent>(
      FieldEncoding.LENGTH_DELIMITED, 
      NetworkEvent::class, 
      "type.googleapis.com/NetworkEvent", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: NetworkEvent): Int {
        var size = value.unknownFields.size
        size += NetworkOpen.ADAPTER.encodedSizeWithTag(1, value.network_open)
        size += NetworkClose.ADAPTER.encodedSizeWithTag(2, value.network_close)
        return size
      }

      override fun encode(writer: ProtoWriter, value: NetworkEvent) {
        NetworkOpen.ADAPTER.encodeWithTag(writer, 1, value.network_open)
        NetworkClose.ADAPTER.encodeWithTag(writer, 2, value.network_close)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): NetworkEvent {
        var network_open: NetworkOpen? = null
        var network_close: NetworkClose? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> network_open = NetworkOpen.ADAPTER.decode(reader)
            2 -> network_close = NetworkClose.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return NetworkEvent(
          network_open = network_open,
          network_close = network_close,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: NetworkEvent): NetworkEvent = value.copy(
        network_open = value.network_open?.let(NetworkOpen.ADAPTER::redact),
        network_close = value.network_close?.let(NetworkClose.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }

  class NetworkOpen(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#UINT64",
      label = WireField.Label.OMIT_IDENTITY,
      jsonName = "networkId"
    )
    val network_id: Long = 0L,
    @field:WireField(
      tag = 2,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY,
      jsonName = "networkKey"
    )
    val network_key: ByteString = ByteString.EMPTY,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<NetworkOpen, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is NetworkOpen) return false
      if (unknownFields != other.unknownFields) return false
      if (network_id != other.network_id) return false
      if (network_key != other.network_key) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + network_id.hashCode()
        result = result * 37 + network_key.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """network_id=$network_id"""
      result += """network_key=$network_key"""
      return result.joinToString(prefix = "NetworkOpen{", separator = ", ", postfix = "}")
    }

    fun copy(
      network_id: Long = this.network_id,
      network_key: ByteString = this.network_key,
      unknownFields: ByteString = this.unknownFields
    ): NetworkOpen = NetworkOpen(network_id, network_key, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<NetworkOpen> = object : ProtoAdapter<NetworkOpen>(
        FieldEncoding.LENGTH_DELIMITED, 
        NetworkOpen::class, 
        "type.googleapis.com/NetworkEvent.NetworkOpen", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: NetworkOpen): Int {
          var size = value.unknownFields.size
          if (value.network_id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1,
              value.network_id)
          if (value.network_key != ByteString.EMPTY) size +=
              ProtoAdapter.BYTES.encodedSizeWithTag(2, value.network_key)
          return size
        }

        override fun encode(writer: ProtoWriter, value: NetworkOpen) {
          if (value.network_id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.network_id)
          if (value.network_key != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 2,
              value.network_key)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): NetworkOpen {
          var network_id: Long = 0L
          var network_key: ByteString = ByteString.EMPTY
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> network_id = ProtoAdapter.UINT64.decode(reader)
              2 -> network_key = ProtoAdapter.BYTES.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return NetworkOpen(
            network_id = network_id,
            network_key = network_key,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: NetworkOpen): NetworkOpen = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class NetworkClose(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#UINT64",
      label = WireField.Label.OMIT_IDENTITY,
      jsonName = "networkId"
    )
    val network_id: Long = 0L,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<NetworkClose, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is NetworkClose) return false
      if (unknownFields != other.unknownFields) return false
      if (network_id != other.network_id) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + network_id.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """network_id=$network_id"""
      return result.joinToString(prefix = "NetworkClose{", separator = ", ", postfix = "}")
    }

    fun copy(network_id: Long = this.network_id, unknownFields: ByteString = this.unknownFields):
        NetworkClose = NetworkClose(network_id, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<NetworkClose> = object : ProtoAdapter<NetworkClose>(
        FieldEncoding.LENGTH_DELIMITED, 
        NetworkClose::class, 
        "type.googleapis.com/NetworkEvent.NetworkClose", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: NetworkClose): Int {
          var size = value.unknownFields.size
          if (value.network_id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1,
              value.network_id)
          return size
        }

        override fun encode(writer: ProtoWriter, value: NetworkClose) {
          if (value.network_id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.network_id)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): NetworkClose {
          var network_id: Long = 0L
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> network_id = ProtoAdapter.UINT64.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return NetworkClose(
            network_id = network_id,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: NetworkClose): NetworkClose = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }
}
