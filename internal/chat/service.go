// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	"github.com/MemeLabs/strims/internal/network/dialer"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/debug"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"golang.org/x/exp/slices"
)

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

const broadcastInterval = 15 * time.Second

func newChatService(
	logger *zap.Logger,
	ew *protoutil.ChunkStreamWriter,
	observers *event.Observers,
	store dao.Store,
	config *chatv1.Server,
) *chatService {
	return &chatService{
		logger:          logger,
		eventWriter:     ew,
		observers:       observers,
		store:           store,
		config:          syncutil.NewPointer(config),
		broadcastTicker: time.NewTicker(broadcastInterval),
		entityExtractor: newEntityExtractor(),
		combos:          newComboTransformer(),
		profileCache:    dao.NewChatProfileCache(store, nil),
	}
}

type chatService struct {
	logger            *zap.Logger
	eventWriter       *protoutil.ChunkStreamWriter
	observers         *event.Observers
	store             dao.Store
	config            atomic.Pointer[chatv1.Server]
	listingID         uint64
	broadcastTicker   *time.Ticker
	lastBroadcastTime timeutil.Time
	lock              sync.Mutex
	entityExtractor   *entityExtractor
	combos            *comboTransformer
	profileCache      dao.ChatProfileCache
}

func (d *chatService) Run(ctx context.Context) error {
	defer d.Close()

	events, done := d.observers.Events()
	defer done()

	for {
		select {
		case now := <-d.broadcastTicker.C:
			if err := d.broadcast(timeutil.NewFromTime(now)); err != nil {
				return err
			}
		case e := <-events:
			if e, ok := e.(event.DirectoryEvent); ok {
				d.handleDirectoryEvent(e)
			}
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

func (d *chatService) Close() {
	d.profileCache.Close()
	d.broadcastTicker.Stop()
}

func (d *chatService) SyncConfig(config *chatv1.Server) {
	d.config.Swap(config)
}

func (d *chatService) SetListingID(id uint64) {
	d.listingID = id
}

func (d *chatService) Sync(config *chatv1.Server, emotes []*chatv1.Emote, modifiers []*chatv1.Modifier, tags []*chatv1.Tag) error {
	d.config.Swap(config)

	var emoteNames, modifierNames, tagNames [][]rune
	var internalModifiers []*chatv1.Modifier
	for _, emote := range emotes {
		emoteNames = append(emoteNames, []rune(emote.Name))
	}
	for _, modifier := range modifiers {
		if modifier.Internal {
			internalModifiers = append(internalModifiers, modifier)
		} else {
			modifierNames = append(modifierNames, []rune(modifier.Name))
		}
	}
	for _, tag := range tags {
		tagNames = append(tagNames, []rune(tag.Name))
	}

	d.entityExtractor.ParserContext().Emotes.Replace(emoteNames)
	d.entityExtractor.ParserContext().EmoteModifiers.Replace(modifierNames)
	d.entityExtractor.ParserContext().Tags.Replace(tagNames)
	d.entityExtractor.SetInternalModifiers(internalModifiers)

	return nil
}

func (d *chatService) broadcast(now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	var events []*networkv1directory.Event

	if events != nil {
		err := d.eventWriter.Write(&networkv1directory.EventBroadcast{
			Events: events,
		})
		if err != nil {
			return err
		}
	}

	d.lastBroadcastTime = now
	return nil
}

func (d *chatService) handleDirectoryEvent(e event.DirectoryEvent) {
	if bytes.Equal(e.NetworkKey, d.config.Load().NetworkKey) {
		for _, e := range e.Broadcast.Events {
			if e := e.GetUserPresenceChange(); e != nil {
				if e.Online && slices.Contains(e.ListingIds, d.listingID) {
					d.entityExtractor.ParserContext().Nicks.InsertWithMeta([]rune(e.Alias), e.PeerKey)
				} else {
					d.entityExtractor.ParserContext().Nicks.Remove([]rune(e.Alias))
				}
			}
		}
	}
}

func (d *chatService) getOrInsertProfile(peerKey []byte, alias string) (p *chatv1.Profile, err error) {
	p, _, err = d.profileCache.ByPeerKey.GetOrInsert(
		dao.FormatChatProfilePeerKey(d.config.Load().Id, peerKey),
		func() (*chatv1.Profile, error) {
			return dao.NewChatProfile(d.store, d.config.Load().Id, peerKey, alias)
		},
	)
	return
}

func (d *chatService) isModerator(peerKey []byte) bool {
	for _, k := range d.config.Load().AdminPeerKeys {
		if bytes.Equal(k, peerKey) {
			return true
		}
	}
	return false
}

func (d *chatService) SendMessage(ctx context.Context, req *chatv1.SendMessageRequest) (*chatv1.SendMessageResponse, error) {
	peerCert := dialer.VPNCertificate(ctx).GetParent()
	p, err := d.getOrInsertProfile(peerCert.Key, peerCert.Subject)
	if err != nil {
		return nil, fmt.Errorf("loading profile failed: %w", err)
	}

	muteDeadline := timeutil.Unix(p.MuteDeadline, 0)
	if muteDeadline.After(timeutil.Now()) {
		return nil, fmt.Errorf("cannot send mesasges while muted. mute expires: %s", muteDeadline)
	}

	m := &chatv1.Message{
		ServerTime: time.Now().UnixNano() / int64(time.Millisecond),
		PeerKey:    peerCert.Key,
		Nick:       peerCert.Subject,
		Body:       req.Body,
		Entities:   d.entityExtractor.Extract(req.Body),
	}

	if err := d.combos.Transform(m); err != nil {
		return nil, err
	}

	err = d.eventWriter.Write(&chatv1.ServerEvent{
		Body: &chatv1.ServerEvent_Message{
			Message: m,
		},
	})
	if err != nil {
		return nil, err
	}

	return &chatv1.SendMessageResponse{}, nil
}

func (d *chatService) Mute(ctx context.Context, req *chatv1.MuteRequest) (*chatv1.MuteResponse, error) {
	peerCert := dialer.VPNCertificate(ctx).GetParent()

	if !d.isModerator(peerCert.Key) {
		return nil, errors.New("operation requires moderator role")
	}

	_, err := d.profileCache.ByPeerKey.Transform(
		dao.FormatChatProfilePeerKey(d.config.Load().Id, req.PeerKey),
		func(m *chatv1.Profile) error {
			m.MuteDeadline = timeutil.Now().Add(time.Duration(req.DurationSecs) * time.Second).Unix()
			if req.Message != "" {
				m.Mutes = append(m.Mutes, &chatv1.Profile_Mute{
					CreatedAt:        timeutil.Now().Unix(),
					DurationSecs:     req.DurationSecs,
					Message:          req.Message,
					ModeratorPeerKey: peerCert.Key,
				})
			}
			return nil
		},
	)
	if err != nil {
		d.logger.Debug("transform failed", zap.Error(err))
		return nil, fmt.Errorf("updating profile failed: %w", err)
	}

	return &chatv1.MuteResponse{}, nil
}

func (d *chatService) Unmute(ctx context.Context, req *chatv1.UnmuteRequest) (*chatv1.UnmuteResponse, error) {
	peerCert := dialer.VPNCertificate(ctx).GetParent()

	if !d.isModerator(peerCert.Key) {
		return nil, errors.New("operation requires moderator role")
	}

	_, err := d.profileCache.ByPeerKey.Transform(
		dao.FormatChatProfilePeerKey(d.config.Load().Id, req.PeerKey),
		func(m *chatv1.Profile) error {
			m.MuteDeadline = timeutil.Now().Unix()
			return nil
		},
	)
	if err != nil {
		d.logger.Debug("transform failed", zap.Error(err))
		return nil, fmt.Errorf("updating profile failed: %w", err)
	}

	return &chatv1.UnmuteResponse{}, nil
}

func (d *chatService) GetMute(ctx context.Context, req *chatv1.GetMuteRequest) (*chatv1.GetMuteResponse, error) {
	debug.PrintJSON(req)
	return &chatv1.GetMuteResponse{}, nil
}
