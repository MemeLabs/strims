package versionvector

import (
	"fmt"

	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

type VersionedMessage interface {
	proto.Message
	GetVersion() *daov1.VersionVector
}

func ProtoFieldDescriptor[T VersionedMessage]() (protoreflect.FieldDescriptor, error) {
	var m T
	d := m.ProtoReflect().Descriptor().Fields().ByTextName("version")
	if d == nil {
		return nil, fmt.Errorf("version field not found in type %T", m)
	}
	return d, nil
}
