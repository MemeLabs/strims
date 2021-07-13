package frontend

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/control"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"google.golang.org/protobuf/proto"
)

// errors ...
var (
	ErrAlreadyJoinedNetwork = errors.New("user already has a membership for that network")
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
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
	app     control.AppControl
}

// Create ...
func (s *networkService) Create(ctx context.Context, r *networkv1.CreateNetworkRequest) (*networkv1.CreateNetworkResponse, error) {
	network, err := dao.NewNetwork(s.store, r.Name, r.Icon, s.profile)
	if err != nil {
		return nil, err
	}

	if err := s.app.Network().Add(network); err != nil {
		return nil, err
	}

	return &networkv1.CreateNetworkResponse{Network: network}, nil
}

// Update ...
func (s *networkService) Update(ctx context.Context, r *networkv1.UpdateNetworkRequest) (*networkv1.UpdateNetworkResponse, error) {
	return nil, rpc.ErrNotImplemented
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
	network, err := dao.GetNetwork(s.store, r.Id)
	if err != nil {
		return nil, err
	}
	return &networkv1.GetNetworkResponse{Network: network}, nil
}

// List ...
func (s *networkService) List(ctx context.Context, r *networkv1.ListNetworksRequest) (*networkv1.ListNetworksResponse, error) {
	networks, err := dao.GetNetworks(s.store)
	if err != nil {
		return nil, err
	}
	return &networkv1.ListNetworksResponse{Networks: networks}, nil
}

// CreateInvitation ...
func (s *networkService) CreateInvitation(ctx context.Context, r *networkv1.CreateNetworkInvitationRequest) (*networkv1.CreateNetworkInvitationResponse, error) {
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

	return &networkv1.CreateNetworkInvitationResponse{
		InvitationB64:   b64,
		InvitationBytes: b,
	}, nil
}

// CreateFromInvitation ...
func (s *networkService) CreateFromInvitation(ctx context.Context, r *networkv1.CreateNetworkFromInvitationRequest) (*networkv1.CreateNetworkFromInvitationResponse, error) {
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
		return nil, errors.New("Invitation has no content")
	default:
		return nil, fmt.Errorf("Invitation has unexpected type %T", x)
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

	if err := s.app.Network().Add(network); err != nil {
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

// SetDisplayOrder ...
func (s *networkService) SetDisplayOrder(ctx context.Context, r *networkv1.SetDisplayOrderRequest) (*networkv1.SetDisplayOrderResponse, error) {
	if err := s.app.Network().SetDisplayOrder(r.NetworkIds); err != nil {
		return nil, err
	}
	return &networkv1.SetDisplayOrderResponse{}, nil
}
