package frontend

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/services/network"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// errors ...
var (
	ErrAlreadyJoinedNetwork = errors.New("user already has a membership for that network")
)

func newNetworkService(ctx context.Context, logger *zap.Logger, profile *pb.Profile, store *dao.ProfileStore, vpnHost *vpn.Host) (api.NetworkService, error) {
	svc := &networkService{
		ctx:     ctx,
		logger:  logger,
		profile: profile,
		store:   store,
		vpnHost: vpnHost,
	}

	networks, err := dao.GetNetworks(store)
	if err != nil {
		return nil, err
	}
	for _, n := range networks {
		if err := svc.startNetwork(ctx, n); err != nil {
			cert := dao.GetRootCert(n.Certificate)
			logger.Debug(
				"starting network failed",
				zap.String("networkName", cert.Subject),
				logutil.ByteHex("networkKey", cert.Key),
				zap.Error(err),
			)
		}
	}

	return svc, nil
}

// networkService ...
type networkService struct {
	ctx     context.Context
	logger  *zap.Logger
	profile *pb.Profile
	store   *dao.ProfileStore
	vpnHost *vpn.Host
}

// Create ...
func (s *networkService) Create(ctx context.Context, r *pb.CreateNetworkRequest) (*pb.CreateNetworkResponse, error) {
	network, err := dao.NewNetwork(r.Name, r.Icon, s.profile)
	if err != nil {
		return nil, err
	}

	if err := dao.UpsertNetwork(s.store, network); err != nil {
		return nil, err
	}

	if err := s.startNetwork(s.ctx, network); err != nil {
		return nil, err
	}

	return &pb.CreateNetworkResponse{Network: network}, nil
}

// Update ...
func (s *networkService) Update(ctx context.Context, r *pb.UpdateNetworkRequest) (*pb.UpdateNetworkResponse, error) {
	return nil, errors.New("not implemented")
}

// Delete ...
func (s *networkService) Delete(ctx context.Context, r *pb.DeleteNetworkRequest) (*pb.DeleteNetworkResponse, error) {
	err := s.store.Update(func(tx kv.RWTx) error {
		network, err := dao.GetNetwork(tx, r.Id)
		if err != nil {
			return err
		}
		s.vpnHost.RemoveNetwork(network.Key.Public)

		if err := dao.DeleteNetwork(tx, r.Id); err != nil {
			return err
		}
		return nil
	})
	return &pb.DeleteNetworkResponse{}, err
}

// Get ...
func (s *networkService) Get(ctx context.Context, r *pb.GetNetworkRequest) (*pb.GetNetworkResponse, error) {
	network, err := dao.GetNetwork(s.store, r.Id)
	if err != nil {
		return nil, err
	}
	return &pb.GetNetworkResponse{Network: network}, nil
}

// List ...
func (s *networkService) List(ctx context.Context, r *pb.ListNetworksRequest) (*pb.ListNetworksResponse, error) {
	networks, err := dao.GetNetworks(s.store)
	if err != nil {
		return nil, err
	}
	return &pb.ListNetworksResponse{Networks: networks}, nil
}

// CreateInvitation ...
func (s *networkService) CreateInvitation(ctx context.Context, r *pb.CreateNetworkInvitationRequest) (*pb.CreateNetworkInvitationResponse, error) {
	invitation, err := dao.NewInvitationV0(r.SigningKey, r.SigningCert)
	if err != nil {
		return nil, err
	}

	b, err := proto.Marshal(invitation)
	if err != nil {
		return nil, err
	}

	b, err = proto.Marshal(&pb.Invitation{
		Version: 0,
		Data:    b,
	})
	if err != nil {
		return nil, err
	}

	b64 := base64.StdEncoding.WithPadding(base64.NoPadding).EncodeToString(b)

	return &pb.CreateNetworkInvitationResponse{
		InvitationB64:   b64,
		InvitationBytes: b,
	}, nil
}

// CreateFromInvitation ...
func (s *networkService) CreateFromInvitation(ctx context.Context, r *pb.CreateNetworkFromInvitationRequest) (*pb.CreateNetworkFromInvitationResponse, error) {
	var invBytes []byte

	switch x := r.Invitation.(type) {
	case *pb.CreateNetworkFromInvitationRequest_InvitationB64:
		var err error
		invBytes, err = base64.StdEncoding.WithPadding(base64.NoPadding).DecodeString(r.GetInvitationB64())
		if err != nil {
			return nil, err
		}
	case *pb.CreateNetworkFromInvitationRequest_InvitationBytes:
		invBytes = r.GetInvitationBytes()
	case nil:
		return nil, errors.New("Invitation has no content")
	default:
		return nil, fmt.Errorf("Invitation has unexpected type %T", x)
	}

	var wrapper pb.Invitation
	err := proto.Unmarshal(invBytes, &wrapper)
	if err != nil {
		return nil, err
	}

	var invitation pb.InvitationV0
	err = proto.Unmarshal(wrapper.Data, &invitation)
	if err != nil {
		return nil, err
	}

	network, err := dao.NewNetworkFromInvitationV0(&invitation, s.profile)
	if err != nil {
		return nil, err
	}

	if err := s.startNetwork(s.ctx, network); err != nil {
		return nil, err
	}

	if err := dao.UpsertNetwork(s.store, network); err != nil {
		return nil, err
	}

	return &pb.CreateNetworkFromInvitationResponse{
		Network: network,
	}, nil
}

// StartVPN ...
func (s *networkService) StartVPN(ctx context.Context, r *pb.StartVPNRequest) (<-chan *pb.NetworkEvent, error) {
	return nil, errors.New("not implemented")
}

// StopVPN ...
func (s *networkService) StopVPN(ctx context.Context, r *pb.StopVPNRequest) (*pb.StopVPNResponse, error) {
	return nil, errors.New("not implemented")
}

func (s *networkService) startNetwork(ctx context.Context, n *pb.Network) error {
	client, err := s.vpnHost.AddNetwork(n.Certificate)
	if err != nil {
		return err
	}

	if n.Key != nil {
		_, err := network.NewCA(ctx, s.logger, client, n)
		if err != nil {
			return err
		}
	}
	return nil
}

// // loads the NetworkController fron the session store
// // TODO: move to (s *Session) getNetworkController ?
// func (s *Network) getNetworkController(ctx context.Context) (*NetworkController, error) {
// 	session, err := contextSession(ctx)
// 	if err != nil {
// 		return nil, err
// 	}

// 	d, ok := session.Values.Load(vpnKey)
// 	if !ok {
// 		return nil, errors.New("could not get vpn data")
// 	}

// 	data, ok := d.(vpnData)
// 	if !ok {
// 		return nil, errors.New("vpn data has unexpected type")
// 	}

// 	return data.controller, nil
// }

// GetDirectoryEvents ...
func (s *networkService) GetDirectoryEvents(ctx context.Context, r *pb.GetDirectoryEventsRequest) (<-chan *pb.DirectoryServerEvent, error) {
	// ctl, err := s.getNetworkController(ctx)
	// if err != nil {
	// 	return nil, err
	// }

	// // TODO: this should return an ErrNetworkNotFound...
	// svc, ok := ctl.NetworkServices(r.NetworkKey)
	// if !ok {
	// 	return nil, errors.New("unknown network")
	// }

	// ch := make(chan *pb.DirectoryServerEvent, 16)
	// svc.Directory.NotifyEvents(ch)

	// // TDOO: automatically remove closed channels from event.Observables
	// go func() {
	// 	<-ctx.Done()
	// 	s.logger.Debug("GetDirectoryEvents stream closed")
	// 	svc.Directory.StopNotifyingEvents(ch)
	// }()

	// return ch, nil

	return make(chan *pb.DirectoryServerEvent, 16), ErrMethodNotImplemented
}

// TestDirectoryPublish ...
func (s *networkService) TestDirectoryPublish(ctx context.Context, r *pb.TestDirectoryPublishRequest) (*pb.TestDirectoryPublishResponse, error) {
	// ctl, err := s.getNetworkController(ctx)
	// if err != nil {
	// 	return nil, err
	// }

	// // TODO: this should return an ErrNetworkNotFound...
	// svc, ok := ctl.NetworkServices(r.NetworkKey)
	// if !ok {
	// 	return nil, errors.New("unknown network")
	// }

	// key, err := dao.GenerateKey()
	// if err != nil {
	// 	return nil, err
	// }

	// err = svc.Directory.Publish(ctx, &pb.DirectoryListing{
	// 	Key:         key.Public,
	// 	MimeType:    "text/plain",
	// 	Title:       "test",
	// 	Description: "test publication",
	// 	Tags:        []string{"foo", "bar", "baz"},
	// })
	// if err != nil {
	// 	s.logger.Debug("publishing listing failed", zap.Error(err))
	// }

	// return &pb.TestDirectoryPublishResponse{}, err
	return &pb.TestDirectoryPublishResponse{}, ErrMethodNotImplemented
}
