package peer

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type networkService struct {
	Peer *app.Peer
	App  *app.Control
}

func (s *networkService) Negotiate(ctx context.Context, req *pb.NetworkPeerNegotiateRequest) (*pb.NetworkPeerNegotiateResponse, error) {
	s.Peer.Network().SetPeerInit(req.KeyCount)
	return &pb.NetworkPeerNegotiateResponse{}, nil
}

func (s *networkService) Open(ctx context.Context, req *pb.NetworkPeerOpenRequest) (*pb.NetworkPeerOpenResponse, error) {
	s.Peer.Network().SetPeerBindings(req.Bindings)
	return &pb.NetworkPeerOpenResponse{}, nil
}

func (s *networkService) Close(ctx context.Context, req *pb.NetworkPeerCloseRequest) (*pb.NetworkPeerCloseResponse, error) {
	return nil, errors.New("not implemented")
}
