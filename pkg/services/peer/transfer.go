package peer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type transferService struct {
	Peer control.Peer
	App  control.AppControl
}

func (s *transferService) AnnounceSwarm(ctx context.Context, req *pb.TransferPeerAnnounceSwarmRequest) (*pb.TransferPeerAnnounceSwarmResponse, error) {
	port, ok := s.Peer.Transfer().AssignPort(req.SwarmId, uint16(req.Port))
	if ok {
		return &pb.TransferPeerAnnounceSwarmResponse{
			Body: &pb.TransferPeerAnnounceSwarmResponse_Port{
				Port: uint32(port),
			},
		}, nil
	}
	return &pb.TransferPeerAnnounceSwarmResponse{}, nil
}
