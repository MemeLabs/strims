package peer

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

type bootstrapService struct {
	Peer  *app.Peer
	App   *app.Control
	Store *dao.ProfileStore
}

func (s *bootstrapService) GetPublishEnabled(ctx context.Context, req *pb.BootstrapPeerGetPublishEnabledRequest) (*pb.BootstrapPeerGetPublishEnabledResponse, error) {
	return &pb.BootstrapPeerGetPublishEnabledResponse{Enabled: s.App.Bootstrap().PublishingEnabled()}, nil
}

func (s *bootstrapService) ListNetworks(ctx context.Context, req *pb.BootstrapPeerListNetworksRequest) (*pb.BootstrapPeerListNetworksResponse, error) {
	return nil, api.ErrNotImplemented
}

func (s *bootstrapService) Publish(ctx context.Context, req *pb.BootstrapPeerPublishRequest) (*pb.BootstrapPeerPublishResponse, error) {
	if !s.App.Bootstrap().PublishingEnabled() {
		return nil, errors.New("not supported")
	}

	network, err := dao.NewNetworkFromCertificate(s.Store, req.Certificate)
	if err != nil {
		return nil, err
	}

	return nil, s.App.Network().Add(network)
}
