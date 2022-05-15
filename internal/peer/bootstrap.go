// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package peer

import (
	"context"
	"errors"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
)

type bootstrapService struct {
	Peer  app.Peer
	App   app.Control
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

	if err := s.App.Network().Add(network); err != nil {
		return nil, err
	}

	return &bootstrap.BootstrapPeerPublishResponse{}, nil
}
