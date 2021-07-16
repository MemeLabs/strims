package frontend

import (
	"context"

	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		chatv1.RegisterChatService(server, &chatService{})
	})
}

// chatService ...
type chatService struct{}

// CreateServer ...
func (s *chatService) CreateServer(ctx context.Context, req *chatv1.CreateChatServerRequest) (*chatv1.CreateChatServerResponse, error) {
	return &chatv1.CreateChatServerResponse{}, rpc.ErrNotImplemented
}

// UpdateServer ...
func (s *chatService) UpdateServer(ctx context.Context, req *chatv1.UpdateChatServerRequest) (*chatv1.UpdateChatServerResponse, error) {
	return &chatv1.UpdateChatServerResponse{}, rpc.ErrNotImplemented
}

// DeleteServer ...
func (s *chatService) DeleteServer(ctx context.Context, req *chatv1.DeleteChatServerRequest) (*chatv1.DeleteChatServerResponse, error) {
	return &chatv1.DeleteChatServerResponse{}, rpc.ErrNotImplemented
}

// GetServer ...
func (s *chatService) GetServer(ctx context.Context, req *chatv1.GetChatServerRequest) (*chatv1.GetChatServerResponse, error) {
	return &chatv1.GetChatServerResponse{}, rpc.ErrNotImplemented
}

// ListServers ...
func (s *chatService) ListServers(ctx context.Context, req *chatv1.ListChatServersRequest) (*chatv1.ListChatServersResponse, error) {
	return &chatv1.ListChatServersResponse{}, rpc.ErrNotImplemented
}

// OpenServer ...
func (s *chatService) OpenServer(ctx context.Context, req *chatv1.OpenChatServerRequest) (<-chan *chatv1.ChatServerEvent, error) {
	return nil, rpc.ErrNotImplemented
}

// OpenClient ...
func (s *chatService) OpenClient(ctx context.Context, req *chatv1.OpenChatClientRequest) (<-chan *chatv1.ChatClientEvent, error) {
	return nil, rpc.ErrNotImplemented
}

// CallClient ...
func (s *chatService) CallClient(ctx context.Context, req *chatv1.CallChatClientRequest) (*chatv1.CallChatClientResponse, error) {
	return &chatv1.CallChatClientResponse{}, rpc.ErrNotImplemented
}
