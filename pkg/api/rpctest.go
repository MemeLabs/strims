package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterRPCTestService ...
func RegisterRPCTestService(host ServiceRegistry, service RPCTestService) {
	host.RegisterMethod("RPCTest/CallUnary", service.CallUnary)
	host.RegisterMethod("RPCTest/CallStream", service.CallStream)
}

// RPCTestService ...
type RPCTestService interface {
	CallUnary(
		ctx context.Context,
		req *pb.RPCCallUnaryRequest,
	) (*pb.RPCCallUnaryResponse, error)
	CallStream(
		ctx context.Context,
		req *pb.RPCCallStreamRequest,
	) (<-chan *pb.RPCCallStreamResponse, error)
}

// RPCTestClient ...
type RPCTestClient struct {
	client Caller
}

// NewRPCTestClient ...
func NewRPCTestClient(client Caller) *RPCTestClient {
	return &RPCTestClient{client}
}

// CallUnary ...
func (c *RPCTestClient) CallUnary(
	ctx context.Context,
	req *pb.RPCCallUnaryRequest,
	res *pb.RPCCallUnaryResponse,
) error {
	return c.client.CallUnary(ctx, "RPCTest/CallUnary", req, res)
}

// CallStream ...
func (c *RPCTestClient) CallStream(
	ctx context.Context,
	req *pb.RPCCallStreamRequest,
	res chan *pb.RPCCallStreamResponse,
) error {
	return c.client.CallStreaming(ctx, "RPCTest/CallStream", req, res)
}
