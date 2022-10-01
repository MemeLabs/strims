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
)

// errors ...
var (
	ErrProfileNameNotAvailable = errors.New("profile name not available")
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		profilev1.RegisterProfileFrontendService(server, &profileService{
			profile: params.Profile,
			store:   params.Store,
			app:     params.App,
		})
	})
}

// profileService ...
type profileService struct {
	profilev1.UnimplementedProfileFrontendService
	profile *profilev1.Profile
	store   dao.Store
	app     app.Control
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

// DeleteDevice ...
func (s *profileService) DeleteDevice(ctx context.Context, r *profilev1.DeleteDeviceRequest) (*profilev1.DeleteDeviceResponse, error) {
	if r.Id == s.profile.DeviceId {
		return nil, errors.New("cannot delete current device")
	}

	if err := dao.Devices.Delete(s.store, r.Id); err != nil {
		return nil, err
	}
	return &profilev1.DeleteDeviceResponse{}, nil
}

// GetDevice ...
func (s *profileService) GetDevice(ctx context.Context, r *profilev1.GetDeviceRequest) (*profilev1.GetDeviceResponse, error) {
	d, err := dao.Devices.Get(s.store, r.Id)
	if err != nil {
		return nil, err
	}
	return &profilev1.GetDeviceResponse{Device: d}, nil
}

// ListDevices ...
func (s *profileService) ListDevices(ctx context.Context, r *profilev1.ListDevicesRequest) (*profilev1.ListDevicesResponse, error) {
	ds, err := dao.Devices.GetAll(s.store)
	if err != nil {
		return nil, err
	}
	res := &profilev1.ListDevicesResponse{}
	for _, d := range ds {
		if d.Id == s.profile.DeviceId {
			res.CurrentDevice = d
		} else {
			res.Devices = append(res.Devices, d)
		}
	}
	return res, nil
}
