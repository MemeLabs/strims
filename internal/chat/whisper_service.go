// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"bytes"
	"context"
	"errors"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/network/dialer"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"golang.org/x/exp/slices"
)

const (
	whisperRateLimitMaxCount = 15
	whisperRateLimitPeriod   = 30 * time.Second
)

var (
	errWhisperRateLimit     = errors.New("whisper rate limit exceeded")
	errWhisperInternalError = errors.New("unable to receive whisper")
)

func newWhisperService(
	logger *zap.Logger,
	store *dao.ProfileStore,
	config *chatv1.UIConfig,
) *whisperService {
	return &whisperService{
		logger: logger,
		store:  store,
		config: syncutil.NewPointer(config),
		done:   make(chan struct{}),
	}
}

type whisperService struct {
	logger    *zap.Logger
	store     *dao.ProfileStore
	config    atomic.Pointer[chatv1.UIConfig]
	closeOnce sync.Once
	done      chan struct{}
	lock      sync.Mutex
}

func (d *whisperService) SyncConfig(config *chatv1.UIConfig) {
	d.config.Swap(config)
}

func (d *whisperService) SendMessage(ctx context.Context, req *chatv1.WhisperSendMessageRequest) (*chatv1.WhisperSendMessageResponse, error) {
	now := timeutil.Now()
	peerCert := dialer.VPNCertificate(ctx).GetParent()

	ignoreIndex := slices.IndexFunc(d.config.Load().Ignores, func(e *chatv1.UIConfig_Ignore) bool {
		return bytes.Equal(e.PeerKey, peerCert.Key) && (e.Deadline == 0 || e.Deadline > now.Unix())
	})
	if ignoreIndex != -1 {
		return &chatv1.WhisperSendMessageResponse{}, nil
	}

	record, err := dao.NewChatWhisperRecord(
		d.store,
		dao.CertificateNetworkKey(peerCert),
		req.ServerKey,
		peerCert.Key,
		peerCert,
		req.Body,
		ExtractMessageEntities(req.Body),
	)
	if err != nil {
		return nil, errWhisperInternalError
	}

	err = d.store.Update(func(tx kv.RWTx) error {
		thread, err := dao.ChatWhisperThreadsByPeerKey.Get(tx, peerCert.Key)
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}
		if thread == nil {
			thread, err = dao.NewChatWhisperThread(d.store, peerCert)
			if err != nil {
				return err
			}
		}

		thread.Alias = peerCert.Subject
		thread.UnreadCount++
		thread.LastReceiveTimes = updateWhisperLastReceiveTimes(thread.LastReceiveTimes, now)
		thread.LastMessageTime = now.UnixNano() / int64(timeutil.Precision)
		thread.LastMessageId = record.Id

		if len(thread.LastReceiveTimes) > whisperRateLimitMaxCount {
			return errWhisperRateLimit
		}

		record.ThreadId = thread.Id
		if err := dao.ChatWhisperRecords.Insert(tx, record); err != nil {
			return err
		}
		return dao.ChatWhisperThreads.Upsert(tx, thread)
	})
	if err != nil {
		return nil, errWhisperInternalError
	}

	return &chatv1.WhisperSendMessageResponse{}, nil
}

func updateWhisperLastReceiveTimes(src []int64, now timeutil.Time) []int64 {
	threshold := now.Add(-whisperRateLimitPeriod).UnixNano() / int64(timeutil.Precision)
	dst := make([]int64, 0, len(src)+1)
	for _, t := range src {
		if t > threshold {
			dst = append(dst, t)
		}
	}
	return append(dst, now.UnixNano()/int64(timeutil.Precision))
}
