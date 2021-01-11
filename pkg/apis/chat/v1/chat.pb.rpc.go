package chat

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

// RegisterChatService ...
func RegisterChatService(host api.ServiceRegistry, service ChatService) {
	host.RegisterMethod(".strims.chat.v1.Chat.CreateServer", service.CreateServer)
	host.RegisterMethod(".strims.chat.v1.Chat.UpdateServer", service.UpdateServer)
	host.RegisterMethod(".strims.chat.v1.Chat.DeleteServer", service.DeleteServer)
	host.RegisterMethod(".strims.chat.v1.Chat.GetServer", service.GetServer)
	host.RegisterMethod(".strims.chat.v1.Chat.ListServers", service.ListServers)
	host.RegisterMethod(".strims.chat.v1.Chat.OpenServer", service.OpenServer)
	host.RegisterMethod(".strims.chat.v1.Chat.OpenClient", service.OpenClient)
	host.RegisterMethod(".strims.chat.v1.Chat.CallClient", service.CallClient)
}

// ChatService ...
type ChatService interface {
	CreateServer(
		ctx context.Context,
		req *CreateChatServerRequest,
	) (*CreateChatServerResponse, error)
	UpdateServer(
		ctx context.Context,
		req *UpdateChatServerRequest,
	) (*UpdateChatServerResponse, error)
	DeleteServer(
		ctx context.Context,
		req *DeleteChatServerRequest,
	) (*DeleteChatServerResponse, error)
	GetServer(
		ctx context.Context,
		req *GetChatServerRequest,
	) (*GetChatServerResponse, error)
	ListServers(
		ctx context.Context,
		req *ListChatServersRequest,
	) (*ListChatServersResponse, error)
	OpenServer(
		ctx context.Context,
		req *OpenChatServerRequest,
	) (<-chan *ChatServerEvent, error)
	OpenClient(
		ctx context.Context,
		req *OpenChatClientRequest,
	) (<-chan *ChatClientEvent, error)
	CallClient(
		ctx context.Context,
		req *CallChatClientRequest,
	) (*CallChatClientResponse, error)
}

// ChatClient ...
type ChatClient struct {
	client api.Caller
}

// NewChatClient ...
func NewChatClient(client api.Caller) *ChatClient {
	return &ChatClient{client}
}

// CreateServer ...
func (c *ChatClient) CreateServer(
	ctx context.Context,
	req *CreateChatServerRequest,
	res *CreateChatServerResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.chat.v1.Chat.CreateServer", req, res)
}

// UpdateServer ...
func (c *ChatClient) UpdateServer(
	ctx context.Context,
	req *UpdateChatServerRequest,
	res *UpdateChatServerResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.chat.v1.Chat.UpdateServer", req, res)
}

// DeleteServer ...
func (c *ChatClient) DeleteServer(
	ctx context.Context,
	req *DeleteChatServerRequest,
	res *DeleteChatServerResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.chat.v1.Chat.DeleteServer", req, res)
}

// GetServer ...
func (c *ChatClient) GetServer(
	ctx context.Context,
	req *GetChatServerRequest,
	res *GetChatServerResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.chat.v1.Chat.GetServer", req, res)
}

// ListServers ...
func (c *ChatClient) ListServers(
	ctx context.Context,
	req *ListChatServersRequest,
	res *ListChatServersResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.chat.v1.Chat.ListServers", req, res)
}

// OpenServer ...
func (c *ChatClient) OpenServer(
	ctx context.Context,
	req *OpenChatServerRequest,
	res chan *ChatServerEvent,
) error {
	return c.client.CallStreaming(ctx, ".strims.chat.v1.Chat.OpenServer", req, res)
}

// OpenClient ...
func (c *ChatClient) OpenClient(
	ctx context.Context,
	req *OpenChatClientRequest,
	res chan *ChatClientEvent,
) error {
	return c.client.CallStreaming(ctx, ".strims.chat.v1.Chat.OpenClient", req, res)
}

// CallClient ...
func (c *ChatClient) CallClient(
	ctx context.Context,
	req *CallChatClientRequest,
	res *CallChatClientResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.chat.v1.Chat.CallClient", req, res)
}
