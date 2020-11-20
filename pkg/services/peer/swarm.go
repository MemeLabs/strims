package peer

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type swarmService struct {
	Peer *app.Peer
	App  *app.Control
}

func (s *swarmService) AnnounceSwarm(ctx context.Context, req *pb.SwarmPeerAnnounceSwarmRequest) (*pb.SwarmPeerAnnounceSwarmResponse, error) {
	return nil, errors.New("not implemented")
}
