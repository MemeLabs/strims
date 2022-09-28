// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"context"
	"fmt"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/network"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/logutil"
	"go.uber.org/zap"
)

// TODO: retry delivery when recipient comes online?

const whisperDeliveryTimeout = 30 * time.Second

func newWhisperDeliveryService(
	logger *zap.Logger,
	store dao.Store,
	dialer network.Dialer,
) *whisperDeliveryService {
	return &whisperDeliveryService{
		logger: logger,
		store:  store,
		dialer: dialer,

		enqueued: make(chan *chatv1.WhisperRecord, 1),
		finished: make(chan struct{}, 1),
	}
}

type whisperDeliveryService struct {
	logger *zap.Logger
	store  dao.Store
	dialer network.Dialer

	enqueued chan *chatv1.WhisperRecord
	finished chan struct{}
}

func (s *whisperDeliveryService) Run(ctx context.Context) error {
	var closing bool
	var activeSends int

	rs, err := dao.ChatWhisperRecordsByState.GetAll(
		s.store,
		dao.FormatChatWhisperRecordStateKey(chatv1.WhisperRecord_WHISPER_STATE_ENQUEUED),
	)
	if err != nil {
		return err
	}
	for _, r := range rs {
		activeSends++
		go s.send(r)
	}

	for {
		select {
		case r := <-s.enqueued:
			if !closing {
				activeSends++
				go s.send(r)
			}
		case <-s.finished:
			activeSends--
			if activeSends == 0 && closing {
				return nil
			}
		case <-ctx.Done():
			closing = true
			if activeSends == 0 {
				return nil
			}
		}
	}
}

func (s *whisperDeliveryService) HandleWhisper(r *chatv1.WhisperRecord) {
	if r.State == chatv1.WhisperRecord_WHISPER_STATE_ENQUEUED {
		s.enqueued <- r
	}
}

func (s *whisperDeliveryService) send(r *chatv1.WhisperRecord) {
	logger := s.logger.With(
		logutil.ByteHex("peer", r.Message.PeerKey),
		zap.Uint64("whisper", r.Id),
	)

	var state chatv1.WhisperRecord_State
	if err := s.send1(r); err != nil {
		state = chatv1.WhisperRecord_WHISPER_STATE_FAILED
		logger.Debug("whisper delivery failed", zap.Error(err))
	} else {
		state = chatv1.WhisperRecord_WHISPER_STATE_DELIVERED
		logger.Debug("delivered whisper", zap.Error(err))
	}

	_, err := dao.ChatWhisperRecords.Transform(s.store, r.Id, func(p *chatv1.WhisperRecord) error {
		if p == nil {
			return kv.ErrRecordNotFound
		}
		p.State = state
		return nil
	})
	if err != nil {
		logger.Debug("storing whisper state failed", zap.Error(err))
	}

	s.finished <- struct{}{}
}

func (s *whisperDeliveryService) send1(r *chatv1.WhisperRecord) error {
	ctx, cancel := context.WithTimeout(context.Background(), whisperDeliveryTimeout)
	defer cancel()

	client, err := s.dialer.Client(ctx, r.NetworkKey, r.PeerKey, WhisperAddressSalt)
	if err != nil {
		return fmt.Errorf("opening client: %w", err)
	}
	defer client.Close()

	return chatv1.NewWhisperClient(client).SendMessage(ctx, &chatv1.WhisperSendMessageRequest{Body: r.Message.Body}, &chatv1.WhisperSendMessageResponse{})
}
