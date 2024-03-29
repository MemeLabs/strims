// DO NOT EDIT.
// swift-format-ignore-file
//
// Generated by the Swift generator plugin for the protocol buffer compiler.
// Source: nickserv.proto
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

public struct PBServerConfig {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var key: PBKey {
    get {return _key ?? PBKey()}
    set {_key = newValue}
  }
  /// Returns true if `key` has been explicitly set.
  public var hasKey: Bool {return self._key != nil}
  /// Clears the value of `key`. Subsequent reads from it will return its default value.
  public mutating func clearKey() {self._key = nil}

  public var nameChangeQuota: UInt32 = 0

  public var tokenTtl: SwiftProtobuf.Google_Protobuf_Duration {
    get {return _tokenTtl ?? SwiftProtobuf.Google_Protobuf_Duration()}
    set {_tokenTtl = newValue}
  }
  /// Returns true if `tokenTtl` has been explicitly set.
  public var hasTokenTtl: Bool {return self._tokenTtl != nil}
  /// Clears the value of `tokenTtl`. Subsequent reads from it will return its default value.
  public mutating func clearTokenTtl() {self._tokenTtl = nil}

  public var roles: [String] = []

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}

  fileprivate var _key: PBKey? = nil
  fileprivate var _tokenTtl: SwiftProtobuf.Google_Protobuf_Duration? = nil
}

public struct PBNickservNick {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var id: UInt64 = 0

  public var key: Data = Data()

  public var nick: String = String()

  public var remainingNameChangeQuota: UInt32 = 0

  public var updatedTimestamp: UInt64 = 0

  public var createdTimestamp: UInt64 = 0

  public var roles: [String] = []

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}
}

public struct PBNickServToken {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var key: Data = Data()

  public var nick: String = String()

  public var validUntil: SwiftProtobuf.Google_Protobuf_Timestamp {
    get {return _validUntil ?? SwiftProtobuf.Google_Protobuf_Timestamp()}
    set {_validUntil = newValue}
  }
  /// Returns true if `validUntil` has been explicitly set.
  public var hasValidUntil: Bool {return self._validUntil != nil}
  /// Clears the value of `validUntil`. Subsequent reads from it will return its default value.
  public mutating func clearValidUntil() {self._validUntil = nil}

  public var signature: Data = Data()

  public var roles: [String] = []

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public init() {}

  fileprivate var _validUntil: SwiftProtobuf.Google_Protobuf_Timestamp? = nil
}

public struct PBNickServRPCCommand {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var requestID: UInt64 = 0

  public var sourcePublicKey: Data = Data()

  public var body: PBNickServRPCCommand.OneOf_Body? = nil

  public var create: PBNickServRPCCommand.Create {
    get {
      if case .create(let v)? = body {return v}
      return PBNickServRPCCommand.Create()
    }
    set {body = .create(newValue)}
  }

  public var retrieve: PBNickServRPCCommand.Retrieve {
    get {
      if case .retrieve(let v)? = body {return v}
      return PBNickServRPCCommand.Retrieve()
    }
    set {body = .retrieve(newValue)}
  }

  public var update: PBNickServRPCCommand.Update {
    get {
      if case .update(let v)? = body {return v}
      return PBNickServRPCCommand.Update()
    }
    set {body = .update(newValue)}
  }

  public var delete: PBNickServRPCCommand.Delete {
    get {
      if case .delete(let v)? = body {return v}
      return PBNickServRPCCommand.Delete()
    }
    set {body = .delete(newValue)}
  }

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public enum OneOf_Body: Equatable {
    case create(PBNickServRPCCommand.Create)
    case retrieve(PBNickServRPCCommand.Retrieve)
    case update(PBNickServRPCCommand.Update)
    case delete(PBNickServRPCCommand.Delete)

  #if !swift(>=4.1)
    public static func ==(lhs: PBNickServRPCCommand.OneOf_Body, rhs: PBNickServRPCCommand.OneOf_Body) -> Bool {
      switch (lhs, rhs) {
      case (.create(let l), .create(let r)): return l == r
      case (.retrieve(let l), .retrieve(let r)): return l == r
      case (.update(let l), .update(let r)): return l == r
      case (.delete(let l), .delete(let r)): return l == r
      default: return false
      }
    }
  #endif
  }

  public struct Create {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var nick: String = String()

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct Retrieve {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct Update {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var param: PBNickServRPCCommand.Update.OneOf_Param? = nil

    public var nick: PBNickServRPCCommand.Update.ChangeNick {
      get {
        if case .nick(let v)? = param {return v}
        return PBNickServRPCCommand.Update.ChangeNick()
      }
      set {param = .nick(newValue)}
    }

    public var nameChangeQuota: UInt32 {
      get {
        if case .nameChangeQuota(let v)? = param {return v}
        return 0
      }
      set {param = .nameChangeQuota(newValue)}
    }

    public var roles: PBNickServRPCCommand.Update.Roles {
      get {
        if case .roles(let v)? = param {return v}
        return PBNickServRPCCommand.Update.Roles()
      }
      set {param = .roles(newValue)}
    }

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public enum OneOf_Param: Equatable {
      case nick(PBNickServRPCCommand.Update.ChangeNick)
      case nameChangeQuota(UInt32)
      case roles(PBNickServRPCCommand.Update.Roles)

    #if !swift(>=4.1)
      public static func ==(lhs: PBNickServRPCCommand.Update.OneOf_Param, rhs: PBNickServRPCCommand.Update.OneOf_Param) -> Bool {
        switch (lhs, rhs) {
        case (.nick(let l), .nick(let r)): return l == r
        case (.nameChangeQuota(let l), .nameChangeQuota(let r)): return l == r
        case (.roles(let l), .roles(let r)): return l == r
        default: return false
        }
      }
    #endif
    }

    public struct Roles {
      // SwiftProtobuf.Message conformance is added in an extension below. See the
      // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
      // methods supported on all messages.

      public var roles: [String] = []

      public var unknownFields = SwiftProtobuf.UnknownStorage()

      public init() {}
    }

    public struct ChangeNick {
      // SwiftProtobuf.Message conformance is added in an extension below. See the
      // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
      // methods supported on all messages.

      public var oldNick: String = String()

      public var newNick: String = String()

      public var unknownFields = SwiftProtobuf.UnknownStorage()

      public init() {}
    }

    public init() {}
  }

  public struct Delete {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public init() {}
}

public struct PBNickServRPCResponse {
  // SwiftProtobuf.Message conformance is added in an extension below. See the
  // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
  // methods supported on all messages.

  public var requestID: UInt64 = 0

  public var body: PBNickServRPCResponse.OneOf_Body? = nil

  public var error: String {
    get {
      if case .error(let v)? = body {return v}
      return String()
    }
    set {body = .error(newValue)}
  }

  public var update: PBNickServRPCResponse.Update {
    get {
      if case .update(let v)? = body {return v}
      return PBNickServRPCResponse.Update()
    }
    set {body = .update(newValue)}
  }

  public var delete: PBNickServRPCResponse.Delete {
    get {
      if case .delete(let v)? = body {return v}
      return PBNickServRPCResponse.Delete()
    }
    set {body = .delete(newValue)}
  }

  public var create: PBNickServToken {
    get {
      if case .create(let v)? = body {return v}
      return PBNickServToken()
    }
    set {body = .create(newValue)}
  }

  public var retrieve: PBNickServToken {
    get {
      if case .retrieve(let v)? = body {return v}
      return PBNickServToken()
    }
    set {body = .retrieve(newValue)}
  }

  public var unknownFields = SwiftProtobuf.UnknownStorage()

  public enum OneOf_Body: Equatable {
    case error(String)
    case update(PBNickServRPCResponse.Update)
    case delete(PBNickServRPCResponse.Delete)
    case create(PBNickServToken)
    case retrieve(PBNickServToken)

  #if !swift(>=4.1)
    public static func ==(lhs: PBNickServRPCResponse.OneOf_Body, rhs: PBNickServRPCResponse.OneOf_Body) -> Bool {
      switch (lhs, rhs) {
      case (.error(let l), .error(let r)): return l == r
      case (.update(let l), .update(let r)): return l == r
      case (.delete(let l), .delete(let r)): return l == r
      case (.create(let l), .create(let r)): return l == r
      case (.retrieve(let l), .retrieve(let r)): return l == r
      default: return false
      }
    }
  #endif
  }

  public struct Update {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public struct Delete {
    // SwiftProtobuf.Message conformance is added in an extension below. See the
    // `Message` and `Message+*Additions` files in the SwiftProtobuf library for
    // methods supported on all messages.

    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
  }

  public init() {}
}

// MARK: - Code below here is support for the SwiftProtobuf runtime.

extension PBServerConfig: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "ServerConfig"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
    2: .standard(proto: "name_change_quota"),
    3: .standard(proto: "token_ttl"),
    4: .same(proto: "roles"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularMessageField(value: &self._key)
      case 2: try decoder.decodeSingularUInt32Field(value: &self.nameChangeQuota)
      case 3: try decoder.decodeSingularMessageField(value: &self._tokenTtl)
      case 4: try decoder.decodeRepeatedStringField(value: &self.roles)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if let v = self._key {
      try visitor.visitSingularMessageField(value: v, fieldNumber: 1)
    }
    if self.nameChangeQuota != 0 {
      try visitor.visitSingularUInt32Field(value: self.nameChangeQuota, fieldNumber: 2)
    }
    if let v = self._tokenTtl {
      try visitor.visitSingularMessageField(value: v, fieldNumber: 3)
    }
    if !self.roles.isEmpty {
      try visitor.visitRepeatedStringField(value: self.roles, fieldNumber: 4)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBServerConfig, rhs: PBServerConfig) -> Bool {
    if lhs._key != rhs._key {return false}
    if lhs.nameChangeQuota != rhs.nameChangeQuota {return false}
    if lhs._tokenTtl != rhs._tokenTtl {return false}
    if lhs.roles != rhs.roles {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickservNick: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "NickservNick"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "id"),
    2: .same(proto: "key"),
    3: .same(proto: "nick"),
    4: .standard(proto: "remaining_name_change_quota"),
    5: .standard(proto: "updated_timestamp"),
    6: .standard(proto: "created_timestamp"),
    7: .same(proto: "roles"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularUInt64Field(value: &self.id)
      case 2: try decoder.decodeSingularBytesField(value: &self.key)
      case 3: try decoder.decodeSingularStringField(value: &self.nick)
      case 4: try decoder.decodeSingularUInt32Field(value: &self.remainingNameChangeQuota)
      case 5: try decoder.decodeSingularUInt64Field(value: &self.updatedTimestamp)
      case 6: try decoder.decodeSingularUInt64Field(value: &self.createdTimestamp)
      case 7: try decoder.decodeRepeatedStringField(value: &self.roles)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if self.id != 0 {
      try visitor.visitSingularUInt64Field(value: self.id, fieldNumber: 1)
    }
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 2)
    }
    if !self.nick.isEmpty {
      try visitor.visitSingularStringField(value: self.nick, fieldNumber: 3)
    }
    if self.remainingNameChangeQuota != 0 {
      try visitor.visitSingularUInt32Field(value: self.remainingNameChangeQuota, fieldNumber: 4)
    }
    if self.updatedTimestamp != 0 {
      try visitor.visitSingularUInt64Field(value: self.updatedTimestamp, fieldNumber: 5)
    }
    if self.createdTimestamp != 0 {
      try visitor.visitSingularUInt64Field(value: self.createdTimestamp, fieldNumber: 6)
    }
    if !self.roles.isEmpty {
      try visitor.visitRepeatedStringField(value: self.roles, fieldNumber: 7)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickservNick, rhs: PBNickservNick) -> Bool {
    if lhs.id != rhs.id {return false}
    if lhs.key != rhs.key {return false}
    if lhs.nick != rhs.nick {return false}
    if lhs.remainingNameChangeQuota != rhs.remainingNameChangeQuota {return false}
    if lhs.updatedTimestamp != rhs.updatedTimestamp {return false}
    if lhs.createdTimestamp != rhs.createdTimestamp {return false}
    if lhs.roles != rhs.roles {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServToken: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "NickServToken"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "key"),
    2: .same(proto: "nick"),
    3: .standard(proto: "valid_until"),
    4: .same(proto: "signature"),
    5: .same(proto: "roles"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularBytesField(value: &self.key)
      case 2: try decoder.decodeSingularStringField(value: &self.nick)
      case 3: try decoder.decodeSingularMessageField(value: &self._validUntil)
      case 4: try decoder.decodeSingularBytesField(value: &self.signature)
      case 5: try decoder.decodeRepeatedStringField(value: &self.roles)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.key.isEmpty {
      try visitor.visitSingularBytesField(value: self.key, fieldNumber: 1)
    }
    if !self.nick.isEmpty {
      try visitor.visitSingularStringField(value: self.nick, fieldNumber: 2)
    }
    if let v = self._validUntil {
      try visitor.visitSingularMessageField(value: v, fieldNumber: 3)
    }
    if !self.signature.isEmpty {
      try visitor.visitSingularBytesField(value: self.signature, fieldNumber: 4)
    }
    if !self.roles.isEmpty {
      try visitor.visitRepeatedStringField(value: self.roles, fieldNumber: 5)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServToken, rhs: PBNickServToken) -> Bool {
    if lhs.key != rhs.key {return false}
    if lhs.nick != rhs.nick {return false}
    if lhs._validUntil != rhs._validUntil {return false}
    if lhs.signature != rhs.signature {return false}
    if lhs.roles != rhs.roles {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCCommand: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "NickServRPCCommand"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .standard(proto: "request_id"),
    2: .standard(proto: "source_public_key"),
    5: .same(proto: "create"),
    6: .same(proto: "retrieve"),
    7: .same(proto: "update"),
    8: .same(proto: "delete"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularUInt64Field(value: &self.requestID)
      case 2: try decoder.decodeSingularBytesField(value: &self.sourcePublicKey)
      case 5:
        var v: PBNickServRPCCommand.Create?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .create(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .create(v)}
      case 6:
        var v: PBNickServRPCCommand.Retrieve?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .retrieve(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .retrieve(v)}
      case 7:
        var v: PBNickServRPCCommand.Update?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .update(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .update(v)}
      case 8:
        var v: PBNickServRPCCommand.Delete?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .delete(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .delete(v)}
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if self.requestID != 0 {
      try visitor.visitSingularUInt64Field(value: self.requestID, fieldNumber: 1)
    }
    if !self.sourcePublicKey.isEmpty {
      try visitor.visitSingularBytesField(value: self.sourcePublicKey, fieldNumber: 2)
    }
    switch self.body {
    case .create(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 5)
    case .retrieve(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 6)
    case .update(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 7)
    case .delete(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 8)
    case nil: break
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCCommand, rhs: PBNickServRPCCommand) -> Bool {
    if lhs.requestID != rhs.requestID {return false}
    if lhs.sourcePublicKey != rhs.sourcePublicKey {return false}
    if lhs.body != rhs.body {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCCommand.Create: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCCommand.protoMessageName + ".Create"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "nick"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularStringField(value: &self.nick)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.nick.isEmpty {
      try visitor.visitSingularStringField(value: self.nick, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCCommand.Create, rhs: PBNickServRPCCommand.Create) -> Bool {
    if lhs.nick != rhs.nick {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCCommand.Retrieve: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCCommand.protoMessageName + ".Retrieve"
  public static let _protobuf_nameMap = SwiftProtobuf._NameMap()

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let _ = try decoder.nextFieldNumber() {
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCCommand.Retrieve, rhs: PBNickServRPCCommand.Retrieve) -> Bool {
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCCommand.Update: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCCommand.protoMessageName + ".Update"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "nick"),
    2: .standard(proto: "name_change_quota"),
    3: .same(proto: "roles"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1:
        var v: PBNickServRPCCommand.Update.ChangeNick?
        if let current = self.param {
          try decoder.handleConflictingOneOf()
          if case .nick(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.param = .nick(v)}
      case 2:
        if self.param != nil {try decoder.handleConflictingOneOf()}
        var v: UInt32?
        try decoder.decodeSingularUInt32Field(value: &v)
        if let v = v {self.param = .nameChangeQuota(v)}
      case 3:
        var v: PBNickServRPCCommand.Update.Roles?
        if let current = self.param {
          try decoder.handleConflictingOneOf()
          if case .roles(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.param = .roles(v)}
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    switch self.param {
    case .nick(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 1)
    case .nameChangeQuota(let v)?:
      try visitor.visitSingularUInt32Field(value: v, fieldNumber: 2)
    case .roles(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 3)
    case nil: break
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCCommand.Update, rhs: PBNickServRPCCommand.Update) -> Bool {
    if lhs.param != rhs.param {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCCommand.Update.Roles: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCCommand.Update.protoMessageName + ".Roles"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .same(proto: "roles"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeRepeatedStringField(value: &self.roles)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.roles.isEmpty {
      try visitor.visitRepeatedStringField(value: self.roles, fieldNumber: 1)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCCommand.Update.Roles, rhs: PBNickServRPCCommand.Update.Roles) -> Bool {
    if lhs.roles != rhs.roles {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCCommand.Update.ChangeNick: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCCommand.Update.protoMessageName + ".ChangeNick"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .standard(proto: "old_nick"),
    2: .standard(proto: "new_nick"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularStringField(value: &self.oldNick)
      case 2: try decoder.decodeSingularStringField(value: &self.newNick)
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if !self.oldNick.isEmpty {
      try visitor.visitSingularStringField(value: self.oldNick, fieldNumber: 1)
    }
    if !self.newNick.isEmpty {
      try visitor.visitSingularStringField(value: self.newNick, fieldNumber: 2)
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCCommand.Update.ChangeNick, rhs: PBNickServRPCCommand.Update.ChangeNick) -> Bool {
    if lhs.oldNick != rhs.oldNick {return false}
    if lhs.newNick != rhs.newNick {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCCommand.Delete: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCCommand.protoMessageName + ".Delete"
  public static let _protobuf_nameMap = SwiftProtobuf._NameMap()

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let _ = try decoder.nextFieldNumber() {
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCCommand.Delete, rhs: PBNickServRPCCommand.Delete) -> Bool {
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCResponse: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = "NickServRPCResponse"
  public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
    1: .standard(proto: "request_id"),
    2: .same(proto: "error"),
    3: .same(proto: "update"),
    4: .same(proto: "delete"),
    5: .same(proto: "create"),
    6: .same(proto: "retrieve"),
  ]

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let fieldNumber = try decoder.nextFieldNumber() {
      switch fieldNumber {
      case 1: try decoder.decodeSingularUInt64Field(value: &self.requestID)
      case 2:
        if self.body != nil {try decoder.handleConflictingOneOf()}
        var v: String?
        try decoder.decodeSingularStringField(value: &v)
        if let v = v {self.body = .error(v)}
      case 3:
        var v: PBNickServRPCResponse.Update?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .update(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .update(v)}
      case 4:
        var v: PBNickServRPCResponse.Delete?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .delete(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .delete(v)}
      case 5:
        var v: PBNickServToken?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .create(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .create(v)}
      case 6:
        var v: PBNickServToken?
        if let current = self.body {
          try decoder.handleConflictingOneOf()
          if case .retrieve(let m) = current {v = m}
        }
        try decoder.decodeSingularMessageField(value: &v)
        if let v = v {self.body = .retrieve(v)}
      default: break
      }
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    if self.requestID != 0 {
      try visitor.visitSingularUInt64Field(value: self.requestID, fieldNumber: 1)
    }
    switch self.body {
    case .error(let v)?:
      try visitor.visitSingularStringField(value: v, fieldNumber: 2)
    case .update(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 3)
    case .delete(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 4)
    case .create(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 5)
    case .retrieve(let v)?:
      try visitor.visitSingularMessageField(value: v, fieldNumber: 6)
    case nil: break
    }
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCResponse, rhs: PBNickServRPCResponse) -> Bool {
    if lhs.requestID != rhs.requestID {return false}
    if lhs.body != rhs.body {return false}
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCResponse.Update: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCResponse.protoMessageName + ".Update"
  public static let _protobuf_nameMap = SwiftProtobuf._NameMap()

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let _ = try decoder.nextFieldNumber() {
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCResponse.Update, rhs: PBNickServRPCResponse.Update) -> Bool {
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}

extension PBNickServRPCResponse.Delete: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
  public static let protoMessageName: String = PBNickServRPCResponse.protoMessageName + ".Delete"
  public static let _protobuf_nameMap = SwiftProtobuf._NameMap()

  public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
    while let _ = try decoder.nextFieldNumber() {
    }
  }

  public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
    try unknownFields.traverse(visitor: &visitor)
  }

  public static func ==(lhs: PBNickServRPCResponse.Delete, rhs: PBNickServRPCResponse.Delete) -> Bool {
    if lhs.unknownFields != rhs.unknownFields {return false}
    return true
  }
}
