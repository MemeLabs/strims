// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"
	"errors"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrProfileNameNotAvailable = errors.New("profile name not available")
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		profilev1.RegisterProfileFrontendService(server, &profileService{
			store: params.Store,
			app:   params.App,
		})
	})
}

// profileService ...
type profileService struct {
	profilev1.UnimplementedProfileFrontendService
	logger *zap.Logger
	store  dao.Store
	app    app.Control
}

// Update ...
func (s *profileService) Update(ctx context.Context, r *profilev1.UpdateProfileRequest) (*profilev1.UpdateProfileResponse, error) {
	return &profilev1.UpdateProfileResponse{}, rpc.ErrNotImplemented
}

// Get ...
func (s *profileService) Get(ctx context.Context, r *profilev1.GetProfileRequest) (*profilev1.GetProfileResponse, error) {
	profile, err := dao.Profile.Get(s.store)
	if err != nil {
		return nil, err
	}

	return &profilev1.GetProfileResponse{Profile: profile}, nil
}
