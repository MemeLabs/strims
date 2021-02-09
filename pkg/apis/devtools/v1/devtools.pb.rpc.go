package devtools

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterDevToolsService ...
func RegisterDevToolsService(host rpc.ServiceRegistry, service DevToolsService) {
	host.RegisterMethod("strims.devtools.v1.DevTools.Test", service.Test)
}

// DevToolsService ...
type DevToolsService interface {
	Test(
		ctx context.Context,
		req *DevToolsTestRequest,
	) (*DevToolsTestResponse, error)
}

// DevToolsClient ...
type DevToolsClient struct {
	client rpc.Caller
}

// NewDevToolsClient ...
func NewDevToolsClient(client rpc.Caller) *DevToolsClient {
	return &DevToolsClient{client}
}

// Test ...
func (c *DevToolsClient) Test(
	ctx context.Context,
	req *DevToolsTestRequest,
	res *DevToolsTestResponse,
) error {
	return c.client.CallUnary(ctx, "strims.devtools.v1.DevTools.Test", req, res)
}
