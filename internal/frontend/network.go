package frontend

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"

	"github.com/MemeLabs/go-ppspp/internal/app"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"google.golang.org/protobuf/proto"
)

// errors ...
var (
	ErrAlreadyJoinedNetwork = errors.New("user already has a membership for that network")
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		networkv1.RegisterNetworkServiceService(server, &networkService{
			profile: params.Profile,
			store:   params.Store,
			app:     params.App,
		})
	})
}

// networkService ...
type networkService struct {
	profile *profilev1.Profile
	store   *dao.ProfileStore
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

	var logs []*networkv1ca.CertificateLog
	for c := network.Certificate; c != nil; c = c.GetParent() {
		log, err := dao.NewCertificateLog(s.store, network.Id, c)
		if err != nil {
			return nil, err
		}
		logs = append(logs, log)
	}

	if err := s.app.Network().Add(network, logs); err != nil {
		return nil, err
	}

	return &networkv1.CreateServerResponse{Network: network}, nil
}

// UpdateServerConfig ...
func (s *networkService) UpdateServerConfig(ctx context.Context, r *networkv1.UpdateServerConfigRequest) (*networkv1.UpdateServerConfigResponse, error) {
	network, err := dao.Networks.Get(s.store, r.NetworkId)
	if err != nil {
		return nil, err
	}

	if network.GetServerConfig() == nil {
		return nil, errors.New("network server config not set")
	}
	if r.ServerConfig == nil {
		return nil, errors.New("network server config not set")
	}

	network.ServerConfigOneof = &networkv1.Network_ServerConfig{
		ServerConfig: r.ServerConfig,
	}

	if err := dao.Networks.Update(s.store, network); err != nil {
		return nil, err
	}

	return &networkv1.UpdateServerConfigResponse{Network: network}, nil
}

// Delete ...
func (s *networkService) Delete(ctx context.Context, r *networkv1.DeleteNetworkRequest) (*networkv1.DeleteNetworkResponse, error) {
	if err := s.app.Network().Remove(r.Id); err != nil {
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
	invitation, err := dao.NewInvitationV0(r.SigningKey, r.SigningCert)
	if err != nil {
		return nil, err
	}

	b, err := proto.Marshal(invitation)
	if err != nil {
		return nil, err
	}

	b, err = proto.Marshal(&networkv1.Invitation{
		Version: 0,
		Data:    b,
	})
	if err != nil {
		return nil, err
	}

	b64 := base64.StdEncoding.WithPadding(base64.NoPadding).EncodeToString(b)

	return &networkv1.CreateInvitationResponse{
		InvitationB64:   b64,
		InvitationBytes: b,
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

	network, err := dao.NewNetworkFromInvitationV0(s.store, &invitation, s.profile)
	if err != nil {
		return nil, err
	}

	if err := s.app.Network().Add(network, nil); err != nil {
		return nil, err
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
	if err := s.app.Network().UpdateDisplayOrder(r.NetworkIds); err != nil {
		return nil, err
	}
	return &networkv1.UpdateDisplayOrderResponse{}, nil
}

// UpdateAlias ...
func (s *networkService) UpdateAlias(ctx context.Context, r *networkv1.UpdateAliasRequest) (*networkv1.UpdateAliasResponse, error) {
	return nil, rpc.ErrNotImplemented
}
