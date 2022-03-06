package chat

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/network"
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/debug"
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

func newChatService(logger *zap.Logger, ew *protoutil.ChunkStreamWriter) *chatService {
	return &chatService{
		logger:          logger,
		done:            make(chan struct{}),
		broadcastTicker: time.NewTicker(broadcastInterval),
		eventWriter:     ew,
		entityExtractor: newEntityExtractor(),
		combos:          newComboTransformer(),
	}
}

type chatService struct {
	logger            *zap.Logger
	closeOnce         sync.Once
	done              chan struct{}
	broadcastTicker   *time.Ticker
	lastBroadcastTime timeutil.Time
	eventWriter       *protoutil.ChunkStreamWriter
	lock              sync.Mutex
	certificate       *certificate.Certificate
	entityExtractor   *entityExtractor
	combos            *comboTransformer
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

func (d *chatService) Sync(config *chatv1.Server, emotes []*chatv1.Emote, modifiers []*chatv1.Modifier, tags []*chatv1.Tag) error {
	var emoteNames, modifierNames, tagNames [][]rune
	for _, emote := range emotes {
		emoteNames = append(emoteNames, []rune(emote.Name))
	}
	for _, modifier := range modifiers {
		if !modifier.Internal {
			modifierNames = append(modifierNames, []rune(modifier.Name))
		}
	}
	for _, tag := range tags {
		tagNames = append(tagNames, []rune(tag.Name))
	}

	d.entityExtractor.parserCtx.Emotes.Replace(emoteNames)
	d.entityExtractor.parserCtx.EmoteModifiers.Replace(modifierNames)
	d.entityExtractor.parserCtx.Tags.Replace(tagNames)

	return nil
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

func (d *chatService) SendMessage(ctx context.Context, req *chatv1.SendMessageRequest) (*chatv1.SendMessageResponse, error) {
	hostCert := network.VPNCertificate(ctx)

	m := &chatv1.Message{
		ServerTime: time.Now().UnixNano() / int64(time.Millisecond),
		HostId:     hostCert.Key,
		Nick:       hostCert.GetParent().Subject,
		Body:       req.Body,
		Entities:   d.entityExtractor.Extract(req.Body),
	}

	if err := d.combos.Transform(m); err != nil {
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

func (d *chatService) Mute(ctx context.Context, req *chatv1.MuteRequest) (*chatv1.MuteResponse, error) {
	debug.PrintJSON(req)
	return &chatv1.MuteResponse{}, nil
}

func (d *chatService) Unmute(ctx context.Context, req *chatv1.UnmuteRequest) (*chatv1.UnmuteResponse, error) {
	debug.PrintJSON(req)
	return &chatv1.UnmuteResponse{}, nil
}

func (d *chatService) GetMute(ctx context.Context, req *chatv1.GetMuteRequest) (*chatv1.GetMuteResponse, error) {
	debug.PrintJSON(req)
	return &chatv1.GetMuteResponse{}, nil
}
