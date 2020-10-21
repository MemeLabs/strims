package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

func RegisterChatService(host *rpc.Host, service ChatService) {
	host.RegisterService("Chat", service)
}

type ChatService interface {
	CreateServer(
		ctx context.Context,
		req *pb.CreateChatServerRequest,
	) (*pb.CreateChatServerResponse, error)
	UpdateServer(
		ctx context.Context,
		req *pb.UpdateChatServerRequest,
	) (*pb.UpdateChatServerResponse, error)
	DeleteServer(
		ctx context.Context,
		req *pb.DeleteChatServerRequest,
	) (*pb.DeleteChatServerResponse, error)
	GetServer(
		ctx context.Context,
		req *pb.GetChatServerRequest,
	) (*pb.GetChatServerResponse, error)
	ListServers(
		ctx context.Context,
		req *pb.ListChatServersRequest,
	) (*pb.ListChatServersResponse, error)
	OpenServer(
		ctx context.Context,
		req *pb.OpenChatServerRequest,
	) (<-chan *pb.ChatServerEvent, error)
	OpenClient(
		ctx context.Context,
		req *pb.OpenChatClientRequest,
	) (<-chan *pb.ChatClientEvent, error)
	CallClient(
		ctx context.Context,
		req *pb.CallChatClientRequest,
	) (*pb.CallChatClientResponse, error)
}

type ChatClient struct {
	client *rpc.Client
}

// New ...
func NewChatClient(client *rpc.Client) *ChatClient {
	return &ChatClient{client}
}

// CreateServer ...
func (c *ChatClient) CreateServer(
	ctx context.Context,
	req *pb.CreateChatServerRequest,
	res *pb.CreateChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "Chat/CreateServer", req, res)
}

// UpdateServer ...
func (c *ChatClient) UpdateServer(
	ctx context.Context,
	req *pb.UpdateChatServerRequest,
	res *pb.UpdateChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "Chat/UpdateServer", req, res)
}

// DeleteServer ...
func (c *ChatClient) DeleteServer(
	ctx context.Context,
	req *pb.DeleteChatServerRequest,
	res *pb.DeleteChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "Chat/DeleteServer", req, res)
}

// GetServer ...
func (c *ChatClient) GetServer(
	ctx context.Context,
	req *pb.GetChatServerRequest,
	res *pb.GetChatServerResponse,
) error {
	return c.client.CallUnary(ctx, "Chat/GetServer", req, res)
}

// ListServers ...
func (c *ChatClient) ListServers(
	ctx context.Context,
	req *pb.ListChatServersRequest,
	res *pb.ListChatServersResponse,
) error {
	return c.client.CallUnary(ctx, "Chat/ListServers", req, res)
}

// OpenServer ...
func (c *ChatClient) OpenServer(
	ctx context.Context,
	req *pb.OpenChatServerRequest,
	res chan *pb.ChatServerEvent,
) error {
	return c.client.CallStreaming(ctx, "Chat/OpenServer", req, res)
}

// OpenClient ...
func (c *ChatClient) OpenClient(
	ctx context.Context,
	req *pb.OpenChatClientRequest,
	res chan *pb.ChatClientEvent,
) error {
	return c.client.CallStreaming(ctx, "Chat/OpenClient", req, res)
}

// CallClient ...
func (c *ChatClient) CallClient(
	ctx context.Context,
	req *pb.CallChatClientRequest,
	res *pb.CallChatClientResponse,
) error {
	return c.client.CallUnary(ctx, "Chat/CallClient", req, res)
}
