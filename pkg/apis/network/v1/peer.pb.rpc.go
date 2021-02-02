package network

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterNetworkPeerService ...
func RegisterNetworkPeerService(host rpc.ServiceRegistry, service NetworkPeerService) {
	host.RegisterMethod("strims.network.v1.NetworkPeer.Negotiate", service.Negotiate)
	host.RegisterMethod("strims.network.v1.NetworkPeer.Open", service.Open)
	host.RegisterMethod("strims.network.v1.NetworkPeer.Close", service.Close)
	host.RegisterMethod("strims.network.v1.NetworkPeer.UpdateCertificate", service.UpdateCertificate)
}

// NetworkPeerService ...
type NetworkPeerService interface {
	Negotiate(
		ctx context.Context,
		req *NetworkPeerNegotiateRequest,
	) (*NetworkPeerNegotiateResponse, error)
	Open(
		ctx context.Context,
		req *NetworkPeerOpenRequest,
	) (*NetworkPeerOpenResponse, error)
	Close(
		ctx context.Context,
		req *NetworkPeerCloseRequest,
	) (*NetworkPeerCloseResponse, error)
	UpdateCertificate(
		ctx context.Context,
		req *NetworkPeerUpdateCertificateRequest,
	) (*NetworkPeerUpdateCertificateResponse, error)
}

// NetworkPeerClient ...
type NetworkPeerClient struct {
	client rpc.Caller
}

// NewNetworkPeerClient ...
func NewNetworkPeerClient(client rpc.Caller) *NetworkPeerClient {
	return &NetworkPeerClient{client}
}

// Negotiate ...
func (c *NetworkPeerClient) Negotiate(
	ctx context.Context,
	req *NetworkPeerNegotiateRequest,
	res *NetworkPeerNegotiateResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkPeer.Negotiate", req, res)
}

// Open ...
func (c *NetworkPeerClient) Open(
	ctx context.Context,
	req *NetworkPeerOpenRequest,
	res *NetworkPeerOpenResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkPeer.Open", req, res)
}

// Close ...
func (c *NetworkPeerClient) Close(
	ctx context.Context,
	req *NetworkPeerCloseRequest,
	res *NetworkPeerCloseResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkPeer.Close", req, res)
}

// UpdateCertificate ...
func (c *NetworkPeerClient) UpdateCertificate(
	ctx context.Context,
	req *NetworkPeerUpdateCertificateRequest,
	res *NetworkPeerUpdateCertificateResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.NetworkPeer.UpdateCertificate", req, res)
}
