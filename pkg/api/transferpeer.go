package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterTransferPeerService ...
func RegisterTransferPeerService(host ServiceRegistry, service TransferPeerService) {
	host.RegisterMethod("TransferPeer/AnnounceSwarm", service.AnnounceSwarm)
}

// TransferPeerService ...
type TransferPeerService interface {
	AnnounceSwarm(
		ctx context.Context,
		req *pb.TransferPeerAnnounceSwarmRequest,
	) (*pb.TransferPeerAnnounceSwarmResponse, error)
}

// TransferPeerClient ...
type TransferPeerClient struct {
	client Caller
}

// NewTransferPeerClient ...
func NewTransferPeerClient(client Caller) *TransferPeerClient {
	return &TransferPeerClient{client}
}

// AnnounceSwarm ...
func (c *TransferPeerClient) AnnounceSwarm(
	ctx context.Context,
	req *pb.TransferPeerAnnounceSwarmRequest,
	res *pb.TransferPeerAnnounceSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "TransferPeer/AnnounceSwarm", req, res)
}
