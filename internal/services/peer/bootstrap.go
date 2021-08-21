package peer

import (
	"context"
	"errors"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	"github.com/MemeLabs/protobuf/pkg/rpc"
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
