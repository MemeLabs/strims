package peer

import (
	"context"

	control "github.com/MemeLabs/go-ppspp/internal"
	transfer "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
)

type transferService struct {
	Peer control.Peer
	App  control.AppControl
}

func (s *transferService) Announce(ctx context.Context, req *transfer.TransferPeerAnnounceRequest) (*transfer.TransferPeerAnnounceResponse, error) {
	channel, ok := s.Peer.Transfer().AssignPort(req.Id, req.Channel)
	if ok {
		return &transfer.TransferPeerAnnounceResponse{
			Body: &transfer.TransferPeerAnnounceResponse_Channel{
				Channel: channel,
			},
		}, nil
	}
	return &transfer.TransferPeerAnnounceResponse{}, nil
}

func (s *transferService) Close(ctx context.Context, req *transfer.TransferPeerCloseRequest) (*transfer.TransferPeerCloseResponse, error) {
	s.Peer.Transfer().Close(req.Id)
	return &transfer.TransferPeerCloseResponse{}, nil
}
