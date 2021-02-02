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
