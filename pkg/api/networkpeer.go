package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterNetworkPeerService ...
func RegisterNetworkPeerService(host ServiceRegistry, service NetworkPeerService) {
	host.RegisterMethod("NetworkPeer/Negotiate", service.Negotiate)
	host.RegisterMethod("NetworkPeer/Open", service.Open)
	host.RegisterMethod("NetworkPeer/Close", service.Close)
	host.RegisterMethod("NetworkPeer/UpdateCertificate", service.UpdateCertificate)
}

// NetworkPeerService ...
type NetworkPeerService interface {
	Negotiate(
		ctx context.Context,
		req *pb.NetworkPeerNegotiateRequest,
	) (*pb.NetworkPeerNegotiateResponse, error)
	Open(
		ctx context.Context,
		req *pb.NetworkPeerOpenRequest,
	) (*pb.NetworkPeerOpenResponse, error)
	Close(
		ctx context.Context,
		req *pb.NetworkPeerCloseRequest,
	) (*pb.NetworkPeerCloseResponse, error)
	UpdateCertificate(
		ctx context.Context,
		req *pb.NetworkPeerUpdateCertificateRequest,
	) (*pb.NetworkPeerUpdateCertificateResponse, error)
}

// NetworkPeerClient ...
type NetworkPeerClient struct {
	client Caller
}

// NewNetworkPeerClient ...
func NewNetworkPeerClient(client Caller) *NetworkPeerClient {
	return &NetworkPeerClient{client}
}

// Negotiate ...
func (c *NetworkPeerClient) Negotiate(
	ctx context.Context,
	req *pb.NetworkPeerNegotiateRequest,
	res *pb.NetworkPeerNegotiateResponse,
) error {
	return c.client.CallUnary(ctx, "NetworkPeer/Negotiate", req, res)
}

// Open ...
func (c *NetworkPeerClient) Open(
	ctx context.Context,
	req *pb.NetworkPeerOpenRequest,
	res *pb.NetworkPeerOpenResponse,
) error {
	return c.client.CallUnary(ctx, "NetworkPeer/Open", req, res)
}

// Close ...
func (c *NetworkPeerClient) Close(
	ctx context.Context,
	req *pb.NetworkPeerCloseRequest,
	res *pb.NetworkPeerCloseResponse,
) error {
	return c.client.CallUnary(ctx, "NetworkPeer/Close", req, res)
}

// UpdateCertificate ...
func (c *NetworkPeerClient) UpdateCertificate(
	ctx context.Context,
	req *pb.NetworkPeerUpdateCertificateRequest,
	res *pb.NetworkPeerUpdateCertificateResponse,
) error {
	return c.client.CallUnary(ctx, "NetworkPeer/UpdateCertificate", req, res)
}
