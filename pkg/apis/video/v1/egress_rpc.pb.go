// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package video

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterEgressService ...
func RegisterEgressService(host rpc.ServiceRegistry, service EgressService) {
	host.RegisterMethod("strims.video.v1.Egress.OpenStream", service.OpenStream)
}

// EgressService ...
type EgressService interface {
	OpenStream(
		ctx context.Context,
		req *EgressOpenStreamRequest,
	) (<-chan *EgressOpenStreamResponse, error)
}

// UnimplementedEgressService ...
type UnimplementedEgressService struct{}

func (s *UnimplementedEgressService) OpenStream(
	ctx context.Context,
	req *EgressOpenStreamRequest,
) (<-chan *EgressOpenStreamResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ EgressService = (*UnimplementedEgressService)(nil)

// EgressClient ...
type EgressClient struct {
	client rpc.Caller
}

// NewEgressClient ...
func NewEgressClient(client rpc.Caller) *EgressClient {
	return &EgressClient{client}
}

// OpenStream ...
func (c *EgressClient) OpenStream(
	ctx context.Context,
	req *EgressOpenStreamRequest,
	res chan *EgressOpenStreamResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.video.v1.Egress.OpenStream", req, res)
}
