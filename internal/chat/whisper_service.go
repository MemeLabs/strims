package chat

import (
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network/dialer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"github.com/MemeLabs/go-ppspp/pkg/syncutil"
	"go.uber.org/zap"
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
	config    syncutil.Pointer[chatv1.UIConfig]
	closeOnce sync.Once
	done      chan struct{}
	lock      sync.Mutex
}

func (d *whisperService) SyncConfig(config *chatv1.UIConfig) {
	d.config.Swap(config)
}

func (d *whisperService) SendMessage(ctx context.Context, req *chatv1.WhisperSendMessageRequest) (*chatv1.WhisperSendMessageResponse, error) {
	peerCert := dialer.VPNCertificate(ctx).GetParent()

	// check last received messages for rate limiting
	// check chat config for blocked peer id

	// whisper message or conversation for network/chat server room context
	m := &chatv1.Message{
		ServerTime: time.Now().UnixNano() / int64(time.Millisecond),
		PeerKey:    peerCert.Key,
		Nick:       peerCert.Subject,
		Body:       req.Body,
	}

	debug.PrintJSON(m)

	// dao store message

	return &chatv1.WhisperSendMessageResponse{}, nil
}
