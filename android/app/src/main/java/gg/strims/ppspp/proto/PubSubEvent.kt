// Code generated by Wire protocol buffer compiler, do not edit.
// Source: PubSubEvent in pub_sub.proto
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

class PubSubEvent(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.ppspp.proto.PubSubEvent${'$'}Message#ADAPTER"
  )
  val message: Message? = null,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.ppspp.proto.PubSubEvent${'$'}Close#ADAPTER"
  )
  val close: Close? = null,
  @field:WireField(
    tag = 3,
    adapter = "gg.strims.ppspp.proto.PubSubEvent${'$'}Padding#ADAPTER"
  )
  val padding: Padding? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<PubSubEvent, Nothing>(ADAPTER, unknownFields) {
  init {
    require(countNonNull(message, close, padding) <= 1) {
      "At most one of message, close, padding may be non-null"
    }
  }

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is PubSubEvent) return false
    if (unknownFields != other.unknownFields) return false
    if (message != other.message) return false
    if (close != other.close) return false
    if (padding != other.padding) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + message.hashCode()
      result = result * 37 + close.hashCode()
      result = result * 37 + padding.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (message != null) result += """message=$message"""
    if (close != null) result += """close=$close"""
    if (padding != null) result += """padding=$padding"""
    return result.joinToString(prefix = "PubSubEvent{", separator = ", ", postfix = "}")
  }

  fun copy(
    message: Message? = this.message,
    close: Close? = this.close,
    padding: Padding? = this.padding,
    unknownFields: ByteString = this.unknownFields
  ): PubSubEvent = PubSubEvent(message, close, padding, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<PubSubEvent> = object : ProtoAdapter<PubSubEvent>(
      FieldEncoding.LENGTH_DELIMITED, 
      PubSubEvent::class, 
      "type.googleapis.com/PubSubEvent", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: PubSubEvent): Int {
        var size = value.unknownFields.size
        size += Message.ADAPTER.encodedSizeWithTag(1, value.message)
        size += Close.ADAPTER.encodedSizeWithTag(2, value.close)
        size += Padding.ADAPTER.encodedSizeWithTag(3, value.padding)
        return size
      }

      override fun encode(writer: ProtoWriter, value: PubSubEvent) {
        Message.ADAPTER.encodeWithTag(writer, 1, value.message)
        Close.ADAPTER.encodeWithTag(writer, 2, value.close)
        Padding.ADAPTER.encodeWithTag(writer, 3, value.padding)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): PubSubEvent {
        var message: Message? = null
        var close: Close? = null
        var padding: Padding? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> message = Message.ADAPTER.decode(reader)
            2 -> close = Close.ADAPTER.decode(reader)
            3 -> padding = Padding.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return PubSubEvent(
          message = message,
          close = close,
          padding = padding,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: PubSubEvent): PubSubEvent = value.copy(
        message = value.message?.let(Message.ADAPTER::redact),
        close = value.close?.let(Close.ADAPTER::redact),
        padding = value.padding?.let(Padding.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }

  class Message(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#INT64",
      label = WireField.Label.OMIT_IDENTITY
    )
    val time: Long = 0L,
    @field:WireField(
      tag = 3,
      adapter = "com.squareup.wire.ProtoAdapter#STRING",
      label = WireField.Label.OMIT_IDENTITY
    )
    val key: String = "",
    @field:WireField(
      tag = 4,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val body: ByteString = ByteString.EMPTY,
    unknownFields: ByteString = ByteString.EMPTY
  ) : com.squareup.wire.Message<Message, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Message) return false
      if (unknownFields != other.unknownFields) return false
      if (time != other.time) return false
      if (key != other.key) return false
      if (body != other.body) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + time.hashCode()
        result = result * 37 + key.hashCode()
        result = result * 37 + body.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """time=$time"""
      result += """key=${sanitize(key)}"""
      result += """body=$body"""
      return result.joinToString(prefix = "Message{", separator = ", ", postfix = "}")
    }

    fun copy(
      time: Long = this.time,
      key: String = this.key,
      body: ByteString = this.body,
      unknownFields: ByteString = this.unknownFields
    ): Message = Message(time, key, body, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Message> = object : ProtoAdapter<Message>(
        FieldEncoding.LENGTH_DELIMITED, 
        Message::class, 
        "type.googleapis.com/PubSubEvent.Message", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Message): Int {
          var size = value.unknownFields.size
          if (value.time != 0L) size += ProtoAdapter.INT64.encodedSizeWithTag(1, value.time)
          if (value.key != "") size += ProtoAdapter.STRING.encodedSizeWithTag(3, value.key)
          if (value.body != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(4,
              value.body)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Message) {
          if (value.time != 0L) ProtoAdapter.INT64.encodeWithTag(writer, 1, value.time)
          if (value.key != "") ProtoAdapter.STRING.encodeWithTag(writer, 3, value.key)
          if (value.body != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 4,
              value.body)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Message {
          var time: Long = 0L
          var key: String = ""
          var body: ByteString = ByteString.EMPTY
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> time = ProtoAdapter.INT64.decode(reader)
              3 -> key = ProtoAdapter.STRING.decode(reader)
              4 -> body = ProtoAdapter.BYTES.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Message(
            time = time,
            key = key,
            body = body,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Message): Message = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class Close(
    unknownFields: ByteString = ByteString.EMPTY
  ) : com.squareup.wire.Message<Close, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Close) return false
      if (unknownFields != other.unknownFields) return false
      return true
    }

    override fun hashCode(): Int = unknownFields.hashCode()

    override fun toString(): String = "Close{}"

    fun copy(unknownFields: ByteString = this.unknownFields): Close = Close(unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Close> = object : ProtoAdapter<Close>(
        FieldEncoding.LENGTH_DELIMITED, 
        Close::class, 
        "type.googleapis.com/PubSubEvent.Close", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Close): Int {
          var size = value.unknownFields.size
          return size
        }

        override fun encode(writer: ProtoWriter, value: Close) {
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Close {
          val unknownFields = reader.forEachTag(reader::readUnknownField)
          return Close(
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Close): Close = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class Padding(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val body: ByteString = ByteString.EMPTY,
    unknownFields: ByteString = ByteString.EMPTY
  ) : com.squareup.wire.Message<Padding, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Padding) return false
      if (unknownFields != other.unknownFields) return false
      if (body != other.body) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + body.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """body=$body"""
      return result.joinToString(prefix = "Padding{", separator = ", ", postfix = "}")
    }

    fun copy(body: ByteString = this.body, unknownFields: ByteString = this.unknownFields): Padding
        = Padding(body, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Padding> = object : ProtoAdapter<Padding>(
        FieldEncoding.LENGTH_DELIMITED, 
        Padding::class, 
        "type.googleapis.com/PubSubEvent.Padding", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Padding): Int {
          var size = value.unknownFields.size
          if (value.body != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
              value.body)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Padding) {
          if (value.body != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1,
              value.body)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Padding {
          var body: ByteString = ByteString.EMPTY
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> body = ProtoAdapter.BYTES.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Padding(
            body = body,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Padding): Padding = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }
}