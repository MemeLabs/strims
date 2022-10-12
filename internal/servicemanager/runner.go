// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package servicemanager

import (
	"context"
	"fmt"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/logutil"
	"go.uber.org/zap"
)

type Adapter[R any] interface {
	CanServe() bool
	Mutex() *dao.Mutex
	Client() (Readable[R], error)
	Server() (Readable[R], error)
}

type Readable[R any] interface {
	Reader(ctx context.Context) (R, error)
	Run(context.Context) error
	Close(context.Context) error
}

func New[R any, T Adapter[R]](
	logger *zap.Logger,
	ctx context.Context,
	adapter T,
) (*Runner[R, T], error) {
	ctx, cancel := context.WithCancel(ctx)
	r := &Runner[R, T]{
		logger:  logger,
		ctx:     ctx,
		cancel:  cancel,
		adapter: adapter,
	}

	if adapter.CanServe() {
		go r.runServer()
	}

	return r, nil
}

type Runner[R any, T Adapter[R]] struct {
	logger  *zap.Logger
	ctx     context.Context
	cancel  context.CancelFunc
	adapter T

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

func (r *Runner[R, T]) Reader(ctx context.Context) (reader R, stop StopFunc, err error) {
	r.lock.Lock()
	defer r.lock.Unlock()

	if err = r.ctx.Err(); err != nil {
		return
	}

	if r.ref == nil {
		client, err := r.adapter.Client()
		if err != nil {
			return empty[R](), nil, fmt.Errorf("failed to start client: %w", err)
		}
		r.logger.Debug("created new client", logutil.Type("client", client))
		go r.runClient(client)

		r.ref = &ref[R]{readable: client}
	}

	r.logger.Debug("creating reader", logutil.Type("source", r.ref.readable))
	reader, err = r.ref.readable.Reader(ctx)
	if err != nil {
		return reader, nil, err
	}

	ref := r.ref
	ref.count++

	var stopOnce sync.Once
	stop = func() {
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

func (r *Runner[R, T]) runServer() {
	for r.ctx.Err() == nil {
		mu := r.adapter.Mutex()
		ctx, err := mu.Lock(r.ctx)
		if err != nil {
			r.logger.Debug("error acquiring lock", zap.Error(err))
			return
		}

		s, err := r.adapter.Server()
		if err != nil {
			r.logger.Debug("failed to start server: %w", zap.Error(err))
		} else {
			r.lock.Lock()
			if r.ref != nil {
				r.ref.closeOnce.Do(func() { r.closeClient(r.ref.readable) })
			}
			r.ref = &ref[R]{readable: s, count: 1}
			r.lock.Unlock()

			r.logger.Debug("server starting", logutil.Type("server", s))
			if err := s.Run(ctx); err != nil {
				r.logger.Debug("server closed", zap.Error(err), logutil.Type("server", s))
			}
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

type ref[R any] struct {
	readable  Readable[R]
	count     int32
	closeOnce sync.Once
}

type StopFunc func()
