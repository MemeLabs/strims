package transfer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

// RegisterTransferPeerService ...
func RegisterTransferPeerService(host rpc.ServiceRegistry, service TransferPeerService) {
	host.RegisterMethod("strims.transfer.v1.TransferPeer.AnnounceSwarm", service.AnnounceSwarm)
	host.RegisterMethod("strims.transfer.v1.TransferPeer.CloseSwarm", service.CloseSwarm)
}

// TransferPeerService ...
type TransferPeerService interface {
	AnnounceSwarm(
		ctx context.Context,
		req *TransferPeerAnnounceSwarmRequest,
	) (*TransferPeerAnnounceSwarmResponse, error)
	CloseSwarm(
		ctx context.Context,
		req *TransferPeerCloseSwarmRequest,
	) (*TransferPeerCloseSwarmResponse, error)
}

// TransferPeerClient ...
type TransferPeerClient struct {
	client rpc.Caller
}

// NewTransferPeerClient ...
func NewTransferPeerClient(client rpc.Caller) *TransferPeerClient {
	return &TransferPeerClient{client}
}

// AnnounceSwarm ...
func (c *TransferPeerClient) AnnounceSwarm(
	ctx context.Context,
	req *TransferPeerAnnounceSwarmRequest,
	res *TransferPeerAnnounceSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "strims.transfer.v1.TransferPeer.AnnounceSwarm", req, res)
}

// CloseSwarm ...
func (c *TransferPeerClient) CloseSwarm(
	ctx context.Context,
	req *TransferPeerCloseSwarmRequest,
	res *TransferPeerCloseSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "strims.transfer.v1.TransferPeer.CloseSwarm", req, res)
}
