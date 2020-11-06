package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterBootstrapService ...
func RegisterBootstrapService(host ServiceRegistry, service BootstrapService) {
	host.RegisterMethod("Bootstrap/CreateClient", service.CreateClient)
	host.RegisterMethod("Bootstrap/UpdateClient", service.UpdateClient)
	host.RegisterMethod("Bootstrap/DeleteClient", service.DeleteClient)
	host.RegisterMethod("Bootstrap/GetClient", service.GetClient)
	host.RegisterMethod("Bootstrap/ListClients", service.ListClients)
	host.RegisterMethod("Bootstrap/ListPeers", service.ListPeers)
	host.RegisterMethod("Bootstrap/PublishNetworkToPeer", service.PublishNetworkToPeer)
}

// BootstrapService ...
type BootstrapService interface {
	CreateClient(
		ctx context.Context,
		req *pb.CreateBootstrapClientRequest,
	) (*pb.CreateBootstrapClientResponse, error)
	UpdateClient(
		ctx context.Context,
		req *pb.UpdateBootstrapClientRequest,
	) (*pb.UpdateBootstrapClientResponse, error)
	DeleteClient(
		ctx context.Context,
		req *pb.DeleteBootstrapClientRequest,
	) (*pb.DeleteBootstrapClientResponse, error)
	GetClient(
		ctx context.Context,
		req *pb.GetBootstrapClientRequest,
	) (*pb.GetBootstrapClientResponse, error)
	ListClients(
		ctx context.Context,
		req *pb.ListBootstrapClientsRequest,
	) (*pb.ListBootstrapClientsResponse, error)
	ListPeers(
		ctx context.Context,
		req *pb.ListBootstrapPeersRequest,
	) (*pb.ListBootstrapPeersResponse, error)
	PublishNetworkToPeer(
		ctx context.Context,
		req *pb.PublishNetworkToBootstrapPeerRequest,
	) (*pb.PublishNetworkToBootstrapPeerResponse, error)
}

// BootstrapClient ...
type BootstrapClient struct {
	client *rpc.Client
}

// NewBootstrapClient ...
func NewBootstrapClient(client *rpc.Client) *BootstrapClient {
	return &BootstrapClient{client}
}

// CreateClient ...
func (c *BootstrapClient) CreateClient(
	ctx context.Context,
	req *pb.CreateBootstrapClientRequest,
	res *pb.CreateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "Bootstrap/CreateClient", req, res)
}

// UpdateClient ...
func (c *BootstrapClient) UpdateClient(
	ctx context.Context,
	req *pb.UpdateBootstrapClientRequest,
	res *pb.UpdateBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "Bootstrap/UpdateClient", req, res)
}

// DeleteClient ...
func (c *BootstrapClient) DeleteClient(
	ctx context.Context,
	req *pb.DeleteBootstrapClientRequest,
	res *pb.DeleteBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "Bootstrap/DeleteClient", req, res)
}

// GetClient ...
func (c *BootstrapClient) GetClient(
	ctx context.Context,
	req *pb.GetBootstrapClientRequest,
	res *pb.GetBootstrapClientResponse,
) error {
	return c.client.CallUnary(ctx, "Bootstrap/GetClient", req, res)
}

// ListClients ...
func (c *BootstrapClient) ListClients(
	ctx context.Context,
	req *pb.ListBootstrapClientsRequest,
	res *pb.ListBootstrapClientsResponse,
) error {
	return c.client.CallUnary(ctx, "Bootstrap/ListClients", req, res)
}

// ListPeers ...
func (c *BootstrapClient) ListPeers(
	ctx context.Context,
	req *pb.ListBootstrapPeersRequest,
	res *pb.ListBootstrapPeersResponse,
) error {
	return c.client.CallUnary(ctx, "Bootstrap/ListPeers", req, res)
}

// PublishNetworkToPeer ...
func (c *BootstrapClient) PublishNetworkToPeer(
	ctx context.Context,
	req *pb.PublishNetworkToBootstrapPeerRequest,
	res *pb.PublishNetworkToBootstrapPeerResponse,
) error {
	return c.client.CallUnary(ctx, "Bootstrap/PublishNetworkToPeer", req, res)
}
