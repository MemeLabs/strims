package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterDirectoryService ...
func RegisterDirectoryService(host ServiceRegistry, service DirectoryService) {
	host.RegisterMethod("Directory/Publish", service.Publish)
	host.RegisterMethod("Directory/Unpublish", service.Unpublish)
	host.RegisterMethod("Directory/Join", service.Join)
	host.RegisterMethod("Directory/Part", service.Part)
	host.RegisterMethod("Directory/Ping", service.Ping)
}

// DirectoryService ...
type DirectoryService interface {
	Publish(
		ctx context.Context,
		req *pb.DirectoryPublishRequest,
	) (*pb.DirectoryPublishResponse, error)
	Unpublish(
		ctx context.Context,
		req *pb.DirectoryUnpublishRequest,
	) (*pb.DirectoryUnpublishResponse, error)
	Join(
		ctx context.Context,
		req *pb.DirectoryJoinRequest,
	) (*pb.DirectoryJoinResponse, error)
	Part(
		ctx context.Context,
		req *pb.DirectoryPartRequest,
	) (*pb.DirectoryPartResponse, error)
	Ping(
		ctx context.Context,
		req *pb.DirectoryPingRequest,
	) (*pb.DirectoryPingResponse, error)
}

// DirectoryClient ...
type DirectoryClient struct {
	client Caller
}

// NewDirectoryClient ...
func NewDirectoryClient(client Caller) *DirectoryClient {
	return &DirectoryClient{client}
}

// Publish ...
func (c *DirectoryClient) Publish(
	ctx context.Context,
	req *pb.DirectoryPublishRequest,
	res *pb.DirectoryPublishResponse,
) error {
	return c.client.CallUnary(ctx, "Directory/Publish", req, res)
}

// Unpublish ...
func (c *DirectoryClient) Unpublish(
	ctx context.Context,
	req *pb.DirectoryUnpublishRequest,
	res *pb.DirectoryUnpublishResponse,
) error {
	return c.client.CallUnary(ctx, "Directory/Unpublish", req, res)
}

// Join ...
func (c *DirectoryClient) Join(
	ctx context.Context,
	req *pb.DirectoryJoinRequest,
	res *pb.DirectoryJoinResponse,
) error {
	return c.client.CallUnary(ctx, "Directory/Join", req, res)
}

// Part ...
func (c *DirectoryClient) Part(
	ctx context.Context,
	req *pb.DirectoryPartRequest,
	res *pb.DirectoryPartResponse,
) error {
	return c.client.CallUnary(ctx, "Directory/Part", req, res)
}

// Ping ...
func (c *DirectoryClient) Ping(
	ctx context.Context,
	req *pb.DirectoryPingRequest,
	res *pb.DirectoryPingResponse,
) error {
	return c.client.CallUnary(ctx, "Directory/Ping", req, res)
}
