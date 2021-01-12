package network

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterDirectoryService ...
func RegisterDirectoryService(host rpc.ServiceRegistry, service DirectoryService) {
	host.RegisterMethod("strims.network.v1.Directory.Publish", service.Publish)
	host.RegisterMethod("strims.network.v1.Directory.Unpublish", service.Unpublish)
	host.RegisterMethod("strims.network.v1.Directory.Join", service.Join)
	host.RegisterMethod("strims.network.v1.Directory.Part", service.Part)
	host.RegisterMethod("strims.network.v1.Directory.Ping", service.Ping)
}

// DirectoryService ...
type DirectoryService interface {
	Publish(
		ctx context.Context,
		req *DirectoryPublishRequest,
	) (*DirectoryPublishResponse, error)
	Unpublish(
		ctx context.Context,
		req *DirectoryUnpublishRequest,
	) (*DirectoryUnpublishResponse, error)
	Join(
		ctx context.Context,
		req *DirectoryJoinRequest,
	) (*DirectoryJoinResponse, error)
	Part(
		ctx context.Context,
		req *DirectoryPartRequest,
	) (*DirectoryPartResponse, error)
	Ping(
		ctx context.Context,
		req *DirectoryPingRequest,
	) (*DirectoryPingResponse, error)
}

// DirectoryClient ...
type DirectoryClient struct {
	client rpc.Caller
}

// NewDirectoryClient ...
func NewDirectoryClient(client rpc.Caller) *DirectoryClient {
	return &DirectoryClient{client}
}

// Publish ...
func (c *DirectoryClient) Publish(
	ctx context.Context,
	req *DirectoryPublishRequest,
	res *DirectoryPublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.Directory.Publish", req, res)
}

// Unpublish ...
func (c *DirectoryClient) Unpublish(
	ctx context.Context,
	req *DirectoryUnpublishRequest,
	res *DirectoryUnpublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.Directory.Unpublish", req, res)
}

// Join ...
func (c *DirectoryClient) Join(
	ctx context.Context,
	req *DirectoryJoinRequest,
	res *DirectoryJoinResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.Directory.Join", req, res)
}

// Part ...
func (c *DirectoryClient) Part(
	ctx context.Context,
	req *DirectoryPartRequest,
	res *DirectoryPartResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.Directory.Part", req, res)
}

// Ping ...
func (c *DirectoryClient) Ping(
	ctx context.Context,
	req *DirectoryPingRequest,
	res *DirectoryPingResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.Directory.Ping", req, res)
}

// RegisterDirectoryFrontendService ...
func RegisterDirectoryFrontendService(host rpc.ServiceRegistry, service DirectoryFrontendService) {
	host.RegisterMethod("strims.network.v1.DirectoryFrontend.Open", service.Open)
	host.RegisterMethod("strims.network.v1.DirectoryFrontend.Test", service.Test)
}

// DirectoryFrontendService ...
type DirectoryFrontendService interface {
	Open(
		ctx context.Context,
		req *DirectoryFrontendOpenRequest,
	) (<-chan *DirectoryFrontendOpenResponse, error)
	Test(
		ctx context.Context,
		req *DirectoryFrontendTestRequest,
	) (*DirectoryFrontendTestResponse, error)
}

// DirectoryFrontendClient ...
type DirectoryFrontendClient struct {
	client rpc.Caller
}

// NewDirectoryFrontendClient ...
func NewDirectoryFrontendClient(client rpc.Caller) *DirectoryFrontendClient {
	return &DirectoryFrontendClient{client}
}

// Open ...
func (c *DirectoryFrontendClient) Open(
	ctx context.Context,
	req *DirectoryFrontendOpenRequest,
	res chan *DirectoryFrontendOpenResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.DirectoryFrontend.Open", req, res)
}

// Test ...
func (c *DirectoryFrontendClient) Test(
	ctx context.Context,
	req *DirectoryFrontendTestRequest,
	res *DirectoryFrontendTestResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.DirectoryFrontend.Test", req, res)
}
