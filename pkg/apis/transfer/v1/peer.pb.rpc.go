package transfer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

// RegisterTransferPeerService ...
func RegisterTransferPeerService(host api.ServiceRegistry, service TransferPeerService) {
	host.RegisterMethod(".strims.transfer.v1.TransferPeer.AnnounceSwarm", service.AnnounceSwarm)
}

// TransferPeerService ...
type TransferPeerService interface {
	AnnounceSwarm(
		ctx context.Context,
		req *TransferPeerAnnounceSwarmRequest,
	) (*TransferPeerAnnounceSwarmResponse, error)
}

// TransferPeerClient ...
type TransferPeerClient struct {
	client api.Caller
}

// NewTransferPeerClient ...
func NewTransferPeerClient(client api.Caller) *TransferPeerClient {
	return &TransferPeerClient{client}
}

// AnnounceSwarm ...
func (c *TransferPeerClient) AnnounceSwarm(
	ctx context.Context,
	req *TransferPeerAnnounceSwarmRequest,
	res *TransferPeerAnnounceSwarmResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.transfer.v1.TransferPeer.AnnounceSwarm", req, res)
}
