package api

import (
	"context"
	"errors"

	"github.com/golang/protobuf/proto"
)

// ErrNotImplemented ...
var ErrNotImplemented = errors.New("not implemented")

// ServiceRegistry ...
type ServiceRegistry interface {
	RegisterMethod(name string, method interface{})
}

// Caller ...
type Caller interface {
	CallUnary(ctx context.Context, method string, req proto.Message, res proto.Message) error
	CallStreaming(ctx context.Context, method string, req proto.Message, res interface{}) error
}

// PeerClient ...
type PeerClient interface {
	Bootstrap() *BootstrapPeerClient
	CA() *CAPeerClient
	Transfer() *TransferPeerClient
	Network() *NetworkPeerClient
}
