// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	"go.uber.org/zap"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		bootstrap.RegisterBootstrapFrontendService(server, &bootstrapService{
			logger: params.Logger,
			store:  params.Store,
			app:    params.App,
		})
	})
}

// bootstrapService ...
type bootstrapService struct {
	logger *zap.Logger
	store  dao.Store
	app    app.Control
}

// SetConfig ...
func (s *bootstrapService) SetConfig(ctx context.Context, r *bootstrap.SetConfigRequest) (*bootstrap.SetConfigResponse, error) {
	if err := dao.BootstrapConfig.Set(s.store, r.Config); err != nil {
		return nil, err
	}
	return &bootstrap.SetConfigResponse{Config: r.Config}, nil
}

// GetConfig ...
func (s *bootstrapService) GetConfig(ctx context.Context, r *bootstrap.GetConfigRequest) (*bootstrap.GetConfigResponse, error) {
	config, err := dao.BootstrapConfig.Get(s.store)
	if err != nil {
		return nil, err
	}
	return &bootstrap.GetConfigResponse{Config: config}, nil
}

// CreateClient ...
func (s *bootstrapService) CreateClient(ctx context.Context, r *bootstrap.CreateBootstrapClientRequest) (*bootstrap.CreateBootstrapClientResponse, error) {
	var client *bootstrap.BootstrapClient
	var err error
	switch v := r.GetClientOptions().(type) {
	case *bootstrap.CreateBootstrapClientRequest_WebsocketOptions:
		client, err = dao.NewWebSocketBootstrapClient(s.store, v.WebsocketOptions.Url, v.WebsocketOptions.InsecureSkipVerifyTls)
	default:
		return nil, errors.New("unexpected client options type")
	}
	if err != nil {
		return nil, err
	}

	if err := dao.BootstrapClients.Insert(s.store, client); err != nil {
		return nil, err
	}

	return &bootstrap.CreateBootstrapClientResponse{BootstrapClient: client}, nil
}

// UpdateClient ...
func (s *bootstrapService) UpdateClient(ctx context.Context, r *bootstrap.UpdateBootstrapClientRequest) (*bootstrap.UpdateBootstrapClientResponse, error) {
	client, err := dao.BootstrapClients.Transform(s.store, r.Id, func(p *bootstrap.BootstrapClient) error {
		switch v := r.GetClientOptions().(type) {
		case *bootstrap.UpdateBootstrapClientRequest_WebsocketOptions:
			p.ClientOptions = &bootstrap.BootstrapClient_WebsocketOptions{
				WebsocketOptions: &bootstrap.BootstrapClientWebSocketOptions{
					Url:                   v.WebsocketOptions.Url,
					InsecureSkipVerifyTls: v.WebsocketOptions.InsecureSkipVerifyTls,
				},
			}
		default:
			return errors.New("unexpected client options type")
		}
		return nil
	})
	if err != nil {
		return nil, err
	}

	return &bootstrap.UpdateBootstrapClientResponse{BootstrapClient: client}, nil
}

// DeleteClient ...
func (s *bootstrapService) DeleteClient(ctx context.Context, r *bootstrap.DeleteBootstrapClientRequest) (*bootstrap.DeleteBootstrapClientResponse, error) {
	if err := dao.BootstrapClients.Delete(s.store, r.Id); err != nil {
		return nil, err
	}

	return &bootstrap.DeleteBootstrapClientResponse{}, nil
}

// GetClient ...
func (s *bootstrapService) GetClient(ctx context.Context, r *bootstrap.GetBootstrapClientRequest) (*bootstrap.GetBootstrapClientResponse, error) {
	client, err := dao.BootstrapClients.Get(s.store, r.Id)
	if err != nil {
		return nil, err
	}

	return &bootstrap.GetBootstrapClientResponse{BootstrapClient: client}, nil
}

// ListClients ...
func (s *bootstrapService) ListClients(ctx context.Context, r *bootstrap.ListBootstrapClientsRequest) (*bootstrap.ListBootstrapClientsResponse, error) {
	clients, err := dao.BootstrapClients.GetAll(s.store)
	if err != nil {
		return nil, err
	}

	return &bootstrap.ListBootstrapClientsResponse{BootstrapClients: clients}, nil
}

// ListPeers ...
func (s *bootstrapService) ListPeers(ctx context.Context, r *bootstrap.ListBootstrapPeersRequest) (*bootstrap.ListBootstrapPeersResponse, error) {
	peers := []*bootstrap.BootstrapPeer{}
	for _, p := range s.app.Peer().List() {
		cert := p.VNIC().Certificate
		peers = append(peers, &bootstrap.BootstrapPeer{
			PeerId: p.ID(),
			Label:  fmt.Sprintf("%s (%x)", cert.Subject, cert.Key),
		})
	}

	return &bootstrap.ListBootstrapPeersResponse{Peers: peers}, nil
}

// PublishNetworkToPeer ...
func (s *bootstrapService) PublishNetworkToPeer(ctx context.Context, r *bootstrap.PublishNetworkToBootstrapPeerRequest) (*bootstrap.PublishNetworkToBootstrapPeerResponse, error) {
	network, err := dao.Networks.Get(s.store, r.NetworkId)
	if err != nil {
		return nil, err
	}

	if err := s.app.Bootstrap().Publish(ctx, r.PeerId, network, time.Hour*24*365*2); err != nil {
		return nil, err
	}

	return &bootstrap.PublishNetworkToBootstrapPeerResponse{}, nil
}
