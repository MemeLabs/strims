package peer

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/app"
	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
)

type networkService struct {
	Peer app.Peer
	App  app.Control
}

func (s *networkService) Negotiate(ctx context.Context, req *network.NetworkPeerNegotiateRequest) (*network.NetworkPeerNegotiateResponse, error) {
	s.Peer.Network().HandlePeerNegotiate(req.KeyCount)
	return &network.NetworkPeerNegotiateResponse{}, nil
}

func (s *networkService) Open(ctx context.Context, req *network.NetworkPeerOpenRequest) (*network.NetworkPeerOpenResponse, error) {
	s.Peer.Network().HandlePeerOpen(req.Bindings)
	return &network.NetworkPeerOpenResponse{}, nil
}

func (s *networkService) Close(ctx context.Context, req *network.NetworkPeerCloseRequest) (*network.NetworkPeerCloseResponse, error) {
	s.Peer.Network().HandlePeerClose(req.Key)
	return &network.NetworkPeerCloseResponse{}, nil
}

func (s *networkService) UpdateCertificate(ctx context.Context, req *network.NetworkPeerUpdateCertificateRequest) (*network.NetworkPeerUpdateCertificateResponse, error) {
	if err := s.Peer.Network().HandlePeerUpdateCertificate(req.Certificate); err != nil {
		return nil, err
	}
	return &network.NetworkPeerUpdateCertificateResponse{}, nil
}
