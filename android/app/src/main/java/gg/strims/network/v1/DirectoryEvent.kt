// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.network.v1.DirectoryEvent in network/v1/directory.proto
package gg.strims.network.v1

import com.squareup.wire.FieldEncoding
import com.squareup.wire.Message
import com.squareup.wire.ProtoAdapter
import com.squareup.wire.ProtoReader
import com.squareup.wire.ProtoWriter
import com.squareup.wire.Syntax
import com.squareup.wire.Syntax.PROTO_3
import com.squareup.wire.WireField
import com.squareup.wire.internal.countNonNull
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

class DirectoryEvent(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.network.v1.DirectoryEvent${'$'}Publish#ADAPTER"
  )
  val publish: Publish? = null,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.network.v1.DirectoryEvent${'$'}Unpublish#ADAPTER"
  )
  val unpublish: Unpublish? = null,
  @field:WireField(
    tag = 3,
    adapter = "gg.strims.network.v1.DirectoryEvent${'$'}ViewerCountChange#ADAPTER",
    jsonName = "viewerCountChange"
  )
  val viewer_count_change: ViewerCountChange? = null,
  @field:WireField(
    tag = 4,
    adapter = "gg.strims.network.v1.DirectoryEvent${'$'}ViewerStateChange#ADAPTER",
    jsonName = "viewerStateChange"
  )
  val viewer_state_change: ViewerStateChange? = null,
  @field:WireField(
    tag = 5,
    adapter = "gg.strims.network.v1.DirectoryEvent${'$'}Ping#ADAPTER"
  )
  val ping: Ping? = null,
  @field:WireField(
    tag = 6,
    adapter = "gg.strims.network.v1.DirectoryEvent${'$'}Padding#ADAPTER"
  )
  val padding: Padding? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<DirectoryEvent, Nothing>(ADAPTER, unknownFields) {
  init {
    require(countNonNull(publish, unpublish, viewer_count_change, viewer_state_change, ping,
        padding) <= 1) {
      "At most one of publish, unpublish, viewer_count_change, viewer_state_change, ping, padding may be non-null"
    }
  }

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is DirectoryEvent) return false
    if (unknownFields != other.unknownFields) return false
    if (publish != other.publish) return false
    if (unpublish != other.unpublish) return false
    if (viewer_count_change != other.viewer_count_change) return false
    if (viewer_state_change != other.viewer_state_change) return false
    if (ping != other.ping) return false
    if (padding != other.padding) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + publish.hashCode()
      result = result * 37 + unpublish.hashCode()
      result = result * 37 + viewer_count_change.hashCode()
      result = result * 37 + viewer_state_change.hashCode()
      result = result * 37 + ping.hashCode()
      result = result * 37 + padding.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (publish != null) result += """publish=$publish"""
    if (unpublish != null) result += """unpublish=$unpublish"""
    if (viewer_count_change != null) result += """viewer_count_change=$viewer_count_change"""
    if (viewer_state_change != null) result += """viewer_state_change=$viewer_state_change"""
    if (ping != null) result += """ping=$ping"""
    if (padding != null) result += """padding=$padding"""
    return result.joinToString(prefix = "DirectoryEvent{", separator = ", ", postfix = "}")
  }

  fun copy(
    publish: Publish? = this.publish,
    unpublish: Unpublish? = this.unpublish,
    viewer_count_change: ViewerCountChange? = this.viewer_count_change,
    viewer_state_change: ViewerStateChange? = this.viewer_state_change,
    ping: Ping? = this.ping,
    padding: Padding? = this.padding,
    unknownFields: ByteString = this.unknownFields
  ): DirectoryEvent = DirectoryEvent(publish, unpublish, viewer_count_change, viewer_state_change,
      ping, padding, unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<DirectoryEvent> = object : ProtoAdapter<DirectoryEvent>(
      FieldEncoding.LENGTH_DELIMITED, 
      DirectoryEvent::class, 
      "type.googleapis.com/strims.network.v1.DirectoryEvent", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: DirectoryEvent): Int {
        var size = value.unknownFields.size
        size += Publish.ADAPTER.encodedSizeWithTag(1, value.publish)
        size += Unpublish.ADAPTER.encodedSizeWithTag(2, value.unpublish)
        size += ViewerCountChange.ADAPTER.encodedSizeWithTag(3, value.viewer_count_change)
        size += ViewerStateChange.ADAPTER.encodedSizeWithTag(4, value.viewer_state_change)
        size += Ping.ADAPTER.encodedSizeWithTag(5, value.ping)
        size += Padding.ADAPTER.encodedSizeWithTag(6, value.padding)
        return size
      }

      override fun encode(writer: ProtoWriter, value: DirectoryEvent) {
        Publish.ADAPTER.encodeWithTag(writer, 1, value.publish)
        Unpublish.ADAPTER.encodeWithTag(writer, 2, value.unpublish)
        ViewerCountChange.ADAPTER.encodeWithTag(writer, 3, value.viewer_count_change)
        ViewerStateChange.ADAPTER.encodeWithTag(writer, 4, value.viewer_state_change)
        Ping.ADAPTER.encodeWithTag(writer, 5, value.ping)
        Padding.ADAPTER.encodeWithTag(writer, 6, value.padding)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): DirectoryEvent {
        var publish: Publish? = null
        var unpublish: Unpublish? = null
        var viewer_count_change: ViewerCountChange? = null
        var viewer_state_change: ViewerStateChange? = null
        var ping: Ping? = null
        var padding: Padding? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> publish = Publish.ADAPTER.decode(reader)
            2 -> unpublish = Unpublish.ADAPTER.decode(reader)
            3 -> viewer_count_change = ViewerCountChange.ADAPTER.decode(reader)
            4 -> viewer_state_change = ViewerStateChange.ADAPTER.decode(reader)
            5 -> ping = Ping.ADAPTER.decode(reader)
            6 -> padding = Padding.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return DirectoryEvent(
          publish = publish,
          unpublish = unpublish,
          viewer_count_change = viewer_count_change,
          viewer_state_change = viewer_state_change,
          ping = ping,
          padding = padding,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: DirectoryEvent): DirectoryEvent = value.copy(
        publish = value.publish?.let(Publish.ADAPTER::redact),
        unpublish = value.unpublish?.let(Unpublish.ADAPTER::redact),
        viewer_count_change = value.viewer_count_change?.let(ViewerCountChange.ADAPTER::redact),
        viewer_state_change = value.viewer_state_change?.let(ViewerStateChange.ADAPTER::redact),
        ping = value.ping?.let(Ping.ADAPTER::redact),
        padding = value.padding?.let(Padding.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }

  class Publish(
    @field:WireField(
      tag = 1,
      adapter = "gg.strims.network.v1.DirectoryListing#ADAPTER",
      label = WireField.Label.OMIT_IDENTITY
    )
    val listing: DirectoryListing? = null,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<Publish, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Publish) return false
      if (unknownFields != other.unknownFields) return false
      if (listing != other.listing) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + listing.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      if (listing != null) result += """listing=$listing"""
      return result.joinToString(prefix = "Publish{", separator = ", ", postfix = "}")
    }

    fun copy(listing: DirectoryListing? = this.listing, unknownFields: ByteString =
        this.unknownFields): Publish = Publish(listing, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Publish> = object : ProtoAdapter<Publish>(
        FieldEncoding.LENGTH_DELIMITED, 
        Publish::class, 
        "type.googleapis.com/strims.network.v1.DirectoryEvent.Publish", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Publish): Int {
          var size = value.unknownFields.size
          if (value.listing != null) size += DirectoryListing.ADAPTER.encodedSizeWithTag(1,
              value.listing)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Publish) {
          if (value.listing != null) DirectoryListing.ADAPTER.encodeWithTag(writer, 1,
              value.listing)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Publish {
          var listing: DirectoryListing? = null
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> listing = DirectoryListing.ADAPTER.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Publish(
            listing = listing,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Publish): Publish = value.copy(
          listing = value.listing?.let(DirectoryListing.ADAPTER::redact),
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class Unpublish(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val key: ByteString = ByteString.EMPTY,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<Unpublish, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Unpublish) return false
      if (unknownFields != other.unknownFields) return false
      if (key != other.key) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + key.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """key=$key"""
      return result.joinToString(prefix = "Unpublish{", separator = ", ", postfix = "}")
    }

    fun copy(key: ByteString = this.key, unknownFields: ByteString = this.unknownFields): Unpublish
        = Unpublish(key, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Unpublish> = object : ProtoAdapter<Unpublish>(
        FieldEncoding.LENGTH_DELIMITED, 
        Unpublish::class, 
        "type.googleapis.com/strims.network.v1.DirectoryEvent.Unpublish", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Unpublish): Int {
          var size = value.unknownFields.size
          if (value.key != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
              value.key)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Unpublish) {
          if (value.key != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1, value.key)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Unpublish {
          var key: ByteString = ByteString.EMPTY
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> key = ProtoAdapter.BYTES.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Unpublish(
            key = key,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Unpublish): Unpublish = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class ViewerCountChange(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val key: ByteString = ByteString.EMPTY,
    @field:WireField(
      tag = 2,
      adapter = "com.squareup.wire.ProtoAdapter#UINT32",
      label = WireField.Label.OMIT_IDENTITY
    )
    val count: Int = 0,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<ViewerCountChange, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is ViewerCountChange) return false
      if (unknownFields != other.unknownFields) return false
      if (key != other.key) return false
      if (count != other.count) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + key.hashCode()
        result = result * 37 + count.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """key=$key"""
      result += """count=$count"""
      return result.joinToString(prefix = "ViewerCountChange{", separator = ", ", postfix = "}")
    }

    fun copy(
      key: ByteString = this.key,
      count: Int = this.count,
      unknownFields: ByteString = this.unknownFields
    ): ViewerCountChange = ViewerCountChange(key, count, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<ViewerCountChange> = object : ProtoAdapter<ViewerCountChange>(
        FieldEncoding.LENGTH_DELIMITED, 
        ViewerCountChange::class, 
        "type.googleapis.com/strims.network.v1.DirectoryEvent.ViewerCountChange", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: ViewerCountChange): Int {
          var size = value.unknownFields.size
          if (value.key != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
              value.key)
          if (value.count != 0) size += ProtoAdapter.UINT32.encodedSizeWithTag(2, value.count)
          return size
        }

        override fun encode(writer: ProtoWriter, value: ViewerCountChange) {
          if (value.key != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1, value.key)
          if (value.count != 0) ProtoAdapter.UINT32.encodeWithTag(writer, 2, value.count)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): ViewerCountChange {
          var key: ByteString = ByteString.EMPTY
          var count: Int = 0
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> key = ProtoAdapter.BYTES.decode(reader)
              2 -> count = ProtoAdapter.UINT32.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return ViewerCountChange(
            key = key,
            count = count,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: ViewerCountChange): ViewerCountChange = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class ViewerStateChange(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#STRING",
      label = WireField.Label.OMIT_IDENTITY
    )
    val subject: String = "",
    @field:WireField(
      tag = 2,
      adapter = "com.squareup.wire.ProtoAdapter#BOOL",
      label = WireField.Label.OMIT_IDENTITY
    )
    val online: Boolean = false,
    viewing_keys: List<ByteString> = emptyList(),
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<ViewerStateChange, Nothing>(ADAPTER, unknownFields) {
    @field:WireField(
      tag = 3,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.REPEATED,
      jsonName = "viewingKeys"
    )
    val viewing_keys: List<ByteString> = immutableCopyOf("viewing_keys", viewing_keys)

    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is ViewerStateChange) return false
      if (unknownFields != other.unknownFields) return false
      if (subject != other.subject) return false
      if (online != other.online) return false
      if (viewing_keys != other.viewing_keys) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + subject.hashCode()
        result = result * 37 + online.hashCode()
        result = result * 37 + viewing_keys.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """subject=${sanitize(subject)}"""
      result += """online=$online"""
      if (viewing_keys.isNotEmpty()) result += """viewing_keys=$viewing_keys"""
      return result.joinToString(prefix = "ViewerStateChange{", separator = ", ", postfix = "}")
    }

    fun copy(
      subject: String = this.subject,
      online: Boolean = this.online,
      viewing_keys: List<ByteString> = this.viewing_keys,
      unknownFields: ByteString = this.unknownFields
    ): ViewerStateChange = ViewerStateChange(subject, online, viewing_keys, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<ViewerStateChange> = object : ProtoAdapter<ViewerStateChange>(
        FieldEncoding.LENGTH_DELIMITED, 
        ViewerStateChange::class, 
        "type.googleapis.com/strims.network.v1.DirectoryEvent.ViewerStateChange", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: ViewerStateChange): Int {
          var size = value.unknownFields.size
          if (value.subject != "") size += ProtoAdapter.STRING.encodedSizeWithTag(1, value.subject)
          if (value.online != false) size += ProtoAdapter.BOOL.encodedSizeWithTag(2, value.online)
          size += ProtoAdapter.BYTES.asRepeated().encodedSizeWithTag(3, value.viewing_keys)
          return size
        }

        override fun encode(writer: ProtoWriter, value: ViewerStateChange) {
          if (value.subject != "") ProtoAdapter.STRING.encodeWithTag(writer, 1, value.subject)
          if (value.online != false) ProtoAdapter.BOOL.encodeWithTag(writer, 2, value.online)
          ProtoAdapter.BYTES.asRepeated().encodeWithTag(writer, 3, value.viewing_keys)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): ViewerStateChange {
          var subject: String = ""
          var online: Boolean = false
          val viewing_keys = mutableListOf<ByteString>()
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> subject = ProtoAdapter.STRING.decode(reader)
              2 -> online = ProtoAdapter.BOOL.decode(reader)
              3 -> viewing_keys.add(ProtoAdapter.BYTES.decode(reader))
              else -> reader.readUnknownField(tag)
            }
          }
          return ViewerStateChange(
            subject = subject,
            online = online,
            viewing_keys = viewing_keys,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: ViewerStateChange): ViewerStateChange = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class Ping(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#INT64",
      label = WireField.Label.OMIT_IDENTITY
    )
    val time: Long = 0L,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<Ping, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Ping) return false
      if (unknownFields != other.unknownFields) return false
      if (time != other.time) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + time.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """time=$time"""
      return result.joinToString(prefix = "Ping{", separator = ", ", postfix = "}")
    }

    fun copy(time: Long = this.time, unknownFields: ByteString = this.unknownFields): Ping =
        Ping(time, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Ping> = object : ProtoAdapter<Ping>(
        FieldEncoding.LENGTH_DELIMITED, 
        Ping::class, 
        "type.googleapis.com/strims.network.v1.DirectoryEvent.Ping", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Ping): Int {
          var size = value.unknownFields.size
          if (value.time != 0L) size += ProtoAdapter.INT64.encodedSizeWithTag(1, value.time)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Ping) {
          if (value.time != 0L) ProtoAdapter.INT64.encodeWithTag(writer, 1, value.time)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Ping {
          var time: Long = 0L
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> time = ProtoAdapter.INT64.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Ping(
            time = time,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Ping): Ping = value.copy(
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
    val data: ByteString = ByteString.EMPTY,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<Padding, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Padding) return false
      if (unknownFields != other.unknownFields) return false
      if (data != other.data) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + data.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """data=$data"""
      return result.joinToString(prefix = "Padding{", separator = ", ", postfix = "}")
    }

    fun copy(data: ByteString = this.data, unknownFields: ByteString = this.unknownFields): Padding
        = Padding(data, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Padding> = object : ProtoAdapter<Padding>(
        FieldEncoding.LENGTH_DELIMITED, 
        Padding::class, 
        "type.googleapis.com/strims.network.v1.DirectoryEvent.Padding", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Padding): Int {
          var size = value.unknownFields.size
          if (value.data != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
              value.data)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Padding) {
          if (value.data != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1,
              value.data)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Padding {
          var data: ByteString = ByteString.EMPTY
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> data = ProtoAdapter.BYTES.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Padding(
            data = data,
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
