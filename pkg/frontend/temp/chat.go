package frontend

import (
	"context"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/frontend/server"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
)

func newChatService(logger *zap.Logger, store *dao.ProfileStore) api.ChatService {
	return &chatService{logger, store}
}

// chatService ...
type chatService struct {
	logger *zap.Logger
	store  *dao.ProfileStore
}

// CreateChatServer ...
func (s *chatService) CreateServer(ctx context.Context, r *pb.CreateChatServerRequest) (*pb.CreateChatServerResponse, error) {
	session, err := server.AuthenticatedContextSession(ctx)
	if err != nil {
		return nil, err
	}

	server, err := dao.NewChatServer(s.store, r.NetworkKey, r.ChatRoom)
	if err != nil {
		return nil, err
	}

	if err := dao.InsertChatServer(session.ProfileStore(), server); err != nil {
		return nil, err
	}

	return &pb.CreateChatServerResponse{ChatServer: server}, nil
}

// UpdateChatServer ...
func (s *chatService) UpdateServer(ctx context.Context, r *pb.UpdateChatServerRequest) (*pb.UpdateChatServerResponse, error) {

	return &pb.UpdateChatServerResponse{ChatServer: nil}, nil
}

// DeleteChatServer ...
func (s *chatService) DeleteServer(ctx context.Context, r *pb.DeleteChatServerRequest) (*pb.DeleteChatServerResponse, error) {
	session, err := server.AuthenticatedContextSession(ctx)
	if err != nil {
		return nil, err
	}

	if err := dao.DeleteChatServer(session.ProfileStore(), r.Id); err != nil {
		return nil, err
	}

	return &pb.DeleteChatServerResponse{}, nil
}

// GetChatServer ...
func (s *chatService) GetServer(ctx context.Context, r *pb.GetChatServerRequest) (*pb.GetChatServerResponse, error) {
	return &pb.GetChatServerResponse{ChatServer: nil}, nil
}

// ListServers ...
func (s *chatService) ListServers(ctx context.Context, r *pb.ListChatServersRequest) (*pb.ListChatServersResponse, error) {
	session, err := server.AuthenticatedContextSession(ctx)
	if err != nil {
		return nil, err
	}

	servers, err := dao.GetChatServers(session.ProfileStore())
	if err != nil {
		return nil, err
	}

	return &pb.ListChatServersResponse{ChatServers: servers}, nil
}

// OpenChatServer ...
func (s *chatService) OpenServer(ctx context.Context, r *pb.OpenChatServerRequest) (<-chan *pb.ChatServerEvent, error) {
	session, err := server.AuthenticatedContextSession(ctx)
	if err != nil {
		return nil, err
	}

	ctl, err := s.getNetworkController(ctx)
	if err != nil {
		return nil, err
	}

	ch := make(chan *pb.ChatServerEvent, 1)

	// TODO: this should return an ErrNetworkNotFound...
	svc, ok := ctl.NetworkServices(r.Server.NetworkKey)
	if !ok {
		return nil, errors.New("unknown network")
	}

	server, err := NewChatServer(s.logger, svc, r.Server.Key)
	if err != nil {
		return nil, err
	}

	id := session.Store(server)
	ch <- &pb.ChatServerEvent{
		Body: &pb.ChatServerEvent_Open_{
			Open: &pb.ChatServerEvent_Open{
				ServerId: id,
			},
		},
	}

	go func() {
		for e := range server.Events() {
			ch <- e
		}
		ch <- &pb.ChatServerEvent{
			Body: &pb.ChatServerEvent_Close_{
				Close: &pb.ChatServerEvent_Close{},
			},
		}

		session.Delete(id)
		close(ch)
	}()

	return ch, nil
}

// CallChatServer ...
func (s *chatService) CallServer(ctx context.Context, r *pb.CallChatServerRequest) error {
	session, err := server.AuthenticatedContextSession(ctx)
	if err != nil {
		return nil, err
	}

	serverIf, _ := session.Load(r.ServerId)
	server, ok := serverIf.(*ChatServer)
	if !ok {
		return errors.New("server id does not exist")
	}

	switch r.Body.(type) {
	case *pb.CallChatServerRequest_Close_:
		server.Close()
	}

	return nil
}

// OpenChatClient ...
func (s *chatService) OpenClient(ctx context.Context, r *pb.OpenChatClientRequest) (<-chan *pb.ChatClientEvent, error) {
	ctl, err := s.getNetworkController(ctx)
	if err != nil {
		return nil, err
	}

	ch := make(chan *pb.ChatClientEvent, 1)

	// TODO: this should return an ErrNetworkNotFound...
	svc, ok := ctl.NetworkServices(r.NetworkKey)
	if !ok {
		return nil, errors.New("unknown network")
	}

	session := ContextSession(ctx)

	client, err := NewChatClient(s.logger, svc, r.ServerKey)
	if err != nil {
		return nil, err
	}

	id := session.Store(client)
	ch <- &pb.ChatClientEvent{
		Body: &pb.ChatClientEvent_Open_{
			Open: &pb.ChatClientEvent_Open{
				ClientId: id,
			},
		},
	}

	go func() {
		for e := range client.Events() {
			ch <- e
		}
		ch <- &pb.ChatClientEvent{
			Body: &pb.ChatClientEvent_Close_{
				Close: &pb.ChatClientEvent_Close{},
			},
		}

		session.Delete(id)
		close(ch)
	}()

	return ch, nil
}

// CallChatClient ...
func (s *chatService) CallClient(ctx context.Context, r *pb.CallChatClientRequest) (*pb.CallChatClientResponse, error) {
	session, err := server.AuthenticatedContextSession(ctx)
	if err != nil {
		return nil, err
	}

	clientIf, _ := session.Load(r.ClientId)
	client, ok := clientIf.(*ChatClient)
	if !ok {
		return nil, errors.New("client id does not exist")
	}

	switch b := r.Body.(type) {
	case *pb.CallChatClientRequest_Message_:
		if err := client.Send(&pb.ChatClientEvent_Message{
			Body: b.Message.Body,
		}); err != nil {
			return nil, err
		}
	case *pb.CallChatClientRequest_Close_:
		client.Close()
	}

	return &pb.CallChatClientResponse{}, nil
}
