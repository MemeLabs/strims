// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.28.1
// 	protoc        v3.20.1
// source: network/v1/peer.proto

package networkv1

import (
	certificate "github.com/MemeLabs/strims/pkg/apis/type/certificate"
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

type NetworkPeerNegotiateRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	KeyCount uint32 `protobuf:"varint,1,opt,name=key_count,json=keyCount,proto3" json:"key_count,omitempty"`
}

func (x *NetworkPeerNegotiateRequest) Reset() {
	*x = NetworkPeerNegotiateRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerNegotiateRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerNegotiateRequest) ProtoMessage() {}

func (x *NetworkPeerNegotiateRequest) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerNegotiateRequest.ProtoReflect.Descriptor instead.
func (*NetworkPeerNegotiateRequest) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{0}
}

func (x *NetworkPeerNegotiateRequest) GetKeyCount() uint32 {
	if x != nil {
		return x.KeyCount
	}
	return 0
}

type NetworkPeerNegotiateResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	KeyCount uint32 `protobuf:"varint,1,opt,name=key_count,json=keyCount,proto3" json:"key_count,omitempty"`
}

func (x *NetworkPeerNegotiateResponse) Reset() {
	*x = NetworkPeerNegotiateResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[1]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerNegotiateResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerNegotiateResponse) ProtoMessage() {}

func (x *NetworkPeerNegotiateResponse) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[1]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerNegotiateResponse.ProtoReflect.Descriptor instead.
func (*NetworkPeerNegotiateResponse) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{1}
}

func (x *NetworkPeerNegotiateResponse) GetKeyCount() uint32 {
	if x != nil {
		return x.KeyCount
	}
	return 0
}

type NetworkPeerBinding struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Port        uint32                   `protobuf:"varint,1,opt,name=port,proto3" json:"port,omitempty"`
	Certificate *certificate.Certificate `protobuf:"bytes,2,opt,name=certificate,proto3" json:"certificate,omitempty"`
}

func (x *NetworkPeerBinding) Reset() {
	*x = NetworkPeerBinding{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[2]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerBinding) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerBinding) ProtoMessage() {}

func (x *NetworkPeerBinding) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[2]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerBinding.ProtoReflect.Descriptor instead.
func (*NetworkPeerBinding) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{2}
}

func (x *NetworkPeerBinding) GetPort() uint32 {
	if x != nil {
		return x.Port
	}
	return 0
}

func (x *NetworkPeerBinding) GetCertificate() *certificate.Certificate {
	if x != nil {
		return x.Certificate
	}
	return nil
}

type NetworkPeerOpenRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Bindings []*NetworkPeerBinding `protobuf:"bytes,1,rep,name=bindings,proto3" json:"bindings,omitempty"`
}

func (x *NetworkPeerOpenRequest) Reset() {
	*x = NetworkPeerOpenRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[3]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerOpenRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerOpenRequest) ProtoMessage() {}

func (x *NetworkPeerOpenRequest) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[3]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerOpenRequest.ProtoReflect.Descriptor instead.
func (*NetworkPeerOpenRequest) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{3}
}

func (x *NetworkPeerOpenRequest) GetBindings() []*NetworkPeerBinding {
	if x != nil {
		return x.Bindings
	}
	return nil
}

type NetworkPeerOpenResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Bindings []*NetworkPeerBinding `protobuf:"bytes,1,rep,name=bindings,proto3" json:"bindings,omitempty"`
}

func (x *NetworkPeerOpenResponse) Reset() {
	*x = NetworkPeerOpenResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[4]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerOpenResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerOpenResponse) ProtoMessage() {}

func (x *NetworkPeerOpenResponse) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[4]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerOpenResponse.ProtoReflect.Descriptor instead.
func (*NetworkPeerOpenResponse) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{4}
}

func (x *NetworkPeerOpenResponse) GetBindings() []*NetworkPeerBinding {
	if x != nil {
		return x.Bindings
	}
	return nil
}

type NetworkPeerCloseRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Key []byte `protobuf:"bytes,1,opt,name=key,proto3" json:"key,omitempty"`
}

func (x *NetworkPeerCloseRequest) Reset() {
	*x = NetworkPeerCloseRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[5]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerCloseRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerCloseRequest) ProtoMessage() {}

func (x *NetworkPeerCloseRequest) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[5]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerCloseRequest.ProtoReflect.Descriptor instead.
func (*NetworkPeerCloseRequest) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{5}
}

func (x *NetworkPeerCloseRequest) GetKey() []byte {
	if x != nil {
		return x.Key
	}
	return nil
}

type NetworkPeerCloseResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields
}

func (x *NetworkPeerCloseResponse) Reset() {
	*x = NetworkPeerCloseResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[6]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerCloseResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerCloseResponse) ProtoMessage() {}

func (x *NetworkPeerCloseResponse) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[6]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerCloseResponse.ProtoReflect.Descriptor instead.
func (*NetworkPeerCloseResponse) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{6}
}

type NetworkPeerUpdateCertificateRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Certificate *certificate.Certificate `protobuf:"bytes,1,opt,name=certificate,proto3" json:"certificate,omitempty"`
}

func (x *NetworkPeerUpdateCertificateRequest) Reset() {
	*x = NetworkPeerUpdateCertificateRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[7]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerUpdateCertificateRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerUpdateCertificateRequest) ProtoMessage() {}

func (x *NetworkPeerUpdateCertificateRequest) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[7]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerUpdateCertificateRequest.ProtoReflect.Descriptor instead.
func (*NetworkPeerUpdateCertificateRequest) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{7}
}

func (x *NetworkPeerUpdateCertificateRequest) GetCertificate() *certificate.Certificate {
	if x != nil {
		return x.Certificate
	}
	return nil
}

type NetworkPeerUpdateCertificateResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields
}

func (x *NetworkPeerUpdateCertificateResponse) Reset() {
	*x = NetworkPeerUpdateCertificateResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_network_v1_peer_proto_msgTypes[8]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkPeerUpdateCertificateResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkPeerUpdateCertificateResponse) ProtoMessage() {}

func (x *NetworkPeerUpdateCertificateResponse) ProtoReflect() protoreflect.Message {
	mi := &file_network_v1_peer_proto_msgTypes[8]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkPeerUpdateCertificateResponse.ProtoReflect.Descriptor instead.
func (*NetworkPeerUpdateCertificateResponse) Descriptor() ([]byte, []int) {
	return file_network_v1_peer_proto_rawDescGZIP(), []int{8}
}

var File_network_v1_peer_proto protoreflect.FileDescriptor

var file_network_v1_peer_proto_rawDesc = []byte{
	0x0a, 0x15, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2f, 0x76, 0x31, 0x2f, 0x70, 0x65, 0x65,
	0x72, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x12, 0x11, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e,
	0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76, 0x31, 0x1a, 0x16, 0x74, 0x79, 0x70, 0x65,
	0x2f, 0x63, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x2e, 0x70, 0x72, 0x6f,
	0x74, 0x6f, 0x22, 0x3a, 0x0a, 0x1b, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65,
	0x72, 0x4e, 0x65, 0x67, 0x6f, 0x74, 0x69, 0x61, 0x74, 0x65, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73,
	0x74, 0x12, 0x1b, 0x0a, 0x09, 0x6b, 0x65, 0x79, 0x5f, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x18, 0x01,
	0x20, 0x01, 0x28, 0x0d, 0x52, 0x08, 0x6b, 0x65, 0x79, 0x43, 0x6f, 0x75, 0x6e, 0x74, 0x22, 0x3b,
	0x0a, 0x1c, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x4e, 0x65, 0x67,
	0x6f, 0x74, 0x69, 0x61, 0x74, 0x65, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x1b,
	0x0a, 0x09, 0x6b, 0x65, 0x79, 0x5f, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x18, 0x01, 0x20, 0x01, 0x28,
	0x0d, 0x52, 0x08, 0x6b, 0x65, 0x79, 0x43, 0x6f, 0x75, 0x6e, 0x74, 0x22, 0x64, 0x0a, 0x12, 0x4e,
	0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x42, 0x69, 0x6e, 0x64, 0x69, 0x6e,
	0x67, 0x12, 0x12, 0x0a, 0x04, 0x70, 0x6f, 0x72, 0x74, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0d, 0x52,
	0x04, 0x70, 0x6f, 0x72, 0x74, 0x12, 0x3a, 0x0a, 0x0b, 0x63, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69,
	0x63, 0x61, 0x74, 0x65, 0x18, 0x02, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x18, 0x2e, 0x73, 0x74, 0x72,
	0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x2e, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69,
	0x63, 0x61, 0x74, 0x65, 0x52, 0x0b, 0x63, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74,
	0x65, 0x22, 0x5b, 0x0a, 0x16, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72,
	0x4f, 0x70, 0x65, 0x6e, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x41, 0x0a, 0x08, 0x62,
	0x69, 0x6e, 0x64, 0x69, 0x6e, 0x67, 0x73, 0x18, 0x01, 0x20, 0x03, 0x28, 0x0b, 0x32, 0x25, 0x2e,
	0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76,
	0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x42, 0x69, 0x6e,
	0x64, 0x69, 0x6e, 0x67, 0x52, 0x08, 0x62, 0x69, 0x6e, 0x64, 0x69, 0x6e, 0x67, 0x73, 0x22, 0x5c,
	0x0a, 0x17, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x4f, 0x70, 0x65,
	0x6e, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x41, 0x0a, 0x08, 0x62, 0x69, 0x6e,
	0x64, 0x69, 0x6e, 0x67, 0x73, 0x18, 0x01, 0x20, 0x03, 0x28, 0x0b, 0x32, 0x25, 0x2e, 0x73, 0x74,
	0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76, 0x31, 0x2e,
	0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x42, 0x69, 0x6e, 0x64, 0x69,
	0x6e, 0x67, 0x52, 0x08, 0x62, 0x69, 0x6e, 0x64, 0x69, 0x6e, 0x67, 0x73, 0x22, 0x2b, 0x0a, 0x17,
	0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x43, 0x6c, 0x6f, 0x73, 0x65,
	0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x10, 0x0a, 0x03, 0x6b, 0x65, 0x79, 0x18, 0x01,
	0x20, 0x01, 0x28, 0x0c, 0x52, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x1a, 0x0a, 0x18, 0x4e, 0x65, 0x74,
	0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x43, 0x6c, 0x6f, 0x73, 0x65, 0x52, 0x65, 0x73,
	0x70, 0x6f, 0x6e, 0x73, 0x65, 0x22, 0x61, 0x0a, 0x23, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b,
	0x50, 0x65, 0x65, 0x72, 0x55, 0x70, 0x64, 0x61, 0x74, 0x65, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66,
	0x69, 0x63, 0x61, 0x74, 0x65, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x3a, 0x0a, 0x0b,
	0x63, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x18, 0x01, 0x20, 0x01, 0x28,
	0x0b, 0x32, 0x18, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x2e,
	0x43, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x52, 0x0b, 0x63, 0x65, 0x72,
	0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x22, 0x26, 0x0a, 0x24, 0x4e, 0x65, 0x74, 0x77,
	0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x55, 0x70, 0x64, 0x61, 0x74, 0x65, 0x43, 0x65, 0x72,
	0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65,
	0x32, 0xc3, 0x03, 0x0a, 0x0b, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72,
	0x12, 0x6c, 0x0a, 0x09, 0x4e, 0x65, 0x67, 0x6f, 0x74, 0x69, 0x61, 0x74, 0x65, 0x12, 0x2e, 0x2e,
	0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76,
	0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x4e, 0x65, 0x67,
	0x6f, 0x74, 0x69, 0x61, 0x74, 0x65, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x1a, 0x2f, 0x2e,
	0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76,
	0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x4e, 0x65, 0x67,
	0x6f, 0x74, 0x69, 0x61, 0x74, 0x65, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x5d,
	0x0a, 0x04, 0x4f, 0x70, 0x65, 0x6e, 0x12, 0x29, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e,
	0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76, 0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f,
	0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x4f, 0x70, 0x65, 0x6e, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73,
	0x74, 0x1a, 0x2a, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77, 0x6f,
	0x72, 0x6b, 0x2e, 0x76, 0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65,
	0x72, 0x4f, 0x70, 0x65, 0x6e, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x60, 0x0a,
	0x05, 0x43, 0x6c, 0x6f, 0x73, 0x65, 0x12, 0x2a, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e,
	0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76, 0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f,
	0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x43, 0x6c, 0x6f, 0x73, 0x65, 0x52, 0x65, 0x71, 0x75, 0x65,
	0x73, 0x74, 0x1a, 0x2b, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77,
	0x6f, 0x72, 0x6b, 0x2e, 0x76, 0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65,
	0x65, 0x72, 0x43, 0x6c, 0x6f, 0x73, 0x65, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12,
	0x84, 0x01, 0x0a, 0x11, 0x55, 0x70, 0x64, 0x61, 0x74, 0x65, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66,
	0x69, 0x63, 0x61, 0x74, 0x65, 0x12, 0x36, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e,
	0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76, 0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72,
	0x6b, 0x50, 0x65, 0x65, 0x72, 0x55, 0x70, 0x64, 0x61, 0x74, 0x65, 0x43, 0x65, 0x72, 0x74, 0x69,
	0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x1a, 0x37, 0x2e,
	0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76,
	0x31, 0x2e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x50, 0x65, 0x65, 0x72, 0x55, 0x70, 0x64,
	0x61, 0x74, 0x65, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x52, 0x65,
	0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x42, 0x56, 0x0a, 0x14, 0x67, 0x67, 0x2e, 0x73, 0x74, 0x72,
	0x69, 0x6d, 0x73, 0x2e, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2e, 0x76, 0x31, 0x5a, 0x38,
	0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x4d, 0x65, 0x6d, 0x65, 0x4c,
	0x61, 0x62, 0x73, 0x2f, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2f, 0x70, 0x6b, 0x67, 0x2f, 0x61,
	0x70, 0x69, 0x73, 0x2f, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x2f, 0x76, 0x31, 0x3b, 0x6e,
	0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x76, 0x31, 0xba, 0x02, 0x03, 0x53, 0x4e, 0x54, 0x62, 0x06,
	0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_network_v1_peer_proto_rawDescOnce sync.Once
	file_network_v1_peer_proto_rawDescData = file_network_v1_peer_proto_rawDesc
)

func file_network_v1_peer_proto_rawDescGZIP() []byte {
	file_network_v1_peer_proto_rawDescOnce.Do(func() {
		file_network_v1_peer_proto_rawDescData = protoimpl.X.CompressGZIP(file_network_v1_peer_proto_rawDescData)
	})
	return file_network_v1_peer_proto_rawDescData
}

var file_network_v1_peer_proto_msgTypes = make([]protoimpl.MessageInfo, 9)
var file_network_v1_peer_proto_goTypes = []interface{}{
	(*NetworkPeerNegotiateRequest)(nil),          // 0: strims.network.v1.NetworkPeerNegotiateRequest
	(*NetworkPeerNegotiateResponse)(nil),         // 1: strims.network.v1.NetworkPeerNegotiateResponse
	(*NetworkPeerBinding)(nil),                   // 2: strims.network.v1.NetworkPeerBinding
	(*NetworkPeerOpenRequest)(nil),               // 3: strims.network.v1.NetworkPeerOpenRequest
	(*NetworkPeerOpenResponse)(nil),              // 4: strims.network.v1.NetworkPeerOpenResponse
	(*NetworkPeerCloseRequest)(nil),              // 5: strims.network.v1.NetworkPeerCloseRequest
	(*NetworkPeerCloseResponse)(nil),             // 6: strims.network.v1.NetworkPeerCloseResponse
	(*NetworkPeerUpdateCertificateRequest)(nil),  // 7: strims.network.v1.NetworkPeerUpdateCertificateRequest
	(*NetworkPeerUpdateCertificateResponse)(nil), // 8: strims.network.v1.NetworkPeerUpdateCertificateResponse
	(*certificate.Certificate)(nil),              // 9: strims.type.Certificate
}
var file_network_v1_peer_proto_depIdxs = []int32{
	9, // 0: strims.network.v1.NetworkPeerBinding.certificate:type_name -> strims.type.Certificate
	2, // 1: strims.network.v1.NetworkPeerOpenRequest.bindings:type_name -> strims.network.v1.NetworkPeerBinding
	2, // 2: strims.network.v1.NetworkPeerOpenResponse.bindings:type_name -> strims.network.v1.NetworkPeerBinding
	9, // 3: strims.network.v1.NetworkPeerUpdateCertificateRequest.certificate:type_name -> strims.type.Certificate
	0, // 4: strims.network.v1.NetworkPeer.Negotiate:input_type -> strims.network.v1.NetworkPeerNegotiateRequest
	3, // 5: strims.network.v1.NetworkPeer.Open:input_type -> strims.network.v1.NetworkPeerOpenRequest
	5, // 6: strims.network.v1.NetworkPeer.Close:input_type -> strims.network.v1.NetworkPeerCloseRequest
	7, // 7: strims.network.v1.NetworkPeer.UpdateCertificate:input_type -> strims.network.v1.NetworkPeerUpdateCertificateRequest
	1, // 8: strims.network.v1.NetworkPeer.Negotiate:output_type -> strims.network.v1.NetworkPeerNegotiateResponse
	4, // 9: strims.network.v1.NetworkPeer.Open:output_type -> strims.network.v1.NetworkPeerOpenResponse
	6, // 10: strims.network.v1.NetworkPeer.Close:output_type -> strims.network.v1.NetworkPeerCloseResponse
	8, // 11: strims.network.v1.NetworkPeer.UpdateCertificate:output_type -> strims.network.v1.NetworkPeerUpdateCertificateResponse
	8, // [8:12] is the sub-list for method output_type
	4, // [4:8] is the sub-list for method input_type
	4, // [4:4] is the sub-list for extension type_name
	4, // [4:4] is the sub-list for extension extendee
	0, // [0:4] is the sub-list for field type_name
}

func init() { file_network_v1_peer_proto_init() }
func file_network_v1_peer_proto_init() {
	if File_network_v1_peer_proto != nil {
		return
	}
	if !protoimpl.UnsafeEnabled {
		file_network_v1_peer_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerNegotiateRequest); i {
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
		file_network_v1_peer_proto_msgTypes[1].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerNegotiateResponse); i {
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
		file_network_v1_peer_proto_msgTypes[2].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerBinding); i {
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
		file_network_v1_peer_proto_msgTypes[3].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerOpenRequest); i {
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
		file_network_v1_peer_proto_msgTypes[4].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerOpenResponse); i {
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
		file_network_v1_peer_proto_msgTypes[5].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerCloseRequest); i {
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
		file_network_v1_peer_proto_msgTypes[6].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerCloseResponse); i {
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
		file_network_v1_peer_proto_msgTypes[7].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerUpdateCertificateRequest); i {
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
		file_network_v1_peer_proto_msgTypes[8].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkPeerUpdateCertificateResponse); i {
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
	type x struct{}
	out := protoimpl.TypeBuilder{
		File: protoimpl.DescBuilder{
			GoPackagePath: reflect.TypeOf(x{}).PkgPath(),
			RawDescriptor: file_network_v1_peer_proto_rawDesc,
			NumEnums:      0,
			NumMessages:   9,
			NumExtensions: 0,
			NumServices:   1,
		},
		GoTypes:           file_network_v1_peer_proto_goTypes,
		DependencyIndexes: file_network_v1_peer_proto_depIdxs,
		MessageInfos:      file_network_v1_peer_proto_msgTypes,
	}.Build()
	File_network_v1_peer_proto = out.File
	file_network_v1_peer_proto_rawDesc = nil
	file_network_v1_peer_proto_goTypes = nil
	file_network_v1_peer_proto_depIdxs = nil
}
