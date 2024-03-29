// Code generated by Wire protocol buffer compiler, do not edit.
// Source: strims.vpn.v1.HashTableMessage in vpn/v1/hash_table.proto
package gg.strims.vpn.v1

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

class HashTableMessage(
  @field:WireField(
    tag = 1,
    adapter = "gg.strims.vpn.v1.HashTableMessage${'$'}Publish#ADAPTER"
  )
  val publish: Publish? = null,
  @field:WireField(
    tag = 2,
    adapter = "gg.strims.vpn.v1.HashTableMessage${'$'}Unpublish#ADAPTER"
  )
  val unpublish: Unpublish? = null,
  @field:WireField(
    tag = 3,
    adapter = "gg.strims.vpn.v1.HashTableMessage${'$'}GetRequest#ADAPTER",
    jsonName = "getRequest"
  )
  val get_request: GetRequest? = null,
  @field:WireField(
    tag = 4,
    adapter = "gg.strims.vpn.v1.HashTableMessage${'$'}GetResponse#ADAPTER",
    jsonName = "getResponse"
  )
  val get_response: GetResponse? = null,
  unknownFields: ByteString = ByteString.EMPTY
) : Message<HashTableMessage, Nothing>(ADAPTER, unknownFields) {
  init {
    require(countNonNull(publish, unpublish, get_request, get_response) <= 1) {
      "At most one of publish, unpublish, get_request, get_response may be non-null"
    }
  }

  @Deprecated(
    message = "Shouldn't be used in Kotlin",
    level = DeprecationLevel.HIDDEN
  )
  override fun newBuilder(): Nothing = throw AssertionError()

  override fun equals(other: Any?): Boolean {
    if (other === this) return true
    if (other !is HashTableMessage) return false
    if (unknownFields != other.unknownFields) return false
    if (publish != other.publish) return false
    if (unpublish != other.unpublish) return false
    if (get_request != other.get_request) return false
    if (get_response != other.get_response) return false
    return true
  }

  override fun hashCode(): Int {
    var result = super.hashCode
    if (result == 0) {
      result = unknownFields.hashCode()
      result = result * 37 + publish.hashCode()
      result = result * 37 + unpublish.hashCode()
      result = result * 37 + get_request.hashCode()
      result = result * 37 + get_response.hashCode()
      super.hashCode = result
    }
    return result
  }

  override fun toString(): String {
    val result = mutableListOf<String>()
    if (publish != null) result += """publish=$publish"""
    if (unpublish != null) result += """unpublish=$unpublish"""
    if (get_request != null) result += """get_request=$get_request"""
    if (get_response != null) result += """get_response=$get_response"""
    return result.joinToString(prefix = "HashTableMessage{", separator = ", ", postfix = "}")
  }

  fun copy(
    publish: Publish? = this.publish,
    unpublish: Unpublish? = this.unpublish,
    get_request: GetRequest? = this.get_request,
    get_response: GetResponse? = this.get_response,
    unknownFields: ByteString = this.unknownFields
  ): HashTableMessage = HashTableMessage(publish, unpublish, get_request, get_response,
      unknownFields)

  companion object {
    @JvmField
    val ADAPTER: ProtoAdapter<HashTableMessage> = object : ProtoAdapter<HashTableMessage>(
      FieldEncoding.LENGTH_DELIMITED, 
      HashTableMessage::class, 
      "type.googleapis.com/strims.vpn.v1.HashTableMessage", 
      PROTO_3, 
      null
    ) {
      override fun encodedSize(value: HashTableMessage): Int {
        var size = value.unknownFields.size
        size += Publish.ADAPTER.encodedSizeWithTag(1, value.publish)
        size += Unpublish.ADAPTER.encodedSizeWithTag(2, value.unpublish)
        size += GetRequest.ADAPTER.encodedSizeWithTag(3, value.get_request)
        size += GetResponse.ADAPTER.encodedSizeWithTag(4, value.get_response)
        return size
      }

      override fun encode(writer: ProtoWriter, value: HashTableMessage) {
        Publish.ADAPTER.encodeWithTag(writer, 1, value.publish)
        Unpublish.ADAPTER.encodeWithTag(writer, 2, value.unpublish)
        GetRequest.ADAPTER.encodeWithTag(writer, 3, value.get_request)
        GetResponse.ADAPTER.encodeWithTag(writer, 4, value.get_response)
        writer.writeBytes(value.unknownFields)
      }

      override fun decode(reader: ProtoReader): HashTableMessage {
        var publish: Publish? = null
        var unpublish: Unpublish? = null
        var get_request: GetRequest? = null
        var get_response: GetResponse? = null
        val unknownFields = reader.forEachTag { tag ->
          when (tag) {
            1 -> publish = Publish.ADAPTER.decode(reader)
            2 -> unpublish = Unpublish.ADAPTER.decode(reader)
            3 -> get_request = GetRequest.ADAPTER.decode(reader)
            4 -> get_response = GetResponse.ADAPTER.decode(reader)
            else -> reader.readUnknownField(tag)
          }
        }
        return HashTableMessage(
          publish = publish,
          unpublish = unpublish,
          get_request = get_request,
          get_response = get_response,
          unknownFields = unknownFields
        )
      }

      override fun redact(value: HashTableMessage): HashTableMessage = value.copy(
        publish = value.publish?.let(Publish.ADAPTER::redact),
        unpublish = value.unpublish?.let(Unpublish.ADAPTER::redact),
        get_request = value.get_request?.let(GetRequest.ADAPTER::redact),
        get_response = value.get_response?.let(GetResponse.ADAPTER::redact),
        unknownFields = ByteString.EMPTY
      )
    }

    private const val serialVersionUID: Long = 0L
  }

  class Record(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val key: ByteString = ByteString.EMPTY,
    @field:WireField(
      tag = 2,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val salt: ByteString = ByteString.EMPTY,
    @field:WireField(
      tag = 3,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val value: ByteString = ByteString.EMPTY,
    @field:WireField(
      tag = 4,
      adapter = "com.squareup.wire.ProtoAdapter#INT64",
      label = WireField.Label.OMIT_IDENTITY
    )
    val timestamp: Long = 0L,
    @field:WireField(
      tag = 5,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val signature: ByteString = ByteString.EMPTY,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<Record, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is Record) return false
      if (unknownFields != other.unknownFields) return false
      if (key != other.key) return false
      if (salt != other.salt) return false
      if (value != other.value) return false
      if (timestamp != other.timestamp) return false
      if (signature != other.signature) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + key.hashCode()
        result = result * 37 + salt.hashCode()
        result = result * 37 + value.hashCode()
        result = result * 37 + timestamp.hashCode()
        result = result * 37 + signature.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """key=$key"""
      result += """salt=$salt"""
      result += """value=$value"""
      result += """timestamp=$timestamp"""
      result += """signature=$signature"""
      return result.joinToString(prefix = "Record{", separator = ", ", postfix = "}")
    }

    fun copy(
      key: ByteString = this.key,
      salt: ByteString = this.salt,
      value: ByteString = this.value,
      timestamp: Long = this.timestamp,
      signature: ByteString = this.signature,
      unknownFields: ByteString = this.unknownFields
    ): Record = Record(key, salt, value, timestamp, signature, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Record> = object : ProtoAdapter<Record>(
        FieldEncoding.LENGTH_DELIMITED, 
        Record::class, 
        "type.googleapis.com/strims.vpn.v1.HashTableMessage.Record", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Record): Int {
          var size = value.unknownFields.size
          if (value.key != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(1,
              value.key)
          if (value.salt != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(2,
              value.salt)
          if (value.value != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(3,
              value.value)
          if (value.timestamp != 0L) size += ProtoAdapter.INT64.encodedSizeWithTag(4,
              value.timestamp)
          if (value.signature != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(5,
              value.signature)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Record) {
          if (value.key != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 1, value.key)
          if (value.salt != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 2,
              value.salt)
          if (value.value != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 3,
              value.value)
          if (value.timestamp != 0L) ProtoAdapter.INT64.encodeWithTag(writer, 4, value.timestamp)
          if (value.signature != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 5,
              value.signature)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Record {
          var key: ByteString = ByteString.EMPTY
          var salt: ByteString = ByteString.EMPTY
          var value: ByteString = ByteString.EMPTY
          var timestamp: Long = 0L
          var signature: ByteString = ByteString.EMPTY
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> key = ProtoAdapter.BYTES.decode(reader)
              2 -> salt = ProtoAdapter.BYTES.decode(reader)
              3 -> value = ProtoAdapter.BYTES.decode(reader)
              4 -> timestamp = ProtoAdapter.INT64.decode(reader)
              5 -> signature = ProtoAdapter.BYTES.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Record(
            key = key,
            salt = salt,
            value = value,
            timestamp = timestamp,
            signature = signature,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Record): Record = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class Publish(
    @field:WireField(
      tag = 1,
      adapter = "gg.strims.vpn.v1.HashTableMessage${'$'}Record#ADAPTER",
      label = WireField.Label.OMIT_IDENTITY
    )
    val record: Record? = null,
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
      if (record != other.record) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + record.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      if (record != null) result += """record=$record"""
      return result.joinToString(prefix = "Publish{", separator = ", ", postfix = "}")
    }

    fun copy(record: Record? = this.record, unknownFields: ByteString = this.unknownFields): Publish
        = Publish(record, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Publish> = object : ProtoAdapter<Publish>(
        FieldEncoding.LENGTH_DELIMITED, 
        Publish::class, 
        "type.googleapis.com/strims.vpn.v1.HashTableMessage.Publish", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Publish): Int {
          var size = value.unknownFields.size
          if (value.record != null) size += Record.ADAPTER.encodedSizeWithTag(1, value.record)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Publish) {
          if (value.record != null) Record.ADAPTER.encodeWithTag(writer, 1, value.record)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Publish {
          var record: Record? = null
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> record = Record.ADAPTER.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Publish(
            record = record,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Publish): Publish = value.copy(
          record = value.record?.let(Record.ADAPTER::redact),
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class Unpublish(
    @field:WireField(
      tag = 1,
      adapter = "gg.strims.vpn.v1.HashTableMessage${'$'}Record#ADAPTER",
      label = WireField.Label.OMIT_IDENTITY
    )
    val record: Record? = null,
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
      if (record != other.record) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + record.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      if (record != null) result += """record=$record"""
      return result.joinToString(prefix = "Unpublish{", separator = ", ", postfix = "}")
    }

    fun copy(record: Record? = this.record, unknownFields: ByteString = this.unknownFields):
        Unpublish = Unpublish(record, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<Unpublish> = object : ProtoAdapter<Unpublish>(
        FieldEncoding.LENGTH_DELIMITED, 
        Unpublish::class, 
        "type.googleapis.com/strims.vpn.v1.HashTableMessage.Unpublish", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: Unpublish): Int {
          var size = value.unknownFields.size
          if (value.record != null) size += Record.ADAPTER.encodedSizeWithTag(1, value.record)
          return size
        }

        override fun encode(writer: ProtoWriter, value: Unpublish) {
          if (value.record != null) Record.ADAPTER.encodeWithTag(writer, 1, value.record)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): Unpublish {
          var record: Record? = null
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> record = Record.ADAPTER.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return Unpublish(
            record = record,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: Unpublish): Unpublish = value.copy(
          record = value.record?.let(Record.ADAPTER::redact),
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class GetRequest(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#UINT64",
      label = WireField.Label.OMIT_IDENTITY,
      jsonName = "requestId"
    )
    val request_id: Long = 0L,
    @field:WireField(
      tag = 2,
      adapter = "com.squareup.wire.ProtoAdapter#BYTES",
      label = WireField.Label.OMIT_IDENTITY
    )
    val hash: ByteString = ByteString.EMPTY,
    @field:WireField(
      tag = 3,
      adapter = "com.squareup.wire.ProtoAdapter#INT64",
      label = WireField.Label.OMIT_IDENTITY,
      jsonName = "ifModifiedSince"
    )
    val if_modified_since: Long = 0L,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<GetRequest, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is GetRequest) return false
      if (unknownFields != other.unknownFields) return false
      if (request_id != other.request_id) return false
      if (hash != other.hash) return false
      if (if_modified_since != other.if_modified_since) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + request_id.hashCode()
        result = result * 37 + hash.hashCode()
        result = result * 37 + if_modified_since.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """request_id=$request_id"""
      result += """hash=$hash"""
      result += """if_modified_since=$if_modified_since"""
      return result.joinToString(prefix = "GetRequest{", separator = ", ", postfix = "}")
    }

    fun copy(
      request_id: Long = this.request_id,
      hash: ByteString = this.hash,
      if_modified_since: Long = this.if_modified_since,
      unknownFields: ByteString = this.unknownFields
    ): GetRequest = GetRequest(request_id, hash, if_modified_since, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<GetRequest> = object : ProtoAdapter<GetRequest>(
        FieldEncoding.LENGTH_DELIMITED, 
        GetRequest::class, 
        "type.googleapis.com/strims.vpn.v1.HashTableMessage.GetRequest", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: GetRequest): Int {
          var size = value.unknownFields.size
          if (value.request_id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1,
              value.request_id)
          if (value.hash != ByteString.EMPTY) size += ProtoAdapter.BYTES.encodedSizeWithTag(2,
              value.hash)
          if (value.if_modified_since != 0L) size += ProtoAdapter.INT64.encodedSizeWithTag(3,
              value.if_modified_since)
          return size
        }

        override fun encode(writer: ProtoWriter, value: GetRequest) {
          if (value.request_id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.request_id)
          if (value.hash != ByteString.EMPTY) ProtoAdapter.BYTES.encodeWithTag(writer, 2,
              value.hash)
          if (value.if_modified_since != 0L) ProtoAdapter.INT64.encodeWithTag(writer, 3,
              value.if_modified_since)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): GetRequest {
          var request_id: Long = 0L
          var hash: ByteString = ByteString.EMPTY
          var if_modified_since: Long = 0L
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> request_id = ProtoAdapter.UINT64.decode(reader)
              2 -> hash = ProtoAdapter.BYTES.decode(reader)
              3 -> if_modified_since = ProtoAdapter.INT64.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return GetRequest(
            request_id = request_id,
            hash = hash,
            if_modified_since = if_modified_since,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: GetRequest): GetRequest = value.copy(
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }

  class GetResponse(
    @field:WireField(
      tag = 1,
      adapter = "com.squareup.wire.ProtoAdapter#UINT64",
      label = WireField.Label.OMIT_IDENTITY,
      jsonName = "requestId"
    )
    val request_id: Long = 0L,
    @field:WireField(
      tag = 2,
      adapter = "gg.strims.vpn.v1.HashTableMessage${'$'}Record#ADAPTER",
      label = WireField.Label.OMIT_IDENTITY
    )
    val record: Record? = null,
    unknownFields: ByteString = ByteString.EMPTY
  ) : Message<GetResponse, Nothing>(ADAPTER, unknownFields) {
    @Deprecated(
      message = "Shouldn't be used in Kotlin",
      level = DeprecationLevel.HIDDEN
    )
    override fun newBuilder(): Nothing = throw AssertionError()

    override fun equals(other: Any?): Boolean {
      if (other === this) return true
      if (other !is GetResponse) return false
      if (unknownFields != other.unknownFields) return false
      if (request_id != other.request_id) return false
      if (record != other.record) return false
      return true
    }

    override fun hashCode(): Int {
      var result = super.hashCode
      if (result == 0) {
        result = unknownFields.hashCode()
        result = result * 37 + request_id.hashCode()
        result = result * 37 + record.hashCode()
        super.hashCode = result
      }
      return result
    }

    override fun toString(): String {
      val result = mutableListOf<String>()
      result += """request_id=$request_id"""
      if (record != null) result += """record=$record"""
      return result.joinToString(prefix = "GetResponse{", separator = ", ", postfix = "}")
    }

    fun copy(
      request_id: Long = this.request_id,
      record: Record? = this.record,
      unknownFields: ByteString = this.unknownFields
    ): GetResponse = GetResponse(request_id, record, unknownFields)

    companion object {
      @JvmField
      val ADAPTER: ProtoAdapter<GetResponse> = object : ProtoAdapter<GetResponse>(
        FieldEncoding.LENGTH_DELIMITED, 
        GetResponse::class, 
        "type.googleapis.com/strims.vpn.v1.HashTableMessage.GetResponse", 
        PROTO_3, 
        null
      ) {
        override fun encodedSize(value: GetResponse): Int {
          var size = value.unknownFields.size
          if (value.request_id != 0L) size += ProtoAdapter.UINT64.encodedSizeWithTag(1,
              value.request_id)
          if (value.record != null) size += Record.ADAPTER.encodedSizeWithTag(2, value.record)
          return size
        }

        override fun encode(writer: ProtoWriter, value: GetResponse) {
          if (value.request_id != 0L) ProtoAdapter.UINT64.encodeWithTag(writer, 1, value.request_id)
          if (value.record != null) Record.ADAPTER.encodeWithTag(writer, 2, value.record)
          writer.writeBytes(value.unknownFields)
        }

        override fun decode(reader: ProtoReader): GetResponse {
          var request_id: Long = 0L
          var record: Record? = null
          val unknownFields = reader.forEachTag { tag ->
            when (tag) {
              1 -> request_id = ProtoAdapter.UINT64.decode(reader)
              2 -> record = Record.ADAPTER.decode(reader)
              else -> reader.readUnknownField(tag)
            }
          }
          return GetResponse(
            request_id = request_id,
            record = record,
            unknownFields = unknownFields
          )
        }

        override fun redact(value: GetResponse): GetResponse = value.copy(
          record = value.record?.let(Record.ADAPTER::redact),
          unknownFields = ByteString.EMPTY
        )
      }

      private const val serialVersionUID: Long = 0L
    }
  }
}
