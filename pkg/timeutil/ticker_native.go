//go:build !js

package timeutil

import (
	"time"
)

func newFuncTicker(ivl time.Duration, fn func(t Time)) *funcTicker {
	return &funcTicker{
		ivl: ivl,
		fn:  fn,
	}
}

type funcTicker struct {
	ivl time.Duration
	fn  func(t Time)
	*time.Ticker
}

func (f *funcTicker) Start() {
	if f.Ticker == nil {
		f.Ticker = time.NewTicker(f.ivl)
	} else {
		f.Ticker.Reset(f.ivl)
	}

	go f.run(f.fn)
}

func (f *funcTicker) run(fn func(t Time)) {
	for t := range f.Ticker.C {
		fn(NewFromTime(t))
	}
}
