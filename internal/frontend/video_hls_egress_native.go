// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package frontend

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
	"go.uber.org/zap"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		videov1.RegisterHLSEgressService(server, &videoHLSEgressService{
			app:    params.App,
			logger: params.Logger,
			store:  params.Store,
		})
	})
}

// videoHLSEgressService ...
type videoHLSEgressService struct {
	app    app.Control
	logger *zap.Logger
	store  *dao.ProfileStore
}

// IsSupported ...
func (s *videoHLSEgressService) IsSupported(ctx context.Context, r *videov1.HLSEgressIsSupportedRequest) (*videov1.HLSEgressIsSupportedResponse, error) {
	return &videov1.HLSEgressIsSupportedResponse{Supported: true}, nil
}

func (s *videoHLSEgressService) GetConfig(ctx context.Context, r *videov1.HLSEgressGetConfigRequest) (*videov1.HLSEgressGetConfigResponse, error) {
	config, err := dao.HLSEgressConfig.Get(s.store)
	if err != nil {
		return nil, err
	}
	return &videov1.HLSEgressGetConfigResponse{Config: config}, nil
}

func (s *videoHLSEgressService) SetConfig(ctx context.Context, r *videov1.HLSEgressSetConfigRequest) (*videov1.HLSEgressSetConfigResponse, error) {
	if err := dao.HLSEgressConfig.Set(s.store, r.Config); err != nil {
		return nil, err
	}
	return &videov1.HLSEgressSetConfigResponse{Config: r.Config}, nil
}

// OpenStream ...
func (s *videoHLSEgressService) OpenStream(ctx context.Context, r *videov1.HLSEgressOpenStreamRequest) (*videov1.HLSEgressOpenStreamResponse, error) {
	uri, err := s.app.VideoEgress().OpenHLSStream(r.SwarmUri, r.NetworkKeys)
	if err != nil {
		s.logger.Debug("opening stream failed", zap.Error(err))
		return nil, err
	}

	return &videov1.HLSEgressOpenStreamResponse{
		PlaylistUrl: uri,
	}, nil
}

// CloseStream ...
func (s *videoHLSEgressService) CloseStream(ctx context.Context, r *videov1.HLSEgressCloseStreamRequest) (*videov1.HLSEgressCloseStreamResponse, error) {
	return nil, rpc.ErrNotImplemented
}
