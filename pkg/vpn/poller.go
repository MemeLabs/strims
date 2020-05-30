package vpn

import (
	"sync"
	"time"
)

// NewPoller ...
func NewPoller(ivl time.Duration, tick func(t time.Time), stop func()) *Poller {
	p := &Poller{
		stop: make(chan struct{}, 1),
	}
	go p.run(ivl, tick, stop)
	return p
}

// Poller ...
type Poller struct {
	stop     chan struct{}
	stopOnce sync.Once
}

func (a *Poller) run(ivl time.Duration, tick func(t time.Time), stop func()) {
	t := time.NewTicker(ivl)
	tick(time.Now())
	for {
		select {
		case <-a.stop:
			if stop != nil {
				stop()
			}
			return
		case now := <-t.C:
			tick(now)
		}
	}
}

// Stop ...
func (a *Poller) Stop() {
	a.stopOnce.Do(func() {
		a.stop <- struct{}{}
		close(a.stop)
	})
}
