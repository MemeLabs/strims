// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/app"
	"github.com/MemeLabs/strims/internal/dao"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/chanutil"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"golang.org/x/exp/slices"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		svc := &chatService{
			app:    params.App,
			store:  params.Store,
			logger: params.Logger,
		}
		chatv1.RegisterChatServerFrontendService(server, svc)
		chatv1.RegisterChatFrontendService(server, svc)
	})
}

// chatService ...
type chatService struct {
	app    app.Control
	store  *dao.ProfileStore
	logger *zap.Logger
}

// CreateServer ...
func (s *chatService) CreateServer(ctx context.Context, req *chatv1.CreateServerRequest) (*chatv1.CreateServerResponse, error) {
	server, err := dao.NewChatServer(s.store, req.NetworkKey, req.Room)
	if err != nil {
		return nil, err
	}
	if err := dao.ChatServers.Insert(s.store, server); err != nil {
		return nil, err
	}
	return &chatv1.CreateServerResponse{Server: server}, nil
}

// UpdateServer ...
func (s *chatService) UpdateServer(ctx context.Context, req *chatv1.UpdateServerRequest) (*chatv1.UpdateServerResponse, error) {
	server, err := dao.ChatServers.Transform(s.store, req.Id, func(v *chatv1.Server) error {
		v.NetworkKey = req.NetworkKey
		v.Room = req.Room
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &chatv1.UpdateServerResponse{Server: server}, nil
}

// DeleteServer ...
func (s *chatService) DeleteServer(ctx context.Context, req *chatv1.DeleteServerRequest) (*chatv1.DeleteServerResponse, error) {
	if err := dao.ChatServers.Delete(s.store, req.Id); err != nil {
		return nil, err
	}
	return &chatv1.DeleteServerResponse{}, nil
}

// GetServer ...
func (s *chatService) GetServer(ctx context.Context, req *chatv1.GetServerRequest) (*chatv1.GetServerResponse, error) {
	server, err := dao.ChatServers.Get(s.store, req.Id)
	if err != nil {
		return nil, err
	}
	return &chatv1.GetServerResponse{Server: server}, nil
}

// ListServers ...
func (s *chatService) ListServers(ctx context.Context, req *chatv1.ListServersRequest) (*chatv1.ListServersResponse, error) {
	servers, err := dao.ChatServers.GetAll(s.store)
	if err != nil {
		return nil, err
	}
	return &chatv1.ListServersResponse{Servers: servers}, nil
}

// CreateEmote ...
func (s *chatService) CreateEmote(ctx context.Context, req *chatv1.CreateEmoteRequest) (*chatv1.CreateEmoteResponse, error) {
	emote, err := dao.NewChatEmote(
		s.store,
		req.ServerId,
		req.Name,
		req.Images,
		req.Effects,
		req.Contributor,
	)
	if err != nil {
		return nil, err
	}
	if err := dao.ChatEmotes.Insert(s.store, emote); err != nil {
		return nil, err
	}
	return &chatv1.CreateEmoteResponse{Emote: emote}, nil
}

// UpdateEmote ...
func (s *chatService) UpdateEmote(ctx context.Context, req *chatv1.UpdateEmoteRequest) (*chatv1.UpdateEmoteResponse, error) {
	emote, err := dao.ChatEmotes.Transform(s.store, req.Id, func(v *chatv1.Emote) error {
		v.Name = req.Name
		v.Images = req.Images
		v.Contributor = req.Contributor
		v.Effects = req.Effects
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &chatv1.UpdateEmoteResponse{Emote: emote}, nil
}

// DeleteEmote ...
func (s *chatService) DeleteEmote(ctx context.Context, req *chatv1.DeleteEmoteRequest) (*chatv1.DeleteEmoteResponse, error) {
	if err := dao.ChatEmotes.Delete(s.store, req.Id); err != nil {
		return nil, err
	}
	return &chatv1.DeleteEmoteResponse{}, nil
}

// GetEmote ...
func (s *chatService) GetEmote(ctx context.Context, req *chatv1.GetEmoteRequest) (*chatv1.GetEmoteResponse, error) {
	emote, err := dao.ChatEmotes.Get(s.store, req.Id)
	if err != nil {
		return nil, err
	}
	return &chatv1.GetEmoteResponse{Emote: emote}, nil
}

// ListEmotes ...
func (s *chatService) ListEmotes(ctx context.Context, req *chatv1.ListEmotesRequest) (*chatv1.ListEmotesResponse, error) {
	emotes, err := dao.GetChatEmotesByServerID(s.store, req.ServerId)
	if err != nil {
		return nil, err
	}
	return &chatv1.ListEmotesResponse{Emotes: emotes}, nil
}

// CreateModifier ...
func (s *chatService) CreateModifier(ctx context.Context, req *chatv1.CreateModifierRequest) (*chatv1.CreateModifierResponse, error) {
	modifier, err := dao.NewChatModifier(
		s.store,
		req.ServerId,
		req.Name,
		req.Priority,
		req.Internal,
	)
	if err != nil {
		return nil, err
	}
	if err := dao.ChatModifiers.Insert(s.store, modifier); err != nil {
		return nil, err
	}
	return &chatv1.CreateModifierResponse{Modifier: modifier}, nil
}

// UpdateModifier ...
func (s *chatService) UpdateModifier(ctx context.Context, req *chatv1.UpdateModifierRequest) (*chatv1.UpdateModifierResponse, error) {
	modifier, err := dao.ChatModifiers.Transform(s.store, req.Id, func(v *chatv1.Modifier) error {
		v.Name = req.Name
		v.Priority = req.Priority
		v.Internal = req.Internal
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &chatv1.UpdateModifierResponse{Modifier: modifier}, nil
}

// DeleteModifier ...
func (s *chatService) DeleteModifier(ctx context.Context, req *chatv1.DeleteModifierRequest) (*chatv1.DeleteModifierResponse, error) {
	if err := dao.ChatModifiers.Delete(s.store, req.Id); err != nil {
		return nil, err
	}
	return &chatv1.DeleteModifierResponse{}, nil
}

// GetModifier ...
func (s *chatService) GetModifier(ctx context.Context, req *chatv1.GetModifierRequest) (*chatv1.GetModifierResponse, error) {
	emote, err := dao.ChatModifiers.Get(s.store, req.Id)
	if err != nil {
		return nil, err
	}
	return &chatv1.GetModifierResponse{Modifier: emote}, nil
}

// ListModifiers ...
func (s *chatService) ListModifiers(ctx context.Context, req *chatv1.ListModifiersRequest) (*chatv1.ListModifiersResponse, error) {
	modifiers, err := dao.GetChatModifiersByServerID(s.store, req.ServerId)
	if err != nil {
		return nil, err
	}
	return &chatv1.ListModifiersResponse{Modifiers: modifiers}, nil
}

// CreateTag ...
func (s *chatService) CreateTag(ctx context.Context, req *chatv1.CreateTagRequest) (*chatv1.CreateTagResponse, error) {
	tag, err := dao.NewChatTag(
		s.store,
		req.ServerId,
		req.Name,
		req.Color,
		req.Sensitive,
	)
	if err != nil {
		return nil, err
	}
	if err := dao.ChatTags.Insert(s.store, tag); err != nil {
		return nil, err
	}
	return &chatv1.CreateTagResponse{Tag: tag}, nil
}

// UpdateTag ...
func (s *chatService) UpdateTag(ctx context.Context, req *chatv1.UpdateTagRequest) (*chatv1.UpdateTagResponse, error) {
	tag, err := dao.ChatTags.Transform(s.store, req.Id, func(v *chatv1.Tag) error {
		v.Name = req.Name
		v.Color = req.Color
		v.Sensitive = req.Sensitive
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &chatv1.UpdateTagResponse{Tag: tag}, nil
}

// DeleteTag ...
func (s *chatService) DeleteTag(ctx context.Context, req *chatv1.DeleteTagRequest) (*chatv1.DeleteTagResponse, error) {
	if err := dao.ChatTags.Delete(s.store, req.Id); err != nil {
		return nil, err
	}
	return &chatv1.DeleteTagResponse{}, nil
}

// GetTag ...
func (s *chatService) GetTag(ctx context.Context, req *chatv1.GetTagRequest) (*chatv1.GetTagResponse, error) {
	tag, err := dao.ChatTags.Get(s.store, req.Id)
	if err != nil {
		return nil, err
	}
	return &chatv1.GetTagResponse{Tag: tag}, nil
}

// ListTags ...
func (s *chatService) ListTags(ctx context.Context, req *chatv1.ListTagsRequest) (*chatv1.ListTagsResponse, error) {
	tags, err := dao.GetChatTagsByServerID(s.store, req.ServerId)
	if err != nil {
		return nil, err
	}
	return &chatv1.ListTagsResponse{Tags: tags}, nil
}

// SyncAssets ...
func (s *chatService) SyncAssets(ctx context.Context, req *chatv1.SyncAssetsRequest) (*chatv1.SyncAssetsResponse, error) {
	err := s.app.Chat().SyncAssets(req.ServerId, req.ForceUnifiedUpdate)
	if err != nil {
		return nil, err
	}
	return &chatv1.SyncAssetsResponse{}, nil
}

// OpenClient ...
func (s *chatService) OpenClient(ctx context.Context, req *chatv1.OpenClientRequest) (<-chan *chatv1.OpenClientResponse, error) {
	ch := make(chan *chatv1.OpenClientResponse)

	go func() {
		events, assets, err := s.app.Chat().ReadServer(ctx, req.NetworkKey, req.ServerKey)
		if err != nil {
			close(ch)
			return
		}

		ch <- &chatv1.OpenClientResponse{
			Body: &chatv1.OpenClientResponse_Open_{
				Open: &chatv1.OpenClientResponse_Open{},
			},
		}

		for {
			select {
			case e, ok := <-events:
				if !ok {
					return
				}

				ch <- &chatv1.OpenClientResponse{
					Body: &chatv1.OpenClientResponse_ServerEvents_{
						ServerEvents: &chatv1.OpenClientResponse_ServerEvents{
							Events: chanutil.AppendAll([]*chatv1.ServerEvent{e}, events),
						},
					},
				}
			case b, ok := <-assets:
				if !ok {
					return
				}

				ch <- &chatv1.OpenClientResponse{
					Body: &chatv1.OpenClientResponse_AssetBundle{
						AssetBundle: b,
					},
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	return ch, nil
}

// ClientSendMessage ...
func (s *chatService) ClientSendMessage(ctx context.Context, req *chatv1.ClientSendMessageRequest) (*chatv1.ClientSendMessageResponse, error) {
	if err := s.app.Chat().SendMessage(ctx, req.NetworkKey, req.ServerKey, req.Body); err != nil {
		return nil, err
	}
	return &chatv1.ClientSendMessageResponse{}, nil
}

// ClientMute ...
func (s *chatService) ClientMute(ctx context.Context, req *chatv1.ClientMuteRequest) (*chatv1.ClientMuteResponse, error) {
	duration, err := time.ParseDuration(req.Duration)
	if err != nil {
		return nil, fmt.Errorf("parsing duration failed: %w", err)
	}

	cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
	if err != nil {
		return nil, fmt.Errorf("finding peer cert failed: %w", err)
	}

	if err := s.app.Chat().Mute(ctx, req.NetworkKey, req.ServerKey, cert.Key, duration, req.Message); err != nil {
		return nil, err
	}
	return &chatv1.ClientMuteResponse{}, nil
}

// ClientUnmute ...
func (s *chatService) ClientUnmute(ctx context.Context, req *chatv1.ClientUnmuteRequest) (*chatv1.ClientUnmuteResponse, error) {
	cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
	if err != nil {
		return nil, fmt.Errorf("finding peer cert failed: %w", err)
	}

	if err := s.app.Chat().Unmute(ctx, req.NetworkKey, req.ServerKey, cert.Key); err != nil {
		return nil, err
	}
	return &chatv1.ClientUnmuteResponse{}, nil
}

// ClientGetMute ...
func (s *chatService) ClientGetMute(ctx context.Context, req *chatv1.ClientGetMuteRequest) (*chatv1.ClientGetMuteResponse, error) {
	res, err := s.app.Chat().GetMute(ctx, req.NetworkKey, req.ServerKey)
	if err != nil {
		return nil, err
	}
	return &chatv1.ClientGetMuteResponse{
		EndTime: res.EndTime,
		Message: res.Message,
	}, nil
}

// Whisper ...
func (s *chatService) Whisper(ctx context.Context, req *chatv1.WhisperRequest) (*chatv1.WhisperResponse, error) {
	cert, ok := s.app.Network().Certificate(req.NetworkKey)
	if !ok {
		return nil, errors.New("no certificate found for network key")
	}

	var peerCert *certificate.Certificate
	var err error
	if req.PeerKey != nil {
		peerCert, err = s.app.Network().CA().FindByKey(ctx, req.NetworkKey, req.PeerKey)
	} else if req.Alias != "" {
		peerCert, err = s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
	} else {
		return nil, errors.New("whisper subject undefined")
	}
	if err != nil {
		return nil, fmt.Errorf("finding peer cert failed: %w", err)
	}

	record, err := dao.NewChatWhisperRecord(
		s.store,
		req.NetworkKey,
		req.ServerKey,
		peerCert.Key,
		cert,
		req.Body,
	)
	if err != nil {
		return nil, err
	}

	err = s.store.Update(func(tx kv.RWTx) error {
		thread, err := dao.GetChatWhisperThreadByPeerKey(tx, peerCert.Key)
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}
		if thread == nil {
			thread, err = dao.NewChatWhisperThread(s.store, peerCert)
			if err != nil {
				return err
			}
		}

		thread.Alias = peerCert.Subject
		thread.LastMessageTime = timeutil.Now().UnixNano() / int64(timeutil.Precision)
		thread.LastMessageId = record.Id

		record.ThreadId = thread.Id
		if err := dao.ChatWhisperRecords.Insert(tx, record); err != nil {
			return err
		}
		return dao.ChatWhisperThreads.Upsert(tx, thread)
	})
	if err != nil {
		return nil, err
	}
	return &chatv1.WhisperResponse{}, nil
}

func (s *chatService) ListWhispers(ctx context.Context, req *chatv1.ListWhispersRequest) (*chatv1.ListWhispersResponse, error) {
	thread, err := dao.GetChatWhisperThreadByPeerKey(s.store, req.PeerKey)
	if err != nil {
		return nil, err
	}
	whispers, err := dao.GetChatWhisperRecordsByPeerKey(s.store, req.PeerKey)
	if err != nil {
		return nil, err
	}
	return &chatv1.ListWhispersResponse{
		Thread:   thread,
		Whispers: whispers,
	}, nil
}

func (s *chatService) WatchWhispers(ctx context.Context, req *chatv1.WatchWhispersRequest) (<-chan *chatv1.WatchWhispersResponse, error) {
	ch := make(chan *chatv1.WatchWhispersResponse, 1)

	go func() {
		defer close(ch)

		events, done := s.app.Events().Events()
		defer done()

		threads, err := dao.ChatWhisperThreads.GetAll(s.store)
		if err != nil {
			s.logger.Debug("error loading whisper threads", zap.Error(err))
			return
		}

		for _, t := range threads {
			ch <- &chatv1.WatchWhispersResponse{
				Body: &chatv1.WatchWhispersResponse_ThreadUpdate{
					ThreadUpdate: t,
				},
			}
		}

		for {
			select {
			case e := <-events:
				switch e := e.(type) {
				case *chatv1.WhisperThreadChangeEvent:
					ch <- &chatv1.WatchWhispersResponse{
						PeerKey: e.WhisperThread.PeerKey,
						Body: &chatv1.WatchWhispersResponse_ThreadUpdate{
							ThreadUpdate: e.WhisperThread,
						},
					}
				case *chatv1.WhisperRecordChangeEvent:
					ch <- &chatv1.WatchWhispersResponse{
						PeerKey: e.WhisperRecord.PeerKey,
						Body: &chatv1.WatchWhispersResponse_WhisperUpdate{
							WhisperUpdate: e.WhisperRecord,
						},
					}
				case *chatv1.WhisperRecordDeleteEvent:
					ch <- &chatv1.WatchWhispersResponse{
						PeerKey: e.WhisperRecord.PeerKey,
						Body: &chatv1.WatchWhispersResponse_WhisperDelete_{
							WhisperDelete: &chatv1.WatchWhispersResponse_WhisperDelete{
								RecordId: e.WhisperRecord.Id,
								ThreadId: e.WhisperRecord.ThreadId,
							},
						},
					}
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	return ch, nil
}

func (s *chatService) MarkWhispersRead(ctx context.Context, req *chatv1.MarkWhispersReadRequest) (*chatv1.MarkWhispersReadResponse, error) {
	_, err := dao.ChatWhisperThreads.Transform(s.store, req.ThreadId, func(p *chatv1.WhisperThread) error {
		p.UnreadCount = 0
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &chatv1.MarkWhispersReadResponse{}, nil
}

// SetUIConfig ...
func (s *chatService) SetUIConfig(ctx context.Context, req *chatv1.SetUIConfigRequest) (*chatv1.SetUIConfigResponse, error) {
	err := dao.ChatUIConfig.Set(s.store, req.UiConfig)
	if err != nil {
		return nil, err
	}
	return &chatv1.SetUIConfigResponse{}, nil
}

// WatchUIConfig ...
func (s *chatService) WatchUIConfig(ctx context.Context, req *chatv1.WatchUIConfigRequest) (<-chan *chatv1.WatchUIConfigResponse, error) {
	ch := make(chan *chatv1.WatchUIConfigResponse, 1)

	c, err := dao.ChatUIConfig.Get(s.store)
	if err == nil {
		ch <- &chatv1.WatchUIConfigResponse{UiConfig: c}
	} else if !errors.Is(err, kv.ErrRecordNotFound) {
		return nil, err
	}

	go func() {
		events, done := s.app.Events().Events()
		defer done()

		for {
			select {
			case e := <-events:
				switch e := e.(type) {
				case *chatv1.UIConfigChangeEvent:
					ch <- &chatv1.WatchUIConfigResponse{UiConfig: e.UiConfig}
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	return ch, nil
}

// Ignore ...
func (s *chatService) Ignore(ctx context.Context, req *chatv1.IgnoreRequest) (*chatv1.IgnoreResponse, error) {
	var deadline int64
	if req.Duration != "" {
		duration, err := time.ParseDuration(req.Duration)
		if err != nil {
			return nil, fmt.Errorf("parsing duration failed: %w", err)
		}
		deadline = timeutil.Now().Add(duration).Unix()
	}

	cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
	if err != nil {
		return nil, fmt.Errorf("finding peer cert failed: %w", err)
	}

	_, err = dao.ChatUIConfig.Transform(s.store, func(p *chatv1.UIConfig) error {
		if i := slices.IndexFunc(p.Ignores, peerKeyFilter[*chatv1.UIConfig_Ignore](cert.Key)); i != -1 {
			p.Ignores = slices.Delete(p.Ignores, i, i+1)
		}
		p.Ignores = append(p.Ignores, &chatv1.UIConfig_Ignore{
			Alias:    cert.Subject,
			PeerKey:  cert.Key,
			Deadline: deadline,
		})
		return nil
	})
	return &chatv1.IgnoreResponse{}, err
}

// Unignore ...
func (s *chatService) Unignore(ctx context.Context, req *chatv1.UnignoreRequest) (*chatv1.UnignoreResponse, error) {
	peerKey := req.PeerKey
	if len(peerKey) == 0 {
		cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
		if err != nil {
			return nil, fmt.Errorf("finding peer cert failed: %w", err)
		}
		peerKey = cert.Key
	}

	_, err := dao.ChatUIConfig.Transform(s.store, func(p *chatv1.UIConfig) error {
		if i := slices.IndexFunc(p.Ignores, peerKeyFilter[*chatv1.UIConfig_Ignore](peerKey)); i != -1 {
			p.Ignores = slices.Delete(p.Ignores, i, i+1)
		}
		return nil
	})
	return &chatv1.UnignoreResponse{}, err
}

// Highlight ...
func (s *chatService) Highlight(ctx context.Context, req *chatv1.HighlightRequest) (*chatv1.HighlightResponse, error) {
	cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
	if err != nil {
		return nil, fmt.Errorf("finding peer cert failed: %w", err)
	}

	_, err = dao.ChatUIConfig.Transform(s.store, func(p *chatv1.UIConfig) error {
		if i := slices.IndexFunc(p.Highlights, peerKeyFilter[*chatv1.UIConfig_Highlight](cert.Key)); i != -1 {
			p.Highlights = slices.Delete(p.Highlights, i, i+1)
		}
		p.Highlights = append(p.Highlights, &chatv1.UIConfig_Highlight{
			Alias:   cert.Subject,
			PeerKey: cert.Key,
		})
		return nil
	})
	return &chatv1.HighlightResponse{}, err
}

// Unhighlight ...
func (s *chatService) Unhighlight(ctx context.Context, req *chatv1.UnhighlightRequest) (*chatv1.UnhighlightResponse, error) {
	peerKey := req.PeerKey
	if len(peerKey) == 0 {
		cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
		if err != nil {
			return nil, fmt.Errorf("finding peer cert failed: %w", err)
		}
		peerKey = cert.Key
	}

	_, err := dao.ChatUIConfig.Transform(s.store, func(p *chatv1.UIConfig) error {
		if i := slices.IndexFunc(p.Highlights, peerKeyFilter[*chatv1.UIConfig_Highlight](peerKey)); i != -1 {
			p.Highlights = slices.Delete(p.Highlights, i, i+1)
		}
		return nil
	})
	return &chatv1.UnhighlightResponse{}, err
}

// Tag ...
func (s *chatService) Tag(ctx context.Context, req *chatv1.TagRequest) (*chatv1.TagResponse, error) {
	cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
	if err != nil {
		return nil, fmt.Errorf("finding peer cert failed: %w", err)
	}

	_, err = dao.ChatUIConfig.Transform(s.store, func(p *chatv1.UIConfig) error {
		if i := slices.IndexFunc(p.Tags, peerKeyFilter[*chatv1.UIConfig_Tag](cert.Key)); i != -1 {
			p.Tags = slices.Delete(p.Tags, i, i+1)
		}
		p.Tags = append(p.Tags, &chatv1.UIConfig_Tag{
			Alias:   cert.Subject,
			PeerKey: cert.Key,
			Color:   req.Color,
		})
		return nil
	})
	return &chatv1.TagResponse{}, err
}

type peerKeyGetter interface {
	GetPeerKey() []byte
}

func peerKeyFilter[T peerKeyGetter](peerKey []byte) func(e T) bool {
	return func(e T) bool {
		return bytes.Equal(peerKey, e.GetPeerKey())
	}
}

// Untag ...
func (s *chatService) Untag(ctx context.Context, req *chatv1.UntagRequest) (*chatv1.UntagResponse, error) {
	peerKey := req.PeerKey
	if len(peerKey) == 0 {
		cert, err := s.app.Network().CA().FindBySubject(ctx, req.NetworkKey, req.Alias)
		if err != nil {
			return nil, fmt.Errorf("finding peer cert failed: %w", err)
		}
		peerKey = cert.Key
	}

	_, err := dao.ChatUIConfig.Transform(s.store, func(p *chatv1.UIConfig) error {
		if i := slices.IndexFunc(p.Tags, peerKeyFilter[*chatv1.UIConfig_Tag](peerKey)); i != -1 {
			p.Tags = slices.Delete(p.Tags, i, i+1)
		}
		return nil
	})
	return &chatv1.UntagResponse{}, err
}
