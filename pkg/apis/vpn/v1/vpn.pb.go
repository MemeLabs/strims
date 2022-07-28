// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.28.1
// 	protoc        v3.20.1
// source: vpn/v1/vpn.proto

package vpn

import (
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

type NetworkAddress struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	HostId []byte `protobuf:"bytes,1,opt,name=host_id,json=hostId,proto3" json:"host_id,omitempty"`
	Port   uint32 `protobuf:"varint,2,opt,name=port,proto3" json:"port,omitempty"`
}

func (x *NetworkAddress) Reset() {
	*x = NetworkAddress{}
	if protoimpl.UnsafeEnabled {
		mi := &file_vpn_v1_vpn_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *NetworkAddress) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*NetworkAddress) ProtoMessage() {}

func (x *NetworkAddress) ProtoReflect() protoreflect.Message {
	mi := &file_vpn_v1_vpn_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use NetworkAddress.ProtoReflect.Descriptor instead.
func (*NetworkAddress) Descriptor() ([]byte, []int) {
	return file_vpn_v1_vpn_proto_rawDescGZIP(), []int{0}
}

func (x *NetworkAddress) GetHostId() []byte {
	if x != nil {
		return x.HostId
	}
	return nil
}

func (x *NetworkAddress) GetPort() uint32 {
	if x != nil {
		return x.Port
	}
	return 0
}

var File_vpn_v1_vpn_proto protoreflect.FileDescriptor

var file_vpn_v1_vpn_proto_rawDesc = []byte{
	0x0a, 0x10, 0x76, 0x70, 0x6e, 0x2f, 0x76, 0x31, 0x2f, 0x76, 0x70, 0x6e, 0x2e, 0x70, 0x72, 0x6f,
	0x74, 0x6f, 0x12, 0x0d, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x76, 0x70, 0x6e, 0x2e, 0x76,
	0x31, 0x22, 0x3d, 0x0a, 0x0e, 0x4e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x41, 0x64, 0x64, 0x72,
	0x65, 0x73, 0x73, 0x12, 0x17, 0x0a, 0x07, 0x68, 0x6f, 0x73, 0x74, 0x5f, 0x69, 0x64, 0x18, 0x01,
	0x20, 0x01, 0x28, 0x0c, 0x52, 0x06, 0x68, 0x6f, 0x73, 0x74, 0x49, 0x64, 0x12, 0x12, 0x0a, 0x04,
	0x70, 0x6f, 0x72, 0x74, 0x18, 0x02, 0x20, 0x01, 0x28, 0x0d, 0x52, 0x04, 0x70, 0x6f, 0x72, 0x74,
	0x42, 0x48, 0x0a, 0x10, 0x67, 0x67, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x76, 0x70,
	0x6e, 0x2e, 0x76, 0x31, 0x5a, 0x2e, 0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d,
	0x2f, 0x4d, 0x65, 0x6d, 0x65, 0x4c, 0x61, 0x62, 0x73, 0x2f, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73,
	0x2f, 0x70, 0x6b, 0x67, 0x2f, 0x61, 0x70, 0x69, 0x73, 0x2f, 0x76, 0x70, 0x6e, 0x2f, 0x76, 0x31,
	0x3b, 0x76, 0x70, 0x6e, 0xba, 0x02, 0x03, 0x53, 0x56, 0x4e, 0x62, 0x06, 0x70, 0x72, 0x6f, 0x74,
	0x6f, 0x33,
}

var (
	file_vpn_v1_vpn_proto_rawDescOnce sync.Once
	file_vpn_v1_vpn_proto_rawDescData = file_vpn_v1_vpn_proto_rawDesc
)

func file_vpn_v1_vpn_proto_rawDescGZIP() []byte {
	file_vpn_v1_vpn_proto_rawDescOnce.Do(func() {
		file_vpn_v1_vpn_proto_rawDescData = protoimpl.X.CompressGZIP(file_vpn_v1_vpn_proto_rawDescData)
	})
	return file_vpn_v1_vpn_proto_rawDescData
}

var file_vpn_v1_vpn_proto_msgTypes = make([]protoimpl.MessageInfo, 1)
var file_vpn_v1_vpn_proto_goTypes = []interface{}{
	(*NetworkAddress)(nil), // 0: strims.vpn.v1.NetworkAddress
}
var file_vpn_v1_vpn_proto_depIdxs = []int32{
	0, // [0:0] is the sub-list for method output_type
	0, // [0:0] is the sub-list for method input_type
	0, // [0:0] is the sub-list for extension type_name
	0, // [0:0] is the sub-list for extension extendee
	0, // [0:0] is the sub-list for field type_name
}

func init() { file_vpn_v1_vpn_proto_init() }
func file_vpn_v1_vpn_proto_init() {
	if File_vpn_v1_vpn_proto != nil {
		return
	}
	if !protoimpl.UnsafeEnabled {
		file_vpn_v1_vpn_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*NetworkAddress); i {
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
			RawDescriptor: file_vpn_v1_vpn_proto_rawDesc,
			NumEnums:      0,
			NumMessages:   1,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_vpn_v1_vpn_proto_goTypes,
		DependencyIndexes: file_vpn_v1_vpn_proto_depIdxs,
		MessageInfos:      file_vpn_v1_vpn_proto_msgTypes,
	}.Build()
	File_vpn_v1_vpn_proto = out.File
	file_vpn_v1_vpn_proto_rawDesc = nil
	file_vpn_v1_vpn_proto_goTypes = nil
	file_vpn_v1_vpn_proto_depIdxs = nil
}
