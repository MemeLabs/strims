// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.26.0
// 	protoc        v3.20.1
// source: chat/v1/events.proto

package chatv1

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

type ServerChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Server *Server `protobuf:"bytes,1,opt,name=server,proto3" json:"server,omitempty"`
}

func (x *ServerChangeEvent) Reset() {
	*x = ServerChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ServerChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ServerChangeEvent) ProtoMessage() {}

func (x *ServerChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ServerChangeEvent.ProtoReflect.Descriptor instead.
func (*ServerChangeEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{0}
}

func (x *ServerChangeEvent) GetServer() *Server {
	if x != nil {
		return x.Server
	}
	return nil
}

type ServerDeleteEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Server *Server `protobuf:"bytes,1,opt,name=server,proto3" json:"server,omitempty"`
}

func (x *ServerDeleteEvent) Reset() {
	*x = ServerDeleteEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[1]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ServerDeleteEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ServerDeleteEvent) ProtoMessage() {}

func (x *ServerDeleteEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[1]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ServerDeleteEvent.ProtoReflect.Descriptor instead.
func (*ServerDeleteEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{1}
}

func (x *ServerDeleteEvent) GetServer() *Server {
	if x != nil {
		return x.Server
	}
	return nil
}

type EmoteChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Emote *Emote `protobuf:"bytes,1,opt,name=emote,proto3" json:"emote,omitempty"`
}

func (x *EmoteChangeEvent) Reset() {
	*x = EmoteChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[2]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *EmoteChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*EmoteChangeEvent) ProtoMessage() {}

func (x *EmoteChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[2]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use EmoteChangeEvent.ProtoReflect.Descriptor instead.
func (*EmoteChangeEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{2}
}

func (x *EmoteChangeEvent) GetEmote() *Emote {
	if x != nil {
		return x.Emote
	}
	return nil
}

type EmoteDeleteEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Emote *Emote `protobuf:"bytes,1,opt,name=emote,proto3" json:"emote,omitempty"`
}

func (x *EmoteDeleteEvent) Reset() {
	*x = EmoteDeleteEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[3]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *EmoteDeleteEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*EmoteDeleteEvent) ProtoMessage() {}

func (x *EmoteDeleteEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[3]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use EmoteDeleteEvent.ProtoReflect.Descriptor instead.
func (*EmoteDeleteEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{3}
}

func (x *EmoteDeleteEvent) GetEmote() *Emote {
	if x != nil {
		return x.Emote
	}
	return nil
}

type ModifierChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Modifier *Modifier `protobuf:"bytes,1,opt,name=modifier,proto3" json:"modifier,omitempty"`
}

func (x *ModifierChangeEvent) Reset() {
	*x = ModifierChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[4]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ModifierChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ModifierChangeEvent) ProtoMessage() {}

func (x *ModifierChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[4]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ModifierChangeEvent.ProtoReflect.Descriptor instead.
func (*ModifierChangeEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{4}
}

func (x *ModifierChangeEvent) GetModifier() *Modifier {
	if x != nil {
		return x.Modifier
	}
	return nil
}

type ModifierDeleteEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Modifier *Modifier `protobuf:"bytes,1,opt,name=modifier,proto3" json:"modifier,omitempty"`
}

func (x *ModifierDeleteEvent) Reset() {
	*x = ModifierDeleteEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[5]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *ModifierDeleteEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*ModifierDeleteEvent) ProtoMessage() {}

func (x *ModifierDeleteEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[5]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use ModifierDeleteEvent.ProtoReflect.Descriptor instead.
func (*ModifierDeleteEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{5}
}

func (x *ModifierDeleteEvent) GetModifier() *Modifier {
	if x != nil {
		return x.Modifier
	}
	return nil
}

type TagChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Tag *Tag `protobuf:"bytes,1,opt,name=tag,proto3" json:"tag,omitempty"`
}

func (x *TagChangeEvent) Reset() {
	*x = TagChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[6]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *TagChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*TagChangeEvent) ProtoMessage() {}

func (x *TagChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[6]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use TagChangeEvent.ProtoReflect.Descriptor instead.
func (*TagChangeEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{6}
}

func (x *TagChangeEvent) GetTag() *Tag {
	if x != nil {
		return x.Tag
	}
	return nil
}

type TagDeleteEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Tag *Tag `protobuf:"bytes,1,opt,name=tag,proto3" json:"tag,omitempty"`
}

func (x *TagDeleteEvent) Reset() {
	*x = TagDeleteEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[7]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *TagDeleteEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*TagDeleteEvent) ProtoMessage() {}

func (x *TagDeleteEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[7]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use TagDeleteEvent.ProtoReflect.Descriptor instead.
func (*TagDeleteEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{7}
}

func (x *TagDeleteEvent) GetTag() *Tag {
	if x != nil {
		return x.Tag
	}
	return nil
}

type UIConfigChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	UiConfig *UIConfig `protobuf:"bytes,1,opt,name=ui_config,json=uiConfig,proto3" json:"ui_config,omitempty"`
}

func (x *UIConfigChangeEvent) Reset() {
	*x = UIConfigChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[8]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *UIConfigChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*UIConfigChangeEvent) ProtoMessage() {}

func (x *UIConfigChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[8]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use UIConfigChangeEvent.ProtoReflect.Descriptor instead.
func (*UIConfigChangeEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{8}
}

func (x *UIConfigChangeEvent) GetUiConfig() *UIConfig {
	if x != nil {
		return x.UiConfig
	}
	return nil
}

type SyncAssetsEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	ServerId           uint64 `protobuf:"varint,1,opt,name=server_id,json=serverId,proto3" json:"server_id,omitempty"`
	ForceUnifiedUpdate bool   `protobuf:"varint,2,opt,name=force_unified_update,json=forceUnifiedUpdate,proto3" json:"force_unified_update,omitempty"`
}

func (x *SyncAssetsEvent) Reset() {
	*x = SyncAssetsEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[9]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *SyncAssetsEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*SyncAssetsEvent) ProtoMessage() {}

func (x *SyncAssetsEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[9]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use SyncAssetsEvent.ProtoReflect.Descriptor instead.
func (*SyncAssetsEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{9}
}

func (x *SyncAssetsEvent) GetServerId() uint64 {
	if x != nil {
		return x.ServerId
	}
	return 0
}

func (x *SyncAssetsEvent) GetForceUnifiedUpdate() bool {
	if x != nil {
		return x.ForceUnifiedUpdate
	}
	return false
}

type WhisperThreadChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	WhisperThread *WhisperThread `protobuf:"bytes,1,opt,name=whisper_thread,json=whisperThread,proto3" json:"whisper_thread,omitempty"`
}

func (x *WhisperThreadChangeEvent) Reset() {
	*x = WhisperThreadChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[10]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *WhisperThreadChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*WhisperThreadChangeEvent) ProtoMessage() {}

func (x *WhisperThreadChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[10]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use WhisperThreadChangeEvent.ProtoReflect.Descriptor instead.
func (*WhisperThreadChangeEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{10}
}

func (x *WhisperThreadChangeEvent) GetWhisperThread() *WhisperThread {
	if x != nil {
		return x.WhisperThread
	}
	return nil
}

type WhisperRecordChangeEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	WhisperRecord *WhisperRecord `protobuf:"bytes,1,opt,name=whisper_record,json=whisperRecord,proto3" json:"whisper_record,omitempty"`
}

func (x *WhisperRecordChangeEvent) Reset() {
	*x = WhisperRecordChangeEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[11]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *WhisperRecordChangeEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*WhisperRecordChangeEvent) ProtoMessage() {}

func (x *WhisperRecordChangeEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[11]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use WhisperRecordChangeEvent.ProtoReflect.Descriptor instead.
func (*WhisperRecordChangeEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{11}
}

func (x *WhisperRecordChangeEvent) GetWhisperRecord() *WhisperRecord {
	if x != nil {
		return x.WhisperRecord
	}
	return nil
}

type WhisperRecordDeleteEvent struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	WhisperRecord *WhisperRecord `protobuf:"bytes,1,opt,name=whisper_record,json=whisperRecord,proto3" json:"whisper_record,omitempty"`
}

func (x *WhisperRecordDeleteEvent) Reset() {
	*x = WhisperRecordDeleteEvent{}
	if protoimpl.UnsafeEnabled {
		mi := &file_chat_v1_events_proto_msgTypes[12]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *WhisperRecordDeleteEvent) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*WhisperRecordDeleteEvent) ProtoMessage() {}

func (x *WhisperRecordDeleteEvent) ProtoReflect() protoreflect.Message {
	mi := &file_chat_v1_events_proto_msgTypes[12]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use WhisperRecordDeleteEvent.ProtoReflect.Descriptor instead.
func (*WhisperRecordDeleteEvent) Descriptor() ([]byte, []int) {
	return file_chat_v1_events_proto_rawDescGZIP(), []int{12}
}

func (x *WhisperRecordDeleteEvent) GetWhisperRecord() *WhisperRecord {
	if x != nil {
		return x.WhisperRecord
	}
	return nil
}

var File_chat_v1_events_proto protoreflect.FileDescriptor

var file_chat_v1_events_proto_rawDesc = []byte{
	0x0a, 0x14, 0x63, 0x68, 0x61, 0x74, 0x2f, 0x76, 0x31, 0x2f, 0x65, 0x76, 0x65, 0x6e, 0x74, 0x73,
	0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x12, 0x0e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x63,
	0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x1a, 0x12, 0x63, 0x68, 0x61, 0x74, 0x2f, 0x76, 0x31, 0x2f,
	0x63, 0x68, 0x61, 0x74, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x22, 0x43, 0x0a, 0x11, 0x53, 0x65,
	0x72, 0x76, 0x65, 0x72, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12,
	0x2e, 0x0a, 0x06, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32,
	0x16, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31,
	0x2e, 0x53, 0x65, 0x72, 0x76, 0x65, 0x72, 0x52, 0x06, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x22,
	0x43, 0x0a, 0x11, 0x53, 0x65, 0x72, 0x76, 0x65, 0x72, 0x44, 0x65, 0x6c, 0x65, 0x74, 0x65, 0x45,
	0x76, 0x65, 0x6e, 0x74, 0x12, 0x2e, 0x0a, 0x06, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x18, 0x01,
	0x20, 0x01, 0x28, 0x0b, 0x32, 0x16, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x63, 0x68,
	0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x53, 0x65, 0x72, 0x76, 0x65, 0x72, 0x52, 0x06, 0x73, 0x65,
	0x72, 0x76, 0x65, 0x72, 0x22, 0x3f, 0x0a, 0x10, 0x45, 0x6d, 0x6f, 0x74, 0x65, 0x43, 0x68, 0x61,
	0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x2b, 0x0a, 0x05, 0x65, 0x6d, 0x6f, 0x74,
	0x65, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x15, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73,
	0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x45, 0x6d, 0x6f, 0x74, 0x65, 0x52, 0x05,
	0x65, 0x6d, 0x6f, 0x74, 0x65, 0x22, 0x3f, 0x0a, 0x10, 0x45, 0x6d, 0x6f, 0x74, 0x65, 0x44, 0x65,
	0x6c, 0x65, 0x74, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x2b, 0x0a, 0x05, 0x65, 0x6d, 0x6f,
	0x74, 0x65, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x15, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d,
	0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x45, 0x6d, 0x6f, 0x74, 0x65, 0x52,
	0x05, 0x65, 0x6d, 0x6f, 0x74, 0x65, 0x22, 0x4b, 0x0a, 0x13, 0x4d, 0x6f, 0x64, 0x69, 0x66, 0x69,
	0x65, 0x72, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x34, 0x0a,
	0x08, 0x6d, 0x6f, 0x64, 0x69, 0x66, 0x69, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32,
	0x18, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31,
	0x2e, 0x4d, 0x6f, 0x64, 0x69, 0x66, 0x69, 0x65, 0x72, 0x52, 0x08, 0x6d, 0x6f, 0x64, 0x69, 0x66,
	0x69, 0x65, 0x72, 0x22, 0x4b, 0x0a, 0x13, 0x4d, 0x6f, 0x64, 0x69, 0x66, 0x69, 0x65, 0x72, 0x44,
	0x65, 0x6c, 0x65, 0x74, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x34, 0x0a, 0x08, 0x6d, 0x6f,
	0x64, 0x69, 0x66, 0x69, 0x65, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x18, 0x2e, 0x73,
	0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x4d, 0x6f,
	0x64, 0x69, 0x66, 0x69, 0x65, 0x72, 0x52, 0x08, 0x6d, 0x6f, 0x64, 0x69, 0x66, 0x69, 0x65, 0x72,
	0x22, 0x37, 0x0a, 0x0e, 0x54, 0x61, 0x67, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65,
	0x6e, 0x74, 0x12, 0x25, 0x0a, 0x03, 0x74, 0x61, 0x67, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32,
	0x13, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31,
	0x2e, 0x54, 0x61, 0x67, 0x52, 0x03, 0x74, 0x61, 0x67, 0x22, 0x37, 0x0a, 0x0e, 0x54, 0x61, 0x67,
	0x44, 0x65, 0x6c, 0x65, 0x74, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x25, 0x0a, 0x03, 0x74,
	0x61, 0x67, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x13, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d,
	0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x54, 0x61, 0x67, 0x52, 0x03, 0x74,
	0x61, 0x67, 0x22, 0x4c, 0x0a, 0x13, 0x55, 0x49, 0x43, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x43, 0x68,
	0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x35, 0x0a, 0x09, 0x75, 0x69, 0x5f,
	0x63, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x18, 0x2e, 0x73,
	0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x55, 0x49,
	0x43, 0x6f, 0x6e, 0x66, 0x69, 0x67, 0x52, 0x08, 0x75, 0x69, 0x43, 0x6f, 0x6e, 0x66, 0x69, 0x67,
	0x22, 0x60, 0x0a, 0x0f, 0x53, 0x79, 0x6e, 0x63, 0x41, 0x73, 0x73, 0x65, 0x74, 0x73, 0x45, 0x76,
	0x65, 0x6e, 0x74, 0x12, 0x1b, 0x0a, 0x09, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x5f, 0x69, 0x64,
	0x18, 0x01, 0x20, 0x01, 0x28, 0x04, 0x52, 0x08, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x49, 0x64,
	0x12, 0x30, 0x0a, 0x14, 0x66, 0x6f, 0x72, 0x63, 0x65, 0x5f, 0x75, 0x6e, 0x69, 0x66, 0x69, 0x65,
	0x64, 0x5f, 0x75, 0x70, 0x64, 0x61, 0x74, 0x65, 0x18, 0x02, 0x20, 0x01, 0x28, 0x08, 0x52, 0x12,
	0x66, 0x6f, 0x72, 0x63, 0x65, 0x55, 0x6e, 0x69, 0x66, 0x69, 0x65, 0x64, 0x55, 0x70, 0x64, 0x61,
	0x74, 0x65, 0x22, 0x60, 0x0a, 0x18, 0x57, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72, 0x54, 0x68, 0x72,
	0x65, 0x61, 0x64, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74, 0x12, 0x44,
	0x0a, 0x0e, 0x77, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72, 0x5f, 0x74, 0x68, 0x72, 0x65, 0x61, 0x64,
	0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x1d, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e,
	0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x57, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72, 0x54,
	0x68, 0x72, 0x65, 0x61, 0x64, 0x52, 0x0d, 0x77, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72, 0x54, 0x68,
	0x72, 0x65, 0x61, 0x64, 0x22, 0x60, 0x0a, 0x18, 0x57, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72, 0x52,
	0x65, 0x63, 0x6f, 0x72, 0x64, 0x43, 0x68, 0x61, 0x6e, 0x67, 0x65, 0x45, 0x76, 0x65, 0x6e, 0x74,
	0x12, 0x44, 0x0a, 0x0e, 0x77, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72, 0x5f, 0x72, 0x65, 0x63, 0x6f,
	0x72, 0x64, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x1d, 0x2e, 0x73, 0x74, 0x72, 0x69, 0x6d,
	0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x57, 0x68, 0x69, 0x73, 0x70, 0x65,
	0x72, 0x52, 0x65, 0x63, 0x6f, 0x72, 0x64, 0x52, 0x0d, 0x77, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72,
	0x52, 0x65, 0x63, 0x6f, 0x72, 0x64, 0x22, 0x60, 0x0a, 0x18, 0x57, 0x68, 0x69, 0x73, 0x70, 0x65,
	0x72, 0x52, 0x65, 0x63, 0x6f, 0x72, 0x64, 0x44, 0x65, 0x6c, 0x65, 0x74, 0x65, 0x45, 0x76, 0x65,
	0x6e, 0x74, 0x12, 0x44, 0x0a, 0x0e, 0x77, 0x68, 0x69, 0x73, 0x70, 0x65, 0x72, 0x5f, 0x72, 0x65,
	0x63, 0x6f, 0x72, 0x64, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32, 0x1d, 0x2e, 0x73, 0x74, 0x72,
	0x69, 0x6d, 0x73, 0x2e, 0x63, 0x68, 0x61, 0x74, 0x2e, 0x76, 0x31, 0x2e, 0x57, 0x68, 0x69, 0x73,
	0x70, 0x65, 0x72, 0x52, 0x65, 0x63, 0x6f, 0x72, 0x64, 0x52, 0x0d, 0x77, 0x68, 0x69, 0x73, 0x70,
	0x65, 0x72, 0x52, 0x65, 0x63, 0x6f, 0x72, 0x64, 0x42, 0x51, 0x0a, 0x15, 0x67, 0x67, 0x2e, 0x73,
	0x74, 0x72, 0x69, 0x6d, 0x73, 0x2e, 0x70, 0x70, 0x73, 0x70, 0x70, 0x2e, 0x70, 0x72, 0x6f, 0x74,
	0x6f, 0x5a, 0x32, 0x67, 0x69, 0x74, 0x68, 0x75, 0x62, 0x2e, 0x63, 0x6f, 0x6d, 0x2f, 0x4d, 0x65,
	0x6d, 0x65, 0x4c, 0x61, 0x62, 0x73, 0x2f, 0x73, 0x74, 0x72, 0x69, 0x6d, 0x73, 0x2f, 0x70, 0x6b,
	0x67, 0x2f, 0x61, 0x70, 0x69, 0x73, 0x2f, 0x63, 0x68, 0x61, 0x74, 0x2f, 0x76, 0x31, 0x3b, 0x63,
	0x68, 0x61, 0x74, 0x76, 0x31, 0xba, 0x02, 0x03, 0x53, 0x43, 0x48, 0x62, 0x06, 0x70, 0x72, 0x6f,
	0x74, 0x6f, 0x33,
}

var (
	file_chat_v1_events_proto_rawDescOnce sync.Once
	file_chat_v1_events_proto_rawDescData = file_chat_v1_events_proto_rawDesc
)

func file_chat_v1_events_proto_rawDescGZIP() []byte {
	file_chat_v1_events_proto_rawDescOnce.Do(func() {
		file_chat_v1_events_proto_rawDescData = protoimpl.X.CompressGZIP(file_chat_v1_events_proto_rawDescData)
	})
	return file_chat_v1_events_proto_rawDescData
}

var file_chat_v1_events_proto_msgTypes = make([]protoimpl.MessageInfo, 13)
var file_chat_v1_events_proto_goTypes = []interface{}{
	(*ServerChangeEvent)(nil),        // 0: strims.chat.v1.ServerChangeEvent
	(*ServerDeleteEvent)(nil),        // 1: strims.chat.v1.ServerDeleteEvent
	(*EmoteChangeEvent)(nil),         // 2: strims.chat.v1.EmoteChangeEvent
	(*EmoteDeleteEvent)(nil),         // 3: strims.chat.v1.EmoteDeleteEvent
	(*ModifierChangeEvent)(nil),      // 4: strims.chat.v1.ModifierChangeEvent
	(*ModifierDeleteEvent)(nil),      // 5: strims.chat.v1.ModifierDeleteEvent
	(*TagChangeEvent)(nil),           // 6: strims.chat.v1.TagChangeEvent
	(*TagDeleteEvent)(nil),           // 7: strims.chat.v1.TagDeleteEvent
	(*UIConfigChangeEvent)(nil),      // 8: strims.chat.v1.UIConfigChangeEvent
	(*SyncAssetsEvent)(nil),          // 9: strims.chat.v1.SyncAssetsEvent
	(*WhisperThreadChangeEvent)(nil), // 10: strims.chat.v1.WhisperThreadChangeEvent
	(*WhisperRecordChangeEvent)(nil), // 11: strims.chat.v1.WhisperRecordChangeEvent
	(*WhisperRecordDeleteEvent)(nil), // 12: strims.chat.v1.WhisperRecordDeleteEvent
	(*Server)(nil),                   // 13: strims.chat.v1.Server
	(*Emote)(nil),                    // 14: strims.chat.v1.Emote
	(*Modifier)(nil),                 // 15: strims.chat.v1.Modifier
	(*Tag)(nil),                      // 16: strims.chat.v1.Tag
	(*UIConfig)(nil),                 // 17: strims.chat.v1.UIConfig
	(*WhisperThread)(nil),            // 18: strims.chat.v1.WhisperThread
	(*WhisperRecord)(nil),            // 19: strims.chat.v1.WhisperRecord
}
var file_chat_v1_events_proto_depIdxs = []int32{
	13, // 0: strims.chat.v1.ServerChangeEvent.server:type_name -> strims.chat.v1.Server
	13, // 1: strims.chat.v1.ServerDeleteEvent.server:type_name -> strims.chat.v1.Server
	14, // 2: strims.chat.v1.EmoteChangeEvent.emote:type_name -> strims.chat.v1.Emote
	14, // 3: strims.chat.v1.EmoteDeleteEvent.emote:type_name -> strims.chat.v1.Emote
	15, // 4: strims.chat.v1.ModifierChangeEvent.modifier:type_name -> strims.chat.v1.Modifier
	15, // 5: strims.chat.v1.ModifierDeleteEvent.modifier:type_name -> strims.chat.v1.Modifier
	16, // 6: strims.chat.v1.TagChangeEvent.tag:type_name -> strims.chat.v1.Tag
	16, // 7: strims.chat.v1.TagDeleteEvent.tag:type_name -> strims.chat.v1.Tag
	17, // 8: strims.chat.v1.UIConfigChangeEvent.ui_config:type_name -> strims.chat.v1.UIConfig
	18, // 9: strims.chat.v1.WhisperThreadChangeEvent.whisper_thread:type_name -> strims.chat.v1.WhisperThread
	19, // 10: strims.chat.v1.WhisperRecordChangeEvent.whisper_record:type_name -> strims.chat.v1.WhisperRecord
	19, // 11: strims.chat.v1.WhisperRecordDeleteEvent.whisper_record:type_name -> strims.chat.v1.WhisperRecord
	12, // [12:12] is the sub-list for method output_type
	12, // [12:12] is the sub-list for method input_type
	12, // [12:12] is the sub-list for extension type_name
	12, // [12:12] is the sub-list for extension extendee
	0,  // [0:12] is the sub-list for field type_name
}

func init() { file_chat_v1_events_proto_init() }
func file_chat_v1_events_proto_init() {
	if File_chat_v1_events_proto != nil {
		return
	}
	file_chat_v1_chat_proto_init()
	if !protoimpl.UnsafeEnabled {
		file_chat_v1_events_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ServerChangeEvent); i {
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
		file_chat_v1_events_proto_msgTypes[1].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ServerDeleteEvent); i {
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
		file_chat_v1_events_proto_msgTypes[2].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*EmoteChangeEvent); i {
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
		file_chat_v1_events_proto_msgTypes[3].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*EmoteDeleteEvent); i {
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
		file_chat_v1_events_proto_msgTypes[4].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ModifierChangeEvent); i {
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
		file_chat_v1_events_proto_msgTypes[5].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*ModifierDeleteEvent); i {
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
		file_chat_v1_events_proto_msgTypes[6].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*TagChangeEvent); i {
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
		file_chat_v1_events_proto_msgTypes[7].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*TagDeleteEvent); i {
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
		file_chat_v1_events_proto_msgTypes[8].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*UIConfigChangeEvent); i {
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
		file_chat_v1_events_proto_msgTypes[9].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*SyncAssetsEvent); i {
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
		file_chat_v1_events_proto_msgTypes[10].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*WhisperThreadChangeEvent); i {
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
		file_chat_v1_events_proto_msgTypes[11].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*WhisperRecordChangeEvent); i {
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
		file_chat_v1_events_proto_msgTypes[12].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*WhisperRecordDeleteEvent); i {
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
			RawDescriptor: file_chat_v1_events_proto_rawDesc,
			NumEnums:      0,
			NumMessages:   13,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_chat_v1_events_proto_goTypes,
		DependencyIndexes: file_chat_v1_events_proto_depIdxs,
		MessageInfos:      file_chat_v1_events_proto_msgTypes,
	}.Build()
	File_chat_v1_events_proto = out.File
	file_chat_v1_events_proto_rawDesc = nil
	file_chat_v1_events_proto_goTypes = nil
	file_chat_v1_events_proto_depIdxs = nil
}
