package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterNetworkService ...
func RegisterNetworkService(host ServiceRegistry, service NetworkService) {
	host.RegisterService("Network", service)
}

// NetworkService ...
type NetworkService interface {
	Create(
		ctx context.Context,
		req *pb.CreateNetworkRequest,
	) (*pb.CreateNetworkResponse, error)
	Update(
		ctx context.Context,
		req *pb.UpdateNetworkRequest,
	) (*pb.UpdateNetworkResponse, error)
	Delete(
		ctx context.Context,
		req *pb.DeleteNetworkRequest,
	) (*pb.DeleteNetworkResponse, error)
	Get(
		ctx context.Context,
		req *pb.GetNetworkRequest,
	) (*pb.GetNetworkResponse, error)
	List(
		ctx context.Context,
		req *pb.ListNetworksRequest,
	) (*pb.ListNetworksResponse, error)
	CreateInvitation(
		ctx context.Context,
		req *pb.CreateNetworkInvitationRequest,
	) (*pb.CreateNetworkInvitationResponse, error)
	CreateFromInvitation(
		ctx context.Context,
		req *pb.CreateNetworkFromInvitationRequest,
	) (*pb.CreateNetworkFromInvitationResponse, error)
	StartVPN(
		ctx context.Context,
		req *pb.StartVPNRequest,
	) (<-chan *pb.NetworkEvent, error)
	StopVPN(
		ctx context.Context,
		req *pb.StopVPNRequest,
	) (*pb.StopVPNResponse, error)
	GetDirectoryEvents(
		ctx context.Context,
		req *pb.GetDirectoryEventsRequest,
	) (<-chan *pb.DirectoryServerEvent, error)
	TestDirectoryPublish(
		ctx context.Context,
		req *pb.TestDirectoryPublishRequest,
	) (*pb.TestDirectoryPublishResponse, error)
}

// NetworkClient ...
type NetworkClient struct {
	client StreamCaller
}

// NewNetworkClient ...
func NewNetworkClient(client StreamCaller) *NetworkClient {
	return &NetworkClient{client}
}

// Create ...
func (c *NetworkClient) Create(
	ctx context.Context,
	req *pb.CreateNetworkRequest,
	res *pb.CreateNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "Network/Create", req, res)
}

// Update ...
func (c *NetworkClient) Update(
	ctx context.Context,
	req *pb.UpdateNetworkRequest,
	res *pb.UpdateNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "Network/Update", req, res)
}

// Delete ...
func (c *NetworkClient) Delete(
	ctx context.Context,
	req *pb.DeleteNetworkRequest,
	res *pb.DeleteNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "Network/Delete", req, res)
}

// Get ...
func (c *NetworkClient) Get(
	ctx context.Context,
	req *pb.GetNetworkRequest,
	res *pb.GetNetworkResponse,
) error {
	return c.client.CallUnary(ctx, "Network/Get", req, res)
}

// List ...
func (c *NetworkClient) List(
	ctx context.Context,
	req *pb.ListNetworksRequest,
	res *pb.ListNetworksResponse,
) error {
	return c.client.CallUnary(ctx, "Network/List", req, res)
}

// CreateInvitation ...
func (c *NetworkClient) CreateInvitation(
	ctx context.Context,
	req *pb.CreateNetworkInvitationRequest,
	res *pb.CreateNetworkInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "Network/CreateInvitation", req, res)
}

// CreateFromInvitation ...
func (c *NetworkClient) CreateFromInvitation(
	ctx context.Context,
	req *pb.CreateNetworkFromInvitationRequest,
	res *pb.CreateNetworkFromInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "Network/CreateFromInvitation", req, res)
}

// StartVPN ...
func (c *NetworkClient) StartVPN(
	ctx context.Context,
	req *pb.StartVPNRequest,
	res chan *pb.NetworkEvent,
) error {
	return c.client.CallStreaming(ctx, "Network/StartVPN", req, res)
}

// StopVPN ...
func (c *NetworkClient) StopVPN(
	ctx context.Context,
	req *pb.StopVPNRequest,
	res *pb.StopVPNResponse,
) error {
	return c.client.CallUnary(ctx, "Network/StopVPN", req, res)
}

// GetDirectoryEvents ...
func (c *NetworkClient) GetDirectoryEvents(
	ctx context.Context,
	req *pb.GetDirectoryEventsRequest,
	res chan *pb.DirectoryServerEvent,
) error {
	return c.client.CallStreaming(ctx, "Network/GetDirectoryEvents", req, res)
}

// TestDirectoryPublish ...
func (c *NetworkClient) TestDirectoryPublish(
	ctx context.Context,
	req *pb.TestDirectoryPublishRequest,
	res *pb.TestDirectoryPublishResponse,
) error {
	return c.client.CallUnary(ctx, "Network/TestDirectoryPublish", req, res)
}
