// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package timeutil

import (
	"context"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/randutil"
)

const maxTickerFuzz = 60 * time.Minute

var DefaultTickEmitter = NewTickEmitter(100 * time.Millisecond)

func NewTickEmitter(ivl time.Duration) *TickEmitter {
	t := &TickEmitter{
		ivl: ivl,
		i:   errutil.Must(randutil.Int63n(int64(maxTickerFuzz / ivl))),
	}
	t.t = newFuncTicker(ivl, t.run)
	return t
}

type TickEmitter struct {
	lock    sync.Mutex
	chans   []chan Time
	ivls    []int64
	i       int64
	ivl     time.Duration
	t       *funcTicker
	logOnce sync.Once
}

type StopFunc func()

type Ticker struct {
	stop StopFunc
	C    <-chan Time
}

func (t *Ticker) Stop() {
	t.stop()
}

func (r *TickEmitter) Stop() {
	r.t.Stop()
}

func (r *TickEmitter) DefaultSubscribe(fn func(t Time)) StopFunc {
	return r.Subscribe(r.ivl, fn, nil)
}

func (r *TickEmitter) Subscribe(ivl time.Duration, fn func(t Time), done func()) StopFunc {
	ch, stop := r.Chan(ivl)
	go func() {
		for t := range ch {
			fn(t)
		}
		if done != nil {
			done()
		}
	}()

	return stop
}

func (r *TickEmitter) SubscribeCtx(ctx context.Context, ivl time.Duration, fn func(t Time), done func()) StopFunc {
	ch, stop := r.Chan(ivl)
	go func() {
	TickLoop:
		for {
			select {
			case t, ok := <-ch:
				if !ok {
					break TickLoop
				}
				fn(t)
			case <-ctx.Done():
				stop()
			}
		}
		if done != nil {
			done()
		}
		stop()
	}()

	return stop
}

func (r *TickEmitter) DefaultTicker() Ticker {
	return r.Ticker(r.ivl)
}

func (r *TickEmitter) Ticker(ivl time.Duration) Ticker {
	var t Ticker
	t.C, t.stop = r.Chan(ivl)
	return t
}

func (r *TickEmitter) DefaultChan() (<-chan Time, StopFunc) {
	return r.Chan(r.ivl)
}

func (r *TickEmitter) Chan(ivl time.Duration) (<-chan Time, StopFunc) {
	ch := make(chan Time, 1)
	var stopOnce sync.Once
	stop := func() {
		stopOnce.Do(func() {
			r.unsubscribe(ch)
			close(ch)
		})
	}

	n := int64(1)
	if ivl > r.ivl {
		n = int64(ivl / r.ivl)
	}

	r.lock.Lock()
	defer r.lock.Unlock()
	r.chans = append(r.chans, ch)
	r.ivls = append(r.ivls, n)

	if len(r.chans) == 1 {
		r.t.Start()
	}

	return ch, stop
}

type DebouncedFunc func(context.Context) StopFunc

func (r *TickEmitter) Debounce(fn func(context.Context), wait time.Duration) DebouncedFunc {
	var mu sync.Mutex
	var lastCall Time
	var lastCtx context.Context
	var ticks <-chan Time
	var stop StopFunc

	cleanup := func() {
		mu.Lock()
		defer mu.Unlock()
		stop()
		lastCtx = nil
		ticks = nil
		stop = nil
	}

	run := func() {
		defer cleanup()

		mu.Lock()
		ctx := lastCtx
		mu.Unlock()

		for {
			select {
			case <-ctx.Done():
				return
			case t, ok := <-ticks:
				if !ok {
					return
				}

				mu.Lock()
				ctx = lastCtx
				d := t.Sub(lastCall)
				mu.Unlock()

				if d > wait && ctx.Err() == nil {
					fn(ctx)
					return
				}
			}
		}
	}

	return func(ctx context.Context) StopFunc {
		mu.Lock()
		defer mu.Unlock()
		lastCall = Now()
		lastCtx = ctx

		if ticks == nil {
			ticks, stop = r.Chan(r.ivl)
			go run()
		}
		return stop
	}
}

func (r *TickEmitter) unsubscribe(ch chan Time) {
	r.lock.Lock()
	defer r.lock.Unlock()

	for i := range r.chans {
		if r.chans[i] == ch {
			l := len(r.chans) - 1

			r.chans[i] = r.chans[l]
			r.ivls[i] = r.ivls[l]

			r.chans[l] = nil

			r.chans = r.chans[:l]
			r.ivls = r.ivls[:l]
			return
		}
	}

	if len(r.chans) == 0 {
		r.t.Stop()
	}
}

func (r *TickEmitter) run(t Time) {
	r.lock.Lock()
	defer r.lock.Unlock()

	r.i++
	for i, ivl := range r.ivls {
		if r.i%ivl == 0 {
			select {
			case r.chans[i] <- t:
			default:
			}
		}
	}
}
