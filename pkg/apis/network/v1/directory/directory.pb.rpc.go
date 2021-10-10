package networkv1directory

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterDirectoryService ...
func RegisterDirectoryService(host rpc.ServiceRegistry, service DirectoryService) {
	host.RegisterMethod("strims.network.v1.directory.Directory.Publish", service.Publish)
	host.RegisterMethod("strims.network.v1.directory.Directory.Unpublish", service.Unpublish)
	host.RegisterMethod("strims.network.v1.directory.Directory.Join", service.Join)
	host.RegisterMethod("strims.network.v1.directory.Directory.Part", service.Part)
	host.RegisterMethod("strims.network.v1.directory.Directory.Ping", service.Ping)
}

// DirectoryService ...
type DirectoryService interface {
	Publish(
		ctx context.Context,
		req *PublishRequest,
	) (*PublishResponse, error)
	Unpublish(
		ctx context.Context,
		req *UnpublishRequest,
	) (*UnpublishResponse, error)
	Join(
		ctx context.Context,
		req *JoinRequest,
	) (*JoinResponse, error)
	Part(
		ctx context.Context,
		req *PartRequest,
	) (*PartResponse, error)
	Ping(
		ctx context.Context,
		req *PingRequest,
	) (*PingResponse, error)
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
	req *PublishRequest,
	res *PublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Publish", req, res)
}

// Unpublish ...
func (c *DirectoryClient) Unpublish(
	ctx context.Context,
	req *UnpublishRequest,
	res *UnpublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Unpublish", req, res)
}

// Join ...
func (c *DirectoryClient) Join(
	ctx context.Context,
	req *JoinRequest,
	res *JoinResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Join", req, res)
}

// Part ...
func (c *DirectoryClient) Part(
	ctx context.Context,
	req *PartRequest,
	res *PartResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Part", req, res)
}

// Ping ...
func (c *DirectoryClient) Ping(
	ctx context.Context,
	req *PingRequest,
	res *PingResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Ping", req, res)
}

// RegisterDirectoryFrontendService ...
func RegisterDirectoryFrontendService(host rpc.ServiceRegistry, service DirectoryFrontendService) {
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Open", service.Open)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Test", service.Test)
}

// DirectoryFrontendService ...
type DirectoryFrontendService interface {
	Open(
		ctx context.Context,
		req *FrontendOpenRequest,
	) (<-chan *FrontendOpenResponse, error)
	Test(
		ctx context.Context,
		req *FrontendTestRequest,
	) (*FrontendTestResponse, error)
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
	req *FrontendOpenRequest,
	res chan *FrontendOpenResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.directory.DirectoryFrontend.Open", req, res)
}

// Test ...
func (c *DirectoryFrontendClient) Test(
	ctx context.Context,
	req *FrontendTestRequest,
	res *FrontendTestResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.Test", req, res)
}

// RegisterDirectorySnippetService ...
func RegisterDirectorySnippetService(host rpc.ServiceRegistry, service DirectorySnippetService) {
	host.RegisterMethod("strims.network.v1.directory.DirectorySnippet.Subscribe", service.Subscribe)
}

// DirectorySnippetService ...
type DirectorySnippetService interface {
	Subscribe(
		ctx context.Context,
		req *SnippetSubscribeRequest,
	) (<-chan *SnippetSubscribeResponse, error)
}

// DirectorySnippetClient ...
type DirectorySnippetClient struct {
	client rpc.Caller
}

// NewDirectorySnippetClient ...
func NewDirectorySnippetClient(client rpc.Caller) *DirectorySnippetClient {
	return &DirectorySnippetClient{client}
}

// Subscribe ...
func (c *DirectorySnippetClient) Subscribe(
	ctx context.Context,
	req *SnippetSubscribeRequest,
	res chan *SnippetSubscribeResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.directory.DirectorySnippet.Subscribe", req, res)
}
