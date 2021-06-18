package timeutil

import (
	"context"
	"time"
)

// TODO: merge with ticker

// TickerBFunc ...
func TickerBFunc(ctx context.Context, ivl time.Duration, tick func(t Time)) *TickerB {
	return TickerBFuncWithCleanup(ctx, ivl, tick, nil)
}

// TickerBFuncWithCleanup ...
func TickerBFuncWithCleanup(ctx context.Context, ivl time.Duration, tick func(t Time), stop func()) *TickerB {
	p := &TickerB{
		ctx:  ctx,
		stop: make(chan struct{}),
	}
	go p.run(ivl, tick, stop)
	return p
}

// TickerB ...
type TickerB struct {
	ctx  context.Context
	stop chan struct{}
}

// Stop ...
func (a *TickerB) Stop() {
	close(a.stop)
}

func (a *TickerB) run(ivl time.Duration, tick func(t Time), stop func()) {
	t := time.NewTicker(ivl)

	defer func() {
		t.Stop()
		if stop != nil {
			stop()
		}
	}()

	tick(Now())

	for {
		select {
		case <-a.ctx.Done():
			return
		case <-a.stop:
			return
		case now := <-t.C:
			tick(NewFromTime(now))
		}
	}
}
