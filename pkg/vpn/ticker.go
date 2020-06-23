package vpn

import (
	"context"
	"time"
)

// TickerFunc ...
func TickerFunc(ctx context.Context, ivl time.Duration, tick func(t time.Time)) *Ticker {
	return TickerFuncWithCleanup(ctx, ivl, tick, nil)
}

// TickerFuncWithCleanup ...
func TickerFuncWithCleanup(ctx context.Context, ivl time.Duration, tick func(t time.Time), stop func()) *Ticker {
	p := &Ticker{
		ctx:  ctx,
		stop: make(chan struct{}),
	}
	go p.run(ivl, tick, stop)
	return p
}

// Ticker ...
type Ticker struct {
	ctx  context.Context
	stop chan struct{}
}

// Stop ...
func (a *Ticker) Stop() {
	close(a.stop)
}

func (a *Ticker) run(ivl time.Duration, tick func(t time.Time), stop func()) {
	t := time.NewTicker(ivl)

	defer func() {
		t.Stop()
		if stop != nil {
			stop()
		}
	}()

	tick(time.Now())

	for {
		select {
		case <-a.ctx.Done():
			return
		case <-a.stop:
			return
		case now := <-t.C:
			tick(now)
		}
	}
}
