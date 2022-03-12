// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.26.0
// 	protoc        v3.19.1
// source: autoseed/v1/events.proto

package autoseedv1

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
		mi := &file_autoseed_v1_events_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ConfigChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ConfigChangeEvent) ProtoMessage() {}

func (x *ConfigChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_autoseed_v1_events_proto_msgTypes[0]
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
	return file_autoseed_v1_events_proto_rawDescGZIP(), []int{0}
}

func (x *ConfigChangeEvent) GetConfig() *Config {
	if x != nil {
		return x.Config
	}
	return nil
}

type RuleChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Rule *Rule `protobuf:"bytes,1,opt,name=rule,proto3" json:"rule,omitempty"`
}

func (x *RuleChangeEvent) Reset() {
	*x = RuleChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_autoseed_v1_events_proto_msgTypes[1]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *RuleChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*RuleChangeEvent) ProtoMessage() {}

func (x *RuleChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_autoseed_v1_events_proto_msgTypes[1]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use RuleChangeEvent.ProtoReflect.Descriptor instead.
func (*RuleChangeEvent) Descriptor() ([]byte, []int) {
	return file_autoseed_v1_events_proto_rawDescGZIP(), []int{1}
}

func (x *RuleChangeEvent) GetRule() *Rule {
	if x != nil {
		return x.Rule
	}
	return nil
}

type RuleDeleteEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Rule *Rule `protobuf:"bytes,1,opt,name=rule,proto3" json:"rule,omitempty"`
}

func (x *RuleDeleteEvent) Reset() {
	*x = RuleDeleteEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_autoseed_v1_events_proto_msgTypes[2]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *RuleDeleteEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*RuleDeleteEvent) ProtoMessage() {}

func (x *RuleDeleteEvent) ProtoReflect() protoreflect.Message {
	mi := &file_autoseed_v1_events_proto_msgTypes[2]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use RuleDeleteEvent.ProtoReflect.Descriptor instead.
func (*RuleDeleteEvent) Descriptor() ([]byte, []int) {
	return file_autoseed_v1_events_proto_rawDescGZIP(), []int{2}
}

func (x *RuleDeleteEvent) GetRule() *Rule {
	if x != nil {
		return x.Rule
	}
	return nil
}

var File_autoseed_v1_events_proto protoreflect.FileDescriptor

var file_autoseed_v1_events_proto_rawDesc = []byte{
	0x0a, 0x18, 0x61, 0x75, 0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x2f, 0x76, 0x31, 0x2f, 0x65, 0x76,
	0x65, 0x6e, 0x74, 0x73, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x12, 0x12, 0x73, 0x74, 0x72, 0x69,
	0x6d, 0x73, 0x2e, 0x61, 0x75, 0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x2e, 0x76, 0x31, 0x1a, 0x1a,
	0x61, 0x75, 0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x2f, 0x76, 0x31, 0x2f, 0x61, 0x75, 0x74, 0x6f,
	0x73, 0x65, 0x65, 0x64, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x22, 0x47, 0x0a, 0x11, 0x43, 0x6f,
	0x6e, 0x66, 0x69, 0x67, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12,
	0x32, 0x0a, 0x06, 0x63, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32,
	0x1a, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x61, 0x75, 0x74, 0x6f, 0x73, 0x65, 0x65,
	0x64, 0x2e, 0x76, 0x31, 0x2e, 0x43, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x52, 0x06, 0x63, 0x6f, 0x6e,
	0x66, 0x69, 0x67, 0x22, 0x3f, 0x0a, 0x0f, 0x52, 0x75, 0x6c, 0x65, 0x43, 0x68, 0x61, 0x6e, 0x67,
	0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x2c, 0x0a, 0x04, 0x72, 0x75, 0x6c, 0x65, 0x18, 0x01,
	0x20, 0x01, 0x28, 0x0b, 0x32, 0x18, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x61, 0x75,
	0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x2e, 0x76, 0x31, 0x2e, 0x52, 0x75, 0x6c, 0x65, 0x52, 0x04,
	0x72, 0x75, 0x6c, 0x65, 0x22, 0x3f, 0x0a, 0x0f, 0x52, 0x75, 0x6c, 0x65, 0x44, 0x65, 0x6c, 0x65,
	0x74, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x2c, 0x0a, 0x04, 0x72, 0x75, 0x6c, 0x65, 0x18,
	0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x18, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x61,
	0x75, 0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x2e, 0x76, 0x31, 0x2e, 0x52, 0x75, 0x6c, 0x65, 0x52,
	0x04, 0x72, 0x75, 0x6c, 0x65, 0x42, 0x5b, 0x0a, 0x15, 0x67, 0x67, 0x2e, 0x73, 0x74, 0x72, 0x69,
	0x6d, 0x73, 0x2e, 0x61, 0x75, 0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x2e, 0x76, 0x31, 0x5a, 0x3c,
	0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x4d, 0x65, 0x6d, 0x65, 0x4c,
	0x61, 0x62, 0x73, 0x2f, 0x67, 0x6f, 0x2d, 0x70, 0x70, 0x73, 0x70, 0x70, 0x2f, 0x70, 0x6b, 0x67,
	0x2f, 0x61, 0x70, 0x69, 0x73, 0x2f, 0x61, 0x75, 0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x2f, 0x76,
	0x31, 0x3b, 0x61, 0x75, 0x74, 0x6f, 0x73, 0x65, 0x65, 0x64, 0x76, 0x31, 0xba, 0x02, 0x03, 0x53,
	0x41, 0x53, 0x62, 0x06, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_autoseed_v1_events_proto_rawDescOnce sync.Once
	file_autoseed_v1_events_proto_rawDescData = file_autoseed_v1_events_proto_rawDesc
)

func file_autoseed_v1_events_proto_rawDescGZIP() []byte {
	file_autoseed_v1_events_proto_rawDescOnce.Do(func() {
		file_autoseed_v1_events_proto_rawDescData = protoimpl.X.CompressGZIP(file_autoseed_v1_events_proto_rawDescData)
	})
	return file_autoseed_v1_events_proto_rawDescData
}

var file_autoseed_v1_events_proto_msgTypes = make([]protoimpl.MessageInfo, 3)
var file_autoseed_v1_events_proto_goTypes = []interface{}{
	(*ConfigChangeEvent)(nil), // 0: strims.autoseed.v1.ConfigChangeEvent
	(*RuleChangeEvent)(nil),   // 1: strims.autoseed.v1.RuleChangeEvent
	(*RuleDeleteEvent)(nil),   // 2: strims.autoseed.v1.RuleDeleteEvent
	(*Config)(nil),            // 3: strims.autoseed.v1.Config
	(*Rule)(nil),              // 4: strims.autoseed.v1.Rule
}
var file_autoseed_v1_events_proto_depIdxs = []int32{
	3, // 0: strims.autoseed.v1.ConfigChangeEvent.config:type_name -> strims.autoseed.v1.Config
	4, // 1: strims.autoseed.v1.RuleChangeEvent.rule:type_name -> strims.autoseed.v1.Rule
	4, // 2: strims.autoseed.v1.RuleDeleteEvent.rule:type_name -> strims.autoseed.v1.Rule
	3, // [3:3] is the sub-list for method output_type
	3, // [3:3] is the sub-list for method input_type
	3, // [3:3] is the sub-list for extension type_name
	3, // [3:3] is the sub-list for extension extendee
	0, // [0:3] is the sub-list for field type_name
}

func init() { file_autoseed_v1_events_proto_init() }
func file_autoseed_v1_events_proto_init() {
	if File_autoseed_v1_events_proto != nil {
		return
	}
	file_autoseed_v1_autoseed_proto_init()
	if !protoimpl.UnsafeEnabled {
		file_autoseed_v1_events_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
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
		file_autoseed_v1_events_proto_msgTypes[1].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*RuleChangeEvent); i {
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
		file_autoseed_v1_events_proto_msgTypes[2].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*RuleDeleteEvent); i {
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
			RawDescriptor: file_autoseed_v1_events_proto_rawDesc,
			NumEnums:      0,
			NumMessages:   3,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_autoseed_v1_events_proto_goTypes,
		DependencyIndexes: file_autoseed_v1_events_proto_depIdxs,
		MessageInfos:      file_autoseed_v1_events_proto_msgTypes,
	}.Build()
	File_autoseed_v1_events_proto = out.File
	file_autoseed_v1_events_proto_rawDesc = nil
	file_autoseed_v1_events_proto_goTypes = nil
	file_autoseed_v1_events_proto_depIdxs = nil
}