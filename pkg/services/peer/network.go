package peer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type networkService struct {
	Peer *app.Peer
	App  *app.Control
}

func (s *networkService) Negotiate(ctx context.Context, req *pb.NetworkPeerNegotiateRequest) (*pb.NetworkPeerNegotiateResponse, error) {
	s.Peer.Network().HandlePeerNegotiate(req.KeyCount)
	return &pb.NetworkPeerNegotiateResponse{}, nil
}

func (s *networkService) Open(ctx context.Context, req *pb.NetworkPeerOpenRequest) (*pb.NetworkPeerOpenResponse, error) {
	s.Peer.Network().HandlePeerOpen(req.Bindings)
	return &pb.NetworkPeerOpenResponse{}, nil
}

func (s *networkService) Close(ctx context.Context, req *pb.NetworkPeerCloseRequest) (*pb.NetworkPeerCloseResponse, error) {
	s.Peer.Network().HandlePeerClose(req.Key)
	return &pb.NetworkPeerCloseResponse{}, nil
}

func (s *networkService) UpdateCertificate(ctx context.Context, req *pb.NetworkPeerUpdateCertificateRequest) (*pb.NetworkPeerUpdateCertificateResponse, error) {
	if err := s.Peer.Network().HandlePeerUpdateCertificate(req.Certificate); err != nil {
		return nil, err
	}
	return &pb.NetworkPeerUpdateCertificateResponse{}, nil
}
