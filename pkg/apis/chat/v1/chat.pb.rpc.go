package chat

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterChatFrontendService ...
func RegisterChatFrontendService(host rpc.ServiceRegistry, service ChatFrontendService) {
	host.RegisterMethod("strims.chat.v1.ChatFrontend.CreateServer", service.CreateServer)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.UpdateServer", service.UpdateServer)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.DeleteServer", service.DeleteServer)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.GetServer", service.GetServer)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ListServers", service.ListServers)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.CreateEmote", service.CreateEmote)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.UpdateEmote", service.UpdateEmote)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.DeleteEmote", service.DeleteEmote)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.GetEmote", service.GetEmote)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ListEmotes", service.ListEmotes)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.CreateModifier", service.CreateModifier)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.UpdateModifier", service.UpdateModifier)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.DeleteModifier", service.DeleteModifier)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.GetModifier", service.GetModifier)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ListModifiers", service.ListModifiers)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.CreateTag", service.CreateTag)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.UpdateTag", service.UpdateTag)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.DeleteTag", service.DeleteTag)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.GetTag", service.GetTag)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ListTags", service.ListTags)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.SyncAssets", service.SyncAssets)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.OpenClient", service.OpenClient)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ClientSendMessage", service.ClientSendMessage)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.SetUIConfig", service.SetUIConfig)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.GetUIConfig", service.GetUIConfig)
}

// ChatFrontendService ...
type ChatFrontendService interface {
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
	CreateModifier(
		ctx context.Context,
		req *CreateModifierRequest,
	) (*CreateModifierResponse, error)
	UpdateModifier(
		ctx context.Context,
		req *UpdateModifierRequest,
	) (*UpdateModifierResponse, error)
	DeleteModifier(
		ctx context.Context,
		req *DeleteModifierRequest,
	) (*DeleteModifierResponse, error)
	GetModifier(
		ctx context.Context,
		req *GetModifierRequest,
	) (*GetModifierResponse, error)
	ListModifiers(
		ctx context.Context,
		req *ListModifiersRequest,
	) (*ListModifiersResponse, error)
	CreateTag(
		ctx context.Context,
		req *CreateTagRequest,
	) (*CreateTagResponse, error)
	UpdateTag(
		ctx context.Context,
		req *UpdateTagRequest,
	) (*UpdateTagResponse, error)
	DeleteTag(
		ctx context.Context,
		req *DeleteTagRequest,
	) (*DeleteTagResponse, error)
	GetTag(
		ctx context.Context,
		req *GetTagRequest,
	) (*GetTagResponse, error)
	ListTags(
		ctx context.Context,
		req *ListTagsRequest,
	) (*ListTagsResponse, error)
	SyncAssets(
		ctx context.Context,
		req *SyncAssetsRequest,
	) (*SyncAssetsResponse, error)
	OpenClient(
		ctx context.Context,
		req *OpenClientRequest,
	) (<-chan *OpenClientResponse, error)
	ClientSendMessage(
		ctx context.Context,
		req *ClientSendMessageRequest,
	) (*ClientSendMessageResponse, error)
	SetUIConfig(
		ctx context.Context,
		req *SetUIConfigRequest,
	) (*SetUIConfigResponse, error)
	GetUIConfig(
		ctx context.Context,
		req *GetUIConfigRequest,
	) (*GetUIConfigResponse, error)
}

// ChatFrontendClient ...
type ChatFrontendClient struct {
	client rpc.Caller
}

// NewChatFrontendClient ...
func NewChatFrontendClient(client rpc.Caller) *ChatFrontendClient {
	return &ChatFrontendClient{client}
}

// CreateServer ...
func (c *ChatFrontendClient) CreateServer(
	ctx context.Context,
	req *CreateServerRequest,
	res *CreateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.CreateServer", req, res)
}

// UpdateServer ...
func (c *ChatFrontendClient) UpdateServer(
	ctx context.Context,
	req *UpdateServerRequest,
	res *UpdateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.UpdateServer", req, res)
}

// DeleteServer ...
func (c *ChatFrontendClient) DeleteServer(
	ctx context.Context,
	req *DeleteServerRequest,
	res *DeleteServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.DeleteServer", req, res)
}

// GetServer ...
func (c *ChatFrontendClient) GetServer(
	ctx context.Context,
	req *GetServerRequest,
	res *GetServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.GetServer", req, res)
}

// ListServers ...
func (c *ChatFrontendClient) ListServers(
	ctx context.Context,
	req *ListServersRequest,
	res *ListServersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ListServers", req, res)
}

// CreateEmote ...
func (c *ChatFrontendClient) CreateEmote(
	ctx context.Context,
	req *CreateEmoteRequest,
	res *CreateEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.CreateEmote", req, res)
}

// UpdateEmote ...
func (c *ChatFrontendClient) UpdateEmote(
	ctx context.Context,
	req *UpdateEmoteRequest,
	res *UpdateEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.UpdateEmote", req, res)
}

// DeleteEmote ...
func (c *ChatFrontendClient) DeleteEmote(
	ctx context.Context,
	req *DeleteEmoteRequest,
	res *DeleteEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.DeleteEmote", req, res)
}

// GetEmote ...
func (c *ChatFrontendClient) GetEmote(
	ctx context.Context,
	req *GetEmoteRequest,
	res *GetEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.GetEmote", req, res)
}

// ListEmotes ...
func (c *ChatFrontendClient) ListEmotes(
	ctx context.Context,
	req *ListEmotesRequest,
	res *ListEmotesResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ListEmotes", req, res)
}

// CreateModifier ...
func (c *ChatFrontendClient) CreateModifier(
	ctx context.Context,
	req *CreateModifierRequest,
	res *CreateModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.CreateModifier", req, res)
}

// UpdateModifier ...
func (c *ChatFrontendClient) UpdateModifier(
	ctx context.Context,
	req *UpdateModifierRequest,
	res *UpdateModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.UpdateModifier", req, res)
}

// DeleteModifier ...
func (c *ChatFrontendClient) DeleteModifier(
	ctx context.Context,
	req *DeleteModifierRequest,
	res *DeleteModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.DeleteModifier", req, res)
}

// GetModifier ...
func (c *ChatFrontendClient) GetModifier(
	ctx context.Context,
	req *GetModifierRequest,
	res *GetModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.GetModifier", req, res)
}

// ListModifiers ...
func (c *ChatFrontendClient) ListModifiers(
	ctx context.Context,
	req *ListModifiersRequest,
	res *ListModifiersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ListModifiers", req, res)
}

// CreateTag ...
func (c *ChatFrontendClient) CreateTag(
	ctx context.Context,
	req *CreateTagRequest,
	res *CreateTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.CreateTag", req, res)
}

// UpdateTag ...
func (c *ChatFrontendClient) UpdateTag(
	ctx context.Context,
	req *UpdateTagRequest,
	res *UpdateTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.UpdateTag", req, res)
}

// DeleteTag ...
func (c *ChatFrontendClient) DeleteTag(
	ctx context.Context,
	req *DeleteTagRequest,
	res *DeleteTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.DeleteTag", req, res)
}

// GetTag ...
func (c *ChatFrontendClient) GetTag(
	ctx context.Context,
	req *GetTagRequest,
	res *GetTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.GetTag", req, res)
}

// ListTags ...
func (c *ChatFrontendClient) ListTags(
	ctx context.Context,
	req *ListTagsRequest,
	res *ListTagsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ListTags", req, res)
}

// SyncAssets ...
func (c *ChatFrontendClient) SyncAssets(
	ctx context.Context,
	req *SyncAssetsRequest,
	res *SyncAssetsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.SyncAssets", req, res)
}

// OpenClient ...
func (c *ChatFrontendClient) OpenClient(
	ctx context.Context,
	req *OpenClientRequest,
	res chan *OpenClientResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.chat.v1.ChatFrontend.OpenClient", req, res)
}

// ClientSendMessage ...
func (c *ChatFrontendClient) ClientSendMessage(
	ctx context.Context,
	req *ClientSendMessageRequest,
	res *ClientSendMessageResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ClientSendMessage", req, res)
}

// SetUIConfig ...
func (c *ChatFrontendClient) SetUIConfig(
	ctx context.Context,
	req *SetUIConfigRequest,
	res *SetUIConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.SetUIConfig", req, res)
}

// GetUIConfig ...
func (c *ChatFrontendClient) GetUIConfig(
	ctx context.Context,
	req *GetUIConfigRequest,
	res *GetUIConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.GetUIConfig", req, res)
}

// RegisterChatService ...
func RegisterChatService(host rpc.ServiceRegistry, service ChatService) {
	host.RegisterMethod("strims.chat.v1.Chat.SendMessage", service.SendMessage)
}

// ChatService ...
type ChatService interface {
	SendMessage(
		ctx context.Context,
		req *SendMessageRequest,
	) (*SendMessageResponse, error)
}

// ChatClient ...
type ChatClient struct {
	client rpc.Caller
}

// NewChatClient ...
func NewChatClient(client rpc.Caller) *ChatClient {
	return &ChatClient{client}
}

// SendMessage ...
func (c *ChatClient) SendMessage(
	ctx context.Context,
	req *SendMessageRequest,
	res *SendMessageResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.SendMessage", req, res)
}
