// DO NOT EDIT.
// swift-format-ignore-file
//
// Generated by the Swift generator plugin for the protocol buffer compiler.
// Source: directory.proto
//
// For information on using the generated types, please see the documentation:
//   https://github.com/apple/swift-protobuf/

import Foundation
import SwiftProtobuf

// If the compiler emits an error on this type, it is because this file
// was generated by a version of the `protoc` Swift plug-in that is
// incompatible with the version of SwiftProtobuf to which you are linking.
// Please ensure that you are building against the same version of the API
// that was used to generate this file.
fileprivate struct _GeneratedWithProtocGenSwiftVersion: SwiftProtobuf.ProtobufAPIVersionCheck {
  struct _2: SwiftProtobuf.ProtobufAPIVersion_2 {}
  typealias Version = _2
}

public struct PBGetDirectoryEventsRequest {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var networkKey: Data = Data()

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}
}

public struct PBTestDirectoryPublishRequest {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var networkKey: Data = Data()

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}
}

public struct PBTestDirectoryPublishResponse {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}
}

public struct PBDirectoryListing {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var key: Data = Data()

  public var mimeType: String = String()

  public var title: String = String()

  public var description_p: String = String()

  public var tags: [String] = []

  public var extra: Data = Data()

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}
}

public struct PBDirectoryEvent {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var body: PBDirectoryEvent.OneOf_Body? = nil

  public var publish: PBDirectoryEvent.Publish {
    get {
      if case .publish(let v)? = body {return v}
      return PBDirectoryEvent.Publish()
    }
    set {body = .publish(newValue)}
  }

  public var unpublish: PBDirectoryEvent.Unpublish {
    get {
      if case .unpublish(let v)? = body {return v}
      return PBDirectoryEvent.Unpublish()
    }
    set {body = .unpublish(newValue)}
  }

  public var `open`: PBDirectoryEvent.ViewerChange {
    get {
      if case .open(let v)? = body {return v}
      return PBDirectoryEvent.ViewerChange()
    }
    set {body = .open(newValue)}
  }

  public var ping: PBDirectoryEvent.Ping {
    get {
      if case .ping(let v)? = body {return v}
      return PBDirectoryEvent.Ping()
    }
    set {body = .ping(newValue)}
  }

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public enum OneOf_Body: Equatable {
    case publish(PBDirectoryEvent.Publish)
    case unpublish(PBDirectoryEvent.Unpublish)
    case `open`(PBDirectoryEvent.ViewerChange)
    case ping(PBDirectoryEvent.Ping)

  #if !swift(>=4.1)
    public static func ==(lhs: PBDirectoryEvent.OneOf_Body, rhs: PBDirectoryEvent.OneOf_Body) -> Bool {
      switch (lhs, rhs) {
      case (.publish(let l), .publish(let r)): return l == r
      case (.unpublish(let l), .unpublish(let r)): return l == r
      case (.open(let l), .open(let r)): return l == r
      case (.ping(let l), .ping(let r)): return l == r
      default: return false
      }
    }
  #endif
  }

  public struct Publish {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var listing: PBDirectoryListing {
      get {return _listing ?? PBDirectoryListing()}
      set {_listing = newValue}
    }
    /// Returns true if `listing` has been explicitly set.
    public var hasListing: Bool {return self._listing != nil}
    /// Clears the value of `listing`. Subsequent reads from it will return its default value.
    public mutating func clearListing() {self._listing = nil}

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}

    fileprivate var _listing: PBDirectoryListing? = nil
  }

  public struct Unpublish {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var key: Data = Data()

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct ViewerChange {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var key: Data = Data()

    public var count: UInt32 = 0

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct Ping {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var time: Int64 = 0

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public init() {}
}

public struct PBCallDirectoryServerRequest {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var networkKey: Data = Data()

  public var body: PBCallDirectoryServerRequest.OneOf_Body? = nil

  public var listing: PBCallDirectoryServerRequest.RemoveListing {
    get {
      if case .listing(let v)? = body {return v}
      return PBCallDirectoryServerRequest.RemoveListing()
    }
    set {body = .listing(newValue)}
  }

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public enum OneOf_Body: Equatable {
    case listing(PBCallDirectoryServerRequest.RemoveListing)

  #if !swift(>=4.1)
    public static func ==(lhs: PBCallDirectoryServerRequest.OneOf_Body, rhs: PBCallDirectoryServerRequest.OneOf_Body) -> Bool {
      switch (lhs, rhs) {
      case (.listing(let l), .listing(let r)): return l == r
      }
    }
  #endif
  }

  public struct RemoveListing {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var key: Data = Data()

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public init() {}
}

public struct PBOpenDirectoryClientRequest {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var networkKey: Data = Data()

  public var serverKey: Data = Data()

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}
}

public struct PBDirectoryClientEvent {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var body: PBDirectoryClientEvent.OneOf_Body? = nil

  public var publish: PBDirectoryClientEvent.Publish {
    get {
      if case .publish(let v)? = body {return v}
      return PBDirectoryClientEvent.Publish()
    }
    set {body = .publish(newValue)}
  }

  public var unpublish: PBDirectoryClientEvent.Unpublish {
    get {
      if case .unpublish(let v)? = body {return v}
      return PBDirectoryClientEvent.Unpublish()
    }
    set {body = .unpublish(newValue)}
  }

  public var join: PBDirectoryClientEvent.Join {
    get {
      if case .join(let v)? = body {return v}
      return PBDirectoryClientEvent.Join()
    }
    set {body = .join(newValue)}
  }

  public var part: PBDirectoryClientEvent.Part {
    get {
      if case .part(let v)? = body {return v}
      return PBDirectoryClientEvent.Part()
    }
    set {body = .part(newValue)}
  }

  public var ping: PBDirectoryClientEvent.Ping {
    get {
      if case .ping(let v)? = body {return v}
      return PBDirectoryClientEvent.Ping()
    }
    set {body = .ping(newValue)}
  }

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public enum OneOf_Body: Equatable {
    case publish(PBDirectoryClientEvent.Publish)
    case unpublish(PBDirectoryClientEvent.Unpublish)
    case join(PBDirectoryClientEvent.Join)
    case part(PBDirectoryClientEvent.Part)
    case ping(PBDirectoryClientEvent.Ping)

  #if !swift(>=4.1)
    public static func ==(lhs: PBDirectoryClientEvent.OneOf_Body, rhs: PBDirectoryClientEvent.OneOf_Body) -> Bool {
      switch (lhs, rhs) {
      case (.publish(let l), .publish(let r)): return l == r
      case (.unpublish(let l), .unpublish(let r)): return l == r
      case (.join(let l), .join(let r)): return l == r
      case (.part(let l), .part(let r)): return l == r
      case (.ping(let l), .ping(let r)): return l == r
      default: return false
      }
    }
  #endif
  }

  public struct Publish {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var listing: PBDirectoryListing {
      get {return _listing ?? PBDirectoryListing()}
      set {_listing = newValue}
    }
    /// Returns true if `listing` has been explicitly set.
    public var hasListing: Bool {return self._listing != nil}
    /// Clears the value of `listing`. Subsequent reads from it will return its default value.
    public mutating func clearListing() {self._listing = nil}

    public var signature: Data = Data()

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}

    fileprivate var _listing: PBDirectoryListing? = nil
  }

  public struct Unpublish {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var key: Data = Data()

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct Join {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var key: Data = Data()

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct Part {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var key: Data = Data()

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct Ping {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var time: Int64 = 0

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public init() {}
}

// MARK: - Code below here is support for the SwiftProtobuf runtime.

extension PBGetDirectoryEventsRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "GetDirectoryEventsRequest"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .standard(proto: "network_key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.networkKey)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.networkKey.isEmpty {
      try visitor.visitSingularBytesField(value: self.networkKey, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBGetDirectoryEventsRequest, rhs: PBGetDirectoryEventsRequest) -> Bool {
    if lhs.networkKey != rhs.networkKey {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBTestDirectoryPublishRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "TestDirectoryPublishRequest"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .standard(proto: "network_key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.networkKey)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.networkKey.isEmpty {
      try visitor.visitSingularBytesField(value: self.networkKey, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBTestDirectoryPublishRequest, rhs: PBTestDirectoryPublishRequest) -> Bool {
    if lhs.networkKey != rhs.networkKey {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBTestDirectoryPublishResponse: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "TestDirectoryPublishResponse"
  public static let _protobuf_nameMap = SwiftProtobuf._NameMap()

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let _ = try decoder.nextFieldNumber() {
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBTestDirectoryPublishResponse, rhs: PBTestDirectoryPublishResponse) -> Bool {
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryListing: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "DirectoryListing"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
    2: .standard(proto: "mime_type"),
    3: .same(proto: "title"),
    4: .same(proto: "description"),
    5: .same(proto: "tags"),
    6: .same(proto: "extra"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      case 2: try decoder.decodeSingularStringField(value: &self.mimeType)
      case 3: try decoder.decodeSingularStringField(value: &self.title)
      case 4: try decoder.decodeSingularStringField(value: &self.description_p)
      case 5: try decoder.decodeRepeatedStringField(value: &self.tags)
      case 6: try decoder.decodeSingularBytesField(value: &self.extra)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    if !self.mimeType.isEmpty {
      try visitor.visitSingularStringField(value: self.mimeType, fieldNumber: 2)
    }
    if !self.title.isEmpty {
      try visitor.visitSingularStringField(value: self.title, fieldNumber: 3)
    }
    if !self.description_p.isEmpty {
      try visitor.visitSingularStringField(value: self.description_p, fieldNumber: 4)
    }
    if !self.tags.isEmpty {
      try visitor.visitRepeatedStringField(value: self.tags, fieldNumber: 5)
    }
    if !self.extra.isEmpty {
      try visitor.visitSingularBytesField(value: self.extra, fieldNumber: 6)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryListing, rhs: PBDirectoryListing) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.mimeType != rhs.mimeType {return false}
    if lhs.title != rhs.title {return false}
    if lhs.description_p != rhs.description_p {return false}
    if lhs.tags != rhs.tags {return false}
    if lhs.extra != rhs.extra {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryEvent: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "DirectoryEvent"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "publish"),
    2: .same(proto: "unpublish"),
    3: .same(proto: "open"),
    4: .same(proto: "ping"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1:
        var v: PBDirectoryEvent.Publish?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .publish(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .publish(v)}
      case 2:
        var v: PBDirectoryEvent.Unpublish?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .unpublish(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .unpublish(v)}
      case 3:
        var v: PBDirectoryEvent.ViewerChange?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .open(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .open(v)}
      case 4:
        var v: PBDirectoryEvent.Ping?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .ping(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .ping(v)}
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    switch self.body {
    case .publish(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 1)
    case .unpublish(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 2)
    case .open(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 3)
    case .ping(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 4)
    case nil: break
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryEvent, rhs: PBDirectoryEvent) -> Bool {
    if lhs.body != rhs.body {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryEvent.Publish: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryEvent.protoMessageName + ".Publish"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "listing"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularMessageField(value: &self._listing)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if let v = self._listing {
      try visitor.visitSingularMessageField(value: v, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryEvent.Publish, rhs: PBDirectoryEvent.Publish) -> Bool {
    if lhs._listing != rhs._listing {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryEvent.Unpublish: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryEvent.protoMessageName + ".Unpublish"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryEvent.Unpublish, rhs: PBDirectoryEvent.Unpublish) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryEvent.ViewerChange: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryEvent.protoMessageName + ".ViewerChange"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
    2: .same(proto: "count"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      case 2: try decoder.decodeSingularUInt32Field(value: &self.count)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    if self.count != 0 {
      try visitor.visitSingularUInt32Field(value: self.count, fieldNumber: 2)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryEvent.ViewerChange, rhs: PBDirectoryEvent.ViewerChange) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.count != rhs.count {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryEvent.Ping: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryEvent.protoMessageName + ".Ping"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "time"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularInt64Field(value: &self.time)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if self.time != 0 {
      try visitor.visitSingularInt64Field(value: self.time, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryEvent.Ping, rhs: PBDirectoryEvent.Ping) -> Bool {
    if lhs.time != rhs.time {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBCallDirectoryServerRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "CallDirectoryServerRequest"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .standard(proto: "network_key"),
    2: .same(proto: "listing"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.networkKey)
      case 2:
        var v: PBCallDirectoryServerRequest.RemoveListing?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .listing(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .listing(v)}
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.networkKey.isEmpty {
      try visitor.visitSingularBytesField(value: self.networkKey, fieldNumber: 1)
    }
    if case .listing(let v)? = self.body {
      try visitor.visitSingularMessageField(value: v, fieldNumber: 2)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBCallDirectoryServerRequest, rhs: PBCallDirectoryServerRequest) -> Bool {
    if lhs.networkKey != rhs.networkKey {return false}
    if lhs.body != rhs.body {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBCallDirectoryServerRequest.RemoveListing: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBCallDirectoryServerRequest.protoMessageName + ".RemoveListing"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBCallDirectoryServerRequest.RemoveListing, rhs: PBCallDirectoryServerRequest.RemoveListing) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBOpenDirectoryClientRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "OpenDirectoryClientRequest"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .standard(proto: "network_key"),
    2: .standard(proto: "server_key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.networkKey)
      case 2: try decoder.decodeSingularBytesField(value: &self.serverKey)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.networkKey.isEmpty {
      try visitor.visitSingularBytesField(value: self.networkKey, fieldNumber: 1)
    }
    if !self.serverKey.isEmpty {
      try visitor.visitSingularBytesField(value: self.serverKey, fieldNumber: 2)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBOpenDirectoryClientRequest, rhs: PBOpenDirectoryClientRequest) -> Bool {
    if lhs.networkKey != rhs.networkKey {return false}
    if lhs.serverKey != rhs.serverKey {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryClientEvent: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "DirectoryClientEvent"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "publish"),
    2: .same(proto: "unpublish"),
    3: .same(proto: "join"),
    4: .same(proto: "part"),
    5: .same(proto: "ping"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1:
        var v: PBDirectoryClientEvent.Publish?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .publish(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .publish(v)}
      case 2:
        var v: PBDirectoryClientEvent.Unpublish?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .unpublish(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .unpublish(v)}
      case 3:
        var v: PBDirectoryClientEvent.Join?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .join(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .join(v)}
      case 4:
        var v: PBDirectoryClientEvent.Part?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .part(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .part(v)}
      case 5:
        var v: PBDirectoryClientEvent.Ping?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .ping(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .ping(v)}
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    switch self.body {
    case .publish(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 1)
    case .unpublish(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 2)
    case .join(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 3)
    case .part(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 4)
    case .ping(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 5)
    case nil: break
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryClientEvent, rhs: PBDirectoryClientEvent) -> Bool {
    if lhs.body != rhs.body {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryClientEvent.Publish: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryClientEvent.protoMessageName + ".Publish"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "listing"),
    2: .same(proto: "signature"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularMessageField(value: &self._listing)
      case 2: try decoder.decodeSingularBytesField(value: &self.signature)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if let v = self._listing {
      try visitor.visitSingularMessageField(value: v, fieldNumber: 1)
    }
    if !self.signature.isEmpty {
      try visitor.visitSingularBytesField(value: self.signature, fieldNumber: 2)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryClientEvent.Publish, rhs: PBDirectoryClientEvent.Publish) -> Bool {
    if lhs._listing != rhs._listing {return false}
    if lhs.signature != rhs.signature {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryClientEvent.Unpublish: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryClientEvent.protoMessageName + ".Unpublish"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryClientEvent.Unpublish, rhs: PBDirectoryClientEvent.Unpublish) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryClientEvent.Join: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryClientEvent.protoMessageName + ".Join"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryClientEvent.Join, rhs: PBDirectoryClientEvent.Join) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryClientEvent.Part: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryClientEvent.protoMessageName + ".Part"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryClientEvent.Part, rhs: PBDirectoryClientEvent.Part) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBDirectoryClientEvent.Ping: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBDirectoryClientEvent.protoMessageName + ".Ping"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "time"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularInt64Field(value: &self.time)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if self.time != 0 {
      try visitor.visitSingularInt64Field(value: self.time, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBDirectoryClientEvent.Ping, rhs: PBDirectoryClientEvent.Ping) -> Bool {
    if lhs.time != rhs.time {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}
