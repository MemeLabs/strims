package api

import (
	"context"

	"github.com/golang/protobuf/proto"
)

// ServiceRegistry ...
type ServiceRegistry interface {
	RegisterService(name string, service interface{})
}

// UnaryCaller ...
type UnaryCaller interface {
	CallUnary(ctx context.Context, method string, req, res proto.Message) error
}

// StreamCaller ...
type StreamCaller interface {
	UnaryCaller
	CallStreaming(ctx context.Context, method string, req proto.Message, ch interface{}) error
}
