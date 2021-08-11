package chat

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterChatService ...
func RegisterChatService(host rpc.ServiceRegistry, service ChatService) {
	host.RegisterMethod("strims.chat.v1.Chat.CreateServer", service.CreateServer)
	host.RegisterMethod("strims.chat.v1.Chat.UpdateServer", service.UpdateServer)
	host.RegisterMethod("strims.chat.v1.Chat.DeleteServer", service.DeleteServer)
	host.RegisterMethod("strims.chat.v1.Chat.GetServer", service.GetServer)
	host.RegisterMethod("strims.chat.v1.Chat.ListServers", service.ListServers)
	host.RegisterMethod("strims.chat.v1.Chat.CreateEmote", service.CreateEmote)
	host.RegisterMethod("strims.chat.v1.Chat.UpdateEmote", service.UpdateEmote)
	host.RegisterMethod("strims.chat.v1.Chat.DeleteEmote", service.DeleteEmote)
	host.RegisterMethod("strims.chat.v1.Chat.GetEmote", service.GetEmote)
	host.RegisterMethod("strims.chat.v1.Chat.ListEmotes", service.ListEmotes)
	host.RegisterMethod("strims.chat.v1.Chat.OpenServer", service.OpenServer)
	host.RegisterMethod("strims.chat.v1.Chat.OpenClient", service.OpenClient)
	host.RegisterMethod("strims.chat.v1.Chat.CallClient", service.CallClient)
}

// ChatService ...
type ChatService interface {
	CreateServer(
		ctx context.Context,
		req *CreateServerRequest,
	) (*CreateServerResponse, error)
	UpdateServer(
		ctx context.Context,
		req *UpdateServerRequest,
	) (*UpdateServerResponse, error)
	DeleteServer(
		ctx context.Context,
		req *DeleteServerRequest,
	) (*DeleteServerResponse, error)
	GetServer(
		ctx context.Context,
		req *GetServerRequest,
	) (*GetServerResponse, error)
	ListServers(
		ctx context.Context,
		req *ListServersRequest,
	) (*ListServersResponse, error)
	CreateEmote(
		ctx context.Context,
		req *CreateEmoteRequest,
	) (*CreateEmoteResponse, error)
	UpdateEmote(
		ctx context.Context,
		req *UpdateEmoteRequest,
	) (*UpdateEmoteResponse, error)
	DeleteEmote(
		ctx context.Context,
		req *DeleteEmoteRequest,
	) (*DeleteEmoteResponse, error)
	GetEmote(
		ctx context.Context,
		req *GetEmoteRequest,
	) (*GetEmoteResponse, error)
	ListEmotes(
		ctx context.Context,
		req *ListEmotesRequest,
	) (*ListEmotesResponse, error)
	OpenServer(
		ctx context.Context,
		req *OpenServerRequest,
	) (<-chan *ServerEvent, error)
	OpenClient(
		ctx context.Context,
		req *OpenClientRequest,
	) (<-chan *ClientEvent, error)
	CallClient(
		ctx context.Context,
		req *CallClientRequest,
	) (*CallClientResponse, error)
}

// ChatClient ...
type ChatClient struct {
	client rpc.Caller
}

// NewChatClient ...
func NewChatClient(client rpc.Caller) *ChatClient {
	return &ChatClient{client}
}

// CreateServer ...
func (c *ChatClient) CreateServer(
	ctx context.Context,
	req *CreateServerRequest,
	res *CreateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.CreateServer", req, res)
}

// UpdateServer ...
func (c *ChatClient) UpdateServer(
	ctx context.Context,
	req *UpdateServerRequest,
	res *UpdateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.UpdateServer", req, res)
}

// DeleteServer ...
func (c *ChatClient) DeleteServer(
	ctx context.Context,
	req *DeleteServerRequest,
	res *DeleteServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.DeleteServer", req, res)
}

// GetServer ...
func (c *ChatClient) GetServer(
	ctx context.Context,
	req *GetServerRequest,
	res *GetServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.GetServer", req, res)
}

// ListServers ...
func (c *ChatClient) ListServers(
	ctx context.Context,
	req *ListServersRequest,
	res *ListServersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.ListServers", req, res)
}

// CreateEmote ...
func (c *ChatClient) CreateEmote(
	ctx context.Context,
	req *CreateEmoteRequest,
	res *CreateEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.CreateEmote", req, res)
}

// UpdateEmote ...
func (c *ChatClient) UpdateEmote(
	ctx context.Context,
	req *UpdateEmoteRequest,
	res *UpdateEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.UpdateEmote", req, res)
}

// DeleteEmote ...
func (c *ChatClient) DeleteEmote(
	ctx context.Context,
	req *DeleteEmoteRequest,
	res *DeleteEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.DeleteEmote", req, res)
}

// GetEmote ...
func (c *ChatClient) GetEmote(
	ctx context.Context,
	req *GetEmoteRequest,
	res *GetEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.GetEmote", req, res)
}

// ListEmotes ...
func (c *ChatClient) ListEmotes(
	ctx context.Context,
	req *ListEmotesRequest,
	res *ListEmotesResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.ListEmotes", req, res)
}

// OpenServer ...
func (c *ChatClient) OpenServer(
	ctx context.Context,
	req *OpenServerRequest,
	res chan *ServerEvent,
) error {
	return c.client.CallStreaming(ctx, "strims.chat.v1.Chat.OpenServer", req, res)
}

// OpenClient ...
func (c *ChatClient) OpenClient(
	ctx context.Context,
	req *OpenClientRequest,
	res chan *ClientEvent,
) error {
	return c.client.CallStreaming(ctx, "strims.chat.v1.Chat.OpenClient", req, res)
}

// CallClient ...
func (c *ChatClient) CallClient(
	ctx context.Context,
	req *CallClientRequest,
	res *CallClientResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.CallClient", req, res)
}
