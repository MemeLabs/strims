package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterSwarmPeerService ...
func RegisterSwarmPeerService(host ServiceRegistry, service SwarmPeerService) {
	host.RegisterMethod("SwarmPeer/AnnounceSwarm", service.AnnounceSwarm)
}

// SwarmPeerService ...
type SwarmPeerService interface {
	AnnounceSwarm(
		ctx context.Context,
		req *pb.SwarmPeerAnnounceSwarmRequest,
	) (*pb.SwarmPeerAnnounceSwarmResponse, error)
}

// SwarmPeerClient ...
type SwarmPeerClient struct {
	client Caller
}

// NewSwarmPeerClient ...
func NewSwarmPeerClient(client Caller) *SwarmPeerClient {
	return &SwarmPeerClient{client}
}

// AnnounceSwarm ...
func (c *SwarmPeerClient) AnnounceSwarm(
	ctx context.Context,
	req *pb.SwarmPeerAnnounceSwarmRequest,
	res *pb.SwarmPeerAnnounceSwarmResponse,
) error {
	return c.client.CallUnary(ctx, "SwarmPeer/AnnounceSwarm", req, res)
}
