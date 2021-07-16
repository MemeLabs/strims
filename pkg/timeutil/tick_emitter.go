package timeutil

import (
	"sync"
	"time"
)

var DefaultTickEmitter = NewTickEmitter(100 * time.Millisecond)

func NewTickEmitter(ivl time.Duration) *TickEmitter {
	t := &TickEmitter{}
	t.t = newFuncTicker(ivl, t.run)
	return t
}

type TickEmitter struct {
	lock    sync.Mutex
	chans   []chan Time
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

func (r *TickEmitter) Subscribe(fn func(t Time)) StopFunc {
	ch, stop := r.Chan()
	go func() {
		for t := range ch {
			fn(t)
		}
	}()

	return stop
}

func (r *TickEmitter) Ticker() Ticker {
	var t Ticker
	t.C, t.stop = r.Chan()
	return t
}

func (r *TickEmitter) Chan() (<-chan Time, StopFunc) {
	ch := make(chan Time)
	stop := func() {
		r.unsubscribe(ch)
		close(ch)
	}

	r.lock.Lock()
	defer r.lock.Unlock()
	r.chans = append(r.chans, ch)

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
			r.chans[l] = nil
			r.chans = r.chans[:l]
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
	for _, ch := range r.chans {
		select {
		case ch <- t:
		default:
		}
	}
}
