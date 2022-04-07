package chat

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/event"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/servicemanager"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"go.uber.org/zap"
)

func newWhisperRunner(
	ctx context.Context,
	logger *zap.Logger,
	store *dao.ProfileStore,
	observers *event.Observers,
	profile *profilev1.Profile,
	dialer network.Dialer,
) (*whisperRunner, error) {
	a := &whisperRunnerAdapter{
		logger:    logger,
		store:     store,
		observers: observers,
		profile:   profile,
		dialer:    dialer,
	}

	m, err := servicemanager.New[any](logger, ctx, a)
	if err != nil {
		return nil, err
	}

	return &whisperRunner{
		Runner: m,
	}, nil
}

type whisperRunner struct {
	*servicemanager.Runner[any, *whisperRunnerAdapter]
}

type whisperRunnerAdapter struct {
	logger    *zap.Logger
	store     *dao.ProfileStore
	observers *event.Observers
	profile   *profilev1.Profile
	dialer    network.Dialer
}

func (s *whisperRunnerAdapter) Mutex() *dao.Mutex {
	return dao.NewMutex(s.logger, s.store, "whisper")
}

func (s *whisperRunnerAdapter) Client() (servicemanager.Readable[any], error) {
	return servicemanager.NewNoOpClient[any]()
}

func (s *whisperRunnerAdapter) Server() (servicemanager.Readable[any], error) {
	return newWhisperServer(s.logger, s.store, s.observers, s.profile, s.dialer)
}
