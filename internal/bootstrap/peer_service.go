// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bootstrap

import (
	"context"
	"errors"
	"sync/atomic"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	networkv1bootstrap "github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	"github.com/MemeLabs/strims/pkg/vnic"
)

var _ networkv1bootstrap.PeerServiceService = (*peerService)(nil)

type peerService struct {
	store               dao.Store
	vnicPeer            *vnic.Peer
	client              *networkv1bootstrap.PeerServiceClient
	allowReceivePublish *atomic.Bool
	allowSendPublish    atomic.Bool
}

func (s *peerService) GetPublishEnabled(ctx context.Context, req *bootstrap.BootstrapPeerGetPublishEnabledRequest) (*bootstrap.BootstrapPeerGetPublishEnabledResponse, error) {
	return &bootstrap.BootstrapPeerGetPublishEnabledResponse{Enabled: s.allowReceivePublish.Load()}, nil
}

func (s *peerService) ListNetworks(ctx context.Context, req *bootstrap.BootstrapPeerListNetworksRequest) (*bootstrap.BootstrapPeerListNetworksResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *peerService) Publish(ctx context.Context, req *bootstrap.BootstrapPeerPublishRequest) (*bootstrap.BootstrapPeerPublishResponse, error) {
	if !s.allowReceivePublish.Load() {
		return nil, errors.New("not supported")
	}

	network, err := dao.NewNetworkFromCertificate(s.store, req.Certificate)
	if err != nil {
		return nil, err
	}

	if err := dao.Networks.Insert(s.store, network); err != nil {
		return nil, err
	}

	return &bootstrap.BootstrapPeerPublishResponse{}, nil
}
