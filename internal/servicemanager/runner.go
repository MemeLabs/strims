package servicemanager

import (
	"context"
	"fmt"
	"reflect"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"go.uber.org/zap"
)

type Service[R any] interface {
	Mutex() *dao.Mutex
	Client() (Readable[R], error)
	Server() (Readable[R], error)
}

type Readable[R any] interface {
	Reader(ctx context.Context) (R, error)
	Run(context.Context) error
	Close(context.Context) error
}

func New[R any, T Service[R]](
	logger *zap.Logger,
	ctx context.Context,
	service T,
) (*Runner[R, T], error) {
	ctx, cancel := context.WithCancel(ctx)
	r := &Runner[R, T]{
		logger:  logger,
		ctx:     ctx,
		cancel:  cancel,
		service: service,
	}

	server, err := service.Server()
	if err != nil {
		return nil, fmt.Errorf("failed to start server: %w", err)
	}
	if server != nil && !reflect.ValueOf(server).IsNil() {
		go r.runServer(server)
	}

	return r, nil
}

type Runner[R any, T Service[R]] struct {
	logger  *zap.Logger
	ctx     context.Context
	cancel  context.CancelFunc
	service T

	lock sync.Mutex
	ref  *ref[R]
}

func (r *Runner[R, T]) Close() {
	r.lock.Lock()
	defer r.lock.Unlock()

	r.cancel()

	if r.ref != nil {
		r.ref.closeOnce.Do(func() {
			if err := r.ref.readable.Close(r.ctx); err != nil {
				r.logger.Debug("error closing source", zap.Error(err), logutil.Type("source", r.ref.readable))
			}
		})
	}
}

func (r *Runner[R, T]) Reader(ctx context.Context) (R, StopFunc, error) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if r.ref == nil {
		client, err := r.service.Client()
		if err != nil {
			return empty[R](), nil, fmt.Errorf("failed to start client: %w", err)
		}
		r.logger.Debug("created new client", logutil.Type("client", client))
		go r.runClient(client)

		r.ref = newRef(client, 0)
	}

	r.logger.Debug("creating reader", logutil.Type("source", r.ref.readable))
	reader, err := r.ref.readable.Reader(ctx)
	if err != nil {
		return reader, nil, err
	}

	ref := r.ref
	ref.count++

	var stopOnce sync.Once
	stop := func() {
		stopOnce.Do(func() {
			r.lock.Lock()
			defer r.lock.Unlock()

			if ref.count--; ref.count == 0 {
				ref.closeOnce.Do(func() {
					r.closeClient(ref.readable)
					r.ref = nil
				})
			}
		})
	}

	return reader, stop, nil
}

func (r *Runner[R, T]) runClient(c Readable[R]) {
	r.logger.Debug("client starting", logutil.Type("client", c))
	if err := c.Run(r.ctx); err != nil {
		r.logger.Debug("client closed", zap.Error(err), logutil.Type("client", c))
	}
}

func (r *Runner[R, T]) closeClient(c Readable[R]) {
	r.logger.Debug("client closing", logutil.Type("client", c))
	if err := c.Close(r.ctx); err != nil {
		r.logger.Debug("error closing client", zap.Error(err), logutil.Type("client", c))
	}
}

func (r *Runner[R, T]) runServer(s Readable[R]) {
	for r.ctx.Err() == nil {
		mu := r.service.Mutex()
		ctx, err := mu.Lock(r.ctx)
		if err != nil {
			r.logger.Debug("error acquiring lock", zap.Error(err))
			return
		}

		r.lock.Lock()
		if r.ref != nil {
			r.ref.closeOnce.Do(func() { r.closeClient(r.ref.readable) })
		}
		r.ref = newRef(s, 1)
		r.lock.Unlock()

		r.logger.Debug("server starting", logutil.Type("server", s))
		if err := s.Run(ctx); err != nil {
			r.logger.Debug("server closed", zap.Error(err), logutil.Type("server", s))
		}

		r.lock.Lock()
		r.ref = nil
		r.lock.Unlock()

		if err := mu.Release(); err != nil {
			r.logger.Debug("error releasing lock", zap.Error(err))
		}
	}
}

func empty[T any]() (v T) { return }

func newRef[R any](r Readable[R], n int32) *ref[R] {
	return &ref[R]{readable: r, count: n}
}

type ref[R any] struct {
	readable  Readable[R]
	count     int32
	closeOnce sync.Once
}

type StopFunc func()

var noopCloseFunc StopFunc = func() {}
