package api

import (
	"context"

	"github.com/golang/protobuf/proto"
)

// ServiceRegistry ...
type ServiceRegistry interface {
	RegisterMethod(name string, method interface{})
}

// Caller ...
type Caller interface {
	CallUnary(ctx context.Context, method string, req proto.Message, res proto.Message) error
	CallStreaming(ctx context.Context, method string, req proto.Message, res interface{}) error
}
