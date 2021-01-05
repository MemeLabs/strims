package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterDirectoryFrontendService ...
func RegisterDirectoryFrontendService(host ServiceRegistry, service DirectoryFrontendService) {
	host.RegisterMethod("DirectoryFrontend/Open", service.Open)
	host.RegisterMethod("DirectoryFrontend/Test", service.Test)
}

// DirectoryFrontendService ...
type DirectoryFrontendService interface {
	Open(
		ctx context.Context,
		req *pb.DirectoryFrontendOpenRequest,
	) (<-chan *pb.DirectoryFrontendOpenResponse, error)
	Test(
		ctx context.Context,
		req *pb.DirectoryFrontendTestRequest,
	) (*pb.DirectoryFrontendTestResponse, error)
}

// DirectoryFrontendClient ...
type DirectoryFrontendClient struct {
	client Caller
}

// NewDirectoryFrontendClient ...
func NewDirectoryFrontendClient(client Caller) *DirectoryFrontendClient {
	return &DirectoryFrontendClient{client}
}

// Open ...
func (c *DirectoryFrontendClient) Open(
	ctx context.Context,
	req *pb.DirectoryFrontendOpenRequest,
	res chan *pb.DirectoryFrontendOpenResponse,
) error {
	return c.client.CallStreaming(ctx, "DirectoryFrontend/Open", req, res)
}

// Test ...
func (c *DirectoryFrontendClient) Test(
	ctx context.Context,
	req *pb.DirectoryFrontendTestRequest,
	res *pb.DirectoryFrontendTestResponse,
) error {
	return c.client.CallUnary(ctx, "DirectoryFrontend/Test", req, res)
}
