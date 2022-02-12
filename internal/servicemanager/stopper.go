package servicemanager

import (
	"context"
	"sync"
)

var closedCh = make(chan struct{})

func init() {
	close(closedCh)
}

type Stopper struct {
	closeLock sync.Mutex
	cancel    context.CancelFunc
	closed    <-chan struct{}
}

func (s *Stopper) Start(ctx context.Context) (DoneFunc, context.Context) {
	ctx, cancel := context.WithCancel(ctx)
	closed := make(chan struct{})

	done := func() {
		close(closed)

		s.closeLock.Lock()
		defer s.closeLock.Unlock()
		if s.closed == closed {
			s.cancel = nil
			s.closed = nil
		}
	}

	s.closeLock.Lock()
	defer s.closeLock.Unlock()
	s.cancel = cancel
	s.closed = closed

	return done, ctx
}

func (s *Stopper) Stop() <-chan struct{} {
	s.closeLock.Lock()
	defer s.closeLock.Unlock()
	if s.cancel != nil {
		s.cancel()
		return s.closed
	}
	return closedCh
}

type DoneFunc func()
