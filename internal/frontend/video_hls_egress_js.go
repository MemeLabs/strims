// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package frontend

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		videov1.RegisterHLSEgressService(server, &videoHLSEgressService{})
	})
}

// videoHLSEgressService ...
type videoHLSEgressService struct {
	videov1.UnimplementedHLSEgressService
}

// IsSupported ...
func (s *videoHLSEgressService) IsSupported(ctx context.Context, r *videov1.HLSEgressIsSupportedRequest) (*videov1.HLSEgressIsSupportedResponse, error) {
	return &videov1.HLSEgressIsSupportedResponse{Supported: false}, nil
}
