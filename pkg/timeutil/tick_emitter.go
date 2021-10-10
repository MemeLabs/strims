package timeutil

import (
	"sync"
	"time"
)

var DefaultTickEmitter = NewTickEmitter(100 * time.Millisecond)

func NewTickEmitter(ivl time.Duration) *TickEmitter {
	t := &TickEmitter{ivl: ivl}
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
	return r.Subscribe(fn, r.ivl)
}

func (r *TickEmitter) Subscribe(fn func(t Time), ivl time.Duration) StopFunc {
	ch, stop := r.Chan(ivl)
	go func() {
		for t := range ch {
			fn(t)
		}
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
	stop := func() {
		r.unsubscribe(ch)
		close(ch)
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
	for i, ch := range r.chans {
		if r.i%r.ivls[i] == 0 {
			select {
			case ch <- t:
			default:
			}
		}
	}
}
