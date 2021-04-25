package timeutil

import (
	"sync"
	"time"
)

func NewTickEmitter(ivl time.Duration) *TickEmitter {
	t := &TickEmitter{}
	t.t = newFuncTicker(ivl, t.run)
	return t
}

type TickEmitter struct {
	lock    sync.Mutex
	chans   []chan time.Time
	t       *funcTicker
	logOnce sync.Once
}

type StopFunc func()

type Ticker struct {
	stop StopFunc
	C    <-chan time.Time
}

func (t *Ticker) Stop() {
	t.stop()
}

func (r *TickEmitter) Stop() {
	r.t.Stop()
}

func (r *TickEmitter) Subscribe(fn func(t time.Time)) StopFunc {
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

func (r *TickEmitter) Chan() (<-chan time.Time, StopFunc) {
	ch := make(chan time.Time, 1)
	stop := func() {
		r.unsubscribe(ch)
		close(ch)
	}

	r.lock.Lock()
	defer r.lock.Unlock()
	r.chans = append(r.chans, ch)

	return ch, stop
}

func (r *TickEmitter) unsubscribe(ch chan time.Time) {
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
}

func (r *TickEmitter) run(t time.Time) {
	r.lock.Lock()
	defer r.lock.Unlock()
	for _, ch := range r.chans {
		select {
		case ch <- t:
		default:
		}
	}
}
