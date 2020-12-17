package frontend

import (
	"context"
	"fmt"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/control/app"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

func newBootstrapService(ctx context.Context, logger *zap.Logger, store *dao.ProfileStore, vpn *vpn.Host, app *app.Control) api.BootstrapService {
	return &bootstrapService{logger, store, vpn, app}
}

// bootstrapService ...
type bootstrapService struct {
	logger *zap.Logger
	store  *dao.ProfileStore
	vpn    *vpn.Host
	app    *app.Control
}

// CreateClient ...
func (s *bootstrapService) CreateClient(ctx context.Context, r *pb.CreateBootstrapClientRequest) (*pb.CreateBootstrapClientResponse, error) {
	var client *pb.BootstrapClient
	var err error
	switch v := r.GetClientOptions().(type) {
	case *pb.CreateBootstrapClientRequest_WebsocketOptions:
		client, err = dao.NewWebSocketBootstrapClient(s.store, v.WebsocketOptions.Url, v.WebsocketOptions.InsecureSkipVerifyTls)
	}
	if err != nil {
		return nil, err
	}

	if err := dao.InsertBootstrapClient(s.store, client); err != nil {
		return nil, err
	}

	return &pb.CreateBootstrapClientResponse{BootstrapClient: client}, nil
}

// UpdateClient ...
func (s *bootstrapService) UpdateClient(ctx context.Context, r *pb.UpdateBootstrapClientRequest) (*pb.UpdateBootstrapClientResponse, error) {

	return &pb.UpdateBootstrapClientResponse{BootstrapClient: nil}, nil
}

// DeleteClient ...
func (s *bootstrapService) DeleteClient(ctx context.Context, r *pb.DeleteBootstrapClientRequest) (*pb.DeleteBootstrapClientResponse, error) {
	if err := dao.DeleteBootstrapClient(s.store, r.Id); err != nil {
		return nil, err
	}

	return &pb.DeleteBootstrapClientResponse{}, nil
}

// GetClient ...
func (s *bootstrapService) GetClient(ctx context.Context, r *pb.GetBootstrapClientRequest) (*pb.GetBootstrapClientResponse, error) {
	return &pb.GetBootstrapClientResponse{BootstrapClient: nil}, nil
}

// ListClients ...
func (s *bootstrapService) ListClients(ctx context.Context, r *pb.ListBootstrapClientsRequest) (*pb.ListBootstrapClientsResponse, error) {
	clients, err := dao.GetBootstrapClients(s.store)
	if err != nil {
		return nil, err
	}

	return &pb.ListBootstrapClientsResponse{BootstrapClients: clients}, nil
}

// ListPeers ...
func (s *bootstrapService) ListPeers(ctx context.Context, r *pb.ListBootstrapPeersRequest) (*pb.ListBootstrapPeersResponse, error) {
	peers := []*pb.BootstrapPeer{}
	for _, p := range s.app.Peer().List() {
		cert := p.VNIC().Certificate
		peers = append(peers, &pb.BootstrapPeer{
			PeerId: p.ID(),
			Label:  fmt.Sprintf("%s (%x)", cert.Subject, cert.Key),
		})
	}

	return &pb.ListBootstrapPeersResponse{Peers: peers}, nil
}

// PublishNetworkToPeer ...
func (s *bootstrapService) PublishNetworkToPeer(ctx context.Context, r *pb.PublishNetworkToBootstrapPeerRequest) (*pb.PublishNetworkToBootstrapPeerResponse, error) {
	if err := s.app.Bootstrap().Publish(ctx, r.PeerId, r.Network, time.Hour*24*365*2); err != nil {
		return nil, err
	}

	return &pb.PublishNetworkToBootstrapPeerResponse{}, nil
}
