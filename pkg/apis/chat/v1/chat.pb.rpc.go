package chatv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterChatServerFrontendService ...
func RegisterChatServerFrontendService(host rpc.ServiceRegistry, service ChatServerFrontendService) {
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.CreateServer", service.CreateServer)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.UpdateServer", service.UpdateServer)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.DeleteServer", service.DeleteServer)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.GetServer", service.GetServer)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.ListServers", service.ListServers)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.CreateEmote", service.CreateEmote)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.UpdateEmote", service.UpdateEmote)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.DeleteEmote", service.DeleteEmote)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.GetEmote", service.GetEmote)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.ListEmotes", service.ListEmotes)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.CreateModifier", service.CreateModifier)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.UpdateModifier", service.UpdateModifier)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.DeleteModifier", service.DeleteModifier)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.GetModifier", service.GetModifier)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.ListModifiers", service.ListModifiers)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.CreateTag", service.CreateTag)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.UpdateTag", service.UpdateTag)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.DeleteTag", service.DeleteTag)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.GetTag", service.GetTag)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.ListTags", service.ListTags)
	host.RegisterMethod("strims.chat.v1.ChatServerFrontend.SyncAssets", service.SyncAssets)
}

// ChatServerFrontendService ...
type ChatServerFrontendService interface {
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
}

// ChatServerFrontendService ...
type UnimplementedChatServerFrontendService struct{}

func (s *UnimplementedChatServerFrontendService) CreateServer(
	ctx context.Context,
	req *CreateServerRequest,
) (*CreateServerResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) UpdateServer(
	ctx context.Context,
	req *UpdateServerRequest,
) (*UpdateServerResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) DeleteServer(
	ctx context.Context,
	req *DeleteServerRequest,
) (*DeleteServerResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) GetServer(
	ctx context.Context,
	req *GetServerRequest,
) (*GetServerResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) ListServers(
	ctx context.Context,
	req *ListServersRequest,
) (*ListServersResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) CreateEmote(
	ctx context.Context,
	req *CreateEmoteRequest,
) (*CreateEmoteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) UpdateEmote(
	ctx context.Context,
	req *UpdateEmoteRequest,
) (*UpdateEmoteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) DeleteEmote(
	ctx context.Context,
	req *DeleteEmoteRequest,
) (*DeleteEmoteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) GetEmote(
	ctx context.Context,
	req *GetEmoteRequest,
) (*GetEmoteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) ListEmotes(
	ctx context.Context,
	req *ListEmotesRequest,
) (*ListEmotesResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) CreateModifier(
	ctx context.Context,
	req *CreateModifierRequest,
) (*CreateModifierResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) UpdateModifier(
	ctx context.Context,
	req *UpdateModifierRequest,
) (*UpdateModifierResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) DeleteModifier(
	ctx context.Context,
	req *DeleteModifierRequest,
) (*DeleteModifierResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) GetModifier(
	ctx context.Context,
	req *GetModifierRequest,
) (*GetModifierResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) ListModifiers(
	ctx context.Context,
	req *ListModifiersRequest,
) (*ListModifiersResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) CreateTag(
	ctx context.Context,
	req *CreateTagRequest,
) (*CreateTagResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) UpdateTag(
	ctx context.Context,
	req *UpdateTagRequest,
) (*UpdateTagResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) DeleteTag(
	ctx context.Context,
	req *DeleteTagRequest,
) (*DeleteTagResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) GetTag(
	ctx context.Context,
	req *GetTagRequest,
) (*GetTagResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) ListTags(
	ctx context.Context,
	req *ListTagsRequest,
) (*ListTagsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatServerFrontendService) SyncAssets(
	ctx context.Context,
	req *SyncAssetsRequest,
) (*SyncAssetsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ ChatServerFrontendService = (*UnimplementedChatServerFrontendService)(nil)

// ChatServerFrontendClient ...
type ChatServerFrontendClient struct {
	client rpc.Caller
}

// NewChatServerFrontendClient ...
func NewChatServerFrontendClient(client rpc.Caller) *ChatServerFrontendClient {
	return &ChatServerFrontendClient{client}
}

// CreateServer ...
func (c *ChatServerFrontendClient) CreateServer(
	ctx context.Context,
	req *CreateServerRequest,
	res *CreateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.CreateServer", req, res)
}

// UpdateServer ...
func (c *ChatServerFrontendClient) UpdateServer(
	ctx context.Context,
	req *UpdateServerRequest,
	res *UpdateServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.UpdateServer", req, res)
}

// DeleteServer ...
func (c *ChatServerFrontendClient) DeleteServer(
	ctx context.Context,
	req *DeleteServerRequest,
	res *DeleteServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.DeleteServer", req, res)
}

// GetServer ...
func (c *ChatServerFrontendClient) GetServer(
	ctx context.Context,
	req *GetServerRequest,
	res *GetServerResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.GetServer", req, res)
}

// ListServers ...
func (c *ChatServerFrontendClient) ListServers(
	ctx context.Context,
	req *ListServersRequest,
	res *ListServersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.ListServers", req, res)
}

// CreateEmote ...
func (c *ChatServerFrontendClient) CreateEmote(
	ctx context.Context,
	req *CreateEmoteRequest,
	res *CreateEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.CreateEmote", req, res)
}

// UpdateEmote ...
func (c *ChatServerFrontendClient) UpdateEmote(
	ctx context.Context,
	req *UpdateEmoteRequest,
	res *UpdateEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.UpdateEmote", req, res)
}

// DeleteEmote ...
func (c *ChatServerFrontendClient) DeleteEmote(
	ctx context.Context,
	req *DeleteEmoteRequest,
	res *DeleteEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.DeleteEmote", req, res)
}

// GetEmote ...
func (c *ChatServerFrontendClient) GetEmote(
	ctx context.Context,
	req *GetEmoteRequest,
	res *GetEmoteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.GetEmote", req, res)
}

// ListEmotes ...
func (c *ChatServerFrontendClient) ListEmotes(
	ctx context.Context,
	req *ListEmotesRequest,
	res *ListEmotesResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.ListEmotes", req, res)
}

// CreateModifier ...
func (c *ChatServerFrontendClient) CreateModifier(
	ctx context.Context,
	req *CreateModifierRequest,
	res *CreateModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.CreateModifier", req, res)
}

// UpdateModifier ...
func (c *ChatServerFrontendClient) UpdateModifier(
	ctx context.Context,
	req *UpdateModifierRequest,
	res *UpdateModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.UpdateModifier", req, res)
}

// DeleteModifier ...
func (c *ChatServerFrontendClient) DeleteModifier(
	ctx context.Context,
	req *DeleteModifierRequest,
	res *DeleteModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.DeleteModifier", req, res)
}

// GetModifier ...
func (c *ChatServerFrontendClient) GetModifier(
	ctx context.Context,
	req *GetModifierRequest,
	res *GetModifierResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.GetModifier", req, res)
}

// ListModifiers ...
func (c *ChatServerFrontendClient) ListModifiers(
	ctx context.Context,
	req *ListModifiersRequest,
	res *ListModifiersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.ListModifiers", req, res)
}

// CreateTag ...
func (c *ChatServerFrontendClient) CreateTag(
	ctx context.Context,
	req *CreateTagRequest,
	res *CreateTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.CreateTag", req, res)
}

// UpdateTag ...
func (c *ChatServerFrontendClient) UpdateTag(
	ctx context.Context,
	req *UpdateTagRequest,
	res *UpdateTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.UpdateTag", req, res)
}

// DeleteTag ...
func (c *ChatServerFrontendClient) DeleteTag(
	ctx context.Context,
	req *DeleteTagRequest,
	res *DeleteTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.DeleteTag", req, res)
}

// GetTag ...
func (c *ChatServerFrontendClient) GetTag(
	ctx context.Context,
	req *GetTagRequest,
	res *GetTagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.GetTag", req, res)
}

// ListTags ...
func (c *ChatServerFrontendClient) ListTags(
	ctx context.Context,
	req *ListTagsRequest,
	res *ListTagsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.ListTags", req, res)
}

// SyncAssets ...
func (c *ChatServerFrontendClient) SyncAssets(
	ctx context.Context,
	req *SyncAssetsRequest,
	res *SyncAssetsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatServerFrontend.SyncAssets", req, res)
}

// RegisterChatFrontendService ...
func RegisterChatFrontendService(host rpc.ServiceRegistry, service ChatFrontendService) {
	host.RegisterMethod("strims.chat.v1.ChatFrontend.OpenClient", service.OpenClient)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ClientSendMessage", service.ClientSendMessage)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ClientMute", service.ClientMute)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ClientUnmute", service.ClientUnmute)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ClientGetMute", service.ClientGetMute)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.Whisper", service.Whisper)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.ListWhispers", service.ListWhispers)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.WatchWhispers", service.WatchWhispers)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.MarkWhispersRead", service.MarkWhispersRead)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.SetUIConfig", service.SetUIConfig)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.WatchUIConfig", service.WatchUIConfig)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.Ignore", service.Ignore)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.Unignore", service.Unignore)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.Highlight", service.Highlight)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.Unhighlight", service.Unhighlight)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.Tag", service.Tag)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.Untag", service.Untag)
	host.RegisterMethod("strims.chat.v1.ChatFrontend.GetEmoji", service.GetEmoji)
}

// ChatFrontendService ...
type ChatFrontendService interface {
	OpenClient(
		ctx context.Context,
		req *OpenClientRequest,
	) (<-chan *OpenClientResponse, error)
	ClientSendMessage(
		ctx context.Context,
		req *ClientSendMessageRequest,
	) (*ClientSendMessageResponse, error)
	ClientMute(
		ctx context.Context,
		req *ClientMuteRequest,
	) (*ClientMuteResponse, error)
	ClientUnmute(
		ctx context.Context,
		req *ClientUnmuteRequest,
	) (*ClientUnmuteResponse, error)
	ClientGetMute(
		ctx context.Context,
		req *ClientGetMuteRequest,
	) (*ClientGetMuteResponse, error)
	Whisper(
		ctx context.Context,
		req *WhisperRequest,
	) (*WhisperResponse, error)
	ListWhispers(
		ctx context.Context,
		req *ListWhispersRequest,
	) (*ListWhispersResponse, error)
	WatchWhispers(
		ctx context.Context,
		req *WatchWhispersRequest,
	) (<-chan *WatchWhispersResponse, error)
	MarkWhispersRead(
		ctx context.Context,
		req *MarkWhispersReadRequest,
	) (*MarkWhispersReadResponse, error)
	SetUIConfig(
		ctx context.Context,
		req *SetUIConfigRequest,
	) (*SetUIConfigResponse, error)
	WatchUIConfig(
		ctx context.Context,
		req *WatchUIConfigRequest,
	) (<-chan *WatchUIConfigResponse, error)
	Ignore(
		ctx context.Context,
		req *IgnoreRequest,
	) (*IgnoreResponse, error)
	Unignore(
		ctx context.Context,
		req *UnignoreRequest,
	) (*UnignoreResponse, error)
	Highlight(
		ctx context.Context,
		req *HighlightRequest,
	) (*HighlightResponse, error)
	Unhighlight(
		ctx context.Context,
		req *UnhighlightRequest,
	) (*UnhighlightResponse, error)
	Tag(
		ctx context.Context,
		req *TagRequest,
	) (*TagResponse, error)
	Untag(
		ctx context.Context,
		req *UntagRequest,
	) (*UntagResponse, error)
	GetEmoji(
		ctx context.Context,
		req *GetEmojiRequest,
	) (*GetEmojiResponse, error)
}

// ChatFrontendService ...
type UnimplementedChatFrontendService struct{}

func (s *UnimplementedChatFrontendService) OpenClient(
	ctx context.Context,
	req *OpenClientRequest,
) (<-chan *OpenClientResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) ClientSendMessage(
	ctx context.Context,
	req *ClientSendMessageRequest,
) (*ClientSendMessageResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) ClientMute(
	ctx context.Context,
	req *ClientMuteRequest,
) (*ClientMuteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) ClientUnmute(
	ctx context.Context,
	req *ClientUnmuteRequest,
) (*ClientUnmuteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) ClientGetMute(
	ctx context.Context,
	req *ClientGetMuteRequest,
) (*ClientGetMuteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) Whisper(
	ctx context.Context,
	req *WhisperRequest,
) (*WhisperResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) ListWhispers(
	ctx context.Context,
	req *ListWhispersRequest,
) (*ListWhispersResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) WatchWhispers(
	ctx context.Context,
	req *WatchWhispersRequest,
) (<-chan *WatchWhispersResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) MarkWhispersRead(
	ctx context.Context,
	req *MarkWhispersReadRequest,
) (*MarkWhispersReadResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) SetUIConfig(
	ctx context.Context,
	req *SetUIConfigRequest,
) (*SetUIConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) WatchUIConfig(
	ctx context.Context,
	req *WatchUIConfigRequest,
) (<-chan *WatchUIConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) Ignore(
	ctx context.Context,
	req *IgnoreRequest,
) (*IgnoreResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) Unignore(
	ctx context.Context,
	req *UnignoreRequest,
) (*UnignoreResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) Highlight(
	ctx context.Context,
	req *HighlightRequest,
) (*HighlightResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) Unhighlight(
	ctx context.Context,
	req *UnhighlightRequest,
) (*UnhighlightResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) Tag(
	ctx context.Context,
	req *TagRequest,
) (*TagResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) Untag(
	ctx context.Context,
	req *UntagRequest,
) (*UntagResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatFrontendService) GetEmoji(
	ctx context.Context,
	req *GetEmojiRequest,
) (*GetEmojiResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ ChatFrontendService = (*UnimplementedChatFrontendService)(nil)

// ChatFrontendClient ...
type ChatFrontendClient struct {
	client rpc.Caller
}

// NewChatFrontendClient ...
func NewChatFrontendClient(client rpc.Caller) *ChatFrontendClient {
	return &ChatFrontendClient{client}
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

// ClientMute ...
func (c *ChatFrontendClient) ClientMute(
	ctx context.Context,
	req *ClientMuteRequest,
	res *ClientMuteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ClientMute", req, res)
}

// ClientUnmute ...
func (c *ChatFrontendClient) ClientUnmute(
	ctx context.Context,
	req *ClientUnmuteRequest,
	res *ClientUnmuteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ClientUnmute", req, res)
}

// ClientGetMute ...
func (c *ChatFrontendClient) ClientGetMute(
	ctx context.Context,
	req *ClientGetMuteRequest,
	res *ClientGetMuteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ClientGetMute", req, res)
}

// Whisper ...
func (c *ChatFrontendClient) Whisper(
	ctx context.Context,
	req *WhisperRequest,
	res *WhisperResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.Whisper", req, res)
}

// ListWhispers ...
func (c *ChatFrontendClient) ListWhispers(
	ctx context.Context,
	req *ListWhispersRequest,
	res *ListWhispersResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.ListWhispers", req, res)
}

// WatchWhispers ...
func (c *ChatFrontendClient) WatchWhispers(
	ctx context.Context,
	req *WatchWhispersRequest,
	res chan *WatchWhispersResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.chat.v1.ChatFrontend.WatchWhispers", req, res)
}

// MarkWhispersRead ...
func (c *ChatFrontendClient) MarkWhispersRead(
	ctx context.Context,
	req *MarkWhispersReadRequest,
	res *MarkWhispersReadResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.MarkWhispersRead", req, res)
}

// SetUIConfig ...
func (c *ChatFrontendClient) SetUIConfig(
	ctx context.Context,
	req *SetUIConfigRequest,
	res *SetUIConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.SetUIConfig", req, res)
}

// WatchUIConfig ...
func (c *ChatFrontendClient) WatchUIConfig(
	ctx context.Context,
	req *WatchUIConfigRequest,
	res chan *WatchUIConfigResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.chat.v1.ChatFrontend.WatchUIConfig", req, res)
}

// Ignore ...
func (c *ChatFrontendClient) Ignore(
	ctx context.Context,
	req *IgnoreRequest,
	res *IgnoreResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.Ignore", req, res)
}

// Unignore ...
func (c *ChatFrontendClient) Unignore(
	ctx context.Context,
	req *UnignoreRequest,
	res *UnignoreResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.Unignore", req, res)
}

// Highlight ...
func (c *ChatFrontendClient) Highlight(
	ctx context.Context,
	req *HighlightRequest,
	res *HighlightResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.Highlight", req, res)
}

// Unhighlight ...
func (c *ChatFrontendClient) Unhighlight(
	ctx context.Context,
	req *UnhighlightRequest,
	res *UnhighlightResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.Unhighlight", req, res)
}

// Tag ...
func (c *ChatFrontendClient) Tag(
	ctx context.Context,
	req *TagRequest,
	res *TagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.Tag", req, res)
}

// Untag ...
func (c *ChatFrontendClient) Untag(
	ctx context.Context,
	req *UntagRequest,
	res *UntagResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.Untag", req, res)
}

// GetEmoji ...
func (c *ChatFrontendClient) GetEmoji(
	ctx context.Context,
	req *GetEmojiRequest,
	res *GetEmojiResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.ChatFrontend.GetEmoji", req, res)
}

// RegisterChatService ...
func RegisterChatService(host rpc.ServiceRegistry, service ChatService) {
	host.RegisterMethod("strims.chat.v1.Chat.SendMessage", service.SendMessage)
	host.RegisterMethod("strims.chat.v1.Chat.Mute", service.Mute)
	host.RegisterMethod("strims.chat.v1.Chat.Unmute", service.Unmute)
	host.RegisterMethod("strims.chat.v1.Chat.GetMute", service.GetMute)
}

// ChatService ...
type ChatService interface {
	SendMessage(
		ctx context.Context,
		req *SendMessageRequest,
	) (*SendMessageResponse, error)
	Mute(
		ctx context.Context,
		req *MuteRequest,
	) (*MuteResponse, error)
	Unmute(
		ctx context.Context,
		req *UnmuteRequest,
	) (*UnmuteResponse, error)
	GetMute(
		ctx context.Context,
		req *GetMuteRequest,
	) (*GetMuteResponse, error)
}

// ChatService ...
type UnimplementedChatService struct{}

func (s *UnimplementedChatService) SendMessage(
	ctx context.Context,
	req *SendMessageRequest,
) (*SendMessageResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatService) Mute(
	ctx context.Context,
	req *MuteRequest,
) (*MuteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatService) Unmute(
	ctx context.Context,
	req *UnmuteRequest,
) (*UnmuteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedChatService) GetMute(
	ctx context.Context,
	req *GetMuteRequest,
) (*GetMuteResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ ChatService = (*UnimplementedChatService)(nil)

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

// Mute ...
func (c *ChatClient) Mute(
	ctx context.Context,
	req *MuteRequest,
	res *MuteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.Mute", req, res)
}

// Unmute ...
func (c *ChatClient) Unmute(
	ctx context.Context,
	req *UnmuteRequest,
	res *UnmuteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.Unmute", req, res)
}

// GetMute ...
func (c *ChatClient) GetMute(
	ctx context.Context,
	req *GetMuteRequest,
	res *GetMuteResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Chat.GetMute", req, res)
}

// RegisterWhisperService ...
func RegisterWhisperService(host rpc.ServiceRegistry, service WhisperService) {
	host.RegisterMethod("strims.chat.v1.Whisper.SendMessage", service.SendMessage)
}

// WhisperService ...
type WhisperService interface {
	SendMessage(
		ctx context.Context,
		req *WhisperSendMessageRequest,
	) (*WhisperSendMessageResponse, error)
}

// WhisperService ...
type UnimplementedWhisperService struct{}

func (s *UnimplementedWhisperService) SendMessage(
	ctx context.Context,
	req *WhisperSendMessageRequest,
) (*WhisperSendMessageResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ WhisperService = (*UnimplementedWhisperService)(nil)

// WhisperClient ...
type WhisperClient struct {
	client rpc.Caller
}

// NewWhisperClient ...
func NewWhisperClient(client rpc.Caller) *WhisperClient {
	return &WhisperClient{client}
}

// SendMessage ...
func (c *WhisperClient) SendMessage(
	ctx context.Context,
	req *WhisperSendMessageRequest,
	res *WhisperSendMessageResponse,
) error {
	return c.client.CallUnary(ctx, "strims.chat.v1.Whisper.SendMessage", req, res)
}
