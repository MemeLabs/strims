package chat

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dialer"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"go.uber.org/zap"
)

// errors
var (
	ErrListingNotFound = errors.New("listing not found")
	ErrSessionNotFound = errors.New("session not found")
	ErrUserNotFound    = errors.New("user not found")
)

const broadcastInterval = 15 * time.Second

func newChatService(logger *zap.Logger, key *key.Key, ew *protoutil.ChunkStreamWriter) *chatService {
	return &chatService{
		logger:          logger,
		done:            make(chan struct{}),
		broadcastTicker: time.NewTicker(broadcastInterval),
		eventWriter:     ew,
	}
}

type chatService struct {
	logger            *zap.Logger
	transfer          *transfer.Control
	closeOnce         sync.Once
	done              chan struct{}
	broadcastTicker   *time.Ticker
	lastBroadcastTime timeutil.Time
	eventWriter       *protoutil.ChunkStreamWriter
	lock              sync.Mutex
	certificate       *certificate.Certificate
}

func (d *chatService) Run(ctx context.Context) error {
	defer d.Close()

	for {
		select {
		case now := <-d.broadcastTicker.C:
			if err := d.broadcast(timeutil.NewFromTime(now)); err != nil {
				return err
			}
		case <-d.done:
			return errors.New("closed")
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

func (d *chatService) Close() {
	d.closeOnce.Do(func() {
		d.broadcastTicker.Stop()
		close(d.done)
	})
}

func (d *chatService) broadcast(now timeutil.Time) error {
	d.lock.Lock()
	defer d.lock.Unlock()

	var events []*networkv1.DirectoryEvent

	if events != nil {
		err := d.eventWriter.Write(&networkv1.DirectoryEventBroadcast{
			Events: events,
		})
		if err != nil {
			return err
		}
	}

	d.lastBroadcastTime = now
	return nil
}

func (d *chatService) SendMessage(ctx context.Context, req *chatv1.SendMessageRequest) (*chatv1.SendMessageResponse, error) {
	hostCert := dialer.VPNCertificate(ctx)

	m := &chatv1.Message{
		ServerTime: time.Now().UnixNano() / int64(time.Millisecond),
		HostId:     hostCert.Key,
		Nick:       hostCert.GetParent().Subject,
		Body:       req.Body,
		Entities:   entities.Extract(req.Body),
	}

	if err := combos.Transform(m); err == ErrComboDuplicate {
		return nil, err
	}

	err := d.eventWriter.Write(&chatv1.ServerEvent{
		Body: &chatv1.ServerEvent_Message{
			Message: m,
		},
	})
	if err != nil {
		return nil, err
	}

	return &chatv1.SendMessageResponse{}, nil
}
