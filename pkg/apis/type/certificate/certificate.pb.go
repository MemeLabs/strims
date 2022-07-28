// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.28.1
// 	protoc        v3.20.1
// source: type/certificate.proto

package certificate

import (
	key "github.com/MemeLabs/strims/pkg/apis/type/key"
	protoreflect "google.golang.org/protobuf/reflect/protoreflect"
	protoimpl "google.golang.org/protobuf/runtime/protoimpl"
	reflect "reflect"
	sync "sync"
)

const (
	// Verify that this generated code is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(20 - protoimpl.MinVersion)
	// Verify that runtime/protoimpl is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(protoimpl.MaxVersion - 20)
)

type KeyUsage int32

const (
	KeyUsage_KEY_USAGE_UNDEFINED KeyUsage = 0
	// PEER allows bearer to connect with members of the signator's network
	KeyUsage_KEY_USAGE_PEER KeyUsage = 1
	// BOOTSTRAP allows the bearer to connect to a network's signators. Invites
	// including transient keys with bootstrap certs allow new members to request
	// peer certs.
	KeyUsage_KEY_USAGE_BOOTSTRAP KeyUsage = 2
	// SIGN allows the bearer to sign certificates.
	KeyUsage_KEY_USAGE_SIGN KeyUsage = 4
	// BROKER allows the bearer to negotiate connections between a network's
	// members.
	KeyUsage_KEY_USAGE_BROKER KeyUsage = 8
	// ENCIPHERMENT allows the key to be used for encrypting messages.
	KeyUsage_KEY_USAGE_ENCIPHERMENT KeyUsage = 16
)

// Enum value maps for KeyUsage.
var (
	KeyUsage_name = map[int32]string{
		0:  "KEY_USAGE_UNDEFINED",
		1:  "KEY_USAGE_PEER",
		2:  "KEY_USAGE_BOOTSTRAP",
		4:  "KEY_USAGE_SIGN",
		8:  "KEY_USAGE_BROKER",
		16: "KEY_USAGE_ENCIPHERMENT",
	}
	KeyUsage_value = map[string]int32{
		"KEY_USAGE_UNDEFINED":    0,
		"KEY_USAGE_PEER":         1,
		"KEY_USAGE_BOOTSTRAP":    2,
		"KEY_USAGE_SIGN":         4,
		"KEY_USAGE_BROKER":       8,
		"KEY_USAGE_ENCIPHERMENT": 16,
	}
)

func (x KeyUsage) Enum() *KeyUsage {
	p := new(KeyUsage)
	*p = x
	return p
}

func (x KeyUsage) String() string {
	return protoimpl.X.EnumStringOf(x.Descriptor(), protoreflect.EnumNumber(x))
}

func (KeyUsage) Descriptor() protoreflect.EnumDescriptor {
	return file_type_certificate_proto_enumTypes[0].Descriptor()
}

func (KeyUsage) Type() protoreflect.EnumType {
	return &file_type_certificate_proto_enumTypes[0]
}

func (x KeyUsage) Number() protoreflect.EnumNumber {
	return protoreflect.EnumNumber(x)
}

// Deprecated: Use KeyUsage.Descriptor instead.
func (KeyUsage) EnumDescriptor() ([]byte, []int) {
	return file_type_certificate_proto_rawDescGZIP(), []int{0}
}

type CertificateRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Key       []byte      `protobuf:"bytes,1,opt,name=key,proto3" json:"key,omitempty"`
	KeyType   key.KeyType `protobuf:"varint,2,opt,name=key_type,json=keyType,proto3,enum=strims.type.KeyType" json:"key_type,omitempty"`
	KeyUsage  KeyUsage    `protobuf:"varint,3,opt,name=key_usage,json=keyUsage,proto3,enum=strims.type.KeyUsage" json:"key_usage,omitempty"`
	Subject   string      `protobuf:"bytes,5,opt,name=subject,proto3" json:"subject,omitempty"`
	Signature []byte      `protobuf:"bytes,4,opt,name=signature,proto3" json:"signature,omitempty"`
}

func (x *CertificateRequest) Reset() {
	*x = CertificateRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_type_certificate_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *CertificateRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*CertificateRequest) ProtoMessage() {}

func (x *CertificateRequest) ProtoReflect() protoreflect.Message {
	mi := &file_type_certificate_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use CertificateRequest.ProtoReflect.Descriptor instead.
func (*CertificateRequest) Descriptor() ([]byte, []int) {
	return file_type_certificate_proto_rawDescGZIP(), []int{0}
}

func (x *CertificateRequest) GetKey() []byte {
	if x != nil {
		return x.Key
	}
	return nil
}

func (x *CertificateRequest) GetKeyType() key.KeyType {
	if x != nil {
		return x.KeyType
	}
	return key.KeyType(0)
}

func (x *CertificateRequest) GetKeyUsage() KeyUsage {
	if x != nil {
		return x.KeyUsage
	}
	return KeyUsage_KEY_USAGE_UNDEFINED
}

func (x *CertificateRequest) GetSubject() string {
	if x != nil {
		return x.Subject
	}
	return ""
}

func (x *CertificateRequest) GetSignature() []byte {
	if x != nil {
		return x.Signature
	}
	return nil
}

type Certificate struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Key          []byte      `protobuf:"bytes,1,opt,name=key,proto3" json:"key,omitempty"`
	KeyType      key.KeyType `protobuf:"varint,2,opt,name=key_type,json=keyType,proto3,enum=strims.type.KeyType" json:"key_type,omitempty"`
	KeyUsage     KeyUsage    `protobuf:"varint,3,opt,name=key_usage,json=keyUsage,proto3,enum=strims.type.KeyUsage" json:"key_usage,omitempty"`
	Subject      string      `protobuf:"bytes,4,opt,name=subject,proto3" json:"subject,omitempty"`
	NotBefore    uint64      `protobuf:"varint,5,opt,name=not_before,json=notBefore,proto3" json:"not_before,omitempty"`
	NotAfter     uint64      `protobuf:"varint,6,opt,name=not_after,json=notAfter,proto3" json:"not_after,omitempty"`
	SerialNumber []byte      `protobuf:"bytes,7,opt,name=serial_number,json=serialNumber,proto3" json:"serial_number,omitempty"`
	Signature    []byte      `protobuf:"bytes,8,opt,name=signature,proto3" json:"signature,omitempty"`
	// Types that are assignable to ParentOneof:
	//	*Certificate_Parent
	//	*Certificate_ParentSerialNumber
	ParentOneof isCertificate_ParentOneof `protobuf_oneof:"parent_oneof"`
}

func (x *Certificate) Reset() {
	*x = Certificate{}
	if protoimpl.UnsafeEnabled {
		mi := &file_type_certificate_proto_msgTypes[1]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *Certificate) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*Certificate) ProtoMessage() {}

func (x *Certificate) ProtoReflect() protoreflect.Message {
	mi := &file_type_certificate_proto_msgTypes[1]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use Certificate.ProtoReflect.Descriptor instead.
func (*Certificate) Descriptor() ([]byte, []int) {
	return file_type_certificate_proto_rawDescGZIP(), []int{1}
}

func (x *Certificate) GetKey() []byte {
	if x != nil {
		return x.Key
	}
	return nil
}

func (x *Certificate) GetKeyType() key.KeyType {
	if x != nil {
		return x.KeyType
	}
	return key.KeyType(0)
}

func (x *Certificate) GetKeyUsage() KeyUsage {
	if x != nil {
		return x.KeyUsage
	}
	return KeyUsage_KEY_USAGE_UNDEFINED
}

func (x *Certificate) GetSubject() string {
	if x != nil {
		return x.Subject
	}
	return ""
}

func (x *Certificate) GetNotBefore() uint64 {
	if x != nil {
		return x.NotBefore
	}
	return 0
}

func (x *Certificate) GetNotAfter() uint64 {
	if x != nil {
		return x.NotAfter
	}
	return 0
}

func (x *Certificate) GetSerialNumber() []byte {
	if x != nil {
		return x.SerialNumber
	}
	return nil
}

func (x *Certificate) GetSignature() []byte {
	if x != nil {
		return x.Signature
	}
	return nil
}

func (m *Certificate) GetParentOneof() isCertificate_ParentOneof {
	if m != nil {
		return m.ParentOneof
	}
	return nil
}

func (x *Certificate) GetParent() *Certificate {
	if x, ok := x.GetParentOneof().(*Certificate_Parent); ok {
		return x.Parent
	}
	return nil
}

func (x *Certificate) GetParentSerialNumber() []byte {
	if x, ok := x.GetParentOneof().(*Certificate_ParentSerialNumber); ok {
		return x.ParentSerialNumber
	}
	return nil
}

type isCertificate_ParentOneof interface {
	isCertificate_ParentOneof()
}

type Certificate_Parent struct {
	Parent *Certificate `protobuf:"bytes,9,opt,name=parent,proto3,oneof"`
}

type Certificate_ParentSerialNumber struct {
	ParentSerialNumber []byte `protobuf:"bytes,10,opt,name=parent_serial_number,json=parentSerialNumber,proto3,oneof"`
}

func (*Certificate_Parent) isCertificate_ParentOneof() {}

func (*Certificate_ParentSerialNumber) isCertificate_ParentOneof() {}

var File_type_certificate_proto protoreflect.FileDescriptor

var file_type_certificate_proto_rawDesc = []byte{
	0x0a, 0x16, 0x74, 0x79, 0x70, 0x65, 0x2f, 0x63, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61,
	0x74, 0x65, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x12, 0x0b, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73,
	0x2e, 0x74, 0x79, 0x70, 0x65, 0x1a, 0x0e, 0x74, 0x79, 0x70, 0x65, 0x2f, 0x6b, 0x65, 0x79, 0x2e,
	0x70, 0x72, 0x6f, 0x74, 0x6f, 0x22, 0xc3, 0x01, 0x0a, 0x12, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66,
	0x69, 0x63, 0x61, 0x74, 0x65, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x10, 0x0a, 0x03,
	0x6b, 0x65, 0x79, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x03, 0x6b, 0x65, 0x79, 0x12, 0x2f,
	0x0a, 0x08, 0x6b, 0x65, 0x79, 0x5f, 0x74, 0x79, 0x70, 0x65, 0x18, 0x02, 0x20, 0x01, 0x28, 0x0e,
	0x32, 0x14, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x2e, 0x4b,
	0x65, 0x79, 0x54, 0x79, 0x70, 0x65, 0x52, 0x07, 0x6b, 0x65, 0x79, 0x54, 0x79, 0x70, 0x65, 0x12,
	0x32, 0x0a, 0x09, 0x6b, 0x65, 0x79, 0x5f, 0x75, 0x73, 0x61, 0x67, 0x65, 0x18, 0x03, 0x20, 0x01,
	0x28, 0x0e, 0x32, 0x15, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65,
	0x2e, 0x4b, 0x65, 0x79, 0x55, 0x73, 0x61, 0x67, 0x65, 0x52, 0x08, 0x6b, 0x65, 0x79, 0x55, 0x73,
	0x61, 0x67, 0x65, 0x12, 0x18, 0x0a, 0x07, 0x73, 0x75, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x18, 0x05,
	0x20, 0x01, 0x28, 0x09, 0x52, 0x07, 0x73, 0x75, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x12, 0x1c, 0x0a,
	0x09, 0x73, 0x69, 0x67, 0x6e, 0x61, 0x74, 0x75, 0x72, 0x65, 0x18, 0x04, 0x20, 0x01, 0x28, 0x0c,
	0x52, 0x09, 0x73, 0x69, 0x67, 0x6e, 0x61, 0x74, 0x75, 0x72, 0x65, 0x22, 0x95, 0x03, 0x0a, 0x0b,
	0x43, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x12, 0x10, 0x0a, 0x03, 0x6b,
	0x65, 0x79, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x03, 0x6b, 0x65, 0x79, 0x12, 0x2f, 0x0a,
	0x08, 0x6b, 0x65, 0x79, 0x5f, 0x74, 0x79, 0x70, 0x65, 0x18, 0x02, 0x20, 0x01, 0x28, 0x0e, 0x32,
	0x14, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x2e, 0x4b, 0x65,
	0x79, 0x54, 0x79, 0x70, 0x65, 0x52, 0x07, 0x6b, 0x65, 0x79, 0x54, 0x79, 0x70, 0x65, 0x12, 0x32,
	0x0a, 0x09, 0x6b, 0x65, 0x79, 0x5f, 0x75, 0x73, 0x61, 0x67, 0x65, 0x18, 0x03, 0x20, 0x01, 0x28,
	0x0e, 0x32, 0x15, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x2e,
	0x4b, 0x65, 0x79, 0x55, 0x73, 0x61, 0x67, 0x65, 0x52, 0x08, 0x6b, 0x65, 0x79, 0x55, 0x73, 0x61,
	0x67, 0x65, 0x12, 0x18, 0x0a, 0x07, 0x73, 0x75, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x18, 0x04, 0x20,
	0x01, 0x28, 0x09, 0x52, 0x07, 0x73, 0x75, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x12, 0x1d, 0x0a, 0x0a,
	0x6e, 0x6f, 0x74, 0x5f, 0x62, 0x65, 0x66, 0x6f, 0x72, 0x65, 0x18, 0x05, 0x20, 0x01, 0x28, 0x04,
	0x52, 0x09, 0x6e, 0x6f, 0x74, 0x42, 0x65, 0x66, 0x6f, 0x72, 0x65, 0x12, 0x1b, 0x0a, 0x09, 0x6e,
	0x6f, 0x74, 0x5f, 0x61, 0x66, 0x74, 0x65, 0x72, 0x18, 0x06, 0x20, 0x01, 0x28, 0x04, 0x52, 0x08,
	0x6e, 0x6f, 0x74, 0x41, 0x66, 0x74, 0x65, 0x72, 0x12, 0x23, 0x0a, 0x0d, 0x73, 0x65, 0x72, 0x69,
	0x61, 0x6c, 0x5f, 0x6e, 0x75, 0x6d, 0x62, 0x65, 0x72, 0x18, 0x07, 0x20, 0x01, 0x28, 0x0c, 0x52,
	0x0c, 0x73, 0x65, 0x72, 0x69, 0x61, 0x6c, 0x4e, 0x75, 0x6d, 0x62, 0x65, 0x72, 0x12, 0x1c, 0x0a,
	0x09, 0x73, 0x69, 0x67, 0x6e, 0x61, 0x74, 0x75, 0x72, 0x65, 0x18, 0x08, 0x20, 0x01, 0x28, 0x0c,
	0x52, 0x09, 0x73, 0x69, 0x67, 0x6e, 0x61, 0x74, 0x75, 0x72, 0x65, 0x12, 0x32, 0x0a, 0x06, 0x70,
	0x61, 0x72, 0x65, 0x6e, 0x74, 0x18, 0x09, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x18, 0x2e, 0x73, 0x74,
	0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x2e, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66,
	0x69, 0x63, 0x61, 0x74, 0x65, 0x48, 0x00, 0x52, 0x06, 0x70, 0x61, 0x72, 0x65, 0x6e, 0x74, 0x12,
	0x32, 0x0a, 0x14, 0x70, 0x61, 0x72, 0x65, 0x6e, 0x74, 0x5f, 0x73, 0x65, 0x72, 0x69, 0x61, 0x6c,
	0x5f, 0x6e, 0x75, 0x6d, 0x62, 0x65, 0x72, 0x18, 0x0a, 0x20, 0x01, 0x28, 0x0c, 0x48, 0x00, 0x52,
	0x12, 0x70, 0x61, 0x72, 0x65, 0x6e, 0x74, 0x53, 0x65, 0x72, 0x69, 0x61, 0x6c, 0x4e, 0x75, 0x6d,
	0x62, 0x65, 0x72, 0x42, 0x0e, 0x0a, 0x0c, 0x70, 0x61, 0x72, 0x65, 0x6e, 0x74, 0x5f, 0x6f, 0x6e,
	0x65, 0x6f, 0x66, 0x2a, 0x96, 0x01, 0x0a, 0x08, 0x4b, 0x65, 0x79, 0x55, 0x73, 0x61, 0x67, 0x65,
	0x12, 0x17, 0x0a, 0x13, 0x4b, 0x45, 0x59, 0x5f, 0x55, 0x53, 0x41, 0x47, 0x45, 0x5f, 0x55, 0x4e,
	0x44, 0x45, 0x46, 0x49, 0x4e, 0x45, 0x44, 0x10, 0x00, 0x12, 0x12, 0x0a, 0x0e, 0x4b, 0x45, 0x59,
	0x5f, 0x55, 0x53, 0x41, 0x47, 0x45, 0x5f, 0x50, 0x45, 0x45, 0x52, 0x10, 0x01, 0x12, 0x17, 0x0a,
	0x13, 0x4b, 0x45, 0x59, 0x5f, 0x55, 0x53, 0x41, 0x47, 0x45, 0x5f, 0x42, 0x4f, 0x4f, 0x54, 0x53,
	0x54, 0x52, 0x41, 0x50, 0x10, 0x02, 0x12, 0x12, 0x0a, 0x0e, 0x4b, 0x45, 0x59, 0x5f, 0x55, 0x53,
	0x41, 0x47, 0x45, 0x5f, 0x53, 0x49, 0x47, 0x4e, 0x10, 0x04, 0x12, 0x14, 0x0a, 0x10, 0x4b, 0x45,
	0x59, 0x5f, 0x55, 0x53, 0x41, 0x47, 0x45, 0x5f, 0x42, 0x52, 0x4f, 0x4b, 0x45, 0x52, 0x10, 0x08,
	0x12, 0x1a, 0x0a, 0x16, 0x4b, 0x45, 0x59, 0x5f, 0x55, 0x53, 0x41, 0x47, 0x45, 0x5f, 0x45, 0x4e,
	0x43, 0x49, 0x50, 0x48, 0x45, 0x52, 0x4d, 0x45, 0x4e, 0x54, 0x10, 0x10, 0x42, 0x58, 0x0a, 0x0e,
	0x67, 0x67, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x5a, 0x40,
	0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x4d, 0x65, 0x6d, 0x65, 0x4c,
	0x61, 0x62, 0x73, 0x2f, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2f, 0x70, 0x6b, 0x67, 0x2f, 0x61,
	0x70, 0x69, 0x73, 0x2f, 0x74, 0x79, 0x70, 0x65, 0x2f, 0x63, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69,
	0x63, 0x61, 0x74, 0x65, 0x3b, 0x63, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65,
	0xba, 0x02, 0x03, 0x53, 0x54, 0x50, 0x62, 0x06, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_type_certificate_proto_rawDescOnce sync.Once
	file_type_certificate_proto_rawDescData = file_type_certificate_proto_rawDesc
)

func file_type_certificate_proto_rawDescGZIP() []byte {
	file_type_certificate_proto_rawDescOnce.Do(func() {
		file_type_certificate_proto_rawDescData = protoimpl.X.CompressGZIP(file_type_certificate_proto_rawDescData)
	})
	return file_type_certificate_proto_rawDescData
}

var file_type_certificate_proto_enumTypes = make([]protoimpl.EnumInfo, 1)
var file_type_certificate_proto_msgTypes = make([]protoimpl.MessageInfo, 2)
var file_type_certificate_proto_goTypes = []interface{}{
	(KeyUsage)(0),              // 0: strims.type.KeyUsage
	(*CertificateRequest)(nil), // 1: strims.type.CertificateRequest
	(*Certificate)(nil),        // 2: strims.type.Certificate
	(key.KeyType)(0),           // 3: strims.type.KeyType
}
var file_type_certificate_proto_depIdxs = []int32{
	3, // 0: strims.type.CertificateRequest.key_type:type_name -> strims.type.KeyType
	0, // 1: strims.type.CertificateRequest.key_usage:type_name -> strims.type.KeyUsage
	3, // 2: strims.type.Certificate.key_type:type_name -> strims.type.KeyType
	0, // 3: strims.type.Certificate.key_usage:type_name -> strims.type.KeyUsage
	2, // 4: strims.type.Certificate.parent:type_name -> strims.type.Certificate
	5, // [5:5] is the sub-list for method output_type
	5, // [5:5] is the sub-list for method input_type
	5, // [5:5] is the sub-list for extension type_name
	5, // [5:5] is the sub-list for extension extendee
	0, // [0:5] is the sub-list for field type_name
}

func init() { file_type_certificate_proto_init() }
func file_type_certificate_proto_init() {
	if File_type_certificate_proto != nil {
		return
	}
	if !protoimpl.UnsafeEnabled {
		file_type_certificate_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*CertificateRequest); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_type_certificate_proto_msgTypes[1].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*Certificate); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
	}
	file_type_certificate_proto_msgTypes[1].OneofWrappers = []interface{}{
		(*Certificate_Parent)(nil),
		(*Certificate_ParentSerialNumber)(nil),
	}
	type x struct{}
	out := protoimpl.TypeBuilder{
		File: protoimpl.DescBuilder{
			GoPackagePath: reflect.TypeOf(x{}).PkgPath(),
			RawDescriptor: file_type_certificate_proto_rawDesc,
			NumEnums:      1,
			NumMessages:   2,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_type_certificate_proto_goTypes,
		DependencyIndexes: file_type_certificate_proto_depIdxs,
		EnumInfos:         file_type_certificate_proto_enumTypes,
		MessageInfos:      file_type_certificate_proto_msgTypes,
	}.Build()
	File_type_certificate_proto = out.File
	file_type_certificate_proto_rawDesc = nil
	file_type_certificate_proto_goTypes = nil
	file_type_certificate_proto_depIdxs = nil
}
