package frontend

import (
	"context"
	"encoding/hex"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/services/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

func newBootstrapService(ctx context.Context, logger *zap.Logger, store *dao.ProfileStore, vpn *vpn.Host) (api.BootstrapService, error) {
	service := bootstrap.NewService(logger, vpn, store, bootstrap.ServiceOptions{EnablePublishing: true})

	clients, err := dao.GetBootstrapClients(store)
	if err != nil {
		return nil, err
	}
	for _, client := range clients {
		service.Dial(client)
	}

	return &bootstrapService{logger, store, vpn, service}, nil
}

// bootstrapService ...
type bootstrapService struct {
	logger  *zap.Logger
	store   *dao.ProfileStore
	vpn     *vpn.Host
	service *bootstrap.Service
}

// CreateClient ...
func (s *bootstrapService) CreateClient(ctx context.Context, r *pb.CreateBootstrapClientRequest) (*pb.CreateBootstrapClientResponse, error) {
	var client *pb.BootstrapClient
	var err error
	switch v := r.GetClientOptions().(type) {
	case *pb.CreateBootstrapClientRequest_WebsocketOptions:
		client, err = dao.NewWebSocketBootstrapClient(v.WebsocketOptions.Url, v.WebsocketOptions.InsecureSkipVerifyTls)
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
	for _, id := range s.service.GetPeerHostIDs() {
		peers = append(peers, &pb.BootstrapPeer{
			HostId: id.Bytes(nil),
			Label:  hex.EncodeToString(id.Bytes(nil)),
		})
	}

	return &pb.ListBootstrapPeersResponse{Peers: peers}, nil
}

// PublishNetworkToPeer ...
func (s *bootstrapService) PublishNetworkToPeer(ctx context.Context, r *pb.PublishNetworkToBootstrapPeerRequest) (*pb.PublishNetworkToBootstrapPeerResponse, error) {
	id, err := kademlia.UnmarshalID(r.HostId)
	if err != nil {
		return nil, err
	}

	if err := s.service.PublishNetwork(id, r.Network); err != nil {
		return nil, err
	}

	return &pb.PublishNetworkToBootstrapPeerResponse{}, nil
}
