// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		vnicv1.RegisterVNICFrontendService(server, &vnicFrontendService{
			store: params.Store,
		})
	})
}

type vnicFrontendService struct {
	store dao.Store
}

func (s *vnicFrontendService) GetConfig(ctx context.Context, r *vnicv1.GetConfigRequest) (*vnicv1.GetConfigResponse, error) {
	config, err := dao.VNICConfig.Get(s.store)
	if err != nil {
		return nil, err
	}
	return &vnicv1.GetConfigResponse{Config: config}, nil
}

func (s *vnicFrontendService) SetConfig(ctx context.Context, r *vnicv1.SetConfigRequest) (*vnicv1.SetConfigResponse, error) {
	if err := dao.VNICConfig.Set(s.store, r.Config); err != nil {
		return nil, err
	}
	return &vnicv1.SetConfigResponse{Config: r.Config}, nil
}
