// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.26.0
// 	protoc        v3.20.1
// source: type/image.proto

package image

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

type ImageType int32

const (
	ImageType_IMAGE_TYPE_UNDEFINED ImageType = 0
	ImageType_IMAGE_TYPE_APNG      ImageType = 1 // image/apng
	ImageType_IMAGE_TYPE_AVIF      ImageType = 2 // image/avif
	ImageType_IMAGE_TYPE_GIF       ImageType = 3 // image/gif
	ImageType_IMAGE_TYPE_JPEG      ImageType = 4 // image/jpeg
	ImageType_IMAGE_TYPE_PNG       ImageType = 5 // image/png
	ImageType_IMAGE_TYPE_WEBP      ImageType = 6 // image/webp
)

// Enum value maps for ImageType.
var (
	ImageType_name = map[int32]string{
		0: "IMAGE_TYPE_UNDEFINED",
		1: "IMAGE_TYPE_APNG",
		2: "IMAGE_TYPE_AVIF",
		3: "IMAGE_TYPE_GIF",
		4: "IMAGE_TYPE_JPEG",
		5: "IMAGE_TYPE_PNG",
		6: "IMAGE_TYPE_WEBP",
	}
	ImageType_value = map[string]int32{
		"IMAGE_TYPE_UNDEFINED": 0,
		"IMAGE_TYPE_APNG":      1,
		"IMAGE_TYPE_AVIF":      2,
		"IMAGE_TYPE_GIF":       3,
		"IMAGE_TYPE_JPEG":      4,
		"IMAGE_TYPE_PNG":       5,
		"IMAGE_TYPE_WEBP":      6,
	}
)

func (x ImageType) Enum() *ImageType {
	p := new(ImageType)
	*p = x
	return p
}

func (x ImageType) String() string {
	return protoimpl.X.EnumStringOf(x.Descriptor(), protoreflect.EnumNumber(x))
}

func (ImageType) Descriptor() protoreflect.EnumDescriptor {
	return file_type_image_proto_enumTypes[0].Descriptor()
}

func (ImageType) Type() protoreflect.EnumType {
	return &file_type_image_proto_enumTypes[0]
}

func (x ImageType) Number() protoreflect.EnumNumber {
	return protoreflect.EnumNumber(x)
}

// Deprecated: Use ImageType.Descriptor instead.
func (ImageType) EnumDescriptor() ([]byte, []int) {
	return file_type_image_proto_rawDescGZIP(), []int{0}
}

type Image struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Type   ImageType `protobuf:"varint,1,opt,name=type,proto3,enum=strims.type.ImageType" json:"type,omitempty"`
	Height uint32    `protobuf:"varint,2,opt,name=height,proto3" json:"height,omitempty"`
	Width  uint32    `protobuf:"varint,3,opt,name=width,proto3" json:"width,omitempty"`
	Data   []byte    `protobuf:"bytes,4,opt,name=data,proto3" json:"data,omitempty"`
}

func (x *Image) Reset() {
	*x = Image{}
	if protoimpl.UnsafeEnabled {
		mi := &file_type_image_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *Image) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*Image) ProtoMessage() {}

func (x *Image) ProtoReflect() protoreflect.Message {
	mi := &file_type_image_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use Image.ProtoReflect.Descriptor instead.
func (*Image) Descriptor() ([]byte, []int) {
	return file_type_image_proto_rawDescGZIP(), []int{0}
}

func (x *Image) GetType() ImageType {
	if x != nil {
		return x.Type
	}
	return ImageType_IMAGE_TYPE_UNDEFINED
}

func (x *Image) GetHeight() uint32 {
	if x != nil {
		return x.Height
	}
	return 0
}

func (x *Image) GetWidth() uint32 {
	if x != nil {
		return x.Width
	}
	return 0
}

func (x *Image) GetData() []byte {
	if x != nil {
		return x.Data
	}
	return nil
}

var File_type_image_proto protoreflect.FileDescriptor

var file_type_image_proto_rawDesc = []byte{
	0x0a, 0x10, 0x74, 0x79, 0x70, 0x65, 0x2f, 0x69, 0x6d, 0x61, 0x67, 0x65, 0x2e, 0x70, 0x72, 0x6f,
	0x74, 0x6f, 0x12, 0x0b, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x22,
	0x75, 0x0a, 0x05, 0x49, 0x6d, 0x61, 0x67, 0x65, 0x12, 0x2a, 0x0a, 0x04, 0x74, 0x79, 0x70, 0x65,
	0x18, 0x01, 0x20, 0x01, 0x28, 0x0e, 0x32, 0x16, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e,
	0x74, 0x79, 0x70, 0x65, 0x2e, 0x49, 0x6d, 0x61, 0x67, 0x65, 0x54, 0x79, 0x70, 0x65, 0x52, 0x04,
	0x74, 0x79, 0x70, 0x65, 0x12, 0x16, 0x0a, 0x06, 0x68, 0x65, 0x69, 0x67, 0x68, 0x74, 0x18, 0x02,
	0x20, 0x01, 0x28, 0x0d, 0x52, 0x06, 0x68, 0x65, 0x69, 0x67, 0x68, 0x74, 0x12, 0x14, 0x0a, 0x05,
	0x77, 0x69, 0x64, 0x74, 0x68, 0x18, 0x03, 0x20, 0x01, 0x28, 0x0d, 0x52, 0x05, 0x77, 0x69, 0x64,
	0x74, 0x68, 0x12, 0x12, 0x0a, 0x04, 0x64, 0x61, 0x74, 0x61, 0x18, 0x04, 0x20, 0x01, 0x28, 0x0c,
	0x52, 0x04, 0x64, 0x61, 0x74, 0x61, 0x2a, 0xa1, 0x01, 0x0a, 0x09, 0x49, 0x6d, 0x61, 0x67, 0x65,
	0x54, 0x79, 0x70, 0x65, 0x12, 0x18, 0x0a, 0x14, 0x49, 0x4d, 0x41, 0x47, 0x45, 0x5f, 0x54, 0x59,
	0x50, 0x45, 0x5f, 0x55, 0x4e, 0x44, 0x45, 0x46, 0x49, 0x4e, 0x45, 0x44, 0x10, 0x00, 0x12, 0x13,
	0x0a, 0x0f, 0x49, 0x4d, 0x41, 0x47, 0x45, 0x5f, 0x54, 0x59, 0x50, 0x45, 0x5f, 0x41, 0x50, 0x4e,
	0x47, 0x10, 0x01, 0x12, 0x13, 0x0a, 0x0f, 0x49, 0x4d, 0x41, 0x47, 0x45, 0x5f, 0x54, 0x59, 0x50,
	0x45, 0x5f, 0x41, 0x56, 0x49, 0x46, 0x10, 0x02, 0x12, 0x12, 0x0a, 0x0e, 0x49, 0x4d, 0x41, 0x47,
	0x45, 0x5f, 0x54, 0x59, 0x50, 0x45, 0x5f, 0x47, 0x49, 0x46, 0x10, 0x03, 0x12, 0x13, 0x0a, 0x0f,
	0x49, 0x4d, 0x41, 0x47, 0x45, 0x5f, 0x54, 0x59, 0x50, 0x45, 0x5f, 0x4a, 0x50, 0x45, 0x47, 0x10,
	0x04, 0x12, 0x12, 0x0a, 0x0e, 0x49, 0x4d, 0x41, 0x47, 0x45, 0x5f, 0x54, 0x59, 0x50, 0x45, 0x5f,
	0x50, 0x4e, 0x47, 0x10, 0x05, 0x12, 0x13, 0x0a, 0x0f, 0x49, 0x4d, 0x41, 0x47, 0x45, 0x5f, 0x54,
	0x59, 0x50, 0x45, 0x5f, 0x57, 0x45, 0x42, 0x50, 0x10, 0x06, 0x42, 0x4c, 0x0a, 0x0e, 0x67, 0x67,
	0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x74, 0x79, 0x70, 0x65, 0x5a, 0x34, 0x67, 0x69,
	0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x4d, 0x65, 0x6d, 0x65, 0x4c, 0x61, 0x62,
	0x73, 0x2f, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2f, 0x70, 0x6b, 0x67, 0x2f, 0x61, 0x70, 0x69,
	0x73, 0x2f, 0x74, 0x79, 0x70, 0x65, 0x2f, 0x69, 0x6d, 0x61, 0x67, 0x65, 0x3b, 0x69, 0x6d, 0x61,
	0x67, 0x65, 0xba, 0x02, 0x03, 0x53, 0x54, 0x50, 0x62, 0x06, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_type_image_proto_rawDescOnce sync.Once
	file_type_image_proto_rawDescData = file_type_image_proto_rawDesc
)

func file_type_image_proto_rawDescGZIP() []byte {
	file_type_image_proto_rawDescOnce.Do(func() {
		file_type_image_proto_rawDescData = protoimpl.X.CompressGZIP(file_type_image_proto_rawDescData)
	})
	return file_type_image_proto_rawDescData
}

var file_type_image_proto_enumTypes = make([]protoimpl.EnumInfo, 1)
var file_type_image_proto_msgTypes = make([]protoimpl.MessageInfo, 1)
var file_type_image_proto_goTypes = []interface{}{
	(ImageType)(0), // 0: strims.type.ImageType
	(*Image)(nil),  // 1: strims.type.Image
}
var file_type_image_proto_depIdxs = []int32{
	0, // 0: strims.type.Image.type:type_name -> strims.type.ImageType
	1, // [1:1] is the sub-list for method output_type
	1, // [1:1] is the sub-list for method input_type
	1, // [1:1] is the sub-list for extension type_name
	1, // [1:1] is the sub-list for extension extendee
	0, // [0:1] is the sub-list for field type_name
}

func init() { file_type_image_proto_init() }
func file_type_image_proto_init() {
	if File_type_image_proto != nil {
		return
	}
	if !protoimpl.UnsafeEnabled {
		file_type_image_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*Image); i {
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
			RawDescriptor: file_type_image_proto_rawDesc,
			NumEnums:      1,
			NumMessages:   1,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_type_image_proto_goTypes,
		DependencyIndexes: file_type_image_proto_depIdxs,
		EnumInfos:         file_type_image_proto_enumTypes,
		MessageInfos:      file_type_image_proto_msgTypes,
	}.Build()
	File_type_image_proto = out.File
	file_type_image_proto_rawDesc = nil
	file_type_image_proto_goTypes = nil
	file_type_image_proto_depIdxs = nil
}
