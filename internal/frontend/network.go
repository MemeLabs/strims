// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"google.golang.org/protobuf/proto"
)

// errors ...
var (
	ErrAlreadyJoinedNetwork = errors.New("user already has a membership for that network")
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		networkv1.RegisterNetworkFrontendService(server, &networkService{
			profile: params.Profile,
			store:   params.Store,
			app:     params.App,
		})
	})
}

// networkService ...
type networkService struct {
	profile *profilev1.Profile
	store   dao.Store
	app     app.Control
}

// CreateServer ...
func (s *networkService) CreateServer(ctx context.Context, r *networkv1.CreateServerRequest) (*networkv1.CreateServerResponse, error) {
	var opts []dao.NewNetworkOption
	if r.Alias != "" {
		opts = append(opts, dao.WithCertificateRequestOption(dao.WithSubject(r.Alias)))
	}

	network, err := dao.NewNetwork(s.store, r.Name, r.Icon, s.profile, opts...)
	if err != nil {
		return nil, err
	}

	if err := dao.Networks.Insert(s.store, network); err != nil {
		return nil, err
	}

	return &networkv1.CreateServerResponse{Network: network}, nil
}

// UpdateServerConfig ...
func (s *networkService) UpdateServerConfig(ctx context.Context, r *networkv1.UpdateServerConfigRequest) (*networkv1.UpdateServerConfigResponse, error) {
	if r.ServerConfig == nil {
		return nil, errors.New("null server config received")
	}

	network, err := dao.Networks.Transform(s.store, r.NetworkId, func(p *networkv1.Network) error {
		if p.ServerConfig == nil {
			return errors.New("previous network server config not found")
		}
		p.ServerConfig = r.ServerConfig
		return nil
	})
	if err != nil {
		return nil, err
	}

	return &networkv1.UpdateServerConfigResponse{Network: network}, nil
}

// Delete ...
func (s *networkService) Delete(ctx context.Context, r *networkv1.DeleteNetworkRequest) (*networkv1.DeleteNetworkResponse, error) {
	if err := dao.Networks.Delete(s.store, r.Id); err != nil {
		return nil, err
	}
	return &networkv1.DeleteNetworkResponse{}, nil
}

// Get ...
func (s *networkService) Get(ctx context.Context, r *networkv1.GetNetworkRequest) (*networkv1.GetNetworkResponse, error) {
	network, err := dao.Networks.Get(s.store, r.Id)
	if err != nil {
		return nil, err
	}
	return &networkv1.GetNetworkResponse{Network: network}, nil
}

// List ...
func (s *networkService) List(ctx context.Context, r *networkv1.ListNetworksRequest) (*networkv1.ListNetworksResponse, error) {
	networks, err := dao.Networks.GetAll(s.store)
	if err != nil {
		return nil, err
	}
	return &networkv1.ListNetworksResponse{Networks: networks}, nil
}

// CreateInvitation ...
func (s *networkService) CreateInvitation(ctx context.Context, r *networkv1.CreateInvitationRequest) (*networkv1.CreateInvitationResponse, error) {
	network, err := dao.Networks.Get(s.store, r.NetworkId)
	if err != nil {
		return nil, err
	}

	var bootstrapClients []*networkv1bootstrap.BootstrapClient
	if r.BootstrapClientId != 0 {
		bootstrapClient, err := dao.BootstrapClients.Get(s.store, r.BootstrapClientId)
		if err != nil {
			return nil, err
		}
		bootstrapClients = append(bootstrapClients, bootstrapClient)
	}

	invitation, err := dao.NewInvitationV0(s.profile.Key, network.Certificate, bootstrapClients)
	if err != nil {
		return nil, err
	}

	b, err := proto.Marshal(invitation)
	if err != nil {
		return nil, err
	}

	return &networkv1.CreateInvitationResponse{
		Invitation: &networkv1.Invitation{
			Version: 0,
			Data:    b,
		},
	}, nil
}

// CreateNetworkFromInvitation ...
func (s *networkService) CreateNetworkFromInvitation(ctx context.Context, r *networkv1.CreateNetworkFromInvitationRequest) (*networkv1.CreateNetworkFromInvitationResponse, error) {
	var invBytes []byte

	switch x := r.Invitation.(type) {
	case *networkv1.CreateNetworkFromInvitationRequest_InvitationB64:
		var err error
		invBytes, err = base64.StdEncoding.WithPadding(base64.NoPadding).DecodeString(r.GetInvitationB64())
		if err != nil {
			return nil, err
		}
	case *networkv1.CreateNetworkFromInvitationRequest_InvitationBytes:
		invBytes = r.GetInvitationBytes()
	case nil:
		return nil, errors.New("invitation has no content")
	default:
		return nil, fmt.Errorf("invitation has unexpected type %T", x)
	}

	var wrapper networkv1.Invitation
	err := proto.Unmarshal(invBytes, &wrapper)
	if err != nil {
		return nil, err
	}

	var invitation networkv1.InvitationV0
	err = proto.Unmarshal(wrapper.Data, &invitation)
	if err != nil {
		return nil, err
	}

	network, err := dao.NewNetworkFromInvitationV0(s.store, &invitation, s.profile, dao.WithAlias(r.Alias))
	if err != nil {
		return nil, err
	}

	var bootstrapClients []*networkv1bootstrap.BootstrapClient
	for _, c := range invitation.BootstrapClients {
		c, err := dao.NewBootstrapClient(s.store, c)
		if err != nil {
			return nil, err
		}
		bootstrapClients = append(bootstrapClients, c)
	}

	if err := dao.Networks.Insert(s.store, network); err != nil {
		return nil, err
	}

	if len(bootstrapClients) != 0 {
		err = s.store.Update(func(tx kv.RWTx) error {
			for _, c := range bootstrapClients {
				if err := dao.BootstrapClients.Insert(tx, c); err != nil {
					if !errors.Is(err, dao.ErrUniqueConstraintViolated) {
						return err
					}
				}
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
	}

	return &networkv1.CreateNetworkFromInvitationResponse{
		Network: network,
	}, nil
}

// Watch ...
func (s *networkService) Watch(ctx context.Context, r *networkv1.WatchNetworksRequest) (<-chan *networkv1.WatchNetworksResponse, error) {
	ch := make(chan *networkv1.WatchNetworksResponse, 1)

	go func() {
		for e := range s.app.Network().ReadEvents(ctx) {
			ch <- &networkv1.WatchNetworksResponse{Event: e}
		}
	}()

	return ch, nil
}

// UpdateDisplayOrder ...
func (s *networkService) UpdateDisplayOrder(ctx context.Context, r *networkv1.UpdateDisplayOrderRequest) (*networkv1.UpdateDisplayOrderResponse, error) {
	_, err := dao.NetworkUIConfig.Transform(s.store, func(p *networkv1.UIConfig) error {
		p.NetworkDisplayOrder = r.NetworkIds
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &networkv1.UpdateDisplayOrderResponse{}, nil
}

// UpdateAlias ...
func (s *networkService) UpdateAlias(ctx context.Context, r *networkv1.UpdateAliasRequest) (*networkv1.UpdateAliasResponse, error) {
	if err := s.app.Network().SetAlias(r.Id, r.Alias); err != nil {
		return nil, err
	}
	return &networkv1.UpdateAliasResponse{}, nil
}

// GetUIConfig ...
func (s *networkService) GetUIConfig(ctx context.Context, r *networkv1.GetUIConfigRequest) (*networkv1.GetUIConfigResponse, error) {
	c, err := dao.NetworkUIConfig.Get(s.store)
	if err != nil {
		return nil, err
	}
	return &networkv1.GetUIConfigResponse{Config: c}, nil
}

// ListPeers ...
func (s *networkService) ListPeers(ctx context.Context, r *networkv1.ListPeersRequest) (*networkv1.ListPeersResponse, error) {
	vs, err := dao.NetworkPeersByNetwork.GetAllByRefID(s.store, r.NetworkId)
	if err != nil {
		return nil, err
	}
	return &networkv1.ListPeersResponse{Peers: vs}, nil
}

func (s *networkService) GrantPeerInvitation(ctx context.Context, r *networkv1.GrantPeerInvitationRequest) (*networkv1.GrantPeerInvitationResponse, error) {
	p, err := dao.NetworkPeers.Transform(s.store, r.Id, func(p *networkv1.Peer) error {
		p.InviteQuota += r.Count
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &networkv1.GrantPeerInvitationResponse{Peer: p}, nil
}

func (s *networkService) TogglePeerBan(ctx context.Context, r *networkv1.TogglePeerBanRequest) (*networkv1.TogglePeerBanResponse, error) {
	p, err := dao.NetworkPeers.Transform(s.store, r.Id, func(p *networkv1.Peer) error {
		p.IsBanned = r.Value
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &networkv1.TogglePeerBanResponse{Peer: p}, nil
}

func (s *networkService) ResetPeerRenameCooldown(ctx context.Context, r *networkv1.ResetPeerRenameCooldownRequest) (*networkv1.ResetPeerRenameCooldownResponse, error) {
	p, err := dao.NetworkPeers.Transform(s.store, r.Id, func(p *networkv1.Peer) error {
		p.AliasChangedAt = 0
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &networkv1.ResetPeerRenameCooldownResponse{Peer: p}, nil
}

func (s *networkService) DeletePeer(ctx context.Context, r *networkv1.DeletePeerRequest) (*networkv1.DeletePeerResponse, error) {
	err := s.store.Update(func(tx kv.RWTx) error {
		p, err := dao.NetworkPeers.Get(tx, r.Id)
		if err != nil {
			return err
		}
		if err := dao.NetworkPeers.Delete(tx, r.Id); err != nil {
			return err
		}

		rid, err := dao.NetworkAliasReservationsByAlias.GetID(tx, dao.FormatNetworkAliasReservationAliasKey(p.NetworkId, p.Alias))
		if err != nil {
			return err
		}
		return dao.NetworkAliasReservations.Release(tx, rid, dao.NetworkAliasReservationCooldown)
	})
	if err != nil {
		return nil, err
	}
	return &networkv1.DeletePeerResponse{}, nil
}

// ListAliasReservations ...
func (s *networkService) ListAliasReservations(ctx context.Context, r *networkv1.ListAliasReservationsRequest) (*networkv1.ListAliasReservationsResponse, error) {
	vs, err := dao.NetworkAliasReservationsByNetwork.GetAllByRefID(s.store, r.NetworkId)
	if err != nil {
		return nil, err
	}
	return &networkv1.ListAliasReservationsResponse{AliasReservations: vs}, nil
}

// ResetAliasReservationCooldown ...
func (s *networkService) ResetAliasReservationCooldown(ctx context.Context, r *networkv1.ResetAliasReservationCooldownRequest) (*networkv1.ResetAliasReservationCooldownResponse, error) {
	_, err := dao.NetworkAliasReservations.Transform(s.store, r.Id, func(p *networkv1.AliasReservation) error {
		if p.PeerKey != nil {
			return errors.New("cannot reset cooldown for active reservation")
		}
		p.ReservedUntil = 0
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &networkv1.ResetAliasReservationCooldownResponse{}, nil
}
