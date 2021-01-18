package peer

import (
	"context"

	transfer "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
)

type transferService struct {
	Peer control.Peer
	App  control.AppControl
}

func (s *transferService) AnnounceSwarm(ctx context.Context, req *transfer.TransferPeerAnnounceSwarmRequest) (*transfer.TransferPeerAnnounceSwarmResponse, error) {
	port, ok := s.Peer.Transfer().AssignPort(ppspp.SwarmID(req.SwarmId), uint16(req.Port))
	if ok {
		return &transfer.TransferPeerAnnounceSwarmResponse{
			Body: &transfer.TransferPeerAnnounceSwarmResponse_Port{
				Port: uint32(port),
			},
		}, nil
	}
	return &transfer.TransferPeerAnnounceSwarmResponse{}, nil
}

func (s *transferService) CloseSwarm(ctx context.Context, req *transfer.TransferPeerCloseSwarmRequest) (*transfer.TransferPeerCloseSwarmResponse, error) {
	s.Peer.Transfer().CloseSwarm(ppspp.SwarmID(req.SwarmId))
	return &transfer.TransferPeerCloseSwarmResponse{}, nil
}
