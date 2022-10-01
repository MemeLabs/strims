// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"go.uber.org/zap"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		replicationv1.RegisterReplicationFrontendService(server, &replicationService{
			store: params.Store,
			app:   params.App,
		})
	})
}

// replicationService ...
type replicationService struct {
	replicationv1.UnimplementedReplicationFrontendService
	logger *zap.Logger
	store  dao.Store
	app    app.Control
}

func (s *replicationService) CreatePairingToken(ctx context.Context, r *replicationv1.CreatePairingTokenRequest) (*replicationv1.CreatePairingTokenResponse, error) {
	profile, err := dao.Profile.Get(s.store)
	if err != nil {
		return nil, err
	}

	devices, err := dao.Devices.GetAll(s.store)
	if err != nil {
		return nil, err
	}

	newDevice, err := dao.NewDevice(s.store, "", "")
	if err != nil {
		return nil, err
	}

	profile.DeviceId = newDevice.Id
	devices = append(devices, newDevice)

	auth, err := dao.GetServerAuthThing(s.store.BlobStore(), profile.Name)
	if err != nil {
		return nil, err
	}

	networks, err := dao.Networks.GetAll(s.store)
	if err != nil {
		return nil, err
	}

	bootstraps, err := dao.BootstrapClients.GetAll(s.store)
	if err != nil {
		return nil, err
	}

	nextID, lastID, err := dao.ProfileID.Incr(s.store, 1000)
	if err != nil {
		return nil, err
	}

	network := networks[0]
	network.ServerConfig = nil
	token := &authv1.PairingToken{
		Auth:       auth,
		Profile:    profile,
		Networks:   []*networkv1.Network{network},
		Bootstraps: bootstraps,
		Devices:    devices,
		ProfileId: &profilev1.ProfileID{
			NextId: nextID,
			LastId: lastID,
		},
	}

	return &replicationv1.CreatePairingTokenResponse{Token: token}, nil
}
