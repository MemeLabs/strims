// Code generated by protoc-gen-go. DO NOT EDIT.
// source: api.proto

package service

import (
	fmt "fmt"
	proto "github.com/golang/protobuf/proto"
	math "math"
)

// Reference imports to suppress errors if they are not otherwise used.
var _ = proto.Marshal
var _ = fmt.Errorf
var _ = math.Inf

// This is a compile-time assertion to ensure that this generated file
// is compatible with the proto package it is being compiled against.
// A compilation error at this line likely means your copy of the
// proto package needs to be updated.
const _ = proto.ProtoPackageIsVersion3 // please upgrade the proto package

type JoinSwarmRequest struct {
	SwarmUri             string   `protobuf:"bytes,1,opt,name=swarmUri,proto3" json:"swarmUri,omitempty"`
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *JoinSwarmRequest) Reset()         { *m = JoinSwarmRequest{} }
func (m *JoinSwarmRequest) String() string { return proto.CompactTextString(m) }
func (*JoinSwarmRequest) ProtoMessage()    {}
func (*JoinSwarmRequest) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{0}
}

func (m *JoinSwarmRequest) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_JoinSwarmRequest.Unmarshal(m, b)
}
func (m *JoinSwarmRequest) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_JoinSwarmRequest.Marshal(b, m, deterministic)
}
func (m *JoinSwarmRequest) XXX_Merge(src proto.Message) {
	xxx_messageInfo_JoinSwarmRequest.Merge(m, src)
}
func (m *JoinSwarmRequest) XXX_Size() int {
	return xxx_messageInfo_JoinSwarmRequest.Size(m)
}
func (m *JoinSwarmRequest) XXX_DiscardUnknown() {
	xxx_messageInfo_JoinSwarmRequest.DiscardUnknown(m)
}

var xxx_messageInfo_JoinSwarmRequest proto.InternalMessageInfo

func (m *JoinSwarmRequest) GetSwarmUri() string {
	if m != nil {
		return m.SwarmUri
	}
	return ""
}

type JoinSwarmResponse struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *JoinSwarmResponse) Reset()         { *m = JoinSwarmResponse{} }
func (m *JoinSwarmResponse) String() string { return proto.CompactTextString(m) }
func (*JoinSwarmResponse) ProtoMessage()    {}
func (*JoinSwarmResponse) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{1}
}

func (m *JoinSwarmResponse) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_JoinSwarmResponse.Unmarshal(m, b)
}
func (m *JoinSwarmResponse) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_JoinSwarmResponse.Marshal(b, m, deterministic)
}
func (m *JoinSwarmResponse) XXX_Merge(src proto.Message) {
	xxx_messageInfo_JoinSwarmResponse.Merge(m, src)
}
func (m *JoinSwarmResponse) XXX_Size() int {
	return xxx_messageInfo_JoinSwarmResponse.Size(m)
}
func (m *JoinSwarmResponse) XXX_DiscardUnknown() {
	xxx_messageInfo_JoinSwarmResponse.DiscardUnknown(m)
}

var xxx_messageInfo_JoinSwarmResponse proto.InternalMessageInfo

type LeaveSwarmRequest struct {
	SwarmUri             string   `protobuf:"bytes,1,opt,name=swarmUri,proto3" json:"swarmUri,omitempty"`
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *LeaveSwarmRequest) Reset()         { *m = LeaveSwarmRequest{} }
func (m *LeaveSwarmRequest) String() string { return proto.CompactTextString(m) }
func (*LeaveSwarmRequest) ProtoMessage()    {}
func (*LeaveSwarmRequest) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{2}
}

func (m *LeaveSwarmRequest) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_LeaveSwarmRequest.Unmarshal(m, b)
}
func (m *LeaveSwarmRequest) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_LeaveSwarmRequest.Marshal(b, m, deterministic)
}
func (m *LeaveSwarmRequest) XXX_Merge(src proto.Message) {
	xxx_messageInfo_LeaveSwarmRequest.Merge(m, src)
}
func (m *LeaveSwarmRequest) XXX_Size() int {
	return xxx_messageInfo_LeaveSwarmRequest.Size(m)
}
func (m *LeaveSwarmRequest) XXX_DiscardUnknown() {
	xxx_messageInfo_LeaveSwarmRequest.DiscardUnknown(m)
}

var xxx_messageInfo_LeaveSwarmRequest proto.InternalMessageInfo

func (m *LeaveSwarmRequest) GetSwarmUri() string {
	if m != nil {
		return m.SwarmUri
	}
	return ""
}

type LeaveSwarmResponse struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *LeaveSwarmResponse) Reset()         { *m = LeaveSwarmResponse{} }
func (m *LeaveSwarmResponse) String() string { return proto.CompactTextString(m) }
func (*LeaveSwarmResponse) ProtoMessage()    {}
func (*LeaveSwarmResponse) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{3}
}

func (m *LeaveSwarmResponse) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_LeaveSwarmResponse.Unmarshal(m, b)
}
func (m *LeaveSwarmResponse) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_LeaveSwarmResponse.Marshal(b, m, deterministic)
}
func (m *LeaveSwarmResponse) XXX_Merge(src proto.Message) {
	xxx_messageInfo_LeaveSwarmResponse.Merge(m, src)
}
func (m *LeaveSwarmResponse) XXX_Size() int {
	return xxx_messageInfo_LeaveSwarmResponse.Size(m)
}
func (m *LeaveSwarmResponse) XXX_DiscardUnknown() {
	xxx_messageInfo_LeaveSwarmResponse.DiscardUnknown(m)
}

var xxx_messageInfo_LeaveSwarmResponse proto.InternalMessageInfo

type GetIngressStreamsRequest struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *GetIngressStreamsRequest) Reset()         { *m = GetIngressStreamsRequest{} }
func (m *GetIngressStreamsRequest) String() string { return proto.CompactTextString(m) }
func (*GetIngressStreamsRequest) ProtoMessage()    {}
func (*GetIngressStreamsRequest) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{4}
}

func (m *GetIngressStreamsRequest) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_GetIngressStreamsRequest.Unmarshal(m, b)
}
func (m *GetIngressStreamsRequest) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_GetIngressStreamsRequest.Marshal(b, m, deterministic)
}
func (m *GetIngressStreamsRequest) XXX_Merge(src proto.Message) {
	xxx_messageInfo_GetIngressStreamsRequest.Merge(m, src)
}
func (m *GetIngressStreamsRequest) XXX_Size() int {
	return xxx_messageInfo_GetIngressStreamsRequest.Size(m)
}
func (m *GetIngressStreamsRequest) XXX_DiscardUnknown() {
	xxx_messageInfo_GetIngressStreamsRequest.DiscardUnknown(m)
}

var xxx_messageInfo_GetIngressStreamsRequest proto.InternalMessageInfo

type GetIngressStreamsResponse struct {
	SwarmUri             string   `protobuf:"bytes,1,opt,name=swarmUri,proto3" json:"swarmUri,omitempty"`
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *GetIngressStreamsResponse) Reset()         { *m = GetIngressStreamsResponse{} }
func (m *GetIngressStreamsResponse) String() string { return proto.CompactTextString(m) }
func (*GetIngressStreamsResponse) ProtoMessage()    {}
func (*GetIngressStreamsResponse) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{5}
}

func (m *GetIngressStreamsResponse) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_GetIngressStreamsResponse.Unmarshal(m, b)
}
func (m *GetIngressStreamsResponse) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_GetIngressStreamsResponse.Marshal(b, m, deterministic)
}
func (m *GetIngressStreamsResponse) XXX_Merge(src proto.Message) {
	xxx_messageInfo_GetIngressStreamsResponse.Merge(m, src)
}
func (m *GetIngressStreamsResponse) XXX_Size() int {
	return xxx_messageInfo_GetIngressStreamsResponse.Size(m)
}
func (m *GetIngressStreamsResponse) XXX_DiscardUnknown() {
	xxx_messageInfo_GetIngressStreamsResponse.DiscardUnknown(m)
}

var xxx_messageInfo_GetIngressStreamsResponse proto.InternalMessageInfo

func (m *GetIngressStreamsResponse) GetSwarmUri() string {
	if m != nil {
		return m.SwarmUri
	}
	return ""
}

type StartHLSIngressRequest struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *StartHLSIngressRequest) Reset()         { *m = StartHLSIngressRequest{} }
func (m *StartHLSIngressRequest) String() string { return proto.CompactTextString(m) }
func (*StartHLSIngressRequest) ProtoMessage()    {}
func (*StartHLSIngressRequest) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{6}
}

func (m *StartHLSIngressRequest) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_StartHLSIngressRequest.Unmarshal(m, b)
}
func (m *StartHLSIngressRequest) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_StartHLSIngressRequest.Marshal(b, m, deterministic)
}
func (m *StartHLSIngressRequest) XXX_Merge(src proto.Message) {
	xxx_messageInfo_StartHLSIngressRequest.Merge(m, src)
}
func (m *StartHLSIngressRequest) XXX_Size() int {
	return xxx_messageInfo_StartHLSIngressRequest.Size(m)
}
func (m *StartHLSIngressRequest) XXX_DiscardUnknown() {
	xxx_messageInfo_StartHLSIngressRequest.DiscardUnknown(m)
}

var xxx_messageInfo_StartHLSIngressRequest proto.InternalMessageInfo

type StartHLSIngressResponse struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *StartHLSIngressResponse) Reset()         { *m = StartHLSIngressResponse{} }
func (m *StartHLSIngressResponse) String() string { return proto.CompactTextString(m) }
func (*StartHLSIngressResponse) ProtoMessage()    {}
func (*StartHLSIngressResponse) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{7}
}

func (m *StartHLSIngressResponse) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_StartHLSIngressResponse.Unmarshal(m, b)
}
func (m *StartHLSIngressResponse) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_StartHLSIngressResponse.Marshal(b, m, deterministic)
}
func (m *StartHLSIngressResponse) XXX_Merge(src proto.Message) {
	xxx_messageInfo_StartHLSIngressResponse.Merge(m, src)
}
func (m *StartHLSIngressResponse) XXX_Size() int {
	return xxx_messageInfo_StartHLSIngressResponse.Size(m)
}
func (m *StartHLSIngressResponse) XXX_DiscardUnknown() {
	xxx_messageInfo_StartHLSIngressResponse.DiscardUnknown(m)
}

var xxx_messageInfo_StartHLSIngressResponse proto.InternalMessageInfo

type StartHLSEgressRequest struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *StartHLSEgressRequest) Reset()         { *m = StartHLSEgressRequest{} }
func (m *StartHLSEgressRequest) String() string { return proto.CompactTextString(m) }
func (*StartHLSEgressRequest) ProtoMessage()    {}
func (*StartHLSEgressRequest) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{8}
}

func (m *StartHLSEgressRequest) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_StartHLSEgressRequest.Unmarshal(m, b)
}
func (m *StartHLSEgressRequest) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_StartHLSEgressRequest.Marshal(b, m, deterministic)
}
func (m *StartHLSEgressRequest) XXX_Merge(src proto.Message) {
	xxx_messageInfo_StartHLSEgressRequest.Merge(m, src)
}
func (m *StartHLSEgressRequest) XXX_Size() int {
	return xxx_messageInfo_StartHLSEgressRequest.Size(m)
}
func (m *StartHLSEgressRequest) XXX_DiscardUnknown() {
	xxx_messageInfo_StartHLSEgressRequest.DiscardUnknown(m)
}

var xxx_messageInfo_StartHLSEgressRequest proto.InternalMessageInfo

type StartHLSEgressResponse struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *StartHLSEgressResponse) Reset()         { *m = StartHLSEgressResponse{} }
func (m *StartHLSEgressResponse) String() string { return proto.CompactTextString(m) }
func (*StartHLSEgressResponse) ProtoMessage()    {}
func (*StartHLSEgressResponse) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{9}
}

func (m *StartHLSEgressResponse) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_StartHLSEgressResponse.Unmarshal(m, b)
}
func (m *StartHLSEgressResponse) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_StartHLSEgressResponse.Marshal(b, m, deterministic)
}
func (m *StartHLSEgressResponse) XXX_Merge(src proto.Message) {
	xxx_messageInfo_StartHLSEgressResponse.Merge(m, src)
}
func (m *StartHLSEgressResponse) XXX_Size() int {
	return xxx_messageInfo_StartHLSEgressResponse.Size(m)
}
func (m *StartHLSEgressResponse) XXX_DiscardUnknown() {
	xxx_messageInfo_StartHLSEgressResponse.DiscardUnknown(m)
}

var xxx_messageInfo_StartHLSEgressResponse proto.InternalMessageInfo

type StopHLSEgressRequest struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *StopHLSEgressRequest) Reset()         { *m = StopHLSEgressRequest{} }
func (m *StopHLSEgressRequest) String() string { return proto.CompactTextString(m) }
func (*StopHLSEgressRequest) ProtoMessage()    {}
func (*StopHLSEgressRequest) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{10}
}

func (m *StopHLSEgressRequest) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_StopHLSEgressRequest.Unmarshal(m, b)
}
func (m *StopHLSEgressRequest) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_StopHLSEgressRequest.Marshal(b, m, deterministic)
}
func (m *StopHLSEgressRequest) XXX_Merge(src proto.Message) {
	xxx_messageInfo_StopHLSEgressRequest.Merge(m, src)
}
func (m *StopHLSEgressRequest) XXX_Size() int {
	return xxx_messageInfo_StopHLSEgressRequest.Size(m)
}
func (m *StopHLSEgressRequest) XXX_DiscardUnknown() {
	xxx_messageInfo_StopHLSEgressRequest.DiscardUnknown(m)
}

var xxx_messageInfo_StopHLSEgressRequest proto.InternalMessageInfo

type StopHLSEgressResponse struct {
	XXX_NoUnkeyedLiteral struct{} `json:"-"`
	XXX_unrecognized     []byte   `json:"-"`
	XXX_sizecache        int32    `json:"-"`
}

func (m *StopHLSEgressResponse) Reset()         { *m = StopHLSEgressResponse{} }
func (m *StopHLSEgressResponse) String() string { return proto.CompactTextString(m) }
func (*StopHLSEgressResponse) ProtoMessage()    {}
func (*StopHLSEgressResponse) Descriptor() ([]byte, []int) {
	return fileDescriptor_00212fb1f9d3bf1c, []int{11}
}

func (m *StopHLSEgressResponse) XXX_Unmarshal(b []byte) error {
	return xxx_messageInfo_StopHLSEgressResponse.Unmarshal(m, b)
}
func (m *StopHLSEgressResponse) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	return xxx_messageInfo_StopHLSEgressResponse.Marshal(b, m, deterministic)
}
func (m *StopHLSEgressResponse) XXX_Merge(src proto.Message) {
	xxx_messageInfo_StopHLSEgressResponse.Merge(m, src)
}
func (m *StopHLSEgressResponse) XXX_Size() int {
	return xxx_messageInfo_StopHLSEgressResponse.Size(m)
}
func (m *StopHLSEgressResponse) XXX_DiscardUnknown() {
	xxx_messageInfo_StopHLSEgressResponse.DiscardUnknown(m)
}

var xxx_messageInfo_StopHLSEgressResponse proto.InternalMessageInfo

func init() {
	proto.RegisterType((*JoinSwarmRequest)(nil), "JoinSwarmRequest")
	proto.RegisterType((*JoinSwarmResponse)(nil), "JoinSwarmResponse")
	proto.RegisterType((*LeaveSwarmRequest)(nil), "LeaveSwarmRequest")
	proto.RegisterType((*LeaveSwarmResponse)(nil), "LeaveSwarmResponse")
	proto.RegisterType((*GetIngressStreamsRequest)(nil), "GetIngressStreamsRequest")
	proto.RegisterType((*GetIngressStreamsResponse)(nil), "GetIngressStreamsResponse")
	proto.RegisterType((*StartHLSIngressRequest)(nil), "StartHLSIngressRequest")
	proto.RegisterType((*StartHLSIngressResponse)(nil), "StartHLSIngressResponse")
	proto.RegisterType((*StartHLSEgressRequest)(nil), "StartHLSEgressRequest")
	proto.RegisterType((*StartHLSEgressResponse)(nil), "StartHLSEgressResponse")
	proto.RegisterType((*StopHLSEgressRequest)(nil), "StopHLSEgressRequest")
	proto.RegisterType((*StopHLSEgressResponse)(nil), "StopHLSEgressResponse")
}

func init() { proto.RegisterFile("api.proto", fileDescriptor_00212fb1f9d3bf1c) }

var fileDescriptor_00212fb1f9d3bf1c = []byte{
	// 204 bytes of a gzipped FileDescriptorProto
	0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xe2, 0xe2, 0x4c, 0x2c, 0xc8, 0xd4,
	0x2b, 0x28, 0xca, 0x2f, 0xc9, 0x57, 0xd2, 0xe3, 0x12, 0xf0, 0xca, 0xcf, 0xcc, 0x0b, 0x2e, 0x4f,
	0x2c, 0xca, 0x0d, 0x4a, 0x2d, 0x2c, 0x4d, 0x2d, 0x2e, 0x11, 0x92, 0xe2, 0xe2, 0x28, 0x06, 0xf1,
	0x43, 0x8b, 0x32, 0x25, 0x18, 0x15, 0x18, 0x35, 0x38, 0x83, 0xe0, 0x7c, 0x25, 0x61, 0x2e, 0x41,
	0x24, 0xf5, 0xc5, 0x05, 0xf9, 0x79, 0xc5, 0xa9, 0x4a, 0xfa, 0x5c, 0x82, 0x3e, 0xa9, 0x89, 0x65,
	0xa9, 0x44, 0x9b, 0x22, 0xc2, 0x25, 0x84, 0xac, 0x01, 0x6a, 0x8c, 0x14, 0x97, 0x84, 0x7b, 0x6a,
	0x89, 0x67, 0x5e, 0x7a, 0x51, 0x6a, 0x71, 0x71, 0x70, 0x49, 0x51, 0x6a, 0x62, 0x6e, 0x31, 0xd4,
	0x34, 0x25, 0x73, 0x2e, 0x49, 0x2c, 0x72, 0x10, 0x8d, 0x78, 0xad, 0x92, 0xe0, 0x12, 0x0b, 0x2e,
	0x49, 0x2c, 0x2a, 0xf1, 0xf0, 0x09, 0x86, 0xea, 0x86, 0x19, 0x29, 0xc9, 0x25, 0x8e, 0x21, 0x03,
	0x75, 0x89, 0x38, 0x97, 0x28, 0x4c, 0xca, 0x15, 0x45, 0x0f, 0x92, 0x69, 0xae, 0xa8, 0x5a, 0xc4,
	0xb8, 0x44, 0x82, 0x4b, 0xf2, 0x0b, 0x30, 0x74, 0x80, 0x8d, 0x42, 0x11, 0x87, 0x68, 0x70, 0xe2,
	0x8c, 0x62, 0x2f, 0x4e, 0x2d, 0x2a, 0xcb, 0x4c, 0x4e, 0x4d, 0x62, 0x03, 0xc7, 0x85, 0x31, 0x20,
	0x00, 0x00, 0xff, 0xff, 0x82, 0xcc, 0xd5, 0x84, 0x98, 0x01, 0x00, 0x00,
}
