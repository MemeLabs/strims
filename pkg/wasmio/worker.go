// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package wasmio

import (
	"syscall/js"

	"go.uber.org/zap/zapcore"
)

type WorkerOptions struct {
	LogLevel zapcore.Level
}

func (o *WorkerOptions) FromJSValue(vs js.Value) {
	if v := vs.Get("logLevel"); !v.IsUndefined() {
		o.LogLevel = zapcore.Level(v.Int())
	}
}

func (o *WorkerOptions) JSValue() js.Value {
	v := jsObject.New()
	v.Set("logLevel", js.ValueOf(float64(o.LogLevel)))
	return v
}

// NewWorkerProxy ...
func NewWorkerProxy(bridge js.Value, service string, opt WorkerOptions) Bus {
	ch := make(chan Bus, 1)
	openBus := js.FuncOf(func(this js.Value, args []js.Value) any {
		b, proxy := newBusFromProxy(args[0].Int())
		ch <- b
		return proxy
	})
	defer openBus.Release()

	proxy := jsObject.New()
	proxy.Set("openBus", openBus)
	bridge.Call("openWorker", service, proxy, opt.JSValue())

	return <-ch
}
