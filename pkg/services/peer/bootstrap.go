package peer

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
)

type bootstrapService struct {
	Peer  control.Peer
	App   control.AppControl
	Store *dao.ProfileStore
}

func (s *bootstrapService) GetPublishEnabled(ctx context.Context, req *bootstrap.BootstrapPeerGetPublishEnabledRequest) (*bootstrap.BootstrapPeerGetPublishEnabledResponse, error) {
	return &bootstrap.BootstrapPeerGetPublishEnabledResponse{Enabled: s.App.Bootstrap().PublishingEnabled()}, nil
}

func (s *bootstrapService) ListNetworks(ctx context.Context, req *bootstrap.BootstrapPeerListNetworksRequest) (*bootstrap.BootstrapPeerListNetworksResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *bootstrapService) Publish(ctx context.Context, req *bootstrap.BootstrapPeerPublishRequest) (*bootstrap.BootstrapPeerPublishResponse, error) {
	if !s.App.Bootstrap().PublishingEnabled() {
		return nil, errors.New("not supported")
	}

	network, err := dao.NewNetworkFromCertificate(s.Store, req.Certificate)
	if err != nil {
		return nil, err
	}

	return nil, s.App.Network().Add(network)
}
