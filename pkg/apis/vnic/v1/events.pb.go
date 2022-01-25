// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.26.0
// 	protoc        v3.19.1
// source: vnic/v1/events.proto

package vnicv1

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

type ConfigChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Config *Config `protobuf:"bytes,1,opt,name=config,proto3" json:"config,omitempty"`
}

func (x *ConfigChangeEvent) Reset() {
	*x = ConfigChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_vnic_v1_events_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ConfigChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ConfigChangeEvent) ProtoMessage() {}

func (x *ConfigChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_vnic_v1_events_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ConfigChangeEvent.ProtoReflect.Descriptor instead.
func (*ConfigChangeEvent) Descriptor() ([]byte, []int) {
	return file_vnic_v1_events_proto_rawDescGZIP(), []int{0}
}

func (x *ConfigChangeEvent) GetConfig() *Config {
	if x != nil {
		return x.Config
	}
	return nil
}

var File_vnic_v1_events_proto protoreflect.FileDescriptor

var file_vnic_v1_events_proto_rawDesc = []byte{
	0x0a, 0x14, 0x76, 0x6e, 0x69, 0x63, 0x2f, 0x76, 0x31, 0x2f, 0x65, 0x76, 0x65, 0x6e, 0x74, 0x73,
	0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x12, 0x0e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x76,
	0x6e, 0x69, 0x63, 0x2e, 0x76, 0x31, 0x1a, 0x12, 0x76, 0x6e, 0x69, 0x63, 0x2f, 0x76, 0x31, 0x2f,
	0x76, 0x6e, 0x69, 0x63, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x22, 0x43, 0x0a, 0x11, 0x43, 0x6f,
	0x6e, 0x66, 0x69, 0x67, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12,
	0x2e, 0x0a, 0x06, 0x63, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32,
	0x16, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x76, 0x6e, 0x69, 0x63, 0x2e, 0x76, 0x31,
	0x2e, 0x43, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x52, 0x06, 0x63, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x42,
	0x4f, 0x0a, 0x11, 0x67, 0x67, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x76, 0x6e, 0x69,
	0x63, 0x2e, 0x76, 0x31, 0x5a, 0x34, 0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d,
	0x2f, 0x4d, 0x65, 0x6d, 0x65, 0x4c, 0x61, 0x62, 0x73, 0x2f, 0x67, 0x6f, 0x2d, 0x70, 0x70, 0x73,
	0x70, 0x70, 0x2f, 0x70, 0x6b, 0x67, 0x2f, 0x61, 0x70, 0x69, 0x73, 0x2f, 0x76, 0x6e, 0x69, 0x63,
	0x2f, 0x76, 0x31, 0x3b, 0x76, 0x6e, 0x69, 0x63, 0x76, 0x31, 0xba, 0x02, 0x03, 0x53, 0x56, 0x4e,
	0x62, 0x06, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_vnic_v1_events_proto_rawDescOnce sync.Once
	file_vnic_v1_events_proto_rawDescData = file_vnic_v1_events_proto_rawDesc
)

func file_vnic_v1_events_proto_rawDescGZIP() []byte {
	file_vnic_v1_events_proto_rawDescOnce.Do(func() {
		file_vnic_v1_events_proto_rawDescData = protoimpl.X.CompressGZIP(file_vnic_v1_events_proto_rawDescData)
	})
	return file_vnic_v1_events_proto_rawDescData
}

var file_vnic_v1_events_proto_msgTypes = make([]protoimpl.MessageInfo, 1)
var file_vnic_v1_events_proto_goTypes = []interface{}{
	(*ConfigChangeEvent)(nil), // 0: strims.vnic.v1.ConfigChangeEvent
	(*Config)(nil),            // 1: strims.vnic.v1.Config
}
var file_vnic_v1_events_proto_depIdxs = []int32{
	1, // 0: strims.vnic.v1.ConfigChangeEvent.config:type_name -> strims.vnic.v1.Config
	1, // [1:1] is the sub-list for method output_type
	1, // [1:1] is the sub-list for method input_type
	1, // [1:1] is the sub-list for extension type_name
	1, // [1:1] is the sub-list for extension extendee
	0, // [0:1] is the sub-list for field type_name
}

func init() { file_vnic_v1_events_proto_init() }
func file_vnic_v1_events_proto_init() {
	if File_vnic_v1_events_proto != nil {
		return
	}
	file_vnic_v1_vnic_proto_init()
	if !protoimpl.UnsafeEnabled {
		file_vnic_v1_events_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ConfigChangeEvent); i {
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
			RawDescriptor: file_vnic_v1_events_proto_rawDesc,
			NumEnums:      0,
			NumMessages:   1,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_vnic_v1_events_proto_goTypes,
		DependencyIndexes: file_vnic_v1_events_proto_depIdxs,
		MessageInfos:      file_vnic_v1_events_proto_msgTypes,
	}.Build()
	File_vnic_v1_events_proto = out.File
	file_vnic_v1_events_proto_rawDesc = nil
	file_vnic_v1_events_proto_goTypes = nil
	file_vnic_v1_events_proto_depIdxs = nil
}
