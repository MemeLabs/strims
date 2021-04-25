package timeutil

import "time"

func newFuncTicker(ivl time.Duration, fn func(t time.Time)) *funcTicker {
	f := &funcTicker{time.NewTicker(ivl)}
	go f.run(fn)
	return f
}

type funcTicker struct {
	*time.Ticker
}

func (f *funcTicker) run(fn func(t time.Time)) {
	for t := range f.C {
		fn(t)
	}
}
