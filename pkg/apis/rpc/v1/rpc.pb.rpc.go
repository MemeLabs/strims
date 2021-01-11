package rpc

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

// RegisterRPCTestService ...
func RegisterRPCTestService(host api.ServiceRegistry, service RPCTestService) {
	host.RegisterMethod(".strims.rpc.v1.RPCTest.CallUnary", service.CallUnary)
	host.RegisterMethod(".strims.rpc.v1.RPCTest.CallStream", service.CallStream)
}

// RPCTestService ...
type RPCTestService interface {
	CallUnary(
		ctx context.Context,
		req *RPCCallUnaryRequest,
	) (*RPCCallUnaryResponse, error)
	CallStream(
		ctx context.Context,
		req *RPCCallStreamRequest,
	) (<-chan *RPCCallStreamResponse, error)
}

// RPCTestClient ...
type RPCTestClient struct {
	client api.Caller
}

// NewRPCTestClient ...
func NewRPCTestClient(client api.Caller) *RPCTestClient {
	return &RPCTestClient{client}
}

// CallUnary ...
func (c *RPCTestClient) CallUnary(
	ctx context.Context,
	req *RPCCallUnaryRequest,
	res *RPCCallUnaryResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.rpc.v1.RPCTest.CallUnary", req, res)
}

// CallStream ...
func (c *RPCTestClient) CallStream(
	ctx context.Context,
	req *RPCCallStreamRequest,
	res chan *RPCCallStreamResponse,
) error {
	return c.client.CallStreaming(ctx, ".strims.rpc.v1.RPCTest.CallStream", req, res)
}
