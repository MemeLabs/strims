// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package videoingress

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/network/dialer"
	video "github.com/MemeLabs/strims/pkg/apis/video/v1"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

// ShareAddressSalt ...
var ShareAddressSalt = []byte("ingressshare")

func newShareService(logger *zap.Logger, node *vpn.Node, store *dao.ProfileStore) *shareService {
	return &shareService{}
}

type shareService struct {
}

func (s *shareService) Run(ctx context.Context) error {
	return nil
}

func (s *shareService) CreateChannel(ctx context.Context, req *video.VideoIngressShareCreateChannelRequest) (*video.VideoIngressShareCreateChannelResponse, error) {
	cert := dialer.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}

func (s *shareService) UpdateChannel(ctx context.Context, req *video.VideoIngressShareUpdateChannelRequest) (*video.VideoIngressShareUpdateChannelResponse, error) {
	cert := dialer.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}

func (s *shareService) DeleteChannel(ctx context.Context, req *video.VideoIngressShareDeleteChannelRequest) (*video.VideoIngressShareDeleteChannelResponse, error) {
	cert := dialer.VPNCertificate(ctx).GetParent()
	_ = cert

	return nil, rpc.ErrNotImplemented
}
