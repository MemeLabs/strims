// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.23.0
// 	protoc        v3.13.0
// source: debug.proto

package pb

import (
	proto "github.com/golang/protobuf/proto"
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

// This is a compile-time assertion that a sufficiently up-to-date version
// of the legacy proto package is being used.
const _ = proto.ProtoPackageIsVersion4

type MetricsFormat int32

const (
	MetricsFormat_METRICS_FORMAT_TEXT          MetricsFormat = 0
	MetricsFormat_METRICS_FORMAT_PROTO_DELIM   MetricsFormat = 1
	MetricsFormat_METRICS_FORMAT_PROTO_TEXT    MetricsFormat = 2
	MetricsFormat_METRICS_FORMAT_PROTO_COMPACT MetricsFormat = 3
	MetricsFormat_METRICS_FORMAT_OPEN_METRICS  MetricsFormat = 4
)

// Enum value maps for MetricsFormat.
var (
	MetricsFormat_name = map[int32]string{
		0: "METRICS_FORMAT_TEXT",
		1: "METRICS_FORMAT_PROTO_DELIM",
		2: "METRICS_FORMAT_PROTO_TEXT",
		3: "METRICS_FORMAT_PROTO_COMPACT",
		4: "METRICS_FORMAT_OPEN_METRICS",
	}
	MetricsFormat_value = map[string]int32{
		"METRICS_FORMAT_TEXT":          0,
		"METRICS_FORMAT_PROTO_DELIM":   1,
		"METRICS_FORMAT_PROTO_TEXT":    2,
		"METRICS_FORMAT_PROTO_COMPACT": 3,
		"METRICS_FORMAT_OPEN_METRICS":  4,
	}
)

func (x MetricsFormat) Enum() *MetricsFormat {
	p := new(MetricsFormat)
	*p = x
	return p
}

func (x MetricsFormat) String() string {
	return protoimpl.X.EnumStringOf(x.Descriptor(), protoreflect.EnumNumber(x))
}

func (MetricsFormat) Descriptor() protoreflect.EnumDescriptor {
	return file_debug_proto_enumTypes[0].Descriptor()
}

func (MetricsFormat) Type() protoreflect.EnumType {
	return &file_debug_proto_enumTypes[0]
}

func (x MetricsFormat) Number() protoreflect.EnumNumber {
	return protoreflect.EnumNumber(x)
}

// Deprecated: Use MetricsFormat.Descriptor instead.
func (MetricsFormat) EnumDescriptor() ([]byte, []int) {
	return file_debug_proto_rawDescGZIP(), []int{0}
}

type PProfRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Name  string `protobuf:"bytes,1,opt,name=name,proto3" json:"name,omitempty"`
	Debug bool   `protobuf:"varint,2,opt,name=debug,proto3" json:"debug,omitempty"`
	Gc    bool   `protobuf:"varint,3,opt,name=gc,proto3" json:"gc,omitempty"`
}

func (x *PProfRequest) Reset() {
	*x = PProfRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_debug_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *PProfRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*PProfRequest) ProtoMessage() {}

func (x *PProfRequest) ProtoReflect() protoreflect.Message {
	mi := &file_debug_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use PProfRequest.ProtoReflect.Descriptor instead.
func (*PProfRequest) Descriptor() ([]byte, []int) {
	return file_debug_proto_rawDescGZIP(), []int{0}
}

func (x *PProfRequest) GetName() string {
	if x != nil {
		return x.Name
	}
	return ""
}

func (x *PProfRequest) GetDebug() bool {
	if x != nil {
		return x.Debug
	}
	return false
}

func (x *PProfRequest) GetGc() bool {
	if x != nil {
		return x.Gc
	}
	return false
}

type PProfResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Name string `protobuf:"bytes,1,opt,name=name,proto3" json:"name,omitempty"`
	Data []byte `protobuf:"bytes,2,opt,name=data,proto3" json:"data,omitempty"`
}

func (x *PProfResponse) Reset() {
	*x = PProfResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_debug_proto_msgTypes[1]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *PProfResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*PProfResponse) ProtoMessage() {}

func (x *PProfResponse) ProtoReflect() protoreflect.Message {
	mi := &file_debug_proto_msgTypes[1]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use PProfResponse.ProtoReflect.Descriptor instead.
func (*PProfResponse) Descriptor() ([]byte, []int) {
	return file_debug_proto_rawDescGZIP(), []int{1}
}

func (x *PProfResponse) GetName() string {
	if x != nil {
		return x.Name
	}
	return ""
}

func (x *PProfResponse) GetData() []byte {
	if x != nil {
		return x.Data
	}
	return nil
}

type ReadMetricsRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Format MetricsFormat `protobuf:"varint,1,opt,name=format,proto3,enum=MetricsFormat" json:"format,omitempty"`
}

func (x *ReadMetricsRequest) Reset() {
	*x = ReadMetricsRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_debug_proto_msgTypes[2]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ReadMetricsRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ReadMetricsRequest) ProtoMessage() {}

func (x *ReadMetricsRequest) ProtoReflect() protoreflect.Message {
	mi := &file_debug_proto_msgTypes[2]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ReadMetricsRequest.ProtoReflect.Descriptor instead.
func (*ReadMetricsRequest) Descriptor() ([]byte, []int) {
	return file_debug_proto_rawDescGZIP(), []int{2}
}

func (x *ReadMetricsRequest) GetFormat() MetricsFormat {
	if x != nil {
		return x.Format
	}
	return MetricsFormat_METRICS_FORMAT_TEXT
}

type ReadMetricsResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Data []byte `protobuf:"bytes,1,opt,name=data,proto3" json:"data,omitempty"`
}

func (x *ReadMetricsResponse) Reset() {
	*x = ReadMetricsResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_debug_proto_msgTypes[3]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ReadMetricsResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ReadMetricsResponse) ProtoMessage() {}

func (x *ReadMetricsResponse) ProtoReflect() protoreflect.Message {
	mi := &file_debug_proto_msgTypes[3]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ReadMetricsResponse.ProtoReflect.Descriptor instead.
func (*ReadMetricsResponse) Descriptor() ([]byte, []int) {
	return file_debug_proto_rawDescGZIP(), []int{3}
}

func (x *ReadMetricsResponse) GetData() []byte {
	if x != nil {
		return x.Data
	}
	return nil
}

var File_debug_proto protoreflect.FileDescriptor

var file_debug_proto_rawDesc = []byte{
	0x0a, 0x0b, 0x64, 0x65, 0x62, 0x75, 0x67, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x22, 0x48, 0x0a,
	0x0c, 0x50, 0x50, 0x72, 0x6f, 0x66, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x12, 0x0a,
	0x04, 0x6e, 0x61, 0x6d, 0x65, 0x18, 0x01, 0x20, 0x01, 0x28, 0x09, 0x52, 0x04, 0x6e, 0x61, 0x6d,
	0x65, 0x12, 0x14, 0x0a, 0x05, 0x64, 0x65, 0x62, 0x75, 0x67, 0x18, 0x02, 0x20, 0x01, 0x28, 0x08,
	0x52, 0x05, 0x64, 0x65, 0x62, 0x75, 0x67, 0x12, 0x0e, 0x0a, 0x02, 0x67, 0x63, 0x18, 0x03, 0x20,
	0x01, 0x28, 0x08, 0x52, 0x02, 0x67, 0x63, 0x22, 0x37, 0x0a, 0x0d, 0x50, 0x50, 0x72, 0x6f, 0x66,
	0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x12, 0x0a, 0x04, 0x6e, 0x61, 0x6d, 0x65,
	0x18, 0x01, 0x20, 0x01, 0x28, 0x09, 0x52, 0x04, 0x6e, 0x61, 0x6d, 0x65, 0x12, 0x12, 0x0a, 0x04,
	0x64, 0x61, 0x74, 0x61, 0x18, 0x02, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x04, 0x64, 0x61, 0x74, 0x61,
	0x22, 0x3c, 0x0a, 0x12, 0x52, 0x65, 0x61, 0x64, 0x4d, 0x65, 0x74, 0x72, 0x69, 0x63, 0x73, 0x52,
	0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x26, 0x0a, 0x06, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74,
	0x18, 0x01, 0x20, 0x01, 0x28, 0x0e, 0x32, 0x0e, 0x2e, 0x4d, 0x65, 0x74, 0x72, 0x69, 0x63, 0x73,
	0x46, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x52, 0x06, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x22, 0x29,
	0x0a, 0x13, 0x52, 0x65, 0x61, 0x64, 0x4d, 0x65, 0x74, 0x72, 0x69, 0x63, 0x73, 0x52, 0x65, 0x73,
	0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x12, 0x0a, 0x04, 0x64, 0x61, 0x74, 0x61, 0x18, 0x01, 0x20,
	0x01, 0x28, 0x0c, 0x52, 0x04, 0x64, 0x61, 0x74, 0x61, 0x2a, 0xaa, 0x01, 0x0a, 0x0d, 0x4d, 0x65,
	0x74, 0x72, 0x69, 0x63, 0x73, 0x46, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x12, 0x17, 0x0a, 0x13, 0x4d,
	0x45, 0x54, 0x52, 0x49, 0x43, 0x53, 0x5f, 0x46, 0x4f, 0x52, 0x4d, 0x41, 0x54, 0x5f, 0x54, 0x45,
	0x58, 0x54, 0x10, 0x00, 0x12, 0x1e, 0x0a, 0x1a, 0x4d, 0x45, 0x54, 0x52, 0x49, 0x43, 0x53, 0x5f,
	0x46, 0x4f, 0x52, 0x4d, 0x41, 0x54, 0x5f, 0x50, 0x52, 0x4f, 0x54, 0x4f, 0x5f, 0x44, 0x45, 0x4c,
	0x49, 0x4d, 0x10, 0x01, 0x12, 0x1d, 0x0a, 0x19, 0x4d, 0x45, 0x54, 0x52, 0x49, 0x43, 0x53, 0x5f,
	0x46, 0x4f, 0x52, 0x4d, 0x41, 0x54, 0x5f, 0x50, 0x52, 0x4f, 0x54, 0x4f, 0x5f, 0x54, 0x45, 0x58,
	0x54, 0x10, 0x02, 0x12, 0x20, 0x0a, 0x1c, 0x4d, 0x45, 0x54, 0x52, 0x49, 0x43, 0x53, 0x5f, 0x46,
	0x4f, 0x52, 0x4d, 0x41, 0x54, 0x5f, 0x50, 0x52, 0x4f, 0x54, 0x4f, 0x5f, 0x43, 0x4f, 0x4d, 0x50,
	0x41, 0x43, 0x54, 0x10, 0x03, 0x12, 0x1f, 0x0a, 0x1b, 0x4d, 0x45, 0x54, 0x52, 0x49, 0x43, 0x53,
	0x5f, 0x46, 0x4f, 0x52, 0x4d, 0x41, 0x54, 0x5f, 0x4f, 0x50, 0x45, 0x4e, 0x5f, 0x4d, 0x45, 0x54,
	0x52, 0x49, 0x43, 0x53, 0x10, 0x04, 0x42, 0x44, 0x0a, 0x15, 0x67, 0x67, 0x2e, 0x73, 0x74, 0x72,
	0x69, 0x6d, 0x73, 0x2e, 0x70, 0x70, 0x73, 0x70, 0x70, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x5a,
	0x26, 0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x4d, 0x65, 0x6d, 0x65,
	0x4c, 0x61, 0x62, 0x73, 0x2f, 0x67, 0x6f, 0x2d, 0x70, 0x70, 0x73, 0x70, 0x70, 0x2f, 0x70, 0x6b,
	0x67, 0x2f, 0x70, 0x62, 0x3b, 0x70, 0x62, 0xba, 0x02, 0x02, 0x50, 0x42, 0x62, 0x06, 0x70, 0x72,
	0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_debug_proto_rawDescOnce sync.Once
	file_debug_proto_rawDescData = file_debug_proto_rawDesc
)

func file_debug_proto_rawDescGZIP() []byte {
	file_debug_proto_rawDescOnce.Do(func() {
		file_debug_proto_rawDescData = protoimpl.X.CompressGZIP(file_debug_proto_rawDescData)
	})
	return file_debug_proto_rawDescData
}

var file_debug_proto_enumTypes = make([]protoimpl.EnumInfo, 1)
var file_debug_proto_msgTypes = make([]protoimpl.MessageInfo, 4)
var file_debug_proto_goTypes = []interface{}{
	(MetricsFormat)(0),          // 0: MetricsFormat
	(*PProfRequest)(nil),        // 1: PProfRequest
	(*PProfResponse)(nil),       // 2: PProfResponse
	(*ReadMetricsRequest)(nil),  // 3: ReadMetricsRequest
	(*ReadMetricsResponse)(nil), // 4: ReadMetricsResponse
}
var file_debug_proto_depIdxs = []int32{
	0, // 0: ReadMetricsRequest.format:type_name -> MetricsFormat
	1, // [1:1] is the sub-list for method output_type
	1, // [1:1] is the sub-list for method input_type
	1, // [1:1] is the sub-list for extension type_name
	1, // [1:1] is the sub-list for extension extendee
	0, // [0:1] is the sub-list for field type_name
}

func init() { file_debug_proto_init() }
func file_debug_proto_init() {
	if File_debug_proto != nil {
		return
	}
	if !protoimpl.UnsafeEnabled {
		file_debug_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*PProfRequest); i {
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
		file_debug_proto_msgTypes[1].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*PProfResponse); i {
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
		file_debug_proto_msgTypes[2].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ReadMetricsRequest); i {
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
		file_debug_proto_msgTypes[3].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ReadMetricsResponse); i {
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
			RawDescriptor: file_debug_proto_rawDesc,
			NumEnums:      1,
			NumMessages:   4,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_debug_proto_goTypes,
		DependencyIndexes: file_debug_proto_depIdxs,
		EnumInfos:         file_debug_proto_enumTypes,
		MessageInfos:      file_debug_proto_msgTypes,
	}.Build()
	File_debug_proto = out.File
	file_debug_proto_rawDesc = nil
	file_debug_proto_goTypes = nil
	file_debug_proto_depIdxs = nil
}
