package vpn

import (
	"context"
	"time"
)

// NewPoller ...
func NewPoller(ctx context.Context, ivl time.Duration, tick func(t time.Time), stop func()) *Poller {
	p := &Poller{
		ctx: ctx,
	}
	go p.run(ivl, tick, stop)
	return p
}

// Poller ...
type Poller struct {
	ctx context.Context
}

func (a *Poller) run(ivl time.Duration, tick func(t time.Time), stop func()) {
	t := time.NewTicker(ivl)
	tick(time.Now())
	for {
		select {
		case <-a.ctx.Done():
			if stop != nil {
				stop()
			}
			return
		case now := <-t.C:
			tick(now)
		}
	}
}
