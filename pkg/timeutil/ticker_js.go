//go:build js

package timeutil

import (
	// "syscall/js"
	"time"
)

/*
the runtime spamming setTimeout/clearTimeout accounts for ~10% of our cpu time.
most of our use cases expect (or at least tolerate) imprecise tick intervals
so we can probaly use our own timer wheel to dispatch them.

verify that the runtime doesn't set intervals when there are no native timers
running.

replace all the native timers with timeutil... the api should probably be
rewritten to match time's to simplify migration and fall through to the
native implementation.

implement a helper to pass the time to the tick function
*/

// func newFuncTicker(ivl time.Duration, fn func(t Time)) *funcTicker {
// 	jsfn := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
// 		fn(Time(args[0].Float()) * Time(time.Millisecond))
// 		return nil
// 	})

// 	id := js.Global().Call("setInterval", jsfn, js.ValueOf(int64(ivl/time.Millisecond)))

// 	return &funcTicker{jsfn, id}
// }

// type funcTicker struct {
// 	fn js.Func
// 	id js.Value
// }

// func (f *funcTicker) Stop() {
// 	js.Global().Call("clearInterval", f.id)
// 	f.fn.Release()
// }

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
		SyncNow(t.UnixNano())
		fn(Now())
	}
}
