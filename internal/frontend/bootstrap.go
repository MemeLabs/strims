package frontend

import (
	"context"
	"fmt"
	"time"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		bootstrap.RegisterBootstrapFrontendService(server, &bootstrapService{
			logger: params.Logger,
			store:  params.Store,
			vpn:    params.VPN,
			app:    params.App,
		})
	})
}

// bootstrapService ...
type bootstrapService struct {
	logger *zap.Logger
	store  *dao.ProfileStore
	vpn    *vpn.Host
	app    control.AppControl
}

// CreateClient ...
func (s *bootstrapService) CreateClient(ctx context.Context, r *bootstrap.CreateBootstrapClientRequest) (*bootstrap.CreateBootstrapClientResponse, error) {
	var client *bootstrap.BootstrapClient
	var err error
	switch v := r.GetClientOptions().(type) {
	case *bootstrap.CreateBootstrapClientRequest_WebsocketOptions:
		client, err = dao.NewWebSocketBootstrapClient(s.store, v.WebsocketOptions.Url, v.WebsocketOptions.InsecureSkipVerifyTls)
	}
	if err != nil {
		return nil, err
	}

	if err := dao.InsertBootstrapClient(s.store, client); err != nil {
		return nil, err
	}

	return &bootstrap.CreateBootstrapClientResponse{BootstrapClient: client}, nil
}

// UpdateClient ...
func (s *bootstrapService) UpdateClient(ctx context.Context, r *bootstrap.UpdateBootstrapClientRequest) (*bootstrap.UpdateBootstrapClientResponse, error) {

	return &bootstrap.UpdateBootstrapClientResponse{BootstrapClient: nil}, nil
}

// DeleteClient ...
func (s *bootstrapService) DeleteClient(ctx context.Context, r *bootstrap.DeleteBootstrapClientRequest) (*bootstrap.DeleteBootstrapClientResponse, error) {
	if err := dao.DeleteBootstrapClient(s.store, r.Id); err != nil {
		return nil, err
	}

	return &bootstrap.DeleteBootstrapClientResponse{}, nil
}

// GetClient ...
func (s *bootstrapService) GetClient(ctx context.Context, r *bootstrap.GetBootstrapClientRequest) (*bootstrap.GetBootstrapClientResponse, error) {
	return &bootstrap.GetBootstrapClientResponse{BootstrapClient: nil}, nil
}

// ListClients ...
func (s *bootstrapService) ListClients(ctx context.Context, r *bootstrap.ListBootstrapClientsRequest) (*bootstrap.ListBootstrapClientsResponse, error) {
	clients, err := dao.GetBootstrapClients(s.store)
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
	if err := s.app.Bootstrap().Publish(ctx, r.PeerId, r.Network, time.Hour*24*365*2); err != nil {
		return nil, err
	}

	return &bootstrap.PublishNetworkToBootstrapPeerResponse{}, nil
}
